/*
 * wall_clock.c — implementation of the injectable clock.
 * ===========================================================================
 *
 * Everything is plain f64 arithmetic on value types. The polymorphic `WcClock`
 * is a fat pointer — a `now` function plus a `void *` context — and the two
 * concrete clocks expose an `as_clock` that binds the right thunk to a borrowed
 * pointer, the C analog of coercing `&FixedClock` to `&dyn Clock`.
 */
#include "wall_clock.h"

/* ===========================================================================
 *  Instant
 * =========================================================================== */

WcInstant wc_instant_from_secs(double s) {
    WcInstant i;
    i.seconds_since_epoch = s;
    return i;
}

WcInstant wc_instant_epoch(void) { return wc_instant_from_secs(0.0); }

WcInstant wc_instant_add_secs(WcInstant self, double secs) {
    return wc_instant_from_secs(self.seconds_since_epoch + secs);
}

double wc_instant_duration_since(WcInstant self, WcInstant other) {
    return self.seconds_since_epoch - other.seconds_since_epoch;
}

int wc_instant_eq(WcInstant a, WcInstant b) {
    return a.seconds_since_epoch == b.seconds_since_epoch;
}
int wc_instant_lt(WcInstant a, WcInstant b) {
    return a.seconds_since_epoch < b.seconds_since_epoch;
}
int wc_instant_le(WcInstant a, WcInstant b) {
    return a.seconds_since_epoch <= b.seconds_since_epoch;
}
int wc_instant_gt(WcInstant a, WcInstant b) {
    return a.seconds_since_epoch > b.seconds_since_epoch;
}
int wc_instant_ge(WcInstant a, WcInstant b) {
    return a.seconds_since_epoch >= b.seconds_since_epoch;
}

/* ===========================================================================
 *  WcClock
 * =========================================================================== */

WcInstant wc_clock_now(WcClock clock) { return clock.now_fn(clock.ctx); }

/* ===========================================================================
 *  FixedClock
 * =========================================================================== */

WcFixedClock wc_fixed_clock_new(WcInstant instant) {
    WcFixedClock c;
    c.instant = instant;
    return c;
}

WcFixedClock wc_fixed_clock_epoch(void) {
    return wc_fixed_clock_new(wc_instant_epoch());
}

WcInstant wc_fixed_clock_now(const WcFixedClock *c) { return c->instant; }

/* Thunk: interpret the context as a WcFixedClock and read its instant. */
static WcInstant fixed_now_thunk(void *ctx) {
    return wc_fixed_clock_now((const WcFixedClock *)ctx);
}

WcClock wc_fixed_clock_as_clock(WcFixedClock *c) {
    WcClock clock;
    clock.now_fn = fixed_now_thunk;
    clock.ctx = c;
    return clock;
}

/* ===========================================================================
 *  AdvancingClock
 * =========================================================================== */

WcAdvancingClock wc_advancing_clock_new(WcInstant start, double step_seconds) {
    WcAdvancingClock c;
    c.state = start.seconds_since_epoch;
    c.step = step_seconds;
    return c;
}

WcInstant wc_advancing_clock_now(WcAdvancingClock *c) {
    double current = c->state;
    c->state = current + c->step;
    return wc_instant_from_secs(current);
}

/* Thunk: interpret the context as a WcAdvancingClock and advance it. */
static WcInstant advancing_now_thunk(void *ctx) {
    return wc_advancing_clock_now((WcAdvancingClock *)ctx);
}

WcClock wc_advancing_clock_as_clock(WcAdvancingClock *c) {
    WcClock clock;
    clock.now_fn = advancing_now_thunk;
    clock.ctx = c;
    return clock;
}
