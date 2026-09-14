use serde::{Deserialize, Serialize};
use tauri::State;
use tlkgrid_core::display::{self, Monitor};
use tlkgrid_core::frame::{self, Border};
use tlkgrid_core::layout::{self, Anchor, AspectPreset, Cell, Rect};
use tlkgrid_core::target::{self, TargetWindow};

use crate::locale;
use crate::session::Session;

type Answer<T> = Result<T, String>;

fn fail(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn system_language() -> &'static str {
    locale::system_language()
}

#[tauri::command]
pub fn list_windows() -> Answer<Vec<TargetWindow>> {
    target::list().map_err(fail)
}

#[tauri::command]
pub fn list_monitors() -> Answer<Vec<Monitor>> {
    display::list().map_err(fail)
}

#[tauri::command]
pub fn monitor_for_window(handle: isize) -> Answer<Monitor> {
    display::for_window(handle).map_err(fail)
}

/// The cells of the ×4 / ×6 / ×8 / ×10 panel, drawn over the preview canvas.
#[tauri::command]
pub fn split_cells(work: Rect, divisions: u32) -> Vec<Cell> {
    layout::split(work, divisions)
}

#[tauri::command]
pub fn aspect_presets(work: Rect) -> Vec<AspectOption> {
    AspectPreset::ALL
        .iter()
        .map(|preset| AspectOption {
            id: *preset,
            label: preset.label().to_string(),
            rect: preset.fit(work),
        })
        .collect()
}

#[derive(Serialize)]
pub struct AspectOption {
    pub id: AspectPreset,
    pub label: String,
    pub rect: Rect,
}

#[tauri::command]
pub fn anchor_rect(work: Rect, anchor: Anchor, size: Option<(i32, i32)>) -> Rect {
    anchor.apply(work, size)
}

#[tauri::command]
pub fn letterbox_bars(monitor: Rect, window: Rect) -> Vec<Rect> {
    layout::letterbox(monitor, window)
}

/// `Result: 5760 × 1620 (base 1920×540)` — recomputed on every keystroke.
#[tauri::command]
pub fn zoom_preview(base_w: i32, base_h: i32, factor: String) -> Answer<Rect> {
    let f = layout::parse_zoom(&factor).ok_or_else(|| "not a number".to_string())?;
    let (w, h) = layout::zoom_result((base_w, base_h), f);
    Ok(Rect::new(0, 0, w, h))
}

#[derive(Deserialize)]
pub struct Placement {
    pub handle: isize,
    pub rect: Rect,
    #[serde(default)]
    pub borderless: bool,
}

#[tauri::command]
pub fn apply_placement(session: State<'_, Session>, request: Placement) -> Answer<Rect> {
    if !session.is_touched(request.handle) {
        session.remember(frame::capture(request.handle).map_err(fail)?);
    }
    let border = if request.borderless {
        Border::Borderless
    } else {
        Border::Keep
    };
    frame::place(request.handle, request.rect, border).map_err(fail)
}

#[tauri::command]
pub fn window_rect(handle: isize) -> Answer<Rect> {
    frame::visible_rect(handle).map_err(fail)
}

/// One card's Close button: put that window back the way it was.
#[tauri::command]
pub fn release_window(session: State<'_, Session>, handle: isize) -> Answer<()> {
    match session.forget(handle) {
        Some(state) => frame::restore(&state).map_err(fail),
        None => Ok(()),
    }
}

/// The F8 panic button. Restores every window tlk-grid has touched and reports
/// how many came back, so the status bar can say something true.
#[tauri::command]
pub fn restore_everything(session: State<'_, Session>) -> RestoreReport {
    let mut restored = 0;
    let mut failed = 0;
    for state in session.drain() {
        match frame::restore(&state) {
            Ok(()) => restored += 1,
            Err(_) => failed += 1,
        }
    }
    RestoreReport { restored, failed }
}

#[derive(Serialize)]
pub struct RestoreReport {
    pub restored: u32,
    pub failed: u32,
}
