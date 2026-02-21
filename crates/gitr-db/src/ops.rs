use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use gitr_core::models::collection::{Collection, CollectionId, CollectionMember};
use gitr_core::models::host::{Host, HostId, HostKind};
use gitr_core::models::repo::{DiscoverySource, Repo, RepoId};
use gitr_core::models::sync_link::{
    MergeStrategy, SyncDirection, SyncLink, SyncLinkId, SyncTrigger,
};
use gitr_core::models::sync_state::{BranchSnapshot, SyncRecord, SyncStatus};

// ── Helpers ──

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn fmt_dt(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn opt_dt(dt: &Option<DateTime<Utc>>) -> Option<String> {
    dt.as_ref().map(fmt_dt)
}

// ── Hosts ──

pub fn insert_host(conn: &Connection, host: &Host) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO hosts (id, label, kind, api_url, username, credential_key)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            host.id.0.to_string(),
            host.label,
            host.kind.to_string(),
            host.api_url.to_string(),
            host.username,
            host.credential_key,
        ],
    )?;
    Ok(())
}

pub fn get_host_by_label(conn: &Connection, label: &str) -> anyhow::Result<Option<Host>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, kind, api_url, username, credential_key
         FROM hosts WHERE label = ?1",
    )?;
    let mut rows = stmt.query(params![label])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_host(row)?)),
        None => Ok(None),
    }
}

pub fn get_host_by_id(conn: &Connection, id: &HostId) -> anyhow::Result<Option<Host>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, kind, api_url, username, credential_key
         FROM hosts WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id.0.to_string()])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_host(row)?)),
        None => Ok(None),
    }
}

pub fn list_hosts(conn: &Connection) -> anyhow::Result<Vec<Host>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, kind, api_url, username, credential_key
         FROM hosts ORDER BY label",
    )?;
    let rows = stmt.query_map([], |row| row_to_host(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_host(conn: &Connection, id: &HostId) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM hosts WHERE id = ?1",
        params![id.0.to_string()],
    )?;
    Ok(())
}

fn row_to_host(row: &rusqlite::Row) -> rusqlite::Result<Host> {
    let id_str: String = row.get(0)?;
    let label: String = row.get(1)?;
    let kind_str: String = row.get(2)?;
    let api_url_str: String = row.get(3)?;
    let username: String = row.get(4)?;
    let credential_key: String = row.get(5)?;

    Ok(Host {
        id: HostId::from_uuid(Uuid::parse_str(&id_str).unwrap_or_default()),
        label,
        kind: kind_str.parse().unwrap_or(HostKind::GitHub),
        api_url: url::Url::parse(&api_url_str).unwrap_or_else(|_| {
            url::Url::parse("https://api.github.com").unwrap()
        }),
        username,
        credential_key,
    })
}

// ── Repos ──

pub fn insert_repo(conn: &Connection, repo: &Repo) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO repos (id, full_name, owner, name, host_id, clone_url, local_path, is_fork, upstream_repo_id, upstream_full_name, default_branch, discovery_source, last_synced_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            repo.id.0.to_string(),
            repo.full_name,
            repo.owner,
            repo.name,
            repo.host_id.0.to_string(),
            repo.clone_url,
            repo.local_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            repo.is_fork as i32,
            repo.upstream_repo_id.as_ref().map(|id| id.0.to_string()),
            repo.upstream_full_name,
            repo.default_branch,
            repo.discovery_source.to_string(),
            opt_dt(&repo.last_synced_at),
            fmt_dt(&repo.created_at),
        ],
    )?;
    Ok(())
}

pub fn get_repo_by_id(conn: &Connection, id: &RepoId) -> anyhow::Result<Option<Repo>> {
    let mut stmt = conn.prepare(
        "SELECT id, full_name, owner, name, host_id, clone_url, local_path, is_fork, upstream_repo_id, upstream_full_name, default_branch, discovery_source, last_synced_at, created_at
         FROM repos WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id.0.to_string()])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_repo(row)?)),
        None => Ok(None),
    }
}

pub fn get_repo_by_full_name(
    conn: &Connection,
    host_id: &HostId,
    full_name: &str,
) -> anyhow::Result<Option<Repo>> {
    let mut stmt = conn.prepare(
        "SELECT id, full_name, owner, name, host_id, clone_url, local_path, is_fork, upstream_repo_id, upstream_full_name, default_branch, discovery_source, last_synced_at, created_at
         FROM repos WHERE host_id = ?1 AND full_name = ?2",
    )?;
    let mut rows = stmt.query(params![host_id.0.to_string(), full_name])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_repo(row)?)),
        None => Ok(None),
    }
}

pub fn list_repos(conn: &Connection) -> anyhow::Result<Vec<Repo>> {
    let mut stmt = conn.prepare(
        "SELECT id, full_name, owner, name, host_id, clone_url, local_path, is_fork, upstream_repo_id, upstream_full_name, default_branch, discovery_source, last_synced_at, created_at
         FROM repos ORDER BY full_name",
    )?;
    let rows = stmt.query_map([], |row| row_to_repo(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_repos_for_host(conn: &Connection, host_id: &HostId) -> anyhow::Result<Vec<Repo>> {
    let mut stmt = conn.prepare(
        "SELECT id, full_name, owner, name, host_id, clone_url, local_path, is_fork, upstream_repo_id, upstream_full_name, default_branch, discovery_source, last_synced_at, created_at
         FROM repos WHERE host_id = ?1 ORDER BY full_name",
    )?;
    let rows = stmt.query_map(params![host_id.0.to_string()], |row| row_to_repo(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_fork_repos(conn: &Connection) -> anyhow::Result<Vec<Repo>> {
    let mut stmt = conn.prepare(
        "SELECT id, full_name, owner, name, host_id, clone_url, local_path, is_fork, upstream_repo_id, upstream_full_name, default_branch, discovery_source, last_synced_at, created_at
         FROM repos WHERE is_fork = 1 ORDER BY full_name",
    )?;
    let rows = stmt.query_map([], |row| row_to_repo(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn update_repo_local_path(
    conn: &Connection,
    id: &RepoId,
    local_path: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE repos SET local_path = ?1 WHERE id = ?2",
        params![
            local_path.map(|p| p.to_string_lossy().to_string()),
            id.0.to_string(),
        ],
    )?;
    Ok(())
}

pub fn update_repo_last_synced(
    conn: &Connection,
    id: &RepoId,
    ts: &DateTime<Utc>,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE repos SET last_synced_at = ?1 WHERE id = ?2",
        params![fmt_dt(ts), id.0.to_string()],
    )?;
    Ok(())
}

pub fn delete_repo(conn: &Connection, id: &RepoId) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM repos WHERE id = ?1",
        params![id.0.to_string()],
    )?;
    Ok(())
}

fn row_to_repo(row: &rusqlite::Row) -> rusqlite::Result<Repo> {
    let id_str: String = row.get(0)?;
    let full_name: String = row.get(1)?;
    let owner: String = row.get(2)?;
    let name: String = row.get(3)?;
    let host_id_str: String = row.get(4)?;
    let clone_url: String = row.get(5)?;
    let local_path: Option<String> = row.get(6)?;
    let is_fork: i32 = row.get(7)?;
    let upstream_repo_id: Option<String> = row.get(8)?;
    let upstream_full_name: Option<String> = row.get(9)?;
    let default_branch: String = row.get(10)?;
    let discovery_source_str: String = row.get(11)?;
    let last_synced_str: Option<String> = row.get(12)?;
    let created_str: String = row.get(13)?;

    Ok(Repo {
        id: RepoId::from_uuid(Uuid::parse_str(&id_str).unwrap_or_default()),
        full_name,
        owner,
        name,
        host_id: HostId::from_uuid(Uuid::parse_str(&host_id_str).unwrap_or_default()),
        clone_url,
        local_path: local_path.map(Into::into),
        is_fork: is_fork != 0,
        upstream_repo_id: upstream_repo_id
            .and_then(|s| Uuid::parse_str(&s).ok())
            .map(RepoId::from_uuid),
        upstream_full_name,
        default_branch,
        discovery_source: discovery_source_str
            .parse()
            .unwrap_or(DiscoverySource::Api),
        last_synced_at: last_synced_str.map(|s| parse_dt(&s)),
        created_at: parse_dt(&created_str),
    })
}

// ── Collections ──

pub fn insert_collection(conn: &Connection, col: &Collection) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO collections (id, name, description) VALUES (?1, ?2, ?3)",
        params![col.id.0.to_string(), col.name, col.description],
    )?;
    Ok(())
}

pub fn list_collections(conn: &Connection) -> anyhow::Result<Vec<Collection>> {
    let mut stmt =
        conn.prepare("SELECT id, name, description FROM collections ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        let id_str: String = row.get(0)?;
        Ok(Collection {
            id: CollectionId::from_uuid(Uuid::parse_str(&id_str).unwrap_or_default()),
            name: row.get(1)?,
            description: row.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_collection(conn: &Connection, id: &CollectionId) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM collections WHERE id = ?1",
        params![id.0.to_string()],
    )?;
    Ok(())
}

pub fn add_collection_member(conn: &Connection, member: &CollectionMember) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO collection_members (collection_id, repo_id) VALUES (?1, ?2)",
        params![
            member.collection_id.0.to_string(),
            member.repo_id.0.to_string(),
        ],
    )?;
    Ok(())
}

pub fn remove_collection_member(
    conn: &Connection,
    collection_id: &CollectionId,
    repo_id: &RepoId,
) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM collection_members WHERE collection_id = ?1 AND repo_id = ?2",
        params![collection_id.0.to_string(), repo_id.0.to_string()],
    )?;
    Ok(())
}

// ── Sync Links ──

pub fn insert_sync_link(conn: &Connection, link: &SyncLink) -> anyhow::Result<()> {
    let instructions_json =
        serde_json::to_string(&link.instructions).unwrap_or_else(|_| "{}".to_string());
    conn.execute(
        "INSERT INTO sync_links (id, source_repo_id, target_repo_id, direction, merge_strategy, trigger, instructions, enabled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            link.id.0.to_string(),
            link.source_repo_id.0.to_string(),
            link.target_repo_id.0.to_string(),
            link.direction.to_string(),
            link.merge_strategy.to_string(),
            link.trigger.to_string(),
            instructions_json,
            link.enabled as i32,
        ],
    )?;
    Ok(())
}

pub fn list_sync_links(conn: &Connection) -> anyhow::Result<Vec<SyncLink>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_repo_id, target_repo_id, direction, merge_strategy, trigger, instructions, enabled
         FROM sync_links ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| row_to_sync_link(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_sync_link(conn: &Connection, id: &SyncLinkId) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM sync_links WHERE id = ?1",
        params![id.0.to_string()],
    )?;
    Ok(())
}

fn row_to_sync_link(row: &rusqlite::Row) -> rusqlite::Result<SyncLink> {
    let id_str: String = row.get(0)?;
    let source_str: String = row.get(1)?;
    let target_str: String = row.get(2)?;
    let dir_str: String = row.get(3)?;
    let strat_str: String = row.get(4)?;
    let trigger_str: String = row.get(5)?;
    let instr_str: String = row.get(6)?;
    let enabled: i32 = row.get(7)?;

    let trigger = if trigger_str.starts_with("schedule:") {
        SyncTrigger::Schedule {
            cron: trigger_str.strip_prefix("schedule:").unwrap_or("").to_string(),
        }
    } else {
        match trigger_str.as_str() {
            "always" => SyncTrigger::Always,
            _ => SyncTrigger::Manual,
        }
    };

    Ok(SyncLink {
        id: SyncLinkId::from_uuid(Uuid::parse_str(&id_str).unwrap_or_default()),
        source_repo_id: RepoId::from_uuid(Uuid::parse_str(&source_str).unwrap_or_default()),
        target_repo_id: RepoId::from_uuid(Uuid::parse_str(&target_str).unwrap_or_default()),
        direction: dir_str.parse().unwrap_or(SyncDirection::Pull),
        merge_strategy: strat_str.parse().unwrap_or(MergeStrategy::FastForward),
        trigger,
        instructions: serde_json::from_str(&instr_str).unwrap_or_default(),
        enabled: enabled != 0,
    })
}

// ── Sync History ──

pub fn insert_sync_record(conn: &Connection, record: &SyncRecord) -> anyhow::Result<()> {
    let errors_json =
        serde_json::to_string(&record.errors).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO sync_history (id, repo_id, sync_link_id, branches_synced, branches_failed, commits_transferred, status, errors, started_at, finished_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            record.id.to_string(),
            record.repo_id.0.to_string(),
            record.sync_link_id.as_ref().map(|id| id.0.to_string()),
            record.branches_synced as i64,
            record.branches_failed as i64,
            record.commits_transferred as i64,
            record.status.to_string(),
            errors_json,
            fmt_dt(&record.started_at),
            fmt_dt(&record.finished_at),
        ],
    )?;
    Ok(())
}

pub fn list_sync_history(
    conn: &Connection,
    repo_id: Option<&RepoId>,
    limit: u32,
) -> anyhow::Result<Vec<SyncRecord>> {
    let (sql, bind_id) = match repo_id {
        Some(id) => (
            "SELECT id, repo_id, sync_link_id, branches_synced, branches_failed, commits_transferred, status, errors, started_at, finished_at
             FROM sync_history WHERE repo_id = ?1 ORDER BY started_at DESC LIMIT ?2",
            Some(id.0.to_string()),
        ),
        None => (
            "SELECT id, repo_id, sync_link_id, branches_synced, branches_failed, commits_transferred, status, errors, started_at, finished_at
             FROM sync_history ORDER BY started_at DESC LIMIT ?2",
            None,
        ),
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(ref id_str) = bind_id {
        stmt.query_map(params![id_str, limit], |row| row_to_sync_record(row))?
    } else {
        // When no repo_id filter, ?2 becomes ?1 positionally — re-prepare
        drop(stmt);
        let mut stmt2 = conn.prepare(
            "SELECT id, repo_id, sync_link_id, branches_synced, branches_failed, commits_transferred, status, errors, started_at, finished_at
             FROM sync_history ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt2.query_map(params![limit], |row| row_to_sync_record(row))?;
        return Ok(rows.filter_map(|r| r.ok()).collect());
    };
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn row_to_sync_record(row: &rusqlite::Row) -> rusqlite::Result<SyncRecord> {
    let id_str: String = row.get(0)?;
    let repo_id_str: String = row.get(1)?;
    let link_id_str: Option<String> = row.get(2)?;
    let branches_synced: i64 = row.get(3)?;
    let branches_failed: i64 = row.get(4)?;
    let commits: i64 = row.get(5)?;
    let status_str: String = row.get(6)?;
    let errors_str: String = row.get(7)?;
    let started_str: String = row.get(8)?;
    let finished_str: String = row.get(9)?;

    Ok(SyncRecord {
        id: Uuid::parse_str(&id_str).unwrap_or_default(),
        repo_id: RepoId::from_uuid(Uuid::parse_str(&repo_id_str).unwrap_or_default()),
        sync_link_id: link_id_str
            .and_then(|s| Uuid::parse_str(&s).ok())
            .map(SyncLinkId::from_uuid),
        branches_synced: branches_synced as u32,
        branches_failed: branches_failed as u32,
        commits_transferred: commits as u32,
        status: status_str.parse().unwrap_or(SyncStatus::Failed),
        errors: serde_json::from_str(&errors_str).unwrap_or_default(),
        started_at: parse_dt(&started_str),
        finished_at: parse_dt(&finished_str),
    })
}

// ── Branch Snapshots ──

pub fn upsert_branch_snapshot(conn: &Connection, snap: &BranchSnapshot) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO branch_snapshots (repo_id, branch, local_sha, remote_sha, upstream_sha, behind_count, ahead_count, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            snap.repo_id.0.to_string(),
            snap.branch,
            snap.local_sha,
            snap.remote_sha,
            snap.upstream_sha,
            snap.behind_count as i64,
            snap.ahead_count as i64,
            fmt_dt(&snap.updated_at),
        ],
    )?;
    Ok(())
}

pub fn get_branch_snapshots(
    conn: &Connection,
    repo_id: &RepoId,
) -> anyhow::Result<Vec<BranchSnapshot>> {
    let mut stmt = conn.prepare(
        "SELECT repo_id, branch, local_sha, remote_sha, upstream_sha, behind_count, ahead_count, updated_at
         FROM branch_snapshots WHERE repo_id = ?1 ORDER BY branch",
    )?;
    let rows = stmt.query_map(params![repo_id.0.to_string()], |row| {
        let repo_id_str: String = row.get(0)?;
        let branch: String = row.get(1)?;
        let local_sha: Option<String> = row.get(2)?;
        let remote_sha: Option<String> = row.get(3)?;
        let upstream_sha: Option<String> = row.get(4)?;
        let behind: i64 = row.get(5)?;
        let ahead: i64 = row.get(6)?;
        let updated_str: String = row.get(7)?;
        Ok(BranchSnapshot {
            repo_id: RepoId::from_uuid(Uuid::parse_str(&repo_id_str).unwrap_or_default()),
            branch,
            local_sha,
            remote_sha,
            upstream_sha,
            behind_count: behind as u32,
            ahead_count: ahead as u32,
            updated_at: parse_dt(&updated_str),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory_db;
    use gitr_core::models::host::HostKind;
    use gitr_core::models::repo::DiscoverySource;

    #[test]
    fn test_host_crud() {
        let conn = open_memory_db().unwrap();
        let host = Host::new("gh".to_string(), HostKind::GitHub, "testuser".to_string());
        insert_host(&conn, &host).unwrap();

        let found = get_host_by_label(&conn, "gh").unwrap().unwrap();
        assert_eq!(found.label, "gh");
        assert_eq!(found.kind, HostKind::GitHub);
        assert_eq!(found.username, "testuser");

        let all = list_hosts(&conn).unwrap();
        assert_eq!(all.len(), 1);

        delete_host(&conn, &host.id).unwrap();
        assert!(get_host_by_label(&conn, "gh").unwrap().is_none());
    }

    #[test]
    fn test_repo_crud() {
        let conn = open_memory_db().unwrap();
        let host = Host::new("gh".to_string(), HostKind::GitHub, "testuser".to_string());
        insert_host(&conn, &host).unwrap();

        let repo = Repo::new(
            "testuser/myrepo".to_string(),
            host.id.clone(),
            "https://github.com/testuser/myrepo.git".to_string(),
            "main".to_string(),
            DiscoverySource::Api,
        );
        insert_repo(&conn, &repo).unwrap();

        let found = get_repo_by_id(&conn, &repo.id).unwrap().unwrap();
        assert_eq!(found.full_name, "testuser/myrepo");
        assert_eq!(found.owner, "testuser");
        assert_eq!(found.name, "myrepo");

        let by_name = get_repo_by_full_name(&conn, &host.id, "testuser/myrepo")
            .unwrap()
            .unwrap();
        assert_eq!(by_name.id, repo.id);

        let all = list_repos(&conn).unwrap();
        assert_eq!(all.len(), 1);

        delete_repo(&conn, &repo.id).unwrap();
        assert!(get_repo_by_id(&conn, &repo.id).unwrap().is_none());
    }

    #[test]
    fn test_sync_record_crud() {
        let conn = open_memory_db().unwrap();
        let host = Host::new("gh".to_string(), HostKind::GitHub, "user".to_string());
        insert_host(&conn, &host).unwrap();

        let repo = Repo::new(
            "user/repo".to_string(),
            host.id.clone(),
            "https://github.com/user/repo.git".to_string(),
            "main".to_string(),
            DiscoverySource::Api,
        );
        insert_repo(&conn, &repo).unwrap();

        let mut record = SyncRecord::new(repo.id.clone());
        record.branches_synced = 1;
        record.status = SyncStatus::Success;
        insert_sync_record(&conn, &record).unwrap();

        let history = list_sync_history(&conn, Some(&repo.id), 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].branches_synced, 1);
    }
}
