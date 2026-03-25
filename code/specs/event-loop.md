# Event Loop

## Overview

An event loop is the heartbeat of any interactive application. It is a program structure
that waits for things to happen, collects those happenings into a queue, and dispatches
each one to registered handlers — over and over until told to stop.

```
while running:
    collect events from all sources
    for each event in the queue:
        dispatch to handlers
        if any handler says "exit" → stop
```

That is it. Everything else — keyboard input, timers, network packets, GPU frames,
mouse clicks — is just a different kind of event source plugged into this same loop.

### Why study this?

If you are building a text editor, the event loop is the outermost shell. The rendering
pipeline (what GPUI calls "prepaint → paint → present") runs inside one trip through the
loop. Keystrokes, mouse events, and redraws are all events. Understanding the loop first
gives you the skeleton onto which every other part of the editor hangs.

### Why make it generic and pluggable?

A naïve event loop hardcodes what events look like (`KeyPress`, `MouseMove`, etc.) and
what generates them (a windowing system). That coupling makes the loop untestable and
inflexible. A generic loop:

- Is testable in complete isolation — inject a mock source that emits exactly the events
  you want.
- Works at any layer — a game loop, an OS scheduler, a network server, and a GUI all
  share the same shape.
- Teaches the abstraction cleanly before real-world complexity (window handles, OS APIs,
  async runtimes) enters the picture.

---

## Core Concepts

### 1. Event

An event is anything that happened. What "anything" means is up to you — the loop does
not care. In a text editor it might be:

```
enum Event {
    KeyPress(char),
    MouseMove(x, y),
    Resize(width, height),
    Quit,
}
```

In a test it might be:

```
enum Event { Ping, Pong, Done }
```

The loop is generic over `E`, the event type. It never inspects the event — it just
delivers it.

### 2. Event Source

A source is anything that can produce events. It exposes a single non-blocking operation:

```
poll() → List<E>
```

"Non-blocking" is the critical property. `poll()` returns immediately — either with a
list of new events (possibly empty) or with nothing. It never sleeps waiting for
something to happen. Sleeping is the loop's job, not the source's.

Sources can be:

| Source type   | What it does                                           |
|---------------|--------------------------------------------------------|
| Timer         | Returns one event when a deadline has passed           |
| Channel       | Drains a thread-safe queue filled by other threads     |
| Keyboard      | Reads buffered keystrokes from the OS                  |
| Mock          | Returns a fixed list on the first poll, empty after    |
| File watcher  | Checks mtimes and fires on change                      |

### 3. Handler

A handler is a function that receives one event and decides what happens next:

```
handler(event: E) → ControlFlow
```

Multiple handlers can be registered. The loop dispatches each event to handlers in
registration order. Any handler can signal that the loop should exit.

### 4. Control Flow

```
enum ControlFlow {
    Continue,   // keep looping
    Exit,       // stop the loop after this event
}
```

Using an enum (instead of a boolean) is deliberate. It makes the intent explicit at the
call site — `ControlFlow::Exit` is far more readable than `return true` — and leaves room
for future variants (`Pause`, `ScheduleNext(deadline)`, etc.) without breaking existing
handlers.

### 5. The Loop Itself

```
struct EventLoop<E> {
    sources:  List<EventSource<E>>
    handlers: List<fn(E) → ControlFlow>
    stopped:  bool
}
```

Key operations:

| Operation              | Behaviour                                               |
|------------------------|---------------------------------------------------------|
| `add_source(s)`        | Register a new event source                             |
| `on_event(f)`          | Register a handler function                             |
| `run()`                | Block and drive the loop until stopped or handler exits |
| `stop()`               | Signal exit from outside a handler                      |

---

## Loop Algorithm

```
fn run():
    while not stopped:
        # Phase 1 — Collect
        queue = []
        for each source in sources:
            queue.extend(source.poll())

        # Phase 2 — Dispatch
        for each event in queue:
            for each handler in handlers:
                flow = handler(event)
                if flow == Exit:
                    return

        # Phase 3 — Idle
        if queue was empty:
            yield_cpu()   # give other threads a turn; avoid spinning
```

### The three phases visualised

```
┌─────────────────────────────────────────────────────┐
│  Phase 1: Collect                                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │ Source A │  │ Source B │  │ Source C │  ...       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘           │
│       └─────────────┴─────────────┘                 │
│                      │                              │
│                  event queue                        │
│                      │                              │
│  Phase 2: Dispatch   │                              │
│                      ▼                              │
│       ┌──────────────────────────┐                  │
│       │  handler 1(event) → cf   │                  │
│       │  handler 2(event) → cf   │  if any cf=Exit  │
│       │  ...                     │ ─────────────────┤
│       └──────────────────────────┘      return      │
│                                                     │
│  Phase 3: Idle (if queue was empty)                 │
│       yield_cpu() / sleep(0)                        │
└─────────────────────────────────────────────────────┘
                    ↑ repeat ↑
```

---

## Public API

The API is expressed in language-neutral pseudocode. Each implementation adapts idioms
to its language (traits in Rust, interfaces in Go, protocols in Python, etc.).

```
# ── Control flow signal ──────────────────────────────────────────────────────

enum ControlFlow:
    Continue    # loop proceeds normally
    Exit        # loop terminates after this event


# ── Event source abstraction ─────────────────────────────────────────────────

interface EventSource<E>:
    poll() → List<E>
        """
        Return all events currently available. Must return immediately.
        Return an empty list if nothing is ready. Never block.
        """


# ── The loop ─────────────────────────────────────────────────────────────────

class EventLoop<E>:
    new() → EventLoop<E>
        """Create an empty loop with no sources and no handlers."""

    add_source(source: EventSource<E>) → void
        """Register an event source. Sources are polled in registration order."""

    on_event(handler: fn(E) → ControlFlow) → void
        """
        Register an event handler. Handlers receive events in registration order.
        If any handler returns Exit, the loop terminates immediately.
        """

    run() → void
        """
        Start the loop. Blocks until a handler returns Exit or stop() is called.
        Continuously polls all sources, drains the queue, and dispatches events.
        When the queue is empty yields the CPU rather than busy-waiting.
        """

    stop() → void
        """
        Signal the loop to exit on the next iteration. Safe to call from
        outside a handler (e.g., from another thread or a timer callback).
        """
```

---

## Design Decisions

### Why not `async`/`await`?

Async runtimes (Tokio, asyncio, Node's libuv) ARE event loops — sophisticated ones with
I/O reactor integration, timers, and task scheduling. We deliberately build a synchronous
loop first so the concept is visible. Once you understand the synchronous version, the
async version is "this but with the OS doing the blocking for you."

### Why `poll()` instead of `push()`?

A push-based source sends events whenever it wants, requiring thread safety (locks,
channels) in the loop core. A pull-based source is called by the loop on its schedule —
simpler, safer, and sufficient for single-threaded use. Multi-threaded sources can still
exist by buffering into a channel and exposing a `poll()` that drains it.

### Why `run()` blocks?

A text editor's main thread has nothing useful to do except process events. Blocking is
correct behaviour. Non-blocking variants (e.g., `run_until_empty()`) are easy to add
on top.

### Why yield when idle?

Busy-spinning (`while true { poll(); }`) consumes 100% CPU even when nothing is
happening. Yielding (calling `thread.yield()`, `runtime.Gosched()`, or sleeping 1ms)
gives other threads CPU time and reduces heat/battery drain on laptops — important when
the editor is open but the user is not typing.

---

## Connection to GPUI

GPUI uses `winit` as its platform event loop. Here is how the concepts map:

| This spec            | GPUI / winit equivalent                          |
|----------------------|--------------------------------------------------|
| `EventSource<E>`     | OS event queue (keyboard, mouse, resize, etc.)   |
| `event.poll()`       | `EventLoopProxy`, `winit::event_loop::EventLoop` |
| `handler(event)`     | `event_loop.run(|event, target| { ... })`        |
| `ControlFlow::Exit`  | `target.exit()`                                  |
| yield when idle      | `AboutToWait` → `window.request_redraw()`        |
| `stop()`             | `EventLoopProxy::send_event()`                   |

GPUI adds a rendering phase inside the loop iteration (prepaint → paint → present) that
runs after events are dispatched. That is the only addition — the loop shape is identical.

---

## Thread Safety Note

This implementation is **single-threaded by design**. The loop, its sources, and its
handlers all live on one thread. This is intentional for the learning version.

Multi-threaded event injection (e.g., a background file watcher pushing events) is
handled by wrapping a `Mutex<VecDeque<E>>` in a `ChannelSource` that `poll()` drains.
The loop never needs to know.

Full async integration (epoll, kqueue, IOCP) is a natural next step once this foundation
is solid.
