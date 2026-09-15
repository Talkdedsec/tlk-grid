# Third-party notices

tlk-grid is MIT licensed. It builds on the crates below, each under its own
licence. Run `cargo tree` for the full resolved graph, or `cargo license` for a
licence-by-crate listing.

## Direct dependencies

| Crate | Licence | Used for |
| --- | --- | --- |
| [`tauri`](https://crates.io/crates/tauri) | Apache-2.0 OR MIT | Application shell, WebView2 host, tray icon |
| [`windows`](https://crates.io/crates/windows) | Apache-2.0 OR MIT | Win32, DWM, GDI and accessibility bindings |
| [`image`](https://crates.io/crates/image) | MIT OR Apache-2.0 | PNG, JPEG and GIF decoding |
| [`serde`](https://crates.io/crates/serde) / [`serde_json`](https://crates.io/crates/serde_json) | Apache-2.0 OR MIT | Profiles and the command boundary |
| [`thiserror`](https://crates.io/crates/thiserror) | Apache-2.0 OR MIT | Error types |

## Runtime

The interface runs in Microsoft Edge WebView2, which ships with Windows 10 and
11 and is covered by its own Microsoft terms. tlk-grid neither bundles nor
modifies it.

## Not bundled

NVIDIA's NvAPI headers are not included. When the SQUASH module lands it will
open `nvapi64.dll` at runtime and declare the few structures it needs itself,
so nothing under NVIDIA's licence enters this repository.
