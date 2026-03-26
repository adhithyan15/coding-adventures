# event-loop (Python)

A pluggable, generic event loop — the heartbeat of any interactive application.

## What is this?

An event loop is the outermost structure of any interactive program. It
repeatedly polls registered **sources** for events, dispatches each event to
registered **handlers**, and stops when a handler signals exit.

This is step one on the road to building a text editor from scratch (the
GPUI framework that powers Zed uses `winit`'s event loop, which has the same
shape as this library).

## Usage

```python
from event_loop import ControlFlow, EventLoop, EventSource
from typing import List

class TickSource:
    def __init__(self):
        self.count = 0

    def poll(self) -> List[str]:
        self.count += 1
        if self.count <= 3:
            return ["tick"]
        return ["quit"]

loop: EventLoop[str] = EventLoop()
loop.add_source(TickSource())
loop.on_event(lambda e: ControlFlow.EXIT if e == "quit" else (print("tick!") or ControlFlow.CONTINUE))
loop.run()
```

## API

| Item | Description |
|---|---|
| `ControlFlow` | `CONTINUE` or `EXIT` — what a handler returns |
| `EventSource[E]` | Protocol: `def poll(self) -> List[E]` — must be non-blocking |
| `EventLoop[E]` | Generic loop class |
| `loop.add_source(s)` | Register an event source |
| `loop.on_event(f)` | Register a handler `Callable[[E], ControlFlow]` |
| `loop.run()` | Start the loop (blocks until exit) |
| `loop.stop()` | Signal exit from another thread |
| `VERSION` | Package version string |

## Design notes

- **Generic over `E`**: uses `typing.Generic[E]`; the loop never inspects events.
- **Protocol-based sources**: `EventSource` is a `@runtime_checkable Protocol` — no inheritance required.
- **Pull-based**: `poll()` is called by the loop; sources must never block.
- **Thread-safe stop**: `stop()` sets a threading `Event`; safe to call from any thread.
- **CPU-friendly idle**: `time.sleep(0)` yields the GIL when the queue is empty.

## Development

```bash
cd code/packages/python/event-loop
pip install -e ".[dev]"
pytest --cov=event_loop --cov-report=term-missing
```
