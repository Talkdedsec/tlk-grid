//! Which of the two interface languages to open in.
//!
//! English is the default everywhere. A Turkish Windows opens in Turkish, and
//! the toolbar switch overrides either choice for good.

pub const SUPPORTED: [&str; 2] = ["en", "tr"];

pub fn normalize(tag: &str) -> &'static str {
    let primary = tag
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    SUPPORTED
        .iter()
        .copied()
        .find(|s| *s == primary)
        .unwrap_or("en")
}

#[cfg(windows)]
pub fn system_language() -> &'static str {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;

    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if len <= 0 {
        return "en";
    }
    let tag = String::from_utf16_lossy(&buf[..(len - 1) as usize]);
    normalize(&tag)
}

#[cfg(not(windows))]
pub fn system_language() -> &'static str {
    "en"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_windows_opens_in_turkish() {
        assert_eq!(normalize("tr-TR"), "tr");
        assert_eq!(normalize("tr"), "tr");
    }

    #[test]
    fn anything_else_falls_back_to_english() {
        assert_eq!(normalize("de-DE"), "en");
        assert_eq!(normalize("ru-RU"), "en");
        assert_eq!(normalize(""), "en");
    }
}
