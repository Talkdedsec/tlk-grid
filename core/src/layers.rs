//! Every overlay tlk-grid puts on screen, run from one thread.
//!
//! The zoom, the scope lens, the HUD cut-outs, the letterbox bars, the crosshair
//! and the artwork around the window are all the same thing: a click-through,
//! never-activated, always-on-top window with something drawn in it. Only the
//! *something* differs, and only in three ways:
//!
//! - `Mirror` — a live DWM copy of another window, optionally cropped. The zoom,
//!   the scope lens and every HUD layer are this one call with different rects.
//! - `Fill` — flat black, dimmable. The letterbox bars.
//! - `Picture` — a bitmap, animated if it came from a GIF. Crosshair and artwork.
//!
//! `Mirror` and `Fill` take constant-alpha layering, because per-pixel layering
//! puts a window on a path where DWM never draws the thumbnail into it.
//! `Picture` needs per-pixel alpha and hosts no thumbnail, so it uses
//! `UpdateLayeredWindow` instead. Keeping them in separate windows is what lets
//! both be true at once.

use std::sync::Arc;

use crate::layout::Rect;
use crate::picture::Picture;

pub type LayerId = u32;

/// Fixed slots, so callers never collide. HUD layers count up from `HUD`.
pub const ZOOM: LayerId = 1;
pub const SCOPE: LayerId = 2;
pub const CROSSHAIR: LayerId = 3;
pub const BAR_TOP: LayerId = 10;
pub const BAR_BOTTOM: LayerId = 11;
pub const BAR_LEFT: LayerId = 12;
pub const BAR_RIGHT: LayerId = 13;
pub const ART_TOP: LayerId = 20;
pub const ART_BOTTOM: LayerId = 21;
pub const ART_LEFT: LayerId = 22;
pub const ART_RIGHT: LayerId = 23;
pub const HUD: LayerId = 100;

pub const BARS: [LayerId; 4] = [BAR_TOP, BAR_BOTTOM, BAR_LEFT, BAR_RIGHT];
pub const ART: [LayerId; 4] = [ART_TOP, ART_BOTTOM, ART_LEFT, ART_RIGHT];

/// Stacking among our own overlays. The crosshair has to sit above the lens,
/// and the lens above the zoom, or the sniper view is useless.
pub fn depth(id: LayerId) -> i32 {
    match id {
        ZOOM => 0,
        BAR_TOP..=BAR_RIGHT => 1,
        ART_TOP..=ART_RIGHT => 2,
        SCOPE => 3,
        CROSSHAIR => 5,
        id if id >= HUD => 4,
        _ => 0,
    }
}

#[derive(Clone)]
pub enum Paint {
    /// Live copy of `source`. `region` crops the source in its own client
    /// coordinates; `None` mirrors all of it. `destination` is where the copy
    /// lands, in screen coordinates, and may reach past `bounds` — that overflow
    /// is what magnifies.
    Mirror {
        source: isize,
        region: Option<Rect>,
        destination: Rect,
    },
    /// Flat black. 255 hides what is behind it completely; less than that dims.
    Fill {
        alpha: u8,
    },
    Picture {
        picture: Arc<Picture>,
    },
}

#[derive(Clone)]
pub struct Layer {
    /// Where the overlay window sits on screen.
    pub bounds: Rect,
    pub paint: Paint,
}

impl Layer {
    pub fn mirror(source: isize, bounds: Rect, destination: Rect) -> Layer {
        Layer {
            bounds,
            paint: Paint::Mirror {
                source,
                region: None,
                destination,
            },
        }
    }

    /// A lens: the centre of the source, blown up to fill `bounds`.
    pub fn lens(source: isize, bounds: Rect, region: Rect) -> Layer {
        Layer {
            bounds,
            paint: Paint::Mirror {
                source,
                region: Some(region),
                destination: bounds,
            },
        }
    }

    pub fn bar(bounds: Rect) -> Layer {
        Layer {
            bounds,
            paint: Paint::Fill { alpha: 255 },
        }
    }

    pub fn picture(bounds: Rect, picture: Arc<Picture>) -> Layer {
        Layer {
            bounds,
            paint: Paint::Picture { picture },
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::mpsc::channel;
    use std::sync::{Mutex, OnceLock};

    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{
        COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
    };
    use windows::Win32::Graphics::Dwm::{
        DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
        DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE,
        DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
    };
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetStockObject,
        ReleaseDC, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        BLACK_BRUSH, BLENDFUNCTION, DIB_RGB_COLORS, HBRUSH,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
        PostMessageW, PostQuitMessage, PostThreadMessageW, RegisterClassExW,
        SetLayeredWindowAttributes, SetTimer, SetWindowPos, ShowWindow, TranslateMessage,
        UpdateLayeredWindow, CW_USEDEFAULT, HTTRANSPARENT, HWND_TOPMOST, LWA_ALPHA, MSG,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_APP,
        WM_DESTROY, WM_NCHITTEST, WM_QUIT, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    use super::{depth, Layer, LayerId, Paint};
    use crate::error::Error;
    use crate::layout::Rect;

    const WM_SYNC: u32 = WM_APP + 1;
    const FRAME_TIMER: usize = 1;

    static WANTED: OnceLock<Mutex<HashMap<LayerId, Option<Layer>>>> = OnceLock::new();
    static ANCHOR: AtomicIsize = AtomicIsize::new(0);

    fn wanted() -> &'static Mutex<HashMap<LayerId, Option<Layer>>> {
        WANTED.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// One overlay window and whatever it needs to keep drawing.
    struct Live {
        hwnd: HWND,
        thumbnail: isize,
        mirrored: isize,
        frames: Option<std::sync::Arc<crate::picture::Picture>>,
        frame: usize,
        size: (i32, i32),
        /// Fill windows come from a different class, so a layer that changes kind
        /// needs a new window rather than a repaint.
        filled: bool,
    }

    pub struct Layers {
        thread_id: u32,
    }

    impl Layers {
        pub fn start() -> crate::Result<Layers> {
            let (ready_tx, ready_rx) = channel::<crate::Result<u32>>();
            std::thread::Builder::new()
                .name("tlk-grid-layers".into())
                .spawn(move || pump(ready_tx))
                .map_err(|_| Error::win32("CreateThread", -1))?;
            let thread_id = ready_rx
                .recv()
                .map_err(|_| Error::win32("CreateWindowEx", -1))??;
            Ok(Layers { thread_id })
        }

        pub fn set(&self, id: LayerId, layer: Layer) {
            wanted()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id, Some(layer));
            wake();
        }

        pub fn remove(&self, id: LayerId) {
            wanted()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id, None);
            wake();
        }

        /// Takes down every overlay. F8 and "the game closed" both end here.
        pub fn clear(&self) {
            {
                let mut state = wanted().lock().unwrap_or_else(|e| e.into_inner());
                for id in ALL_SLOTS {
                    state.insert(id, None);
                }
                for id in 0..32 {
                    state.insert(super::HUD + id, None);
                }
            }
            wake();
        }
    }

    const ALL_SLOTS: [LayerId; 11] = [
        super::ZOOM,
        super::SCOPE,
        super::CROSSHAIR,
        super::BAR_TOP,
        super::BAR_BOTTOM,
        super::BAR_LEFT,
        super::BAR_RIGHT,
        super::ART_TOP,
        super::ART_BOTTOM,
        super::ART_LEFT,
        super::ART_RIGHT,
    ];

    impl Drop for Layers {
        fn drop(&mut self) {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }

    fn wake() {
        let anchor = ANCHOR.load(Ordering::Acquire);
        if anchor != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(anchor as *mut _)), WM_SYNC, WPARAM(0), LPARAM(0));
            }
        }
    }

    const CLASS_CLEAR: PCWSTR = w!("tlk_grid_layer");
    const CLASS_FILL: PCWSTR = w!("tlk_grid_fill");

    fn pump(ready: std::sync::mpsc::Sender<crate::Result<u32>>) {
        unsafe {
            let instance: HINSTANCE = match GetModuleHandleW(None) {
                Ok(i) => i.into(),
                Err(e) => {
                    let _ = ready.send(Err(Error::win32("GetModuleHandle", e.code().0)));
                    return;
                }
            };

            let clear = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(layer_proc),
                hInstance: instance,
                lpszClassName: CLASS_CLEAR,
                ..Default::default()
            };
            RegisterClassExW(&clear);

            // Black bars need no painting code at all: the class brush is the
            // whole feature.
            let fill = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(layer_proc),
                hInstance: instance,
                lpszClassName: CLASS_FILL,
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                ..Default::default()
            };
            RegisterClassExW(&fill);

            // A hidden 1×1 window owns the message queue, so `wake` has somewhere
            // to post even when no overlay exists yet.
            let anchor = match CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_CLEAR,
                w!("tlk-grid layers"),
                WS_POPUP,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1,
                1,
                None,
                None,
                Some(instance),
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    let _ = ready.send(Err(Error::win32("CreateWindowEx", e.code().0)));
                    return;
                }
            };
            ANCHOR.store(anchor.0 as isize, Ordering::Release);
            let _ = ready.send(Ok(GetCurrentThreadId()));

            let mut live: HashMap<LayerId, Live> = HashMap::new();
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                match msg.message {
                    WM_SYNC => sync(instance, &mut live),
                    WM_TIMER => advance_frame(&mut live, msg.hwnd),
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }

            for (_, entry) in live.drain() {
                teardown(entry);
            }
            let _ = DestroyWindow(anchor);
        }
    }

    unsafe fn teardown(entry: Live) {
        if entry.thumbnail != 0 {
            let _ = DwmUnregisterThumbnail(entry.thumbnail);
        }
        let _ = KillTimer(Some(entry.hwnd), FRAME_TIMER);
        let _ = DestroyWindow(entry.hwnd);
    }

    unsafe fn sync(instance: HINSTANCE, live: &mut HashMap<LayerId, Live>) {
        let requests: Vec<(LayerId, Option<Layer>)> = {
            let mut state = wanted().lock().unwrap_or_else(|e| e.into_inner());
            state.drain().collect()
        };

        // Apply in stacking order so the last topmost placement wins.
        let mut requests = requests;
        requests.sort_by_key(|(id, _)| depth(*id));

        for (id, request) in requests {
            let Some(layer) = request else {
                if let Some(entry) = live.remove(&id) {
                    teardown(entry);
                }
                continue;
            };

            let wants_fill = matches!(layer.paint, Paint::Fill { .. });
            if live.get(&id).is_some_and(|e| e.filled != wants_fill) {
                if let Some(entry) = live.remove(&id) {
                    teardown(entry);
                }
            }

            let entry = live.entry(id).or_insert_with(|| Live {
                hwnd: create(instance, wants_fill),
                thumbnail: 0,
                mirrored: 0,
                frames: None,
                frame: 0,
                size: (0, 0),
                filled: wants_fill,
            });
            if entry.hwnd.is_invalid() {
                live.remove(&id);
                continue;
            }
            apply(entry, &layer);
        }
    }

    unsafe fn create(instance: HINSTANCE, fill: bool) -> HWND {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED,
            if fill { CLASS_FILL } else { CLASS_CLEAR },
            w!("tlk-grid layer"),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            16,
            16,
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap_or(HWND(std::ptr::null_mut()))
    }

    unsafe fn apply(entry: &mut Live, layer: &Layer) {
        let _ = SetWindowPos(
            entry.hwnd,
            Some(HWND_TOPMOST),
            layer.bounds.x,
            layer.bounds.y,
            layer.bounds.w,
            layer.bounds.h,
            SWP_NOACTIVATE,
        );
        entry.size = (layer.bounds.w, layer.bounds.h);

        match &layer.paint {
            Paint::Mirror {
                source,
                region,
                destination,
            } => {
                let _ = SetLayeredWindowAttributes(entry.hwnd, COLORREF(0), 255, LWA_ALPHA);
                if entry.mirrored != *source {
                    if entry.thumbnail != 0 {
                        let _ = DwmUnregisterThumbnail(entry.thumbnail);
                        entry.thumbnail = 0;
                    }
                    match DwmRegisterThumbnail(entry.hwnd, HWND(*source as *mut _)) {
                        Ok(thumb) => {
                            entry.thumbnail = thumb;
                            entry.mirrored = *source;
                        }
                        Err(_) => {
                            // The source went away between the bind and here.
                            let _ = ShowWindow(entry.hwnd, SW_HIDE);
                            return;
                        }
                    }
                }

                let mut props = DWM_THUMBNAIL_PROPERTIES {
                    dwFlags: DWM_TNP_RECTDESTINATION
                        | DWM_TNP_VISIBLE
                        | DWM_TNP_OPACITY
                        | DWM_TNP_SOURCECLIENTAREAONLY,
                    rcDestination: relative(*destination, layer.bounds),
                    opacity: 255,
                    fVisible: true.into(),
                    fSourceClientAreaOnly: true.into(),
                    ..Default::default()
                };
                if let Some(region) = region {
                    props.dwFlags |= DWM_TNP_RECTSOURCE;
                    props.rcSource = RECT {
                        left: region.x,
                        top: region.y,
                        right: region.right(),
                        bottom: region.bottom(),
                    };
                }
                let _ = DwmUpdateThumbnailProperties(entry.thumbnail, &props);
            }

            Paint::Fill { alpha } => {
                let _ = SetLayeredWindowAttributes(entry.hwnd, COLORREF(0), *alpha, LWA_ALPHA);
            }

            Paint::Picture { picture } => {
                let restart = entry
                    .frames
                    .as_ref()
                    .is_none_or(|current| !std::sync::Arc::ptr_eq(current, picture));
                if restart {
                    entry.frames = Some(picture.clone());
                    entry.frame = 0;
                }
                draw_frame(entry);
                let _ = KillTimer(Some(entry.hwnd), FRAME_TIMER);
                if picture.is_animated() {
                    SetTimer(
                        Some(entry.hwnd),
                        FRAME_TIMER,
                        picture.frames[entry.frame].delay_ms,
                        None,
                    );
                }
            }
        }

        let _ = ShowWindow(entry.hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            entry.hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        );
    }

    fn relative(destination: Rect, bounds: Rect) -> RECT {
        RECT {
            left: destination.x - bounds.x,
            top: destination.y - bounds.y,
            right: destination.right() - bounds.x,
            bottom: destination.bottom() - bounds.y,
        }
    }

    /// Blits one premultiplied frame into the window through
    /// `UpdateLayeredWindow`, which is the only way to get per-pixel alpha.
    unsafe fn draw_frame(entry: &mut Live) {
        let Some(picture) = entry.frames.clone() else {
            return;
        };
        let Some(source) = picture.frames.get(entry.frame) else {
            return;
        };
        let (width, height) = entry.size;
        if width <= 0 || height <= 0 {
            return;
        }

        let scaled = crate::picture::resize(source, width as u32, height as u32);

        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Negative height means top-down, matching our buffer order.
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();

        if let Ok(bitmap) =
            CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0)
        {
            if !bits.is_null() {
                let wanted = (width as usize) * (height as usize) * 4;
                std::ptr::copy_nonoverlapping(
                    scaled.bgra.as_ptr(),
                    bits as *mut u8,
                    scaled.bgra.len().min(wanted),
                );
            }
            let previous = SelectObject(memory, bitmap.into());
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let size = SIZE {
                cx: width,
                cy: height,
            };
            let origin = POINT { x: 0, y: 0 };
            let _ = UpdateLayeredWindow(
                entry.hwnd,
                Some(screen),
                None,
                Some(&size),
                Some(memory),
                Some(&origin),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            SelectObject(memory, previous);
            let _ = DeleteObject(bitmap.into());
        }

        // `info` is read by CreateDIBSection and must outlive the call.
        let _ = &mut info;
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
    }

    unsafe fn advance_frame(live: &mut HashMap<LayerId, Live>, hwnd: HWND) {
        let Some(entry) = live.values_mut().find(|e| e.hwnd == hwnd) else {
            return;
        };
        let Some(picture) = entry.frames.clone() else {
            return;
        };
        entry.frame = (entry.frame + 1) % picture.frames.len();
        draw_frame(entry);
        let _ = KillTimer(Some(hwnd), FRAME_TIMER);
        SetTimer(
            Some(hwnd),
            FRAME_TIMER,
            picture.frames[entry.frame].delay_ms,
            None,
        );
    }

    unsafe extern "system" fn layer_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            // Every overlay is click-through; aiming has to keep working.
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
pub use imp::Layers;

#[cfg(not(windows))]
pub struct Layers;

#[cfg(not(windows))]
impl Layers {
    pub fn start() -> crate::Result<Layers> {
        Ok(Layers)
    }
    pub fn set(&self, _id: LayerId, _layer: Layer) {}
    pub fn remove(&self, _id: LayerId) {}
    pub fn clear(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crosshair_sits_above_everything_it_has_to() {
        assert!(depth(CROSSHAIR) > depth(SCOPE));
        assert!(depth(SCOPE) > depth(ZOOM));
        assert!(depth(HUD) > depth(SCOPE));
        assert!(depth(BAR_TOP) > depth(ZOOM));
    }

    #[test]
    fn every_bar_and_art_slot_shares_its_neighbours_depth() {
        assert!(BARS.iter().all(|id| depth(*id) == depth(BAR_TOP)));
        assert!(ART.iter().all(|id| depth(*id) == depth(ART_TOP)));
    }
}
