use crate::core::{Event, EventPayload, EventSource};
use crate::storage::{db::EventRecord, Database};
use std::sync::Arc;
use tauri::State;
use xxhash_rust::xxh3::xxh3_64;

/// Helper function to convert EventRecord to JSON
fn event_record_to_json(event: EventRecord) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "id": event.id,
        "timestamp": event.timestamp,
        "source": event.source,
        "payload_type": event.payload_type,
        "payload": serde_json::from_str::<serde_json::Value>(&event.payload_data)
            .unwrap_or_else(|_| serde_json::json!({})),
        "window_title": event.window_title,
        "source_app": event.source_app,
        "content_hash": event.content_hash,
        "pinned": event.pinned,
        "created_at": event.created_at,
    }))
}

/// Get events with optional filters
#[tauri::command]
pub fn get_events(
    pinned_only: Option<bool>,
    source_app: Option<String>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<serde_json::Value>, String> {
    let source_app_ref = source_app.as_deref();
    db.get_events(pinned_only, source_app_ref)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(event_record_to_json)
        .collect()
}

/// Get all events (backwards compatible)
#[tauri::command]
pub fn get_all_events(db: State<'_, Arc<Database>>) -> Result<Vec<serde_json::Value>, String> {
    db.get_all_events_full()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(event_record_to_json)
        .collect()
}

/// Get events within a timestamp range
#[tauri::command]
pub fn get_events_by_timestamp_range(
    start_ms: i64,
    end_ms: i64,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<serde_json::Value>, String> {
    db.get_events_by_timestamp_range(start_ms, end_ms)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(event_record_to_json)
        .collect()
}

/// Delete an event
#[tauri::command]
pub fn delete_event(event_id: String, db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.delete_event(&event_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_all_events(db: State<'_, Arc<Database>>) -> Result<(), String> {
    db.delete_all_events().map_err(|e| e.to_string())
}

/// Get pinned events
#[tauri::command]
pub fn get_pinned_events(db: State<'_, Arc<Database>>) -> Result<Vec<serde_json::Value>, String> {
    db.get_pinned_events()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(event_record_to_json)
        .collect()
}

/// Pin an event
#[tauri::command]
pub fn pin_event(
    event_id: String,
    db: State<'_, Arc<Database>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    db.pin_event(&event_id).map_err(|e| e.to_string())?;
    crate::refresh_tray_menu(&app).map_err(|e| e.to_string())?;
    Ok(())
}

/// Unpin an event
#[tauri::command]
pub fn unpin_event(
    event_id: String,
    db: State<'_, Arc<Database>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    db.unpin_event(&event_id).map_err(|e| e.to_string())?;
    crate::refresh_tray_menu(&app).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get count of all events
#[tauri::command]
pub fn get_event_count(db: State<'_, Arc<Database>>) -> Result<i64, String> {
    db.count_events().map_err(|e| e.to_string())
}

/// Test command: Insert a sample clipboard event
#[tauri::command]
pub fn test_insert_clipboard_event(
    content: String,
    db: State<'_, Arc<Database>>,
) -> Result<String, String> {
    let event = Event::new(
        EventSource::Clipboard,
        EventPayload::ClipboardText {
            content: content.clone(),
            is_truncated: false,
            content_hash: format!("{:x}", xxh3_64(content.as_bytes())),
        },
    );

    let event_id = event.id.clone();
    let content_hash = format!("{:x}", xxh3_64(content.as_bytes()));

    db.insert_event(
        &event.id,
        event.timestamp,
        event.source.as_str(),
        event.payload.payload_type(),
        &serde_json::to_string(&event.payload).map_err(|e| e.to_string())?,
        None,
        None,
        Some(&content_hash),
    )
    .map_err(|e| e.to_string())?;

    Ok(event_id)
}
