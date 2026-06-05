use crate::domain::EventRecord;
use crate::error::AppResult;
use crate::persistence::Database;
use serde_json::{json, Value};
use std::sync::Arc;

/// Service layer for event operations
/// Encapsulates business logic and reduces boilerplate in command handlers
#[derive(Clone)]
pub struct EventService {
    db: Arc<Database>,
}

impl EventService {
    /// Create a new event service with a database instance
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get events with optional filters
    pub fn get_events(
        &self,
        pinned_only: Option<bool>,
        source_app: Option<&str>,
        classification: Option<&str>,
        query: Option<&str>,
    ) -> AppResult<Vec<Value>> {
        self.db
            .get_events(pinned_only, source_app, classification, query)
            .map_err(|e| e.into())
            .map(|records| records.into_iter().map(format_event_record).collect())
    }

    /// Pin an event by ID
    pub fn pin_event(&self, event_id: &str) -> AppResult<()> {
        self.db.pin_event(event_id).map_err(|e| e.into())
    }

    /// Unpin an event by ID
    pub fn unpin_event(&self, event_id: &str) -> AppResult<()> {
        self.db.unpin_event(event_id).map_err(|e| e.into())
    }

    /// Delete an event by ID
    pub fn delete_event(&self, event_id: &str) -> AppResult<()> {
        self.db.delete_event(event_id).map_err(|e| e.into())
    }

    /// Delete all unpinned events
    pub fn delete_all_events(&self) -> AppResult<()> {
        self.db.delete_all_events().map_err(|e| e.into())
    }
}

/// Format an EventRecord as JSON for frontend consumption
pub fn format_event_record(event: EventRecord) -> Value {
    json!({
        "id": event.id,
        "timestamp": event.timestamp,
        "source": event.source,
        "payload_type": event.payload_type,
        "payload": serde_json::from_str::<Value>(&event.payload_data)
            .unwrap_or_else(|_| json!({})),
        "window_title": event.window_title,
        "source_app": event.source_app,
        "content_hash": event.content_hash,
        "pinned": event.pinned,
        "created_at": event.created_at,
        "classification": event.classification,
    })
}
