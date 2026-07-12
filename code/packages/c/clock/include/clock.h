/*
 * clock.h — the heartbeat of every digital circuit, in pure ISO C17. A faithful
 * port of the Rust `clock` crate.
 * ===========================================================================
 *
 * Every sequential circuit — flip-flops, registers, counters, CPU pipeline
 * stages — is driven by a clock: a square wave that alternates 0 and 1. On each
 * rising edge (0→1) synchronous logic captures its inputs. This crate simulates
 * that heartbeat:
 *
 *   - Clock           : a square-wave generator (tick / full-cycle / run), with
 *                       edge listeners and a cycle/tick count.
 *   - ClockDivider    : derives a slower clock (source / divisor).
 *   - MultiPhaseClock : rotates a single active phase across N non-overlapping
 *                       outputs.
 *
 * A complete cycle is two ticks: low→high (rising edge, the "active" half) then
 * high→low (falling edge). The cycle count increments on each rising edge.
 *
 * LISTENERS. Rust stores `Box<dyn FnMut(&ClockEdge)>` closures; this port uses a
 * C callback function pointer plus a `void *userdata` (the classic C analog of a
 * captured closure). The Clock owns the list; each is invoked on every edge.
 *
 * DIVERGENCE FROM RUST. Where Rust `new` PANICS on an invalid argument
 * (frequency 0, divisor < 2, phases < 2), this port's constructor returns NULL;
 * `mpc_get_phase` returns 0 for an out-of-range index rather than panicking.
 *
 * PORTABILITY. Pure ISO C17 — no extensions. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_CLOCK_H
#define CA_CLOCK_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Record of one clock transition — like a timestamp in a logic-analyzer trace. */
typedef struct {
    uint64_t cycle;  /* which cycle (counts from 1 on the first rising edge) */
    uint8_t value;   /* the new signal level (0 or 1) */
    int is_rising;   /* was this a 0→1 transition? */
    int is_falling;  /* was this a 1→0 transition? */
} ClockEdge;

/* A listener called on every edge, with caller-supplied context. */
typedef void (*ClockListener)(const ClockEdge *edge, void *userdata);

/* ── Clock ──────────────────────────────────────────────────────────────── */

typedef struct Clock Clock;

/* Create a clock at `frequency_hz` (starts at value 0, cycle 0, 0 ticks).
 * Returns NULL if `frequency_hz` is 0 (the Rust panic) or on allocation
 * failure. */
Clock *clock_new(uint64_t frequency_hz);
void clock_free(Clock *c);

/* Advance one half-cycle (toggle the value), notify listeners, return the edge. */
ClockEdge clock_tick(Clock *c);
/* One complete cycle: writes the rising then falling edge through the outs. */
void clock_full_cycle(Clock *c, ClockEdge *rising_out, ClockEdge *falling_out);
/* Run `cycles` complete cycles. Returns a malloc'd array of 2·cycles edges (the
 * count via *count_out; free with free()), or NULL on OOM / overflow. A count of
 * 0 yields a NULL array and *count_out == 0. */
ClockEdge *clock_run(Clock *c, uint64_t cycles, size_t *count_out);
/* Register an edge listener. Returns 0, or -1 on allocation failure. */
int clock_register_listener(Clock *c, ClockListener fn, void *userdata);
/* Reset the timing state (value, cycle, ticks) to 0; listeners are preserved. */
void clock_reset(Clock *c);

double clock_period_ns(const Clock *c);       /* 1e9 / frequency_hz */
uint64_t clock_total_ticks(const Clock *c);
uint64_t clock_frequency_hz(const Clock *c);
uint64_t clock_cycle(const Clock *c);
uint8_t clock_value(const Clock *c);

/* ── ClockDivider ───────────────────────────────────────────────────────── */

typedef struct ClockDivider ClockDivider;

/* Divide a source clock by `divisor` (>= 2). The output runs at
 * source_frequency_hz / divisor. Returns NULL if `divisor` < 2 (the Rust panic),
 * if the resulting output frequency would be 0, or on allocation failure. */
ClockDivider *clock_divider_new(uint64_t source_frequency_hz, uint64_t divisor);
void clock_divider_free(ClockDivider *d);
/* Feed a source-clock edge: every `divisor` rising edges emits one output cycle. */
void clock_divider_on_edge(ClockDivider *d, const ClockEdge *edge);
const Clock *clock_divider_output(const ClockDivider *d);
Clock *clock_divider_output_mut(ClockDivider *d);

/* ── MultiPhaseClock ────────────────────────────────────────────────────── */

typedef struct MultiPhaseClock MultiPhaseClock;

/* Create an N-phase clock (`phases` >= 2). Returns NULL if `phases` < 2 (the
 * Rust panic) or on allocation failure. */
MultiPhaseClock *mpc_new(size_t phases);
void mpc_free(MultiPhaseClock *m);
/* On each source rising edge, exactly one phase is high; the active phase
 * rotates. */
void mpc_on_edge(MultiPhaseClock *m, const ClockEdge *edge);
/* The value (0/1) of phase `index`, or 0 if `index` is out of range. */
uint8_t mpc_get_phase(const MultiPhaseClock *m, size_t index);
size_t mpc_num_phases(const MultiPhaseClock *m);

#ifdef __cplusplus
}
#endif

#endif /* CA_CLOCK_H */
