# Changelog

Notable changes, newest first. Dates are the day the work landed on `main`.
Versions follow [Semantic Versioning](https://semver.org/); nothing is released
yet, so everything below sits under Unreleased.

## Unreleased

### Added
- Window discovery with HWND and process id, so two copies of the same game are
  told apart and numbered `#1 #2 #3`.
- Monitor detection: EDID name, native mode, refresh rate, DPI scale.
- Placement: quick halves, ×4/×6/×8/×10 split cells, aspect presets
  (21:9, 32:9, 16:9, 4:3, 16:10, 1:1), typed X/Y/W/H, and a to-scale preview you
  can drag a box on.
- Borderless on demand, with the invisible Windows 10/11 resize border
  compensated so the visible frame lands exactly where asked.
- Hotkeys through low-level keyboard and mouse hooks: mouse side buttons, hold
  and toggle, key held back from the game, master bypass, and F8 as a global
  emergency reset.
- A guard that re-asserts a pinned layout when the game moves its own window
  back after Alt-Tab.
- Zoom: THUMBNAIL (a live DWM copy magnified on an overlay, the game untouched),
  WINDOW and STRETCH. Wheel adjusts the factor while the bind is held.
- Overlay engine: click-through, never-activated, always-on-top layers drawing a
  mirrored window, flat black, or a picture — including animated GIFs.
- Letterbox bars, a centred scope lens with a black or see-through backdrop, and
  a custom crosshair that survives a restart.
- Two interface languages: English by default, Turkish on a Turkish Windows,
  switchable live from the toolbar.

### Known gaps
- DPI zoom is listed but does nothing at runtime: per-application DPI is a
  compatibility flag Windows reads when a program starts.
- Custom artwork around the letterbox and user-picked HUD layers are designed
  but not built.
- CurveFX and SQUASH are not started.
