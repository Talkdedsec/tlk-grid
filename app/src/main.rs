#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod hotkeys;
mod locale;
mod session;
mod tray;

use tauri::{Manager, WindowEvent};
use tlkgrid_core::bind::panic_bind;
use tlkgrid_core::input::Input;
use tlkgrid_core::overlay::Overlay;

use session::Session;

fn main() {
    tauri::Builder::default()
        .manage(Session::default())
        .setup(|app| {
            let (input, events) = Input::start()?;
            // F8 is live from startup, before any profile is loaded.
            input.set_binds(vec![panic_bind()]);
            app.manage(input);
            app.manage(Overlay::start()?);
            hotkeys::spawn(app.handle(), events);
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
            commands::begin_bind_capture,
            commands::cancel_bind_capture,
            commands::set_binds,
            commands::set_bind_target,
            commands::set_master,
            commands::set_wheel_adjusts,
            commands::trigger_label,
            commands::key_name,
            commands::set_zoom,
            commands::zoom_shortcuts,
            commands::zoom_destination,
        ])
        .run(tauri::generate_context!())
        .expect("tlk-grid failed to start");
}
