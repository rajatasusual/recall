//! Event domain model - Pure business entities
//!
//! Contains event definitions and related types independent of
//! persistence or framework concerns.

/// Full event record with metadata (database representation)
#[derive(Debug, Clone)]
pub struct EventRecord {
    pub id: String,
    pub timestamp: i64,
    pub source: String,
    pub payload_type: String,
    pub payload_data: String,
    pub window_title: Option<String>,
    pub source_app: Option<String>,
    pub content_hash: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
}

