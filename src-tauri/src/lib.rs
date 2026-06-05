use serde_json::Value;
use tauri::{
    tray::TrayIconBuilder, Emitter, EventLoopMessage, Manager, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_window_state::{Builder as WindowBuilder, StateFlags};

// New module structure (Phase 2)
pub mod config;
pub mod domain;
pub mod persistence;

// Legacy modules (maintained for backward compatibility)
mod api;
mod commands;
pub mod core;
mod error;

pub use error::{AppError, AppResult};

use crate::config::{ClipboardConfig, WriterConfig};

type AppMenu = tauri::menu::Menu<tauri_runtime_wry::Wry<EventLoopMessage>>;
type AppTrayIcon = tauri::tray::TrayIcon<tauri_runtime_wry::Wry<EventLoopMessage>>;

#[derive(Clone, serde::Serialize)]
struct SingleInstancePayload {
    args: Vec<String>,
    cwd: String,
}

fn build_tray_menu(
    app: &tauri::AppHandle,
    db: &std::sync::Arc<persistence::Database>,
) -> tauri::Result<AppMenu> {
    let clipboard_items = db
        .get_events(Some(true), None, None, None)
        .unwrap_or_default();

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
            clip_items
                .iter()
                .map(|item| item as &dyn IsMenuItem<_>)
                .collect();

        item_refs.push(&quit_i);

        tauri::menu::Menu::with_items(app, &item_refs)
    }
}

fn refresh_tray_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let db = app.state::<std::sync::Arc<persistence::Database>>().clone();
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
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Recall")
                .decorations(true)
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .shadow(true)
                .hidden_title(true)
                .center()
                .inner_size(640.0, 800.0)
                .min_inner_size(640.0, 800.0)
                .build()?;

            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::ShortcutState;

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcuts(["CmdOrCtrl+Q"])?
                        .with_handler(|app, shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                tracing::info!("Global shortcut triggered: {shortcut}");
                                app.exit(0);
                            }
                        })
                        .build(),
                )?;
            }

            // Initialize database first (needed for tray menu)
            let app_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;

            let db_config = config::StorageConfig::default();

            let db_path = app_dir.join(db_config.database_path.clone());
            let db = std::sync::Arc::new(
                persistence::Database::open(&db_path, db_config).expect("Failed to open database"),
            );

            db.init_schema()
                .expect("Failed to initialize database schema");

            // tray icon setup with clipboard items

            // Get recent clipboard items
            let menu_handle = app.handle();
            let menu = build_tray_menu(menu_handle, &db)?;

            // Clone db for the menu event handler
            let db_for_menu = db.clone();

            let tray_icon = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    if event.id.as_ref() == "quit" {
                        tracing::info!("quit menu item was clicked");
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
                            let clipboard_items = db_for_menu
                                .get_events(Some(true), None, None, None)
                                .unwrap_or_default();
                            if let Some(rec) = clipboard_items.get(idx) {
                                tracing::info!("Copying clipboard item {} to clipboard", idx);
                                domain::clipboard::copy_event_to_clipboard(rec, &db_for_menu, app);
                            }
                        }
                    } else {
                        tracing::info!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;

            // Store the tray icon handle for runtime menu refreshes
            app.manage(std::sync::Arc::new(tray_icon.clone()));

            // Initialize event writer
            let writer = std::sync::Arc::new(persistence::EventWriter::new(
                db.clone(),
                WriterConfig::default(),
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
            let clipboard_app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                domain::clipboard::start_clipboard_watcher(
                    clipboard_writer,
                    clipboard_app_handle,
                    ClipboardConfig::default(),
                )
                .await;
            });

            app.manage(db.clone());
            app.manage(writer);

            // Initialize the event service layer
            let event_service = api::EventService::new(db);
            app.manage(event_service);

            tracing::info!("Database initialized at {:?}", db_path);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent actual close
                api.prevent_close();
                // Hide or minimize instead
                let _ = window.minimize();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::events::get_events,
            commands::events::pin_event,
            commands::events::unpin_event,
            commands::events::delete_event,
            commands::events::delete_all_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
