use crate::core::{Event, EventPayload, EventSource};
use crate::storage::EventWriter;
use arboard::Clipboard;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{error, warn};
use xxhash_rust::xxh3::xxh3_64;

const CLIPBOARD_POLL_INTERVAL_MS: u64 = 150;
const MAX_CLIPBOARD_PAYLOAD_BYTES: usize = 50 * 1024;
const CLIPBOARD_PREVIEW_CHARS: usize = 2048;

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

pub async fn start_clipboard_watcher(
    writer: Arc<EventWriter>,
    interval_ms: u64,
) {
    let poll_interval =
        Duration::from_millis(interval_ms.max(CLIPBOARD_POLL_INTERVAL_MS));

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
            match clipboard.get_text() {
                Ok(contents) => {
                    let trimmed = contents.trim();

                    if trimmed.is_empty() {
                        thread::sleep(poll_interval);
                        continue;
                    }

                    let content_hash = xxh3_64(trimmed.as_bytes());

                    if last_hash == Some(content_hash) {
                        thread::sleep(poll_interval);
                        continue;
                    }

                    let (stored_content, is_truncated) =
                        if trimmed.as_bytes().len() > MAX_CLIPBOARD_PAYLOAD_BYTES {
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

                    let hash_string = format!("{content_hash:016x}");

                    let (source_app, window_title) =
                        get_active_app_and_window();

                    match writer.content_exists(&hash_string) {
                        Ok(true) => {
                            last_hash = Some(content_hash);
                        }

                        Ok(false) => {
                            let event = Event::new(
                                EventSource::Clipboard,
                                EventPayload::ClipboardText {
                                    content: stored_content,
                                    is_truncated,
                                    content_hash: hash_string.clone(),
                                },
                            )
                            .with_context(window_title, source_app);

                            if let Err(err) = writer.write_event(event) {
                                warn!(
                                    "clipboard_event_dropped: reason={} content_hash={}",
                                    err,
                                    hash_string
                                );
                            }

                            last_hash = Some(content_hash);
                        }

                        Err(err) => {
                            warn!(
                                "clipboard_dedup_check_failed: {}",
                                err
                            );

                            let event = Event::new(
                                EventSource::Clipboard,
                                EventPayload::ClipboardText {
                                    content: stored_content,
                                    is_truncated,
                                    content_hash: hash_string.clone(),
                                },
                            )
                            .with_context(window_title, source_app);

                            if let Err(write_err) = writer.write_event(event) {
                                warn!(
                                    "clipboard_event_dropped: reason={} content_hash={}",
                                    write_err,
                                    hash_string
                                );
                            }

                            last_hash = Some(content_hash);
                        }
                    }
                }

                Err(err) => {
                    let msg = err.to_string();

                    if !(msg.contains("NSPasteboard#types")
                        || msg.contains("NSPasteboardTypeString"))
                    {
                        error!(
                            "clipboard_watch_error: failed to read clipboard: {}",
                            err
                        );
                    }
                }
            }

            thread::sleep(poll_interval);
        }
    });
}