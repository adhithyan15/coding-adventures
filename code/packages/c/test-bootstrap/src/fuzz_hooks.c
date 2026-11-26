/*
 * fuzz_hooks.c -- the fuzz engine's test-seam state (see fuzz_hooks.h).
 *
 * This is the only mutable global state of the fuzz engine besides the handle ring.
 */
#include "fuzz_hooks.h"

#include <string.h>

fz_hooks_t fz_hooks;

void fz_hooks_reset(void)
{
    memset(&fz_hooks, 0, sizeof fz_hooks);
}
