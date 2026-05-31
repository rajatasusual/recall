use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
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
                .shadow(false)
                .hidden_title(true)
                .center()
                .inner_size(800.0, 600.0)
                .build()?;

            // tray icon setup
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        println!("quit menu item was clicked");
                        app.exit(0);
                    }
                    _ => {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;

            // Initialize database
            let app_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;

            let db_path = app_dir.join("events.db");
            let db = std::sync::Arc::new(
                storage::Database::open(&db_path).expect("Failed to open database"),
            );

            db.init_schema()
                .expect("Failed to initialize database schema");

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
