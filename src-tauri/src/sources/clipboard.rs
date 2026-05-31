use crate::core::{Event, EventPayload, EventSource};
use crate::storage::EventWriter;
use copypasta::{ClipboardContext, ClipboardProvider};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn};

const CLIPBOARD_POLL_INTERVAL_MS: u64 = 400;
const MAX_CLIPBOARD_PAYLOAD_BYTES: usize = 50 * 1024;
const CLIPBOARD_PREVIEW_CHARS: usize = 2048;

/// Get the current active application name (macOS-specific using system command)
#[cfg(target_os = "macos")]
fn get_active_app_and_window() -> (Option<String>, Option<String>) {
    use std::process::Command;
    // Use AppleScript to get the frontmost app and window title
    let app_name = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to return name of first application process whose frontmost is true")
        .output()
        .ok()
        .and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else { None })
        .filter(|s| !s.is_empty());

    let window_title = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to tell first application process whose frontmost is true to try\n  return value of attribute \"AXTitle\" of front window\non error\n  return \"\"\nend try")
        .output()
        .ok()
        .and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else { None })
        .filter(|s| !s.is_empty());

    (app_name, window_title)
}

/// Get the current active application name and window title (fallback for other platforms)
#[cfg(not(target_os = "macos"))]
fn get_active_app_and_window() -> (Option<String>, Option<String>) {
    (None, None)
}

pub async fn start_clipboard_watcher(writer: Arc<EventWriter>, interval_ms: u64) {
    let interval = Duration::from_millis(interval_ms.max(CLIPBOARD_POLL_INTERVAL_MS));

    let mut clipboard = match ClipboardContext::new() {
        Ok(ctx) => ctx,
        Err(err) => {
            error!(
                "clipboard_watch_error: failed to initialize clipboard: {}",
                err
            );
            return;
        }
    };

    let mut last_hash: Option<String> = None;

    loop {
        match clipboard.get_contents() {
            Ok(contents) => {
                let trimmed = contents.trim().to_string();
                if !trimmed.is_empty() {
                    {
                        let content_hash = format!("{:x}", md5::compute(trimmed.as_bytes()));

                        if last_hash.as_deref() != Some(&content_hash) {
                            let (stored_content, is_truncated) =
                                if trimmed.as_bytes().len() > MAX_CLIPBOARD_PAYLOAD_BYTES {
                                    let preview: String =
                                        trimmed.chars().take(CLIPBOARD_PREVIEW_CHARS).collect();
                                    (preview, true)
                                } else {
                                    (trimmed.clone(), false)
                                };

                            // Capture application context
                            let (source_app, window_title) = get_active_app_and_window();

                            // Check DB deduplication first (persisted duplicates)
                            match writer.content_exists(&content_hash) {
                                Ok(true) => {
                                    // Already stored in DB; update last_hash and skip
                                    last_hash = Some(content_hash);
                                }
                                Ok(false) => {
                                    let event = Event::new(
                                        EventSource::Clipboard,
                                        EventPayload::ClipboardText {
                                            content: stored_content,
                                            is_truncated,
                                            content_hash: content_hash.clone(),
                                        },
                                    )
                                    .with_context(window_title, source_app);

                                    if let Err(err) = writer.write_event(event) {
                                        warn!(
                                            "clipboard_event_dropped: reason={} content_hash={}",
                                            err, content_hash
                                        );
                                    }

                                    last_hash = Some(content_hash);
                                }
                                Err(e) => {
                                    // On DB error, fallback to enqueueing the event
                                    warn!("clipboard_dedup_check_failed: {}", e);
                                    let event = Event::new(
                                        EventSource::Clipboard,
                                        EventPayload::ClipboardText {
                                            content: stored_content,
                                            is_truncated,
                                            content_hash: content_hash.clone(),
                                        },
                                    )
                                    .with_context(window_title, source_app);

                                    if let Err(err) = writer.write_event(event) {
                                        warn!(
                                            "clipboard_event_dropped: reason={} content_hash={}",
                                            err, content_hash
                                        );
                                    }

                                    last_hash = Some(content_hash);
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                // On macOS the pasteboard may not contain a string type for some events
                // (e.g. images or rich types). Treat the specific NSPasteboard message as
                // non-fatal and log at debug level to avoid noisy errors in logs.
                let msg = err.to_string();
                if !(msg.contains("NSPasteboard#types") || msg.contains("NSPasteboardTypeString")) {
                    error!("clipboard_watch_error: failed to read clipboard: {}", err);                    
                }
            }
        }

        sleep(interval).await;
    }
}
