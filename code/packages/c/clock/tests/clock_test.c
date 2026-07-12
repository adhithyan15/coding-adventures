/*
 * Tests for the C clock simulator, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own unit tests — tick edges, full
 * cycles, run, period, reset, listeners, and the invalid-argument (NULL)
 * boundaries — plus functional checks of the divider and multi-phase clock.
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>

#include "clock.h"

/* A listener that counts rising edges through its userdata. */
static void count_rising(const ClockEdge *e, void *userdata) {
    if (e->is_rising) (*(uint32_t *)userdata)++;
}

int main(void) {
    /* ── new ────────────────────────────────────────────────────────────── */
    {
        Clock *clk = clock_new(1000000);
        ISO_CHECK(clk != NULL);
        ISO_CHECK(clock_frequency_hz(clk) == 1000000u);
        ISO_CHECK(clock_cycle(clk) == 0u);
        ISO_CHECK(clock_value(clk) == 0u);
        ISO_CHECK(clock_total_ticks(clk) == 0u);
        clock_free(clk);
    }

    /* ── a rising then a falling tick ───────────────────────────────────── */
    {
        Clock *clk = clock_new(1000000);
        ClockEdge e = clock_tick(clk);
        ISO_CHECK(e.is_rising && !e.is_falling);
        ISO_CHECK(e.value == 1 && e.cycle == 1u);
        e = clock_tick(clk);
        ISO_CHECK(!e.is_rising && e.is_falling);
        ISO_CHECK(e.value == 0 && e.cycle == 1u);
        clock_free(clk);
    }

    /* ── full cycle ─────────────────────────────────────────────────────── */
    {
        Clock *clk = clock_new(1000000);
        ClockEdge r, f;
        clock_full_cycle(clk, &r, &f);
        ISO_CHECK(r.is_rising && f.is_falling);
        ISO_CHECK(clock_cycle(clk) == 1u);
        ISO_CHECK(clock_total_ticks(clk) == 2u);
        clock_free(clk);
    }

    /* ── run 5 cycles → 10 edges ────────────────────────────────────────── */
    {
        Clock *clk = clock_new(1000000);
        size_t n = 0;
        ClockEdge *edges = clock_run(clk, 5, &n);
        ISO_CHECK(edges != NULL);
        ISO_CHECK_EQ_UINT(n, 10u);
        ISO_CHECK(clock_cycle(clk) == 5u);
        ISO_CHECK(clock_total_ticks(clk) == 10u);
        /* Edges alternate rising, falling, rising, … */
        if (n == 10) {
            ISO_CHECK(edges[0].is_rising && edges[1].is_falling);
            ISO_CHECK(edges[8].is_rising && edges[9].is_falling);
        }
        free(edges);
        clock_free(clk);
    }

    /* ── period_ns ──────────────────────────────────────────────────────── */
    {
        Clock *clk = clock_new(1000000000);
        ISO_CHECK_EQ_DBL(clock_period_ns(clk), 1.0, 0.001);
        clock_free(clk);
    }

    /* ── reset ──────────────────────────────────────────────────────────── */
    {
        Clock *clk = clock_new(1000000);
        size_t n;
        free(clock_run(clk, 5, &n));
        clock_reset(clk);
        ISO_CHECK(clock_cycle(clk) == 0u);
        ISO_CHECK(clock_value(clk) == 0u);
        ISO_CHECK(clock_total_ticks(clk) == 0u);
        clock_free(clk);
    }

    /* ── a listener fires on each rising edge ───────────────────────────── */
    {
        Clock *clk = clock_new(1000000);
        uint32_t count = 0;
        ISO_CHECK(clock_register_listener(clk, count_rising, &count) == 0);
        size_t n;
        free(clock_run(clk, 3, &n));
        ISO_CHECK_EQ_UINT(count, 3u); /* three rising edges in three cycles */
        clock_free(clk);
    }

    /* ── invalid arguments return NULL (the Rust panics) ────────────────── */
    ISO_CHECK(clock_new(0) == NULL);
    ISO_CHECK(clock_divider_new(1000000, 1) == NULL);
    ISO_CHECK(mpc_new(1) == NULL);

    /* ── ClockDivider: output frequency and cycle generation ────────────── */
    {
        ClockDivider *d = clock_divider_new(1000000000, 4); /* 1 GHz / 4 */
        ISO_CHECK(d != NULL);
        ISO_CHECK(clock_frequency_hz(clock_divider_output(d)) == 250000000u);
        /* Feed rising edges: every 4 → one output cycle. */
        ClockEdge rising = {0, 1, 1, 0};
        for (int i = 0; i < 8; i++) clock_divider_on_edge(d, &rising);
        ISO_CHECK(clock_cycle(clock_divider_output(d)) == 2u); /* 8/4 = 2 cycles */
        /* Falling edges do not advance the divider. */
        ClockEdge falling = {0, 0, 0, 1};
        clock_divider_on_edge(d, &falling);
        ISO_CHECK(clock_cycle(clock_divider_output(d)) == 2u);
        clock_divider_free(d);
    }

    /* ── MultiPhaseClock: one active phase, rotating ────────────────────── */
    {
        MultiPhaseClock *m = mpc_new(4);
        ISO_CHECK(m != NULL);
        ISO_CHECK(mpc_num_phases(m) == 4u);
        ISO_CHECK(mpc_get_phase(m, 0) == 0); /* no edges yet */
        ClockEdge rising = {0, 1, 1, 0};

        mpc_on_edge(m, &rising); /* phase 0 active */
        ISO_CHECK(mpc_get_phase(m, 0) == 1);
        ISO_CHECK(mpc_get_phase(m, 1) == 0);

        mpc_on_edge(m, &rising); /* phase 1 active */
        ISO_CHECK(mpc_get_phase(m, 1) == 1);
        ISO_CHECK(mpc_get_phase(m, 0) == 0);

        mpc_on_edge(m, &rising); /* phase 2 */
        mpc_on_edge(m, &rising); /* phase 3 */
        ISO_CHECK(mpc_get_phase(m, 3) == 1);
        mpc_on_edge(m, &rising); /* wraps back to phase 0 */
        ISO_CHECK(mpc_get_phase(m, 0) == 1);

        /* An out-of-range index yields 0, not a crash. */
        ISO_CHECK(mpc_get_phase(m, 99) == 0);
        mpc_free(m);
    }

    return ISO_TEST_RESULT();
}
