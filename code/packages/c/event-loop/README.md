# event-loop (C)

**CCPP02 port campaign — bucket B, thread slice, port #1.** A pluggable, generic
event loop: the outermost structure of any interactive program, which runs until
told to stop, repeatedly collecting events from pluggable sources and dispatching
them to handlers. The C port of the Rust `event-loop` crate, and the first proof
that a deferred *thread*-using crate runs on os-platform's concurrency primitives
with no per-OS code of its own.

```text
while running:
    collect events from every source      (Phase 1 — sources must not block)
    for each event, for each handler:      (Phase 2 — dispatch in order)
        if a handler returns EVL_EXIT → stop
    if nothing arrived → nap briefly        (Phase 3 — don't spin a core at 100%)
```

## API (`event_loop/event_loop.h`)

```c
#include "event_loop/event_loop.h"

/* A source: fill `out` with up to `cap` event pointers, return how many. */
static size_t ticks(void *ctx, const void **out, size_t cap) {
    struct my_state *s = ctx;
    if (cap == 0 || s->done) return 0;
    out[0] = &s->tick;            /* events are borrowed — the loop never frees */
    return 1;
}

/* A handler: inspect one event, decide whether to keep looping. */
static evl_control on_tick(const void *event, void *user) {
    return (*(const int *)event == QUIT) ? EVL_EXIT : EVL_CONTINUE;
}

event_loop *lp;
event_loop_create(&lp);
event_loop_add_source(lp, ticks, &state);
event_loop_on_event(lp, on_tick, &state);
event_loop_run(lp);              /* blocks until EVL_EXIT or a stop */
event_loop_destroy(lp);
```

| Function | Purpose |
|----------|---------|
| `event_loop_create(&lp)` | new empty loop |
| `event_loop_add_source(lp, poll, ctx)` | register a pull-based event source |
| `event_loop_on_event(lp, fn, user)` | register a handler |
| `event_loop_stop_handle(lp)` | a thread-safe handle to stop the loop from outside |
| `event_loop_stop(lp)` | ask the loop to exit on its next iteration |
| `evl_stop_signal(h)` | stop via a handle (from another thread / a callback) |
| `event_loop_run(lp)` | run until `EVL_EXIT` or a stop; blocks |
| `event_loop_destroy(lp)` | free the loop |

**Generics → C.** Rust's `EventLoop<E>` is generic over the event type; C has no
generics, so an event is an opaque `const void *` and the caller defines what it
points at — the same `void *` + function-pointer idiom `tcp_runtime`'s handler
uses. A source is a poll function plus its state; a handler is a function plus its
captured `user` state.

## The thread story (why this is a *thread*-bucket port)

`run()` is single-threaded — sources and handlers all run on the calling thread.
The concurrency is the **stop signal**: `event_loop_stop_handle` returns a handle
you can hand to another thread (or a timer callback) and call `evl_stop_signal`
on. That handle wraps a **`osp_mutex`-guarded flag**, so the worker's write and
the loop's per-iteration read never race. The idle nap uses **`osp_sleep_ns`**.
The test spawns a real worker thread (`osp_thread_spawn`) that stops the loop from
outside — the end-to-end proof.

**Lifetime.** A stop handle points at the loop's internal flag, so it is valid
only while the loop is alive; quiesce every thread holding one before
`event_loop_destroy`, as with any shared object.

## Build & test

`tests/event_loop_test.c` mirrors the Rust crate's tests (all events delivered,
`EVL_EXIT` stops dispatch immediately, multiple handlers, multiple sources merged,
stop-from-within) and adds the cross-thread stop via `osp_thread_spawn`.

```sh
cd code/packages/c/event-loop
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 66 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks.

## Layout

```
event-loop/
├── include/event_loop/event_loop.h   # public API (reuses os_platform/status.h)
├── src/event_loop.c                   # the loop — one OS-agnostic file
├── tests/event_loop_test.c            # tests, incl. a cross-thread stop
├── tools/run.sh  · run.ps1            # build with os-platform thread + clock
├── BUILD  · BUILD_windows             # per-OS build drivers
└── required_capabilities.json         # CI needs gcc, clang, cl
```

The loop composes os-platform's `thread` backend (`osp_mutex`) and `clock`
backend (`osp_sleep_ns`), so the build compiles those backends and links the OS
thread library (`-pthread` on POSIX; the CRT on Windows). No changes to
os-platform itself — this is a pure consumer.
