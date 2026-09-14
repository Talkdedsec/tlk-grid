# tlk-grid

Resize, position and zoom a game window without touching the game's internal
rendering resolution. The picture gets bigger; the frame rate does not move,
because nothing is re-rendered — only the window, and a live DWM copy of it,
change size.

Open source, Windows only. [Türkçe](README.tr.md)

> **Status: early.** Window discovery, monitor detection, layout and placement
> work. Zoom, overlays, CurveFX and SQUASH are on the roadmap below and are not
> in the build yet.

## What it does

- **Place any window exactly** — quick halves, ×4/×6/×8/×10 split cells, aspect
  presets (21:9, 32:9, 16:9, 4:3, 16:10, 1:1) or typed X/Y/W/H, drawn on a
  to-scale preview of your monitor.
- **Borderless on demand** — strips the caption and frame, keeps the rendered
  resolution.
- **Multiple instances** — two copies of the same game are told apart by their
  window handle and process id, and numbered `#1 #2 #3`.
- **Put it back** — F8 restores every window tlk-grid has touched to the style
  and rect it had before.

## How it works

Four documented Windows APIs do all of it:

| Piece | API |
| --- | --- |
| Window discovery, placement, borderless | Win32 `EnumWindows`, `SetWindowPos`, window styles |
| Pixel-accurate framing | DWM `DWMWA_EXTENDED_FRAME_BOUNDS` |
| Zoom, scope lens, HUD layers *(planned)* | DWM thumbnails (`DwmRegisterThumbnail` + `rcSource`) |
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
| 2 | Hotkeys: low-level hook, hold/toggle, master bypass, Alt-Tab guard | next |
| 3 | Thumbnail zoom, window/stretch/DPI methods, wheel factor | |
| 4 | Black bars, custom overlays, crosshair, scope lens, HUD layers | |
| 5 | CurveFX: curved screen, CRT, four motion-blur modes | |
| 6 | SQUASH: NvAPI custom display modes | |
| 7 | Profiles, preset library, language switch | partial (language done) |
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
