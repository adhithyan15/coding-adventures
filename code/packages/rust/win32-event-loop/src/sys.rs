//! Windows-only runtime layer: WNDPROC, routing table, message pump.
//!
//! This module compiles only on Windows (gated by `#[cfg(target_os =
//! "windows")]` in `lib.rs`). It is the *only* place in the crate that
//! calls Win32 APIs directly; everything else stays cross-platform and
//! goes through the pure [`crate::translate`] function.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DispatchMessageW, GetMessageW, PeekMessageW, PostQuitMessage,
    TranslateMessage, MSG, PM_REMOVE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};

use crate::{Action, EventHandler, LoopError, Modifiers, Rect};

// ---------------------------------------------------------------------
// Thread-local routing table
// ---------------------------------------------------------------------
//
// Win32 message queues are per-thread, so the routing table is too.
// Using `thread_local!` rather than a `Mutex<HashMap>` lets us keep
// `EventLoop` `!Send + !Sync` honestly — there is no shared state at
// all across threads.

thread_local! {
    static ROUTING: RefCell<HashMap<isize, Box<dyn EventHandler>>> =
        RefCell::new(HashMap::new());
}

/// Insert `handler` into the thread-local routing table for `hwnd`.
///
/// # Panics
///
/// Panics if `hwnd` is already registered on this thread.
pub(crate) fn register(hwnd: HWND, handler: Box<dyn EventHandler>) {
    let key = hwnd.0 as isize;
    ROUTING.with(|cell| {
        let mut table = cell.borrow_mut();
        if table.contains_key(&key) {
            panic!("win32-event-loop: HWND {:?} already registered on this thread", hwnd);
        }
        table.insert(key, handler);
    });
}

/// Remove `hwnd` from the routing table. Called by `Registration::drop`
/// and by the WNDPROC on `WM_DESTROY`.
pub(crate) fn unregister(hwnd: HWND) {
    let key = hwnd.0 as isize;
    ROUTING.with(|cell| {
        // Ignore the result — double-unregister is safe (idempotent).
        let _ = cell.borrow_mut().remove(&key);
    });
}

/// Whether the routing table for this thread is empty.
fn routing_is_empty() -> bool {
    ROUTING.with(|cell| cell.borrow().is_empty())
}

// ---------------------------------------------------------------------
// Shared WNDPROC
// ---------------------------------------------------------------------
//
// Callers that want their windows routed through this crate's loop pass
// this function as `WNDCLASS::lpfnWndProc` when registering the class.
// Windows-Mosaic-emitted hosts always do; hand-written hosts opt in by
// pointing `lpfnWndProc` here.

/// Window procedure used by every WNDCLASS that participates in this
/// event loop. Takes the standard `(hwnd, message, wparam, lparam)`
/// arguments and returns `LRESULT`.
///
/// The function:
/// 1. Captures modifier-key state via `GetKeyState` so the pure
///    [`crate::translate`] function can stay platform-agnostic.
/// 2. For `WM_PAINT`, brackets the dispatch with `BeginPaint`/`EndPaint`
///    and feeds the resulting rect into the event.
/// 3. For `WM_DESTROY`, auto-unregisters the HWND and posts
///    `WM_QUIT(0)` if the routing table is now empty.
/// 4. Looks up the registered handler, dispatches the translated event,
///    and either consumes the message ([`Action::Consumed`]) or hands
///    it to `DefWindowProc`.
///
/// # Safety
///
/// The standard Win32 contract — Windows guarantees a stable HWND, the
/// other args are passed straight to `DefWindowProc` on the fallback
/// path. The internal `BeginPaint`/`EndPaint` pair is correctly balanced.
pub extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // ── WM_DESTROY: auto-unregister, post quit when last window goes away ──
    if message == crate::msg::WM_DESTROY {
        unregister(hwnd);
        if routing_is_empty() {
            // SAFETY: PostQuitMessage is always safe to call.
            unsafe { PostQuitMessage(0) };
        }
        // Still hand to DefWindowProc so Windows does its own destruction
        // bookkeeping; we just made sure our table is clean first.
        // SAFETY: passing through the standard arguments verbatim.
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }

    // ── Capture modifier-key state ──
    let modifiers = current_modifiers();

    // ── Special-case WM_PAINT so the rect comes from BeginPaint ──
    if message == crate::msg::WM_PAINT {
        let mut ps = PAINTSTRUCT::default();
        // SAFETY: BeginPaint is paired with EndPaint below. The PAINTSTRUCT
        // is a stack local of the right type.
        let _hdc = unsafe { BeginPaint(hwnd, &mut ps) };
        let paint_rect = Rect {
            left:   ps.rcPaint.left,
            top:    ps.rcPaint.top,
            right:  ps.rcPaint.right,
            bottom: ps.rcPaint.bottom,
        };
        let action = dispatch_to_handler(
            hwnd, message, wparam.0, lparam.0, paint_rect, modifiers,
        );
        // SAFETY: matches the BeginPaint above; ps is initialised.
        let _ = unsafe { EndPaint(hwnd, &ps) };
        return match action {
            Action::Consumed => LRESULT(0),
            // SAFETY: see top of file.
            Action::Default  => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        };
    }

    // ── All other messages: translate + dispatch ──
    let action = dispatch_to_handler(
        hwnd, message, wparam.0, lparam.0, Rect::default(), modifiers,
    );
    match action {
        Action::Consumed => LRESULT(0),
        Action::Default  => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// Translate the raw Win32 args and hand the resulting event to the
/// registered handler. Returns the handler's action, or
/// [`Action::Default`] when no handler is registered for `hwnd`.
fn dispatch_to_handler(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    paint_rect: Rect,
    modifiers: Modifiers,
) -> Action {
    let Some(event) = crate::translate(hwnd, message, wparam, lparam, paint_rect, modifiers)
    else {
        // translate() returned None — message was handled internally.
        return Action::Consumed;
    };

    let key = hwnd.0 as isize;
    ROUTING.with(|cell| {
        // We take ownership of the handler box out of the table for the
        // duration of `handle()` so the handler can re-enter the table
        // (e.g. register a new HWND from inside a paint callback) without
        // tripping the RefCell's mutable-borrow guard. Re-insertion is
        // unconditional unless the handler itself unregistered.
        let handler_opt = cell.borrow_mut().remove(&key);
        if let Some(mut handler) = handler_opt {
            let action = handler.handle(event);
            // Re-insert unless someone else already put a new handler in
            // for this hwnd (very unlikely; would be a programming bug).
            let mut table = cell.borrow_mut();
            table.entry(key).or_insert(handler);
            action
        } else {
            Action::Default
        }
    })
}

/// Capture modifier-key state from `GetKeyState`. Bit 0x8000 of the
/// returned short means the key is currently down.
fn current_modifiers() -> Modifiers {
    fn down(vk: VIRTUAL_KEY) -> bool {
        // SAFETY: GetKeyState is always safe.
        let state = unsafe { GetKeyState(vk.0 as i32) };
        (state as u16) & 0x8000 != 0
    }
    Modifiers {
        shift: down(VK_SHIFT),
        ctrl:  down(VK_CONTROL),
        alt:   down(VK_MENU),
        win:   down(VK_LWIN) || down(VK_RWIN),
    }
}

// ---------------------------------------------------------------------
// run() / pump_once() / request_quit()
// ---------------------------------------------------------------------

/// Blocking pump — drains until `WM_QUIT` arrives.
pub(crate) fn run() -> Result<i32, LoopError> {
    loop {
        let mut msg = MSG::default();
        // SAFETY: GetMessageW with hwnd=null pumps the whole thread queue.
        let res = unsafe { GetMessageW(&mut msg, HWND(ptr::null_mut()), 0, 0) };
        match res.0 {
            -1 => {
                // SAFETY: GetLastError is always safe.
                let err = unsafe {
                    windows::Win32::Foundation::GetLastError().0
                };
                return Err(LoopError::GetMessageFailed(err));
            }
            0 => {
                // WM_QUIT — return the WPARAM.
                return Ok(msg.wParam.0 as i32);
            }
            _ => {
                // SAFETY: both calls take a fully-initialised MSG.
                unsafe {
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }
            }
        }
    }
}

/// Non-blocking pump — drains up to `max` waiting messages and returns.
pub(crate) fn pump_once(max: usize) -> Result<usize, LoopError> {
    let mut count = 0usize;
    while count < max {
        let mut msg = MSG::default();
        // SAFETY: PeekMessageW with PM_REMOVE pops one waiting message.
        let got = unsafe {
            PeekMessageW(&mut msg, HWND(ptr::null_mut()), 0, 0, PM_REMOVE).as_bool()
        };
        if !got {
            break;
        }
        if msg.message == crate::msg::WM_QUIT {
            // Re-post so a subsequent run() can see it; pump_once does
            // not return WM_QUIT directly because its return type is the
            // count of messages processed.
            // SAFETY: PostQuitMessage is always safe.
            unsafe { PostQuitMessage(msg.wParam.0 as i32) };
            break;
        }
        // SAFETY: msg is fully initialised by PeekMessageW.
        unsafe {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
        count += 1;
    }
    Ok(count)
}

/// Post `WM_QUIT(0)` to the current thread.
pub(crate) fn request_quit() {
    // SAFETY: PostQuitMessage is always safe.
    unsafe { PostQuitMessage(0) };
}

