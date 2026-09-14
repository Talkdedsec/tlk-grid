//! Zoom arithmetic, kept away from the platform so it can be tested.
//!
//! The four methods differ only in *what* gets resized:
//!
//! - `Thumbnail` blows up a live DWM copy of the window on an overlay and never
//!   touches the game. It is the default because it cannot break anything.
//! - `Window` moves the real window so the part you care about overflows the
//!   screen; the game keeps rendering at its own resolution either way.
//! - `Stretch` resizes the real window to exact numbers instead of a factor.
//! - `Dpi` leans on per-application DPI scaling and needs the game restarted.
//!
//! The destination is the readout under the bind row: a 1920×540 window at 3×
//! reads `5760 × 1620 (base 1920×540)`. Clipping that to the monitor is what
//! produces the magnified centre.

use serde::{Deserialize, Serialize};

use crate::layout::{clamp_zoom, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    #[default]
    Thumbnail,
    Window,
    Stretch,
    Dpi,
}

impl Method {
    /// Whether engaging the bind moves the game's own window. Thumbnail does
    /// not, which is why it is safe to leave bound during a match.
    pub fn touches_the_game(self) -> bool {
        !matches!(self, Method::Thumbnail)
    }
}

/// Where the magnified copy lands, in screen coordinates, before clipping.
pub fn destination(base: Rect, factor: f64) -> Rect {
    base.scaled_about_center(clamp_zoom(factor))
}

/// The multipliers on the buttons beside the result readout.
pub const SHORTCUTS: [f64; 3] = [1.25, 1.5, 2.0];

/// Wheel steps follow a ladder rather than one fixed increment: 0.1 is useless
/// at 20× and 1.0 is far too coarse at 1.2×.
pub fn wheel_step(factor: f64) -> f64 {
    match factor {
        f if f < 2.0 => 0.1,
        f if f < 5.0 => 0.25,
        f if f < 10.0 => 0.5,
        _ => 1.0,
    }
}

/// Apply wheel notches to a factor. Rounded to the step so repeated scrolling
/// lands on readable numbers instead of 2.9000000000000004.
pub fn nudge(factor: f64, notches: i32) -> f64 {
    let mut value = clamp_zoom(factor);
    for _ in 0..notches.unsigned_abs() {
        let step = wheel_step(value);
        value = clamp_zoom(if notches > 0 {
            value + step
        } else {
            value - step
        });
        value = (value / step).round() * step;
    }
    clamp_zoom((value * 100.0).round() / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{ZOOM_MAX, ZOOM_MIN};

    const SUPERWIDE: Rect = Rect::new(0, 270, 1920, 540);

    #[test]
    fn the_destination_matches_the_readout() {
        let dest = destination(SUPERWIDE, 3.0);
        assert_eq!((dest.w, dest.h), (5760, 1620));
    }

    #[test]
    fn the_magnified_copy_stays_centred_on_the_window() {
        assert_eq!(destination(SUPERWIDE, 4.0).center(), SUPERWIDE.center());
        assert_eq!(destination(SUPERWIDE, 0.5).center(), SUPERWIDE.center());
    }

    #[test]
    fn only_thumbnail_leaves_the_game_window_alone() {
        assert!(!Method::Thumbnail.touches_the_game());
        assert!(Method::Window.touches_the_game());
        assert!(Method::Stretch.touches_the_game());
        assert!(Method::Dpi.touches_the_game());
    }

    #[test]
    fn the_wheel_step_grows_with_the_factor() {
        assert_eq!(wheel_step(1.5), 0.1);
        assert_eq!(wheel_step(3.0), 0.25);
        assert_eq!(wheel_step(8.0), 0.5);
        assert_eq!(wheel_step(30.0), 1.0);
    }

    #[test]
    fn scrolling_lands_on_readable_numbers() {
        assert_eq!(nudge(1.0, 1), 1.1);
        assert_eq!(nudge(1.0, 5), 1.5);
        assert_eq!(nudge(2.0, 1), 2.25);
        assert_eq!(nudge(1.5, -1), 1.4);
    }

    #[test]
    fn scrolling_cannot_leave_the_documented_range() {
        assert_eq!(nudge(ZOOM_MAX, 10), ZOOM_MAX);
        assert_eq!(nudge(ZOOM_MIN, -10), ZOOM_MIN);
    }

    #[test]
    fn a_full_scroll_up_and_back_returns_to_the_start() {
        let start = 1.5;
        assert_eq!(nudge(nudge(start, 3), -3), start);
    }
}
