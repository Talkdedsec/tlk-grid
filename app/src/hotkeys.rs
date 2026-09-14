//! Bridges the input thread to the rest of the app.
//!
//! The hook thread must never block, so it only reports; every decision that
//! needs to touch a window happens here, on a plain worker thread.

use std::sync::mpsc::Receiver;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tlkgrid_core::bind::Action;
use tlkgrid_core::frame;
use tlkgrid_core::input::{Input, InputEvent};

use crate::session::Session;

/// Everything the frontend listens for arrives under this one name, tagged by
/// the `event` field of `InputEvent`.
pub const CHANNEL: &str = "input";

/// Carries the real count after an F8, so the status bar reports what happened
/// instead of guessing from the cards on screen.
pub const RESTORED: &str = "restored";

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
    let session = app.state::<Session>();
    match event {
        InputEvent::Engaged {
            action: Action::Panic,
        } => {
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
        } => {
            app.state::<Input>().set_master(false);
        }
        InputEvent::Released {
            action: Action::Master,
        } => {
            app.state::<Input>().set_master(true);
        }
        // The game reasserted its own geometry — usually on the way back from
        // Alt-Tab. Put the layout back, but only for a window the user pinned.
        InputEvent::Moved { handle } => {
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
