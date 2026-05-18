//! # win32-event-loop
//!
//! A small, single-purpose Win32 message pump decoupled from window creation
//! and rendering. See `code/specs/win32-event-loop.md` for the design.
//!
//! ## What this crate does
//!
//! 1. Translates the standard `WM_*` messages into a typed [`Event`] enum.
//! 2. Maintains a per-thread routing table from `HWND` → [`EventHandler`].
//! 3. Drains the queue via `GetMessage` / `DispatchMessage` either blocking
//!    ([`EventLoop::run`]) or non-blocking with a budget ([`EventLoop::pump_once`]).
//!
//! ## What this crate does NOT do
//!
//! - Create windows or register `WNDCLASS`es — that is the caller's job.
//!   Use the `windows` crate directly, or `window-win32`, then pass the
//!   resulting `HWND` to [`EventLoop::register`].
//! - Render. The [`Event::Paint`] variant only carries the invalidate rect;
//!   the consumer is expected to perform its own drawing inside the handler.
//! - Cross-platform abstraction. That belongs in `window-core` /
//!   `window-win32`. This crate's public types are platform-agnostic but
//!   the actual pump only runs on Windows; non-Windows builds expose the
//!   same surface with stub methods that return [`LoopError::Unsupported`].
//!
//! ## Public-API stability promise
//!
//! The types in this module — [`Event`], [`Action`], [`EventHandler`],
//! [`Registration`], [`EventLoop`], [`LoopError`] — are the contract that
//! `mosaic-emit-win32` and other consumers compile against. The crate
//! version follows semver; breaking changes to these types bump the major.

// Doc-comments are added on the major public items; individual enum
// variant fields are self-explanatory from their names. We don't enforce
// `missing_docs` at the field level because it adds noise without
// readability benefit.

use std::fmt;

#[cfg(target_os = "windows")]
mod sys;

#[cfg(target_os = "windows")]
pub use sys::wndproc;

// =====================================================================
// HWND abstraction
// =====================================================================
//
// On Windows, `Hwnd` is a re-export of `windows::Win32::Foundation::HWND`.
// On other platforms (Linux / macOS) it is a transparent newtype around
// `usize` so the public API stays one shape and crates that depend on
// this one can `cargo check` cross-platform.

/// Opaque Win32 window handle.
///
/// On Windows this is `windows::Win32::Foundation::HWND`; on every other
/// platform it is a stub newtype that carries no behaviour. Cross-platform
/// dependents can use it in signatures but cannot do anything useful with
/// it without `#[cfg(target_os = "windows")]`.
#[cfg(target_os = "windows")]
pub type Hwnd = windows::Win32::Foundation::HWND;

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hwnd(pub usize);

#[cfg(not(target_os = "windows"))]
impl Hwnd {
    /// A null/invalid HWND placeholder for non-Windows builds.
    pub const fn null() -> Self { Hwnd(0) }
}

// =====================================================================
// Geometry
// =====================================================================

/// Rectangle in physical pixels, with `right`/`bottom` exclusive — the
/// same shape `Win32::RECT` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left edge in pixels.
    pub left:   i32,
    /// Top edge in pixels.
    pub top:    i32,
    /// Right edge in pixels (exclusive).
    pub right:  i32,
    /// Bottom edge in pixels (exclusive).
    pub bottom: i32,
}

/// Width × height in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    /// Width in pixels.
    pub width:  u32,
    /// Height in pixels.
    pub height: u32,
}

/// Modifier-key bitset captured at the moment the event was dispatched.
///
/// On Windows the values come from `GetKeyState(VK_SHIFT)` etc. at message
/// dispatch time. On non-Windows builds this struct is constructible but
/// never produced by the loop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Shift key was held.
    pub shift: bool,
    /// Ctrl key was held.
    pub ctrl:  bool,
    /// Alt key was held.
    pub alt:   bool,
    /// Windows ("Super") key was held.
    pub win:   bool,
}

// =====================================================================
// Virtual keys and mouse buttons
// =====================================================================

/// Subset of the Win32 virtual-key namespace this loop translates.
///
/// Anything outside this enum surfaces as [`VirtualKey::Other`] carrying
/// the raw VK code, so callers can still inspect it. F-keys and ASCII
/// alphanumerics get their own variants because they are by far the most
/// common.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VirtualKey {
    /// Return / Enter.
    Enter,
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Function key F1..F24. The inner u8 is the index (1-based).
    F(u8),
    /// ASCII A–Z or 0–9 (uppercase form — VK_A == 0x41 etc.).
    Char(u8),
    /// Anything else — the raw Win32 VK code is preserved.
    Other(u32),
}

/// Which mouse button fired a click event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Middle mouse button (wheel click).
    Middle,
    /// Right mouse button.
    Right,
    /// Extra button 1 (back).
    X1,
    /// Extra button 2 (forward).
    X2,
}

// =====================================================================
// Event enum
// =====================================================================

/// A single typed event delivered to a registered HWND's handler.
///
/// The variants are a minimal common subset every Win32 consumer cares
/// about; the raw message is also exposed via [`Event::Raw`] for consumers
/// that need device-specific or custom `WM_USER+N` messages.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// Window asked to repaint a region. The handler must perform the
    /// paint synchronously; the loop calls `BeginPaint` / `EndPaint`
    /// around the dispatch so the consumer only sees the invalidate
    /// rect.
    Paint { hwnd: Hwnd, rect: Rect },

    /// Window resized. `client` is the new client-area size in physical
    /// pixels.
    Resize { hwnd: Hwnd, client: Size },

    /// A character was typed (translated `WM_CHAR` — Unicode codepoint
    /// after IME composition).
    Char { hwnd: Hwnd, codepoint: u32, modifiers: Modifiers },

    /// A key was pressed (raw virtual-key code).
    KeyDown { hwnd: Hwnd, vk: VirtualKey, modifiers: Modifiers, repeat: bool },
    /// A key was released.
    KeyUp { hwnd: Hwnd, vk: VirtualKey, modifiers: Modifiers },

    /// Mouse button pressed. (`x`, `y`) are client-area coordinates.
    MouseDown { hwnd: Hwnd, button: MouseButton, x: i32, y: i32, modifiers: Modifiers },
    /// Mouse button released.
    MouseUp   { hwnd: Hwnd, button: MouseButton, x: i32, y: i32, modifiers: Modifiers },
    /// Pointer moved within the window's client area.
    MouseMove { hwnd: Hwnd, x: i32, y: i32, modifiers: Modifiers },
    /// Mouse wheel scrolled. `delta` is signed and in `WHEEL_DELTA` (120)
    /// units.
    MouseWheel { hwnd: Hwnd, x: i32, y: i32, delta: i32, modifiers: Modifiers },

    /// Window close was requested (the X button, Alt+F4, system menu).
    /// Returning [`Action::Default`] still closes the window; return
    /// [`Action::Consumed`] to veto.
    CloseRequested { hwnd: Hwnd },

    /// Window gained focus.
    FocusIn { hwnd: Hwnd },
    /// Window lost focus.
    FocusOut { hwnd: Hwnd },

    /// Escape hatch: an unrecognised message. The handler may return
    /// [`Action::Default`] to let `DefWindowProc` handle it, or
    /// [`Action::Consumed`] to return zero without calling `DefWindowProc`.
    Raw { hwnd: Hwnd, message: u32, wparam: usize, lparam: isize },
}

/// What the handler did with the event.
///
/// - [`Action::Consumed`] — the handler fully handled the message; the
///   loop returns zero and does NOT call `DefWindowProc`.
/// - [`Action::Default`]  — the handler did nothing or wants the OS
///   default behaviour; the loop calls `DefWindowProc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Handler took responsibility; do not call `DefWindowProc`.
    Consumed,
    /// Hand off to `DefWindowProc`.
    Default,
}

// =====================================================================
// Handler trait + closure adapter
// =====================================================================

/// Object-safe handler trait. Consumers either implement this directly or
/// use [`closure_handler`] for simple lambdas.
pub trait EventHandler: 'static {
    /// Called once per dispatched event. Return [`Action::Consumed`] to
    /// suppress the default `DefWindowProc` call, or [`Action::Default`]
    /// to let the OS handle the message normally.
    fn handle(&mut self, event: Event) -> Action;
}

/// Adapt a `FnMut(Event) -> Action` closure into an [`EventHandler`].
pub fn closure_handler<F>(f: F) -> impl EventHandler
where
    F: FnMut(Event) -> Action + 'static,
{
    struct ClosureHandler<F>(F);
    impl<F: FnMut(Event) -> Action + 'static> EventHandler for ClosureHandler<F> {
        fn handle(&mut self, event: Event) -> Action {
            (self.0)(event)
        }
    }
    ClosureHandler(f)
}

// =====================================================================
// Errors
// =====================================================================

/// Recoverable errors returned by [`EventLoop::run`] and
/// [`EventLoop::pump_once`].
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LoopError {
    /// `GetMessage` returned -1 (a real Win32 failure). The inner u32
    /// is `GetLastError()` at the time of the failure.
    #[error("GetMessage returned -1 (Win32 error {0:#x})")]
    GetMessageFailed(u32),

    /// The crate was built for a non-Windows target. The stub `run` /
    /// `pump_once` always return this — no message queue exists.
    #[error("win32-event-loop runtime only runs on Windows")]
    Unsupported,
}

// =====================================================================
// Registration (RAII)
// =====================================================================

/// RAII handle returned by [`EventLoop::register`]. Dropping it
/// unregisters the HWND from the loop's routing table; the HWND itself
/// is NOT destroyed — that remains the caller's responsibility.
#[must_use = "dropping the Registration unregisters the HWND from the event loop"]
pub struct Registration {
    hwnd: Hwnd,
    // The handle is logically `!Send + !Sync` because the routing table
    // is thread-local. The PhantomData enforces that without leaking
    // implementation details through the type.
    _not_send_sync: std::marker::PhantomData<*const ()>,
}

impl fmt::Debug for Registration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registration")
            .field("hwnd", &format_args!("{:?}", self.hwnd))
            .finish()
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        // On non-Windows the routing table is a no-op stub; calling
        // `unregister` is still safe.
        let _ = self.hwnd;
        #[cfg(target_os = "windows")]
        sys::unregister(self.hwnd);
    }
}

// =====================================================================
// EventLoop
// =====================================================================

/// The event-loop namespace. `EventLoop` itself is a zero-sized handle;
/// all the per-thread state (routing table, queue) lives in the `sys`
/// module's thread-locals. Calling `register`, `run`, or `pump_once`
/// from a different thread than the one that originally registered an
/// HWND is harmless — the call simply won't see the other thread's
/// routing table.
///
/// The returned [`Registration`] handle is `!Send + !Sync` so it cannot
/// be moved across threads, which is the actual cross-thread foot-gun
/// this design avoids.
pub struct EventLoop {
    // No state — this is a marker type. The routing table that conceptually
    // belongs to "the event loop" lives in `sys`'s thread-locals.
    _private: (),
}

impl EventLoop {
    /// Get the current thread's event loop.
    ///
    /// `EventLoop` is a marker namespace; the actual per-thread state
    /// lives in the `sys` module's `thread_local!`. On non-Windows
    /// builds the returned loop is a stub whose `run` / `pump_once`
    /// return [`LoopError::Unsupported`].
    pub fn current() -> &'static EventLoop {
        static LOOP: EventLoop = EventLoop { _private: () };
        &LOOP
    }

    /// Register a handler for messages destined to `hwnd`. Returns a
    /// [`Registration`] whose `Drop` impl unregisters automatically.
    ///
    /// # Panics
    ///
    /// - If `hwnd` is already registered on this loop.
    /// - If called from a thread different from the one that owns the
    ///   HWND (Win32 message queues are per-thread).
    pub fn register<H>(&self, hwnd: Hwnd, handler: H) -> Registration
    where
        H: EventHandler,
    {
        #[cfg(target_os = "windows")]
        sys::register(hwnd, Box::new(handler));
        #[cfg(not(target_os = "windows"))]
        let _ = handler; // suppress unused warning
        Registration {
            hwnd,
            _not_send_sync: std::marker::PhantomData,
        }
    }

    /// Block forever, draining the message queue, until `PostQuitMessage`
    /// is called (e.g. via [`EventLoop::request_quit`] or from a handler
    /// that decides it's done). Returns the WPARAM passed to `WM_QUIT`
    /// (conventionally `0` on clean exit).
    pub fn run(&self) -> Result<i32, LoopError> {
        #[cfg(target_os = "windows")]
        return sys::run();
        #[cfg(not(target_os = "windows"))]
        return Err(LoopError::Unsupported);
    }

    /// Drain at most `max` messages, returning the number actually
    /// processed. Useful for integrating with non-Win32 schedulers —
    /// call this every frame instead of [`EventLoop::run`]. Returns
    /// `Ok(0)` when the queue is empty AND nothing was waiting.
    pub fn pump_once(&self, max: usize) -> Result<usize, LoopError> {
        #[cfg(target_os = "windows")]
        return sys::pump_once(max);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = max;
            Err(LoopError::Unsupported)
        }
    }

    /// Post `WM_QUIT(0)` to this thread, causing any in-flight
    /// [`EventLoop::run`] to return after the current message finishes
    /// dispatching.
    pub fn request_quit(&self) {
        #[cfg(target_os = "windows")]
        sys::request_quit();
    }
}

// =====================================================================
// Pure translation table — cross-platform-testable
// =====================================================================

/// Translate a raw Win32 message tuple into the typed [`Event`] enum.
///
/// This function is pure: it does NOT call `BeginPaint`, `GetKeyState`,
/// or any other Win32 API. The real `wndproc` (in `sys.rs`) calls it
/// after capturing modifier-key state and the paint rect; the pure-
/// translation table is what we unit-test cross-platform.
///
/// Returns `None` only for messages the routing layer handles internally
/// (today: just `WM_DESTROY` — that one auto-unregisters the HWND).
///
/// The mapping mirrors §5.3 of `code/specs/win32-event-loop.md`.
pub fn translate(
    hwnd: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    paint_rect: Rect,
    modifiers: Modifiers,
) -> Option<Event> {
    use msg::*;

    // Decompose `lparam` for mouse messages: low 16 bits = x, high 16 = y.
    // Both are signed (negative coordinates happen with mouse capture).
    let mouse_x = (lparam & 0xFFFF) as i16 as i32;
    let mouse_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

    match message {
        WM_PAINT => Some(Event::Paint { hwnd, rect: paint_rect }),

        WM_SIZE => {
            let width = (lparam & 0xFFFF) as u32;
            let height = ((lparam >> 16) & 0xFFFF) as u32;
            Some(Event::Resize { hwnd, client: Size { width, height } })
        }

        WM_CHAR => Some(Event::Char {
            hwnd,
            codepoint: wparam as u32,
            modifiers,
        }),

        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let repeat = (lparam & (1 << 30)) != 0;
            Some(Event::KeyDown {
                hwnd,
                vk: vk_from_raw(wparam as u32),
                modifiers,
                repeat,
            })
        }
        WM_KEYUP | WM_SYSKEYUP => Some(Event::KeyUp {
            hwnd,
            vk: vk_from_raw(wparam as u32),
            modifiers,
        }),

        WM_LBUTTONDOWN => Some(Event::MouseDown {
            hwnd, button: MouseButton::Left, x: mouse_x, y: mouse_y, modifiers,
        }),
        WM_MBUTTONDOWN => Some(Event::MouseDown {
            hwnd, button: MouseButton::Middle, x: mouse_x, y: mouse_y, modifiers,
        }),
        WM_RBUTTONDOWN => Some(Event::MouseDown {
            hwnd, button: MouseButton::Right, x: mouse_x, y: mouse_y, modifiers,
        }),
        WM_XBUTTONDOWN => Some(Event::MouseDown {
            hwnd,
            button: xbutton_from_wparam(wparam),
            x: mouse_x, y: mouse_y, modifiers,
        }),

        WM_LBUTTONUP => Some(Event::MouseUp {
            hwnd, button: MouseButton::Left, x: mouse_x, y: mouse_y, modifiers,
        }),
        WM_MBUTTONUP => Some(Event::MouseUp {
            hwnd, button: MouseButton::Middle, x: mouse_x, y: mouse_y, modifiers,
        }),
        WM_RBUTTONUP => Some(Event::MouseUp {
            hwnd, button: MouseButton::Right, x: mouse_x, y: mouse_y, modifiers,
        }),
        WM_XBUTTONUP => Some(Event::MouseUp {
            hwnd,
            button: xbutton_from_wparam(wparam),
            x: mouse_x, y: mouse_y, modifiers,
        }),

        WM_MOUSEMOVE => Some(Event::MouseMove {
            hwnd, x: mouse_x, y: mouse_y, modifiers,
        }),
        WM_MOUSEWHEEL => {
            // The wheel delta is in the HIGH 16 bits of wparam, signed.
            let delta = ((wparam >> 16) & 0xFFFF) as i16 as i32;
            Some(Event::MouseWheel {
                hwnd, x: mouse_x, y: mouse_y, delta, modifiers,
            })
        }

        WM_CLOSE     => Some(Event::CloseRequested { hwnd }),
        WM_SETFOCUS  => Some(Event::FocusIn  { hwnd }),
        WM_KILLFOCUS => Some(Event::FocusOut { hwnd }),

        // WM_DESTROY is consumed by the routing layer (auto-unregister)
        // and never reaches a handler as a typed Event. The routing
        // layer calls translate() AFTER its own bookkeeping so we
        // return None here to mean "do not surface".
        WM_DESTROY => None,

        // Anything else falls through as the escape hatch.
        _ => Some(Event::Raw { hwnd, message, wparam, lparam }),
    }
}

/// Map a raw Win32 VK code into the [`VirtualKey`] enum.
fn vk_from_raw(vk: u32) -> VirtualKey {
    use vk_codes::*;
    match vk {
        VK_RETURN    => VirtualKey::Enter,
        VK_ESCAPE    => VirtualKey::Escape,
        VK_TAB       => VirtualKey::Tab,
        VK_BACK      => VirtualKey::Backspace,
        VK_DELETE    => VirtualKey::Delete,
        VK_LEFT      => VirtualKey::Left,
        VK_RIGHT     => VirtualKey::Right,
        VK_UP        => VirtualKey::Up,
        VK_DOWN      => VirtualKey::Down,
        VK_HOME      => VirtualKey::Home,
        VK_END       => VirtualKey::End,
        VK_PRIOR     => VirtualKey::PageUp,    // VK_PRIOR == Page Up
        VK_NEXT      => VirtualKey::PageDown,  // VK_NEXT == Page Down
        // F1..F24 are 0x70..0x87 contiguously.
        v if (0x70..=0x87).contains(&v) => VirtualKey::F((v - 0x70 + 1) as u8),
        // ASCII A..Z (0x41..0x5A) and 0..9 (0x30..0x39) — the uppercase
        // VK codes are identical to ASCII for these ranges.
        v if (0x30..=0x39).contains(&v) || (0x41..=0x5A).contains(&v) => {
            VirtualKey::Char(v as u8)
        }
        other => VirtualKey::Other(other),
    }
}

/// Decode the WM_XBUTTONDOWN/UP `wparam` to figure out which X button.
fn xbutton_from_wparam(wparam: usize) -> MouseButton {
    // High word of wparam is XBUTTON1 (1) or XBUTTON2 (2).
    match (wparam >> 16) & 0xFFFF {
        1 => MouseButton::X1,
        2 => MouseButton::X2,
        // The spec only defines XBUTTON1/2, but if a future driver
        // emits 3+ we map to X2 rather than panicking.
        _ => MouseButton::X2,
    }
}

// =====================================================================
// Win32 message + VK code constants — duplicated here so the pure
// translation module can be unit-tested on non-Windows targets without
// pulling in the `windows` crate.
// =====================================================================

/// `WM_*` message constants used by the translation table.
#[allow(missing_docs)]
pub mod msg {
    pub const WM_PAINT:        u32 = 0x000F;
    pub const WM_DESTROY:      u32 = 0x0002;
    pub const WM_SIZE:         u32 = 0x0005;
    pub const WM_SETFOCUS:     u32 = 0x0007;
    pub const WM_KILLFOCUS:    u32 = 0x0008;
    pub const WM_CLOSE:        u32 = 0x0010;
    pub const WM_KEYDOWN:      u32 = 0x0100;
    pub const WM_KEYUP:        u32 = 0x0101;
    pub const WM_CHAR:         u32 = 0x0102;
    pub const WM_SYSKEYDOWN:   u32 = 0x0104;
    pub const WM_SYSKEYUP:     u32 = 0x0105;
    pub const WM_MOUSEMOVE:    u32 = 0x0200;
    pub const WM_LBUTTONDOWN:  u32 = 0x0201;
    pub const WM_LBUTTONUP:    u32 = 0x0202;
    pub const WM_RBUTTONDOWN:  u32 = 0x0204;
    pub const WM_RBUTTONUP:    u32 = 0x0205;
    pub const WM_MBUTTONDOWN:  u32 = 0x0207;
    pub const WM_MBUTTONUP:    u32 = 0x0208;
    pub const WM_MOUSEWHEEL:   u32 = 0x020A;
    pub const WM_XBUTTONDOWN:  u32 = 0x020B;
    pub const WM_XBUTTONUP:    u32 = 0x020C;
    pub const WM_QUIT:         u32 = 0x0012;
}

/// `VK_*` virtual-key constants used by [`vk_from_raw`].
#[allow(missing_docs)]
pub mod vk_codes {
    pub const VK_BACK:    u32 = 0x08;
    pub const VK_TAB:     u32 = 0x09;
    pub const VK_RETURN:  u32 = 0x0D;
    pub const VK_ESCAPE:  u32 = 0x1B;
    pub const VK_PRIOR:   u32 = 0x21;
    pub const VK_NEXT:    u32 = 0x22;
    pub const VK_END:     u32 = 0x23;
    pub const VK_HOME:    u32 = 0x24;
    pub const VK_LEFT:    u32 = 0x25;
    pub const VK_UP:      u32 = 0x26;
    pub const VK_RIGHT:   u32 = 0x27;
    pub const VK_DOWN:    u32 = 0x28;
    pub const VK_DELETE:  u32 = 0x2E;
}

// =====================================================================
// Tests — pure-translation, cross-platform
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn hwnd() -> Hwnd {
        // The pure translation function doesn't dereference the HWND;
        // a null/zero handle is a perfectly fine test fixture.
        #[cfg(target_os = "windows")]
        { windows::Win32::Foundation::HWND(std::ptr::null_mut()) }
        #[cfg(not(target_os = "windows"))]
        { Hwnd::null() }
    }

    fn no_mods() -> Modifiers { Modifiers::default() }
    fn no_rect() -> Rect      { Rect::default() }

    // ---- §5.3 table: every row gets a test ----

    #[test]
    fn wm_paint_translates_to_event_paint() {
        let r = Rect { left: 10, top: 20, right: 30, bottom: 40 };
        let e = translate(hwnd(), msg::WM_PAINT, 0, 0, r, no_mods()).unwrap();
        assert!(matches!(e, Event::Paint { rect, .. } if rect == r));
    }

    #[test]
    fn wm_size_packs_low_high_word_as_client_size() {
        // lparam: low word = width 1024, high word = height 768
        let lparam = 1024i32 | (768i32 << 16);
        let e = translate(hwnd(), msg::WM_SIZE, 0, lparam as isize, no_rect(), no_mods()).unwrap();
        match e {
            Event::Resize { client, .. } => {
                assert_eq!(client.width, 1024);
                assert_eq!(client.height, 768);
            }
            _ => panic!("expected Resize, got {e:?}"),
        }
    }

    #[test]
    fn wm_char_translates_codepoint_from_wparam() {
        // wparam = 'A' (0x41)
        let e = translate(hwnd(), msg::WM_CHAR, 0x41, 0, no_rect(), no_mods()).unwrap();
        match e {
            Event::Char { codepoint, .. } => assert_eq!(codepoint, 0x41),
            _ => panic!("expected Char"),
        }
    }

    #[test]
    fn wm_char_passes_modifier_state_through() {
        let mods = Modifiers { shift: true, ctrl: true, ..Default::default() };
        let e = translate(hwnd(), msg::WM_CHAR, b'a' as usize, 0, no_rect(), mods).unwrap();
        match e {
            Event::Char { modifiers, .. } => {
                assert!(modifiers.shift && modifiers.ctrl);
                assert!(!modifiers.alt && !modifiers.win);
            }
            _ => panic!("expected Char"),
        }
    }

    #[test]
    fn wm_keydown_translates_vk_and_marks_repeat_bit() {
        // lparam bit 30 = repeat flag
        let repeat_lparam = 1isize << 30;
        let e = translate(
            hwnd(), msg::WM_KEYDOWN,
            vk_codes::VK_RETURN as usize, repeat_lparam,
            no_rect(), no_mods(),
        ).unwrap();
        match e {
            Event::KeyDown { vk, repeat, .. } => {
                assert_eq!(vk, VirtualKey::Enter);
                assert!(repeat);
            }
            _ => panic!("expected KeyDown"),
        }
    }

    #[test]
    fn wm_keydown_no_repeat_bit_means_first_press() {
        let e = translate(
            hwnd(), msg::WM_KEYDOWN,
            vk_codes::VK_ESCAPE as usize, 0,
            no_rect(), no_mods(),
        ).unwrap();
        match e {
            Event::KeyDown { vk, repeat, .. } => {
                assert_eq!(vk, VirtualKey::Escape);
                assert!(!repeat);
            }
            _ => panic!("expected KeyDown"),
        }
    }

    #[test]
    fn wm_syskeydown_is_treated_as_keydown() {
        let e = translate(
            hwnd(), msg::WM_SYSKEYDOWN,
            vk_codes::VK_TAB as usize, 0,
            no_rect(), no_mods(),
        ).unwrap();
        assert!(matches!(e, Event::KeyDown { vk: VirtualKey::Tab, .. }));
    }

    #[test]
    fn wm_keyup_translates_to_event_keyup() {
        let e = translate(
            hwnd(), msg::WM_KEYUP,
            vk_codes::VK_LEFT as usize, 0,
            no_rect(), no_mods(),
        ).unwrap();
        assert!(matches!(e, Event::KeyUp { vk: VirtualKey::Left, .. }));
    }

    #[test]
    fn wm_lbuttondown_decodes_xy_from_lparam() {
        // low word = x (100), high word = y (200)
        let lparam = 100i32 | (200i32 << 16);
        let e = translate(
            hwnd(), msg::WM_LBUTTONDOWN, 0, lparam as isize,
            no_rect(), no_mods(),
        ).unwrap();
        match e {
            Event::MouseDown { button, x, y, .. } => {
                assert_eq!(button, MouseButton::Left);
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("expected MouseDown"),
        }
    }

    #[test]
    fn wm_mousemove_supports_negative_coordinates() {
        // Negative coords happen during mouse capture. Win32 packs the
        // pair as two signed 16-bit values inside lparam: low word = x,
        // high word = y. -5i16 = 0xFFFB; -3i16 = 0xFFFD; combined
        // lparam = 0xFFFD_FFFB.
        let lparam: isize = (((-3i16 as u16 as u32) << 16) | (-5i16 as u16 as u32)) as isize;
        let e = translate(
            hwnd(), msg::WM_MOUSEMOVE, 0, lparam,
            no_rect(), no_mods(),
        ).unwrap();
        match e {
            Event::MouseMove { x, y, .. } => {
                assert_eq!(x, -5);
                assert_eq!(y, -3);
            }
            _ => panic!("expected MouseMove"),
        }
    }

    #[test]
    fn wm_mousewheel_extracts_signed_delta_from_high_wparam() {
        // delta = -120 (one wheel notch down). HIWORD of wparam is signed.
        let wparam = ((-120i32 as u32) << 16) as usize;
        let e = translate(
            hwnd(), msg::WM_MOUSEWHEEL, wparam, 0,
            no_rect(), no_mods(),
        ).unwrap();
        match e {
            Event::MouseWheel { delta, .. } => assert_eq!(delta, -120),
            _ => panic!("expected MouseWheel"),
        }
    }

    #[test]
    fn wm_xbuttondown_distinguishes_x1_from_x2_via_wparam_high_word() {
        // wparam HIWORD = 1 → XBUTTON1
        let wparam = 1usize << 16;
        let e = translate(
            hwnd(), msg::WM_XBUTTONDOWN, wparam, 0,
            no_rect(), no_mods(),
        ).unwrap();
        assert!(matches!(e, Event::MouseDown { button: MouseButton::X1, .. }));

        // wparam HIWORD = 2 → XBUTTON2
        let wparam = 2usize << 16;
        let e = translate(
            hwnd(), msg::WM_XBUTTONDOWN, wparam, 0,
            no_rect(), no_mods(),
        ).unwrap();
        assert!(matches!(e, Event::MouseDown { button: MouseButton::X2, .. }));
    }

    #[test]
    fn wm_close_translates_to_close_requested() {
        let e = translate(hwnd(), msg::WM_CLOSE, 0, 0, no_rect(), no_mods()).unwrap();
        assert!(matches!(e, Event::CloseRequested { .. }));
    }

    #[test]
    fn wm_setfocus_and_killfocus_translate_to_focus_events() {
        let e = translate(hwnd(), msg::WM_SETFOCUS, 0, 0, no_rect(), no_mods()).unwrap();
        assert!(matches!(e, Event::FocusIn { .. }));
        let e = translate(hwnd(), msg::WM_KILLFOCUS, 0, 0, no_rect(), no_mods()).unwrap();
        assert!(matches!(e, Event::FocusOut { .. }));
    }

    #[test]
    fn wm_destroy_returns_none_so_routing_layer_handles_it() {
        let r = translate(hwnd(), msg::WM_DESTROY, 0, 0, no_rect(), no_mods());
        assert!(r.is_none(), "WM_DESTROY must be consumed by the routing layer");
    }

    #[test]
    fn unknown_message_falls_through_as_raw() {
        // WM_DEVICECHANGE (0x0219) is one of many we don't model explicitly.
        let e = translate(hwnd(), 0x0219, 42, 7, no_rect(), no_mods()).unwrap();
        match e {
            Event::Raw { message, wparam, lparam, .. } => {
                assert_eq!(message, 0x0219);
                assert_eq!(wparam, 42);
                assert_eq!(lparam, 7);
            }
            _ => panic!("expected Raw"),
        }
    }

    // ---- VK code tests ----

    #[test]
    fn vk_arrow_keys_map_to_directional_variants() {
        assert_eq!(vk_from_raw(vk_codes::VK_LEFT),  VirtualKey::Left);
        assert_eq!(vk_from_raw(vk_codes::VK_RIGHT), VirtualKey::Right);
        assert_eq!(vk_from_raw(vk_codes::VK_UP),    VirtualKey::Up);
        assert_eq!(vk_from_raw(vk_codes::VK_DOWN),  VirtualKey::Down);
    }

    #[test]
    fn vk_function_keys_decode_to_f_with_1_based_index() {
        assert_eq!(vk_from_raw(0x70), VirtualKey::F(1));
        assert_eq!(vk_from_raw(0x77), VirtualKey::F(8));
        assert_eq!(vk_from_raw(0x87), VirtualKey::F(24));
    }

    #[test]
    fn vk_letters_decode_to_char_uppercase() {
        assert_eq!(vk_from_raw(b'A' as u32), VirtualKey::Char(b'A'));
        assert_eq!(vk_from_raw(b'Z' as u32), VirtualKey::Char(b'Z'));
    }

    #[test]
    fn vk_digits_decode_to_char() {
        assert_eq!(vk_from_raw(b'0' as u32), VirtualKey::Char(b'0'));
        assert_eq!(vk_from_raw(b'9' as u32), VirtualKey::Char(b'9'));
    }

    #[test]
    fn vk_unmapped_falls_through_as_other_with_raw_code() {
        // 0xFF (VK_PACKET) is way outside our mapped ranges.
        assert_eq!(vk_from_raw(0xFF), VirtualKey::Other(0xFF));
    }

    #[test]
    fn vk_page_up_and_page_down_use_prior_next_codes() {
        assert_eq!(vk_from_raw(vk_codes::VK_PRIOR), VirtualKey::PageUp);
        assert_eq!(vk_from_raw(vk_codes::VK_NEXT),  VirtualKey::PageDown);
    }

    // ---- Closure adapter ----

    #[test]
    fn closure_handler_invokes_inner_fn_on_handle() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let calls = Rc::new(RefCell::new(0u32));
        let calls2 = calls.clone();
        let mut h = closure_handler(move |_| {
            *calls2.borrow_mut() += 1;
            Action::Consumed
        });
        let act = h.handle(Event::CloseRequested { hwnd: hwnd() });
        assert_eq!(act, Action::Consumed);
        assert_eq!(*calls.borrow(), 1);
    }

    // ---- EventLoop stubs ----

    #[test]
    fn current_returns_same_loop_on_repeat_calls() {
        let a = EventLoop::current() as *const _;
        let b = EventLoop::current() as *const _;
        assert_eq!(a, b);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_on_non_windows_returns_unsupported() {
        let err = EventLoop::current().run().unwrap_err();
        assert!(matches!(err, LoopError::Unsupported));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn pump_once_on_non_windows_returns_unsupported() {
        let err = EventLoop::current().pump_once(10).unwrap_err();
        assert!(matches!(err, LoopError::Unsupported));
    }
}
