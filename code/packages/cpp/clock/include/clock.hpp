// clock.hpp — the heartbeat of every digital circuit, in pure ISO C++17,
// header-only, in namespace ca::clk. A faithful port of the Rust `clock` crate.
// ===========================================================================
//
// Every sequential circuit is driven by a clock: a square wave alternating 0/1;
// on each rising edge (0→1) synchronous logic captures its inputs. This crate
// simulates that heartbeat:
//
//   - Clock           : a square-wave generator (tick / full_cycle / run), with
//                       edge listeners and a cycle/tick count.
//   - ClockDivider    : derives a slower clock (source / divisor).
//   - MultiPhaseClock : rotates a single active phase across N outputs.
//
// A complete cycle is two ticks: low→high (rising edge) then high→low (falling).
// The cycle count increments on each rising edge.
//
// LISTENERS. Rust's `Box<dyn FnMut(&ClockEdge)>` maps directly to
// `std::function<void(const ClockEdge&)>` — a stored callable that may mutate
// its captures. The Clock owns the list and invokes each on every edge.
//
// DIVERGENCE FROM RUST. Rust `new` PANICS on an invalid argument; this port's
// constructors throw `std::invalid_argument`, and `MultiPhaseClock::get_phase`
// throws `std::out_of_range` for an out-of-range index (both faithful to the
// panic).
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_CLOCK_HPP
#define CA_CLOCK_HPP

#include <algorithm>
#include <cstdint>
#include <functional>
#include <stdexcept>
#include <utility>
#include <vector>

namespace ca {
namespace clk {

// Record of one clock transition.
struct ClockEdge {
    std::uint64_t cycle;
    std::uint8_t value;
    bool is_rising;
    bool is_falling;

    bool operator==(const ClockEdge& o) const {
        return cycle == o.cycle && value == o.value && is_rising == o.is_rising &&
               is_falling == o.is_falling;
    }
    bool operator!=(const ClockEdge& o) const { return !(*this == o); }
};

// System clock generator — a toggle that counts cycles and notifies listeners.
class Clock {
public:
    // Throws std::invalid_argument if frequency_hz == 0.
    explicit Clock(std::uint64_t frequency_hz) : frequency_hz_(frequency_hz) {
        if (frequency_hz == 0) {
            throw std::invalid_argument("frequency_hz must be > 0");
        }
    }

    // Advance one half-cycle (toggle), notify listeners, return the edge.
    ClockEdge tick() {
        std::uint8_t old_value = value_;
        value_ = static_cast<std::uint8_t>(1 - value_);
        ++total_ticks_;

        bool is_rising = old_value == 0 && value_ == 1;
        bool is_falling = old_value == 1 && value_ == 0;
        if (is_rising) ++cycle_;

        ClockEdge edge{cycle_, value_, is_rising, is_falling};
        for (auto& listener : listeners_) listener(edge);
        return edge;
    }

    // One complete cycle: {rising, falling}.
    std::pair<ClockEdge, ClockEdge> full_cycle() {
        ClockEdge rising = tick();
        ClockEdge falling = tick();
        return {rising, falling};
    }

    // Run `cycles` complete cycles; returns all 2·cycles edges.
    std::vector<ClockEdge> run(std::uint64_t cycles) {
        std::vector<ClockEdge> edges;
        edges.reserve(static_cast<std::size_t>(cycles * 2));
        for (std::uint64_t i = 0; i < cycles; ++i) {
            auto [r, f] = full_cycle();
            edges.push_back(r);
            edges.push_back(f);
        }
        return edges;
    }

    // Register an edge listener.
    void register_listener(std::function<void(const ClockEdge&)> f) {
        listeners_.push_back(std::move(f));
    }

    // Reset the timing state; listeners are preserved.
    void reset() {
        cycle_ = 0;
        value_ = 0;
        total_ticks_ = 0;
    }

    double period_ns() const { return 1e9 / static_cast<double>(frequency_hz_); }
    std::uint64_t total_ticks() const { return total_ticks_; }
    std::uint64_t frequency_hz() const { return frequency_hz_; }
    std::uint64_t cycle() const { return cycle_; }
    std::uint8_t value() const { return value_; }

private:
    std::uint64_t frequency_hz_;
    std::uint64_t cycle_ = 0;
    std::uint8_t value_ = 0;
    std::uint64_t total_ticks_ = 0;
    std::vector<std::function<void(const ClockEdge&)>> listeners_;
};

// Divides a source clock frequency by an integer factor.
class ClockDivider {
public:
    // Throws std::invalid_argument if divisor < 2 or the output frequency
    // (source / divisor) would be 0.
    ClockDivider(std::uint64_t source_frequency_hz, std::uint64_t divisor)
        : divisor_(divisor), output_(make_output(source_frequency_hz, divisor)) {}

    // Feed a source edge: every `divisor` rising edges emits one output cycle.
    void on_edge(const ClockEdge& edge) {
        if (edge.is_rising) {
            ++counter_;
            if (counter_ >= divisor_) {
                counter_ = 0;
                output_.tick(); // rising
                output_.tick(); // falling
            }
        }
    }

    const Clock& output() const { return output_; }
    Clock& output_mut() { return output_; }

private:
    static Clock make_output(std::uint64_t source, std::uint64_t divisor) {
        if (divisor < 2) {
            throw std::invalid_argument("divisor must be >= 2");
        }
        return Clock(source / divisor); // throws if the quotient is 0
    }
    std::uint64_t divisor_;
    std::uint64_t counter_ = 0;
    Clock output_;
};

// Generates N non-overlapping clock phases from a single source.
class MultiPhaseClock {
public:
    // Throws std::invalid_argument if phases < 2.
    explicit MultiPhaseClock(std::size_t phases)
        : phases_(phases), phase_values_(phases, 0) {
        if (phases < 2) {
            throw std::invalid_argument("phases must be >= 2");
        }
    }

    // On each rising edge, exactly one phase is high; the active phase rotates.
    void on_edge(const ClockEdge& edge) {
        if (edge.is_rising) {
            std::fill(phase_values_.begin(), phase_values_.end(), 0);
            phase_values_[active_phase_] = 1;
            active_phase_ = (active_phase_ + 1) % phases_;
        }
    }

    // The value (0/1) of phase `index`; throws std::out_of_range if invalid.
    std::uint8_t get_phase(std::size_t index) const {
        return phase_values_.at(index);
    }

    std::size_t num_phases() const { return phases_; }

private:
    std::size_t phases_;
    std::size_t active_phase_ = 0;
    std::vector<std::uint8_t> phase_values_;
};

}  // namespace clk
}  // namespace ca

#endif  // CA_CLOCK_HPP
