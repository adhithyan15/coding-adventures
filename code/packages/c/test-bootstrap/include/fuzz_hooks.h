/*
 * fuzz_hooks.h -- test seams of the fuzz engine (VER06j section 8).  NOT part of the stable API.
 * ===========================================================================
 *
 * A conformance run that cannot fail proves nothing.  These seams let a test deliberately BREAK one
 * engine decision and watch the corpus runner (or a pinned test) notice.  Every pointer is NULL (or 0)
 * in production; production code only READS them.  Tests call fz_hooks_reset() when they finish.
 *
 * Allocation failure injection and live-allocation counting are the SHARED ones of pbt_hooks.h
 * (pbt_hooks.fail_after, pbt_hooks.live_allocs): the fuzz engine allocates through pbt_alloc.
 */
#ifndef FUZZ_HOOKS_H
#define FUZZ_HOOKS_H

#include "fuzz.h"

typedef struct fz_hooks_t {
    /* the corpus runner calls these instead of the real _big mutators when non-NULL */
    int (*bit_flip)(fz_bytes *, const uint8_t *, size_t, const pbt_big *, const pbt_big *);
    int (*byte_flip)(fz_bytes *, const uint8_t *, size_t, const pbt_big *, const pbt_big *);
    int (*byte_arithmetic)(fz_bytes *, const uint8_t *, size_t, const pbt_big *, const pbt_big *, int, fz_endian);
    int (*boundary_insert)(fz_bytes *, const uint8_t *, size_t, const pbt_big *, int, const pbt_big *, fz_endian);
    int (*block_delete)(fz_bytes *, const uint8_t *, size_t, const pbt_big *, const pbt_big *);
    int (*block_insert)(fz_bytes *, const uint8_t *, size_t, const pbt_big *, const uint8_t *, size_t);
    int (*block_duplicate)(fz_bytes *, const uint8_t *, size_t, const pbt_big *, const pbt_big *, const pbt_big *);
    int (*splice)(fz_bytes *, const uint8_t *, size_t, const uint8_t *, size_t, const pbt_big *, const pbt_big *);
    int (*mutate_sequence)(fz_bytes *, const uint8_t *, size_t, uint64_t, size_t, size_t, uint64_t, uint64_t *);
    int (*corpus_admit)(const uint64_t *, size_t, const uint64_t *, size_t, int *, size_t *);
    int (*is_duplicate)(fz_str, const fz_str *, size_t, fz_str, const fz_str *, size_t);
    int (*is_failure_id)(const char *, size_t);
    int (*format_tap)(char *, size_t, int, const char *, size_t, const char *, size_t, const char *, size_t);
    int (*tfv_validate)(const char *, size_t, char *, size_t);

    /* replacement tables for mutate_sequence (5 interest values, 6 deltas) */
    const uint8_t *interest8;
    const int *deltas8;
    /* replaces floor(x + 0.5) in the power schedule */
    double (*energy_round)(double);
    /* applied to the 64-bit hash before its low 32 bits are taken */
    uint64_t (*sig_post)(uint64_t);

    /* cap overrides for tests (0 = production value) */
    size_t max_crashes;
    uint64_t max_retained_bytes;
} fz_hooks_t;

extern fz_hooks_t fz_hooks;

/* Restore every seam to its production value. */
void fz_hooks_reset(void);

#endif /* FUZZ_HOOKS_H */
