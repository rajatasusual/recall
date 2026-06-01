use crate::domain::EventRecord;
use crate::persistence::{StorageResult, schema};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Thread-safe database connection wrapper
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create or open a database at the given path
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for crash safety
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
        conn.execute_batch("PRAGMA cache_size = -64000;")?; // 64MB cache

        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Initialize schema on database
    pub fn init_schema(&self) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        schema::init(&conn)?;
        Ok(())
    }

    /// Get a connection for operations
    pub fn get_conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Execute a query returning number of rows affected
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> StorageResult<usize> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(sql, params)?;
        Ok(affected)
    }

    /// Insert an event into the database
    pub fn insert_event(
        &self,
        id: &str,
        timestamp: i64,
        source: &str,
        payload_type: &str,
        payload_data: &str,
        window_title: Option<&str>,
        source_app: Option<&str>,
        content_hash: Option<&str>,
    ) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        conn.execute(
            "INSERT INTO events (id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?)",
            params![id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, now],
        )?;
        Ok(())
    }

    /// Insert a binary blob (e.g., image) referenced by content_hash
    pub fn insert_blob(&self, content_hash: &str, mime: &str, data: &[u8]) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        conn.execute(
            "INSERT OR IGNORE INTO blobs (content_hash, mime, data, created_at) VALUES (?, ?, ?, ?)",
            params![content_hash, mime, data, now],
        )?;

        Ok(())
    }

    /// Insert an edge in the graph
    pub fn insert_edge(
        &self,
        from_id: &str,
        to_id: &str,
        relation_type: &str,
    ) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO edges (from_id, to_id, relation_type) VALUES (?, ?, ?)",
            params![from_id, to_id, relation_type],
        )?;
        Ok(())
    }

    /// Get all events with full metadata ordered by timestamp (descending)
    pub fn get_all_events_full(&self) -> StorageResult<Vec<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at FROM events ORDER BY pinned DESC, timestamp DESC LIMIT 1000"
        )?;

        let events = stmt.query_map([], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source: row.get(2)?,
                payload_type: row.get(3)?,
                payload_data: row.get(4)?,
                window_title: row.get(5)?,
                source_app: row.get(6)?,
                content_hash: row.get(7)?,
                pinned: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
            })
        })?;

        let mut result = Vec::new();
        for event in events {
            result.push(event?);
        }
        Ok(result)
    }

    /// Get events with optional filters (pinned_only, source_app)
    pub fn get_events(
        &self,
        pinned_only: Option<bool>,
        source_app: Option<&str>,
    ) -> StorageResult<Vec<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from("SELECT id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at FROM events");
        let mut params_vec: Vec<&dyn rusqlite::ToSql> = Vec::new();
        let mut param_values: Vec<String> = Vec::new();
        let mut where_clauses: Vec<String> = Vec::new();

        if let Some(true) = pinned_only {
            where_clauses.push("pinned = 1".to_string());
        }

        if let Some(app) = source_app {
            where_clauses.push("source_app = ?".to_string());
            param_values.push(app.to_string());
            params_vec.push(param_values.last().unwrap());
        }

        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }

        sql.push_str(" ORDER BY pinned DESC, timestamp DESC LIMIT 1000");

        let mut stmt = conn.prepare(&sql)?;

        let mut result = Vec::new();
        if params_vec.is_empty() {
            let rows = stmt.query_map([], |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    source: row.get(2)?,
                    payload_type: row.get(3)?,
                    payload_data: row.get(4)?,
                    window_title: row.get(5)?,
                    source_app: row.get(6)?,
                    content_hash: row.get(7)?,
                    pinned: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                })
            })?;

            for event in rows {
                result.push(event?);
            }
        } else {
            let rows = stmt.query_map(params_vec.as_slice(), |row| {
                Ok(EventRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    source: row.get(2)?,
                    payload_type: row.get(3)?,
                    payload_data: row.get(4)?,
                    window_title: row.get(5)?,
                    source_app: row.get(6)?,
                    content_hash: row.get(7)?,
                    pinned: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                })
            })?;

            for event in rows {
                result.push(event?);
            }
        }

        Ok(result)
    }

    /// Delete an event by id
    pub fn delete_event(&self, event_id: &str) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM events WHERE id = ?", params![event_id])?;
        Ok(())
    }

    /// Delete all events (use with caution)
    pub fn delete_all_events(&self) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM events WHERE pinned = 0", [])?;
        Ok(())
    }

    /// Get events by timestamp range with full metadata
    pub fn get_events_by_timestamp_range(
        &self,
        start_ms: i64,
        end_ms: i64,
    ) -> StorageResult<Vec<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at FROM events 
             WHERE timestamp >= ? AND timestamp <= ? 
             ORDER BY timestamp DESC LIMIT 1000"
        )?;

        let events = stmt.query_map(params![start_ms, end_ms], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source: row.get(2)?,
                payload_type: row.get(3)?,
                payload_data: row.get(4)?,
                window_title: row.get(5)?,
                source_app: row.get(6)?,
                content_hash: row.get(7)?,
                pinned: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
            })
        })?;

        let mut result = Vec::new();
        for event in events {
            result.push(event?);
        }
        Ok(result)
    }

    /// Get all events ordered by timestamp (descending)
    pub fn get_all_events(&self) -> StorageResult<Vec<(String, i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, payload_data FROM events ORDER BY timestamp DESC LIMIT 1000",
        )?;

        let events = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut result = Vec::new();
        for event in events {
            result.push(event?);
        }
        Ok(result)
    }

    /// Count total events
    pub fn count_events(&self) -> StorageResult<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Check if content hash already exists (for deduplication)
    pub fn content_exists(&self, content_hash: &str) -> StorageResult<bool> {
        let conn = self.conn.lock().unwrap();
        // check both events and blobs for existing content
        let exists_events: i64 = conn.query_row(
            "SELECT COUNT(*) FROM events WHERE content_hash = ?",
            params![content_hash],
            |row| row.get(0),
        )?;

        if exists_events > 0 {
            return Ok(true);
        }

        let exists_blobs: i64 = conn.query_row(
            "SELECT COUNT(*) FROM blobs WHERE content_hash = ?",
            params![content_hash],
            |row| row.get(0),
        )?;

        Ok(exists_blobs > 0)
    }

    /// Retrieve a blob by content_hash
    pub fn get_blob(&self, content_hash: &str) -> StorageResult<Option<(String, Vec<u8>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT mime, data FROM blobs WHERE content_hash = ?")?;
        let mut rows = stmt.query(params![content_hash])?;

        if let Some(row) = rows.next()? {
            let mime: String = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            Ok(Some((mime, data)))
        } else {
            Ok(None)
        }
    }

    /// Pin an event
    pub fn pin_event(&self, event_id: &str) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE events SET pinned = 1 WHERE id = ?",
            params![event_id],
        )?;
        Ok(())
    }

    /// Unpin an event
    pub fn unpin_event(&self, event_id: &str) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE events SET pinned = 0 WHERE id = ?",
            params![event_id],
        )?;
        Ok(())
    }

    /// Get pinned events
    pub fn get_pinned_events(&self) -> StorageResult<Vec<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at FROM events 
             WHERE pinned = 1 
             ORDER BY timestamp DESC"
        )?;

        let events = stmt.query_map([], |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source: row.get(2)?,
                payload_type: row.get(3)?,
                payload_data: row.get(4)?,
                window_title: row.get(5)?,
                source_app: row.get(6)?,
                content_hash: row.get(7)?,
                pinned: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
            })
        })?;

        let mut result = Vec::new();
        for event in events {
            result.push(event?);
        }
        Ok(result)
    }
}