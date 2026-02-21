use rusqlite::Connection;

use crate::schema;

/// Run all pending migrations.
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(schema::CREATE_SCHEMA_VERSION)?;

    let current = get_version(conn)?;

    if current < 1 {
        migrate_v1(conn)?;
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
        assert_eq!(get_version(&conn).unwrap(), 1);
    }
}
