//! Bridges the input thread to the rest of the app.
//!
//! The hook thread must never block, so it only reports; every decision that
//! needs to touch a window happens here, on a plain worker thread.

use std::sync::mpsc::Receiver;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tlkgrid_core::bind::Action;
use tlkgrid_core::input::{Input, InputEvent};
use tlkgrid_core::overlay::{Overlay, Spec};
use tlkgrid_core::zoom::{self, Method};
use tlkgrid_core::{display, frame};

use crate::session::Session;

/// Everything the frontend listens for arrives under this one name, tagged by
/// the `event` field of `InputEvent`.
pub const CHANNEL: &str = "input";

/// Carries the real count after an F8, so the status bar reports what happened
/// instead of guessing from the cards on screen.
pub const RESTORED: &str = "restored";

/// The live factor while the wheel is turning, so the bind row keeps up.
pub const FACTOR: &str = "factor";

pub fn spawn<R: Runtime>(app: &AppHandle<R>, events: Receiver<InputEvent>) {
    let app = app.clone();
    std::thread::Builder::new()
        .name("tlk-grid-bridge".into())
        .spawn(move || {
            for event in events {
                dispatch(&app, event);
                let _ = app.emit(CHANNEL, event);
            }
        })
        .expect("bridge thread");
}

fn dispatch<R: Runtime>(app: &AppHandle<R>, event: InputEvent) {
    match event {
        InputEvent::Engaged {
            action: Action::Panic,
        } => {
            app.state::<Overlay>().hide();
            let session = app.state::<Session>();
            let restored = session
                .drain()
                .iter()
                .filter(|original| frame::restore(original).is_ok())
                .count();
            let _ = app.emit(RESTORED, restored);
        }

        // The master bind is a bypass switch: engaged means the Resize bind is
        // suspended and its key goes straight to the game.
        InputEvent::Engaged {
            action: Action::Master,
        } => app.state::<Input>().set_master(false),
        InputEvent::Released {
            action: Action::Master,
        } => app.state::<Input>().set_master(true),

        InputEvent::Engaged {
            action: Action::Resize,
        } => engage_zoom(app),
        InputEvent::Released {
            action: Action::Resize,
        } => release_zoom(app),
        InputEvent::Wheel {
            action: Action::Resize,
            notches,
        } => {
            let Some(handle) = app.state::<Input>().target() else {
                return;
            };
            if let Some(factor) = app.state::<Session>().nudge_zoom(handle, notches) {
                let _ = app.emit(FACTOR, factor);
                engage_zoom(app);
            }
        }

        // The game reasserted its own geometry — usually on the way back from
        // Alt-Tab. Put the layout back, but only for a window the user pinned.
        InputEvent::Moved { handle } => {
            let session = app.state::<Session>();
            let Some(pin) = session.pin_to_defend(handle) else {
                return;
            };
            let Ok(current) = frame::visible_rect(handle) else {
                return;
            };
            if current != pin.rect {
                let _ = frame::place(handle, pin.rect, pin.border);
                session.mark_applied(handle);
            }
        }

        _ => {}
    }
}

/// Zoom in. Thumbnail mirrors the window on an overlay and leaves the game
/// alone; the other methods move the real window and remember where it was.
fn engage_zoom<R: Runtime>(app: &AppHandle<R>) {
    let Some(handle) = app.state::<Input>().target() else {
        return;
    };
    let session = app.state::<Session>();
    let Some(zoom) = session.zoom(handle) else {
        return;
    };
    let Ok(base) = frame::visible_rect(handle) else {
        return;
    };

    match zoom.method {
        Method::Thumbnail => {
            let Ok(monitor) = display::for_window(handle) else {
                return;
            };
            app.state::<Overlay>().show(Spec {
                source: handle,
                bounds: monitor.bounds,
                destination: zoom::destination(base, zoom.factor),
            });
        }
        Method::Window => {
            session.park(handle, base);
            let resting = session.zoom(handle).and_then(|z| z.resting).unwrap_or(base);
            let _ = frame::place(handle, zoom::destination(resting, zoom.factor), zoom.border);
        }
        // Stretch fills the monitor instead of overflowing it: same window move,
        // different target rect.
        Method::Stretch => {
            session.park(handle, base);
            let Ok(monitor) = display::for_window(handle) else {
                return;
            };
            let _ = frame::place(handle, monitor.bounds, zoom.border);
        }
        // Per-application DPI is a compatibility flag, not something that can be
        // flipped on a running process. The UI says so rather than pretending.
        Method::Dpi => {}
    }
}

fn release_zoom<R: Runtime>(app: &AppHandle<R>) {
    app.state::<Overlay>().hide();
    let Some(handle) = app.state::<Input>().target() else {
        return;
    };
    let session = app.state::<Session>();
    let Some(zoom) = session.zoom(handle) else {
        return;
    };
    if let Some(resting) = session.unpark(handle) {
        let _ = frame::place(handle, resting, zoom.border);
    }
}
