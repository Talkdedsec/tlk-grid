//! Profiles on disk, and putting one back onto live windows.
//!
//! Two things are stored side by side: named profiles the user saves, and a
//! single `session` file rewritten as the user works. The session file is what
//! makes the app usable day to day — without it every launch starts from an
//! empty card.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tlkgrid_core::profile::{self, MatchQuality, Profile};
use tlkgrid_core::target;

/// Name of the file that holds the last state, kept out of the named list.
const SESSION: &str = "session";

fn folder<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "no app data directory".to_string())?
        .join("profiles");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn path_for<R: Runtime>(app: &AppHandle<R>, name: &str) -> Result<PathBuf, String> {
    Ok(folder(app)?.join(format!("{}.json", profile::file_stem(name))))
}

pub fn write<R: Runtime>(app: &AppHandle<R>, profile: &Profile) -> Result<(), String> {
    let path = path_for(app, &profile.name)?;
    std::fs::write(path, profile.to_json()).map_err(|e| e.to_string())
}

pub fn read<R: Runtime>(app: &AppHandle<R>, name: &str) -> Result<Profile, String> {
    let path = path_for(app, name)?;
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Profile::from_json(&text).map_err(|e| e.to_string())
}

pub fn remove<R: Runtime>(app: &AppHandle<R>, name: &str) -> Result<(), String> {
    let path = path_for(app, name)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        // Already gone is the state the caller wanted.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Named profiles, alphabetical, with the session file left out.
pub fn names<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<String>, String> {
    let mut found: Vec<String> = std::fs::read_dir(folder(app)?)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "json" {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            (stem != SESSION).then_some(stem)
        })
        .collect();
    found.sort_by_key(|name| name.to_lowercase());
    Ok(found)
}

pub fn save_session<R: Runtime>(app: &AppHandle<R>, profile: &Profile) -> Result<(), String> {
    let mut session = profile.clone();
    session.name = SESSION.to_string();
    write(app, &session)
}

pub fn read_session<R: Runtime>(app: &AppHandle<R>) -> Option<Profile> {
    read(app, SESSION).ok()
}

/// A profile with each card pointed back at a window that exists now.
#[derive(Serialize)]
pub struct Resolved {
    pub profile: Profile,
    pub windows: Vec<ResolvedWindow>,
}

#[derive(Serialize)]
pub struct ResolvedWindow {
    /// `None` when the game is not running. The card still loads, so the layout
    /// is not lost — it just has nothing to point at yet.
    pub handle: Option<isize>,
    pub quality: Option<MatchQuality>,
    pub label: Option<String>,
}

/// Matches every card in the profile against the windows open right now.
pub fn resolve(profile: Profile) -> Result<Resolved, String> {
    let live = target::list().map_err(|e| e.to_string())?;
    let mut taken: Vec<isize> = Vec::new();
    let mut windows = Vec::with_capacity(profile.windows.len());

    for setup in &profile.windows {
        match profile::rematch(setup, &live, &taken) {
            Some(found) => {
                taken.push(found.handle);
                let label = live
                    .iter()
                    .find(|window| window.handle == found.handle)
                    .map(|window| window.label());
                windows.push(ResolvedWindow {
                    handle: Some(found.handle),
                    quality: Some(found.quality),
                    label,
                });
            }
            None => windows.push(ResolvedWindow {
                handle: None,
                quality: None,
                label: None,
            }),
        }
    }

    Ok(Resolved { profile, windows })
}
