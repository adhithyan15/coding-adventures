/*
 * cpu_cache.h — Configurable CPU cache hierarchy simulator, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `cpu-cache` crate. It simulates a multi-level
 * cache hierarchy (L1I / L1D / L2 / L3 / main memory) like those in modern
 * CPUs. The same `CaCache` serves as any level — only its configuration
 * (size, associativity, latency) differs.
 *
 * ## Address decomposition
 *
 *   address = | tag | set index | offset |
 *   offset     = address & (line_size - 1)
 *   set_index  = (address >> offset_bits) & (num_sets - 1)
 *   tag        = address >> (offset_bits + set_bits)
 *
 * Powers of two make this pure bit-slicing — and let us compute `offset_bits`
 * / `set_bits` as an exact integer log2, with no <math.h>.
 *
 * ## Replacement
 *
 * Each set is N-way set-associative with true LRU: every line records the
 * cycle of its last access; the smallest timestamp is evicted (invalid lines
 * are always preferred). A dirty victim is reported so the caller can account
 * for the writeback.
 *
 * Ownership: `CaCache` owns its sets, each set owns its lines, each line owns
 * its data buffer; `ca_cache_free` releases the whole tree. A `CaCacheHierarchy`
 * owns its level caches; `ca_cache_hierarchy_free` releases them. Access-result
 * structs (`CaCacheAccess`, `CaHierarchyAccess`) are plain values — no cleanup.
 *
 * Divergence from the Rust (documented): the Rust `CacheAccess.evicted` is a
 * full `Option<CacheLine>` clone. Because no code path ever reads the evicted
 * line's *data* (the hierarchy discards it; only its dirty/tag matter), this
 * port records the victim's metadata inline (`has_evicted`/`evicted_dirty`/
 * `evicted_tag`/`evicted_last_access`) rather than copying its bytes. Every
 * observable behavior is identical.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef CPU_CACHE_H
#define CPU_CACHE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Write policy: defer writes (write-back) or propagate immediately
 * (write-through). */
typedef enum { CA_WRITE_BACK, CA_WRITE_THROUGH } CaWritePolicy;

/* ── Cache line — the smallest unit of cached data ──────────────────────────*/
typedef struct {
    int valid;             /* holding real data? */
    int dirty;             /* modified since load (write-back tracking)? */
    uint64_t tag;          /* high address bits identifying the block */
    uint8_t *data;         /* owned line_size-byte buffer */
    size_t data_len;       /* == line_size */
    uint64_t last_access;  /* cycle of last access (for LRU) */
} CaCacheLine;

/* Initialize an invalid line with a `line_size`-byte zeroed buffer.
 * Returns 1 on success, 0 on allocation failure. */
int ca_cache_line_init(CaCacheLine *line, size_t line_size);
void ca_cache_line_free(CaCacheLine *line);
/* Load data (copied) into the line, marking it valid and clean. `data` must be
 * at least `line->data_len` bytes. */
void ca_cache_line_fill(CaCacheLine *line, uint64_t tag, const uint8_t *data,
                        uint64_t cycle);
void ca_cache_line_touch(CaCacheLine *line, uint64_t cycle);
void ca_cache_line_invalidate(CaCacheLine *line);
size_t ca_cache_line_size(const CaCacheLine *line);

/* ── Cache configuration ────────────────────────────────────────────────────*/
typedef struct {
    char name[32];
    size_t total_size;
    size_t line_size;
    size_t associativity;
    uint64_t access_latency;
    CaWritePolicy write_policy;
} CaCacheConfig;

/* Validate and build a config (defaults to write-back). Returns 1 on success,
 * 0 if invalid: total_size==0, line_size not a positive power of 2,
 * associativity==0, or total_size not divisible by line_size*associativity
 * (mirroring the Rust `CacheConfig::new` panics as a rejection). */
int ca_cache_config_new(CaCacheConfig *out, const char *name, size_t total_size,
                        size_t line_size, size_t associativity,
                        uint64_t access_latency);
size_t ca_cache_config_num_lines(const CaCacheConfig *c);
size_t ca_cache_config_num_sets(const CaCacheConfig *c);

/* ── Cache set — a group of `associativity` ways ────────────────────────────*/
typedef struct {
    CaCacheLine *lines; /* owned array of `num_ways` lines */
    size_t num_ways;
    size_t line_size;
} CaCacheSet;

int ca_cache_set_init(CaCacheSet *set, size_t associativity, size_t line_size);
void ca_cache_set_free(CaCacheSet *set);
/* Search the set for a valid line with `tag`. Returns 1 on hit (writes the way
 * index to *out_way if non-NULL), 0 on miss. */
int ca_cache_set_lookup(const CaCacheSet *set, uint64_t tag, size_t *out_way);
/* Access the set: on hit, touch the line and return 1 with its way index; on
 * miss, return 0 with the LRU victim's index (the eviction candidate). */
int ca_cache_set_access(CaCacheSet *set, uint64_t tag, uint64_t cycle,
                        size_t *out_index);
/* Bring data into the set (fill an invalid way, else evict LRU). Returns 1 if a
 * dirty line was evicted (filling the out-params out_dirty, out_tag,
 * out_last_access), else 0 — matching the Rust `allocate` returning Some only
 * for dirty victims. */
int ca_cache_set_allocate(CaCacheSet *set, uint64_t tag, const uint8_t *data,
                          size_t data_len, uint64_t cycle, int *out_dirty,
                          uint64_t *out_tag, uint64_t *out_last_access);

/* ── Statistics ─────────────────────────────────────────────────────────────*/
typedef struct {
    uint64_t reads, writes, hits, misses, evictions, writebacks;
} CaCacheStats;

void ca_cache_stats_init(CaCacheStats *s);
uint64_t ca_cache_stats_total_accesses(const CaCacheStats *s);
double ca_cache_stats_hit_rate(const CaCacheStats *s);
double ca_cache_stats_miss_rate(const CaCacheStats *s);
void ca_cache_stats_record_read(CaCacheStats *s, int hit);
void ca_cache_stats_record_write(CaCacheStats *s, int hit);
void ca_cache_stats_record_eviction(CaCacheStats *s, int dirty);
void ca_cache_stats_reset(CaCacheStats *s);

/* ── Single-access record ───────────────────────────────────────────────────*/
typedef struct {
    uint64_t address;
    int hit;
    uint64_t tag;
    size_t set_index;
    size_t offset;
    uint64_t cycles;
    int has_evicted;             /* a dirty line was evicted this access */
    int evicted_dirty;           /* always 1 when has_evicted */
    uint64_t evicted_tag;
    uint64_t evicted_last_access;
} CaCacheAccess;

/* ── A single configurable cache level ──────────────────────────────────────*/
typedef struct {
    CaCacheConfig config;
    CaCacheSet *sets; /* owned array of `num_sets` sets */
    size_t num_sets;
    CaCacheStats stats;
    uint32_t offset_bits;
    uint32_t set_bits;
    uint64_t set_mask;
} CaCache;

/* Initialize a cache from a (valid) config. Returns 1 on success, 0 on OOM. */
int ca_cache_init(CaCache *cache, const CaCacheConfig *config);
void ca_cache_free(CaCache *cache);
/* Split an address into (tag, set_index, offset). */
void ca_cache_decompose(const CaCache *cache, uint64_t address, uint64_t *tag,
                        size_t *set_index, size_t *offset);
CaCacheAccess ca_cache_read(CaCache *cache, uint64_t address, uint64_t cycle);
CaCacheAccess ca_cache_write(CaCache *cache, uint64_t address,
                             const uint8_t *data, size_t data_len,
                             uint64_t cycle);
void ca_cache_invalidate(CaCache *cache);
/* Directly install data (used by the hierarchy on a fill). Returns 1 if a dirty
 * line was evicted (metadata via out-params), else 0. Does not touch stats. */
int ca_cache_fill_line(CaCache *cache, uint64_t address, const uint8_t *data,
                       size_t data_len, uint64_t cycle, int *out_dirty,
                       uint64_t *out_tag, uint64_t *out_last_access);

/* ── Hierarchy access record ────────────────────────────────────────────────*/
typedef struct {
    uint64_t address;
    char served_by[8]; /* "L1I" / "L1D" / "L2" / "L3" / "memory" */
    uint64_t total_cycles;
    size_t hit_at_level;
    CaCacheAccess level_accesses[4]; /* at most 3 levels are ever walked */
    size_t level_count;
} CaHierarchyAccess;

/* ── Multi-level hierarchy ──────────────────────────────────────────────────*/
typedef struct {
    CaCache *l1i, *l1d, *l2, *l3; /* NULL when absent; owned when present */
    uint64_t main_memory_latency;
} CaCacheHierarchy;

/* Build a hierarchy that takes ownership of the given caches (any may be NULL).
 * The CaCache values are copied by value and the originals zeroed, so the
 * hierarchy owns the heap they point to. */
void ca_cache_hierarchy_init(CaCacheHierarchy *h, CaCache *l1i, CaCache *l1d,
                             CaCache *l2, CaCache *l3,
                             uint64_t main_memory_latency);
void ca_cache_hierarchy_free(CaCacheHierarchy *h);
CaHierarchyAccess ca_cache_hierarchy_read(CaCacheHierarchy *h, uint64_t address,
                                          int is_instruction, uint64_t cycle);
CaHierarchyAccess ca_cache_hierarchy_write(CaCacheHierarchy *h, uint64_t address,
                                           const uint8_t *data, size_t data_len,
                                           uint64_t cycle);
void ca_cache_hierarchy_invalidate_all(CaCacheHierarchy *h);
void ca_cache_hierarchy_reset_stats(CaCacheHierarchy *h);

#ifdef __cplusplus
}
#endif

#endif /* CPU_CACHE_H */
