//! The input thread: low-level keyboard and mouse hooks plus the window guard.
//!
//! Three things force this into its own thread with its own message loop.
//! A low-level hook is dispatched on the thread that installed it, and Windows
//! silently drops a hook whose thread stops pumping messages — so it can never
//! live on a UI thread that might block. `SetWinEventHook` needs the same pump.
//! And the callbacks are plain `extern "system"` functions with no user pointer,
//! so the state they read lives in one process-wide slot.
//!
//! The callback takes that slot with `try_lock` and passes the event through on
//! contention. A blocking lock here would freeze every keystroke on the machine
//! for as long as another thread held it.

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

use crate::bind::{Action, Bind, Mode, Trigger};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum InputEvent {
    /// The next trigger after `begin_capture`, swallowed so it never reaches
    /// whatever is focused while the user is assigning a bind.
    Captured {
        trigger: Trigger,
    },
    Engaged {
        action: Action,
    },
    Released {
        action: Action,
    },
    /// Wheel notches while a hold bind is down. Positive is away from the user.
    Wheel {
        action: Action,
        notches: i32,
    },
    Foreground {
        handle: isize,
    },
    /// A watched window moved or resized on its own — usually the game
    /// reasserting its layout after Alt-Tab.
    Moved {
        handle: isize,
    },
}

struct Shared {
    binds: Vec<Bind>,
    /// Non-global binds only fire while this window is in the foreground.
    target: Option<isize>,
    watched: HashSet<isize>,
    capture: bool,
    master_on: bool,
    wheel_adjusts: bool,
    /// Triggers physically down, so key auto-repeat engages an action once.
    down: HashSet<Trigger>,
    /// Engagement order: the wheel drives whichever action engaged last.
    engaged: Vec<Action>,
    tx: Sender<InputEvent>,
}

static SHARED: OnceLock<Mutex<Shared>> = OnceLock::new();

impl Shared {
    fn bind_for(&self, trigger: Trigger) -> Option<Bind> {
        self.binds
            .iter()
            .copied()
            .find(|b| b.trigger == trigger && self.is_live(b.action))
    }

    fn is_live(&self, action: Action) -> bool {
        match action {
            Action::Master | Action::Panic => true,
            Action::Resize => self.master_on,
            _ => true,
        }
    }

    fn emit(&self, event: InputEvent) {
        let _ = self.tx.send(event);
    }
}

// ---------------------------------------------------------------- public API

pub struct Input {
    thread_id: u32,
}

impl Input {
    /// Installs the hooks and returns the event stream. Calling it twice reuses
    /// the existing thread and returns an error, since the hooks are global.
    pub fn start() -> crate::Result<(Input, Receiver<InputEvent>)> {
        imp::start()
    }

    pub fn set_binds(&self, binds: Vec<Bind>) {
        with_shared(|s| s.binds = binds);
    }

    pub fn set_target(&self, handle: Option<isize>) {
        with_shared(|s| {
            s.target = handle;
            // A target change invalidates anything held against the old window.
            s.engaged.clear();
            s.down.clear();
        });
    }

    pub fn watch(&self, handle: isize) {
        with_shared(|s| {
            s.watched.insert(handle);
        });
    }

    pub fn unwatch(&self, handle: isize) {
        with_shared(|s| {
            s.watched.remove(&handle);
        });
    }

    /// The bind field is armed: swallow the next trigger and report it.
    pub fn begin_capture(&self) {
        with_shared(|s| s.capture = true);
    }

    pub fn cancel_capture(&self) {
        with_shared(|s| s.capture = false);
    }

    pub fn set_master(&self, on: bool) {
        with_shared(|s| {
            s.master_on = on;
            if !on {
                s.engaged.retain(|a| *a != Action::Resize);
            }
        });
    }

    pub fn set_wheel_adjusts(&self, on: bool) {
        with_shared(|s| s.wheel_adjusts = on);
    }

    /// Which window the non-global binds are gated on right now.
    pub fn target(&self) -> Option<isize> {
        read_shared(|s| s.target).flatten()
    }

    pub fn is_engaged(&self, action: Action) -> bool {
        read_shared(|s| s.engaged.contains(&action)).unwrap_or(false)
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        imp::stop(self.thread_id);
    }
}

fn with_shared(edit: impl FnOnce(&mut Shared)) {
    if let Some(lock) = SHARED.get() {
        if let Ok(mut shared) = lock.lock() {
            edit(&mut shared);
        }
    }
}

fn read_shared<T>(read: impl FnOnce(&Shared) -> T) -> Option<T> {
    SHARED.get().and_then(|l| l.lock().ok()).map(|s| read(&s))
}

// -------------------------------------------------------- decision, no Win32

/// Outcome of one raw trigger event. Split out from the hook so the rules can
/// be tested without synthesising system input.
#[derive(Debug, PartialEq, Eq)]
struct Decision {
    events: Vec<InputEvent>,
    swallow: bool,
}

fn decide(shared: &mut Shared, trigger: Trigger, pressed: bool, focused: bool) -> Decision {
    if shared.capture {
        // Releases are ignored so the click that armed the field cannot bind itself.
        if !pressed {
            return Decision {
                events: vec![],
                swallow: true,
            };
        }
        shared.capture = false;
        shared.down.remove(&trigger);
        return Decision {
            events: vec![InputEvent::Captured { trigger }],
            swallow: true,
        };
    }

    let Some(bind) = shared.bind_for(trigger) else {
        return Decision {
            events: vec![],
            swallow: false,
        };
    };

    // Master and Panic reach through from anywhere; everything else waits for
    // the target window to be in front.
    if !bind.action.is_global() && !focused {
        return Decision {
            events: vec![],
            swallow: false,
        };
    }

    let swallow = bind.swallows();
    let mut events = Vec::new();

    if pressed {
        // Auto-repeat: the key is already down, so nothing changes.
        if !shared.down.insert(trigger) {
            return Decision { events, swallow };
        }
        match bind.mode {
            Mode::Hold => {
                if !shared.engaged.contains(&bind.action) {
                    shared.engaged.push(bind.action);
                    events.push(InputEvent::Engaged {
                        action: bind.action,
                    });
                }
            }
            Mode::Toggle => {
                if let Some(at) = shared.engaged.iter().position(|a| *a == bind.action) {
                    shared.engaged.remove(at);
                    events.push(InputEvent::Released {
                        action: bind.action,
                    });
                } else {
                    shared.engaged.push(bind.action);
                    events.push(InputEvent::Engaged {
                        action: bind.action,
                    });
                }
            }
        }
    } else {
        shared.down.remove(&trigger);
        if bind.mode == Mode::Hold {
            if let Some(at) = shared.engaged.iter().position(|a| *a == bind.action) {
                shared.engaged.remove(at);
                events.push(InputEvent::Released {
                    action: bind.action,
                });
            }
        }
    }

    Decision { events, swallow }
}

fn decide_wheel(shared: &Shared, notches: i32) -> Decision {
    if !shared.wheel_adjusts || notches == 0 {
        return Decision {
            events: vec![],
            swallow: false,
        };
    }
    match shared.engaged.last().copied() {
        // Only a held bind claims the wheel; a toggled overlay leaves scrolling
        // alone so the game keeps its weapon switch.
        Some(action) if !action.is_global() => Decision {
            events: vec![InputEvent::Wheel { action, notches }],
            swallow: true,
        },
        _ => Decision {
            events: vec![],
            swallow: false,
        },
    }
}

// --------------------------------------------------------------- Win32 layer

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, PostThreadMessageW,
        SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG,
        MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN,
        WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    };

    use std::collections::HashSet;
    use std::sync::mpsc::{channel, Receiver};
    use std::sync::Mutex;

    use super::{decide, decide_wheel, Decision, Input, InputEvent, Shared, SHARED};
    use crate::bind::{MouseButton, Trigger};
    use crate::error::Error;

    const WHEEL_DELTA: i32 = 120;
    const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
    const EVENT_OBJECT_LOCATIONCHANGE: u32 = 0x800B;
    const OBJID_WINDOW: i32 = 0;
    const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
    const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

    pub fn start() -> crate::Result<(Input, Receiver<InputEvent>)> {
        let (tx, rx) = channel();
        if SHARED
            .set(Mutex::new(Shared {
                binds: Vec::new(),
                target: None,
                watched: HashSet::new(),
                capture: false,
                master_on: true,
                wheel_adjusts: true,
                down: HashSet::new(),
                engaged: Vec::new(),
                tx,
            }))
            .is_err()
        {
            return Err(Error::win32("SetWindowsHookEx", -1));
        }

        let (ready_tx, ready_rx) = channel::<crate::Result<u32>>();
        std::thread::Builder::new()
            .name("tlk-grid-input".into())
            .spawn(move || pump(ready_tx))
            .map_err(|_| Error::win32("CreateThread", -1))?;

        let thread_id = ready_rx
            .recv()
            .map_err(|_| Error::win32("SetWindowsHookEx", -1))??;
        Ok((Input { thread_id }, rx))
    }

    pub fn stop(thread_id: u32) {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }

    fn pump(ready: std::sync::mpsc::Sender<crate::Result<u32>>) {
        unsafe {
            let keyboard = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(on_keyboard), None, 0) {
                Ok(h) => h,
                Err(e) => {
                    let _ = ready.send(Err(Error::win32("SetWindowsHookEx(keyboard)", e.code().0)));
                    return;
                }
            };
            let mouse = match SetWindowsHookExW(WH_MOUSE_LL, Some(on_mouse), None, 0) {
                Ok(h) => h,
                Err(e) => {
                    let _ = UnhookWindowsHookEx(keyboard);
                    let _ = ready.send(Err(Error::win32("SetWindowsHookEx(mouse)", e.code().0)));
                    return;
                }
            };

            // The guard only needs foreground changes and geometry changes, and
            // skipping our own process keeps our overlays from waking it.
            let guard = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_win_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
            let moves = SetWinEventHook(
                EVENT_OBJECT_LOCATIONCHANGE,
                EVENT_OBJECT_LOCATIONCHANGE,
                None,
                Some(on_win_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );

            let _ = ready.send(Ok(GetCurrentThreadId()));

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWinEvent(moves);
            let _ = UnhookWinEvent(guard);
            let _ = UnhookWindowsHookEx(mouse);
            let _ = UnhookWindowsHookEx(keyboard);
        }
    }

    /// Applies a decision and reports whether the event should be eaten.
    fn settle(decision: Decision, shared: &Shared) -> bool {
        for event in &decision.events {
            shared.emit(*event);
        }
        decision.swallow
    }

    fn focused_on_target(shared: &Shared) -> bool {
        match shared.target {
            Some(handle) => unsafe { GetForegroundWindow().0 as isize == handle },
            None => false,
        }
    }

    /// Shared body of both hooks. `try_lock` keeps a busy main thread from
    /// stalling system-wide input.
    fn handle(trigger: Trigger, pressed: bool) -> bool {
        let Some(lock) = SHARED.get() else {
            return false;
        };
        let Ok(mut shared) = lock.try_lock() else {
            return false;
        };
        let focused = focused_on_target(&shared);
        let decision = decide(&mut shared, trigger, pressed, focused);
        settle(decision, &shared)
    }

    unsafe extern "system" fn on_keyboard(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code < 0 {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let pressed = match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => true,
            WM_KEYUP | WM_SYSKEYUP => false,
            _ => return CallNextHookEx(None, code, wparam, lparam),
        };
        if handle(Trigger::Key { vk: info.vkCode }, pressed) {
            return LRESULT(1);
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn on_mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code < 0 {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);

        if wparam.0 as u32 == WM_MOUSEWHEEL {
            let notches = ((info.mouseData >> 16) as i16) as i32 / WHEEL_DELTA;
            let Some(lock) = SHARED.get() else {
                return CallNextHookEx(None, code, wparam, lparam);
            };
            let Ok(shared) = lock.try_lock() else {
                return CallNextHookEx(None, code, wparam, lparam);
            };
            if settle(decide_wheel(&shared, notches), &shared) {
                return LRESULT(1);
            }
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let (button, pressed) = match wparam.0 as u32 {
            WM_LBUTTONDOWN => (MouseButton::Left, true),
            WM_LBUTTONUP => (MouseButton::Left, false),
            WM_RBUTTONDOWN => (MouseButton::Right, true),
            WM_RBUTTONUP => (MouseButton::Right, false),
            WM_MBUTTONDOWN => (MouseButton::Middle, true),
            WM_MBUTTONUP => (MouseButton::Middle, false),
            WM_XBUTTONDOWN | WM_XBUTTONUP => {
                let button = if (info.mouseData >> 16) & 0xFFFF == 1 {
                    MouseButton::X1
                } else {
                    MouseButton::X2
                };
                (button, wparam.0 as u32 == WM_XBUTTONDOWN)
            }
            _ => return CallNextHookEx(None, code, wparam, lparam),
        };

        if handle(Trigger::Mouse { button }, pressed) {
            return LRESULT(1);
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn on_win_event(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        // OBJID_CURSOR fires a location change on every mouse move; only the
        // window itself is interesting.
        if id_object != OBJID_WINDOW || id_child != 0 || hwnd.is_invalid() {
            return;
        }
        let handle = hwnd.0 as isize;
        let Some(lock) = SHARED.get() else { return };
        let Ok(shared) = lock.try_lock() else { return };
        match event {
            EVENT_SYSTEM_FOREGROUND => shared.emit(InputEvent::Foreground { handle }),
            EVENT_OBJECT_LOCATIONCHANGE if shared.watched.contains(&handle) => {
                shared.emit(InputEvent::Moved { handle })
            }
            _ => {}
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::sync::mpsc::{channel, Receiver};

    use super::{Input, InputEvent};

    pub fn start() -> crate::Result<(Input, Receiver<InputEvent>)> {
        let (_tx, rx) = channel();
        Ok((Input { thread_id: 0 }, rx))
    }

    pub fn stop(_thread_id: u32) {}
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;
    use crate::bind::{panic_bind, Bind, MouseButton};

    const RMB: Trigger = Trigger::Mouse {
        button: MouseButton::Right,
    };
    const M4: Trigger = Trigger::Mouse {
        button: MouseButton::X1,
    };
    const KEY_C: Trigger = Trigger::Key { vk: 0x43 };

    fn rig(binds: Vec<Bind>) -> (Shared, Receiver<InputEvent>) {
        let (tx, rx) = channel();
        (
            Shared {
                binds,
                target: Some(1),
                watched: HashSet::new(),
                capture: false,
                master_on: true,
                wheel_adjusts: true,
                down: HashSet::new(),
                engaged: Vec::new(),
                tx,
            },
            rx,
        )
    }

    #[test]
    fn a_hold_bind_engages_once_through_key_repeat() {
        let (mut shared, _rx) = rig(vec![Bind::new(Action::Resize, RMB)]);
        let first = decide(&mut shared, RMB, true, true);
        let repeat = decide(&mut shared, RMB, true, true);
        let release = decide(&mut shared, RMB, false, true);

        assert_eq!(
            first.events,
            vec![InputEvent::Engaged {
                action: Action::Resize
            }]
        );
        assert!(repeat.events.is_empty(), "auto-repeat must not re-engage");
        assert!(repeat.swallow, "the repeat still has to be eaten");
        assert_eq!(
            release.events,
            vec![InputEvent::Released {
                action: Action::Resize
            }]
        );
    }

    #[test]
    fn a_toggle_bind_flips_on_each_press_and_ignores_the_release() {
        let mut bind = Bind::new(Action::Scope, KEY_C);
        bind.mode = Mode::Toggle;
        let (mut shared, _rx) = rig(vec![bind]);

        assert_eq!(
            decide(&mut shared, KEY_C, true, true).events,
            vec![InputEvent::Engaged {
                action: Action::Scope
            }]
        );
        assert!(decide(&mut shared, KEY_C, false, true).events.is_empty());
        assert_eq!(
            decide(&mut shared, KEY_C, true, true).events,
            vec![InputEvent::Released {
                action: Action::Scope
            }]
        );
    }

    #[test]
    fn nothing_fires_while_the_game_is_not_in_front() {
        let (mut shared, _rx) = rig(vec![Bind::new(Action::Resize, RMB)]);
        let out = decide(&mut shared, RMB, true, false);
        assert!(out.events.is_empty());
        assert!(
            !out.swallow,
            "an unfocused game must still get its right-click"
        );
    }

    #[test]
    fn the_master_bind_works_unfocused_and_never_eats_the_key() {
        let (mut shared, _rx) = rig(vec![Bind::new(Action::Master, M4)]);
        let out = decide(&mut shared, M4, true, false);
        assert_eq!(
            out.events,
            vec![InputEvent::Engaged {
                action: Action::Master
            }]
        );
        assert!(!out.swallow);
    }

    #[test]
    fn f8_reaches_through_even_with_the_game_unfocused() {
        let (mut shared, _rx) = rig(vec![panic_bind()]);
        let out = decide(&mut shared, Trigger::Key { vk: 0x77 }, true, false);
        assert_eq!(
            out.events,
            vec![InputEvent::Engaged {
                action: Action::Panic
            }]
        );
        assert!(!out.swallow);
    }

    #[test]
    fn master_off_suspends_resize_but_leaves_the_key_to_the_game() {
        let (mut shared, _rx) = rig(vec![Bind::new(Action::Resize, RMB)]);
        shared.master_on = false;
        let out = decide(&mut shared, RMB, true, true);
        assert!(out.events.is_empty());
        assert!(!out.swallow, "bypassed means the game sees the click");
    }

    #[test]
    fn capture_takes_the_next_press_and_swallows_it() {
        let (mut shared, _rx) = rig(vec![]);
        shared.capture = true;
        assert!(decide(&mut shared, M4, false, true).events.is_empty());
        let out = decide(&mut shared, M4, true, true);
        assert_eq!(out.events, vec![InputEvent::Captured { trigger: M4 }]);
        assert!(out.swallow);
        assert!(!shared.capture, "capture is one-shot");
    }

    #[test]
    fn the_wheel_follows_the_bind_that_engaged_last() {
        let (mut shared, _rx) = rig(vec![
            Bind::new(Action::Resize, RMB),
            Bind::new(Action::Scope, KEY_C),
        ]);
        decide(&mut shared, RMB, true, true);
        decide(&mut shared, KEY_C, true, true);
        let out = decide_wheel(&shared, 1);
        assert_eq!(
            out.events,
            vec![InputEvent::Wheel {
                action: Action::Scope,
                notches: 1
            }]
        );
        assert!(out.swallow, "the game must not also scroll its weapons");
    }

    #[test]
    fn the_wheel_is_left_alone_when_nothing_is_held() {
        let (shared, _rx) = rig(vec![]);
        let out = decide_wheel(&shared, -1);
        assert!(out.events.is_empty());
        assert!(!out.swallow);
    }

    #[test]
    fn turning_wheel_adjust_off_gives_scrolling_back() {
        let (mut shared, _rx) = rig(vec![Bind::new(Action::Resize, RMB)]);
        shared.wheel_adjusts = false;
        decide(&mut shared, RMB, true, true);
        assert!(!decide_wheel(&shared, 3).swallow);
    }

    #[test]
    fn releasing_an_unheld_bind_reports_nothing() {
        let (mut shared, _rx) = rig(vec![Bind::new(Action::Resize, RMB)]);
        assert!(decide(&mut shared, RMB, false, true).events.is_empty());
    }
}
