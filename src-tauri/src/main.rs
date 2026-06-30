#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    // ponytail: tauri-apps/tauri#9394 — fix WebKitGTK blank-window bug on some Linux drivers.
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    let data_dir = AppState::app_data_dir();
    let config_path = data_dir.join("config.toml");
    let sessions_dir = data_dir.join("sessions");

    std::fs::create_dir_all(&sessions_dir).expect("failed to create sessions directory");

    let config = AppState::load_config(&config_path);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            config: std::sync::RwLock::new(config),
            config_path,
            sessions_dir,
        })
        .setup(|app| {
            // Ponytail: Single dashboard window created at startup.
            // Platform windows are spawned on-demand via open_account command.
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("Social Manager")
            .inner_size(800.0, 600.0)
            .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::add_account,
            commands::remove_account,
            commands::open_account,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
