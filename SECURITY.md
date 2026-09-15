# Security

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's
[private vulnerability reporting](https://github.com/Talkdedsec/tlk-grid/security/advisories/new),
or write to 292909750+Talkdedsec@users.noreply.github.com.

Tell me what you found, how to reproduce it, and what an attacker would gain.
You will get a first reply within a week.

## What tlk-grid does and does not do

This matters for judging the attack surface, so it is written down rather than
implied.

**It does:**
- Enumerate top-level windows and read their titles, process ids and geometry.
- Move, resize and re-style windows the user picks, through `SetWindowPos` and
  window styles.
- Install a low-level keyboard and mouse hook to notice the keys the user bound.
- Register DWM thumbnails of the window the user picked, and draw them on
  click-through overlay windows it owns.
- Read image files the user chooses and copy them into its own app-data folder.

**It does not:**
- Inject code into another process.
- Load a kernel driver.
- Read or write another process's memory.
- Log keystrokes. The hook compares each event against the bound triggers and
  forgets it; nothing is stored or sent anywhere.
- Make network requests. The application has no networking code at all.

## Scope

In scope: anything that lets tlk-grid be used to reach outside that list — a
path that runs code from a loaded image, a way for another process to drive the
hook, privilege issues around the app-data folder.

Out of scope: that overlays can cover other windows, that a low-level hook sees
input, and that some anti-cheat software objects to either. Those are the
documented shape of the tool, not defects.
