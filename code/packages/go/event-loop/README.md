# event-loop (Go)

A pluggable, generic event loop — the heartbeat of any interactive application.

## What is this?

An event loop is the outermost structure of any interactive program. It
repeatedly polls registered **sources** for events, dispatches each event to
registered **handlers**, and stops when a handler signals exit.

This is step one on the road to building a text editor from scratch. The
rendering pipeline, keyboard handling, and redraws all live *inside* one trip
through this loop.

## Layer position

```
text editor
    └── event loop      ← you are here
            ├── sources (keyboard, timer, network, …)
            └── handlers (render, update state, …)
```

## Usage

```go
package main

import (
    "fmt"
    eventloop "github.com/adhithyan15/coding-adventures/code/packages/go/event-loop"
)

type AppEvent int

const (
    Tick AppEvent = iota
    Quit
)

type TickSource struct{ n int }

func (s *TickSource) Poll() []AppEvent {
    s.n++
    if s.n <= 3 {
        return []AppEvent{Tick}
    }
    return []AppEvent{Quit}
}

func main() {
    loop := eventloop.New[AppEvent]()
    loop.AddSource(&TickSource{})
    loop.OnEvent(func(e AppEvent) eventloop.ControlFlow {
        if e == Quit {
            return eventloop.Exit
        }
        fmt.Println("tick!")
        return eventloop.Continue
    })
    loop.Run()
}
```

## API

| Type / function | Description |
|---|---|
| `ControlFlow` | `Continue` or `Exit` — what a handler returns |
| `EventSource[E]` | Interface: `Poll() []E` — must be non-blocking |
| `New[E]()` | Create a new empty loop |
| `loop.AddSource(s)` | Register an event source |
| `loop.OnEvent(f)` | Register a handler function |
| `loop.Run()` | Start the loop (blocks until exit) |
| `loop.Stop()` | Signal exit from outside a handler |

## Design notes

- **Generic over `E`**: the loop never inspects events; you define the type.
- **Pull-based sources**: `Poll()` is called by the loop on its schedule. Sources never block.
- **Single-threaded**: all sources and handlers run on the calling goroutine.
- **CPU-friendly idle**: uses `runtime.Gosched()` when the queue is empty.

## Development

```bash
go test ./... -v -cover
```

Coverage: 100%
