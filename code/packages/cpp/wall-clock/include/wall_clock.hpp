// wall_clock.hpp — an injectable source of "now", in pure ISO C++17,
// header-only, in namespace ca::wallclock. A faithful port of the Rust
// `wall-clock` crate (the pure, no-`std::time` core).
// ===========================================================================
//
// Datetime and spreadsheet functions should not reach directly into the host
// clock — that makes them untestable and non-portable. Instead they take a
// reference to a `Clock`, an abstract "now" source:
//
//   - Instant        : a point in time as f64 seconds since the Unix epoch.
//   - Clock          : the abstract clock (virtual now()) — the direct analog
//                      of Rust's `dyn Clock` trait object.
//   - FixedClock     : always returns one instant (for reproducible tests).
//   - AdvancingClock : ticks forward a fixed step on every now().
//
// The Rust crate's `SystemClock` needs `std::time` (behind a feature flag); this
// pure port omits it — inject a host-supplied Clock at the boundary instead.
//
// PORTABILITY. Pure ISO C++17 — no <chrono>/<ctime>. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_WALL_CLOCK_HPP
#define CA_WALL_CLOCK_HPP

namespace ca {
namespace wallclock {

// A point in time: seconds since 1970-01-01T00:00:00Z (negative for earlier).
struct Instant {
    double seconds_since_epoch;

    static constexpr Instant from_secs(double s) { return Instant{s}; }

    // A new instant `secs` seconds after this one.
    constexpr Instant add_secs(double secs) const {
        return Instant{seconds_since_epoch + secs};
    }
    // The difference `*this - other`, in seconds.
    constexpr double duration_since(Instant other) const {
        return seconds_since_epoch - other.seconds_since_epoch;
    }

    // Comparisons follow the derived f64 PartialEq/PartialOrd (exact; any
    // comparison with a NaN instant is false).
    constexpr bool operator==(Instant o) const {
        return seconds_since_epoch == o.seconds_since_epoch;
    }
    constexpr bool operator!=(Instant o) const { return !(*this == o); }
    constexpr bool operator<(Instant o) const {
        return seconds_since_epoch < o.seconds_since_epoch;
    }
    constexpr bool operator<=(Instant o) const {
        return seconds_since_epoch <= o.seconds_since_epoch;
    }
    constexpr bool operator>(Instant o) const {
        return seconds_since_epoch > o.seconds_since_epoch;
    }
    constexpr bool operator>=(Instant o) const {
        return seconds_since_epoch >= o.seconds_since_epoch;
    }
};

// The Unix epoch itself.
inline constexpr Instant EPOCH = Instant::from_secs(0.0);

// A source of "now" that callers depend on instead of touching the host clock.
class Clock {
public:
    virtual ~Clock() = default;
    virtual Instant now() const = 0;
};

// A clock pinned at a single instant (for reproducible tests).
class FixedClock : public Clock {
public:
    explicit FixedClock(Instant instant) : instant_(instant) {}
    static FixedClock epoch() { return FixedClock(EPOCH); }
    Instant now() const override { return instant_; }

private:
    Instant instant_;
};

// A clock that advances a fixed step on every now() (steps may be negative).
class AdvancingClock : public Clock {
public:
    AdvancingClock(Instant start, double step_seconds)
        : state_(start.seconds_since_epoch), step_(step_seconds) {}

    Instant now() const override {
        double current = state_;
        state_ += step_; // interior mutability, matching Rust's Cell<f64>
        return Instant::from_secs(current);
    }

private:
    mutable double state_;
    double step_;
};

}  // namespace wallclock
}  // namespace ca

#endif  // CA_WALL_CLOCK_HPP
