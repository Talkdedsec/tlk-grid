//! Closing the manager window hides it; the tray icon is how you get it back
//! and the only thing that really quits.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open tlk-grid", true, None::<&str>)?;
    let restore = MenuItem::with_id(
        app,
        "restore",
        "Restore all windows (F8)",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &restore, &quit])?;

    TrayIconBuilder::with_id("tlk-grid")
        .icon(app.default_window_icon().cloned().expect("bundled icon"))
        .tooltip("tlk-grid")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => reveal(app),
            "restore" => {
                let session = app.state::<crate::session::Session>();
                for state in session.drain() {
                    let _ = tlkgrid_core::frame::restore(&state);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn reveal<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("manager") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
