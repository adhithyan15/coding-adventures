/*
 * fuzz_internal.h -- the handle structure and its issue/retire pair, shared by fuzz_engine.c and the tests
 * (which live beside the sources and include "src").  Not part of the public API.
 */
#ifndef FUZZ_INTERNAL_H
#define FUZZ_INTERNAL_H

#include "fuzz.h"

struct fz_handle {
    int live;      /* 1 from issue until retire */
    int exhausted; /* 0 none, 1 steps (timeout), 2 memory (oom): the budget that broke FIRST, sticky */
    int sancov;    /* an edge arrived through the sanitizer-coverage callbacks */
    uint64_t epoch;
    uint64_t steps;
    uint64_t step_budget;
    uint64_t reserved;
    uint64_t mem_budget;
    fz_bitmap *cov; /* NULL once retired */
    void *user;
};

/* Take a FRESH handle from the ring (a slot that is not live).  NULL when every slot is live. */
fz_handle *fz_handle_issue(fz_bitmap *cov, uint64_t step_budget, uint64_t mem_budget, void *user);
/* Retire: no later call on it counts.  Returns the exhaustion state (0, 1 or 2); *sancov (may be NULL) gets the flag. */
int fz_handle_retire(fz_handle *h, int *sancov);
/* Make h the active handle until the matching restore; returns the previous one. */
fz_handle *fz_handle_set_active(fz_handle *h);
/* Handles currently live (tests). */
size_t fz_handles_live(void);

#endif /* FUZZ_INTERNAL_H */
