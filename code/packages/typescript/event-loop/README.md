# event-loop (TypeScript)

A pluggable, generic event loop — the heartbeat of any interactive application.

## What is this?

An event loop is the outermost structure of any interactive program. It
repeatedly polls registered **sources** for events, dispatches each event to
registered **handlers**, and stops when a handler signals exit.

This is step one on the road to building a text editor from scratch (the
GPUI framework that powers Zed uses `winit`'s event loop, which has the same
shape as this library).

## Usage

```typescript
import { ControlFlow, EventLoop, EventSource } from "coding-adventures-event-loop";

type AppEvent = "tick" | "quit";

class TickSource implements EventSource<AppEvent> {
  private count = 0;
  poll(): AppEvent[] {
    this.count++;
    return this.count <= 3 ? ["tick"] : ["quit"];
  }
}

const loop = new EventLoop<AppEvent>();
loop.addSource(new TickSource());
loop.onEvent((e) => {
  if (e === "quit") return ControlFlow.Exit;
  console.log("tick!");
  return ControlFlow.Continue;
});
loop.run();
```

## API

| Item | Description |
|---|---|
| `ControlFlow` | String enum: `Continue` or `Exit` — what a handler returns |
| `EventSource<E>` | Interface: `poll(): E[]` — must be non-blocking |
| `new EventLoop<E>()` | Create a new empty loop |
| `loop.addSource(s)` | Register an event source |
| `loop.onEvent(f)` | Register a handler `(event: E) => ControlFlow` |
| `loop.run()` | Start the loop (blocks until exit) |
| `loop.stop()` | Signal exit from within a handler |
| `VERSION` | Package version string |

## Design notes

- **Generic over `E`**: TypeScript generics; the loop never inspects events.
- **Interface-based sources**: structural typing — any object with `poll()` works.
- **Pull-based**: `poll()` is called by the loop; sources must never block.
- **Single-threaded**: JavaScript is single-threaded; `stop()` sets a flag checked each iteration.
- **String enum**: `ControlFlow` uses string values for readable debugging in logs.

## Development

```bash
cd code/packages/typescript/event-loop
npm install
npm test
```
