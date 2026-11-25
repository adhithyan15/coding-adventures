/*
 * pbt_internal.h -- allocation discipline shared by the engine's .c files.
 *
 * Every allocation in the package goes through these three functions so that
 *   1. a NULL result is always handled by the caller,
 *   2. every `count * width` is overflow-checked in ONE place,
 *   3. the tests can count live allocations (leak check) and inject a failure
 *      at the N-th allocation (out-of-memory check) via pbt_hooks.
 */
#ifndef PBT_INTERNAL_H
#define PBT_INTERNAL_H

#include <stddef.h>

/* malloc(count * width) with overflow check; NULL on overflow or failure.  A zero size returns a 1-byte block. */
void *pbt_alloc(size_t count, size_t width);
/* Grow/shrink to count * width; on failure the old block is untouched and NULL is returned. */
void *pbt_realloc(void *p, size_t count, size_t width);
void pbt_free(void *p);

/* a * b, or 0 with *ok = 0 on overflow. */
size_t pbt_mul_size(size_t a, size_t b, int *ok);
/* a + b, or 0 with *ok = 0 on overflow. */
size_t pbt_add_size(size_t a, size_t b, int *ok);

/*
 * Length (1-4) of the strictly valid UTF-8 sequence at s[0..avail), or 0 when it
 * is invalid: no overlong forms, no surrogates, nothing above U+10FFFF.
 */
size_t pbt_utf8_len(const unsigned char *s, size_t avail);

#endif /* PBT_INTERNAL_H */
