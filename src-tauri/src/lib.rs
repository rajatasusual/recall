use arboard::{Clipboard, ImageData};
use serde_json::Value;
use tauri::{
    tray::TrayIconBuilder, Emitter, EventLoopMessage, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_window_state::{Builder as WindowBuilder, StateFlags};
use tracing_subscriber;

mod commands;
pub mod core;
mod sources;
pub mod storage;

type AppMenu = tauri::menu::Menu<tauri_runtime_wry::Wry<EventLoopMessage>>;
type AppTrayIcon = tauri::tray::TrayIcon<tauri_runtime_wry::Wry<EventLoopMessage>>;

#[derive(Clone, serde::Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
}

/// Helper function to copy an event's content to clipboard (handles both text and images)
fn copy_event_to_clipboard(
    event: &storage::db::EventRecord,
    db: &std::sync::Arc<storage::Database>,
) {
    // Determine if this is an image or text
    let is_image = event.payload_type.contains("image");

    if is_image {
        // For images, extract content_hash and fetch the binary data from blobs
        if let Ok(payload) = serde_json::from_str::<Value>(&event.payload_data) {
            if let Some(content_hash) = payload.get("content_hash").and_then(|v| v.as_str()) {
                if let Ok(Some((_mime, data))) = db.get_blob(content_hash) {
                    match Clipboard::new() {
                        Ok(mut clipboard) => {
                            // Decode image bytes into a format usable by arboard
                            // NOTE: arboard expects raw RGBA, not encoded PNG/JPEG
                            if let Ok(img) = image::load_from_memory(&data) {
                                let rgba = img.to_rgba8();
                                let (width, height) = rgba.dimensions();
                                let image = ImageData {
                                    width: width as usize,
                                    height: height as usize,
                                    bytes: std::borrow::Cow::Owned(rgba.into_raw()),
                                };

                                if clipboard.set_image(image).is_ok() {
                                    tracing::info!("Image copied to clipboard");
                                } else {
                                    tracing::error!("Failed to set image in clipboard");
                                }
                            } else {
                                tracing::error!("Failed to decode image bytes");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Clipboard init failed: {}", e);
                        }
                    }
                } else {
                    tracing::warn!("Image blob not found for content_hash: {}", content_hash);
                }
            }
        }
    } else {
        // For text items, extract the "content" field
        if let Ok(payload) = serde_json::from_str::<Value>(&event.payload_data) {
            let text_content = payload
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    // Fallback to the whole payload if "content" field doesn't exist
                    event.payload_data.clone()
                });

            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if let Err(e) = clipboard.set_text(text_content) {
                    tracing::error!("Failed to copy text to clipboard: {}", e);
                } else {
                    tracing::info!("Text copied to clipboard successfully");
                }
            }
        } else {
            tracing::error!("Failed to parse event payload as JSON");
        }
    }
}

fn build_tray_menu(
    app: &tauri::AppHandle,
    db: &std::sync::Arc<storage::Database>,
) -> tauri::Result<AppMenu> {
    let clipboard_items = db.get_pinned_events().unwrap_or_default();

    let quit_i = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    if clipboard_items.is_empty() {
        let no_items = tauri::menu::MenuItem::with_id(
            app,
            "no_clipboard",
            "No clipboard items",
            false,
            None::<&str>,
        )?;

        tauri::menu::Menu::with_items(app, &[&no_items, &quit_i])
    } else {
        use tauri::menu::IsMenuItem;

        let clip_items: Vec<tauri::menu::MenuItem<_>> = clipboard_items
            .iter()
            .enumerate()
            .map(|(idx, rec)| {
                let id = format!("clipboard_{}", idx);

                let text = serde_json::from_str::<Value>(&rec.payload_data)
                    .ok()
                    .and_then(|v| {
                        v.get("content")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| match rec.source_app.as_deref() {
                        Some(source) => format!("Image | {}", source),
                        None => rec.payload_type.clone(),
                    });

                let preview = if text.len() > 25 {
                    format!("{}. {}...", idx + 1, &text[..22])
                } else {
                    format!("{}. {}", idx + 1, &text)
                };

                tauri::menu::MenuItem::with_id(app, id, preview, true, None::<&str>)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut item_refs: Vec<&dyn IsMenuItem<tauri_runtime_wry::Wry<EventLoopMessage>>> =
            clip_items.iter().map(|item| item as &dyn IsMenuItem<_>).collect();

        item_refs.push(&quit_i);

        tauri::menu::Menu::with_items(app, &item_refs)
    }
}

fn refresh_tray_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let db = app.state::<std::sync::Arc<storage::Database>>().clone();
    let tray_icon = app.state::<std::sync::Arc<AppTrayIcon>>().clone();
    let menu = build_tray_menu(app, &db)?;
    tray_icon.set_menu(Some(menu))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("base=debug".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tracing::info!(
                "single-instance triggered: {}, {argv:?}, {cwd}",
                app.package_info().name
            );
            app.emit("single-instance", SingleInstancePayload { args: argv, cwd })
                .unwrap();
        }))
        .plugin(
            WindowBuilder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::DECORATIONS)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Recall")
                .decorations(true)
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .shadow(true)
                .hidden_title(true)
                .center()
                .inner_size(800.0, 600.0)
                .build()?;

            // Initialize database first (needed for tray menu)
            let app_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;

            let db_path = app_dir.join("events.db");
            let db = std::sync::Arc::new(
                storage::Database::open(&db_path).expect("Failed to open database"),
            );

            db.init_schema()
                .expect("Failed to initialize database schema");

            // tray icon setup with clipboard items

            // Get recent clipboard items
            let menu = build_tray_menu(&app.handle(), &db)?;

            // Clone db for the menu event handler
            let db_for_menu = db.clone();

            let tray_icon = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    if event.id.as_ref() == "quit" {
                        println!("quit menu item was clicked");
                        app.exit(0);
                    } else if event.id.as_ref().starts_with("clipboard_") {
                        // Extract index from ID (clipboard_0, clipboard_1, etc.)
                        if let Ok(idx) = event
                            .id
                            .as_ref()
                            .strip_prefix("clipboard_")
                            .unwrap_or("")
                            .parse::<usize>()
                        {
                            let clipboard_items = db_for_menu.get_pinned_events().unwrap_or_default();
                            if let Some(rec) = clipboard_items.get(idx) {
                                tracing::info!("Copying clipboard item {} to clipboard", idx);
                                copy_event_to_clipboard(&rec, &db_for_menu);
                            }
                        }
                    } else {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;

            // Store the tray icon handle for runtime menu refreshes
            app.manage(std::sync::Arc::new(tray_icon.clone()));

            // Initialize event writer
            let writer = std::sync::Arc::new(storage::EventWriter::new(
                db.clone(),
                storage::writer::WriterConfig::default(),
                Some(app.handle().clone()),
            ));

            // Spawn the writer task asynchronously
            let writer_for_spawn = writer.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = writer_for_spawn.spawn_task() {
                    tracing::error!("Failed to spawn event writer task: {}", e);
                }
            });

            // Start clipboard polling
            let clipboard_writer = writer.clone();
            tauri::async_runtime::spawn(async move {
                sources::start_clipboard_watcher(clipboard_writer, 400).await;
            });

            app.manage(db);
            app.manage(writer);

            tracing::info!("Database initialized at {:?}", db_path);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::events::get_all_events,
            commands::events::get_events,
            commands::events::get_events_by_timestamp_range,
            commands::events::get_pinned_events,
            commands::events::pin_event,
            commands::events::unpin_event,
            commands::events::delete_event,
            commands::events::get_event_count,
            commands::events::test_insert_clipboard_event,
            commands::events::delete_all_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
