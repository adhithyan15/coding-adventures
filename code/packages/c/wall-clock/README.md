# wall-clock (C)

An injectable source of "now" in pure ISO C17. A faithful port of the pure
(no-`std::time`) core of the Rust `wall-clock` crate.

Datetime and spreadsheet functions should not reach directly into the host clock
— that makes them untestable and non-portable. Instead they call into a
`WcClock`, an abstract "now" source.

- **`WcInstant`** — a point in time as f64 seconds since the Unix epoch.
- **`WcClock`** — the abstract clock (a `now` function + its context), the C
  analog of Rust's `dyn Clock` trait object.
- **`WcFixedClock`** — always returns one instant (reproducible tests).
- **`WcAdvancingClock`** — ticks forward a fixed step on every `now()`.

## API

```c
#include "wall_clock.h"

WcInstant t = wc_instant_from_secs(1700000000.0);
WcInstant later = wc_instant_add_secs(t, 3600.0);
double dt = wc_instant_duration_since(later, t);   /* 3600.0 */

WcFixedClock fixed = wc_fixed_clock_new(t);
WcInstant now = wc_clock_now(wc_fixed_clock_as_clock(&fixed));  /* polymorphic */
```

All types are plain value types — no allocation, nothing to free. A `WcClock`
borrows the concrete clock it was built from; keep that alive while in use. The
Rust `SystemClock` (which reads the host clock via `std::time`) is omitted from
this pure port — inject a host-supplied `WcClock` at the boundary instead, as
WASM consumers do.

## Portability

Pure ISO C17 — no `<time.h>`, no extensions. Compiles clean under GCC, Clang, and
MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
