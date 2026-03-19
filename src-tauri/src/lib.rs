pub mod commands;
pub mod constants;
pub mod discovery;
pub mod events;
pub mod file_watcher;
pub mod line_reader;
pub mod models;
pub mod persistence;
pub mod project_name;
pub mod state;
pub mod timer_manager;
pub mod transcript_parser;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    // Set Tokio as Tauri's async runtime so tokio::spawn works everywhere.
    // Leak the runtime to keep it alive for the entire app lifetime.
    let rt = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime"),
    ));
    tauri::async_runtime::set(rt.handle().clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::webview_ready,
            commands::save_layout,
            commands::save_agent_seats,
            commands::set_sound_enabled,
            commands::export_layout,
            commands::import_layout,
            commands::open_sessions_folder,
            commands::close_agent,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                discovery::start_discovery_loop(app_handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
