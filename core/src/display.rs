//! Monitors: the surface every layout is measured against.
//!
//! The picker needs the real device name, the native mode and the refresh rate,
//! because SQUASH later has to prove a custom mode stays inside the panel's
//! limits, and the layout preview is drawn at native size.

use serde::{Deserialize, Serialize};

use crate::layout::Rect;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Monitor {
    /// `\\.\DISPLAY1` — what EnumDisplaySettings and NvAPI both key off.
    pub device: String,
    /// Friendly name when Windows knows one, else the device name.
    pub name: String,
    pub primary: bool,
    /// Full monitor bounds in virtual-desktop coordinates.
    pub bounds: Rect,
    /// Bounds minus taskbar and appbars; quick placement uses this.
    pub work: Rect,
    pub refresh_hz: u32,
    pub bits_per_pixel: u32,
    pub scale_percent: u32,
}

impl Monitor {
    pub fn label(&self) -> String {
        format!("{} {}×{}", self.name, self.bounds.w, self.bounds.h)
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW,
        MonitorFromWindow, DEVMODEW, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, HDC, HMONITOR,
        MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    use super::Monitor;
    use crate::error::{Error, Result};
    use crate::layout::Rect;

    /// MONITORINFOF_PRIMARY. The constant has moved between windows-rs releases,
    /// so it is spelled out rather than imported.
    const PRIMARY_FLAG: u32 = 0x0000_0001;

    fn wide_to_string(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    fn rect_of(r: RECT) -> Rect {
        Rect::from_ltrb(r.left, r.top, r.right, r.bottom)
    }

    /// The string on the monitor's own EDID, e.g. `Dell AW2523HF (DP SDR)`.
    /// Falls back to the device path when the driver publishes nothing.
    unsafe fn friendly_name(device: &str) -> Option<String> {
        let mut wide: Vec<u16> = device.encode_utf16().chain(std::iter::once(0)).collect();
        let mut info = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        EnumDisplayDevicesW(windows::core::PCWSTR(wide.as_mut_ptr()), 0, &mut info, 0)
            .as_bool()
            .then(|| wide_to_string(&info.DeviceString))
            .filter(|s| !s.is_empty())
    }

    unsafe fn current_mode(device: &str) -> (u32, u32) {
        let mut wide: Vec<u16> = device.encode_utf16().chain(std::iter::once(0)).collect();
        let mut mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        if EnumDisplaySettingsW(
            windows::core::PCWSTR(wide.as_mut_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut mode,
        )
        .as_bool()
        {
            (mode.dmDisplayFrequency, mode.dmBitsPerPel)
        } else {
            (0, 32)
        }
    }

    unsafe fn scale_percent(monitor: HMONITOR) -> u32 {
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok() {
            (dpi_x * 100) / 96
        } else {
            100
        }
    }

    unsafe fn describe(monitor: HMONITOR) -> Option<Monitor> {
        let mut info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, (&mut info as *mut MONITORINFOEXW).cast()).as_bool() {
            return None;
        }
        let device = wide_to_string(&info.szDevice);
        let (refresh_hz, bits_per_pixel) = current_mode(&device);
        Some(Monitor {
            name: friendly_name(&device).unwrap_or_else(|| device.clone()),
            primary: info.monitorInfo.dwFlags & PRIMARY_FLAG != 0,
            bounds: rect_of(info.monitorInfo.rcMonitor),
            work: rect_of(info.monitorInfo.rcWork),
            refresh_hz,
            bits_per_pixel,
            scale_percent: scale_percent(monitor),
            device,
        })
    }

    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _hdc: HDC,
        _clip: *mut RECT,
        lparam: LPARAM,
    ) -> windows::core::BOOL {
        let found = &mut *(lparam.0 as *mut Vec<Monitor>);
        if let Some(m) = describe(monitor) {
            found.push(m);
        }
        TRUE
    }

    pub fn list() -> Result<Vec<Monitor>> {
        let mut found: Vec<Monitor> = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(collect),
                LPARAM(&mut found as *mut _ as isize),
            )
            .ok()
            .map_err(|e| Error::win32("EnumDisplayMonitors", e.code().0))?;
        }
        found.sort_by_key(|m| (!m.primary, m.bounds.x, m.bounds.y));
        Ok(found)
    }

    pub fn primary() -> Result<Monitor> {
        list()?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NoSuchMonitor("primary".into()))
    }

    /// The monitor a target window currently sits on — what its layout is measured against.
    pub fn for_window(handle: isize) -> Result<Monitor> {
        let monitor =
            unsafe { MonitorFromWindow(HWND(handle as *mut _), MONITOR_DEFAULTTONEAREST) };
        unsafe { describe(monitor) }
            .ok_or_else(|| Error::NoSuchMonitor(format!("window {handle:#x}")))
    }

    pub fn by_device(device: &str) -> Result<Monitor> {
        list()?
            .into_iter()
            .find(|m| m.device == device)
            .ok_or_else(|| Error::NoSuchMonitor(device.to_string()))
    }
}

#[cfg(windows)]
pub use imp::{by_device, for_window, list, primary};
