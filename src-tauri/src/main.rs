#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    // DMABUF disabled — fixes blank window on Niri/Wayland (tauri-apps/tauri#9394).
    // Re-enable scrolling performance by commenting out if your GPU handles DMABUF.
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    // Force native non-overlay scrollbars — fixes event-detection issues in
    // complex web-app modals (Facebook post overlays, Twitter DM panels, etc.).
    #[cfg(target_os = "linux")]
    std::env::set_var("GTK_OVERLAY_SCROLLING", "0");

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
            // Platform tabs are spawned on-demand via open_account command.
            let main_window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("Social Manager")
            .inner_size(800.0, 600.0)
            .build()?;

            // On Linux, restructure the window's widget tree so tab webviews
            // end up inside a viewport-sized GtkFixed overlay child. This is
            // the only way to get correct positioning because wry's
            // set_bounds is a no-op on Wayland when the parent is GtkBox
            // (it needs GtkFixed to set is_in_fixed_parent = true).
            //
            // Layout after setup:
            //   GtkWindow
            //     └── GtkBox (vbox, content area)
            //          └── GtkOverlay
            //               ├── main child: main WebKitWebView (Svelte UI)
            //               └── overlay child: GtkFixed (viewport-sized)
            //                    ├── tab_* webview (if opened)
            //                    └── ...
            //
            // The GtkFixed is smaller than the window (margined to skip
            // sidebar + tab bar), so pointer events on the sidebar/tab bar
            // go through the overlay's empty area to the main webview
            // naturally — no set_overlay_pass_through needed.
            #[cfg(target_os = "linux")]
            {
                use gtk::prelude::*;
                if let Ok(()) = main_window.with_webview(move |pwv| {
                    let main_webview = pwv.inner();
                    if let Some(vbox) = main_webview
                        .parent()
                        .and_then(|p| p.dynamic_cast::<gtk::Box>().ok())
                    {
                        let overlay = gtk::Overlay::new();
                        let fixed = gtk::Fixed::new();
                        fixed.set_halign(gtk::Align::Start);
                        fixed.set_valign(gtk::Align::Start);

                        vbox.remove(&main_webview);
                        overlay.add(&main_webview);
                        overlay.add_overlay(&fixed);
                        overlay.show_all();
                        vbox.pack_start(&overlay, true, true, 0);

                        let ptr = fixed.as_ptr() as *mut gtk::ffi::GtkFixed as *mut std::ffi::c_void;
                        state::FIXED_CONTAINER.store(ptr, std::sync::atomic::Ordering::SeqCst);
                    }
                }) {
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::add_account,
            commands::remove_account,
            commands::rename_account,
            commands::copy_image,
            commands::open_account,
            commands::close_tab,
            commands::switch_tab,
            commands::update_viewport,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
