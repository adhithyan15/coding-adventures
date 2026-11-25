/*
 * pbt_hooks.c -- the allocation wrappers and the test-seam state.
 *
 * Live allocations are counted without any header: the counter is incremented
 * on a successful alloc and decremented on free of a non-NULL pointer.
 * pbt_realloc keeps the count unchanged for a live block and counts a fresh
 * one when handed NULL.
 */
#include "pbt_hooks.h"

#include <stdlib.h>
#include <string.h>

#include "pbt_internal.h"

pbt_hooks_t pbt_hooks;

void pbt_hooks_reset(void)
{
    /* The allocation COUNTERS are accounting, not seams: a reset must not lose track of live blocks. */
    long live = pbt_hooks.live_allocs;
    long count = pbt_hooks.alloc_count;
    long injected = pbt_hooks.injected;
    memset(&pbt_hooks, 0, sizeof pbt_hooks);
    pbt_hooks.injected = injected;
    pbt_hooks.live_allocs = live;
    pbt_hooks.alloc_count = count;
}

size_t pbt_mul_size(size_t a, size_t b, int *ok)
{
    if (a != 0 && b > (size_t)-1 / a) {
        *ok = 0;
        return 0;
    }
    return a * b;
}

size_t pbt_add_size(size_t a, size_t b, int *ok)
{
    if (b > (size_t)-1 - a) {
        *ok = 0;
        return 0;
    }
    return a + b;
}

/* True when the failure-injection counter says this allocation must fail. */
static int inject_failure(void)
{
    if (pbt_hooks.fail_after > 0) {
        pbt_hooks.fail_after--;
        if (pbt_hooks.fail_after == 0) {
            pbt_hooks.injected++;
            return 1;
        }
    }
    return 0;
}

void *pbt_alloc(size_t count, size_t width)
{
    int ok = 1;
    size_t bytes = pbt_mul_size(count, width, &ok);
    void *p;
    if (!ok || inject_failure()) {
        return NULL;
    }
    p = malloc(bytes == 0 ? 1 : bytes);
    if (p != NULL) {
        pbt_hooks.live_allocs++;
        pbt_hooks.alloc_count++;
    }
    return p;
}

void *pbt_realloc(void *old, size_t count, size_t width)
{
    int ok = 1;
    size_t bytes = pbt_mul_size(count, width, &ok);
    void *p;
    if (!ok || inject_failure()) {
        return NULL;
    }
    p = realloc(old, bytes == 0 ? 1 : bytes);
    if (p != NULL && old == NULL) {
        pbt_hooks.live_allocs++;
        pbt_hooks.alloc_count++;
    }
    return p;
}

void pbt_free(void *p)
{
    if (p != NULL) {
        pbt_hooks.live_allocs--;
        free(p);
    }
}
