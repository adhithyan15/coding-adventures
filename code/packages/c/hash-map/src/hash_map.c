/*
 * hash_map.c — implementation of the hash map (see hash_map.h). A faithful port
 * of the Rust `hash-map` crate: separate chaining and open addressing (with
 * tombstones), the four selectable hash functions, and load-factor resizing.
 *
 * Note on hashing: the Rust crate hashes an element's `Debug` string; this port
 * hashes the raw key bytes you pass in. The map is self-consistent either way
 * (set and get hash keys identically), so behaviour is faithful — only the
 * concrete bucket a key lands in differs, which a hash map never exposes.
 */
#include "hash_map.h"

#include <stdint.h> /* uint8_t, uint32_t, uint64_t, SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy, memcmp */

/* Resize thresholds, matching the Rust crate. */
#define CHAINING_RESIZE_THRESHOLD 1.0
#define OPEN_ADDRESSING_RESIZE_THRESHOLD 0.75

/* SipHash key ("codex-dt18-key!!", 16 bytes) — identical to the Rust crate. */
static const uint8_t SIP_KEY[16] = {'c', 'o', 'd', 'e', 'x', '-', 'd', 't',
                                    '1', '8', '-', 'k', 'e', 'y', '!', '!'};

/* ── little-endian loads and rotations ────────────────────────────────────── */
static uint32_t load32le(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) |
           ((uint32_t)p[3] << 24);
}
static uint64_t load64le(const uint8_t *p) {
    return (uint64_t)p[0] | ((uint64_t)p[1] << 8) | ((uint64_t)p[2] << 16) |
           ((uint64_t)p[3] << 24) | ((uint64_t)p[4] << 32) |
           ((uint64_t)p[5] << 40) | ((uint64_t)p[6] << 48) |
           ((uint64_t)p[7] << 56);
}
static uint32_t rotl32(uint32_t x, unsigned n) {
    return (x << n) | (x >> (32 - n));
}
static uint64_t rotl64(uint64_t x, unsigned n) {
    return (x << n) | (x >> (64 - n));
}

/* ── the four hash functions (matching coding_adventures_hash_functions) ──── */
static uint32_t fmix32(uint32_t h) {
    h ^= h >> 16;
    h *= 0x85ebca6bu;
    h ^= h >> 13;
    h *= 0xc2b2ae35u;
    h ^= h >> 16;
    return h;
}
static uint64_t fnv1a_32(const uint8_t *d, size_t n) {
    uint32_t h = 0x811c9dc5u;
    size_t i;
    for (i = 0; i < n; i++) {
        h ^= d[i];
        h *= 0x01000193u;
    }
    return h;
}
static uint64_t djb2(const uint8_t *d, size_t n) {
    uint64_t h = 5381u;
    size_t i;
    for (i = 0; i < n; i++) {
        h = (h << 5) + h + d[i];
    }
    return h;
}
static uint64_t murmur3_32(const uint8_t *d, size_t n) {
    uint32_t hash = 0; /* seed 0 */
    size_t blocks = n / 4;
    size_t i;
    uint32_t k;
    for (i = 0; i < blocks; i++) {
        k = load32le(d + i * 4);
        k *= 0xcc9e2d51u;
        k = rotl32(k, 15);
        k *= 0x1b873593u;
        hash ^= k;
        hash = rotl32(hash, 13);
        hash = hash * 5u + 0xe6546b64u;
    }
    k = 0;
    {
        size_t rem = n & 3u;
        size_t base = blocks * 4;
        size_t j;
        for (j = 0; j < rem; j++) {
            k ^= (uint32_t)d[base + j] << (j * 8);
        }
        if (rem != 0) {
            k *= 0xcc9e2d51u;
            k = rotl32(k, 15);
            k *= 0x1b873593u;
            hash ^= k;
        }
    }
    hash ^= (uint32_t)n;
    return fmix32(hash);
}
static void sipround(uint64_t *v0, uint64_t *v1, uint64_t *v2, uint64_t *v3) {
    *v0 += *v1;
    *v1 = rotl64(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl64(*v0, 32);
    *v2 += *v3;
    *v3 = rotl64(*v3, 16);
    *v3 ^= *v2;
    *v0 += *v3;
    *v3 = rotl64(*v3, 21);
    *v3 ^= *v0;
    *v2 += *v1;
    *v1 = rotl64(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl64(*v2, 32);
}
static uint64_t siphash24(const uint8_t *d, size_t n) {
    uint64_t k0 = load64le(SIP_KEY);
    uint64_t k1 = load64le(SIP_KEY + 8);
    uint64_t v0 = 0x736f6d6570736575u ^ k0;
    uint64_t v1 = 0x646f72616e646f6du ^ k1;
    uint64_t v2 = 0x6c7967656e657261u ^ k0;
    uint64_t v3 = 0x7465646279746573u ^ k1;
    size_t blocks = n / 8;
    size_t i;
    uint64_t last;
    for (i = 0; i < blocks; i++) {
        uint64_t m = load64le(d + i * 8);
        v3 ^= m;
        sipround(&v0, &v1, &v2, &v3);
        sipround(&v0, &v1, &v2, &v3);
        v0 ^= m;
    }
    last = ((uint64_t)n & 0xffu) << 56;
    {
        size_t rem = n & 7u;
        size_t base = blocks * 8;
        size_t j;
        for (j = 0; j < rem; j++) {
            last |= (uint64_t)d[base + j] << (j * 8);
        }
    }
    v3 ^= last;
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);
    v0 ^= last;
    v2 ^= 0xffu;
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);
    return v0 ^ v1 ^ v2 ^ v3;
}

/* ── internal storage ─────────────────────────────────────────────────────── */
typedef struct node {
    uint8_t *key;
    size_t key_len;
    uint8_t *val;
    size_t val_len;
    struct node *next;
} node;

enum { SLOT_EMPTY = 0, SLOT_TOMBSTONE = 1, SLOT_OCCUPIED = 2 };

typedef struct {
    int state;
    uint8_t *key;
    size_t key_len;
    uint8_t *val;
    size_t val_len;
} slot;

struct hashmap {
    hashmap_strategy strategy;
    hashmap_hash hash;
    size_t size;
    size_t capacity;
    node **buckets; /* chaining */
    slot *slots;    /* open addressing */
};

/* dup_bytes — heap-copy `len` bytes (always returns a freeable non-NULL pointer
 * for len 0). NULL on allocation failure. */
static uint8_t *dup_bytes(const void *src, size_t len) {
    uint8_t *p = (uint8_t *)malloc(len ? len : 1);
    if (p == NULL) {
        return NULL;
    }
    if (len) {
        memcpy(p, src, len);
    }
    return p;
}

static int key_eq(const uint8_t *a, size_t alen, const void *b, size_t blen) {
    return alen == blen && (alen == 0 || memcmp(a, b, alen) == 0);
}

static uint64_t map_hash(const hashmap *m, const void *key, size_t len) {
    const uint8_t *d = (const uint8_t *)key;
    switch (m->hash) {
    case HASHMAP_SIPHASH24:
        return siphash24(d, len);
    case HASHMAP_FNV1A32:
        return fnv1a_32(d, len);
    case HASHMAP_MURMUR3_32:
        return murmur3_32(d, len);
    case HASHMAP_DJB2:
        return djb2(d, len);
    }
    return 0; /* unreachable */
}

static size_t bucket_index(const hashmap *m, const void *key, size_t len) {
    return (size_t)(map_hash(m, key, len) % (uint64_t)m->capacity);
}

/* ── construction / destruction ───────────────────────────────────────────── */
hashmap *hashmap_new(size_t capacity, hashmap_strategy strategy,
                     hashmap_hash hash) {
    hashmap *m = (hashmap *)malloc(sizeof *m);
    if (m == NULL) {
        return NULL;
    }
    if (capacity < 1) {
        capacity = 1;
    }
    m->strategy = strategy;
    m->hash = hash;
    m->size = 0;
    m->capacity = capacity;
    m->buckets = NULL;
    m->slots = NULL;
    if (strategy == HASHMAP_CHAINING) {
        m->buckets = (node **)calloc(capacity, sizeof(node *));
        if (m->buckets == NULL) {
            free(m);
            return NULL;
        }
    } else {
        m->slots = (slot *)calloc(capacity, sizeof(slot)); /* zero → SLOT_EMPTY */
        if (m->slots == NULL) {
            free(m);
            return NULL;
        }
    }
    return m;
}

void hashmap_free(hashmap *map) {
    size_t i;
    if (map == NULL) {
        return;
    }
    if (map->buckets != NULL) {
        for (i = 0; i < map->capacity; i++) {
            node *n = map->buckets[i];
            while (n != NULL) {
                node *next = n->next;
                free(n->key);
                free(n->val);
                free(n);
                n = next;
            }
        }
        free(map->buckets);
    }
    if (map->slots != NULL) {
        for (i = 0; i < map->capacity; i++) {
            if (map->slots[i].state == SLOT_OCCUPIED) {
                free(map->slots[i].key);
                free(map->slots[i].val);
            }
        }
        free(map->slots);
    }
    free(map);
}

/* ── resize (no key/value re-duplication) ─────────────────────────────────── */
static void resize_chaining(hashmap *m, size_t new_cap) {
    node **nb = (node **)calloc(new_cap, sizeof(node *));
    size_t i;
    if (nb == NULL) {
        return; /* keep the current table; it is still correct, just denser */
    }
    for (i = 0; i < m->capacity; i++) {
        node *n = m->buckets[i];
        while (n != NULL) {
            node *next = n->next;
            size_t idx = (size_t)(map_hash(m, n->key, n->key_len) %
                                  (uint64_t)new_cap);
            n->next = nb[idx];
            nb[idx] = n;
            n = next;
        }
    }
    free(m->buckets);
    m->buckets = nb;
    m->capacity = new_cap;
}

static void resize_open(hashmap *m, size_t new_cap) {
    slot *ns = (slot *)calloc(new_cap, sizeof(slot));
    size_t i;
    if (ns == NULL) {
        return;
    }
    for (i = 0; i < m->capacity; i++) {
        if (m->slots[i].state == SLOT_OCCUPIED) {
            size_t start =
                (size_t)(map_hash(m, m->slots[i].key, m->slots[i].key_len) %
                         (uint64_t)new_cap);
            size_t probe;
            for (probe = 0; probe < new_cap; probe++) {
                size_t idx = (start + probe) % new_cap;
                if (ns[idx].state == SLOT_EMPTY) {
                    ns[idx] = m->slots[i]; /* move key/val pointers */
                    ns[idx].state = SLOT_OCCUPIED;
                    break;
                }
            }
        }
    }
    free(m->slots);
    m->slots = ns;
    m->capacity = new_cap;
}

static int needs_resize(const hashmap *m) {
    double load = (double)m->size / (double)m->capacity;
    if (m->strategy == HASHMAP_CHAINING) {
        return load > CHAINING_RESIZE_THRESHOLD;
    }
    return load > OPEN_ADDRESSING_RESIZE_THRESHOLD;
}

static void maybe_resize(hashmap *m) {
    if (!needs_resize(m)) {
        return;
    }
    if (m->capacity > SIZE_MAX / 2) {
        return; /* cannot double without overflow — stay as-is */
    }
    if (m->strategy == HASHMAP_CHAINING) {
        resize_chaining(m, m->capacity * 2);
    } else {
        resize_open(m, m->capacity * 2);
    }
}

/* ── insertion ────────────────────────────────────────────────────────────── */
static int insert_chaining(hashmap *m, const void *key, size_t key_len,
                           const void *value, size_t value_len) {
    size_t idx = bucket_index(m, key, key_len);
    node *n = m->buckets[idx];
    while (n != NULL) {
        if (key_eq(n->key, n->key_len, key, key_len)) {
            uint8_t *nv = dup_bytes(value, value_len);
            if (nv == NULL) {
                return 0;
            }
            free(n->val);
            n->val = nv;
            n->val_len = value_len;
            return 1;
        }
        n = n->next;
    }
    /* Not found: allocate a fresh node (key + value copies first). */
    {
        node *fresh = (node *)malloc(sizeof *fresh);
        uint8_t *k, *v;
        if (fresh == NULL) {
            return 0;
        }
        k = dup_bytes(key, key_len);
        v = dup_bytes(value, value_len);
        if (k == NULL || v == NULL) {
            free(k);
            free(v);
            free(fresh);
            return 0;
        }
        fresh->key = k;
        fresh->key_len = key_len;
        fresh->val = v;
        fresh->val_len = value_len;
        fresh->next = m->buckets[idx];
        m->buckets[idx] = fresh;
        m->size++;
    }
    return 1;
}

static int insert_open(hashmap *m, const void *key, size_t key_len,
                       const void *value, size_t value_len) {
    size_t start = bucket_index(m, key, key_len);
    size_t first_tomb = SIZE_MAX;
    size_t probe;
    for (probe = 0; probe < m->capacity; probe++) {
        size_t idx = (start + probe) % m->capacity;
        int st = m->slots[idx].state;
        if (st == SLOT_EMPTY) {
            size_t at = (first_tomb != SIZE_MAX) ? first_tomb : idx;
            uint8_t *k = dup_bytes(key, key_len);
            uint8_t *v = dup_bytes(value, value_len);
            if (k == NULL || v == NULL) {
                free(k);
                free(v);
                return 0;
            }
            m->slots[at].key = k;
            m->slots[at].key_len = key_len;
            m->slots[at].val = v;
            m->slots[at].val_len = value_len;
            m->slots[at].state = SLOT_OCCUPIED;
            m->size++;
            return 1;
        }
        if (st == SLOT_TOMBSTONE) {
            if (first_tomb == SIZE_MAX) {
                first_tomb = idx;
            }
        } else if (key_eq(m->slots[idx].key, m->slots[idx].key_len, key,
                          key_len)) {
            uint8_t *nv = dup_bytes(value, value_len);
            if (nv == NULL) {
                return 0;
            }
            free(m->slots[idx].val);
            m->slots[idx].val = nv;
            m->slots[idx].val_len = value_len;
            return 1;
        }
    }
    /* No empty slot seen: reuse the first tombstone if there was one. */
    if (first_tomb != SIZE_MAX) {
        uint8_t *k = dup_bytes(key, key_len);
        uint8_t *v = dup_bytes(value, value_len);
        if (k == NULL || v == NULL) {
            free(k);
            free(v);
            return 0;
        }
        m->slots[first_tomb].key = k;
        m->slots[first_tomb].key_len = key_len;
        m->slots[first_tomb].val = v;
        m->slots[first_tomb].val_len = value_len;
        m->slots[first_tomb].state = SLOT_OCCUPIED;
        m->size++;
        return 1;
    }
    return 0; /* table full — cannot happen while resizing keeps load < 0.75 */
}

int hashmap_set(hashmap *map, const void *key, size_t key_len, const void *value,
                size_t value_len) {
    int ok;
    if (map->strategy == HASHMAP_CHAINING) {
        ok = insert_chaining(map, key, key_len, value, value_len);
    } else {
        ok = insert_open(map, key, key_len, value, value_len);
    }
    if (!ok) {
        return 0;
    }
    maybe_resize(map);
    return 1;
}

/* ── lookup ───────────────────────────────────────────────────────────────── */
int hashmap_get(const hashmap *map, const void *key, size_t key_len,
                const void **value_out, size_t *value_len_out) {
    if (map->strategy == HASHMAP_CHAINING) {
        size_t idx = bucket_index(map, key, key_len);
        node *n = map->buckets[idx];
        while (n != NULL) {
            if (key_eq(n->key, n->key_len, key, key_len)) {
                if (value_out != NULL) {
                    *value_out = n->val;
                }
                if (value_len_out != NULL) {
                    *value_len_out = n->val_len;
                }
                return 1;
            }
            n = n->next;
        }
        return 0;
    } else {
        size_t start = bucket_index(map, key, key_len);
        size_t probe;
        for (probe = 0; probe < map->capacity; probe++) {
            size_t idx = (start + probe) % map->capacity;
            int st = map->slots[idx].state;
            if (st == SLOT_EMPTY) {
                return 0;
            }
            if (st == SLOT_OCCUPIED &&
                key_eq(map->slots[idx].key, map->slots[idx].key_len, key,
                       key_len)) {
                if (value_out != NULL) {
                    *value_out = map->slots[idx].val;
                }
                if (value_len_out != NULL) {
                    *value_len_out = map->slots[idx].val_len;
                }
                return 1;
            }
        }
        return 0;
    }
}

int hashmap_has(const hashmap *map, const void *key, size_t key_len) {
    return hashmap_get(map, key, key_len, NULL, NULL);
}

/* ── deletion ─────────────────────────────────────────────────────────────── */
int hashmap_delete(hashmap *map, const void *key, size_t key_len) {
    if (map->strategy == HASHMAP_CHAINING) {
        size_t idx = bucket_index(map, key, key_len);
        node *n = map->buckets[idx];
        node *prev = NULL;
        while (n != NULL) {
            if (key_eq(n->key, n->key_len, key, key_len)) {
                if (prev == NULL) {
                    map->buckets[idx] = n->next;
                } else {
                    prev->next = n->next;
                }
                free(n->key);
                free(n->val);
                free(n);
                map->size--;
                return 1;
            }
            prev = n;
            n = n->next;
        }
        return 0;
    } else {
        size_t start = bucket_index(map, key, key_len);
        size_t probe;
        for (probe = 0; probe < map->capacity; probe++) {
            size_t idx = (start + probe) % map->capacity;
            int st = map->slots[idx].state;
            if (st == SLOT_EMPTY) {
                return 0;
            }
            if (st == SLOT_OCCUPIED &&
                key_eq(map->slots[idx].key, map->slots[idx].key_len, key,
                       key_len)) {
                free(map->slots[idx].key);
                free(map->slots[idx].val);
                map->slots[idx].key = NULL;
                map->slots[idx].val = NULL;
                map->slots[idx].key_len = 0;
                map->slots[idx].val_len = 0;
                map->slots[idx].state = SLOT_TOMBSTONE;
                map->size--;
                return 1;
            }
        }
        return 0;
    }
}

/* ── enumeration ──────────────────────────────────────────────────────────── */
void hashmap_for_each(const hashmap *map, hashmap_iter_fn fn, void *user) {
    size_t i;
    if (map->buckets != NULL) {
        for (i = 0; i < map->capacity; i++) {
            node *n = map->buckets[i];
            while (n != NULL) {
                fn(n->key, n->key_len, n->val, n->val_len, user);
                n = n->next;
            }
        }
    } else {
        for (i = 0; i < map->capacity; i++) {
            if (map->slots[i].state == SLOT_OCCUPIED) {
                fn(map->slots[i].key, map->slots[i].key_len, map->slots[i].val,
                   map->slots[i].val_len, user);
            }
        }
    }
}

/* ── accessors ────────────────────────────────────────────────────────────── */
size_t hashmap_size(const hashmap *map) { return map->size; }
size_t hashmap_capacity(const hashmap *map) { return map->capacity; }
double hashmap_load_factor(const hashmap *map) {
    return (double)map->size / (double)map->capacity;
}
hashmap_strategy hashmap_get_strategy(const hashmap *map) {
    return map->strategy;
}
hashmap_hash hashmap_get_hash(const hashmap *map) { return map->hash; }
