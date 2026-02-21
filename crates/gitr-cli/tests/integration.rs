use std::path::PathBuf;

use gitr_core::config::GitrConfig;
use gitr_core::models::host::{Host, HostKind};
use gitr_core::models::repo::{DiscoverySource, Repo};
use gitr_core::models::sync_link::{MergeStrategy, SyncDirection, SyncLink};
use gitr_core::models::sync_state::{BranchSnapshot, SyncRecord, SyncStatus};
use gitr_db::{open_memory_db, ops};

#[test]
fn test_config_defaults() {
    let config = GitrConfig::default();
    assert_eq!(config.sync_concurrency, 8);
    assert_eq!(config.max_scan_depth, 4);
    assert_eq!(config.default_merge_strategy, MergeStrategy::FastForward);
}

#[test]
fn test_config_roundtrip() {
    let config = GitrConfig::default();
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: GitrConfig = toml::from_str(&serialized).unwrap();
    assert_eq!(config.sync_concurrency, deserialized.sync_concurrency);
}

#[test]
fn test_full_pipeline_in_memory() {
    let conn = open_memory_db().unwrap();

    // 1. Register a host
    let host = Host::new("gh".to_string(), HostKind::GitHub, "testuser".to_string());
    ops::insert_host(&conn, &host).unwrap();

    let hosts = ops::list_hosts(&conn).unwrap();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].label, "gh");

    // 2. Track repos (simulating scan results)
    let mut repo1 = Repo::new(
        "testuser/myproject".to_string(),
        host.id.clone(),
        "https://github.com/testuser/myproject.git".to_string(),
        "main".to_string(),
        DiscoverySource::Api,
    );
    repo1.local_path = Some(PathBuf::from("/home/testuser/repos/myproject"));
    ops::insert_repo(&conn, &repo1).unwrap();

    let mut fork = Repo::new(
        "testuser/linux".to_string(),
        host.id.clone(),
        "https://github.com/testuser/linux.git".to_string(),
        "master".to_string(),
        DiscoverySource::Api,
    );
    fork.is_fork = true;
    fork.upstream_full_name = Some("torvalds/linux".to_string());
    ops::insert_repo(&conn, &fork).unwrap();

    let all_repos = ops::list_repos(&conn).unwrap();
    assert_eq!(all_repos.len(), 2);

    let fork_repos = ops::list_fork_repos(&conn).unwrap();
    assert_eq!(fork_repos.len(), 1);
    assert_eq!(fork_repos[0].full_name, "testuser/linux");

    // 3. Record a sync
    let mut record = SyncRecord::new(fork.id.clone());
    record.branches_synced = 1;
    record.commits_transferred = 42;
    record.status = SyncStatus::Success;
    ops::insert_sync_record(&conn, &record).unwrap();

    // Update last synced
    ops::update_repo_last_synced(&conn, &fork.id, &record.finished_at).unwrap();

    // 4. Verify sync history
    let history = ops::list_sync_history(&conn, Some(&fork.id), 10).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].branches_synced, 1);
    assert_eq!(history[0].commits_transferred, 42);

    // 5. Branch snapshot
    let snap = BranchSnapshot {
        repo_id: fork.id.clone(),
        branch: "master".to_string(),
        local_sha: Some("abc123".to_string()),
        remote_sha: Some("abc123".to_string()),
        upstream_sha: Some("def456".to_string()),
        behind_count: 3,
        ahead_count: 0,
        updated_at: chrono::Utc::now(),
    };
    ops::upsert_branch_snapshot(&conn, &snap).unwrap();

    let snapshots = ops::get_branch_snapshots(&conn, &fork.id).unwrap();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].behind_count, 3);

    // 6. Create sync link
    let link = SyncLink::new(
        repo1.id.clone(),
        fork.id.clone(),
        SyncDirection::Pull,
        MergeStrategy::FastForward,
    );
    ops::insert_sync_link(&conn, &link).unwrap();

    let links = ops::list_sync_links(&conn).unwrap();
    assert_eq!(links.len(), 1);

    // 7. Cleanup
    ops::delete_sync_link(&conn, &link.id).unwrap();
    ops::delete_repo(&conn, &fork.id).unwrap();
    ops::delete_repo(&conn, &repo1.id).unwrap();
    ops::delete_host(&conn, &host.id).unwrap();

    assert!(ops::list_hosts(&conn).unwrap().is_empty());
    assert!(ops::list_repos(&conn).unwrap().is_empty());
}

#[test]
fn test_auth_memory_store() {
    use gitr_auth::{CredentialStore, MemoryStore};

    let store = MemoryStore::new();
    assert_eq!(store.get("test").unwrap(), None);
    store.store("test", "token123").unwrap();
    assert_eq!(store.get("test").unwrap(), Some("token123".to_string()));
    store.delete("test").unwrap();
    assert_eq!(store.get("test").unwrap(), None);
}

#[test]
fn test_scanner_finds_git_repos() {
    use gitr_discover::scanner::scan_directory;

    let dir = tempfile::tempdir().unwrap();

    // Create a fake git repo
    let repo_dir = dir.path().join("myrepo");
    let git_dir = repo_dir.join(".git");
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(
        git_dir.join("config"),
        r#"[remote "origin"]
	url = https://github.com/user/myrepo.git
"#,
    )
    .unwrap();

    let repos = scan_directory(dir.path(), 3);
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].remotes.len(), 1);
    assert_eq!(repos[0].remotes[0].name, "origin");
    assert!(repos[0].remotes[0].url.contains("myrepo"));
}

#[test]
fn test_reconcile() {
    use gitr_discover::reconcile::reconcile;
    use gitr_discover::scanner::{ScannedRemote, ScannedRepo};
    use gitr_host::RemoteRepo;

    let local = vec![ScannedRepo {
        path: "/repos/myrepo".into(),
        remotes: vec![ScannedRemote {
            name: "origin".to_string(),
            url: "https://github.com/user/myrepo.git".to_string(),
        }],
    }];

    let remote = vec![
        RemoteRepo {
            full_name: "user/myrepo".to_string(),
            owner: "user".to_string(),
            name: "myrepo".to_string(),
            clone_url: "https://github.com/user/myrepo.git".to_string(),
            ssh_url: "git@github.com:user/myrepo.git".to_string(),
            default_branch: "main".to_string(),
            is_fork: false,
            upstream_full_name: None,
            upstream_clone_url: None,
            description: None,
            is_private: false,
            is_archived: false,
            updated_at: None,
        },
        RemoteRepo {
            full_name: "user/other".to_string(),
            owner: "user".to_string(),
            name: "other".to_string(),
            clone_url: "https://github.com/user/other.git".to_string(),
            ssh_url: "git@github.com:user/other.git".to_string(),
            default_branch: "main".to_string(),
            is_fork: true,
            upstream_full_name: Some("upstream/other".to_string()),
            upstream_clone_url: None,
            description: None,
            is_private: false,
            is_archived: false,
            updated_at: None,
        },
    ];

    let result = reconcile(&local, &remote, "gh");
    assert_eq!(result.matched_count(), 1);
    assert_eq!(result.local_only_count(), 0);
    assert_eq!(result.remote_only_count(), 1);
}

#[test]
fn test_host_kind_roundtrip() {
    let kinds = vec![
        HostKind::GitHub,
        HostKind::GitLab,
        HostKind::Gitea,
        HostKind::Bitbucket,
        HostKind::AzureDevOps,
    ];
    for kind in kinds {
        let s = kind.to_string();
        let parsed: HostKind = s.parse().unwrap();
        assert_eq!(kind, parsed);
    }
}

#[test]
fn test_merge_strategy_roundtrip() {
    let strategies = vec![
        ("ff", MergeStrategy::FastForward),
        ("merge", MergeStrategy::Merge),
        ("rebase", MergeStrategy::Rebase),
        ("force_push", MergeStrategy::ForcePush),
    ];
    for (s, expected) in strategies {
        let parsed: MergeStrategy = s.parse().unwrap();
        assert_eq!(parsed, expected);
    }
}
