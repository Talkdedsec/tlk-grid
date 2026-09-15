//! Turns what the user configured into layer rectangles.
//!
//! The layer engine knows how to draw; this decides where. Keeping the two
//! apart means the geometry can be reasoned about without a compositor: the
//! scope lens is a centred square showing a centred crop, the bars are whatever
//! the monitor has left over once the window is placed, and the crosshair is a
//! scaled picture at the middle of the primary monitor.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tlkgrid_core::layers::{self, Layer, Layers, Paint};
use tlkgrid_core::layout::{self, Rect};
use tlkgrid_core::picture::Picture;

/// Lens sizes the settings window offers, matching the source app's range.
pub const LENS_MIN: i32 = 100;
pub const LENS_MAX: i32 = 1200;

/// Crosshair scale, as a percentage of the image's own size.
pub const CROSSHAIR_MIN: u32 = 25;
pub const CROSSHAIR_MAX: u32 = 400;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LensBackdrop {
    /// The lens sits on black; everything around it is hidden.
    Black,
    /// Only the lens is drawn; the rest of the screen stays at 1×.
    SeeThrough,
}

#[derive(Clone)]
pub struct Crosshair {
    pub picture: Arc<Picture>,
    /// Copy inside app data, so the choice survives the original file moving.
    pub source: PathBuf,
    pub scale_percent: u32,
    pub visible: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    scale_percent: u32,
    visible: bool,
}

impl Crosshair {
    fn settings_path(image: &std::path::Path) -> PathBuf {
        image.with_extension("json")
    }

    pub fn save(&self) {
        let saved = Saved {
            scale_percent: self.scale_percent,
            visible: self.visible,
        };
        if let Ok(text) = serde_json::to_string(&saved) {
            let _ = std::fs::write(Self::settings_path(&self.source), text);
        }
    }

    /// Picks the crosshair back up on the next launch. A missing or unreadable
    /// file simply means there is no crosshair, never an error on the way in.
    pub fn restore(image: PathBuf) -> Option<Crosshair> {
        let picture = crate::overlays::load_picture(&image)?;
        let saved: Saved = std::fs::read_to_string(Self::settings_path(&image))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Saved {
                scale_percent: 100,
                visible: false,
            });
        Some(Crosshair {
            picture,
            source: image,
            scale_percent: saved.scale_percent.clamp(CROSSHAIR_MIN, CROSSHAIR_MAX),
            visible: saved.visible,
        })
    }
}

fn load_picture(path: &std::path::Path) -> Option<Arc<Picture>> {
    tlkgrid_core::picture::load(path).ok().map(Arc::new)
}

#[derive(Clone, Copy)]
pub struct Scope {
    pub enabled: bool,
    pub size: i32,
    pub backdrop: LensBackdrop,
}

impl Default for Scope {
    fn default() -> Self {
        Scope {
            enabled: false,
            size: 650,
            backdrop: LensBackdrop::SeeThrough,
        }
    }
}

#[derive(Default)]
pub struct Overlays {
    crosshair: Mutex<Option<Crosshair>>,
    scope: Mutex<Scope>,
}

impl Overlays {
    pub fn crosshair(&self) -> Option<Crosshair> {
        self.crosshair
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_crosshair(&self, crosshair: Option<Crosshair>) {
        *self.crosshair.lock().unwrap_or_else(|e| e.into_inner()) = crosshair;
    }

    pub fn scope(&self) -> Scope {
        *self.scope.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_scope(&self, scope: Scope) {
        *self.scope.lock().unwrap_or_else(|e| e.into_inner()) = scope;
    }
}

/// A square lens centred on the monitor, clamped to the documented range and to
/// what the monitor can actually show.
pub fn lens_bounds(monitor: Rect, size: i32) -> Rect {
    let side = size.clamp(LENS_MIN, LENS_MAX).min(monitor.w).min(monitor.h);
    Rect::new(0, 0, side, side).centered_in(monitor)
}

/// What the lens shows: the middle of the source, shrunk by the zoom factor so
/// that blowing it back up to the lens size magnifies by exactly that much.
pub fn lens_region(window: Rect, lens: Rect, factor: f64) -> Rect {
    let factor = layout::clamp_zoom(factor).max(1.0);
    let width = ((lens.w as f64 / factor).round() as i32).clamp(1, window.w);
    let height = ((lens.h as f64 / factor).round() as i32).clamp(1, window.h);
    // Source coordinates are the window's own client area, so the centre is
    // half its size rather than a screen position.
    Rect::new(
        (window.w - width) / 2,
        (window.h - height) / 2,
        width,
        height,
    )
}

/// Where a crosshair image lands: centred on the monitor at its scaled size.
pub fn crosshair_bounds(monitor: Rect, image: (u32, u32), scale_percent: u32) -> Rect {
    let percent = scale_percent.clamp(CROSSHAIR_MIN, CROSSHAIR_MAX);
    let width = ((image.0 * percent) / 100).max(1) as i32;
    let height = ((image.1 * percent) / 100).max(1) as i32;
    Rect::new(0, 0, width, height).centered_in(monitor)
}

// ------------------------------------------------------------------ painting

pub fn show_zoom(layers: &Layers, source: isize, monitor: Rect, destination: Rect) {
    layers.set(layers::ZOOM, Layer::mirror(source, monitor, destination));
}

pub fn hide_zoom(layers: &Layers) {
    layers.remove(layers::ZOOM);
}

/// The letterbox. Empty strips are removed rather than drawn as zero-height
/// windows, so a full-width window leaves only a top and a bottom bar.
pub fn show_bars(layers: &Layers, monitor: Rect, window: Rect) {
    let strips = layout::letterbox(monitor, window);
    for (slot, strip) in layers::BARS.iter().zip(strips.iter()) {
        layers.set(*slot, Layer::bar(*strip));
    }
    for slot in layers::BARS.iter().skip(strips.len()) {
        layers.remove(*slot);
    }
}

pub fn hide_bars(layers: &Layers) {
    for slot in layers::BARS {
        layers.remove(slot);
    }
}

pub fn show_scope(
    layers: &Layers,
    scope: Scope,
    source: isize,
    monitor: Rect,
    window: Rect,
    factor: f64,
) {
    let lens = lens_bounds(monitor, scope.size);
    let region = lens_region(window, lens, factor);
    layers.set(layers::SCOPE, Layer::lens(source, lens, region));

    // On black, the lens is the only thing visible; see-through leaves the rest
    // of the screen alone, which is what makes it read like a sniper scope.
    if scope.backdrop == LensBackdrop::Black {
        show_bars(layers, monitor, lens);
    } else {
        hide_bars(layers);
    }
}

pub fn hide_scope(layers: &Layers) {
    layers.remove(layers::SCOPE);
}

pub fn show_crosshair(layers: &Layers, crosshair: &Crosshair, monitor: Rect) {
    let bounds = crosshair_bounds(monitor, crosshair.picture.size(), crosshair.scale_percent);
    layers.set(
        layers::CROSSHAIR,
        Layer {
            bounds,
            paint: Paint::Picture {
                picture: crosshair.picture.clone(),
            },
        },
    );
}

pub fn hide_crosshair(layers: &Layers) {
    layers.remove(layers::CROSSHAIR);
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: Rect = Rect::new(0, 0, 1920, 1080);

    #[test]
    fn the_lens_is_square_and_centred() {
        let lens = lens_bounds(FHD, 650);
        assert_eq!((lens.w, lens.h), (650, 650));
        assert_eq!(lens.center(), FHD.center());
    }

    #[test]
    fn the_lens_cannot_outgrow_the_monitor() {
        let lens = lens_bounds(FHD, LENS_MAX);
        assert_eq!(lens.h, 1080, "a 1200px lens does not fit a 1080p screen");
        assert_eq!(lens.w, 1080, "and it has to stay square");
    }

    #[test]
    fn the_lens_range_is_clamped_at_both_ends() {
        assert_eq!(lens_bounds(FHD, 10).w, LENS_MIN);
        assert_eq!(lens_bounds(FHD, 9000).w, 1080);
    }

    #[test]
    fn a_higher_factor_crops_a_smaller_region() {
        let lens = lens_bounds(FHD, 600);
        let at_two = lens_region(FHD, lens, 2.0);
        let at_four = lens_region(FHD, lens, 4.0);
        assert_eq!((at_two.w, at_two.h), (300, 300));
        assert_eq!((at_four.w, at_four.h), (150, 150));
    }

    #[test]
    fn the_cropped_region_is_the_middle_of_the_window() {
        let lens = lens_bounds(FHD, 600);
        let region = lens_region(FHD, lens, 2.0);
        assert_eq!(region.center(), (FHD.w / 2, FHD.h / 2));
    }

    #[test]
    fn below_one_the_lens_shows_a_region_its_own_size() {
        let lens = lens_bounds(FHD, 400);
        let region = lens_region(FHD, lens, 0.5);
        assert_eq!(
            (region.w, region.h),
            (400, 400),
            "a lens never shrinks the view"
        );
    }

    #[test]
    fn a_crop_never_asks_for_more_than_the_window_holds() {
        let small = Rect::new(0, 0, 320, 240);
        let lens = lens_bounds(FHD, 1000);
        let region = lens_region(small, lens, 1.0);
        assert!(region.w <= small.w && region.h <= small.h);
    }

    #[test]
    fn the_crosshair_scales_around_the_centre() {
        let bounds = crosshair_bounds(FHD, (64, 64), 200);
        assert_eq!((bounds.w, bounds.h), (128, 128));
        assert_eq!(bounds.center(), FHD.center());
    }

    #[test]
    fn the_crosshair_scale_is_clamped_to_the_documented_range() {
        assert_eq!(crosshair_bounds(FHD, (100, 100), 5).w, 25);
        assert_eq!(crosshair_bounds(FHD, (100, 100), 5000).w, 400);
    }
}
