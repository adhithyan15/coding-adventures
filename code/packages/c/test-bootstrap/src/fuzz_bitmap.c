/*
 * fuzz_bitmap.c -- coverage bookkeeping and the power schedule (VER06 section 4).
 *
 * THE MAP
 * -------
 * 65,536 bits in 8,192 bytes: bit e set means "edge e ran".  An edge id is
 * reduced modulo 65,536, so two program points can share a bit (AFL pays the
 * same price for a fixed-size map).
 *
 *      edge 70000  ->  70000 mod 65536 = 4464  ->  byte 558, bit 0
 *
 * The map is paired with the list of ids in the order first seen, so listing
 * the edges of ONE execution costs O(edges hit), not a scan of 8,192 bytes, and
 * clearing it costs the same.
 *
 * ENERGY
 * ------
 * How many mutations a seed gets per turn.  Short seeds that found a big share
 * of the coverage get the most:
 *
 *      energy = max(1, round(base * (1 + new/total) / sqrt(len)))
 *
 * Worked example: base 10, len 4, 3 new of 6 total: 10 * 1.5 / 2 = 7.5, which
 * rounds HALF AWAY FROM ZERO to 8 (floor(7.5 + 0.5)).  The 0.5 tie is the
 * place lanes drift, so the tests pin it.
 */
#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"
#include "pbt_internal.h"

void fz_bitmap_init(fz_bitmap *b)
{
    memset(b->m, 0, sizeof b->m);
    b->nedges = 0;
}

void fz_bitmap_record(fz_bitmap *b, uint64_t edge)
{
    uint64_t e = edge % FZ_COVERAGE_MAP_SIZE;
    uint8_t mask = (uint8_t)(1u << (e % 8));
    if ((b->m[e / 8] & mask) == 0) {
        /* Defence in depth: a bit can be clear only for an edge not yet listed, so single-threaded the list can never
         * outgrow the map.  A target thread racing this call (misuse, VER06j section 5.1) could push nedges past it;
         * refuse rather than write past edges[]. */
        if (b->nedges >= FZ_COVERAGE_MAP_SIZE) {
            return;
        }
        b->m[e / 8] = (uint8_t)(b->m[e / 8] | mask);
        b->edges[b->nedges++] = (uint16_t)e;
    }
}

int fz_bitmap_has(const fz_bitmap *b, uint64_t edge)
{
    uint64_t e = edge % FZ_COVERAGE_MAP_SIZE;
    return (b->m[e / 8] & (1u << (e % 8))) != 0;
}

int fz_bitmap_admit(fz_bitmap *b, const uint16_t *trace, size_t n, size_t *count_new)
{
    size_t fresh = 0;
    size_t i;
    for (i = 0; i < n; i++) {
        if (!fz_bitmap_has(b, trace[i])) {
            fz_bitmap_record(b, trace[i]);
            fresh++;
        }
    }
    if (count_new != NULL) {
        *count_new = fresh;
    }
    return fresh > 0;
}

size_t fz_bitmap_count(const fz_bitmap *b)
{
    size_t total = 0;
    size_t i;
    for (i = 0; i < FZ_COVERAGE_BYTES; i++) {
        unsigned v = b->m[i];
        while (v != 0) {
            total += v & 1u;
            v >>= 1;
        }
    }
    return total;
}

size_t fz_bitmap_edges(const fz_bitmap *b, uint16_t *out)
{
    size_t n = 0;
    size_t i;
    unsigned bit;
    for (i = 0; i < FZ_COVERAGE_BYTES; i++) {
        if (b->m[i] == 0) {
            continue;
        }
        for (bit = 0; bit < 8; bit++) {
            if (b->m[i] & (1u << bit)) {
                out[n++] = (uint16_t)(i * 8 + bit);
            }
        }
    }
    return n;
}

void fz_bitmap_merge(fz_bitmap *b, const fz_bitmap *o)
{
    size_t i;
    for (i = 0; i < o->nedges; i++) {
        fz_bitmap_record(b, o->edges[i]);
    }
}

void fz_bitmap_reset(fz_bitmap *b)
{
    size_t i;
    for (i = 0; i < b->nedges; i++) {
        b->m[b->edges[i] / 8] = (uint8_t)(b->m[b->edges[i] / 8] & ~(1u << (b->edges[i] % 8)));
    }
    b->nedges = 0;
}

uint64_t fz_edge_id(const char *label)
{
    return pbt_fnv1a64(label, strlen(label));
}

/* ---- corpus admission --------------------------------------------------------------------------------- */

static int cmp_u64(const void *a, const void *b)
{
    uint64_t x = *(const uint64_t *)a;
    uint64_t y = *(const uint64_t *)b;
    return x < y ? -1 : (x > y ? 1 : 0);
}

static int in_sorted(const uint64_t *v, size_t n, uint64_t x)
{
    size_t lo = 0;
    size_t hi = n;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (v[mid] < x) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    return lo < n && v[lo] == x;
}

/*
 * Does the trace hold an edge absent from `existing`?  new_edges counts UNIQUE novel edges (a trace that
 * repeats one novel edge three times adds one).
 *
 *     admit([1 2 3], [2 4]) -> (yes, 1)      edge 4 is new
 *     admit([1 2 3], [1 2]) -> (no, 0)
 */
int fz_corpus_admit(const uint64_t *existing, size_t ne, const uint64_t *trace, size_t nt, int *admitted,
                    size_t *new_edges)
{
    uint64_t *have = (uint64_t *)pbt_alloc(ne, sizeof(uint64_t));
    uint64_t *t = (uint64_t *)pbt_alloc(nt, sizeof(uint64_t));
    size_t novel = 0;
    size_t i;
    *admitted = 0;
    *new_edges = 0;
    if (have == NULL || t == NULL) {
        pbt_free(have);
        pbt_free(t);
        return FZ_E_NOMEM;
    }
    if (ne > 0) {
        memcpy(have, existing, ne * sizeof(uint64_t));
        qsort(have, ne, sizeof(uint64_t), cmp_u64);
    }
    if (nt > 0) {
        memcpy(t, trace, nt * sizeof(uint64_t));
        qsort(t, nt, sizeof(uint64_t), cmp_u64);
    }
    for (i = 0; i < nt; i++) {
        if (i > 0 && t[i] == t[i - 1]) {
            continue; /* count each novel edge once */
        }
        if (!in_sorted(have, ne, t[i])) {
            novel++;
        }
    }
    pbt_free(have);
    pbt_free(t);
    *admitted = novel > 0;
    *new_edges = novel;
    return FZ_OK;
}

/* ---- energy ---------------------------------------------------------------------------------------- */

size_t fz_calculate_energy(size_t base_energy, size_t seed_len, size_t new_edges, size_t total_edges)
{
    double base = (double)(base_energy < 1 ? 1 : base_energy);
    double bonus = 1.0;
    double factor = sqrt((double)(seed_len < 1 ? 1 : seed_len));
    double x;
    double energy;
    if (total_edges > 0) {
        bonus = 1.0 + (double)new_edges / (double)total_edges;
    }
    x = base * bonus / factor;
    /* Round half away from zero (x is positive); the sabotage seam replaces the rounding. */
    energy = fz_hooks.energy_round != NULL ? fz_hooks.energy_round(x) : floor(x + 0.5);
    /* Clamp BEFORE the integer conversion (a huge double converts to an unspecified integer). */
    if (!(energy < (double)FZ_MAX_ITERATIONS)) {
        energy = (double)FZ_MAX_ITERATIONS;
    }
    if (energy < 1.0) {
        energy = 1.0;
    }
    return (size_t)energy;
}
