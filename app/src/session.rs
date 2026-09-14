//! What tlk-grid changed and how to undo it.
//!
//! A window's original style and rect are captured the first time it is placed
//! and never overwritten after that, so F8 restores the state the game shipped
//! with rather than the last layout applied to it.

use std::collections::HashMap;
use std::sync::Mutex;

use tlkgrid_core::frame::OriginalState;

#[derive(Default)]
pub struct Session {
    touched: Mutex<HashMap<isize, OriginalState>>,
}

impl Session {
    pub fn remember(&self, state: OriginalState) {
        let mut touched = self.touched.lock().expect("session lock");
        touched.entry(state.handle).or_insert(state);
    }

    pub fn forget(&self, handle: isize) -> Option<OriginalState> {
        self.touched.lock().expect("session lock").remove(&handle)
    }

    pub fn drain(&self) -> Vec<OriginalState> {
        self.touched
            .lock()
            .expect("session lock")
            .drain()
            .map(|(_, s)| s)
            .collect()
    }

    pub fn is_touched(&self, handle: isize) -> bool {
        self.touched
            .lock()
            .expect("session lock")
            .contains_key(&handle)
    }
}
