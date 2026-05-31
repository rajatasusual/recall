use crate::core::{Event, EventPayload, EventSource};
use crate::storage::EventWriter;
use arboard::Clipboard;
use base64::Engine;
use image::imageops::FilterType;
use image::ImageOutputFormat;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{error, warn};
use xxhash_rust::xxh3::xxh3_64;

const CLIPBOARD_POLL_INTERVAL_MS: u64 = 150;
const MAX_CLIPBOARD_PAYLOAD_BYTES: usize = 50 * 1024;
const CLIPBOARD_PREVIEW_CHARS: usize = 2048;
const PREVIEW_MAX_DIM: u32 = 512;

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

    fn to_event(&self, source_app: Option<String>, window_title: Option<String>) -> Event {
        match self {
            Self::Text {
                hash,
                content,
                is_truncated,
            } => Event::new(
                EventSource::Clipboard,
                EventPayload::ClipboardText {
                    content: content.clone(),
                    is_truncated: *is_truncated,
                    content_hash: hash.clone(),
                },
            )
            .with_context(window_title, source_app),
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
            )
            .with_context(window_title, source_app),
        }
    }
}

/// Get the current active application name (macOS-specific using system command)
#[cfg(target_os = "macos")]
fn get_active_app_and_window() -> (Option<String>, Option<String>) {
    use std::process::Command;

    let app_name = Command::new("osascript")
        .arg("-e")
        .arg(
            "tell application \"System Events\" \
             to return name of first application process whose frontmost is true",
        )
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty());

    let window_title = Command::new("osascript")
        .arg("-e")
        .arg(
            "tell application \"System Events\" \
             to tell first application process whose frontmost is true \
             to try
                return value of attribute \"AXTitle\" of front window
             on error
                return \"\"
             end try",
        )
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty());

    (app_name, window_title)
}

/// Fallback for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
fn get_active_app_and_window() -> (Option<String>, Option<String>) {
    (None, None)
}

/// Process text from clipboard into ClipboardContent
fn process_text(text: &str) -> Option<ClipboardContent> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.starts_with("clipboard_") {
        return None;
    }

    let content_hash = xxh3_64(trimmed.as_bytes());
    let hash_string = format!("{content_hash:016x}");

    let (stored_content, is_truncated) = if trimmed.as_bytes().len() > MAX_CLIPBOARD_PAYLOAD_BYTES {
        (
            trimmed
                .chars()
                .take(CLIPBOARD_PREVIEW_CHARS)
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
fn process_image(image: arboard::ImageData) -> Option<ClipboardContent> {
    tracing::debug!(
        "processing image: {}x{}, {} bytes",
        image.width,
        image.height,
        image.bytes.len()
    );

    let expected_len = image.width
        .checked_mul(image.height)?
        .checked_mul(4)?;

    if image.bytes.len() != expected_len {
        tracing::warn!(
            "invalid image size: expected {} bytes, got {}",
            expected_len,
            image.bytes.len()
        );
        return None;
    }

    let buf = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
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
        PREVIEW_MAX_DIM,
        PREVIEW_MAX_DIM,
        FilterType::Lanczos3,
    );

    let mut preview_buf = Vec::new();

    preview
        .write_to(
            &mut std::io::Cursor::new(&mut preview_buf),
            ImageOutputFormat::Png,
        )
        .ok()?;

    let preview_b64 =
        base64::engine::general_purpose::STANDARD.encode(&preview_buf);

    let preview_url =
        format!("data:image/png;base64,{preview_b64}");

    let content_hash =
        format!("{:016x}", xxh3_64(&png_bytes));

    Some(ClipboardContent::Image {
        hash: content_hash,
        png_bytes,
        preview_url,
    })
}
/// Emit clipboard content to storage, handling deduplication
fn emit_content(
    content: ClipboardContent,
    writer: &Arc<EventWriter>,
    source_app: Option<String>,
    window_title: Option<String>,
) {
    let content_hash = content.hash().to_string();

    match writer.content_exists(&content_hash) {
        Ok(true) => {
            // Already stored, skip
        }
        Ok(false) | Err(_) => {
            // Either not found or dedup check failed; emit the event
            let event = content.to_event(source_app, window_title);
            if let Err(err) = writer.write_event(event) {
                warn!(
                    "clipboard_event_dropped: reason={} content_hash={}",
                    err, content_hash
                );
            }
        }
    }
}

pub async fn start_clipboard_watcher(writer: Arc<EventWriter>, interval_ms: u64) {
    let poll_interval = Duration::from_millis(interval_ms.max(CLIPBOARD_POLL_INTERVAL_MS));

    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(err) => {
                error!(
                    "clipboard_watch_error: failed to initialize clipboard: {}",
                    err
                );
                return;
            }
        };

        let mut last_hash: Option<u64> = None;

        loop {
            let (source_app, window_title) = get_active_app_and_window();

            // Try text first
            match clipboard.get_text() {
                Ok(contents) => {
                    if let Some(content) = process_text(&contents) {
                        let hash = xxh3_64(contents.trim().as_bytes());

                        if last_hash != Some(hash) {
                            emit_content(
                                content,
                                &writer,
                                source_app.clone(),
                                window_title.clone(),
                            );
                            last_hash = Some(hash);
                        }
                    }
                }

                Err(_) => {
                    // Text unavailable; clipboard may contain an image.
                    match clipboard.get_image() {
                        Ok(image) => {
                            tracing::debug!(
                                "clipboard image detected: {}x{} ({} bytes)",
                                image.width,
                                image.height,
                                image.bytes.len()
                            );

                            if let Some(content) = process_image(image) {
                                let image_hash =
                                    u64::from_str_radix(content.hash(), 16).unwrap_or(0);

                                if last_hash != Some(image_hash) {
                                    emit_content(
                                        content,
                                        &writer,
                                        source_app.clone(),
                                        window_title.clone(),
                                    );

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
                                error!("clipboard_watch_error: failed to read clipboard: {}", err);
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
