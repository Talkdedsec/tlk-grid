use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn from_ltrb(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            x: left,
            y: top,
            w: right - left,
            h: bottom - top,
        }
    }

    pub const fn right(&self) -> i32 {
        self.x + self.w
    }

    pub const fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub const fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// Move (never resize) so the rect sits inside `outer` when it fits.
    pub fn nudge_into(mut self, outer: Rect) -> Rect {
        if self.w <= outer.w {
            self.x = self.x.clamp(outer.x, outer.right() - self.w);
        }
        if self.h <= outer.h {
            self.y = self.y.clamp(outer.y, outer.bottom() - self.h);
        }
        self
    }

    pub fn centered_in(self, outer: Rect) -> Rect {
        Rect::new(
            outer.x + (outer.w - self.w) / 2,
            outer.y + (outer.h - self.h) / 2,
            self.w,
            self.h,
        )
    }

    /// Grow or shrink around the midpoint. Used by every zoom method.
    pub fn scaled_about_center(self, factor: f64) -> Rect {
        let (cx, cy) = self.center();
        let w = ((self.w as f64) * factor).round() as i32;
        let h = ((self.h as f64) * factor).round() as i32;
        Rect::new(cx - w / 2, cy - h / 2, w.max(1), h.max(1))
    }
}

/// The five quick-placement buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

impl Anchor {
    /// Left/Right/Top/Bottom mean "this half of the screen", matching the toolbar.
    pub fn apply(self, work: Rect, size: Option<(i32, i32)>) -> Rect {
        let half_w = work.w / 2;
        let half_h = work.h / 2;
        match self {
            Anchor::Center => {
                let (w, h) = size.unwrap_or((work.w, work.h));
                Rect::new(0, 0, w, h).centered_in(work)
            }
            Anchor::Left => Rect::new(work.x, work.y, half_w, work.h),
            Anchor::Right => Rect::new(work.x + half_w, work.y, work.w - half_w, work.h),
            Anchor::Top => Rect::new(work.x, work.y, work.w, half_h),
            Anchor::Bottom => Rect::new(work.x, work.y + half_h, work.w, work.h - half_h),
        }
    }
}

/// A numbered cell of the ×4 / ×6 / ×8 / ×10 split panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub index: u32,
    pub rect: Rect,
}

/// Every split is two rows deep; the count only changes how many columns there are.
pub fn split_columns(divisions: u32) -> u32 {
    (divisions / 2).max(1)
}

pub fn split(work: Rect, divisions: u32) -> Vec<Cell> {
    let cols = split_columns(divisions);
    let rows = if divisions >= 2 { 2 } else { 1 };
    let mut cells = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        for col in 0..cols {
            // Distribute the remainder so the cells tile the work area exactly.
            let x0 = work.x + (work.w * col as i32) / cols as i32;
            let x1 = work.x + (work.w * (col + 1) as i32) / cols as i32;
            let y0 = work.y + (work.h * row as i32) / rows as i32;
            let y1 = work.y + (work.h * (row + 1) as i32) / rows as i32;
            cells.push(Cell {
                index: row * cols + col + 1,
                rect: Rect::from_ltrb(x0, y0, x1, y1),
            });
        }
    }
    cells
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AspectPreset {
    Ultrawide21x9,
    Superwide32x9,
    Full16x9,
    Classic4x3,
    Wide16x10,
    Square1x1,
}

impl AspectPreset {
    pub const ALL: [AspectPreset; 6] = [
        AspectPreset::Ultrawide21x9,
        AspectPreset::Superwide32x9,
        AspectPreset::Full16x9,
        AspectPreset::Classic4x3,
        AspectPreset::Wide16x10,
        AspectPreset::Square1x1,
    ];

    pub const fn ratio(self) -> (i32, i32) {
        match self {
            AspectPreset::Ultrawide21x9 => (21, 9),
            AspectPreset::Superwide32x9 => (32, 9),
            AspectPreset::Full16x9 => (16, 9),
            AspectPreset::Classic4x3 => (4, 3),
            AspectPreset::Wide16x10 => (16, 10),
            AspectPreset::Square1x1 => (1, 1),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AspectPreset::Ultrawide21x9 => "21:9",
            AspectPreset::Superwide32x9 => "32:9",
            AspectPreset::Full16x9 => "16:9",
            AspectPreset::Classic4x3 => "4:3",
            AspectPreset::Wide16x10 => "16:10",
            AspectPreset::Square1x1 => "1:1",
        }
    }

    /// Largest box of this ratio that still fits the monitor, centered.
    /// 32:9 on a 1920×1080 screen lands on 1920×540 at y=270 — the superwide recipe.
    pub fn fit(self, work: Rect) -> Rect {
        let (aw, ah) = self.ratio();
        contain(work, aw, ah)
    }
}

pub fn contain(work: Rect, aspect_w: i32, aspect_h: i32) -> Rect {
    debug_assert!(aspect_w > 0 && aspect_h > 0);
    let by_width = Rect::new(0, 0, work.w, (work.w * aspect_h) / aspect_w);
    let box_ = if by_width.h <= work.h {
        by_width
    } else {
        Rect::new(0, 0, (work.h * aspect_w) / aspect_h, work.h)
    };
    box_.centered_in(work)
}

/// The four letterbox strips left over once the window is placed on the monitor.
/// Empty strips are dropped, so a full-width window yields only top and bottom.
pub fn letterbox(monitor: Rect, window: Rect) -> Vec<Rect> {
    let top = Rect::from_ltrb(
        monitor.x,
        monitor.y,
        monitor.right(),
        window.y.max(monitor.y),
    );
    let bottom = Rect::from_ltrb(
        monitor.x,
        window.bottom().min(monitor.bottom()),
        monitor.right(),
        monitor.bottom(),
    );
    let left = Rect::from_ltrb(
        monitor.x,
        window.y,
        window.x.max(monitor.x),
        window.bottom(),
    );
    let right = Rect::from_ltrb(
        window.right().min(monitor.right()),
        window.y,
        monitor.right(),
        window.bottom(),
    );
    [top, bottom, left, right]
        .into_iter()
        .filter(|r| !r.is_empty())
        .collect()
}

/// Zoom factor accepted by the RESIZE field: 0.01× to 64×.
pub const ZOOM_MIN: f64 = 0.01;
pub const ZOOM_MAX: f64 = 64.0;

pub fn clamp_zoom(factor: f64) -> f64 {
    if factor.is_finite() {
        factor.clamp(ZOOM_MIN, ZOOM_MAX)
    } else {
        1.0
    }
}

/// Accepts both `1.5` and `1,5` — the decimal comma is what a Turkish or Russian
/// keyboard produces, and the field is typed into constantly.
pub fn parse_zoom(text: &str) -> Option<f64> {
    let normalized = text.trim().replace(',', ".");
    normalized.parse::<f64>().ok().map(clamp_zoom)
}

/// Powers the `Result: 5760 × 1620 (base 1920×540)` readout under the bind row.
pub fn zoom_result(base: (i32, i32), factor: f64) -> (i32, i32) {
    let f = clamp_zoom(factor);
    (
        ((base.0 as f64) * f).round() as i32,
        ((base.1 as f64) * f).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: Rect = Rect::new(0, 0, 1920, 1080);

    #[test]
    fn superwide_matches_the_1920x540_recipe() {
        assert_eq!(
            AspectPreset::Superwide32x9.fit(FHD),
            Rect::new(0, 270, 1920, 540)
        );
    }

    #[test]
    fn classic_is_pillarboxed_not_cropped() {
        assert_eq!(
            AspectPreset::Classic4x3.fit(FHD),
            Rect::new(240, 0, 1440, 1080)
        );
    }

    #[test]
    fn full_aspect_fills_the_monitor() {
        assert_eq!(AspectPreset::Full16x9.fit(FHD), FHD);
    }

    #[test]
    fn six_way_split_is_three_by_two() {
        let cells = split(FHD, 6);
        assert_eq!(cells.len(), 6);
        assert_eq!(cells[0].rect, Rect::new(0, 0, 640, 540));
        assert_eq!(cells[2].rect, Rect::new(1280, 0, 640, 540));
        assert_eq!(cells[5].rect, Rect::new(1280, 540, 640, 540));
        assert_eq!(cells[5].index, 6);
    }

    #[test]
    fn splits_tile_the_work_area_without_gaps() {
        for divisions in [4, 6, 8, 10] {
            let cells = split(Rect::new(0, 0, 1367, 769), divisions);
            let area: i32 = cells.iter().map(|c| c.rect.w * c.rect.h).sum();
            assert_eq!(area, 1367 * 769, "{divisions}-way split leaves a gap");
        }
    }

    #[test]
    fn halves_cover_the_whole_screen_on_odd_widths() {
        let work = Rect::new(0, 0, 1367, 768);
        let left = Anchor::Left.apply(work, None);
        let right = Anchor::Right.apply(work, None);
        assert_eq!(left.right(), right.x);
        assert_eq!(right.right(), work.right());
    }

    #[test]
    fn letterbox_drops_empty_strips() {
        let bars = letterbox(FHD, Rect::new(0, 270, 1920, 540));
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0], Rect::new(0, 0, 1920, 270));
        assert_eq!(bars[1], Rect::new(0, 810, 1920, 270));
    }

    #[test]
    fn zoom_accepts_a_decimal_comma() {
        assert_eq!(parse_zoom("2,90"), Some(2.90));
        assert_eq!(parse_zoom(" 1.5 "), Some(1.5));
        assert_eq!(parse_zoom("nope"), None);
    }

    #[test]
    fn zoom_is_clamped_to_the_documented_range() {
        assert_eq!(parse_zoom("999"), Some(ZOOM_MAX));
        assert_eq!(parse_zoom("0"), Some(ZOOM_MIN));
    }

    #[test]
    fn zoom_result_matches_the_readout() {
        assert_eq!(zoom_result((1920, 540), 3.0), (5760, 1620));
    }

    #[test]
    fn scaling_keeps_the_midpoint() {
        let r = Rect::new(0, 270, 1920, 540);
        let scaled = r.scaled_about_center(2.0);
        assert_eq!(scaled.center(), r.center());
        assert_eq!((scaled.w, scaled.h), (3840, 1080));
    }
}
