#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod hotkeys;
mod locale;
mod overlays;
mod session;
mod tray;

use tauri::{Manager, WindowEvent};
use tlkgrid_core::bind::panic_bind;
use tlkgrid_core::input::Input;
use tlkgrid_core::layers::Layers;

use session::Session;

fn main() {
    // The settings windows are declared in the config, so their pages start
    // calling commands the moment the app opens — before `setup` would have
    // finished. Everything they reach for is therefore registered here, ahead
    // of the builder, rather than inside it.
    let layers = match Layers::start() {
        Ok(layers) => layers,
        Err(error) => return fail("the overlay thread could not start", error),
    };
    let (input, events) = match Input::start() {
        Ok(started) => started,
        Err(error) => return fail("the keyboard and mouse hooks could not be installed", error),
    };
    // F8 is live from startup, before any profile is loaded.
    input.set_binds(vec![panic_bind()]);

    tauri::Builder::default()
        .manage(Session::default())
        .manage(overlays::Overlays::default())
        .manage(layers)
        .manage(input)
        .setup(move |app| {
            // A crosshair chosen in an earlier session comes back with it.
            if let Ok(dir) = app.path().app_data_dir() {
                if let Some(crosshair) = overlays::Crosshair::restore(dir.join("crosshair.png")) {
                    let state = app.state::<overlays::Overlays>();
                    state.set_crosshair(Some(crosshair.clone()));
                    hotkeys::repaint_crosshair(app.handle(), &crosshair);
                }
            }
            hotkeys::spawn(app.handle(), events);
            tray::install(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the manager hides it to the tray; closing a settings
            // window hides it too, since the config owns them for the whole run.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
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
            commands::set_crosshair_image,
            commands::set_crosshair_scale,
            commands::toggle_crosshair,
            commands::clear_crosshair,
            commands::crosshair_state,
            commands::set_scope,
            commands::scope_state,
        ])
        .run(tauri::generate_context!())
        .expect("tlk-grid failed to start");
}

/// Startup failures are the one place a message box beats a log line: the app
/// is windowless at this point and a silent exit tells the user nothing.
fn fail(what: &str, error: tlkgrid_core::Error) {
    let text = format!("tlk-grid could not start.\n\n{what}.\n{error}");
    #[cfg(windows)]
    unsafe {
        use windows::core::HSTRING;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        MessageBoxW(
            None,
            &HSTRING::from(text.as_str()),
            &HSTRING::from("tlk-grid"),
            MB_OK | MB_ICONERROR,
        );
    }
    #[cfg(not(windows))]
    eprintln!("{text}");
}
