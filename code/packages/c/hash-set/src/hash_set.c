/*
 * hash_set.c — implementation of the hash set (see hash_set.h). Exactly as in
 * the Rust crate, the set is a thin wrapper over the hash map with empty values,
 * so all the real work (hashing, collision handling, resizing) lives in the
 * sibling `hash-map` package. Set algebra is built on hashmap_for_each.
 */
#include "hash_set.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, free */

struct hashset {
    hashmap *map; /* HashMap<element, ()> — values are empty */
};

/* ── construction / destruction ───────────────────────────────────────────── */
hashset *hashset_new_with(size_t capacity, hashmap_strategy strategy,
                          hashmap_hash hash) {
    hashset *s = (hashset *)malloc(sizeof *s);
    if (s == NULL) {
        return NULL;
    }
    s->map = hashmap_new(capacity, strategy, hash);
    if (s->map == NULL) {
        free(s);
        return NULL;
    }
    return s;
}

hashset *hashset_new(void) {
    return hashset_new_with(16, HASHMAP_CHAINING, HASHMAP_SIPHASH24);
}

void hashset_free(hashset *set) {
    if (set == NULL) {
        return;
    }
    hashmap_free(set->map);
    free(set);
}

/* ── membership ───────────────────────────────────────────────────────────── */
int hashset_add(hashset *set, const void *elem, size_t elem_len) {
    /* Store the element as a key with an empty ("()") value. */
    return hashmap_set(set->map, elem, elem_len, "", 0);
}

int hashset_remove(hashset *set, const void *elem, size_t elem_len) {
    return hashmap_delete(set->map, elem, elem_len);
}

int hashset_contains(const hashset *set, const void *elem, size_t elem_len) {
    return hashmap_has(set->map, elem, elem_len);
}

size_t hashset_size(const hashset *set) { return hashmap_size(set->map); }
int hashset_is_empty(const hashset *set) { return hashmap_size(set->map) == 0; }

/* ── enumeration ──────────────────────────────────────────────────────────── */
typedef struct {
    hashset_iter_fn fn;
    void *user;
} fe_ctx;

static void fe_adapter(const void *k, size_t kl, const void *v, size_t vl,
                       void *user) {
    fe_ctx *c = (fe_ctx *)user;
    (void)v;
    (void)vl;
    c->fn(k, kl, c->user);
}

void hashset_for_each(const hashset *set, hashset_iter_fn fn, void *user) {
    fe_ctx c;
    c.fn = fn;
    c.user = user;
    hashmap_for_each(set->map, fe_adapter, &c);
}

/* ── helpers for set algebra ──────────────────────────────────────────────── */
/* Capacity hint = x + y, saturating at SIZE_MAX and never below 1. */
static size_t cap_add(size_t x, size_t y) {
    size_t s;
    if (x > SIZE_MAX - y) {
        return SIZE_MAX;
    }
    s = x + y;
    return s < 1 ? 1 : s;
}

/* Add every enumerated key into ctx->result; records allocation failure. */
typedef struct {
    hashset *result;
    int ok;
} build_ctx;

static void add_cb(const void *k, size_t kl, const void *v, size_t vl,
                   void *user) {
    build_ctx *c = (build_ctx *)user;
    (void)v;
    (void)vl;
    if (c->ok && !hashset_add(c->result, k, kl)) {
        c->ok = 0;
    }
}

/* Add an enumerated key to ctx->result iff its presence in ctx->other matches
 * ctx->want_present (used for intersection and difference). */
typedef struct {
    hashset *result;
    const hashset *other;
    int want_present;
    int ok;
} filter_ctx;

static void filter_cb(const void *k, size_t kl, const void *v, size_t vl,
                      void *user) {
    filter_ctx *c = (filter_ctx *)user;
    (void)v;
    (void)vl;
    if (!c->ok) {
        return;
    }
    if (hashset_contains(c->other, k, kl) == c->want_present) {
        if (!hashset_add(c->result, k, kl)) {
            c->ok = 0;
        }
    }
}

/* ── set algebra ──────────────────────────────────────────────────────────── */
hashset *hashset_union(const hashset *a, const hashset *b) {
    build_ctx c;
    hashset *r = hashset_new_with(cap_add(hashset_size(a), hashset_size(b)),
                                  hashmap_get_strategy(a->map),
                                  hashmap_get_hash(a->map));
    if (r == NULL) {
        return NULL;
    }
    c.result = r;
    c.ok = 1;
    hashmap_for_each(a->map, add_cb, &c);
    hashmap_for_each(b->map, add_cb, &c);
    if (!c.ok) {
        hashset_free(r);
        return NULL;
    }
    return r;
}

hashset *hashset_intersection(const hashset *a, const hashset *b) {
    const hashset *smaller = hashset_size(a) <= hashset_size(b) ? a : b;
    const hashset *larger = (smaller == a) ? b : a;
    size_t cap = hashset_size(smaller);
    filter_ctx c;
    hashset *r;
    if (cap < 1) {
        cap = 1;
    }
    r = hashset_new_with(cap, hashmap_get_strategy(a->map),
                         hashmap_get_hash(a->map));
    if (r == NULL) {
        return NULL;
    }
    c.result = r;
    c.other = larger;
    c.want_present = 1; /* keep elements that ARE in the other set */
    c.ok = 1;
    hashmap_for_each(smaller->map, filter_cb, &c);
    if (!c.ok) {
        hashset_free(r);
        return NULL;
    }
    return r;
}

hashset *hashset_difference(const hashset *a, const hashset *b) {
    size_t cap = hashset_size(a);
    filter_ctx c;
    hashset *r;
    if (cap < 1) {
        cap = 1;
    }
    r = hashset_new_with(cap, hashmap_get_strategy(a->map),
                         hashmap_get_hash(a->map));
    if (r == NULL) {
        return NULL;
    }
    c.result = r;
    c.other = b;
    c.want_present = 0; /* keep elements NOT in b */
    c.ok = 1;
    hashmap_for_each(a->map, filter_cb, &c);
    if (!c.ok) {
        hashset_free(r);
        return NULL;
    }
    return r;
}

hashset *hashset_symmetric_difference(const hashset *a, const hashset *b) {
    filter_ctx ca, cb;
    hashset *r = hashset_new_with(cap_add(hashset_size(a), hashset_size(b)),
                                  hashmap_get_strategy(a->map),
                                  hashmap_get_hash(a->map));
    if (r == NULL) {
        return NULL;
    }
    ca.result = r;
    ca.other = b;
    ca.want_present = 0;
    ca.ok = 1;
    hashmap_for_each(a->map, filter_cb, &ca); /* in a, not in b */
    cb.result = r;
    cb.other = a;
    cb.want_present = 0;
    cb.ok = 1;
    hashmap_for_each(b->map, filter_cb, &cb); /* in b, not in a */
    if (!ca.ok || !cb.ok) {
        hashset_free(r);
        return NULL;
    }
    return r;
}

/* ── relations ────────────────────────────────────────────────────────────── */
typedef struct {
    const hashset *other;
    int flag; /* running boolean result */
} rel_ctx;

static void all_in_cb(const void *k, size_t kl, const void *v, size_t vl,
                      void *user) {
    rel_ctx *c = (rel_ctx *)user;
    (void)v;
    (void)vl;
    if (!hashset_contains(c->other, k, kl)) {
        c->flag = 0;
    }
}

static void none_in_cb(const void *k, size_t kl, const void *v, size_t vl,
                       void *user) {
    rel_ctx *c = (rel_ctx *)user;
    (void)v;
    (void)vl;
    if (hashset_contains(c->other, k, kl)) {
        c->flag = 0;
    }
}

int hashset_is_subset(const hashset *a, const hashset *b) {
    rel_ctx c;
    if (hashset_size(a) > hashset_size(b)) {
        return 0;
    }
    c.other = b;
    c.flag = 1;
    hashmap_for_each(a->map, all_in_cb, &c);
    return c.flag;
}

int hashset_is_superset(const hashset *a, const hashset *b) {
    return hashset_is_subset(b, a);
}

int hashset_is_disjoint(const hashset *a, const hashset *b) {
    const hashset *smaller = hashset_size(a) <= hashset_size(b) ? a : b;
    const hashset *larger = (smaller == a) ? b : a;
    rel_ctx c;
    c.other = larger;
    c.flag = 1;
    hashmap_for_each(smaller->map, none_in_cb, &c);
    return c.flag;
}

int hashset_equals(const hashset *a, const hashset *b) {
    return hashset_size(a) == hashset_size(b) && hashset_is_subset(a, b);
}
