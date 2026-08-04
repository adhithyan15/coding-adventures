/*
 * Tests for cpu-cache, mirroring the Rust crate's unit tests across all five
 * modules (cache_line, cache_set, cache, stats, hierarchy), using the
 * header-only iso_test.h harness (pure ISO C17).
 */
#include "iso_test.h"

#include "cpu_cache.h"

#include <string.h> /* strcmp, memset */

#define EPS 1e-9

/* Convenience: an L1D like the Rust test helper make_l1d(). */
static void make_l1d(CaCache *c) {
    CaCacheConfig cfg;
    ISO_CHECK(ca_cache_config_new(&cfg, "L1D", 1024, 64, 4, 1));
    ISO_CHECK(ca_cache_init(c, &cfg));
}
static void make_l2(CaCache *c) {
    CaCacheConfig cfg;
    ISO_CHECK(ca_cache_config_new(&cfg, "L2", 4096, 64, 8, 10));
    ISO_CHECK(ca_cache_init(c, &cfg));
}
static void make_l3(CaCache *c) {
    CaCacheConfig cfg;
    ISO_CHECK(ca_cache_config_new(&cfg, "L3", 16384, 64, 16, 30));
    ISO_CHECK(ca_cache_init(c, &cfg));
}

int main(void) {
    /* ══ CacheLine ══════════════════════════════════════════════════════ */
    {
        CaCacheLine line;
        ISO_CHECK(ca_cache_line_init(&line, 64));
        ISO_CHECK(!line.valid);
        ISO_CHECK(!line.dirty);
        ISO_CHECK_EQ_UINT(line.tag, 0);
        ISO_CHECK_EQ_UINT(line.last_access, 0);
        ISO_CHECK_EQ_UINT(line.data_len, 64);
        ISO_CHECK_EQ_UINT(ca_cache_line_size(&line), 64);
        ca_cache_line_free(&line);
    }
    {
        CaCacheLine line;
        uint8_t data[64];
        memset(data, 0xAB, sizeof data);
        ca_cache_line_init(&line, 64);
        ca_cache_line_fill(&line, 42, data, 100);
        ISO_CHECK(line.valid);
        ISO_CHECK(!line.dirty);
        ISO_CHECK_EQ_UINT(line.tag, 42);
        ISO_CHECK_EQ_UINT(line.last_access, 100);
        ISO_CHECK_MEM_EQ(line.data, data, 64);
        ca_cache_line_free(&line);
    }
    { /* fill is a defensive copy */
        CaCacheLine line;
        uint8_t data[4] = {1, 2, 3, 4};
        ca_cache_line_init(&line, 4);
        ca_cache_line_fill(&line, 1, data, 0);
        data[0] = 99;
        ISO_CHECK_EQ_INT(line.data[0], 1);
        ca_cache_line_free(&line);
    }
    { /* touch updates LRU */
        CaCacheLine line;
        uint8_t z[64] = {0};
        ca_cache_line_init(&line, 64);
        ca_cache_line_fill(&line, 1, z, 10);
        ISO_CHECK_EQ_UINT(line.last_access, 10);
        ca_cache_line_touch(&line, 50);
        ISO_CHECK_EQ_UINT(line.last_access, 50);
        ca_cache_line_free(&line);
    }
    { /* invalidate clears valid + dirty */
        CaCacheLine line;
        uint8_t z[64] = {0};
        ca_cache_line_init(&line, 64);
        ca_cache_line_fill(&line, 1, z, 10);
        line.dirty = 1;
        ca_cache_line_invalidate(&line);
        ISO_CHECK(!line.valid);
        ISO_CHECK(!line.dirty);
        ca_cache_line_free(&line);
    }
    { /* different line sizes */
        CaCacheLine l32, l128;
        ca_cache_line_init(&l32, 32);
        ca_cache_line_init(&l128, 128);
        ISO_CHECK_EQ_UINT(ca_cache_line_size(&l32), 32);
        ISO_CHECK_EQ_UINT(ca_cache_line_size(&l128), 128);
        ca_cache_line_free(&l32);
        ca_cache_line_free(&l128);
    }

    /* ══ CacheConfig ════════════════════════════════════════════════════ */
    {
        CaCacheConfig cfg;
        ISO_CHECK(ca_cache_config_new(&cfg, "L1D", 1024, 64, 4, 1));
        ISO_CHECK(strcmp(cfg.name, "L1D") == 0);
        ISO_CHECK_EQ_UINT(cfg.total_size, 1024);
        ISO_CHECK_EQ_UINT(cfg.line_size, 64);
        ISO_CHECK_EQ_UINT(cfg.associativity, 4);
        ISO_CHECK_EQ_UINT(cfg.access_latency, 1);
        ISO_CHECK_EQ_UINT(ca_cache_config_num_lines(&cfg), 16);
        ISO_CHECK_EQ_UINT(ca_cache_config_num_sets(&cfg), 4);
    }
    { /* invalid configs are rejected (Rust panics) */
        CaCacheConfig cfg;
        ISO_CHECK(!ca_cache_config_new(&cfg, "bad", 0, 64, 4, 1));
        ISO_CHECK(!ca_cache_config_new(&cfg, "bad", 1024, 48, 4, 1));
        ISO_CHECK(!ca_cache_config_new(&cfg, "bad", 1024, 64, 0, 1));
        ISO_CHECK(!ca_cache_config_new(&cfg, "bad", 1000, 64, 4, 1));
    }
    { /* write-policy field is settable (builder equivalent) */
        CaCacheConfig cfg;
        ca_cache_config_new(&cfg, "L1D", 1024, 64, 4, 1);
        cfg.write_policy = CA_WRITE_THROUGH;
        ISO_CHECK(cfg.write_policy == CA_WRITE_THROUGH);
    }
    { /* direct-mapped config */
        CaCacheConfig cfg;
        ca_cache_config_new(&cfg, "DM", 256, 64, 1, 1);
        ISO_CHECK_EQ_UINT(ca_cache_config_num_lines(&cfg), 4);
        ISO_CHECK_EQ_UINT(ca_cache_config_num_sets(&cfg), 4);
    }

    /* ══ CacheSet ═══════════════════════════════════════════════════════ */
    {
        CaCacheSet set;
        size_t w;
        ca_cache_set_init(&set, 4, 64);
        ISO_CHECK_EQ_UINT(set.num_ways, 4);
        for (w = 0; w < 4; w++) {
            ISO_CHECK(!set.lines[w].valid);
        }
        ISO_CHECK(!ca_cache_set_lookup(&set, 42, NULL));
        ca_cache_set_free(&set);
    }
    { /* allocate into empty slot: no eviction, then lookup hits way 0 */
        CaCacheSet set;
        uint8_t data[64];
        size_t way = 99;
        memset(data, 0xAA, sizeof data);
        ca_cache_set_init(&set, 4, 64);
        ISO_CHECK(!ca_cache_set_allocate(&set, 42, data, 64, 100, NULL, NULL,
                                         NULL));
        ISO_CHECK(ca_cache_set_lookup(&set, 42, &way));
        ISO_CHECK_EQ_UINT(way, 0);
        ISO_CHECK_EQ_UINT(set.lines[0].tag, 42);
        ca_cache_set_free(&set);
    }
    { /* allocate fills sequentially */
        CaCacheSet set;
        uint8_t z[64] = {0};
        uint64_t tag;
        ca_cache_set_init(&set, 4, 64);
        for (tag = 0; tag < 4; tag++) {
            ISO_CHECK(!ca_cache_set_allocate(&set, tag, z, 64, tag, NULL, NULL,
                                             NULL));
        }
        {
            size_t i;
            for (i = 0; i < 4; i++) {
                ISO_CHECK(set.lines[i].valid);
                ISO_CHECK_EQ_UINT(set.lines[i].tag, i);
            }
        }
        ca_cache_set_free(&set);
    }
    { /* LRU eviction: clean victim, no dirty report */
        CaCacheSet set;
        uint8_t z[64] = {0};
        ca_cache_set_init(&set, 2, 64);
        ca_cache_set_allocate(&set, 10, z, 64, 1, NULL, NULL, NULL);
        ca_cache_set_allocate(&set, 20, z, 64, 2, NULL, NULL, NULL);
        ISO_CHECK(!ca_cache_set_allocate(&set, 30, z, 64, 3, NULL, NULL, NULL));
        ISO_CHECK(!ca_cache_set_lookup(&set, 10, NULL));
        ISO_CHECK(ca_cache_set_lookup(&set, 30, NULL));
        ca_cache_set_free(&set);
    }
    { /* dirty eviction returns victim metadata */
        CaCacheSet set;
        uint8_t z[64] = {0};
        int dirty = 0;
        uint64_t vtag = 0, vla = 0;
        ca_cache_set_init(&set, 2, 64);
        ca_cache_set_allocate(&set, 10, z, 64, 1, NULL, NULL, NULL);
        ca_cache_set_allocate(&set, 20, z, 64, 2, NULL, NULL, NULL);
        set.lines[0].dirty = 1;
        ISO_CHECK(ca_cache_set_allocate(&set, 30, z, 64, 3, &dirty, &vtag,
                                        &vla));
        ISO_CHECK(dirty);
        ISO_CHECK_EQ_UINT(vtag, 10);
        ca_cache_set_free(&set);
    }
    { /* access hit updates LRU */
        CaCacheSet set;
        uint8_t z[64] = {0};
        size_t idx = 0;
        ca_cache_set_init(&set, 4, 64);
        ca_cache_set_allocate(&set, 10, z, 64, 1, NULL, NULL, NULL);
        ca_cache_set_allocate(&set, 20, z, 64, 2, NULL, NULL, NULL);
        ISO_CHECK(ca_cache_set_access(&set, 10, 50, &idx));
        ISO_CHECK_EQ_UINT(set.lines[idx].last_access, 50);
        ca_cache_set_free(&set);
    }
    { /* access miss returns LRU index */
        CaCacheSet set;
        uint8_t z[64] = {0};
        size_t idx = 99;
        ca_cache_set_init(&set, 2, 64);
        ca_cache_set_allocate(&set, 10, z, 64, 1, NULL, NULL, NULL);
        ca_cache_set_allocate(&set, 20, z, 64, 2, NULL, NULL, NULL);
        ISO_CHECK(!ca_cache_set_access(&set, 99, 3, &idx));
        ISO_CHECK_EQ_UINT(idx, 0);
        ca_cache_set_free(&set);
    }

    /* ══ Cache ══════════════════════════════════════════════════════════ */
    { /* address decomposition */
        CaCache c;
        uint64_t tag;
        size_t si, off;
        make_l1d(&c);
        ca_cache_decompose(&c, 0x100, &tag, &si, &off);
        ISO_CHECK_EQ_UINT(off, 0);
        ISO_CHECK_EQ_UINT(si, 0);
        ISO_CHECK_EQ_UINT(tag, 0x100 >> 8);
        ca_cache_free(&c);
    }
    { /* first read miss, second hit */
        CaCache c;
        CaCacheAccess a;
        make_l1d(&c);
        a = ca_cache_read(&c, 0x100, 0);
        ISO_CHECK(!a.hit);
        ISO_CHECK_EQ_UINT(a.cycles, 1);
        ISO_CHECK_EQ_UINT(c.stats.reads, 1);
        ISO_CHECK_EQ_UINT(c.stats.misses, 1);
        a = ca_cache_read(&c, 0x100, 1);
        ISO_CHECK(a.hit);
        ISO_CHECK_EQ_UINT(c.stats.hits, 1);
        ca_cache_free(&c);
    }
    { /* conflict misses + eviction in one set */
        CaCache c;
        CaCacheAccess a;
        make_l1d(&c);
        ca_cache_read(&c, 0x000, 0);
        ca_cache_read(&c, 0x100, 1);
        ca_cache_read(&c, 0x200, 2);
        ca_cache_read(&c, 0x300, 3);
        ISO_CHECK_EQ_UINT(c.stats.misses, 4);
        ca_cache_read(&c, 0x400, 4);
        ISO_CHECK_EQ_UINT(c.stats.misses, 5);
        a = ca_cache_read(&c, 0x000, 5);
        ISO_CHECK(!a.hit); /* evicted -> miss again */
        ca_cache_free(&c);
    }
    { /* write hit */
        CaCache c;
        CaCacheAccess a;
        uint8_t b = 0xAB;
        make_l1d(&c);
        ca_cache_read(&c, 0x100, 0);
        a = ca_cache_write(&c, 0x100, &b, 1, 1);
        ISO_CHECK(a.hit);
        ISO_CHECK_EQ_UINT(c.stats.writes, 1);
        ISO_CHECK_EQ_UINT(c.stats.hits, 1);
        ca_cache_free(&c);
    }
    { /* write miss allocates, subsequent read hits */
        CaCache c;
        CaCacheAccess a;
        uint8_t b = 0xAB;
        make_l1d(&c);
        a = ca_cache_write(&c, 0x100, &b, 1, 0);
        ISO_CHECK(!a.hit);
        a = ca_cache_read(&c, 0x100, 1);
        ISO_CHECK(a.hit);
        ca_cache_free(&c);
    }
    { /* write-back marks dirty; dirty line evicted returns victim */
        CaCache c;
        CaCacheAccess a;
        uint8_t b = 0xAB;
        make_l1d(&c);
        ca_cache_read(&c, 0x100, 0);
        ca_cache_write(&c, 0x100, &b, 1, 1);
        ca_cache_read(&c, 0x000, 2);
        ca_cache_read(&c, 0x200, 3);
        ca_cache_read(&c, 0x300, 4);
        a = ca_cache_read(&c, 0x400, 5);
        ISO_CHECK(!a.hit);
        if (a.has_evicted) {
            ISO_CHECK(a.evicted_dirty);
        }
        ca_cache_free(&c);
    }
    { /* write-through leaves the line clean */
        CaCache c;
        CaCacheConfig cfg;
        uint64_t tag;
        size_t si, way = 0;
        uint8_t b = 0xAB;
        ca_cache_config_new(&cfg, "L1D", 1024, 64, 4, 1);
        cfg.write_policy = CA_WRITE_THROUGH;
        ISO_CHECK(ca_cache_init(&c, &cfg));
        ca_cache_read(&c, 0x100, 0);
        ca_cache_write(&c, 0x100, &b, 1, 1);
        ca_cache_decompose(&c, 0x100, &tag, &si, NULL);
        ISO_CHECK(ca_cache_set_lookup(&c.sets[si], tag, &way));
        ISO_CHECK(!c.sets[si].lines[way].dirty);
        ca_cache_free(&c);
    }
    { /* invalidate all -> miss again */
        CaCache c;
        CaCacheAccess a;
        make_l1d(&c);
        ca_cache_read(&c, 0x100, 0);
        ca_cache_read(&c, 0x200, 1);
        ca_cache_invalidate(&c);
        a = ca_cache_read(&c, 0x100, 2);
        ISO_CHECK(!a.hit);
        ca_cache_free(&c);
    }
    { /* fill_line installs directly */
        CaCache c;
        CaCacheAccess a;
        uint8_t data[64];
        memset(data, 0xCD, sizeof data);
        make_l1d(&c);
        ISO_CHECK(!ca_cache_fill_line(&c, 0x100, data, 64, 0, NULL, NULL, NULL));
        a = ca_cache_read(&c, 0x100, 1);
        ISO_CHECK(a.hit);
        ca_cache_free(&c);
    }
    { /* sequential (spatial locality): 1 miss, 63 hits */
        CaCache c;
        uint64_t i;
        make_l1d(&c);
        for (i = 0; i < 64; i++) {
            CaCacheAccess a = ca_cache_read(&c, 0x100 + i, i);
            if (i == 0) {
                ISO_CHECK(!a.hit);
            } else {
                ISO_CHECK(a.hit);
            }
        }
        ISO_CHECK_EQ_UINT(c.stats.hits, 63);
        ISO_CHECK_EQ_UINT(c.stats.misses, 1);
        ca_cache_free(&c);
    }
    { /* strided: 4 misses then 4 hits */
        CaCache c;
        uint64_t i;
        make_l1d(&c);
        for (i = 0; i < 4; i++) {
            CaCacheAccess a = ca_cache_read(&c, i * 64, i);
            ISO_CHECK(!a.hit);
        }
        ISO_CHECK_EQ_UINT(c.stats.misses, 4);
        for (i = 0; i < 4; i++) {
            CaCacheAccess a = ca_cache_read(&c, i * 64, i + 4);
            ISO_CHECK(a.hit);
        }
        ISO_CHECK_EQ_UINT(c.stats.hits, 4);
        ca_cache_free(&c);
    }

    /* ══ CacheStats ═════════════════════════════════════════════════════ */
    {
        CaCacheStats s;
        ca_cache_stats_init(&s);
        ISO_CHECK_EQ_UINT(s.reads, 0);
        ISO_CHECK_EQ_UINT(ca_cache_stats_total_accesses(&s), 0);
        ISO_CHECK_EQ_DBL(ca_cache_stats_hit_rate(&s), 0.0, EPS);
        ISO_CHECK_EQ_DBL(ca_cache_stats_miss_rate(&s), 0.0, EPS);
    }
    {
        CaCacheStats s;
        ca_cache_stats_init(&s);
        ca_cache_stats_record_read(&s, 1);
        ISO_CHECK_EQ_UINT(s.reads, 1);
        ISO_CHECK_EQ_UINT(s.hits, 1);
        ISO_CHECK_EQ_DBL(ca_cache_stats_hit_rate(&s), 1.0, EPS);
        ca_cache_stats_record_read(&s, 0);
        ISO_CHECK_EQ_UINT(s.misses, 1);
        ISO_CHECK_EQ_DBL(ca_cache_stats_hit_rate(&s), 0.5, EPS);
        ISO_CHECK_EQ_DBL(ca_cache_stats_miss_rate(&s), 0.5, EPS);
    }
    {
        CaCacheStats s;
        ca_cache_stats_init(&s);
        ca_cache_stats_record_read(&s, 1);
        ca_cache_stats_record_read(&s, 1);
        ca_cache_stats_record_write(&s, 0);
        ca_cache_stats_record_write(&s, 1);
        ISO_CHECK_EQ_UINT(ca_cache_stats_total_accesses(&s), 4);
        ISO_CHECK_EQ_UINT(s.hits, 3);
        ISO_CHECK_EQ_UINT(s.misses, 1);
        ISO_CHECK_EQ_DBL(ca_cache_stats_hit_rate(&s), 0.75, EPS);
        ISO_CHECK_EQ_DBL(ca_cache_stats_miss_rate(&s), 0.25, EPS);
    }
    {
        CaCacheStats s;
        ca_cache_stats_init(&s);
        ca_cache_stats_record_eviction(&s, 0);
        ca_cache_stats_record_eviction(&s, 1);
        ca_cache_stats_record_eviction(&s, 1);
        ISO_CHECK_EQ_UINT(s.evictions, 3);
        ISO_CHECK_EQ_UINT(s.writebacks, 2);
        ca_cache_stats_reset(&s);
        ISO_CHECK_EQ_UINT(ca_cache_stats_total_accesses(&s), 0);
        ISO_CHECK_EQ_UINT(s.evictions, 0);
        ISO_CHECK_EQ_UINT(s.writebacks, 0);
    }

    /* ══ CacheHierarchy ═════════════════════════════════════════════════ */
    { /* no caches -> memory */
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        ca_cache_hierarchy_init(&h, NULL, NULL, NULL, NULL, 100);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 0);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        ISO_CHECK_EQ_UINT(r.total_cycles, 100);
        ca_cache_hierarchy_free(&h);
    }
    { /* L1 only: miss then hit */
        CaCache l1d;
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        make_l1d(&l1d);
        ca_cache_hierarchy_init(&h, NULL, &l1d, NULL, NULL, 100);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 0);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        ISO_CHECK_EQ_UINT(r.total_cycles, 1 + 100);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 1);
        ISO_CHECK(strcmp(r.served_by, "L1D") == 0);
        ISO_CHECK_EQ_UINT(r.total_cycles, 1);
        ca_cache_hierarchy_free(&h);
    }
    { /* two-level */
        CaCache l1d, l2;
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        make_l1d(&l1d);
        make_l2(&l2);
        ca_cache_hierarchy_init(&h, NULL, &l1d, &l2, NULL, 100);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 0);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        ISO_CHECK_EQ_UINT(r.total_cycles, 1 + 10 + 100);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 1);
        ISO_CHECK(strcmp(r.served_by, "L1D") == 0);
        ISO_CHECK_EQ_UINT(r.total_cycles, 1);
        ca_cache_hierarchy_free(&h);
    }
    { /* three-level */
        CaCache l1d, l2, l3;
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        make_l1d(&l1d);
        make_l2(&l2);
        make_l3(&l3);
        ca_cache_hierarchy_init(&h, NULL, &l1d, &l2, &l3, 100);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 0);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        ISO_CHECK_EQ_UINT(r.total_cycles, 1 + 10 + 30 + 100);
        ca_cache_hierarchy_free(&h);
    }
    { /* write miss then read hit at L1D */
        CaCache l1d, l2;
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        uint8_t b = 0xAB;
        make_l1d(&l1d);
        make_l2(&l2);
        ca_cache_hierarchy_init(&h, NULL, &l1d, &l2, NULL, 100);
        r = ca_cache_hierarchy_write(&h, 0x2000, &b, 1, 0);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        r = ca_cache_hierarchy_read(&h, 0x2000, 0, 1);
        ISO_CHECK(strcmp(r.served_by, "L1D") == 0);
        ca_cache_hierarchy_free(&h);
    }
    { /* Harvard: L1I vs L1D are separate */
        CaCache l1i, l1d;
        CaCacheConfig cfg;
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        ca_cache_config_new(&cfg, "L1I", 1024, 64, 4, 1);
        ISO_CHECK(ca_cache_init(&l1i, &cfg));
        make_l1d(&l1d);
        ca_cache_hierarchy_init(&h, &l1i, &l1d, NULL, NULL, 100);
        r = ca_cache_hierarchy_read(&h, 0x1000, 1, 0);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 1);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        r = ca_cache_hierarchy_read(&h, 0x1000, 1, 2);
        ISO_CHECK(strcmp(r.served_by, "L1I") == 0);
        ca_cache_hierarchy_free(&h);
    }
    { /* invalidate_all -> miss again */
        CaCache l1d, l2;
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        make_l1d(&l1d);
        make_l2(&l2);
        ca_cache_hierarchy_init(&h, NULL, &l1d, &l2, NULL, 100);
        ca_cache_hierarchy_read(&h, 0x1000, 0, 0);
        ca_cache_hierarchy_invalidate_all(&h);
        r = ca_cache_hierarchy_read(&h, 0x1000, 0, 1);
        ISO_CHECK(strcmp(r.served_by, "memory") == 0);
        ca_cache_hierarchy_free(&h);
    }
    { /* reset_stats zeroes level stats */
        CaCache l1d, l2;
        CaCacheHierarchy h;
        make_l1d(&l1d);
        make_l2(&l2);
        ca_cache_hierarchy_init(&h, NULL, &l1d, &l2, NULL, 100);
        ca_cache_hierarchy_read(&h, 0x1000, 0, 0);
        ca_cache_hierarchy_reset_stats(&h);
        ISO_CHECK_EQ_UINT(ca_cache_stats_total_accesses(&h.l1d->stats), 0);
        ca_cache_hierarchy_free(&h);
    }
    { /* inclusive fill: pre-fill L2, hierarchy read fills L1 for next hit */
        CaCache l1d, l2;
        CaCacheHierarchy h;
        CaHierarchyAccess r;
        make_l1d(&l1d);
        make_l2(&l2);
        ca_cache_hierarchy_init(&h, NULL, &l1d, &l2, NULL, 100);
        ca_cache_read(h.l2, 0x3000, 0); /* pre-fill L2 directly */
        ca_cache_hierarchy_read(&h, 0x3000, 0, 1); /* L1 miss, L2 hit */
        r = ca_cache_hierarchy_read(&h, 0x3000, 0, 2);
        ISO_CHECK(strcmp(r.served_by, "L1D") == 0);
        ca_cache_hierarchy_free(&h);
    }

    return ISO_TEST_RESULT();
}
