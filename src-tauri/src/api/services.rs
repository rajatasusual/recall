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
    ) -> AppResult<Vec<Value>> {
        self.db
            .get_events(pinned_only, source_app)
            .map_err(|e| e.into())
            .map(|records| {
                records
                    .into_iter()
                    .map(|rec| self.format_event_record(rec))
                    .collect()
            })
    }

    /// Get all events (backwards compatibility)
    pub fn get_all_events(&self) -> AppResult<Vec<Value>> {
        self.db
            .get_all_events_full()
            .map_err(|e| e.into())
            .map(|records| {
                records
                    .into_iter()
                    .map(|rec| self.format_event_record(rec))
                    .collect()
            })
    }

    /// Get events within a timestamp range
    pub fn get_events_by_timestamp_range(
        &self,
        start_ms: i64,
        end_ms: i64,
    ) -> AppResult<Vec<Value>> {
        self.db
            .get_events_by_timestamp_range(start_ms, end_ms)
            .map_err(|e| e.into())
            .map(|records| {
                records
                    .into_iter()
                    .map(|rec| self.format_event_record(rec))
                    .collect()
            })
    }

    /// Get all pinned events
    pub fn get_pinned_events(&self) -> AppResult<Vec<Value>> {
        self.db
            .get_pinned_events()
            .map_err(|e| e.into())
            .map(|records| {
                records
                    .into_iter()
                    .map(|rec| self.format_event_record(rec))
                    .collect()
            })
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

    /// Get total count of events
    pub fn count_events(&self) -> AppResult<i64> {
        self.db.count_events().map_err(|e| e.into())
    }

    /// Format an EventRecord as JSON for frontend consumption
    fn format_event_record(&self, event: EventRecord) -> Value {
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
        })
    }
}