#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod locale;
mod session;
mod tray;

use tauri::WindowEvent;

use session::Session;

fn main() {
    tauri::Builder::default()
        .manage(Session::default())
        .setup(|app| {
            tray::install(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Same contract as the tray note: close hides, Exit quits.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "manager" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::system_language,
            commands::list_windows,
            commands::list_monitors,
            commands::monitor_for_window,
            commands::split_cells,
            commands::aspect_presets,
            commands::anchor_rect,
            commands::letterbox_bars,
            commands::zoom_preview,
            commands::apply_placement,
            commands::window_rect,
            commands::release_window,
            commands::restore_everything,
        ])
        .run(tauri::generate_context!())
        .expect("tlk-grid failed to start");
}
