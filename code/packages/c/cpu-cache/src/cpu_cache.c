/*
 * cpu_cache.c — Configurable CPU cache hierarchy simulator, pure ISO C17.
 * =====================================================================
 *
 * See cpu_cache.h. A faithful port of the Rust `cpu-cache` crate: cache line →
 * set (N-way, LRU) → configurable level → multi-level hierarchy, with hit/miss
 * and eviction statistics. All arithmetic is exact integer bit-slicing; no
 * <math.h>.
 */
#include "cpu_cache.h"

#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy, memset, strncmp, strncpy */

/* Exact integer log2 for a power-of-two n (n >= 1). */
static uint32_t ilog2(size_t n) {
    uint32_t b = 0;
    while (n > 1) {
        n >>= 1;
        b++;
    }
    return b;
}

/* ── Cache line ─────────────────────────────────────────────────────────────*/
int ca_cache_line_init(CaCacheLine *line, size_t line_size) {
    line->data = (uint8_t *)calloc(line_size ? line_size : 1, 1);
    if (!line->data) {
        return 0;
    }
    line->valid = 0;
    line->dirty = 0;
    line->tag = 0;
    line->data_len = line_size;
    line->last_access = 0;
    return 1;
}

void ca_cache_line_free(CaCacheLine *line) {
    if (line) {
        free(line->data);
        line->data = NULL;
    }
}

void ca_cache_line_fill(CaCacheLine *line, uint64_t tag, const uint8_t *data,
                        uint64_t cycle) {
    line->valid = 1;
    line->dirty = 0; /* freshly loaded data is clean */
    line->tag = tag;
    if (line->data_len > 0) {
        memcpy(line->data, data, line->data_len); /* defensive copy */
    }
    line->last_access = cycle;
}

void ca_cache_line_touch(CaCacheLine *line, uint64_t cycle) {
    line->last_access = cycle;
}

void ca_cache_line_invalidate(CaCacheLine *line) {
    line->valid = 0;
    line->dirty = 0;
}

size_t ca_cache_line_size(const CaCacheLine *line) { return line->data_len; }

/* ── Cache configuration ────────────────────────────────────────────────────*/
int ca_cache_config_new(CaCacheConfig *out, const char *name, size_t total_size,
                        size_t line_size, size_t associativity,
                        uint64_t access_latency) {
    if (!name) {
        return 0; /* the Rust takes &str (never null); reject null defensively */
    }
    if (total_size == 0) {
        return 0;
    }
    if (line_size == 0 || (line_size & (line_size - 1)) != 0) {
        return 0; /* must be a positive power of 2 */
    }
    if (associativity == 0) {
        return 0;
    }
    /* Guard the product against size_t overflow before the divisibility test. */
    if (associativity > (size_t)-1 / line_size) {
        return 0;
    }
    if (total_size % (line_size * associativity) != 0) {
        return 0;
    }
    strncpy(out->name, name, sizeof out->name - 1);
    out->name[sizeof out->name - 1] = '\0';
    out->total_size = total_size;
    out->line_size = line_size;
    out->associativity = associativity;
    out->access_latency = access_latency;
    out->write_policy = CA_WRITE_BACK;
    return 1;
}

size_t ca_cache_config_num_lines(const CaCacheConfig *c) {
    return c->total_size / c->line_size;
}
size_t ca_cache_config_num_sets(const CaCacheConfig *c) {
    return ca_cache_config_num_lines(c) / c->associativity;
}

/* ── Cache set ──────────────────────────────────────────────────────────────*/
int ca_cache_set_init(CaCacheSet *set, size_t associativity, size_t line_size) {
    size_t i;
    set->lines = (CaCacheLine *)calloc(associativity, sizeof(CaCacheLine));
    if (!set->lines) {
        return 0;
    }
    for (i = 0; i < associativity; i++) {
        if (!ca_cache_line_init(&set->lines[i], line_size)) {
            /* Roll back the lines allocated so far. */
            size_t j;
            for (j = 0; j < i; j++) {
                ca_cache_line_free(&set->lines[j]);
            }
            free(set->lines);
            set->lines = NULL;
            return 0;
        }
    }
    set->num_ways = associativity;
    set->line_size = line_size;
    return 1;
}

void ca_cache_set_free(CaCacheSet *set) {
    size_t i;
    if (!set || !set->lines) {
        return;
    }
    for (i = 0; i < set->num_ways; i++) {
        ca_cache_line_free(&set->lines[i]);
    }
    free(set->lines);
    set->lines = NULL;
}

int ca_cache_set_lookup(const CaCacheSet *set, uint64_t tag, size_t *out_way) {
    size_t i;
    for (i = 0; i < set->num_ways; i++) {
        if (set->lines[i].valid && set->lines[i].tag == tag) {
            if (out_way) {
                *out_way = i;
            }
            return 1;
        }
    }
    return 0;
}

/* Least-recently-used way index; an invalid line is always preferred. */
static size_t find_lru(const CaCacheSet *set) {
    size_t best_index = 0, i;
    uint64_t best_time = (uint64_t)-1;
    for (i = 0; i < set->num_ways; i++) {
        if (!set->lines[i].valid) {
            return i;
        }
        if (set->lines[i].last_access < best_time) {
            best_time = set->lines[i].last_access;
            best_index = i;
        }
    }
    return best_index;
}

int ca_cache_set_access(CaCacheSet *set, uint64_t tag, uint64_t cycle,
                        size_t *out_index) {
    size_t way;
    if (ca_cache_set_lookup(set, tag, &way)) {
        ca_cache_line_touch(&set->lines[way], cycle);
        if (out_index) {
            *out_index = way;
        }
        return 1;
    }
    if (out_index) {
        *out_index = find_lru(set);
    }
    return 0;
}

int ca_cache_set_allocate(CaCacheSet *set, uint64_t tag, const uint8_t *data,
                          size_t data_len, uint64_t cycle, int *out_dirty,
                          uint64_t *out_tag, uint64_t *out_last_access) {
    size_t i, lru_index;
    CaCacheLine *victim;
    int evicted_dirty;
    (void)data_len;

    /* Step 1: use an invalid (empty) way if one exists. */
    for (i = 0; i < set->num_ways; i++) {
        if (!set->lines[i].valid) {
            ca_cache_line_fill(&set->lines[i], tag, data, cycle);
            return 0; /* no eviction */
        }
    }

    /* Step 2: all ways full — evict the LRU line. */
    lru_index = find_lru(set);
    victim = &set->lines[lru_index];

    /* Step 3: a dirty victim must be reported for writeback. */
    evicted_dirty = victim->dirty;
    if (evicted_dirty) {
        if (out_dirty) {
            *out_dirty = victim->dirty;
        }
        if (out_tag) {
            *out_tag = victim->tag;
        }
        if (out_last_access) {
            *out_last_access = victim->last_access;
        }
    }

    /* Step 4: overwrite the victim with the new data. */
    ca_cache_line_fill(&set->lines[lru_index], tag, data, cycle);
    return evicted_dirty ? 1 : 0;
}

/* ── Statistics ─────────────────────────────────────────────────────────────*/
void ca_cache_stats_init(CaCacheStats *s) {
    s->reads = s->writes = s->hits = s->misses = s->evictions = s->writebacks =
        0;
}
uint64_t ca_cache_stats_total_accesses(const CaCacheStats *s) {
    return s->reads + s->writes;
}
double ca_cache_stats_hit_rate(const CaCacheStats *s) {
    uint64_t total = ca_cache_stats_total_accesses(s);
    if (total == 0) {
        return 0.0;
    }
    return (double)s->hits / (double)total;
}
double ca_cache_stats_miss_rate(const CaCacheStats *s) {
    uint64_t total = ca_cache_stats_total_accesses(s);
    if (total == 0) {
        return 0.0;
    }
    return (double)s->misses / (double)total;
}
void ca_cache_stats_record_read(CaCacheStats *s, int hit) {
    s->reads++;
    if (hit) {
        s->hits++;
    } else {
        s->misses++;
    }
}
void ca_cache_stats_record_write(CaCacheStats *s, int hit) {
    s->writes++;
    if (hit) {
        s->hits++;
    } else {
        s->misses++;
    }
}
void ca_cache_stats_record_eviction(CaCacheStats *s, int dirty) {
    s->evictions++;
    if (dirty) {
        s->writebacks++;
    }
}
void ca_cache_stats_reset(CaCacheStats *s) { ca_cache_stats_init(s); }

/* ── Cache level ────────────────────────────────────────────────────────────*/
int ca_cache_init(CaCache *cache, const CaCacheConfig *config) {
    size_t num_sets = ca_cache_config_num_sets(config);
    size_t i;
    cache->config = *config;
    cache->sets = (CaCacheSet *)calloc(num_sets ? num_sets : 1,
                                       sizeof(CaCacheSet));
    if (!cache->sets) {
        return 0;
    }
    for (i = 0; i < num_sets; i++) {
        if (!ca_cache_set_init(&cache->sets[i], config->associativity,
                               config->line_size)) {
            size_t j;
            for (j = 0; j < i; j++) {
                ca_cache_set_free(&cache->sets[j]);
            }
            free(cache->sets);
            cache->sets = NULL;
            return 0;
        }
    }
    cache->num_sets = num_sets;
    ca_cache_stats_init(&cache->stats);
    cache->offset_bits = ilog2(config->line_size);
    cache->set_bits = num_sets > 1 ? ilog2(num_sets) : 0;
    cache->set_mask = num_sets > 0 ? (uint64_t)(num_sets - 1) : 0;
    return 1;
}

void ca_cache_free(CaCache *cache) {
    size_t i;
    if (!cache || !cache->sets) {
        return;
    }
    for (i = 0; i < cache->num_sets; i++) {
        ca_cache_set_free(&cache->sets[i]);
    }
    free(cache->sets);
    cache->sets = NULL;
}

void ca_cache_decompose(const CaCache *cache, uint64_t address, uint64_t *tag,
                        size_t *set_index, size_t *offset) {
    uint64_t off = address & (((uint64_t)1 << cache->offset_bits) - 1);
    uint64_t si = (address >> cache->offset_bits) & cache->set_mask;
    if (offset) {
        *offset = (size_t)off;
    }
    if (set_index) {
        *set_index = (size_t)si;
    }
    if (tag) {
        *tag = address >> (cache->offset_bits + cache->set_bits);
    }
}

/* True if every line in the set is valid — used for the Rust's clean-eviction
 * accounting heuristic (which slightly over-counts, replicated faithfully). */
static int all_lines_valid(const CaCacheSet *set) {
    size_t i;
    for (i = 0; i < set->num_ways; i++) {
        if (!set->lines[i].valid) {
            return 0;
        }
    }
    return 1;
}

CaCacheAccess ca_cache_read(CaCache *cache, uint64_t address, uint64_t cycle) {
    CaCacheAccess acc;
    uint64_t tag;
    size_t set_index, offset, idx;
    CaCacheSet *set;

    ca_cache_decompose(cache, address, &tag, &set_index, &offset);
    set = &cache->sets[set_index];

    acc.address = address;
    acc.tag = tag;
    acc.set_index = set_index;
    acc.offset = offset;
    acc.cycles = cache->config.access_latency;
    acc.has_evicted = 0;
    acc.evicted_dirty = 0;
    acc.evicted_tag = 0;
    acc.evicted_last_access = 0;

    if (ca_cache_set_access(set, tag, cycle, &idx)) {
        ca_cache_stats_record_read(&cache->stats, 1);
        acc.hit = 1;
        return acc;
    }

    /* Miss — allocate the line with dummy (zero) data. */
    ca_cache_stats_record_read(&cache->stats, 0);
    acc.hit = 0;
    {
        size_t ls = cache->config.line_size;
        uint8_t *dummy = (uint8_t *)calloc(ls ? ls : 1, 1);
        if (dummy) {
            int ed = 0;
            uint64_t et = 0, el = 0;
            int evicted = ca_cache_set_allocate(set, tag, dummy, ls, cycle, &ed,
                                                &et, &el);
            free(dummy);
            if (evicted) {
                ca_cache_stats_record_eviction(&cache->stats, 1);
                acc.has_evicted = 1;
                acc.evicted_dirty = 1;
                acc.evicted_tag = et;
                acc.evicted_last_access = el;
            } else if (all_lines_valid(set)) {
                ca_cache_stats_record_eviction(&cache->stats, 0);
            }
        }
    }
    return acc;
}

CaCacheAccess ca_cache_write(CaCache *cache, uint64_t address,
                             const uint8_t *data, size_t data_len,
                             uint64_t cycle) {
    CaCacheAccess acc;
    uint64_t tag;
    size_t set_index, offset, idx, i;
    CaCacheSet *set;

    ca_cache_decompose(cache, address, &tag, &set_index, &offset);
    set = &cache->sets[set_index];

    acc.address = address;
    acc.tag = tag;
    acc.set_index = set_index;
    acc.offset = offset;
    acc.cycles = cache->config.access_latency;
    acc.has_evicted = 0;
    acc.evicted_dirty = 0;
    acc.evicted_tag = 0;
    acc.evicted_last_access = 0;

    if (ca_cache_set_access(set, tag, cycle, &idx)) {
        CaCacheLine *line = &set->lines[idx];
        ca_cache_stats_record_write(&cache->stats, 1);
        for (i = 0; i < data_len; i++) {
            if (offset + i < line->data_len) {
                line->data[offset + i] = data[i];
            }
        }
        if (cache->config.write_policy == CA_WRITE_BACK) {
            line->dirty = 1;
        }
        acc.hit = 1;
        return acc;
    }

    /* Write miss — write-allocate, then write into the new line. */
    ca_cache_stats_record_write(&cache->stats, 0);
    acc.hit = 0;
    {
        size_t ls = cache->config.line_size;
        uint8_t *fill = (uint8_t *)calloc(ls ? ls : 1, 1);
        if (fill) {
            int ed = 0, evicted;
            uint64_t et = 0, el = 0;
            for (i = 0; i < data_len; i++) {
                if (offset + i < ls) {
                    fill[offset + i] = data[i];
                }
            }
            evicted = ca_cache_set_allocate(set, tag, fill, ls, cycle, &ed, &et,
                                            &el);
            free(fill);
            if (evicted) {
                ca_cache_stats_record_eviction(&cache->stats, 1);
                acc.has_evicted = 1;
                acc.evicted_dirty = 1;
                acc.evicted_tag = et;
                acc.evicted_last_access = el;
            } else if (all_lines_valid(set)) {
                ca_cache_stats_record_eviction(&cache->stats, 0);
            }
        }
    }

    /* Write-back: mark the freshly allocated line dirty. */
    if (cache->config.write_policy == CA_WRITE_BACK) {
        size_t new_idx;
        if (ca_cache_set_access(set, tag, cycle, &new_idx)) {
            set->lines[new_idx].dirty = 1;
        }
    }
    return acc;
}

void ca_cache_invalidate(CaCache *cache) {
    size_t s, w;
    for (s = 0; s < cache->num_sets; s++) {
        for (w = 0; w < cache->sets[s].num_ways; w++) {
            ca_cache_line_invalidate(&cache->sets[s].lines[w]);
        }
    }
}

int ca_cache_fill_line(CaCache *cache, uint64_t address, const uint8_t *data,
                       size_t data_len, uint64_t cycle, int *out_dirty,
                       uint64_t *out_tag, uint64_t *out_last_access) {
    uint64_t tag;
    size_t set_index;
    ca_cache_decompose(cache, address, &tag, &set_index, NULL);
    return ca_cache_set_allocate(&cache->sets[set_index], tag, data, data_len,
                                 cycle, out_dirty, out_tag, out_last_access);
}

/* ── Hierarchy ──────────────────────────────────────────────────────────────*/
static CaCache *move_to_heap(CaCache *src) {
    CaCache *heap;
    if (!src) {
        return NULL;
    }
    heap = (CaCache *)malloc(sizeof(CaCache));
    if (!heap) {
        return NULL; /* caller keeps ownership of src on OOM */
    }
    *heap = *src;
    memset(src, 0, sizeof *src); /* ownership transferred */
    return heap;
}

void ca_cache_hierarchy_init(CaCacheHierarchy *h, CaCache *l1i, CaCache *l1d,
                             CaCache *l2, CaCache *l3,
                             uint64_t main_memory_latency) {
    h->l1i = move_to_heap(l1i);
    h->l1d = move_to_heap(l1d);
    h->l2 = move_to_heap(l2);
    h->l3 = move_to_heap(l3);
    h->main_memory_latency = main_memory_latency;
}

void ca_cache_hierarchy_free(CaCacheHierarchy *h) {
    CaCache *levels[4];
    int i;
    if (!h) {
        return;
    }
    levels[0] = h->l1i;
    levels[1] = h->l1d;
    levels[2] = h->l2;
    levels[3] = h->l3;
    for (i = 0; i < 4; i++) {
        if (levels[i]) {
            ca_cache_free(levels[i]);
            free(levels[i]);
        }
    }
    h->l1i = h->l1d = h->l2 = h->l3 = NULL;
}

/* Build the ordered list of levels to walk. Writes up to 3 (cache, name)
 * pairs and returns the count. */
static size_t build_level_order(CaCacheHierarchy *h, int is_instruction,
                                CaCache *caches[3], const char *names[3]) {
    size_t n = 0;
    if (is_instruction) {
        if (h->l1i) {
            caches[n] = h->l1i;
            names[n] = "L1I";
            n++;
        }
    } else if (h->l1d) {
        caches[n] = h->l1d;
        names[n] = "L1D";
        n++;
    }
    if (h->l2) {
        caches[n] = h->l2;
        names[n] = "L2";
        n++;
    }
    if (h->l3) {
        caches[n] = h->l3;
        names[n] = "L3";
        n++;
    }
    return n;
}

static void set_served_by(CaHierarchyAccess *r, const char *name) {
    strncpy(r->served_by, name, sizeof r->served_by - 1);
    r->served_by[sizeof r->served_by - 1] = '\0';
}

CaHierarchyAccess ca_cache_hierarchy_read(CaCacheHierarchy *h, uint64_t address,
                                          int is_instruction, uint64_t cycle) {
    CaHierarchyAccess r;
    CaCache *caches[3];
    const char *names[3];
    size_t n, i, hit_level, line_size;
    uint64_t total = 0;

    r.address = address;
    r.level_count = 0;
    n = build_level_order(h, is_instruction, caches, names);

    if (n == 0) {
        set_served_by(&r, "memory");
        r.total_cycles = h->main_memory_latency;
        r.hit_at_level = 0;
        return r;
    }

    set_served_by(&r, "memory");
    hit_level = n;
    for (i = 0; i < n; i++) {
        CaCacheAccess a = ca_cache_read(caches[i], address, cycle);
        total += caches[i]->config.access_latency;
        r.level_accesses[r.level_count++] = a;
        if (a.hit) {
            set_served_by(&r, names[i]);
            hit_level = i;
            break;
        }
    }

    if (strncmp(r.served_by, "memory", sizeof r.served_by) == 0) {
        total += h->main_memory_latency;
    }

    /* Inclusive fill: install the line in every level above where it hit. */
    line_size = caches[0]->config.line_size;
    {
        uint8_t *dummy = (uint8_t *)calloc(line_size ? line_size : 1, 1);
        if (dummy) {
            size_t fi = hit_level;
            while (fi > 0) {
                fi--;
                ca_cache_fill_line(caches[fi], address, dummy, line_size, cycle,
                                   NULL, NULL, NULL);
            }
            free(dummy);
        }
    }

    r.total_cycles = total;
    r.hit_at_level = hit_level;
    return r;
}

CaHierarchyAccess ca_cache_hierarchy_write(CaCacheHierarchy *h, uint64_t address,
                                           const uint8_t *data, size_t data_len,
                                           uint64_t cycle) {
    CaHierarchyAccess r;
    CaCache *caches[3];
    const char *names[3];
    size_t n, i, hit_level;
    uint64_t total;
    CaCacheAccess first;

    r.address = address;
    r.level_count = 0;
    n = build_level_order(h, 0, caches, names);

    if (n == 0) {
        set_served_by(&r, "memory");
        r.total_cycles = h->main_memory_latency;
        r.hit_at_level = 0;
        return r;
    }

    first = ca_cache_write(caches[0], address, data, data_len, cycle);
    total = caches[0]->config.access_latency;
    r.level_accesses[r.level_count++] = first;

    if (first.hit) {
        set_served_by(&r, names[0]);
        r.total_cycles = total;
        r.hit_at_level = 0;
        return r;
    }

    set_served_by(&r, "memory");
    hit_level = n;
    for (i = 1; i < n; i++) {
        CaCacheAccess a = ca_cache_read(caches[i], address, cycle);
        total += caches[i]->config.access_latency;
        r.level_accesses[r.level_count++] = a;
        if (a.hit) {
            set_served_by(&r, names[i]);
            hit_level = i;
            break;
        }
    }

    if (strncmp(r.served_by, "memory", sizeof r.served_by) == 0) {
        total += h->main_memory_latency;
    }

    r.total_cycles = total;
    r.hit_at_level = hit_level;
    return r;
}

void ca_cache_hierarchy_invalidate_all(CaCacheHierarchy *h) {
    if (h->l1i) {
        ca_cache_invalidate(h->l1i);
    }
    if (h->l1d) {
        ca_cache_invalidate(h->l1d);
    }
    if (h->l2) {
        ca_cache_invalidate(h->l2);
    }
    if (h->l3) {
        ca_cache_invalidate(h->l3);
    }
}

void ca_cache_hierarchy_reset_stats(CaCacheHierarchy *h) {
    if (h->l1i) {
        ca_cache_stats_reset(&h->l1i->stats);
    }
    if (h->l1d) {
        ca_cache_stats_reset(&h->l1d->stats);
    }
    if (h->l2) {
        ca_cache_stats_reset(&h->l2->stats);
    }
    if (h->l3) {
        ca_cache_stats_reset(&h->l3->stats);
    }
}
