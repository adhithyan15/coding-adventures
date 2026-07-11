/*
 * hash_map.h — a hash map built from scratch, in pure ISO C17. A faithful port
 * of the Rust `hash-map` crate (DT18).
 * ===========================================================================
 *
 * A hash map (a.k.a. dictionary / associative array) stores key → value pairs
 * and looks them up in expected O(1) time. This implementation offers the two
 * classic collision-resolution strategies:
 *
 *   • Chaining          — each bucket holds a linked list of entries that hash
 *                         to it. Resizes when the load factor exceeds 1.0.
 *   • Open addressing   — one flat array of slots; on a collision we probe the
 *                         next slots linearly. Deletions leave "tombstones" so
 *                         probe chains stay intact. Resizes above load 0.75.
 *
 * and four selectable hash functions (SipHash-2-4 — the default — FNV-1a-32,
 * MurmurHash3-32, and djb2), matching the Rust crate.
 *
 * Keys and values are arbitrary byte strings: the map copies the bytes you give
 * it, owns those copies, and frees them. (Rust's map is generic over owned
 * K, V; a byte-string map is the natural C equivalent — see the note in the .c
 * on how keys are hashed.)
 *
 * Ownership: hashmap_new allocates; pair it with hashmap_free. Every returned
 * value pointer is borrowed from the map and valid until the next mutation.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef HASH_MAP_H
#define HASH_MAP_H

#include <stddef.h> /* size_t */

/* Collision-resolution strategy. */
typedef enum {
    HASHMAP_CHAINING,
    HASHMAP_OPEN_ADDRESSING
} hashmap_strategy;

/* Which hash function keys are run through. */
typedef enum {
    HASHMAP_SIPHASH24,
    HASHMAP_FNV1A32,
    HASHMAP_MURMUR3_32,
    HASHMAP_DJB2
} hashmap_hash;

/* Opaque map handle. */
typedef struct hashmap hashmap;

/* hashmap_new — allocate an empty map with the given initial `capacity`
 * (clamped to at least 1), collision `strategy`, and `hash` function. Returns
 * NULL on allocation failure. Pair with hashmap_free. */
hashmap *hashmap_new(size_t capacity, hashmap_strategy strategy,
                     hashmap_hash hash);

/* hashmap_free — release the map and every key/value copy it owns. NULL-safe. */
void hashmap_free(hashmap *map);

/* hashmap_set — insert or overwrite the value for `key`. The key and value
 * bytes are copied. Returns 1 on success, 0 on allocation failure (the map is
 * left unchanged on failure). */
int hashmap_set(hashmap *map, const void *key, size_t key_len, const void *value,
                size_t value_len);

/* hashmap_get — look up `key`. On a hit, writes a borrowed pointer to the value
 * bytes into *value_out and the length into *value_len_out (either may be NULL)
 * and returns 1. Returns 0 if the key is absent. The pointer is valid until the
 * next mutating call. */
int hashmap_get(const hashmap *map, const void *key, size_t key_len,
                const void **value_out, size_t *value_len_out);

/* hashmap_has — 1 if `key` is present, else 0. */
int hashmap_has(const hashmap *map, const void *key, size_t key_len);

/* hashmap_delete — remove `key`. Returns 1 if it was present, else 0. */
int hashmap_delete(hashmap *map, const void *key, size_t key_len);

/* hashmap_iter_fn — callback for hashmap_for_each. Receives borrowed pointers to
 * one entry's key and value (valid only for the duration of the call), plus the
 * caller's `user` pointer. */
typedef void (*hashmap_iter_fn)(const void *key, size_t key_len,
                                const void *value, size_t value_len,
                                void *user);

/* hashmap_for_each — invoke `fn` once for every entry, in unspecified order.
 * This is the C equivalent of the Rust crate's entries()/keys()/values(). */
void hashmap_for_each(const hashmap *map, hashmap_iter_fn fn, void *user);

/* Accessors. */
size_t hashmap_size(const hashmap *map);
size_t hashmap_capacity(const hashmap *map);
double hashmap_load_factor(const hashmap *map);
hashmap_strategy hashmap_get_strategy(const hashmap *map);
hashmap_hash hashmap_get_hash(const hashmap *map);

#endif /* HASH_MAP_H */
