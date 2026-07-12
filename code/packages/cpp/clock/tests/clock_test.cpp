// Tests for the C++ clock simulator, using the header-only iso_test.h harness
// (pure ISO). Vectors mirror the Rust crate's own unit tests, plus functional
// checks of the divider and multi-phase clock.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <vector>

#include "clock.hpp"

namespace clk = ca::clk;

int main() {
    // ── new ──────────────────────────────────────────────────────────────
    {
        clk::Clock c(1000000);
        ISO_CHECK(c.frequency_hz() == 1000000u);
        ISO_CHECK(c.cycle() == 0u);
        ISO_CHECK(c.value() == 0u);
        ISO_CHECK(c.total_ticks() == 0u);
    }

    // ── rising then falling tick ─────────────────────────────────────────
    {
        clk::Clock c(1000000);
        clk::ClockEdge e = c.tick();
        ISO_CHECK(e.is_rising && !e.is_falling);
        ISO_CHECK(e.value == 1 && e.cycle == 1u);
        e = c.tick();
        ISO_CHECK(!e.is_rising && e.is_falling);
        ISO_CHECK(e.value == 0 && e.cycle == 1u);
    }

    // ── full cycle ───────────────────────────────────────────────────────
    {
        clk::Clock c(1000000);
        auto [r, f] = c.full_cycle();
        ISO_CHECK(r.is_rising && f.is_falling);
        ISO_CHECK(c.cycle() == 1u && c.total_ticks() == 2u);
    }

    // ── run 5 cycles → 10 edges ──────────────────────────────────────────
    {
        clk::Clock c(1000000);
        std::vector<clk::ClockEdge> edges = c.run(5);
        ISO_CHECK_EQ_UINT(edges.size(), 10u);
        ISO_CHECK(c.cycle() == 5u && c.total_ticks() == 10u);
        ISO_CHECK(edges[0].is_rising && edges[1].is_falling);
        ISO_CHECK(edges[8].is_rising && edges[9].is_falling);
    }

    // ── period_ns ────────────────────────────────────────────────────────
    {
        clk::Clock c(1000000000);
        ISO_CHECK_EQ_DBL(c.period_ns(), 1.0, 0.001);
    }

    // ── reset ────────────────────────────────────────────────────────────
    {
        clk::Clock c(1000000);
        c.run(5);
        c.reset();
        ISO_CHECK(c.cycle() == 0u && c.value() == 0u && c.total_ticks() == 0u);
    }

    // ── a listener (std::function) fires on each rising edge ─────────────
    {
        clk::Clock c(1000000);
        std::uint32_t count = 0;
        c.register_listener([&count](const clk::ClockEdge& e) {
            if (e.is_rising) ++count;
        });
        c.run(3);
        ISO_CHECK_EQ_UINT(count, 3u);
    }

    // ── invalid arguments throw (the Rust panics) ────────────────────────
    {
        bool threw = false;
        try {
            clk::Clock c(0);
            (void)c;
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    {
        bool threw = false;
        try {
            clk::ClockDivider d(1000000, 1);
            (void)d;
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    {
        bool threw = false;
        try {
            clk::MultiPhaseClock m(1);
            (void)m;
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── ClockDivider: output frequency and cycle generation ──────────────
    {
        clk::ClockDivider d(1000000000, 4); // 1 GHz / 4 = 250 MHz
        ISO_CHECK(d.output().frequency_hz() == 250000000u);
        clk::ClockEdge rising{0, 1, true, false};
        for (int i = 0; i < 8; ++i) d.on_edge(rising);
        ISO_CHECK(d.output().cycle() == 2u); // 8 / 4 = 2 output cycles
        clk::ClockEdge falling{0, 0, false, true};
        d.on_edge(falling);
        ISO_CHECK(d.output().cycle() == 2u); // falling edges do not advance
    }

    // ── MultiPhaseClock: one active phase, rotating ──────────────────────
    {
        clk::MultiPhaseClock m(4);
        ISO_CHECK(m.num_phases() == 4u);
        ISO_CHECK(m.get_phase(0) == 0); // no edges yet
        clk::ClockEdge rising{0, 1, true, false};

        m.on_edge(rising);
        ISO_CHECK(m.get_phase(0) == 1 && m.get_phase(1) == 0);
        m.on_edge(rising);
        ISO_CHECK(m.get_phase(1) == 1 && m.get_phase(0) == 0);
        m.on_edge(rising);
        m.on_edge(rising);
        ISO_CHECK(m.get_phase(3) == 1);
        m.on_edge(rising); // wraps to phase 0
        ISO_CHECK(m.get_phase(0) == 1);

        // An out-of-range index throws (faithful to the Rust panic).
        bool threw = false;
        try {
            (void)m.get_phase(99);
        } catch (const std::out_of_range&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
