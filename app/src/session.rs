//! What tlk-grid changed, how to undo it, and what to defend.
//!
//! A window's original style and rect are captured the first time it is placed
//! and never overwritten after that, so F8 restores the state the game shipped
//! with rather than the last layout applied to it.
//!
//! A *pinned* window additionally has a layout the guard re-asserts when the
//! game moves it on its own — the Alt-Tab case. Pinning is opt-in per card,
//! because a window that snaps back while the user is dragging it is worse than
//! one that loses its layout.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tlkgrid_core::frame::{Border, OriginalState};
use tlkgrid_core::layout::Rect;
use tlkgrid_core::zoom::{self, Method};

/// A re-apply is skipped this long after our own, so the guard does not chase
/// the move it just made.
const SETTLE: Duration = Duration::from_millis(400);

#[derive(Clone, Copy)]
pub struct Pin {
    pub rect: Rect,
    pub border: Border,
    applied_at: Instant,
}

/// What the Resize bind does for one window.
#[derive(Clone, Copy)]
pub struct Zoom {
    pub factor: f64,
    pub method: Method,
    pub border: Border,
    /// Where the window sat before a window-moving method engaged, so letting
    /// go puts it back.
    pub resting: Option<Rect>,
}

#[derive(Default)]
struct State {
    touched: HashMap<isize, OriginalState>,
    pinned: HashMap<isize, Pin>,
    zoom: HashMap<isize, Zoom>,
}

#[derive(Default)]
pub struct Session {
    state: Mutex<State>,
}

impl Session {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn remember(&self, original: OriginalState) {
        self.lock()
            .touched
            .entry(original.handle)
            .or_insert(original);
    }

    pub fn is_touched(&self, handle: isize) -> bool {
        self.lock().touched.contains_key(&handle)
    }

    pub fn pin(&self, handle: isize, rect: Rect, border: Border) {
        self.lock().pinned.insert(
            handle,
            Pin {
                rect,
                border,
                applied_at: Instant::now(),
            },
        );
    }

    pub fn unpin(&self, handle: isize) {
        self.lock().pinned.remove(&handle);
    }

    /// The layout to re-assert, or `None` when the window is unpinned or the
    /// last apply is still settling.
    pub fn pin_to_defend(&self, handle: isize) -> Option<Pin> {
        self.lock()
            .pinned
            .get(&handle)
            .copied()
            .filter(|pin| pin.applied_at.elapsed() >= SETTLE)
    }

    pub fn mark_applied(&self, handle: isize) {
        if let Some(pin) = self.lock().pinned.get_mut(&handle) {
            pin.applied_at = Instant::now();
        }
    }

    // ------------------------------------------------------------------ zoom

    pub fn set_zoom(&self, handle: isize, factor: f64, method: Method, border: Border) {
        let mut state = self.lock();
        let resting = state.zoom.get(&handle).and_then(|z| z.resting);
        state.zoom.insert(
            handle,
            Zoom {
                factor,
                method,
                border,
                resting,
            },
        );
    }

    pub fn zoom(&self, handle: isize) -> Option<Zoom> {
        self.lock().zoom.get(&handle).copied()
    }

    /// Wheel notches while the bind is held. Returns the new factor so the
    /// caller can redraw and tell the UI.
    pub fn nudge_zoom(&self, handle: isize, notches: i32) -> Option<f64> {
        let mut state = self.lock();
        let entry = state.zoom.get_mut(&handle)?;
        entry.factor = zoom::nudge(entry.factor, notches);
        Some(entry.factor)
    }

    /// Records where a window-moving method found the window, once per hold.
    pub fn park(&self, handle: isize, resting: Rect) {
        if let Some(entry) = self.lock().zoom.get_mut(&handle) {
            entry.resting.get_or_insert(resting);
        }
    }

    /// Hands back the parked rect and clears it, for the release path.
    pub fn unpark(&self, handle: isize) -> Option<Rect> {
        self.lock().zoom.get_mut(&handle)?.resting.take()
    }

    // --------------------------------------------------------------- undoing

    pub fn release(&self, handle: isize) -> Option<OriginalState> {
        let mut state = self.lock();
        state.pinned.remove(&handle);
        state.zoom.remove(&handle);
        state.touched.remove(&handle)
    }

    pub fn drain(&self) -> Vec<OriginalState> {
        let mut state = self.lock();
        state.pinned.clear();
        state.zoom.clear();
        state
            .touched
            .drain()
            .map(|(_, original)| original)
            .collect()
    }
}
