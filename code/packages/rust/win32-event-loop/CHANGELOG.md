# Changelog — win32-event-loop

## [0.1.0] — Unreleased

### Added — initial crate

- Public API: `Event`, `Action`, `EventHandler`, `closure_handler`,
  `Registration`, `EventLoop`, `LoopError`, `Hwnd`, `Modifiers`, `Rect`,
  `Size`, `VirtualKey`, `MouseButton`. All types mirror the spec in
  `code/specs/win32-event-loop.md` §4.
- Pure `translate(hwnd, message, wparam, lparam, paint_rect, modifiers)
  -> Option<Event>` function covering every row of the spec's §5.3
  translation table. Returns `None` only for messages consumed
  internally by the routing layer (today: `WM_DESTROY`).
- Windows-only runtime (`sys.rs`) with the shared `wndproc`, the
  thread-local routing table, blocking `run()`, non-blocking
  `pump_once(max)`, and `request_quit()`. The WNDPROC wraps `WM_PAINT`
  with `BeginPaint`/`EndPaint`, captures modifier-key state via
  `GetKeyState`, auto-unregisters on `WM_DESTROY`, and posts `WM_QUIT(0)`
  when the routing table becomes empty.
- Cross-platform stub: on non-Windows targets `run` and `pump_once`
  return `LoopError::Unsupported` so dependents stay buildable.
- 25 unit tests cover the §5.3 translation table, VK-code mapping, the
  closure-handler adapter, and the non-Windows stub returns. Tests are
  pure-Rust and run on every platform.
- `windows` crate dependency: 0.58 with `Win32_Foundation`,
  `Win32_Graphics_Gdi`, `Win32_UI_WindowsAndMessaging`,
  `Win32_System`, `Win32_System_LibraryLoader`, and
  `Win32_System_Threading` features.

### Known limitations (deferred to follow-up PRs)

- No touch / pen input (`WM_POINTER*`). Adds a `Event::Touch` variant.
- No IME composition events (`WM_IME_*`). `WM_CHAR` after
  `TranslateMessage` is enough for plain ASCII / Unicode text input;
  full IME composition needs its own design.
- No `Event::DpiChanged` for per-monitor DPI tracking. Consumers
  currently work in physical pixels.
- No async / `tokio` adapter. `pump_once` is the synchronous building
  block; an async wrapper can live in a separate crate.
