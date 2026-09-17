# How tlk-grid works

Read this before changing anything under `core/`. Most of the code is short; the
reasons behind it are not, and several of the decisions look arbitrary until you
know what was tried first.

## The one idea

`DwmRegisterThumbnail` hands the compositor a second view of a window. Its
`rcDestination` says how big to draw that view, and DWM clips whatever runs past
the destination window's edges.

Blow the destination up past the monitor, let DWM clip it, and you have a zoom.
The game is never moved, never resized, never told anything. It keeps rendering
at its own resolution, so the frame rate does not move either.

Give the same call an `rcSource` and it crops the mirror instead. That is the
scope lens — a small window showing a small centre region — and it is also every
HUD layer: pick a region of the game, draw it somewhere else, live.

So three headline features are one API call with different rectangles:

| Feature | `rcSource` | `rcDestination` |
| --- | --- | --- |
| Zoom | whole window | base rect × factor, clipped by the monitor |
| Scope lens | centre region, shrunk by the factor | the lens square |
| HUD layer | the region the user picked | wherever they dropped it |

`core/tests/layers_smoke.rs` measures all three off the screen rather than
trusting that they work.

## Why the overlay windows look the way they do

An overlay is `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE |
WS_EX_TRANSPARENT | WS_EX_LAYERED`, shown with `SW_SHOWNOACTIVATE`. Every flag is
load-bearing: no activation so the game keeps focus, transparent so aiming still
works through it, tool window so it stays out of Alt-Tab.

**The trap:** layering has two modes and only one of them works here.
`SetLayeredWindowAttributes` (constant alpha) leaves the window on the
compositor's normal path and DWM draws the thumbnail into it.
`UpdateLayeredWindow` (per-pixel alpha) takes it off that path and **the
thumbnail never appears** — no error, just black.

Per-pixel alpha is exactly what a crosshair PNG needs. So the two cannot share a
window, and `core/src/layers.rs` keeps them apart: mirrors and bars get constant
alpha, pictures get `UpdateLayeredWindow`, and they are separate windows.

Black bars use a second window class whose class brush is `BLACK_BRUSH`. That is
the entire implementation — no paint handler, no allocation.

## Why input lives on its own thread

A low-level hook is dispatched on the thread that installed it, and Windows
silently drops a hook whose thread stops pumping messages. It can therefore never
live on a UI thread that might block. `SetWinEventHook`, used to notice a game
reasserting its own geometry, needs the same pump, so both share one thread in
`core/src/input.rs`.

The callbacks are plain `extern "system"` functions with no user pointer, so the
state they read sits in one process-wide slot. The callback takes that slot with
`try_lock` and passes the event through on contention. **A blocking lock there
would freeze every keystroke on the machine** for as long as another thread held
it.

The decision logic (`decide`, `decide_wheel`) is a pure function over that state,
which is why the bind rules have real unit tests instead of being checked by
hand.

Rules the tests pin down:

- A bound key is held back from the game only while the target window is in
  front. `core/tests/input_smoke.rs` proves this against a real focused window.
- The master bind and F8 work from anywhere and **never** eat their key. A stuck
  state has to be escapable.
- Key auto-repeat engages an action once, not forty times a second.
- The wheel drives whichever bind engaged last, and only while one is held — the
  game keeps its weapon switch otherwise.

Binds are captured through the hook rather than through a webview `keydown`,
because a browser event cannot see the mouse side buttons and those are what
people bind a zoom to.

## Why placement compensates for an invisible border

Since Windows 10, `GetWindowRect` includes a resize border you cannot see —
usually 7px at the sides and bottom. Place a window at x=0 with `SetWindowPos`
and it looks 7px off. `DWMWA_EXTENDED_FRAME_BOUNDS` reports what the user
actually sees, and `core/src/frame.rs` moves by the difference between the two.

The padding is read *after* any style change, because stripping the frame is
exactly what removes it.

## Why a profile does not store a window handle

A handle means nothing after a restart. A saved card describes *which* window it
wants — executable, title, which copy — and `core/src/profile.rs` matches it back
against the live list on load.

Executable first: a title changes with the map, the server or the patch, but
`cs2.exe` is still `cs2.exe`. The match quality (`exact` / `same-title` /
`same-process`) is reported to the interface so a weak match can be seen rather
than silently landing on the wrong window. Windows already claimed by an earlier
card are excluded, so two cards pointing at two copies of a game do not both take
the first one.

## Layout

| Crate / folder | What it holds |
| --- | --- |
| `core/` | Windows calls and the arithmetic they need. No UI, no Tauri. |
| `core/src/layout.rs`, `zoom.rs`, `picture.rs`, `profile.rs` | Pure functions. Most of the test suite lives here. |
| `app/` | Tauri shell: commands, state, tray, and the bridge from the hook thread. |
| `ui/` | Manager window and settings windows. Plain HTML, CSS and ES modules — no build step. |

The split is deliberate: anything that wraps a platform call keeps its geometry
in one of the pure modules, so the behaviour can be tested without a screen and
the wrapper stays thin enough to read.

## What is deliberately not done

- **No injection**, no kernel driver, no reading another process's memory. The
  README promises it and the code has to keep it.
- **No cursor remapping.** Stretched-window setups tempt you to fake mouse
  coordinates for MOBAs. Anti-cheats block it and the accounts pay for it.
- **DPI zoom does nothing at runtime.** Per-application DPI is a compatibility
  flag Windows reads when a program starts; it cannot be flipped mid-game. The
  interface says so rather than pretending.
