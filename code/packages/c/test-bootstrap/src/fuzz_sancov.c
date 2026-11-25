/*
 * fuzz_sancov.c -- OPTIONAL real coverage from the compiler (VER06j section 4.3).  NOT ISO C, NOT in the pure-ISO build.
 * ===========================================================================
 *
 * clang (and recent gcc) can instrument a translation unit so that EVERY basic-block edge calls back into the
 * program:
 *
 *      clang -fsanitize-coverage=trace-pc-guard  target.c
 *
 * The instrumented code owns an array of 32-bit "guards", one per edge, and calls
 *
 *      __sanitizer_cov_trace_pc_guard_init(start, stop)   once per module, before main
 *      __sanitizer_cov_trace_pc_guard(&guard)             every time that edge runs
 *
 * These two functions are that interface.  This file defines them, and nothing else, so linking it is the opt-in:
 *
 *   - the init callback numbers the guards 1, 2, 3 ... in the order the runtime hands them over (0 means "not yet
 *     numbered"), which is deterministic for a given binary;
 *   - the guard callback maps guard id g to an edge of the 65,536-edge map, (g * 0x9E3779B97F4A7C15) >> 48 (the top 16
 *     bits of a multiplicative hash, so neighbouring guards land far apart), and records it in the ACTIVE handle,
 *     the handle of the execution in progress.  Outside a target run there is no active handle and the callback does
 *     nothing, so instrumented code is harmless when run by anything else.
 *
 * KEEP THE ENGINE OUT OF IT.  Only the translation unit that holds the TARGET is compiled with the flag; the engine's
 * own branches would otherwise flood the map with edges that say nothing about the target.  fz_guard_count() lets a
 * test prove it: it counts the guards handed over, which must be only the target's.
 */
#include <stdint.h>

#include "fuzz.h"

static uint32_t g_next_guard = 1;
static uint32_t g_guards;

void __sanitizer_cov_trace_pc_guard_init(uint32_t *start, uint32_t *stop);
void __sanitizer_cov_trace_pc_guard(uint32_t *guard);
uint32_t fz_guard_count(void);

void __sanitizer_cov_trace_pc_guard_init(uint32_t *start, uint32_t *stop)
{
    uint32_t *g;
    if (start == stop || *start != 0) {
        return; /* an empty module, or one numbered already */
    }
    for (g = start; g < stop; g++) {
        *g = g_next_guard++;
        g_guards++;
    }
}

void __sanitizer_cov_trace_pc_guard(uint32_t *guard)
{
    /* Re-entrancy guard: if the engine were instrumented by mistake, the callback's own code would call the callback
     * again, forever.  Edges that arrive while one is being handled are ignored. */
    static int busy;
    fz_handle *h;
    if (busy) {
        return;
    }
    busy = 1;
    h = fz_active_handle();
    if (h != NULL && *guard != 0) {
        fz_edge_sancov(h, *guard);
    }
    busy = 0;
}

uint32_t fz_guard_count(void)
{
    return g_guards;
}
