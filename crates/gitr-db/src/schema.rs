/// SQL statements for creating the Gitr database schema.

pub const CREATE_SCHEMA_VERSION: &str = "
CREATE TABLE IF NOT EXISTS schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL
)";

pub const CREATE_HOSTS: &str = "
CREATE TABLE IF NOT EXISTS hosts (
    id              TEXT PRIMARY KEY,
    label           TEXT NOT NULL UNIQUE,
    kind            TEXT NOT NULL,
    api_url         TEXT NOT NULL,
    username        TEXT NOT NULL,
    credential_key  TEXT NOT NULL
)";

pub const CREATE_REPOS: &str = "
CREATE TABLE IF NOT EXISTS repos (
    id                  TEXT PRIMARY KEY,
    full_name           TEXT NOT NULL,
    owner               TEXT NOT NULL,
    name                TEXT NOT NULL,
    host_id             TEXT NOT NULL,
    clone_url           TEXT NOT NULL,
    local_path          TEXT,
    is_fork             INTEGER NOT NULL DEFAULT 0,
    upstream_repo_id    TEXT,
    upstream_full_name  TEXT,
    default_branch      TEXT NOT NULL DEFAULT 'main',
    discovery_source    TEXT NOT NULL DEFAULT 'api',
    last_synced_at      TEXT,
    created_at          TEXT NOT NULL,
    FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE,
    FOREIGN KEY (upstream_repo_id) REFERENCES repos(id) ON DELETE SET NULL
)";

pub const CREATE_COLLECTIONS: &str = "
CREATE TABLE IF NOT EXISTS collections (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT
)";

pub const CREATE_COLLECTION_MEMBERS: &str = "
CREATE TABLE IF NOT EXISTS collection_members (
    collection_id   TEXT NOT NULL,
    repo_id         TEXT NOT NULL,
    PRIMARY KEY (collection_id, repo_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE
)";

pub const CREATE_SYNC_LINKS: &str = "
CREATE TABLE IF NOT EXISTS sync_links (
    id              TEXT PRIMARY KEY,
    source_repo_id  TEXT NOT NULL,
    target_repo_id  TEXT NOT NULL,
    direction       TEXT NOT NULL DEFAULT 'pull',
    merge_strategy  TEXT NOT NULL DEFAULT 'ff',
    trigger         TEXT NOT NULL DEFAULT 'manual',
    instructions    TEXT NOT NULL DEFAULT '{}',
    enabled         INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (source_repo_id) REFERENCES repos(id) ON DELETE CASCADE,
    FOREIGN KEY (target_repo_id) REFERENCES repos(id) ON DELETE CASCADE
)";

pub const CREATE_SYNC_HISTORY: &str = "
CREATE TABLE IF NOT EXISTS sync_history (
    id                  TEXT PRIMARY KEY,
    repo_id             TEXT NOT NULL,
    sync_link_id        TEXT,
    branches_synced     INTEGER NOT NULL DEFAULT 0,
    branches_failed     INTEGER NOT NULL DEFAULT 0,
    commits_transferred INTEGER NOT NULL DEFAULT 0,
    status              TEXT NOT NULL,
    errors              TEXT NOT NULL DEFAULT '[]',
    started_at          TEXT NOT NULL,
    finished_at         TEXT NOT NULL,
    FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
    FOREIGN KEY (sync_link_id) REFERENCES sync_links(id) ON DELETE SET NULL
)";

pub const CREATE_BRANCH_SNAPSHOTS: &str = "
CREATE TABLE IF NOT EXISTS branch_snapshots (
    repo_id         TEXT NOT NULL,
    branch          TEXT NOT NULL,
    local_sha       TEXT,
    remote_sha      TEXT,
    upstream_sha    TEXT,
    behind_count    INTEGER NOT NULL DEFAULT 0,
    ahead_count     INTEGER NOT NULL DEFAULT 0,
    updated_at      TEXT NOT NULL,
    PRIMARY KEY (repo_id, branch),
    FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE
)";

/// All table creation statements in order.
pub const ALL_TABLES: &[&str] = &[
    CREATE_SCHEMA_VERSION,
    CREATE_HOSTS,
    CREATE_REPOS,
    CREATE_COLLECTIONS,
    CREATE_COLLECTION_MEMBERS,
    CREATE_SYNC_LINKS,
    CREATE_SYNC_HISTORY,
    CREATE_BRANCH_SNAPSHOTS,
];
