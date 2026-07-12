/*
 * zeroize.c — implementation of the secure-wipe primitives.
 * ===========================================================================
 * The one load-bearing idea: write zeros through a `volatile unsigned char *`.
 * A volatile store is observable behavior the compiler may not remove, so the
 * clear survives optimization even when no later read is visible.
 */
#include "zeroize.h"

#include <stdlib.h>

void zeroize_bytes(void *ptr, size_t len) {
    if (len == 0) {
        return; /* nothing to do (ptr may legitimately be NULL) */
    }
    volatile unsigned char *p = (volatile unsigned char *)ptr;
    size_t i;
    for (i = 0; i < len; i++) {
        p[i] = 0u;
    }
}

void zeroize_object(void *ptr, size_t len) { zeroize_bytes(ptr, len); }

void zeroize_u8(uint8_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_u16(uint16_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_u32(uint32_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_u64(uint64_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_i8(int8_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_i16(int16_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_i32(int32_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_i64(int64_t *p) { zeroize_bytes(p, sizeof *p); }
void zeroize_size(size_t *p) { zeroize_bytes(p, sizeof *p); }

/* ---------------------------------------------------------------------------
 *  ZrBytes — a capacity-scrubbing byte buffer
 * ------------------------------------------------------------------------- */

void zr_bytes_init(ZrBytes *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
}

int zr_bytes_reserve(ZrBytes *b, size_t additional) {
    if (additional == 0) {
        return 0;
    }
    if (b->len > ((size_t)-1) - additional) {
        return -1; /* len + additional overflows */
    }
    size_t need = b->len + additional;
    if (need <= b->cap) {
        return 0;
    }
    size_t nc = b->cap ? b->cap : 8;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2) {
            return -1;
        }
        nc *= 2;
    }
    unsigned char *nd = realloc(b->data, nc);
    if (!nd) {
        return -1;
    }
    b->data = nd;
    b->cap = nc;
    return 0;
}

int zr_bytes_push(ZrBytes *b, unsigned char byte) {
    if (zr_bytes_reserve(b, 1) != 0) {
        return -1;
    }
    b->data[b->len++] = byte;
    return 0;
}

int zr_bytes_extend(ZrBytes *b, const unsigned char *src, size_t n) {
    if (n == 0) {
        return 0;
    }
    if (zr_bytes_reserve(b, n) != 0) {
        return -1;
    }
    size_t i;
    for (i = 0; i < n; i++) {
        b->data[b->len + i] = src[i];
    }
    b->len += n;
    return 0;
}

void zr_bytes_zeroize(ZrBytes *b) {
    /* Scrub the full allocated capacity, not just the live prefix: growth may
     * have left stale secret bytes in the unused tail. */
    if (b->data && b->cap) {
        zeroize_bytes(b->data, b->cap);
    }
    b->len = 0;
}

void zr_bytes_free(ZrBytes *b) {
    if (!b) {
        return;
    }
    free(b->data);
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
}
