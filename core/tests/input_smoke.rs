//! Proof that a bound key is actually kept out of the focused window.
//!
//! This is the one claim in the README that cannot be checked with a unit test:
//! `decide` can be asked what it *would* do, but whether returning `LRESULT(1)`
//! from a low-level hook really stops the keystroke reaching the application is
//! a question about Windows, not about our code.
//!
//! So the test builds a real target — a top-level edit control — brings it to
//! the front, and types into it with `SendInput`. Bound, the key must not
//! arrive. Aimed at another window, or unbound, the same keystroke must land.
//! The last two matter as much as the first: a hook that swallowed everything
//! would pass the first check and make the machine unusable.
//!
//! Ignored by default: it takes the keyboard and the foreground window for a
//! couple of seconds. It only looks for the one key it sends, so someone typing
//! at the same machine will not fail it.
//!
//! `cargo test -p tlkgrid-core --test input_smoke -- --ignored --nocapture`

#![cfg(windows)]

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::Duration;

use tlkgrid_core::bind::{Action, Bind, Trigger};
use tlkgrid_core::input::{Input, InputEvent};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, SetFocus, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
    GetMessageW, GetWindowTextW, PostThreadMessageW, SendMessageW, SetForegroundWindow, ShowWindow,
    TranslateMessage, CW_USEDEFAULT, MSG, SW_SHOW, WM_QUIT, WM_SETTEXT, WS_BORDER,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

/// VK_K. Nothing else on the keyboard is bound during the test.
const KEY_K: u32 = 0x4B;

static TARGET: AtomicIsize = AtomicIsize::new(0);

/// A real, focusable window that shows what it received. An edit control is the
/// simplest thing Windows will type into.
fn open_target() -> (HWND, u32) {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<u32>();
    std::thread::spawn(move || unsafe {
        let hwnd = CreateWindowExW(
            Default::default(),
            w!("EDIT"),
            // The window text of an edit control is its content: start empty.
            w!(""),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE | WS_BORDER,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            420,
            120,
            None,
            None,
            None,
            None,
        )
        .expect("edit control");
        TARGET.store(hwnd.0 as isize, Ordering::Release);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = ready_tx.send(windows::Win32::System::Threading::GetCurrentThreadId());

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = DestroyWindow(hwnd);
    });

    let thread_id = ready_rx.recv().expect("target thread");
    // Give the control a moment to exist before anything is sent to it.
    sleep(Duration::from_millis(400));
    (HWND(TARGET.load(Ordering::Acquire) as *mut _), thread_id)
}

/// Windows refuses a foreground change from a process that is not already in
/// front. Synthesising an Alt tap is the documented way to earn that right:
/// the input makes this process foreground-eligible for the next call. Nothing
/// is bound to Alt during the test, so the hook passes it straight through.
///
/// Confirming the result matters as much as asking for it — an unfocused edit
/// control looks exactly like a swallowed key.
fn focus(hwnd: HWND) {
    const VK_MENU: u32 = 0x12;
    for attempt in 0..25 {
        if attempt > 0 {
            tap_raw(VK_MENU);
        }
        unsafe {
            let _ = SetForegroundWindow(hwnd);
            let _ = BringWindowToTop(hwnd);
            let _ = SetFocus(Some(hwnd));
            if GetForegroundWindow() == hwnd {
                sleep(Duration::from_millis(150));
                return;
            }
        }
        sleep(Duration::from_millis(150));
    }
    let front = unsafe { GetForegroundWindow() };
    panic!(
        "could not bring the target window to the foreground; {:?} still has it",
        front.0
    );
}

fn clear(hwnd: HWND) {
    unsafe {
        let empty = windows::core::HSTRING::from("");
        SendMessageW(
            hwnd,
            WM_SETTEXT,
            Some(WPARAM(0)),
            Some(LPARAM(empty.as_ptr() as isize)),
        );
    }
}

fn tap(vk: u32) {
    tap_raw(vk);
    sleep(Duration::from_millis(350));
}

fn tap_raw(vk: u32) {
    let events = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk as u16),
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk as u16),
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ];
    unsafe {
        SendInput(&events, std::mem::size_of::<INPUT>() as i32);
    }
    sleep(Duration::from_millis(40));
}

fn typed_text(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let written = unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..written.max(0) as usize])
}

fn drain(events: &Receiver<InputEvent>) -> Vec<InputEvent> {
    events.try_iter().collect()
}

#[test]
#[ignore = "takes over the keyboard and the foreground window"]
fn the_hook_takes_the_bound_key_and_nothing_else() {
    let (target, target_thread) = open_target();
    let other_window = 0x1234_isize;
    let (input, events) = Input::start().expect("hooks");

    // 1 — bound and focused: the hook takes the key and reports it.
    input.set_target(Some(target.0 as isize));
    input.set_binds(vec![Bind::new(Action::Resize, Trigger::Key { vk: KEY_K })]);
    focus(target);
    clear(target);
    let _ = drain(&events);
    tap(KEY_K);
    let while_bound = typed_text(target);
    let reported = drain(&events);

    // 2 — the bind points at another window: the focus gate has to let the key
    // through to whatever is actually in front.
    input.set_target(Some(other_window));
    focus(target);
    clear(target);
    tap(KEY_K);
    let while_aimed_elsewhere = typed_text(target);
    let quiet = drain(&events);

    // 3 — no bind at all: the same keystroke lands untouched. Without this the
    // first check would also pass for a hook that swallowed everything.
    input.set_target(Some(target.0 as isize));
    input.set_binds(vec![]);
    focus(target);
    clear(target);
    tap(KEY_K);
    let with_no_bind = typed_text(target);

    unsafe {
        let _ = PostThreadMessageW(target_thread, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    drop(input);

    println!("bound {while_bound:?} · aimed elsewhere {while_aimed_elsewhere:?} · unbound {with_no_bind:?}");
    println!("events while bound: {reported:?}");

    assert!(
        while_bound.is_empty(),
        "the bound key reached the window: {while_bound:?} — the hook is not \
         swallowing, so every zoom bind would fire inside the game too"
    );
    assert!(
        reported.iter().any(|event| matches!(
            event,
            InputEvent::Engaged {
                action: Action::Resize
            }
        )),
        "the key was swallowed but no Engaged event arrived; it was eaten for nothing"
    );
    assert!(
        while_aimed_elsewhere.contains('k'),
        "a bind aimed at another window swallowed a key meant for this one:          {while_aimed_elsewhere:?}"
    );
    assert!(
        !quiet
            .iter()
            .any(|event| matches!(event, InputEvent::Engaged { .. })),
        "a bind fired while its own target was not in front"
    );
    assert!(
        with_no_bind.contains('k'),
        "with no bind the keystroke must reach the window untouched: {with_no_bind:?}"
    );
}
