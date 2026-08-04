# clock (C)

The heartbeat of every digital circuit — a clock simulator in pure ISO C17. A
faithful port of the Rust `clock` crate.

Every sequential circuit (flip-flops, registers, CPU pipeline stages) is driven
by a clock: a square wave alternating 0/1, on whose rising edges synchronous
logic captures its inputs. This crate simulates that heartbeat:

- **`Clock`** — a square-wave generator (`tick` / `full_cycle` / `run`), with
  edge listeners and a cycle/tick count.
- **`ClockDivider`** — derives a slower clock (`source / divisor`).
- **`MultiPhaseClock`** — rotates a single active phase across N outputs.

## API

```c
#include "clock.h"

Clock *clk = clock_new(1000000000);       /* 1 GHz */
ClockEdge e = clock_tick(clk);            /* rising edge, cycle 1 */
size_t n; ClockEdge *edges = clock_run(clk, 5, &n);   /* 10 edges */
free(edges);

/* A listener sees every edge (the C analog of a captured closure). */
clock_register_listener(clk, my_cb, &my_state);
clock_free(clk);
```

Listeners are C callbacks (`void (*)(const ClockEdge*, void *userdata)`) — the
port's analog of Rust's `Box<dyn FnMut>`. Where Rust `new` panics on an invalid
argument (frequency 0, divisor < 2, phases < 2), the constructor returns NULL;
`mpc_get_phase` returns 0 for an out-of-range index. `clock_run` guards its
`2·cycles` allocation against `size_t` overflow.

## Portability

Pure ISO C17 — no extensions. Compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
