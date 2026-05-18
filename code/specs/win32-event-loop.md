# win32-event-loop — decoupled Win32 message pump

**Status:** Specification (draft)
**Layer:** Platform plumbing (sits below any Win32-targeting renderer)
**Depends on:** the `windows` crate (Win32 bindings)
**Consumed by:** `mosaic-emit-win32` (UI), future Win32-targeted spreadsheet hosts, any
crate that needs a Win32 message pump without owning window creation

---

## 1. Purpose

The classic Win32 application is built around `GetMessage` / `TranslateMessage` /
`DispatchMessage`. Today the only Mosaic-adjacent crate that runs this pump is
[`window-win32`](../packages/rust/window-win32/), which couples four concerns into
one type:

1. Registering a `WNDCLASS`
2. Creating an `HWND`
3. Running the message loop
4. Routing `WM_PAINT` to a single fixed paint callback

That coupling is fine for a one-window helper but fights any application that
wants to:

- Run multiple top-level windows on one thread (the standard Win32 model — every
  HWND on a thread shares a single message queue).
- Drive its own paint pipeline (e.g. a `paint-vm-direct2d` callback per HWND, or
  a per-component `render(Scene)` function the Mosaic Win32 emitter generates).
- Mix Mosaic-emitted components with non-Mosaic Win32 controls (a host that
  embeds, say, a `mosaic-emit-win32`-generated Grid alongside a hand-written
  ribbon).
- Pump events from a non-blocking context (a unit test, a Tokio runtime, a
  game-loop-style frame scheduler that wants to interleave Win32 events with
  other work).

This spec carves the *event loop itself* into a small, single-purpose crate
that knows nothing about Mosaic, Direct2D, or how individual windows are
created. The Win32 Mosaic backend (UI: `mosaic-emit-win32`) is the first
consumer; future Windows runtimes plug in the same way.

## 2. Non-goals

- **Window creation.** This crate never calls `CreateWindowExW` or registers a
  `WNDCLASS`. Callers create windows themselves (directly via the `windows`
  crate, or indirectly via `window-win32`) and then *register* the resulting
  `HWND` with this crate so it knows where to route messages.
- **Rendering.** This crate does not paint. It exposes a `Paint` event that
  carries the bounds to invalidate; the consumer is expected to call its own
  drawing code (e.g. `paint_vm_direct2d::render(scene, hwnd, rect)`).
- **Cross-platform abstraction.** That belongs in `window-core` /
  `window-win32`. This crate is *pure Win32* — it imports `GetMessageW`,
  `DispatchMessageW`, and the standard message constants directly.
- **Threading model overhauls.** Win32 message queues are per-thread; this
  crate respects that. One `EventLoop` per thread is the only supported model.
  Cross-thread coordination is out of scope (consumers wanting it can post
  thread messages with `PostThreadMessage` themselves).

## 3. Architecture

```
+----------------------+      +----------------------+
|  Mosaic Win32        |      |  Hand-written Win32  |
|  emitter output      |      |  host code           |
|  (mosaic-emit-win32) |      |  (anything else)     |
+----------+-----------+      +-----------+----------+
           |                              |
           |   register(hwnd, handler)    |
           +--------------+---------------+
                          |
                          v
                +---------+----------+
                |   win32-event-loop |
                |  (this crate)      |
                |                    |
                |  - HWND → handler  |
                |    table           |
                |  - GetMessage pump |
                |  - typed Event     |
                |    enum            |
                +---------+----------+
                          |
                          | GetMessageW/DispatchMessageW
                          v
                +---------+----------+
                |    User32/Win32    |
                +--------------------+
```

The crate is structurally trivial — a `WNDPROC`, a global thread-local HWND→
handler table, a message-translation function, and a blocking `run()` /
non-blocking `pump_once()` pair. It is deliberately small (target: under
1500 LOC including tests).

## 4. Public API

```rust
//! win32-event-loop — single-threaded Win32 message pump.

use windows::Win32::Foundation::HWND;

/// A single typed event delivered to a registered HWND's handler.
///
/// The variants are a minimal common subset every Win32 consumer cares about;
/// the raw message is also exposed for consumers that need to look at
/// vendor-specific or device-specific WMs (WM_DEVICECHANGE, custom WM_USER+N,
/// etc.).
#[derive(Debug, Clone)]
pub enum Event {
    /// Window was asked to repaint a region. The handler must perform the
    /// paint synchronously; the loop calls BeginPaint/EndPaint around the
    /// dispatch so the consumer only sees the invalidate rect.
    Paint { hwnd: HWND, rect: Rect },

    /// Window resized. `client` is the new client-area size in physical pixels.
    Resize { hwnd: HWND, client: Size },

    /// A character was typed (translated WM_CHAR — Unicode codepoint after IME).
    Char { hwnd: HWND, codepoint: u32, modifiers: Modifiers },

    /// A key was pressed (raw virtual-key code, not yet translated).
    KeyDown { hwnd: HWND, vk: VirtualKey, modifiers: Modifiers, repeat: bool },
    KeyUp   { hwnd: HWND, vk: VirtualKey, modifiers: Modifiers },

    /// Mouse button events. `button` is Left/Middle/Right/X1/X2.
    MouseDown { hwnd: HWND, button: MouseButton, x: i32, y: i32, modifiers: Modifiers },
    MouseUp   { hwnd: HWND, button: MouseButton, x: i32, y: i32, modifiers: Modifiers },
    MouseMove { hwnd: HWND, x: i32, y: i32, modifiers: Modifiers },
    MouseWheel { hwnd: HWND, x: i32, y: i32, delta: i32, modifiers: Modifiers },

    /// Window close was requested (the X button, Alt+F4, system menu).
    /// Returning Action::Default still closes the window; return
    /// Action::Consumed to veto.
    CloseRequested { hwnd: HWND },

    /// Focus gained / lost.
    FocusIn  { hwnd: HWND },
    FocusOut { hwnd: HWND },

    /// Escape hatch: an unrecognised message. The handler may return
    /// Action::Default to let DefWindowProc handle it, or Action::Consumed
    /// to return zero without calling DefWindowProc.
    Raw { hwnd: HWND, message: u32, wparam: usize, lparam: isize },
}

#[derive(Debug, Clone, Copy)]
pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

#[derive(Debug, Clone, Copy)]
pub struct Size { pub width: u32, pub height: u32 }

/// Modifier-key bitset captured at the moment the event was dispatched.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl:  bool,
    pub alt:   bool,
    pub win:   bool,
}

/// Subset of the virtual-key namespace this loop translates. Variants beyond
/// the named ones fall through as `Raw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualKey {
    Enter, Escape, Tab, Backspace, Delete,
    Left, Right, Up, Down,
    Home, End, PageUp, PageDown,
    F(u8),                       // F1..F24
    Char(u8),                    // A..Z, 0..9 (uppercase ASCII)
    Other(u32),                  // raw VK code for anything else
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton { Left, Middle, Right, X1, X2 }

/// What the handler did with the event.
///
/// - `Consumed` — the handler fully handled the message; the loop returns
///   zero and does NOT call `DefWindowProc`.
/// - `Default`  — the handler did nothing or wants the OS default behaviour;
///   the loop calls `DefWindowProc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action { Consumed, Default }

/// Object-safe handler trait. Consumers either implement this directly or
/// use the `closure_handler` adapter for simple lambdas.
pub trait EventHandler: 'static {
    fn handle(&mut self, event: Event) -> Action;
}

/// Adapt a `FnMut(Event) -> Action` into an `EventHandler`.
pub fn closure_handler<F: FnMut(Event) -> Action + 'static>(f: F) -> impl EventHandler;

/// The thread-local event loop. Construct once per thread; passing it across
/// threads is a compile error (`!Send`).
pub struct EventLoop { /* private */ }

impl EventLoop {
    /// Get the current thread's event loop, creating it if needed. Returns
    /// the same loop on every call within one thread.
    pub fn current() -> &'static EventLoop;

    /// Register a handler for messages destined to `hwnd`.
    ///
    /// Panics if `hwnd` is already registered on this loop. Returns a
    /// `Registration` whose `Drop` impl unregisters automatically.
    pub fn register(&self, hwnd: HWND, handler: impl EventHandler) -> Registration;

    /// Block forever, draining the message queue, until `PostQuitMessage`
    /// is called (e.g. via `request_quit` or a handler that decides it's done).
    /// Returns the WPARAM passed to WM_QUIT (conventionally 0 on clean exit).
    pub fn run(&self) -> i32;

    /// Drain at most `max` messages, returning the number actually processed.
    /// Useful for integrating with non-Win32 schedulers — call this every
    /// frame instead of `run()`. Returns 0 when the queue is empty AND
    /// nothing was waiting.
    pub fn pump_once(&self, max: usize) -> usize;

    /// Post WM_QUIT(0) to this thread, causing any `run()` in flight to
    /// return after the current message finishes dispatching.
    pub fn request_quit(&self);
}

/// RAII handle. Dropping unregisters the HWND from the loop's routing table.
/// The HWND itself is NOT destroyed — that remains the caller's job.
pub struct Registration { /* private */ }
```

## 5. Internal design notes

### 5.1 Routing table

A `Mutex<HashMap<HWND, Box<dyn EventHandler>>>` lives behind a `OnceCell` keyed
per thread. The crate's `extern "system"` window procedure is shared by all
WNDCLASSes that opt into routing (callers pass it when they register their
class); on each message it:

1. Looks up `hwnd` in the table.
2. If found, translates the message into an `Event` and calls `handle()`.
3. If the handler returns `Action::Consumed`, returns 0; otherwise calls
   `DefWindowProcW`.
4. If no handler is registered, just calls `DefWindowProcW`.

### 5.2 The shared WNDPROC

The crate exposes the `extern "system" fn wndproc(...)` symbol so consumers
can pass it to `WNDCLASS::lpfnWndProc` when registering their classes.
Callers that want to use their own WNDPROC must instead implement message
forwarding themselves; see §6.

### 5.3 Message translation

The crate handles a fixed list of `WM_*` constants and translates them into
`Event`. Everything else falls through as `Event::Raw`. Translation rules:

| Win32 message            | Translates to                          | Notes |
|--------------------------|----------------------------------------|-------|
| `WM_PAINT`               | `Event::Paint`                         | Wrapped in `BeginPaint`/`EndPaint`; rect = `PAINTSTRUCT::rcPaint`. |
| `WM_SIZE`                | `Event::Resize`                        | `client = LOWORD(lParam) × HIWORD(lParam)`. |
| `WM_CHAR`                | `Event::Char`                          | After `TranslateMessage`. |
| `WM_KEYDOWN`/`WM_SYSKEYDOWN` | `Event::KeyDown`                   | `repeat = bit 30 of lParam`. |
| `WM_KEYUP`/`WM_SYSKEYUP` | `Event::KeyUp`                         | |
| `WM_LBUTTONDOWN`/...     | `Event::MouseDown`                     | Button derived from message id. |
| `WM_LBUTTONUP`/...       | `Event::MouseUp`                       | |
| `WM_MOUSEMOVE`           | `Event::MouseMove`                     | |
| `WM_MOUSEWHEEL`          | `Event::MouseWheel`                    | `delta = HIWORD(wParam) as i16 as i32`. |
| `WM_CLOSE`               | `Event::CloseRequested`                | `Action::Consumed` vetoes; `Action::Default` lets DefWindowProc destroy. |
| `WM_SETFOCUS`            | `Event::FocusIn`                       | |
| `WM_KILLFOCUS`           | `Event::FocusOut`                      | |
| `WM_DESTROY`             | (internal) auto-unregisters the HWND.  | Then forwards `PostQuitMessage(0)` if it was the last registered window. |
| anything else            | `Event::Raw`                           | |

### 5.4 Thread-safety

`EventLoop` is `!Send + !Sync`. The routing table lives in a thread-local
`OnceLock`; cross-thread access is a compile error. Cross-thread messaging
between two `EventLoop` instances is done via `PostThreadMessageW` — the
crate exposes no helper for that today (out of scope).

## 6. Interop with `window-win32`

`window-win32` today both creates HWNDs and runs its own message loop. This
crate is designed to coexist:

- Callers that want the `window-win32` HWND creation but the loop here:
  construct a `Win32Window` with a paint callback that *does nothing*, then
  `EventLoop::current().register(window.hwnd(), ...)`, then `loop.run()`
  instead of `window.run()`. The shared WNDPROC is opt-in: if `window-win32`
  registered its own class, the messages do not flow through this loop's
  routing table.

- A follow-up PR can refactor `window-win32` to use this crate internally and
  expose only window creation. That is *not* in scope for this spec — it's a
  migration that needs its own review.

## 7. Errors and panics

The crate avoids returning `Result` from the high-traffic path
(`pump_once`/`run`). Panics signal programmer errors only:

- Calling `register(hwnd, _)` twice with the same HWND.
- Calling `register` from a thread different from the one that owns the HWND
  (verified via `GetWindowThreadProcessId`).
- Calling `run`/`pump_once` from inside a handler (re-entrancy).

Recoverable conditions return a `LoopError`:

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LoopError {
    #[error("GetMessage returned -1 (Win32 error {0:#x})")]
    GetMessageFailed(u32),
}
```

`run` returns `Result<i32, LoopError>`; `pump_once` returns
`Result<usize, LoopError>`. (The signatures above in §4 are simplified; the
real API returns Result.)

## 8. Test plan

The crate is hard to unit-test in a vacuum because it depends on the real
Win32 message queue. Test approach:

1. **Pure-translation tests** (run on any OS via `#[cfg(test)]` shims):
   parameterise the translation table — `fn translate(message: u32, wparam,
   lparam) -> Option<Event>` — and unit-test every row of §5.3 without
   touching Win32. ≥30 tests.
2. **Windows-only integration tests** (`#[cfg(all(test, target_os = "windows"))]`):
   - Create a hidden message-only HWND (`HWND_MESSAGE`), register a handler,
     `PostMessage` a synthetic `WM_CHAR`, then `pump_once`. Assert handler saw
     the right `Event::Char`.
   - Two HWNDs, one handler each, two `PostMessage`s — each handler sees
     only its own message.
   - Handler returns `Action::Consumed` — verify `DefWindowProc` is NOT
     called (use a custom WM_USER message that has observable side effects
     in DefWindowProc).
   - `Registration::drop` removes the HWND from the routing table — verify
     by dropping and posting another message, then asserting the handler was
     not invoked.
   - `request_quit` makes `run` return.

3. **Crash-safety tests**: a handler that panics must not corrupt the
   routing table; the panic propagates out of `run`/`pump_once` and the HWND
   stays registered for the next pump call.

Target coverage: 95%+ for the pure-translation paths, 80%+ for the
Windows-only integration paths.

## 9. Crate boilerplate

- Cargo.toml — depends on `windows = { version = "0.58", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_Graphics_Gdi"] }`, `thiserror`, and dev-dep `serial_test` (event-loop integration tests are inherently serial per thread).
- README.md — usage example: create HWND with `windows` crate, register a closure handler, call `run()`.
- CHANGELOG.md — initial 0.1.0 entry on landing.
- BUILD — standard Rust BUILD pattern with the workspace deps.
- The crate is **Windows-only at the implementation level** but compiles to
  a no-op stub on other targets so dependents can be `cargo build`ed
  cross-platform (matches the `#[cfg(target_os = "windows")]` pattern in
  `window-win32`).

## 10. Out of scope (tracked as follow-ups)

- Touch / pen input (`WM_POINTER*`). Add later as new `Event::Touch` variants.
- IME composition events (`WM_IME_*`). Today `WM_CHAR` after `TranslateMessage`
  is enough for plain text input; full IME composition needs its own design.
- High-DPI / per-monitor DPI awareness. Captured in a follow-up — the loop
  reports physical pixels and leaves DPI handling to consumers.
- A `tokio`/`async` adapter. The synchronous `pump_once(max)` is the building
  block; an async wrapper can live in a separate crate without depending on
  this one.
