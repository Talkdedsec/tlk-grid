//! Proof that the DWM thumbnail actually reaches the screen.
//!
//! Everything else about the layer engine can be unit tested; whether the
//! compositor honours `DwmRegisterThumbnail` on a layered, click-through,
//! no-activate window cannot be — it either draws or it does not.
//!
//! One test, not three: the layer engine keeps its state in process-wide slots
//! because the Win32 callbacks it drives take no user pointer, so two tests
//! running in parallel would fight over the same slot and report each other's
//! numbers.
//!
//! Ignored by default — it needs a desktop with real windows on it, so it is a
//! manual check rather than a CI gate.
//!
//! `cargo test -p tlkgrid-core --test layers_smoke -- --ignored --nocapture`

#![cfg(windows)]

use std::thread::sleep;
use std::time::Duration;

use tlkgrid_core::layers::{Layer, Layers, ZOOM};
use tlkgrid_core::layout::Rect;
use tlkgrid_core::{display, target, zoom};

const SETTLE: Duration = Duration::from_millis(600);

/// Reads a patch of the screen and reports how many distinct colours it holds.
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
fn the_compositor_honours_both_thumbnail_rects() {
    // The biggest window on screen is the least likely to be a flat colour.
    let source = target::list()
        .expect("enumeration failed")
        .into_iter()
        .filter(|w| !w.minimized && w.rect.w > 300 && w.rect.h > 200)
        .max_by_key(|w| w.rect.w * w.rect.h)
        .expect("no window big enough to mirror");
    println!("mirroring {}", source.label());

    // A corner patch, so the check never blacks out the whole screen.
    let monitor = display::primary().expect("no monitor");
    let patch = Rect::new(monitor.bounds.x + 40, monitor.bounds.y + 40, 480, 300);

    let layers = Layers::start().expect("layer thread");

    // 1 — it draws at all.
    layers.set(ZOOM, Layer::mirror(source.handle, patch, patch));
    sleep(SETTLE);
    let plain = screen_variety(patch);

    // 2 — rcDestination past the edges magnifies instead of being ignored.
    layers.set(
        ZOOM,
        Layer::mirror(source.handle, patch, zoom::destination(patch, 4.0)),
    );
    sleep(SETTLE);
    let zoomed = screen_variety(patch);

    // 3 — rcSource crops. The scope lens and every HUD layer live on this.
    let third = Rect::new(0, 0, source.rect.w / 3, source.rect.h / 3);
    let middle = Rect::new(
        source.rect.w / 3,
        source.rect.h / 3,
        source.rect.w / 3,
        source.rect.h / 3,
    );
    layers.set(ZOOM, Layer::lens(source.handle, patch, third));
    sleep(SETTLE);
    let corner_crop = screen_variety(patch);

    layers.set(ZOOM, Layer::lens(source.handle, patch, middle));
    sleep(SETTLE);
    let middle_crop = screen_variety(patch);

    layers.remove(ZOOM);
    sleep(Duration::from_millis(300));

    println!("1:1 {plain} · 4x {zoomed} · crop corner {corner_crop} · crop middle {middle_crop}");
    assert!(
        plain > 1,
        "the patch was one flat colour, so DWM drew nothing into the layer — \
         the window's extended styles are wrong"
    );
    assert_ne!(
        plain, zoomed,
        "1:1 and 4x produced the same picture, so rcDestination is being ignored"
    );
    assert_ne!(
        corner_crop, middle_crop,
        "both regions produced the same picture, so rcSource is being ignored — \
         the scope lens and HUD layers depend on it"
    );
}
