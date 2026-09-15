# tlk-grid

Resize, position and zoom a game window without touching the game's internal
rendering resolution. The picture gets bigger; the frame rate does not move,
because nothing is re-rendered — only the window, and a live DWM copy of it,
change size.

Open source, Windows only. [Türkçe](README.tr.md)

> **Status: early.** Window discovery, monitor detection, layout, placement,
> hotkeys and zoom work. Overlays, CurveFX and SQUASH are on the roadmap below
> and are not in the build yet.

## What it does

- **Place any window exactly** — quick halves, ×4/×6/×8/×10 split cells, aspect
  presets (21:9, 32:9, 16:9, 4:3, 16:10, 1:1) or typed X/Y/W/H, drawn on a
  to-scale preview of your monitor.
- **Borderless on demand** — strips the caption and frame, keeps the rendered
  resolution.
- **Multiple instances** — two copies of the same game are told apart by their
  window handle and process id, and numbered `#1 #2 #3`.
- **Hotkeys that behave** — bind a key or a mouse side button; the bind only
  fires while the target window is in front, and the key is held back from the
  game. The master switch suspends it without opening the app, and neither the
  master bind nor F8 ever eats its key.
- **Zoom without touching the game** — hold the bind and a live DWM copy of the
  window is magnified on an overlay. The game is never moved and never told, so
  it keeps rendering at its own resolution and the frame rate does not move.
  Scroll while holding to change the factor. `WINDOW` and `STRETCH` move the
  real window instead, and put it back when you let go.
- **Hold the layout** — pin a window and its layout is re-asserted when the game
  moves it back on Alt-Tab.
- **Picks up where you left off** — the setup is remembered as you work and
  comes back next launch. Save it under a name to keep more than one. A profile
  finds its windows again by executable, so it survives a restart and a changed
  window title.
- **Put it back** — F8 restores every window tlk-grid has touched to the style
  and rect it had before, from anywhere.

## How it works

Four documented Windows APIs do all of it:

| Piece | API |
| --- | --- |
| Window discovery, placement, borderless | Win32 `EnumWindows`, `SetWindowPos`, window styles |
| Pixel-accurate framing | DWM `DWMWA_EXTENDED_FRAME_BOUNDS` |
| Zoom | DWM thumbnails (`DwmRegisterThumbnail` + `rcDestination`) |
| Scope lens, HUD layers *(planned)* | The same, with `rcSource` set to a region |
| Curved screen, CRT, motion blur *(planned)* | Windows.Graphics.Capture → D3D11 → HLSL |
| Custom display modes *(planned)* | NVIDIA NvAPI |

No code injection, no kernel driver, no reading or writing another process's
memory. tlk-grid is a window manager, not a trainer or a cheat.

**A caution that matters:** some kernel-level anti-cheats object to topmost
overlay windows and to another process moving the game window, regardless of
how the move is done. Check your game's rules before using tlk-grid online.

## Requirements

- Windows 10 version 1903 (build 18362) or newer, 64-bit
- A GPU with Desktop Window Manager enabled
- WebView2 runtime — preinstalled on Windows 10/11
- The game running in windowed or borderless windowed mode

## Build

```sh
cargo test --workspace          # layout maths and a live enumeration smoke test
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release           # target/release/tlk-grid.exe, one file
```

## Roadmap

| Phase | Scope | State |
| --- | --- | --- |
| 0 | Shell, tray, single-file build | done |
| 1 | Target discovery, monitors, layout, placement, F8 | done |
| 2 | Hotkeys: low-level hook, hold/toggle, master bypass, Alt-Tab guard | done |
| 3 | Thumbnail zoom, window/stretch methods, wheel factor | done |
| 4 | Black bars, custom overlays, crosshair, scope lens, HUD layers | next |
| 5 | CurveFX: curved screen, CRT, four motion-blur modes | |
| 6 | SQUASH: NvAPI custom display modes | |
| 7 | Profiles, preset library, language switch | partial (profiles and language done) |
| 8 | Guide, diagnostics, release packaging | |

## Interface language

English by default; a Turkish Windows opens in Turkish. The `EN`/`TR` button in
the toolbar switches live, and the choice sticks.

## Credits

The feature set and interaction model are modelled on
[WinGrid](https://store.steampowered.com/app/4847000/WinGrid/) by Zhazira,
which is where this idea was worked out first. No code, asset or text from it
is used here; everything in this repository was written from scratch.

MIT licensed. See [LICENSE](LICENSE).
