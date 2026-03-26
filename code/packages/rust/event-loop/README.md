# event-loop (Rust)

A pluggable, generic event loop — the heartbeat of any interactive application.

## What is this?

An event loop is the outermost structure of any interactive program. It
repeatedly polls registered **sources** for events, dispatches each event to
registered **handlers**, and stops when a handler signals exit.

This is step one on the road to building a text editor from scratch (the
GPUI framework that powers Zed uses `winit`'s event loop, which has the same
shape as this library).

## Usage

```rust
use event_loop::{ControlFlow, EventLoop, EventSource};

#[derive(Debug)]
enum AppEvent { Tick, Quit }

struct TickSource { count: usize }

impl EventSource<AppEvent> for TickSource {
    fn poll(&mut self) -> Vec<AppEvent> {
        self.count += 1;
        if self.count <= 3 { vec![AppEvent::Tick] } else { vec![AppEvent::Quit] }
    }
}

fn main() {
    let mut loop_ = EventLoop::new();
    loop_.add_source(TickSource { count: 0 });
    loop_.on_event(|e: &AppEvent| match e {
        AppEvent::Quit => ControlFlow::Exit,
        AppEvent::Tick => { println!("tick!"); ControlFlow::Continue }
    });
    loop_.run();
}
```

## API

| Item | Description |
|---|---|
| `ControlFlow` | `Continue` or `Exit` — what a handler returns |
| `EventSource<E>` | Trait: `fn poll(&mut self) -> Vec<E>` — must be non-blocking |
| `EventLoop::new()` | Create a new empty loop |
| `loop.add_source(s)` | Register an event source (`S: EventSource<E> + 'static`) |
| `loop.on_event(f)` | Register a handler (`FnMut(&E) -> ControlFlow + 'static`) |
| `loop.run()` | Start the loop (blocks until exit) |
| `loop.stop()` | Signal exit from the same thread |
| `loop.stop_handle()` | Get a `StopHandle` to stop the loop from another thread |

## Design notes

- **Generic over `E`**: the loop never inspects events; you define the type.
- **Pull-based sources**: `poll()` is called by the loop; sources never block.
- **Single-threaded dispatch**: all handlers run on the thread calling `run()`.
- **Thread-safe stop**: `StopHandle` wraps an `Arc<AtomicBool>` — clone freely.
- **CPU-friendly idle**: uses `std::thread::yield_now()` when queue is empty.

## Development

```bash
cargo test -p event-loop
```
