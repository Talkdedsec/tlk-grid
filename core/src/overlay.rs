//! The zoom overlay: a click-through topmost window that hosts a live DWM
//! thumbnail of the target.
//!
//! This is the trick the whole product rests on. `DwmRegisterThumbnail` hands
//! the compositor a second view of a window, and `rcDestination` says how big to
//! draw it. Blowing the destination up past the monitor and letting DWM clip it
//! is what magnifies the centre — at no cost to the game, which never learns
//! any of this happened and keeps rendering at its own resolution.
//!
//! The overlay never activates and never takes the mouse, so aiming still works
//! while it is up.

use serde::{Deserialize, Serialize};

use crate::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spec {
    /// Window being mirrored.
    pub source: isize,
    /// Screen area the overlay covers; anything outside is left alone.
    pub bounds: Rect,
    /// Where the mirrored copy is drawn, in screen coordinates. Larger than
    /// `bounds` is the normal case — that overflow is the zoom.
    pub destination: Rect,
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::mpsc::channel;
    use std::sync::{Mutex, OnceLock};

    use windows::core::w;
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Dwm::{
        DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
        DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION,
        DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
    };
    use windows::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH, HBRUSH};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, IsWindowVisible,
        PostMessageW, PostQuitMessage, PostThreadMessageW, RegisterClassExW,
        SetLayeredWindowAttributes, SetWindowPos, ShowWindow, TranslateMessage, CW_USEDEFAULT,
        HTTRANSPARENT, HWND_TOPMOST, LWA_ALPHA, MSG, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE,
        WM_APP, WM_DESTROY, WM_NCHITTEST, WM_QUIT, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    use super::Spec;
    use crate::error::Error;

    /// Wakes the overlay thread after `WANTED` changes.
    const WM_SYNC: u32 = WM_APP + 1;

    static WINDOW: AtomicIsize = AtomicIsize::new(0);
    static THUMBNAIL: AtomicIsize = AtomicIsize::new(0);
    static MIRRORED: AtomicIsize = AtomicIsize::new(0);
    static WANTED: OnceLock<Mutex<Option<Spec>>> = OnceLock::new();

    fn wanted() -> &'static Mutex<Option<Spec>> {
        WANTED.get_or_init(|| Mutex::new(None))
    }

    pub struct Overlay {
        thread_id: u32,
    }

    impl Overlay {
        pub fn start() -> crate::Result<Overlay> {
            let (ready_tx, ready_rx) = channel::<crate::Result<u32>>();
            std::thread::Builder::new()
                .name("tlk-grid-overlay".into())
                .spawn(move || pump(ready_tx))
                .map_err(|_| Error::win32("CreateThread", -1))?;
            let thread_id = ready_rx
                .recv()
                .map_err(|_| Error::win32("CreateWindowEx", -1))??;
            Ok(Overlay { thread_id })
        }

        pub fn show(&self, spec: Spec) {
            *wanted().lock().unwrap_or_else(|e| e.into_inner()) = Some(spec);
            wake();
        }

        pub fn hide(&self) {
            *wanted().lock().unwrap_or_else(|e| e.into_inner()) = None;
            wake();
        }

        pub fn is_up(&self) -> bool {
            wanted()
                .lock()
                .map(|state| state.is_some())
                .unwrap_or(false)
        }
    }

    impl Drop for Overlay {
        fn drop(&mut self) {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }

    fn wake() {
        let hwnd = WINDOW.load(Ordering::Acquire);
        if hwnd != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(hwnd as *mut _)), WM_SYNC, WPARAM(0), LPARAM(0));
            }
        }
    }

    fn pump(ready: std::sync::mpsc::Sender<crate::Result<u32>>) {
        unsafe {
            let instance = match GetModuleHandleW(None) {
                Ok(i) => i,
                Err(e) => {
                    let _ = ready.send(Err(Error::win32("GetModuleHandle", e.code().0)));
                    return;
                }
            };

            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(overlay_proc),
                hInstance: instance.into(),
                lpszClassName: w!("tlk_grid_overlay"),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                ..Default::default()
            };
            RegisterClassExW(&class);

            // No WS_EX_APPWINDOW and no activation: the overlay must never take
            // focus from the game or appear in Alt-Tab.
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT
                    | WS_EX_LAYERED,
                w!("tlk_grid_overlay"),
                w!("tlk-grid overlay"),
                WS_POPUP,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                16,
                16,
                None,
                None,
                Some(instance.into()),
                None,
            );
            let hwnd = match hwnd {
                Ok(h) => h,
                Err(e) => {
                    let _ = ready.send(Err(Error::win32("CreateWindowEx", e.code().0)));
                    return;
                }
            };

            // Constant alpha rather than per-pixel: UpdateLayeredWindow would
            // take the window off the compositor's normal path and the DWM
            // thumbnail would never appear.
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);

            WINDOW.store(hwnd.0 as isize, Ordering::Release);
            let _ = ready.send(Ok(GetCurrentThreadId()));

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            release_thumbnail();
        }
    }

    unsafe fn release_thumbnail() {
        let handle = THUMBNAIL.swap(0, Ordering::AcqRel);
        if handle != 0 {
            let _ = DwmUnregisterThumbnail(handle);
        }
        MIRRORED.store(0, Ordering::Release);
    }

    unsafe fn sync() {
        let hwnd = HWND(WINDOW.load(Ordering::Acquire) as *mut _);
        if hwnd.is_invalid() {
            return;
        }

        let spec = wanted().lock().map(|state| *state).unwrap_or(None);

        let Some(spec) = spec else {
            release_thumbnail();
            let _ = ShowWindow(hwnd, SW_HIDE);
            return;
        };

        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            spec.bounds.x,
            spec.bounds.y,
            spec.bounds.w,
            spec.bounds.h,
            SWP_NOACTIVATE,
        );

        if !IsWindowVisible(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }

        // Re-registering on every frame would be wasteful; only a new source
        // needs a new thumbnail.
        if MIRRORED.load(Ordering::Acquire) != spec.source {
            release_thumbnail();
            match DwmRegisterThumbnail(hwnd, HWND(spec.source as *mut _)) {
                Ok(thumb) => {
                    THUMBNAIL.store(thumb, Ordering::Release);
                    MIRRORED.store(spec.source, Ordering::Release);
                }
                Err(_) => {
                    // The source went away between the bind and here.
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    return;
                }
            }
        }

        let handle = THUMBNAIL.load(Ordering::Acquire);
        if handle == 0 {
            return;
        }

        // rcDestination is relative to the overlay's client area, and DWM clips
        // it for us — which is exactly the magnification.
        let dest = RECT {
            left: spec.destination.x - spec.bounds.x,
            top: spec.destination.y - spec.bounds.y,
            right: spec.destination.right() - spec.bounds.x,
            bottom: spec.destination.bottom() - spec.bounds.y,
        };
        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION
                | DWM_TNP_VISIBLE
                | DWM_TNP_OPACITY
                | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: dest,
            opacity: 255,
            fVisible: true.into(),
            fSourceClientAreaOnly: true.into(),
            ..Default::default()
        };
        let _ = DwmUpdateThumbnailProperties(handle, &props);
    }

    unsafe extern "system" fn overlay_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_SYNC => {
                sync();
                LRESULT(0)
            }
            // Belt and braces next to WS_EX_TRANSPARENT: the crosshair has to
            // stay clickable-through while the zoom is held.
            WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}

#[cfg(windows)]
pub use imp::Overlay;

#[cfg(not(windows))]
pub struct Overlay;

#[cfg(not(windows))]
impl Overlay {
    pub fn start() -> crate::Result<Overlay> {
        Ok(Overlay)
    }
    pub fn show(&self, _spec: Spec) {}
    pub fn hide(&self) {}
    pub fn is_up(&self) -> bool {
        false
    }
}
