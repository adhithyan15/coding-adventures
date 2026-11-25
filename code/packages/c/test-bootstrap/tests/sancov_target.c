/*
 * sancov_target.c -- the one translation unit compiled with -fsanitize-coverage=trace-pc-guard.
 *
 * It never calls fz_edge: every edge the engine sees comes from the compiler's instrumentation of THIS file.
 * Staircase: bytes ff 80 7f in order; each correct byte is a new basic block.
 */
#include <stddef.h>
#include <stdint.h>

#include "fuzz.h"

int sancov_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out);

int sancov_target(const uint8_t *d, size_t n, fz_handle *h, fz_crash *out)
{
    (void)h;
    if (n > 0 && d[0] == 0xFF) {
        if (n > 1 && d[1] == 0x80) {
            if (n > 2 && d[2] == 0x7F) {
                fz_crash_set(out, "sancov-staircase");
                fz_crash_push(out, "stage3");
                return 1;
            }
            return 0;
        }
        return 0;
    }
    return 0;
}
