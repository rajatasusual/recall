use tauri::{Emitter, Manager};
use tauri_plugin_window_state::{Builder as WindowBuilder, StateFlags};
use tracing_subscriber;

mod commands;
mod sources;
pub mod core;
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
                .add_directive("base=debug".parse().unwrap())
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tracing::info!("single-instance triggered: {}, {argv:?}, {cwd}", app.package_info().name);
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
            // Initialize database
            let app_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            
            let db_path = app_dir.join("events.db");
            let db = std::sync::Arc::new(
                storage::Database::open(&db_path)
                    .expect("Failed to open database")
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
