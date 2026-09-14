//! Moving, resizing and un-bordering a target window.
//!
//! Everything here goes through the visible frame, not the raw window rect.
//! Since Windows 10 a window's `GetWindowRect` includes an invisible resize
//! border — usually 7px on the sides and bottom — so placing a window at x=0
//! with SetWindowPos leaves it looking 7px off. `DWMWA_EXTENDED_FRAME_BOUNDS`
//! reports what the user actually sees, and the difference between the two is
//! the padding every call below compensates for.

use serde::{Deserialize, Serialize};

use crate::layout::Rect;

/// Enough of a window's original state to put it back exactly as it was.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OriginalState {
    pub handle: isize,
    pub style: i32,
    pub ex_style: i32,
    pub rect: Rect,
    pub maximized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Border {
    /// Leave the title bar and frame alone.
    Keep,
    /// Strip the caption and resize frame; the client area then fills the rect.
    Borderless,
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GetWindowPlacement, GetWindowRect, IsIconic, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_NOTOPMOST, HWND_TOPMOST,
        SET_WINDOW_POS_FLAGS, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOZORDER,
        SW_RESTORE, SW_SHOWNORMAL, WINDOWPLACEMENT, WS_CAPTION, WS_EX_CLIENTEDGE,
        WS_EX_DLGMODALFRAME, WS_EX_STATICEDGE, WS_EX_WINDOWEDGE, WS_MAXIMIZE, WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
    };

    use super::{Border, OriginalState};
    use crate::error::{Error, Result};
    use crate::layout::Rect;

    const CAPTION_BITS: i32 =
        (WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0)
            as i32;
    const EDGE_BITS: i32 =
        (WS_EX_DLGMODALFRAME.0 | WS_EX_WINDOWEDGE.0 | WS_EX_CLIENTEDGE.0 | WS_EX_STATICEDGE.0)
            as i32;

    fn hwnd(handle: isize) -> HWND {
        HWND(handle as *mut _)
    }

    fn rect_of(r: RECT) -> Rect {
        Rect::from_ltrb(r.left, r.top, r.right, r.bottom)
    }

    /// The rect Windows reports, invisible border included.
    pub fn window_rect(handle: isize) -> Result<Rect> {
        let mut r = RECT::default();
        unsafe { GetWindowRect(hwnd(handle), &mut r) }
            .map_err(|e| Error::win32("GetWindowRect", e.code().0))?;
        Ok(rect_of(r))
    }

    /// The rect the user sees. Equal to `window_rect` on borderless windows.
    pub fn visible_rect(handle: isize) -> Result<Rect> {
        let mut r = RECT::default();
        let hr = unsafe {
            DwmGetWindowAttribute(
                hwnd(handle),
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&mut r as *mut RECT).cast(),
                std::mem::size_of::<RECT>() as u32,
            )
        };
        match hr {
            Ok(()) => Ok(rect_of(r)),
            // Pre-composition or a window DWM does not track: the raw rect is all there is.
            Err(_) => window_rect(handle),
        }
    }

    /// How much bigger the raw rect is than what the user sees, per edge.
    fn shadow_padding(handle: isize) -> Result<(i32, i32, i32, i32)> {
        let raw = window_rect(handle)?;
        let seen = visible_rect(handle)?;
        Ok((
            seen.x - raw.x,
            seen.y - raw.y,
            raw.right() - seen.right(),
            raw.bottom() - seen.bottom(),
        ))
    }

    pub fn capture(handle: isize) -> Result<OriginalState> {
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        unsafe { GetWindowPlacement(hwnd(handle), &mut placement) }
            .map_err(|e| Error::win32("GetWindowPlacement", e.code().0))?;
        let style = unsafe { GetWindowLongPtrW(hwnd(handle), GWL_STYLE) } as i32;
        Ok(OriginalState {
            handle,
            style,
            ex_style: unsafe { GetWindowLongPtrW(hwnd(handle), GWL_EXSTYLE) } as i32,
            rect: window_rect(handle)?,
            maximized: style & WS_MAXIMIZE.0 as i32 != 0,
        })
    }

    fn apply_border(handle: isize, border: Border) -> Result<()> {
        let style = unsafe { GetWindowLongPtrW(hwnd(handle), GWL_STYLE) } as i32;
        let ex_style = unsafe { GetWindowLongPtrW(hwnd(handle), GWL_EXSTYLE) } as i32;
        let (next_style, next_ex) = match border {
            Border::Keep => return Ok(()),
            Border::Borderless => (
                (style & !CAPTION_BITS) | WS_POPUP.0 as i32,
                ex_style & !EDGE_BITS,
            ),
        };
        if next_style == style && next_ex == ex_style {
            return Ok(());
        }
        unsafe {
            SetWindowLongPtrW(hwnd(handle), GWL_STYLE, next_style as isize);
            SetWindowLongPtrW(hwnd(handle), GWL_EXSTYLE, next_ex as isize);
        }
        Ok(())
    }

    /// Place the *visible* frame at `target`. A maximized or minimized window is
    /// restored first, otherwise Windows silently ignores the move.
    pub fn place(handle: isize, target: Rect, border: Border) -> Result<Rect> {
        if !super::super::target::is_alive(handle) {
            return Err(Error::WindowGone(handle));
        }
        unsafe {
            if IsIconic(hwnd(handle)).as_bool() {
                let _ = ShowWindow(hwnd(handle), SW_RESTORE);
            }
            let style = GetWindowLongPtrW(hwnd(handle), GWL_STYLE) as i32;
            if style & WS_MAXIMIZE.0 as i32 != 0 {
                let _ = ShowWindow(hwnd(handle), SW_SHOWNORMAL);
            }
        }

        let framechanged = if border == Border::Borderless {
            apply_border(handle, border)?;
            SWP_FRAMECHANGED
        } else {
            SET_WINDOW_POS_FLAGS(0)
        };

        // The padding has to be read after the style change, since stripping the
        // frame is exactly what removes it.
        let (pad_l, pad_t, pad_r, pad_b) = shadow_padding(handle)?;
        let adjusted = Rect::from_ltrb(
            target.x - pad_l,
            target.y - pad_t,
            target.right() + pad_r,
            target.bottom() + pad_b,
        );

        unsafe {
            SetWindowPos(
                hwnd(handle),
                None,
                adjusted.x,
                adjusted.y,
                adjusted.w,
                adjusted.h,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOCOPYBITS | framechanged,
            )
        }
        .map_err(|e| Error::win32("SetWindowPos", e.code().0))?;

        visible_rect(handle)
    }

    pub fn set_topmost(handle: isize, topmost: bool) -> Result<()> {
        let after = if topmost {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        unsafe {
            SetWindowPos(
                hwnd(handle),
                Some(after),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SET_WINDOW_POS_FLAGS(0x0001 | 0x0002),
            )
        }
        .map_err(|e| Error::win32("SetWindowPos(topmost)", e.code().0))
    }

    /// F8 and "close the card" both land here: styles and rect back as captured.
    pub fn restore(state: &OriginalState) -> Result<()> {
        if !super::super::target::is_alive(state.handle) {
            return Err(Error::WindowGone(state.handle));
        }
        unsafe {
            SetWindowLongPtrW(hwnd(state.handle), GWL_STYLE, state.style as isize);
            SetWindowLongPtrW(hwnd(state.handle), GWL_EXSTYLE, state.ex_style as isize);
            SetWindowPos(
                hwnd(state.handle),
                None,
                state.rect.x,
                state.rect.y,
                state.rect.w,
                state.rect.h,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
            )
            .map_err(|e| Error::win32("SetWindowPos(restore)", e.code().0))?;
            if state.maximized {
                let _ = ShowWindow(hwnd(state.handle), SW_SHOWNORMAL);
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use imp::{capture, place, restore, set_topmost, visible_rect, window_rect};
