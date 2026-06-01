use crate::api::EventService;
use crate::error::AppError;
use tauri::State;

/// Get events with optional filters
#[tauri::command]
pub fn get_events(
    pinned_only: Option<bool>,
    source_app: Option<String>,
    service: State<'_, EventService>,
) -> Result<serde_json::Value, String> {
    service
        .get_events(pinned_only, source_app.as_deref())
        .map(|events| serde_json::json!(events))
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Get all events (backwards compatible)
#[tauri::command]
pub fn get_all_events(service: State<'_, EventService>) -> Result<serde_json::Value, String> {
    service
        .get_all_events()
        .map(|events| serde_json::json!(events))
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Get events within a timestamp range
#[tauri::command]
pub fn get_events_by_timestamp_range(
    start_ms: i64,
    end_ms: i64,
    service: State<'_, EventService>,
) -> Result<serde_json::Value, String> {
    service
        .get_events_by_timestamp_range(start_ms, end_ms)
        .map(|events| serde_json::json!(events))
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Delete an event
#[tauri::command]
pub fn delete_event(event_id: String, service: State<'_, EventService>) -> Result<(), String> {
    service
        .delete_event(&event_id)
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Delete all unpinned events
#[tauri::command]
pub fn delete_all_events(service: State<'_, EventService>) -> Result<(), String> {
    service
        .delete_all_events()
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Get all pinned events
#[tauri::command]
pub fn get_pinned_events(service: State<'_, EventService>) -> Result<serde_json::Value, String> {
    service
        .get_pinned_events()
        .map(|events| serde_json::json!(events))
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Pin an event
#[tauri::command]
pub fn pin_event(
    event_id: String,
    service: State<'_, EventService>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    service
        .pin_event(&event_id)
        .and_then(|_| {
            crate::refresh_tray_menu(&app).map_err(|e| AppError {
                code: "MENU_ERROR".to_string(),
                message: "Failed to refresh tray menu".to_string(),
                details: Some(e.to_string()),
            })
        })
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Unpin an event
#[tauri::command]
pub fn unpin_event(
    event_id: String,
    service: State<'_, EventService>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    service
        .unpin_event(&event_id)
        .and_then(|_| {
            crate::refresh_tray_menu(&app).map_err(|e| AppError {
                code: "MENU_ERROR".to_string(),
                message: "Failed to refresh tray menu".to_string(),
                details: Some(e.to_string()),
            })
        })
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Get count of all events
#[tauri::command]
pub fn get_event_count(service: State<'_, EventService>) -> Result<i64, String> {
    service
        .count_events()
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}
