#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::WindowBuilder;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_social_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn open_social_app(url: String, label: String) {
    let _ = WindowBuilder::new(
        &tauri::CurrentHandle::default(),
        label,
        tauri::WindowUrl::External(url.parse().unwrap())
    )
    .title("Social App")
    .build();
}