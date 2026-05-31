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

#[derive(Clone, serde::Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
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
            let clipboard_items = db.get_recent_clipboard_items(10).unwrap_or_default();

            let quit_i = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            // Build menu with clipboard items
            let menu = if clipboard_items.is_empty() {
                // No clipboard items - show placeholder
                let no_items = tauri::menu::MenuItem::with_id(
                    app,
                    "no_clipboard",
                    "No clipboard items",
                    false,
                    None::<&str>,
                )?;

                tauri::menu::Menu::with_items(app, &[&no_items, &quit_i])?
            } else {
                use tauri::menu::IsMenuItem;

                let clip_items: Vec<tauri::menu::MenuItem<_>> = clipboard_items
                    .iter()
                    .enumerate()
                    .map(|(idx, (_, payload_data))| {
                        let id = format!("clipboard_{}", idx);

                        // Parse JSON safely
                        let text = serde_json::from_str::<Value>(payload_data)
                            .ok()
                            .and_then(|v| {
                                v.get("content")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string())
                            })
                            .unwrap_or_else(|| payload_data.clone()); // fallback

                        let preview = if text.len() > 25 {
                            format!("{}. {}...", idx + 1, &text[..22])
                        } else {
                            format!("{}. {}", idx + 1, &text)
                        };

                        tauri::menu::MenuItem::with_id(app, id, preview, true, None::<&str>)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // Build trait-object vector
                let mut item_refs: Vec<&dyn IsMenuItem<tauri_runtime_wry::Wry<EventLoopMessage>>> =
                    clip_items
                        .iter()
                        .map(|item| item as &dyn IsMenuItem<_>)
                        .collect();

                item_refs.push(&quit_i);

                tauri::menu::Menu::with_items(app, &item_refs)?
            };

            // Store clipboard items in app state for reference in menu event handler
            let clipboard_items_clone = clipboard_items.clone();

            TrayIconBuilder::new()
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
                            if let Some((_, content)) = clipboard_items_clone.get(idx) {
                                tracing::info!("Copying clipboard item {} to clipboard", idx);
                                // Copy to system clipboard
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    // Parse JSON safely
                                    let content = serde_json::from_str::<Value>(content)
                                        .ok()
                                        .and_then(|v| {
                                            v.get("content")
                                                .and_then(|t| t.as_str())
                                                .map(|s| s.to_string())
                                        })
                                        .unwrap_or_else(|| content.clone()); // fallback

                                    if let Err(e) = clipboard.set_text(content.clone()) {
                                        tracing::error!("Failed to copy to clipboard: {}", e);
                                    }
                                }
                            }
                        }
                    } else {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;

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
