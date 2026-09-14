//! Discovery of the windows a user can point tlk-grid at.
//!
//! Two windows of the same game share a title, so a title alone cannot address
//! one of them. Every entry carries its HWND and PID, and duplicates of a title
//! get a 1-based occurrence number in top-to-bottom Z order — the `#1 #2 #3`
//! suffix shown in the picker.

use serde::{Deserialize, Serialize};

use crate::layout::Rect;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetWindow {
    /// HWND as an integer; the handle type itself is not thread-safe to hold.
    pub handle: isize,
    pub pid: u32,
    pub title: String,
    pub process: String,
    /// 1-based position among windows sharing this title, or `None` when unique.
    pub occurrence: Option<u32>,
    pub rect: Rect,
    pub minimized: bool,
}

impl TargetWindow {
    /// What the dropdown shows: `Counter-Strike 2 (cs2.exe) #2`.
    pub fn label(&self) -> String {
        let mut label = if self.process.is_empty() {
            self.title.clone()
        } else {
            format!("{} ({})", self.title, self.process)
        };
        if let Some(n) = self.occurrence {
            label.push_str(&format!(" #{n}"));
        }
        label
    }
}

#[cfg(windows)]
mod imp {
    use std::collections::HashMap;

    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, MAX_PATH, RECT, TRUE};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW,
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, GWL_EXSTYLE,
        GW_OWNER, WS_EX_TOOLWINDOW,
    };

    use super::TargetWindow;
    use crate::error::{Error, Result};
    use crate::layout::Rect;

    /// Windows that exist but should never appear in the picker: invisible ones,
    /// tool windows, owned dialogs, and the cloaked shells UWP leaves behind.
    unsafe fn is_pickable(hwnd: HWND) -> bool {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        if GetWindowTextLengthW(hwnd) == 0 {
            return false;
        }
        if GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.is_invalid()) {
            return false;
        }
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }
        let mut cloaked = 0u32;
        let queried = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        );
        queried.is_err() || cloaked == 0
    }

    unsafe fn window_title(hwnd: HWND) -> String {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let written = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..written as usize])
    }

    unsafe fn process_name(pid: u32) -> String {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buf = [0u16; MAX_PATH as usize];
        let mut len = buf.len() as u32;
        let name = if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok()
        {
            String::from_utf16_lossy(&buf[..len as usize])
                .rsplit('\\')
                .next()
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };
        let _ = CloseHandle(handle);
        name
    }

    unsafe fn window_rect(hwnd: HWND) -> Rect {
        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r).is_ok() {
            Rect::from_ltrb(r.left, r.top, r.right, r.bottom)
        } else {
            Rect::new(0, 0, 0, 0)
        }
    }

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let found = &mut *(lparam.0 as *mut Vec<TargetWindow>);
        if is_pickable(hwnd) {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            found.push(TargetWindow {
                handle: hwnd.0 as isize,
                pid,
                title: window_title(hwnd),
                process: process_name(pid),
                occurrence: None,
                rect: window_rect(hwnd),
                minimized: IsIconic(hwnd).as_bool(),
            });
        }
        TRUE
    }

    /// Enumerated in Z order, so `#1` is always the frontmost of a duplicate set.
    pub fn list() -> Result<Vec<TargetWindow>> {
        let mut found: Vec<TargetWindow> = Vec::new();
        unsafe {
            EnumWindows(Some(collect), LPARAM(&mut found as *mut _ as isize))
                .map_err(|e| Error::win32("EnumWindows", e.code().0))?;
        }
        number_duplicates(&mut found);
        Ok(found)
    }

    fn number_duplicates(windows: &mut [TargetWindow]) {
        let mut totals: HashMap<&str, u32> = HashMap::new();
        for w in windows.iter() {
            *totals.entry(w.title.as_str()).or_default() += 1;
        }
        let repeated: Vec<String> = totals
            .iter()
            .filter(|(_, &n)| n > 1)
            .map(|(t, _)| (*t).to_string())
            .collect();
        let mut seen: HashMap<String, u32> = HashMap::new();
        for w in windows.iter_mut() {
            if repeated.iter().any(|t| t == &w.title) {
                let n = seen.entry(w.title.clone()).or_default();
                *n += 1;
                w.occurrence = Some(*n);
            }
        }
    }

    pub fn is_alive(handle: isize) -> bool {
        unsafe { IsWindow(Some(HWND(handle as *mut _))).as_bool() }
    }

    pub fn refresh(handle: isize) -> Result<TargetWindow> {
        if !is_alive(handle) {
            return Err(Error::WindowGone(handle));
        }
        let hwnd = HWND(handle as *mut _);
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            Ok(TargetWindow {
                handle,
                pid,
                title: window_title(hwnd),
                process: process_name(pid),
                occurrence: None,
                rect: window_rect(hwnd),
                minimized: IsIconic(hwnd).as_bool(),
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn stub(title: &str) -> TargetWindow {
            TargetWindow {
                handle: 0,
                pid: 0,
                title: title.into(),
                process: String::new(),
                occurrence: None,
                rect: Rect::new(0, 0, 0, 0),
                minimized: false,
            }
        }

        #[test]
        fn only_repeated_titles_get_a_number() {
            let mut windows = vec![stub("Game"), stub("Editor"), stub("Game")];
            number_duplicates(&mut windows);
            assert_eq!(windows[0].occurrence, Some(1));
            assert_eq!(windows[1].occurrence, None);
            assert_eq!(windows[2].occurrence, Some(2));
        }

        /// Smoke test against the live desktop: the enumeration has to come back
        /// with something, since the test runner itself owns a console window.
        #[test]
        fn the_live_desktop_yields_pickable_windows() {
            let windows = list().expect("enumeration failed");
            assert!(
                !windows.is_empty(),
                "no pickable window found on this desktop"
            );
            assert!(windows.iter().all(|w| w.handle != 0 && !w.title.is_empty()));
        }

        #[test]
        fn the_picker_label_carries_the_occurrence() {
            let mut w = stub("Game");
            w.process = "game.exe".into();
            w.occurrence = Some(2);
            assert_eq!(w.label(), "Game (game.exe) #2");
        }
    }
}

#[cfg(windows)]
pub use imp::{is_alive, list, refresh};
