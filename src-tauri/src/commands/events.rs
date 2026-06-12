use crate::api::{ClipboardOutput, EventService};
use crate::domain::ClipboardFormat;
use crate::error::AppError;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Get events with optional filters
#[tauri::command]
pub fn get_events(
    pinned_only: Option<bool>,
    source_app: Option<String>,
    classification: Option<String>,
    query: Option<String>,
    service: State<'_, EventService>,
) -> Result<serde_json::Value, String> {
    service
        .get_events(
            pinned_only,
            source_app.as_deref(),
            classification.as_deref(),
            query.as_deref(),
        )
        .map(|events| serde_json::json!(events))
        .map_err(|e| serde_json::to_string(&e).unwrap_or_else(|_| "Unknown error".to_string()))
}

/// Get the newest events, ignoring pin ordering.
#[tauri::command]
pub fn get_recent_events(
    limit: Option<usize>,
    service: State<'_, EventService>,
) -> Result<serde_json::Value, String> {
    service
        .get_recent_events(limit.unwrap_or(10))
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

/// Copy an event to the clipboard in the selected format.
#[tauri::command]
pub fn copy_event_to_clipboard(
    event_id: String,
    format: Option<ClipboardFormat>,
    service: State<'_, EventService>,
    app: AppHandle,
) -> Result<(), String> {
    let format = format.unwrap_or(ClipboardFormat::Original);
    match service
        .clipboard_output(&event_id, format)
        .map_err(serialize_error)?
    {
        ClipboardOutput::Image(bytes) => {
            let image = image::load_from_memory(&bytes).map_err(|err| {
                serialize_error(AppError {
                    code: "IMAGE_ERROR".to_string(),
                    message: "Failed to decode image clip".to_string(),
                    details: Some(err.to_string()),
                })
            })?;
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            let clipboard_image = tauri::image::Image::new_owned(rgba.into_raw(), width, height);

            app.clipboard()
                .write_image(&clipboard_image)
                .map_err(|err| {
                    serialize_error(AppError {
                        code: "CLIPBOARD_ERROR".to_string(),
                        message: "Failed to write image to clipboard".to_string(),
                        details: Some(err.to_string()),
                    })
                })?;
        }
        ClipboardOutput::Text(text) => app.clipboard().write_text(text).map_err(|err| {
            serialize_error(AppError {
                code: "CLIPBOARD_ERROR".to_string(),
                message: "Failed to write text to clipboard".to_string(),
                details: Some(err.to_string()),
            })
        })?,
    }

    Ok(())
}

/// Hide the quick overlay without coupling it to clipboard writes.
#[tauri::command]
pub fn hide_quick_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("quick_overlay") {
        window.hide().map_err(|err| {
            serialize_error(AppError {
                code: "WINDOW_ERROR".to_string(),
                message: "Failed to hide quick overlay".to_string(),
                details: Some(err.to_string()),
            })
        })?;
    }

    Ok(())
}

fn serialize_error(error: AppError) -> String {
    serde_json::to_string(&error).unwrap_or_else(|_| "Unknown error".to_string())
}
