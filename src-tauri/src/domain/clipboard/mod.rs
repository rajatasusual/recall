//! Clipboard monitoring - Domain-level clipboard operations
//!
//! This module handles clipboard monitoring and content processing,
//! independent of persistence details (which are handled in the API/persistence layers).

use crate::api::classification::classify_text;
use crate::config::ClipboardConfig;
use crate::core::Event;
use crate::core::{EventPayload, EventSource};
use crate::domain::EventRecord;
use crate::persistence::EventWriter;
use base64::Engine;
use image::imageops::FilterType;
use image::ImageOutputFormat;
use serde_json::Value;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, EventLoopMessage, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_runtime_wry::Wry;
use xxhash_rust::xxh3::xxh3_64;

/// Result of processing clipboard content
enum ClipboardContent {
    Text {
        hash: String,
        content: String,
        is_truncated: bool,
    },
    Image {
        hash: String,
        png_bytes: Vec<u8>,
        preview_url: String,
    },
}

impl ClipboardContent {
    fn hash(&self) -> &str {
        match self {
            Self::Text { hash, .. } | Self::Image { hash, .. } => hash,
        }
    }

    fn to_event(&self) -> Event {
        match self {
            Self::Text {
                hash,
                content,
                is_truncated,
            } => {
                let classification = classify_text(content).as_str().to_string();

                Event::new(
                    EventSource::Clipboard,
                    EventPayload::ClipboardText {
                        content: content.clone(),
                        is_truncated: *is_truncated,
                        content_hash: hash.clone(),
                        classification,
                    },
                )
            }
            Self::Image {
                hash,
                png_bytes,
                preview_url,
            } => Event::new(
                EventSource::Clipboard,
                EventPayload::ClipboardImage {
                    content_hash: hash.clone(),
                    mime: "image/png".to_string(),
                    preview: Some(preview_url.clone()),
                    data: Some(png_bytes.clone()),
                },
            ),
        }
    }
}

/// Process text from clipboard into ClipboardContent
fn process_text(text: &str, config: &ClipboardConfig) -> Option<ClipboardContent> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.starts_with("clipboard_") {
        return None;
    }

    let content_hash = xxh3_64(trimmed.as_bytes());
    let hash_string = format!("{content_hash:016x}");

    let (stored_content, is_truncated) = if trimmed.len() > config.max_payload_bytes {
        (
            trimmed
                .chars()
                .take(config.preview_chars)
                .collect::<String>(),
            true,
        )
    } else {
        (trimmed.to_owned(), false)
    };

    Some(ClipboardContent::Text {
        hash: hash_string,
        content: stored_content,
        is_truncated,
    })
}

/// Process image from clipboard into ClipboardContent
fn process_image(
    image: tauri::image::Image<'_>,
    config: &ClipboardConfig,
) -> Option<ClipboardContent> {
    tracing::debug!(
        "processing image: {}x{}, {} bytes",
        image.width(),
        image.height(),
        image.rgba().len()
    );

    let expected_len = image.width().checked_mul(image.height())?.checked_mul(4)?;

    if image.rgba().len() != expected_len as usize {
        tracing::warn!(
            "invalid image size: expected {} bytes, got {}",
            expected_len,
            image.rgba().len()
        );
        return None;
    }

    let buf = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
        image.width(),
        image.height(),
        image.rgba().to_vec(),
    )?;

    let dyn_img = image::DynamicImage::ImageRgba8(buf);

    let mut png_bytes = Vec::new();
    dyn_img
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            ImageOutputFormat::Png,
        )
        .ok()?;

    let preview = dyn_img.resize(
        config.preview_max_dim,
        config.preview_max_dim,
        FilterType::Lanczos3,
    );
    let mut preview_buf = Vec::new();
    preview
        .write_to(
            &mut std::io::Cursor::new(&mut preview_buf),
            ImageOutputFormat::Png,
        )
        .ok()?;

    let preview_b64 = base64::engine::general_purpose::STANDARD.encode(&preview_buf);
    let preview_url = format!("data:image/png;base64,{preview_b64}");
    let content_hash = format!("{:016x}", xxh3_64(&png_bytes));

    Some(ClipboardContent::Image {
        hash: content_hash,
        png_bytes,
        preview_url,
    })
}

/// Emit clipboard content to storage, handling deduplication
fn content_to_db(content: ClipboardContent, writer: &Arc<EventWriter>) {
    let content_hash = content.hash().to_string();

    match writer.content_exists(&content_hash) {
        Ok(true) => {
            // Already stored, skip
        }
        Ok(false) | Err(_) => {
            // Either not found or dedup check failed; emit the event
            let event = content.to_event();
            if let Err(err) = writer.write_event(event) {
                tracing::warn!(
                    "clipboard_event_dropped: reason={} content_hash={}",
                    err,
                    content_hash
                );
            }
        }
    }
}

pub async fn start_clipboard_watcher(
    writer: Arc<EventWriter>,
    app_handle: AppHandle<Wry<EventLoopMessage>>,
    config: ClipboardConfig,
) {
    let poll_interval = Duration::from_millis(config.poll_interval_ms);

    thread::spawn(move || {
        let clipboard = app_handle.clipboard();
        let mut last_hash: Option<String> = None;

        loop {
            // Try text first
            match clipboard.read_text() {
                Ok(contents) => {
                    if let Some(content) = process_text(&contents, &config) {
                        let hash = content.hash().to_string();

                        if last_hash.as_deref() != Some(&hash) {
                            content_to_db(content, &writer);
                            last_hash = Some(hash);
                        }
                    }
                }
                Err(_) => {
                    // Text unavailable; clipboard may contain an image.
                    match clipboard.read_image() {
                        Ok(image) => {
                            tracing::debug!(
                                "clipboard image detected: {}x{} ({} bytes)",
                                image.width(),
                                image.height(),
                                image.rgba().len()
                            );

                            if let Some(content) = process_image(image, &config) {
                                let image_hash = content.hash().to_string();

                                if last_hash.as_deref() != Some(&image_hash) {
                                    content_to_db(content, &writer);

                                    last_hash = Some(image_hash);
                                }
                            } else {
                                tracing::warn!("clipboard image could not be processed");
                            }
                        }
                        Err(err) => {
                            // Neither text nor image available.
                            let msg = err.to_string();

                            if !msg.contains("not available in the requested format")
                                && !msg.contains("NSPasteboardTypeString")
                                && !msg.contains("NSPasteboard#types")
                            {
                                tracing::error!(
                                    "clipboard_watch_error: failed to read clipboard: {}",
                                    err
                                );
                            }

                            last_hash = None;
                        }
                    }
                }
            }

            thread::sleep(poll_interval);
        }
    });
}

/// Helper function to copy an event's content to clipboard (handles both text and images)
pub fn copy_event_to_clipboard<R: Runtime>(
    event: &EventRecord,
    db: &std::sync::Arc<crate::persistence::Database>,
    app: &AppHandle<R>,
) {
    let clipboard = app.clipboard();
    let is_image = event.payload_type.contains("image");

    if is_image {
        if let Ok(payload) = serde_json::from_str::<Value>(&event.payload_data) {
            if let Some(content_hash) = payload.get("content_hash").and_then(|v| v.as_str()) {
                if let Ok(Some((_mime, data))) = db.get_blob(content_hash) {
                    match image::load_from_memory(&data) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            let (width, height) = rgba.dimensions();
                            let image =
                                tauri::image::Image::new_owned(rgba.into_raw(), width, height);

                            if let Err(e) = clipboard.write_image(&image) {
                                tracing::error!("Failed to copy image to clipboard: {}", e);
                            } else {
                                tracing::info!("Image copied to clipboard");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to decode image bytes: {}", e);
                        }
                    }
                } else {
                    tracing::warn!("Image blob not found for content_hash: {}", content_hash);
                }
            }
        }
    } else {
        if let Ok(payload) = serde_json::from_str::<Value>(&event.payload_data) {
            let text_content = payload
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| event.payload_data.clone());

            if let Err(e) = clipboard.write_text(text_content) {
                tracing::error!("Failed to copy text to clipboard: {}", e);
            } else {
                tracing::info!("Text copied to clipboard successfully");
            }
        } else {
            tracing::error!("Failed to parse event payload as JSON");
        }
    }
}
