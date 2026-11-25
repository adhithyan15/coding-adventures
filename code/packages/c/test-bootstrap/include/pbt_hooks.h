/*
 * pbt_hooks.h -- test seams (VER05i section 9).  NOT part of the stable API.
 * ===========================================================================
 *
 * A conformance run that cannot fail proves nothing (the LH-003 false pass).
 * These seams let a test deliberately BREAK one engine decision and watch the
 * corpus run notice:
 *
 *   bisect        replaces the bisection move  current - floor(diff/2)
 *   unit          replaces the unit move       current - sign(diff)
 *   skip_deletion disables phase 2 (single deletion) of sequence shrinking
 *   threshold     replaces the rejection threshold (0 - n) % n
 *   seed_map      maps every seed before the stream is created (a shifted RNG)
 *
 * and watch the memory discipline:
 *
 *   live_allocs / alloc_count  engine allocations outstanding / ever made
 *   fail_after                 0 = never; N>0 = the N-th allocation FROM NOW
 *                              returns NULL (and every later one succeeds)
 *   injected                   how many allocations fail_after has failed
 *
 * plus the budget observables of Java's ShrinkBudget:
 *
 *   default_max_work  0 = PBT_MAX_SHRINK_WORK; otherwise the work cap of a
 *                     fresh root
 *   last_work / last_attempts  totals of the most recently closed owning scope
 *
 * This is the only mutable global state in the package.  Production code only
 * READS the function pointers; tests call pbt_hooks_reset() when they finish.
 */
#ifndef PBT_HOOKS_H
#define PBT_HOOKS_H

#include <stdint.h>

#include "pbt_big.h"

typedef struct pbt_hooks_t {
    void (*bisect)(pbt_big *out, const pbt_big *current, const pbt_big *diff);
    void (*unit)(pbt_big *out, const pbt_big *current, const pbt_big *diff);
    int skip_deletion;
    uint64_t (*threshold)(uint64_t n);
    uint64_t (*seed_map)(uint64_t seed);

    uint64_t default_max_work;
    uint64_t last_work;
    uint64_t last_attempts;

    long live_allocs;
    long alloc_count;
    long fail_after;
    long injected; /* allocations failed by fail_after so far */
} pbt_hooks_t;

extern pbt_hooks_t pbt_hooks;

/* Restore every seam to its production value.  The live/total allocation counters are accounting and are kept. */
void pbt_hooks_reset(void);

#endif /* PBT_HOOKS_H */
