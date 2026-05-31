use rusqlite::Connection;
use crate::storage::StorageResult;

/// Initialize database schema
pub fn init(conn: &Connection) -> StorageResult<()> {
    // Create events table
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            source TEXT NOT NULL,
            payload_type TEXT NOT NULL,
            payload_data TEXT NOT NULL,
            window_title TEXT,
            source_app TEXT,
            content_hash TEXT,
            pinned INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL
        );
        
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_events_payload_type ON events(payload_type);
        CREATE INDEX IF NOT EXISTS idx_events_source ON events(source);
        CREATE INDEX IF NOT EXISTS idx_events_content_hash ON events(content_hash);
        CREATE INDEX IF NOT EXISTS idx_events_pinned ON events(pinned);
        CREATE INDEX IF NOT EXISTS idx_events_source_app ON events(source_app);
        "#,
    )?;

    // Table to store binary blobs (images, attachments) referenced by content_hash
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS blobs (
            content_hash TEXT PRIMARY KEY,
            mime TEXT NOT NULL,
            data BLOB NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_blobs_content_hash ON blobs(content_hash);
        "#,
    )?;

    // Create edges table for graph relationships
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS edges (
            from_id TEXT NOT NULL,
            to_id TEXT NOT NULL,
            relation_type TEXT NOT NULL,
            PRIMARY KEY (from_id, to_id, relation_type),
            FOREIGN KEY (from_id) REFERENCES events(id) ON DELETE CASCADE,
            FOREIGN KEY (to_id) REFERENCES events(id) ON DELETE CASCADE
        );
        
        CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
        CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
        CREATE INDEX IF NOT EXISTS idx_edges_type ON edges(relation_type);
        "#,
    )?;

    // Create a metadata table for tracking state
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_schema_init() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(init(&conn).is_ok());
        
        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert!(tables.contains(&"events".to_string()));
        assert!(tables.contains(&"edges".to_string()));
        assert!(tables.contains(&"metadata".to_string()));
    }
}
