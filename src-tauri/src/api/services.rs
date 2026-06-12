use crate::domain::clipboard_format::{apply_text_format, ClipboardFormat};
use crate::domain::EventRecord;
use crate::error::{AppError, AppResult};
use crate::persistence::Database;
use serde_json::{json, Value};
use std::sync::Arc;

pub enum ClipboardOutput {
    Text(String),
    Image(Vec<u8>),
}

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

    /// Get the newest events without pin ordering.
    pub fn get_recent_events(&self, limit: usize) -> AppResult<Vec<Value>> {
        self.db
            .get_recent_events(limit)
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

    /// Load an event and return the content that should be written to the clipboard.
    pub fn clipboard_output(
        &self,
        event_id: &str,
        format: ClipboardFormat,
    ) -> AppResult<ClipboardOutput> {
        let event = self
            .db
            .get_event(event_id)
            .map_err(AppError::from)?
            .ok_or_else(|| AppError {
                code: "NOT_FOUND".to_string(),
                message: "Clipboard event not found".to_string(),
                details: Some(event_id.to_string()),
            })?;

        let payload =
            serde_json::from_str::<Value>(&event.payload_data).map_err(|err| AppError {
                code: "SERIALIZATION_ERROR".to_string(),
                message: "Failed to parse clipboard event payload".to_string(),
                details: Some(err.to_string()),
            })?;

        if event.payload_type.contains("image") {
            if format != ClipboardFormat::Original {
                return Err(AppError {
                    code: "UNSUPPORTED_FORMAT".to_string(),
                    message: "Image clips only support the original clipboard format".to_string(),
                    details: None,
                });
            }

            let content_hash = payload
                .get("content_hash")
                .and_then(|value| value.as_str())
                .or(event.content_hash.as_deref())
                .ok_or_else(|| AppError {
                    code: "MISSING_IMAGE".to_string(),
                    message: "Image clip is missing its content hash".to_string(),
                    details: Some(event.id.clone()),
                })?;

            let (_mime, bytes) = self
                .db
                .get_blob(content_hash)
                .map_err(AppError::from)?
                .ok_or_else(|| AppError {
                    code: "MISSING_IMAGE".to_string(),
                    message: "Image blob not found".to_string(),
                    details: Some(content_hash.to_string()),
                })?;

            return Ok(ClipboardOutput::Image(bytes));
        }

        let content = payload
            .get("content")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or(event.payload_data);

        Ok(ClipboardOutput::Text(apply_text_format(&content, format)))
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
