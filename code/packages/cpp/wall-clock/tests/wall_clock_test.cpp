// Tests for the C++ wall-clock, using the header-only iso_test.h harness (pure
// ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include "wall_clock.hpp"

namespace wc = ca::wallclock;

// The "real" use case: a function that depends only on an abstract Clock.
static double now_secs(const wc::Clock& clock) {
    return clock.now().seconds_since_epoch;
}

int main() {
    // ── Instant arithmetic round-trips ───────────────────────────────────
    {
        wc::Instant a = wc::Instant::from_secs(100.0);
        wc::Instant b = a.add_secs(50.5);
        ISO_CHECK_EQ_DBL(b.seconds_since_epoch, 150.5, 0.0);
        ISO_CHECK_EQ_DBL(b.duration_since(a), 50.5, 0.0);
    }

    // ── an instant can predate the epoch; comparisons work ───────────────
    {
        wc::Instant pre = wc::Instant::from_secs(-1000.0);
        ISO_CHECK(pre < wc::EPOCH);
        ISO_CHECK(wc::EPOCH > pre);
        ISO_CHECK(wc::EPOCH == wc::Instant::from_secs(0.0));
        ISO_CHECK(pre != wc::EPOCH);
        ISO_CHECK(pre <= pre && pre >= pre);
    }

    // ── constexpr usability ──────────────────────────────────────────────
    {
        constexpr wc::Instant s = wc::Instant::from_secs(2.0).add_secs(3.0);
        static_assert(s.seconds_since_epoch == 5.0, "constexpr add_secs");
        static_assert(wc::EPOCH.seconds_since_epoch == 0.0, "constexpr EPOCH");
    }

    // ── FixedClock returns the same instant every call ───────────────────
    {
        wc::Instant target = wc::Instant::from_secs(42.0);
        wc::FixedClock clock(target);
        for (int i = 0; i < 10; i++) ISO_CHECK(clock.now() == target);
    }
    {
        wc::FixedClock clock = wc::FixedClock::epoch();
        ISO_CHECK(clock.now() == wc::EPOCH);
    }

    // ── AdvancingClock steps forward (and backward) ──────────────────────
    {
        wc::AdvancingClock clock(wc::Instant::from_secs(0.0), 1.0);
        ISO_CHECK_EQ_DBL(clock.now().seconds_since_epoch, 0.0, 0.0);
        ISO_CHECK_EQ_DBL(clock.now().seconds_since_epoch, 1.0, 0.0);
        ISO_CHECK_EQ_DBL(clock.now().seconds_since_epoch, 2.0, 0.0);
    }
    {
        wc::AdvancingClock clock(wc::Instant::from_secs(100.0), -5.0);
        ISO_CHECK_EQ_DBL(clock.now().seconds_since_epoch, 100.0, 0.0);
        ISO_CHECK_EQ_DBL(clock.now().seconds_since_epoch, 95.0, 0.0);
    }

    // ── polymorphic injection through the Clock base ─────────────────────
    {
        wc::FixedClock fixed(wc::Instant::from_secs(7.0));
        ISO_CHECK_EQ_DBL(now_secs(fixed), 7.0, 0.0);

        wc::AdvancingClock advancing(wc::Instant::from_secs(10.0), 1.0);
        ISO_CHECK_EQ_DBL(now_secs(advancing), 10.0, 0.0);
        ISO_CHECK_EQ_DBL(now_secs(advancing), 11.0, 0.0);
    }

    return ISO_TEST_RESULT();
}
