//! Saving a setup and getting it back.
//!
//! The hard part is not the file, it is that a window handle means nothing
//! after a restart. A saved card therefore describes *which* window it wants —
//! the executable, the title, and which of several copies — and is matched back
//! to a live handle when the profile is loaded. Matching is pure arithmetic over
//! a window list, so it is tested rather than guessed at.

use serde::{Deserialize, Serialize};

use crate::bind::Bind;
use crate::layout::Rect;
use crate::target::TargetWindow;
use crate::zoom::Method;

/// Bumped only when an older file can no longer be read as-is. Every field is
/// `#[serde(default)]`, so adding one is not a version change.
pub const VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default = "current_version")]
    pub version: u32,
    #[serde(default)]
    pub name: String,
    /// Interface language at save time, so a shared profile does not drag the
    /// author's language along unless it was deliberately chosen.
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default = "yes")]
    pub wheel_adjusts: bool,
    #[serde(default)]
    pub scope: ScopeSettings,
    #[serde(default)]
    pub crosshair_scale: Option<u32>,
    #[serde(default)]
    pub windows: Vec<WindowSetup>,
}

fn current_version() -> u32 {
    VERSION
}

fn yes() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScopeSettings {
    pub enabled: bool,
    pub size: i32,
    pub see_through: bool,
}

impl Default for ScopeSettings {
    fn default() -> Self {
        ScopeSettings {
            enabled: false,
            size: 650,
            see_through: true,
        }
    }
}

/// One card: what it points at, where it puts it, and how it zooms.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowSetup {
    /// `cs2.exe`. The strongest signal, and the one that survives a patch that
    /// changes the title.
    #[serde(default)]
    pub process: String,
    #[serde(default)]
    pub title: String,
    /// Which copy, when several share a title.
    #[serde(default)]
    pub occurrence: Option<u32>,
    /// `\\.\DISPLAY1`, so a two-monitor setup lands on the right one.
    #[serde(default)]
    pub monitor: String,
    pub rect: Rect,
    #[serde(default)]
    pub borderless: bool,
    #[serde(default)]
    pub pin: bool,
    #[serde(default)]
    pub method: Method,
    #[serde(default = "one")]
    pub factor: f64,
    #[serde(default)]
    pub binds: Vec<Bind>,
}

fn one() -> f64 {
    1.0
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Profile {
        Profile {
            version: VERSION,
            name: name.into(),
            language: None,
            wheel_adjusts: true,
            scope: ScopeSettings::default(),
            crosshair_scale: None,
            windows: Vec::new(),
        }
    }

    /// Reads a profile, refusing anything written by a newer build rather than
    /// silently dropping the parts it does not understand.
    pub fn from_json(text: &str) -> crate::Result<Profile> {
        let profile: Profile =
            serde_json::from_str(text).map_err(|e| crate::Error::BadProfile(e.to_string()))?;
        if profile.version > VERSION {
            return Err(crate::Error::BadProfile(format!(
                "written by a newer version of tlk-grid (profile {}, this build reads {VERSION})",
                profile.version
            )));
        }
        Ok(profile)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }
}

/// How confident a match is. The UI shows the reason so a profile that lands on
/// the wrong copy of a game can be understood rather than just corrected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchQuality {
    /// Same executable, same title, same occurrence.
    Exact,
    /// Same executable and title, but a different copy is in front.
    SameTitle,
    /// Same executable only — the game is running, the window is named
    /// differently than it was.
    SameProcess,
}

#[derive(Clone, Debug)]
pub struct Match {
    pub handle: isize,
    pub quality: MatchQuality,
}

/// Finds the live window a saved card was describing.
///
/// Process name first: a title changes with the map, the server, or the patch,
/// but `cs2.exe` is still `cs2.exe`. Windows already claimed by an earlier card
/// are passed in so two cards pointing at two copies of the same game do not
/// both land on the first one.
pub fn rematch(setup: &WindowSetup, live: &[TargetWindow], taken: &[isize]) -> Option<Match> {
    let free = || live.iter().filter(|window| !taken.contains(&window.handle));

    if !setup.process.is_empty() {
        let same_process: Vec<&TargetWindow> = free()
            .filter(|window| window.process.eq_ignore_ascii_case(&setup.process))
            .collect();

        let same_title: Vec<&&TargetWindow> = same_process
            .iter()
            .filter(|window| window.title == setup.title)
            .collect();

        if let Some(exact) = same_title
            .iter()
            .find(|window| window.occurrence == setup.occurrence)
        {
            return Some(Match {
                handle: exact.handle,
                quality: MatchQuality::Exact,
            });
        }
        if let Some(window) = same_title.first() {
            return Some(Match {
                handle: window.handle,
                quality: MatchQuality::SameTitle,
            });
        }
        if let Some(window) = same_process.first() {
            return Some(Match {
                handle: window.handle,
                quality: MatchQuality::SameProcess,
            });
        }
        return None;
    }

    // A profile saved before a target was picked has only a title to go on.
    free()
        .find(|window| !setup.title.is_empty() && window.title == setup.title)
        .map(|window| Match {
            handle: window.handle,
            quality: MatchQuality::SameTitle,
        })
}

/// File names come from the user, so they are reduced to something that cannot
/// escape the profile folder or upset Windows.
pub fn file_stem(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        "profile".to_string()
    } else {
        cleaned.chars().take(64).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::{Action, Trigger};

    fn window(handle: isize, process: &str, title: &str, occurrence: Option<u32>) -> TargetWindow {
        TargetWindow {
            handle,
            pid: 0,
            title: title.into(),
            process: process.into(),
            occurrence,
            rect: Rect::new(0, 0, 800, 600),
            minimized: false,
        }
    }

    fn setup(process: &str, title: &str, occurrence: Option<u32>) -> WindowSetup {
        WindowSetup {
            process: process.into(),
            title: title.into(),
            occurrence,
            monitor: String::new(),
            rect: Rect::new(0, 270, 1920, 540),
            borderless: true,
            pin: false,
            method: Method::Thumbnail,
            factor: 2.0,
            binds: vec![],
        }
    }

    #[test]
    fn the_same_window_comes_back_exactly() {
        let live = vec![window(10, "cs2.exe", "Counter-Strike 2", None)];
        let found = rematch(&setup("cs2.exe", "Counter-Strike 2", None), &live, &[]).unwrap();
        assert_eq!(found.handle, 10);
        assert_eq!(found.quality, MatchQuality::Exact);
    }

    #[test]
    fn a_changed_title_still_finds_the_game() {
        let live = vec![window(10, "cs2.exe", "Counter-Strike 2 - de_dust2", None)];
        let found = rematch(&setup("cs2.exe", "Counter-Strike 2", None), &live, &[]).unwrap();
        assert_eq!(found.handle, 10);
        assert_eq!(
            found.quality,
            MatchQuality::SameProcess,
            "the match is weaker, and the UI is told so"
        );
    }

    #[test]
    fn two_cards_do_not_both_land_on_the_first_copy() {
        let live = vec![
            window(10, "game.exe", "Game", Some(1)),
            window(11, "game.exe", "Game", Some(2)),
        ];
        let first = rematch(&setup("game.exe", "Game", Some(1)), &live, &[]).unwrap();
        let second = rematch(&setup("game.exe", "Game", Some(2)), &live, &[first.handle]).unwrap();
        assert_eq!(first.handle, 10);
        assert_eq!(second.handle, 11);
    }

    #[test]
    fn a_missing_occurrence_falls_back_to_any_copy_with_that_title() {
        let live = vec![window(11, "game.exe", "Game", Some(2))];
        let found = rematch(&setup("game.exe", "Game", Some(1)), &live, &[]).unwrap();
        assert_eq!(found.quality, MatchQuality::SameTitle);
    }

    #[test]
    fn a_game_that_is_not_running_matches_nothing() {
        let live = vec![window(10, "other.exe", "Other", None)];
        assert!(rematch(&setup("cs2.exe", "Counter-Strike 2", None), &live, &[]).is_none());
    }

    #[test]
    fn executable_names_are_matched_without_regard_to_case() {
        let live = vec![window(10, "CS2.EXE", "Counter-Strike 2", None)];
        assert!(rematch(&setup("cs2.exe", "Counter-Strike 2", None), &live, &[]).is_some());
    }

    #[test]
    fn a_profile_survives_a_round_trip() {
        let mut profile = Profile::new("superwide");
        let mut card = setup("cs2.exe", "Counter-Strike 2", Some(1));
        card.binds = vec![Bind::new(Action::Resize, Trigger::Key { vk: 0x4B })];
        profile.windows.push(card);

        let read = Profile::from_json(&profile.to_json()).expect("round trip");
        assert_eq!(read.name, "superwide");
        assert_eq!(read.windows.len(), 1);
        assert_eq!(read.windows[0].factor, 2.0);
        assert_eq!(read.windows[0].binds.len(), 1);
    }

    #[test]
    fn a_profile_from_a_newer_build_is_refused_rather_than_half_read() {
        let json = r#"{"version": 99, "name": "later", "windows": []}"#;
        let error = Profile::from_json(json).unwrap_err().to_string();
        assert!(error.contains("newer version"), "{error}");
    }

    #[test]
    fn missing_fields_take_their_defaults() {
        let json = r#"{"windows": [{"rect": {"x": 0, "y": 0, "w": 800, "h": 600}}]}"#;
        let profile = Profile::from_json(json).expect("defaults fill in");
        assert_eq!(profile.version, VERSION);
        assert!(profile.wheel_adjusts);
        assert_eq!(profile.windows[0].factor, 1.0);
        assert_eq!(profile.windows[0].method, Method::Thumbnail);
    }

    #[test]
    fn a_name_cannot_escape_the_profile_folder() {
        let stem = file_stem("../../etc/passwd");
        assert!(!stem.contains(['/', '\\', '.']), "{stem}");
        assert_eq!(file_stem("  "), "profile");
        assert_eq!(
            file_stem("4:3 dust"),
            "4-3 dust",
            "spaces are kept, colons are not"
        );
        assert!(file_stem(&"x".repeat(200)).chars().count() <= 64);
    }
}
