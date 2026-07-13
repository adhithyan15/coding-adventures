# clock (C++)

The heartbeat of every digital circuit — a clock simulator in pure ISO C++17,
header-only, in namespace `ca::clk`. A faithful port of the Rust `clock` crate.

- **`Clock`** — a square-wave generator (`tick` / `full_cycle` / `run`), with
  edge listeners and a cycle/tick count.
- **`ClockDivider`** — derives a slower clock (`source / divisor`).
- **`MultiPhaseClock`** — rotates a single active phase across N outputs.

## API

```cpp
#include "clock.hpp"
namespace clk = ca::clk;

clk::Clock c(1000000000);                 // 1 GHz
clk::ClockEdge e = c.tick();              // rising edge, cycle 1
std::vector<clk::ClockEdge> edges = c.run(5);   // 10 edges

std::uint32_t count = 0;
c.register_listener([&count](const clk::ClockEdge& e) {
    if (e.is_rising) ++count;             // std::function ~ Box<dyn FnMut>
});
```

Listeners are `std::function<void(const ClockEdge&)>` — the direct analog of
Rust's `Box<dyn FnMut>`. Where Rust `new` panics on an invalid argument, the
constructors throw `std::invalid_argument`; `MultiPhaseClock::get_phase` throws
`std::out_of_range` for an out-of-range index.

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
