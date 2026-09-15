## What this changes

<!-- One or two sentences. The diff shows how; say why. -->

## How it was checked

<!-- Which of these actually ran, and on what. Delete the rest. -->

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo fmt --all --check`
- [ ] Tried against a real game — which one, and in which window mode:
- [ ] Ran the desktop smoke test: `cargo test -p tlkgrid-core --test layers_smoke -- --ignored`

## House rules

- [ ] No code injection, kernel driver, or other-process memory access was added.
- [ ] Any new interface string was added to **both** the `en` and `tr`
      dictionaries in `ui/js/i18n.js`.
- [ ] New Win32 calls come with a test, or a note saying why one is impossible.
