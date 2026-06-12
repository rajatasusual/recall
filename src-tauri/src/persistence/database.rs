use crate::config::StorageConfig;
use crate::domain::EventRecord;
use crate::persistence::{schema, StorageResult};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Thread-safe database connection wrapper
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create or open a database at the given path
    pub fn open(path: impl AsRef<Path>, config: StorageConfig) -> StorageResult<Self> {
        let conn = Connection::open(path)?;
        let mut pragmas = String::new();

        if config.enable_wal {
            pragmas.push_str(
                r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            "#,
            );
        }

        pragmas.push_str(&format!("PRAGMA cache_size = {};", config.cache_size));
        conn.execute_batch(&pragmas)?;

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

    pub fn drop_database(&self) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DROP TABLE IF EXISTS events", [])?;
        conn.execute("DROP TABLE IF EXISTS blobs", [])?;
        conn.execute("DROP TABLE IF EXISTS edges", [])?;
        conn.execute("DROP TABLE IF EXISTS metadata", [])?;
        Ok(())
    }

    /// Execute a query returning number of rows affected
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> StorageResult<usize> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(sql, params)?;
        Ok(affected)
    }

    /// Insert an event into the database
    pub fn insert_event(&self, event: &EventRecord) -> StorageResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO events (
            id,
            timestamp,
            source,
            payload_type,
            payload_data,
            window_title,
            source_app,
            content_hash,
            pinned,
            created_at,
            classification
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &event.id,
                event.timestamp,
                &event.source,
                &event.payload_type,
                &event.payload_data,
                event.window_title.as_deref(),
                event.source_app.as_deref(),
                event.content_hash.as_deref(),
                event.pinned,
                event.created_at,
                &event.classification,
            ],
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

    /// Get events with optional filters (pinned_only, source_app)
    pub fn get_events(
        &self,
        pinned_only: Option<bool>,
        source_app: Option<&str>,
        classification: Option<&str>,
        query: Option<&str>,
    ) -> StorageResult<Vec<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from("SELECT id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at, classification FROM events");
        let mut param_values: Vec<String> = Vec::new();
        let mut where_clauses: Vec<String> = Vec::new();

        if let Some(true) = pinned_only {
            where_clauses.push("pinned = 1".to_string());
        }

        if let Some(app) = source_app {
            where_clauses.push("source_app = ?".to_string());
            param_values.push(app.to_string());
        }

        if let Some(class) = classification {
            where_clauses.push("classification = ?".to_string());
            param_values.push(class.to_string());
        }

        if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
            for term in query.split_whitespace() {
                let like_term = format!("%{}%", escape_like_term(term));
                where_clauses.push(
                    "(payload_data LIKE ? ESCAPE '\\' \
                    OR source_app LIKE ? ESCAPE '\\' \
                    OR window_title LIKE ? ESCAPE '\\' \
                    OR classification LIKE ? ESCAPE '\\' \
                    OR source LIKE ? ESCAPE '\\' \
                    OR strftime('%Y-%m-%d', timestamp / 1000, 'unixepoch', 'localtime') LIKE ? ESCAPE '\\' \
                    OR strftime('%m/%d/%Y', timestamp / 1000, 'unixepoch', 'localtime') LIKE ? ESCAPE '\\')"
                        .to_string(),
                );
                param_values.extend(std::iter::repeat_n(like_term, 7));
            }
        }

        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }

        sql.push_str(" ORDER BY pinned DESC, timestamp DESC LIMIT 1000");

        let mut stmt = conn.prepare(&sql)?;

        let mut result = Vec::new();
        if param_values.is_empty() {
            let rows = stmt.query_map(rusqlite::params_from_iter(param_values.iter()), |row| {
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
                    classification: row.get(10)?,
                })
            })?;

            for event in rows {
                result.push(event?);
            }
        } else {
            let rows = stmt.query_map(rusqlite::params_from_iter(param_values.iter()), |row| {
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
                    classification: row.get(10)?,
                })
            })?;

            for event in rows {
                result.push(event?);
            }
        }

        Ok(result)
    }

    /// Get the most recent events, ignoring pin ordering.
    pub fn get_recent_events(&self, limit: usize) -> StorageResult<Vec<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        let limit = limit.clamp(1, 100) as i64;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at, classification
             FROM events
             ORDER BY timestamp DESC
             LIMIT ?",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
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
                classification: row.get(10)?,
            })
        })?;

        let mut result = Vec::new();
        for event in rows {
            result.push(event?);
        }

        Ok(result)
    }

    /// Get an event by id.
    pub fn get_event(&self, event_id: &str) -> StorageResult<Option<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, timestamp, source, payload_type, payload_data, window_title, source_app, content_hash, pinned, created_at, classification
             FROM events
             WHERE id = ?",
            params![event_id],
            |row| {
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
                    classification: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
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
}

fn escape_like_term(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
