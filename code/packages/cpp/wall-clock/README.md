# wall-clock (C++)

An injectable source of "now" in pure ISO C++17, header-only, in namespace
`ca::wallclock`. A faithful port of the pure (no-`std::time`) core of the Rust
`wall-clock` crate.

- **`Instant`** — a point in time as f64 seconds since the Unix epoch.
- **`Clock`** — the abstract clock (virtual `now()`), the direct analog of
  Rust's `dyn Clock` trait object.
- **`FixedClock`** — always returns one instant (reproducible tests).
- **`AdvancingClock`** — ticks forward a fixed step on every `now()`.

## API

```cpp
#include "wall_clock.hpp"
namespace wc = ca::wallclock;

wc::Instant t = wc::Instant::from_secs(1700000000.0);
double dt = t.add_secs(3600.0).duration_since(t);   // 3600.0

wc::FixedClock fixed(t);
double now = my_function(fixed);   // takes a const wc::Clock&
```

`Instant`'s `from_secs` / `add_secs` / `duration_since` and comparisons are
`constexpr`; `EPOCH` is an inline constexpr constant. `AdvancingClock::now()` is
`const` with a `mutable` state member (the analog of Rust's `Cell<f64>`). The
Rust `SystemClock` is omitted from this pure port.

## Portability

Pure ISO C++17 — no `<chrono>`/`<ctime>`. Compiles clean under GCC, Clang, and
MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
