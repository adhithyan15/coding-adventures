/*
 * Tests for the C wall-clock, using the header-only iso_test.h harness (pure
 * ISO). Vectors mirror the Rust crate's own unit tests — Instant arithmetic and
 * ordering, the fixed and advancing clocks, and polymorphic clock injection via
 * the WcClock "trait object".
 */
#include "iso_test.h"

#include "wall_clock.h"

/* The "real" use case: a function that depends only on an abstract clock. */
static double now_secs(WcClock clock) {
    return wc_clock_now(clock).seconds_since_epoch;
}

int main(void) {
    /* ── Instant arithmetic round-trips ─────────────────────────────────── */
    {
        WcInstant a = wc_instant_from_secs(100.0);
        WcInstant b = wc_instant_add_secs(a, 50.5);
        ISO_CHECK_EQ_DBL(b.seconds_since_epoch, 150.5, 0.0);
        ISO_CHECK_EQ_DBL(wc_instant_duration_since(b, a), 50.5, 0.0);
    }

    /* ── an instant can predate the epoch ───────────────────────────────── */
    {
        WcInstant pre = wc_instant_from_secs(-1000.0);
        ISO_CHECK(wc_instant_lt(pre, wc_instant_epoch()));
        ISO_CHECK(wc_instant_gt(wc_instant_epoch(), pre));
        ISO_CHECK(wc_instant_eq(wc_instant_epoch(), wc_instant_from_secs(0.0)));
        ISO_CHECK(wc_instant_ge(pre, pre) && wc_instant_le(pre, pre));
    }

    /* ── FixedClock returns the same instant every call ─────────────────── */
    {
        WcInstant target = wc_instant_from_secs(42.0);
        WcFixedClock clock = wc_fixed_clock_new(target);
        for (int i = 0; i < 10; i++) {
            ISO_CHECK(wc_instant_eq(wc_fixed_clock_now(&clock), target));
        }
    }
    {
        WcFixedClock clock = wc_fixed_clock_epoch();
        ISO_CHECK(wc_instant_eq(wc_fixed_clock_now(&clock), wc_instant_epoch()));
    }

    /* ── AdvancingClock steps forward (and backward) ────────────────────── */
    {
        WcAdvancingClock clock =
            wc_advancing_clock_new(wc_instant_from_secs(0.0), 1.0);
        ISO_CHECK_EQ_DBL(wc_advancing_clock_now(&clock).seconds_since_epoch, 0.0,
                         0.0);
        ISO_CHECK_EQ_DBL(wc_advancing_clock_now(&clock).seconds_since_epoch, 1.0,
                         0.0);
        ISO_CHECK_EQ_DBL(wc_advancing_clock_now(&clock).seconds_since_epoch, 2.0,
                         0.0);
    }
    {
        WcAdvancingClock clock =
            wc_advancing_clock_new(wc_instant_from_secs(100.0), -5.0);
        ISO_CHECK_EQ_DBL(wc_advancing_clock_now(&clock).seconds_since_epoch,
                         100.0, 0.0);
        ISO_CHECK_EQ_DBL(wc_advancing_clock_now(&clock).seconds_since_epoch, 95.0,
                         0.0);
    }

    /* ── polymorphic injection through the WcClock trait object ─────────── */
    {
        WcFixedClock fixed = wc_fixed_clock_new(wc_instant_from_secs(7.0));
        ISO_CHECK_EQ_DBL(now_secs(wc_fixed_clock_as_clock(&fixed)), 7.0, 0.0);

        WcAdvancingClock advancing =
            wc_advancing_clock_new(wc_instant_from_secs(10.0), 1.0);
        WcClock ac = wc_advancing_clock_as_clock(&advancing);
        ISO_CHECK_EQ_DBL(now_secs(ac), 10.0, 0.0);
        ISO_CHECK_EQ_DBL(now_secs(ac), 11.0, 0.0); /* the borrowed clock advanced */
    }

    return ISO_TEST_RESULT();
}
