# Contributing

Thanks for looking. tlk-grid is a Windows-only Rust project; everything below
assumes a Windows machine with a desktop, because most of what it does can only
be checked against a real compositor.

## Getting set up

```sh
rustup toolchain install stable      # 1.82 or newer
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release                # target/release/tlk-grid.exe
```

There is no JavaScript build step. The interface is plain HTML, CSS and ES
modules under `ui/`, served straight out of the binary.

## Where things live

| Path | What it holds |
| --- | --- |
| `core/` | Everything that touches Windows, plus the arithmetic it needs. No UI. |
| `app/` | The Tauri shell: commands, state, tray, and the bridge from the hook thread. |
| `ui/` | The manager window and the settings windows. |

`core/src/layout.rs`, `core/src/zoom.rs` and `core/src/picture.rs` are pure
functions and carry most of the test suite. Anything that wraps a Win32 call
keeps its geometry in one of those so it can be tested without a screen.

## House rules

- **No injection.** No code injection, no kernel driver, no reading or writing
  another process's memory. The README makes that promise; the code has to keep
  it. A change that needs any of those will not be merged.
- **Code and interface strings are English.** Turkish lives in the `tr`
  dictionary of `ui/js/i18n.js` and in `README.tr.md`, nowhere else.
- **Both dictionaries move together.** A key added to `en` is added to `tr` in
  the same change; a missing key falls back silently and leaves a half-translated
  screen.
- **New Win32 calls come with a test.** A unit test where the logic allows, and
  a smoke test against the live desktop where it does not — see
  `core/tests/layers_smoke.rs` for the pattern.
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
  both pass before a pull request.

## Commit messages

Plain sentences, present tense, no prefix convention. Say what changed and why
the change is shaped the way it is; the diff already shows how.

## Reporting a bug

Open an issue with your Windows build, GPU, the game and its window mode. A
short clip beats a description for anything about the overlay.
