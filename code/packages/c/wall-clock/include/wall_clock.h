/*
 * wall_clock.h — an injectable source of "now", in pure ISO C17. A faithful
 * port of the Rust `wall-clock` crate (the pure, no-`std::time` core).
 * ===========================================================================
 *
 * Datetime and spreadsheet functions should not reach directly into the host
 * clock — that makes them untestable and non-portable (e.g. bare WASM has no
 * system clock). Instead they call into a `WcClock`: an abstract "now" source.
 *
 *   - WcInstant        : a point in time as f64 seconds since the Unix epoch.
 *   - WcClock          : the abstract clock (a "now" function + its context) —
 *                        the C analog of Rust's `dyn Clock` trait object.
 *   - WcFixedClock     : always returns one instant (for reproducible tests).
 *   - WcAdvancingClock : ticks forward a fixed step on every `now()` (for tests
 *                        of time progression).
 *
 * The Rust crate's `SystemClock` (which reads the host clock) lives behind a
 * feature flag because it needs `std::time`; this pure port omits it — inject a
 * host-supplied `WcClock` at the boundary instead, exactly as WASM consumers do.
 *
 * All types are plain value types — no allocation, nothing to free. A `WcClock`
 * borrows the concrete clock it was built from; keep that alive while in use.
 *
 * PORTABILITY. Pure ISO C17 — no `<time.h>`, no extensions. Builds clean under
 * GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_WALL_CLOCK_H
#define CA_WALL_CLOCK_H

#ifdef __cplusplus
extern "C" {
#endif

/* A point in time: seconds since 1970-01-01T00:00:00Z (negative for earlier). */
typedef struct {
    double seconds_since_epoch;
} WcInstant;

WcInstant wc_instant_from_secs(double s);
WcInstant wc_instant_epoch(void); /* the Unix epoch, {0.0} */
/* A new instant `secs` seconds after `self`. */
WcInstant wc_instant_add_secs(WcInstant self, double secs);
/* The difference `self - other`, in seconds. */
double wc_instant_duration_since(WcInstant self, WcInstant other);

/* Comparisons follow f64 semantics (the Rust `PartialEq`/`PartialOrd` on the
 * stored `f64`) — any comparison involving a NaN instant is false. */
int wc_instant_eq(WcInstant a, WcInstant b);
int wc_instant_lt(WcInstant a, WcInstant b);
int wc_instant_le(WcInstant a, WcInstant b);
int wc_instant_gt(WcInstant a, WcInstant b);
int wc_instant_ge(WcInstant a, WcInstant b);

/* The abstract clock: a `now` function plus its context (a "trait object"). */
typedef WcInstant (*WcClockNowFn)(void *ctx);
typedef struct {
    WcClockNowFn now_fn;
    void *ctx;
} WcClock;

/* The current instant according to `clock`. */
WcInstant wc_clock_now(WcClock clock);

/* A clock pinned at a single instant. */
typedef struct {
    WcInstant instant;
} WcFixedClock;

WcFixedClock wc_fixed_clock_new(WcInstant instant);
WcFixedClock wc_fixed_clock_epoch(void); /* pinned at the Unix epoch */
WcInstant wc_fixed_clock_now(const WcFixedClock *c);
/* View a fixed clock as an abstract WcClock (borrows `*c`). */
WcClock wc_fixed_clock_as_clock(WcFixedClock *c);

/* A clock that advances a fixed step on every `now()` (steps may be negative). */
typedef struct {
    double state;
    double step;
} WcAdvancingClock;

WcAdvancingClock wc_advancing_clock_new(WcInstant start, double step_seconds);
/* Return the current instant, then advance the clock by its step. */
WcInstant wc_advancing_clock_now(WcAdvancingClock *c);
/* View an advancing clock as an abstract WcClock (borrows `*c`; its `now`
 * mutates the borrowed clock's state). */
WcClock wc_advancing_clock_as_clock(WcAdvancingClock *c);

#ifdef __cplusplus
}
#endif

#endif /* CA_WALL_CLOCK_H */
