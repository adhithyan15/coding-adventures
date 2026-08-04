/*
 * clock.c — implementation of the digital-clock simulator.
 * ===========================================================================
 *
 * The Clock is a toggle: each tick flips the value, counts a cycle on every
 * rising edge, and notifies its listeners. The ClockDivider and MultiPhaseClock
 * are driven manually (call their on_edge with each source edge), mirroring the
 * Rust design that keeps data flow explicit rather than hidden behind shared
 * mutable state.
 */
#include "clock.h"

#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  Clock
 * =========================================================================== */

typedef struct {
    ClockListener fn;
    void *userdata;
} Listener;

struct Clock {
    uint64_t frequency_hz;
    uint64_t cycle;
    uint8_t value;
    uint64_t total_ticks;
    Listener *listeners;
    size_t n_listeners, cap_listeners;
};

Clock *clock_new(uint64_t frequency_hz) {
    if (frequency_hz == 0) return NULL; /* Rust panics; we report via NULL */
    Clock *c = calloc(1, sizeof *c);
    if (!c) return NULL;
    c->frequency_hz = frequency_hz;
    return c;
}

void clock_free(Clock *c) {
    if (!c) return;
    free(c->listeners);
    free(c);
}

ClockEdge clock_tick(Clock *c) {
    uint8_t old_value = c->value;
    c->value = (uint8_t)(1 - c->value);
    c->total_ticks++;

    int is_rising = (old_value == 0 && c->value == 1);
    int is_falling = (old_value == 1 && c->value == 0);
    if (is_rising) c->cycle++;

    ClockEdge edge;
    edge.cycle = c->cycle;
    edge.value = c->value;
    edge.is_rising = is_rising;
    edge.is_falling = is_falling;

    for (size_t i = 0; i < c->n_listeners; i++) {
        c->listeners[i].fn(&edge, c->listeners[i].userdata);
    }
    return edge;
}

void clock_full_cycle(Clock *c, ClockEdge *rising_out, ClockEdge *falling_out) {
    ClockEdge r = clock_tick(c);
    ClockEdge f = clock_tick(c);
    if (rising_out) *rising_out = r;
    if (falling_out) *falling_out = f;
}

ClockEdge *clock_run(Clock *c, uint64_t cycles, size_t *count_out) {
    *count_out = 0;
    if (cycles == 0) return NULL;
    /* 2·cycles edges — guard the doubling and the allocation size. */
    if (cycles > ((size_t)-1) / 2 / sizeof(ClockEdge)) return NULL;
    size_t n = (size_t)(cycles * 2);
    ClockEdge *edges = malloc(n * sizeof(ClockEdge));
    if (!edges) return NULL;

    size_t w = 0;
    for (uint64_t i = 0; i < cycles; i++) {
        clock_full_cycle(c, &edges[w], &edges[w + 1]);
        w += 2;
    }
    *count_out = n;
    return edges;
}

int clock_register_listener(Clock *c, ClockListener fn, void *userdata) {
    if (c->n_listeners == c->cap_listeners) {
        size_t nc = c->cap_listeners ? c->cap_listeners * 2 : 4;
        if (c->cap_listeners > ((size_t)-1) / 2 / sizeof(Listener)) return -1;
        Listener *nl = realloc(c->listeners, nc * sizeof(Listener));
        if (!nl) return -1;
        c->listeners = nl;
        c->cap_listeners = nc;
    }
    c->listeners[c->n_listeners].fn = fn;
    c->listeners[c->n_listeners].userdata = userdata;
    c->n_listeners++;
    return 0;
}

void clock_reset(Clock *c) {
    c->cycle = 0;
    c->value = 0;
    c->total_ticks = 0;
}

double clock_period_ns(const Clock *c) {
    return 1e9 / (double)c->frequency_hz;
}
uint64_t clock_total_ticks(const Clock *c) { return c->total_ticks; }
uint64_t clock_frequency_hz(const Clock *c) { return c->frequency_hz; }
uint64_t clock_cycle(const Clock *c) { return c->cycle; }
uint8_t clock_value(const Clock *c) { return c->value; }

/* ===========================================================================
 *  ClockDivider
 * =========================================================================== */

struct ClockDivider {
    uint64_t divisor;
    uint64_t counter;
    Clock *output; /* owned */
};

ClockDivider *clock_divider_new(uint64_t source_frequency_hz, uint64_t divisor) {
    if (divisor < 2) return NULL; /* Rust panics */
    Clock *output = clock_new(source_frequency_hz / divisor); /* NULL if that is 0 */
    if (!output) return NULL;
    ClockDivider *d = calloc(1, sizeof *d);
    if (!d) {
        clock_free(output);
        return NULL;
    }
    d->divisor = divisor;
    d->output = output;
    return d;
}

void clock_divider_free(ClockDivider *d) {
    if (!d) return;
    clock_free(d->output);
    free(d);
}

void clock_divider_on_edge(ClockDivider *d, const ClockEdge *edge) {
    if (edge->is_rising) {
        d->counter++;
        if (d->counter >= d->divisor) {
            d->counter = 0;
            clock_tick(d->output); /* rising */
            clock_tick(d->output); /* falling */
        }
    }
}

const Clock *clock_divider_output(const ClockDivider *d) { return d->output; }
Clock *clock_divider_output_mut(ClockDivider *d) { return d->output; }

/* ===========================================================================
 *  MultiPhaseClock
 * =========================================================================== */

struct MultiPhaseClock {
    size_t phases;
    size_t active_phase;
    uint8_t *phase_values;
};

MultiPhaseClock *mpc_new(size_t phases) {
    if (phases < 2) return NULL; /* Rust panics */
    MultiPhaseClock *m = calloc(1, sizeof *m);
    if (!m) return NULL;
    m->phase_values = calloc(phases, sizeof(uint8_t)); /* checked multiply */
    if (!m->phase_values) {
        free(m);
        return NULL;
    }
    m->phases = phases;
    return m;
}

void mpc_free(MultiPhaseClock *m) {
    if (!m) return;
    free(m->phase_values);
    free(m);
}

void mpc_on_edge(MultiPhaseClock *m, const ClockEdge *edge) {
    if (edge->is_rising) {
        memset(m->phase_values, 0, m->phases * sizeof(uint8_t));
        m->phase_values[m->active_phase] = 1;
        m->active_phase = (m->active_phase + 1) % m->phases;
    }
}

uint8_t mpc_get_phase(const MultiPhaseClock *m, size_t index) {
    return index < m->phases ? m->phase_values[index] : 0;
}

size_t mpc_num_phases(const MultiPhaseClock *m) { return m->phases; }
