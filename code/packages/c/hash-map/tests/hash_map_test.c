/* Tests for the C hash map, using the iso_test.h harness. Exercises both
 * collision strategies, all four hash functions, overwrite/delete semantics,
 * tombstone reuse, and load-factor resizing. */
#include "iso_test.h"

#include <stdio.h>
#include <string.h>

#include "hash_map.h"

/* Convenience: set/get string key → string value. */
static int set_str(hashmap *m, const char *k, const char *v) {
    return hashmap_set(m, k, strlen(k), v, strlen(v));
}
static int get_str_eq(const hashmap *m, const char *k, const char *expect) {
    const void *val;
    size_t vlen;
    if (!hashmap_get(m, k, strlen(k), &val, &vlen)) {
        return 0;
    }
    return vlen == strlen(expect) && memcmp(val, expect, vlen) == 0;
}

/* Run the core behavioural suite against a freshly built map. */
static void exercise(hashmap *m) {
    /* Insert and read back. */
    ISO_CHECK(set_str(m, "one", "1"));
    ISO_CHECK(set_str(m, "two", "2"));
    ISO_CHECK(set_str(m, "three", "3"));
    ISO_CHECK_EQ_UINT(hashmap_size(m), 3);
    ISO_CHECK(get_str_eq(m, "one", "1"));
    ISO_CHECK(get_str_eq(m, "two", "2"));
    ISO_CHECK(get_str_eq(m, "three", "3"));
    ISO_CHECK(hashmap_has(m, "one", 3));
    ISO_CHECK(!hashmap_has(m, "missing", 7));

    /* Overwrite keeps the size and replaces the value. */
    ISO_CHECK(set_str(m, "two", "22"));
    ISO_CHECK_EQ_UINT(hashmap_size(m), 3);
    ISO_CHECK(get_str_eq(m, "two", "22"));

    /* Delete removes the key; a second delete is a no-op. */
    ISO_CHECK(hashmap_delete(m, "two", 3));
    ISO_CHECK_EQ_UINT(hashmap_size(m), 2);
    ISO_CHECK(!hashmap_has(m, "two", 3));
    ISO_CHECK(!hashmap_delete(m, "two", 3));

    /* Reinsert after delete (exercises tombstone reuse in open addressing). */
    ISO_CHECK(set_str(m, "two", "200"));
    ISO_CHECK_EQ_UINT(hashmap_size(m), 3);
    ISO_CHECK(get_str_eq(m, "two", "200"));
}

/* Counter callback for hashmap_for_each. */
static void count_cb(const void *k, size_t kl, const void *v, size_t vl,
                     void *user) {
    (void)k;
    (void)kl;
    (void)v;
    (void)vl;
    (*(size_t *)user)++;
}

int main(void) {
    hashmap_strategy strategies[2] = {HASHMAP_CHAINING, HASHMAP_OPEN_ADDRESSING};
    hashmap_hash hashes[4] = {HASHMAP_SIPHASH24, HASHMAP_FNV1A32,
                              HASHMAP_MURMUR3_32, HASHMAP_DJB2};
    int si, hi;

    /* Every (strategy, hash) combination passes the behavioural suite. */
    for (si = 0; si < 2; si++) {
        for (hi = 0; hi < 4; hi++) {
            hashmap *m = hashmap_new(4, strategies[si], hashes[hi]);
            ISO_CHECK(m != NULL);
            ISO_CHECK_EQ_INT(hashmap_get_strategy(m), strategies[si]);
            ISO_CHECK_EQ_INT(hashmap_get_hash(m), hashes[hi]);
            exercise(m);
            hashmap_free(m);
        }
    }

    /* Resizing: inserting many keys grows capacity and preserves every entry. */
    {
        hashmap *m = hashmap_new(4, HASHMAP_CHAINING, HASHMAP_SIPHASH24);
        char kbuf[16], vbuf[16];
        int i;
        int all_found = 1;
        ISO_CHECK(m != NULL);
        for (i = 0; i < 500; i++) {
            snprintf(kbuf, sizeof kbuf, "k-%d", i);
            snprintf(vbuf, sizeof vbuf, "v-%d", i);
            ISO_CHECK(hashmap_set(m, kbuf, strlen(kbuf), vbuf, strlen(vbuf)));
        }
        ISO_CHECK_EQ_UINT(hashmap_size(m), 500);
        ISO_CHECK(hashmap_capacity(m) > 4); /* it grew */
        for (i = 0; i < 500; i++) {
            snprintf(kbuf, sizeof kbuf, "k-%d", i);
            snprintf(vbuf, sizeof vbuf, "v-%d", i);
            if (!get_str_eq(m, kbuf, vbuf)) {
                all_found = 0;
            }
        }
        ISO_CHECK_MSG(all_found, "every key must survive resizing");
        hashmap_free(m);
    }

    /* Open addressing resizes too (0.75 threshold). */
    {
        hashmap *m = hashmap_new(4, HASHMAP_OPEN_ADDRESSING, HASHMAP_MURMUR3_32);
        char kbuf[16];
        int i;
        int all_found = 1;
        ISO_CHECK(m != NULL);
        for (i = 0; i < 300; i++) {
            snprintf(kbuf, sizeof kbuf, "key%d", i);
            ISO_CHECK(hashmap_set(m, kbuf, strlen(kbuf), &i, sizeof i));
        }
        ISO_CHECK_EQ_UINT(hashmap_size(m), 300);
        for (i = 0; i < 300; i++) {
            const void *val;
            size_t vlen;
            snprintf(kbuf, sizeof kbuf, "key%d", i);
            if (!hashmap_get(m, kbuf, strlen(kbuf), &val, &vlen) ||
                vlen != sizeof i || memcmp(val, &i, sizeof i) != 0) {
                all_found = 0;
            }
        }
        ISO_CHECK_MSG(all_found, "open-addressing entries survive resizing");
        ISO_CHECK(hashmap_load_factor(m) <= 0.75);
        hashmap_free(m);
    }

    /* for_each visits exactly `size` entries. */
    {
        hashmap *m = hashmap_new(4, HASHMAP_OPEN_ADDRESSING, HASHMAP_DJB2);
        size_t count = 0;
        ISO_CHECK(m != NULL);
        set_str(m, "a", "1");
        set_str(m, "b", "2");
        set_str(m, "c", "3");
        hashmap_delete(m, "b", 1); /* leaves a tombstone, not visited */
        hashmap_for_each(m, count_cb, &count);
        ISO_CHECK_EQ_UINT(count, hashmap_size(m));
        ISO_CHECK_EQ_UINT(count, 2);
        hashmap_free(m);
    }

    /* Empty-key and empty-value are valid. */
    {
        hashmap *m = hashmap_new(2, HASHMAP_CHAINING, HASHMAP_DJB2);
        const void *val;
        size_t vlen;
        ISO_CHECK(m != NULL);
        ISO_CHECK(hashmap_set(m, "", 0, "", 0));
        ISO_CHECK(hashmap_get(m, "", 0, &val, &vlen));
        ISO_CHECK_EQ_UINT(vlen, 0);
        ISO_CHECK(hashmap_has(m, "", 0));
        hashmap_free(m);
    }

    return ISO_TEST_RESULT();
}
