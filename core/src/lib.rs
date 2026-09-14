//! tlk-grid core.
//!
//! Window layout and zoom for games, built only on documented Windows APIs:
//! Win32 window management, DWM thumbnails, Windows.Graphics.Capture and NvAPI.
//! Nothing here injects code, reads another process's memory, or loads a driver.
//!
//! `layout` is pure arithmetic and carries the test suite; everything else wraps
//! a platform call and is compiled on Windows only.

pub mod bind;
pub mod display;
pub mod error;
pub mod frame;
pub mod input;
pub mod layout;
pub mod overlay;
pub mod target;
pub mod zoom;

pub use error::{Error, Result};
pub use layout::Rect;

/// Windows 10 1903 is the floor: below it there is no Windows.Graphics.Capture,
/// which CurveFX and the motion-blur pipeline both need.
pub const MIN_WINDOWS_BUILD: u32 = 18362;
