use rusqlite::Connection;

use crate::schema;

/// Run all pending migrations.
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(schema::CREATE_SCHEMA_VERSION)?;

    let current = get_version(conn)?;

    if current < 1 {
        migrate_v1(conn)?;
    }

    if current < 2 {
        migrate_v2(conn)?;
    }

    Ok(())
}

fn get_version(conn: &Connection) -> anyhow::Result<i64> {
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(version)
}

fn set_version(conn: &Connection, version: i64) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?1, datetime('now'))",
        [version],
    )?;
    Ok(())
}

/// Migration v2: add upstream_clone_url column to repos.
/// Idempotent — skips the ALTER if the column already exists (e.g. fresh DB
/// created from the updated CREATE_REPOS statement that includes the column).
fn migrate_v2(conn: &Connection) -> anyhow::Result<()> {
    tracing::info!("applying migration v2: upstream_clone_url column");
    let has_column: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('repos') WHERE name='upstream_clone_url'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !has_column {
        conn.execute_batch("ALTER TABLE repos ADD COLUMN upstream_clone_url TEXT")?;
    }
    set_version(conn, 2)?;
    Ok(())
}

/// Migration v1: create all initial tables.
fn migrate_v1(conn: &Connection) -> anyhow::Result<()> {
    tracing::info!("applying migration v1: initial schema");
    conn.execute_batch(schema::CREATE_HOSTS)?;
    conn.execute_batch(schema::CREATE_REPOS)?;
    conn.execute_batch(schema::CREATE_COLLECTIONS)?;
    conn.execute_batch(schema::CREATE_COLLECTION_MEMBERS)?;
    conn.execute_batch(schema::CREATE_SYNC_LINKS)?;
    conn.execute_batch(schema::CREATE_SYNC_HISTORY)?;
    conn.execute_batch(schema::CREATE_BRANCH_SNAPSHOTS)?;
    set_version(conn, 1)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), 2);
    }
}
