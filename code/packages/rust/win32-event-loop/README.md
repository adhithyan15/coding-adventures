# win32-event-loop

A decoupled Win32 message pump. Owns `GetMessage` / `DispatchMessage` and
an `HWND` → handler routing table. Knows nothing about Mosaic, Direct2D,
window creation, or any specific renderer.

See `code/specs/win32-event-loop.md` for the design.

## Why this crate

`window-win32` today combines window creation, paint dispatch, and the
message loop into one type. Applications that want to:

- Run multiple top-level windows on one thread,
- Drive their own paint pipeline (e.g. `paint-vm-direct2d`),
- Mix Mosaic-emitted components with hand-written Win32 controls,
- Pump events from a non-blocking context (game loop, integration test),

need the *event loop* alone, decoupled from window creation. This crate
provides exactly that.

## Public API at a glance

```rust
use win32_event_loop::{closure_handler, Action, Event, EventLoop, Hwnd};

// 1. Create your HWND yourself (via the `windows` crate or window-win32).
let hwnd: Hwnd = create_my_window();

// 2. Register a handler. The Registration is RAII — drop it to unregister.
let _registration = EventLoop::current().register(hwnd, closure_handler(|ev| {
    match ev {
        Event::Paint { rect, .. } => {
            // call your own paint code (e.g. paint_vm_direct2d::render)
            Action::Consumed
        }
        Event::CloseRequested { .. } => {
            EventLoop::current().request_quit();
            Action::Default  // let DefWindowProc destroy the window
        }
        _ => Action::Default,
    }
}));

// 3. Block until WM_QUIT.
let exit_code = EventLoop::current().run()?;
```

Non-blocking integration:

```rust
loop {
    // Drain up to 32 waiting messages per frame, then yield to the game loop.
    let _ = EventLoop::current().pump_once(32)?;
    render_frame();
}
```

## What it translates

| Win32 message                  | Translates to            |
|--------------------------------|--------------------------|
| `WM_PAINT`                     | `Event::Paint`           |
| `WM_SIZE`                      | `Event::Resize`          |
| `WM_CHAR`                      | `Event::Char`            |
| `WM_KEYDOWN` / `WM_SYSKEYDOWN` | `Event::KeyDown`         |
| `WM_KEYUP` / `WM_SYSKEYUP`     | `Event::KeyUp`           |
| `WM_LBUTTONDOWN` (+ M/R/X)     | `Event::MouseDown`       |
| `WM_LBUTTONUP`   (+ M/R/X)     | `Event::MouseUp`         |
| `WM_MOUSEMOVE`                 | `Event::MouseMove`       |
| `WM_MOUSEWHEEL`                | `Event::MouseWheel`      |
| `WM_CLOSE`                     | `Event::CloseRequested`  |
| `WM_SETFOCUS` / `WM_KILLFOCUS` | `Event::FocusIn` / `Out` |
| `WM_DESTROY`                   | (internal — auto-unregister) |
| anything else                  | `Event::Raw`             |

## Cross-platform builds

The crate compiles on every platform. On non-Windows targets, `run` and
`pump_once` return `LoopError::Unsupported` rather than failing the
build — this lets cross-platform dependents (e.g. `mosaic-emit-win32`'s
generated code) `cargo check` on Linux/macOS CI without
`#[cfg(target_os = "windows")]` everywhere.

The pure-translation function `translate()` and its companion test suite
also run cross-platform.

## Stability

`Event`, `Action`, `EventHandler`, `Registration`, `EventLoop`, and
`LoopError` form the public contract. Semver applies — breaking changes
bump major.
