//! Proof that the DWM thumbnail actually reaches the screen.
//!
//! Everything else about the overlay can be unit tested; whether the compositor
//! honours `DwmRegisterThumbnail` on a layered, click-through, no-activate
//! window cannot be — it either draws or it does not. This shows the overlay
//! over a small patch of screen, grabs that patch, and checks it is not the flat
//! black of the window's own brush.
//!
//! Ignored by default: it needs a desktop with real windows on it, so it is a
//! manual check rather than a CI gate.
//!
//! `cargo test -p tlkgrid-core --test overlay_smoke -- --ignored --nocapture`

#![cfg(windows)]

use std::thread::sleep;
use std::time::Duration;

use tlkgrid_core::layout::Rect;
use tlkgrid_core::overlay::{Overlay, Spec};
use tlkgrid_core::{display, target};

/// Reads a patch of the screen and reports how many distinct greys it holds.
/// Flat black is one.
fn screen_variety(area: Rect) -> usize {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetPixel, ReleaseDC, SelectObject, SRCCOPY,
    };

    unsafe {
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, area.w, area.h);
        let previous = SelectObject(memory, bitmap.into());
        let _ = BitBlt(
            memory,
            0,
            0,
            area.w,
            area.h,
            Some(screen),
            area.x,
            area.y,
            SRCCOPY,
        );

        let mut seen = std::collections::HashSet::new();
        let step = (area.w / 24).max(1);
        let mut y = 0;
        while y < area.h {
            let mut x = 0;
            while x < area.w {
                seen.insert(GetPixel(memory, x, y).0);
                x += step;
            }
            y += step;
        }

        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        seen.len()
    }
}

#[test]
#[ignore = "needs a desktop with real windows"]
fn the_compositor_draws_the_mirrored_window() {
    let monitor = display::primary().expect("no monitor");
    let windows = target::list().expect("enumeration failed");

    // The biggest window on screen is the least likely to be a flat colour.
    let source = windows
        .iter()
        .filter(|w| !w.minimized && w.rect.w > 300 && w.rect.h > 200)
        .max_by_key(|w| w.rect.w * w.rect.h)
        .expect("no window big enough to mirror");
    println!("mirroring {}", source.label());

    // A corner patch, so the check does not black out the whole screen.
    let patch = Rect::new(monitor.bounds.x + 40, monitor.bounds.y + 40, 480, 300);

    let overlay = Overlay::start().expect("overlay thread");
    overlay.show(Spec {
        source: source.handle,
        bounds: patch,
        // Same size as the patch: a 1:1 mirror is enough to prove it draws.
        destination: patch,
    });
    sleep(Duration::from_millis(600));

    let shown = screen_variety(patch);
    overlay.hide();
    sleep(Duration::from_millis(300));

    println!("distinct colours under the overlay: {shown}");
    assert!(
        shown > 1,
        "the patch was one flat colour, so DWM drew the window's brush and not \
         the thumbnail — the overlay's extended styles are wrong"
    );
}

/// The magnification itself: the same patch, once at 1:1 and once with the
/// destination blown up past its edges. If clipping a bigger destination did
/// not magnify, the two would look the same.
#[test]
#[ignore = "needs a desktop with real windows"]
fn a_bigger_destination_changes_what_the_patch_shows() {
    use tlkgrid_core::zoom;

    let monitor = display::primary().expect("no monitor");
    let source = target::list()
        .expect("enumeration failed")
        .into_iter()
        .filter(|w| !w.minimized && w.rect.w > 300 && w.rect.h > 200)
        .max_by_key(|w| w.rect.w * w.rect.h)
        .expect("no window big enough to mirror");

    let patch = Rect::new(monitor.bounds.x + 40, monitor.bounds.y + 40, 480, 300);
    let overlay = Overlay::start().expect("overlay thread");

    overlay.show(Spec {
        source: source.handle,
        bounds: patch,
        destination: patch,
    });
    sleep(Duration::from_millis(600));
    let plain = screen_variety(patch);

    overlay.show(Spec {
        source: source.handle,
        bounds: patch,
        destination: zoom::destination(patch, 4.0),
    });
    sleep(Duration::from_millis(600));
    let zoomed = screen_variety(patch);

    overlay.hide();
    sleep(Duration::from_millis(300));

    println!("colours 1:1 = {plain}, at 4x = {zoomed}");
    assert!(plain > 1 && zoomed > 1, "the thumbnail did not draw");
    assert_ne!(
        plain, zoomed,
        "1:1 and 4x produced the same picture, so rcDestination is being ignored"
    );
}
