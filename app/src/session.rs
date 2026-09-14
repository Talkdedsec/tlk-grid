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

/// A re-apply is skipped this long after our own, so the guard does not chase
/// the move it just made.
const SETTLE: Duration = Duration::from_millis(400);

#[derive(Clone, Copy)]
pub struct Pin {
    pub rect: Rect,
    pub border: Border,
    applied_at: Instant,
}

#[derive(Default)]
struct State {
    touched: HashMap<isize, OriginalState>,
    pinned: HashMap<isize, Pin>,
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

    pub fn release(&self, handle: isize) -> Option<OriginalState> {
        let mut state = self.lock();
        state.pinned.remove(&handle);
        state.touched.remove(&handle)
    }

    pub fn drain(&self) -> Vec<OriginalState> {
        let mut state = self.lock();
        state.pinned.clear();
        state
            .touched
            .drain()
            .map(|(_, original)| original)
            .collect()
    }
}
