use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

/// Unified event model for all system events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub timestamp: i64,
    pub source: EventSource,
    pub payload: EventPayload,
    pub window_title: Option<String>,
    pub source_app: Option<String>,
    pub pinned: bool,
}

impl Event {
    pub fn new(source: EventSource, payload: EventPayload) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().timestamp_millis(),
            source,
            payload,
            window_title: None,
            source_app: None,
            pinned: false,
        }
    }

    pub fn with_context(mut self, window_title: Option<String>, source_app: Option<String>) -> Self {
        self.window_title = window_title;
        self.source_app = source_app;
        self
    }
}

/// Event source classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    Clipboard
}

impl EventSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventSource::Clipboard => "clipboard",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "clipboard" => Some(EventSource::Clipboard),
            _ => None,
        }
    }
}

/// Event payload - source-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    ClipboardText {
        content: String,
        is_truncated: bool,
        content_hash: String,
    },
    ClipboardImage {
        content_hash: String,
        mime: String,
        preview: Option<String>,
        #[serde(skip_serializing)]
        #[serde(default)]
        data: Option<Vec<u8>>,
    },
}

impl EventPayload {
    pub fn payload_type(&self) -> &'static str {
        match self {
            EventPayload::ClipboardText { .. } => "clipboard_text",
            EventPayload::ClipboardImage { .. } => "clipboard_image",
        }
    }
}