use serde::{Deserialize, Serialize};
use tauri::State;
use tlkgrid_core::bind::{key_label, Action, Bind, Trigger};
use tlkgrid_core::display::{self, Monitor};
use tlkgrid_core::frame::{self, Border};
use tlkgrid_core::input::Input;
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
    /// Re-assert this layout when the game moves the window itself.
    #[serde(default)]
    pub pin: bool,
}

#[tauri::command]
pub fn apply_placement(
    session: State<'_, Session>,
    input: State<'_, Input>,
    request: Placement,
) -> Answer<Rect> {
    if !session.is_touched(request.handle) {
        session.remember(frame::capture(request.handle).map_err(fail)?);
    }
    let border = if request.borderless {
        Border::Borderless
    } else {
        Border::Keep
    };
    let placed = frame::place(request.handle, request.rect, border).map_err(fail)?;
    if request.pin {
        session.pin(request.handle, placed, border);
        input.watch(request.handle);
    } else {
        session.unpin(request.handle);
        input.unwatch(request.handle);
    }
    Ok(placed)
}

#[tauri::command]
pub fn window_rect(handle: isize) -> Answer<Rect> {
    frame::visible_rect(handle).map_err(fail)
}

/// One card's Close button: put that window back the way it was.
#[tauri::command]
pub fn release_window(
    session: State<'_, Session>,
    input: State<'_, Input>,
    handle: isize,
) -> Answer<()> {
    input.unwatch(handle);
    match session.release(handle) {
        Some(original) => frame::restore(&original).map_err(fail),
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

// ------------------------------------------------------------------- binds

/// The bind field is armed. The next key or mouse button the hook sees is
/// reported back as an `input` event and never reaches the focused app — which
/// is the only way to capture M4/M5, since a webview keydown cannot see them.
#[tauri::command]
pub fn begin_bind_capture(input: State<'_, Input>) {
    input.begin_capture();
}

#[tauri::command]
pub fn cancel_bind_capture(input: State<'_, Input>) {
    input.cancel_capture();
}

/// Replaces the whole bind set. F8 is appended here rather than stored in the
/// profile, so a hand-edited profile can never delete the escape hatch.
#[tauri::command]
pub fn set_binds(input: State<'_, Input>, binds: Vec<Bind>) {
    let mut all: Vec<Bind> = binds
        .into_iter()
        .filter(|b| b.action != Action::Panic)
        .collect();
    all.push(tlkgrid_core::bind::panic_bind());
    input.set_binds(all);
}

/// Binds other than Master and Panic only fire while this window is in front.
#[tauri::command]
pub fn set_bind_target(input: State<'_, Input>, handle: Option<isize>) {
    input.set_target(handle);
}

#[tauri::command]
pub fn set_master(input: State<'_, Input>, enabled: bool) {
    input.set_master(enabled);
}

/// The toolbar's wheel button: whether scrolling adjusts the factor of the bind
/// currently held down.
#[tauri::command]
pub fn set_wheel_adjusts(input: State<'_, Input>, enabled: bool) {
    input.set_wheel_adjusts(enabled);
}

/// How a captured trigger should read in the bind field.
#[tauri::command]
pub fn trigger_label(trigger: Trigger) -> String {
    trigger.label()
}

#[tauri::command]
pub fn key_name(vk: u32) -> String {
    key_label(vk)
}
