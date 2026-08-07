/*
 * hash_set.h — a hash set for byte-string elements, in pure ISO C17. A faithful
 * port of the Rust `hash-set` crate (DT19).
 * ===========================================================================
 *
 * Just like the Rust crate — "a zero-cost wrapper around the DT18 hash map:
 * HashSet<T> is stored as HashMap<T, ()>" — this set is a thin layer over the
 * sibling `hash-map` package: each element is a key with an empty value. That
 * gives O(1) membership plus the full complement of set algebra.
 *
 *   hashset_add / hashset_remove / hashset_contains — membership
 *   hashset_union / _intersection / _difference / _symmetric_difference
 *       — build and return a NEW set (caller frees it)
 *   hashset_is_subset / _is_superset / _is_disjoint / _equals — relations
 *
 * Elements are arbitrary byte strings; the set copies and owns them.
 *
 * Ownership: hashset_new* allocates; pair with hashset_free. The set-algebra
 * functions allocate a fresh set (NULL on allocation failure) that the caller
 * must free.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef HASH_SET_H
#define HASH_SET_H

#include <stddef.h> /* size_t */

#include "hash_map.h" /* hashmap_strategy, hashmap_hash */

typedef struct hashset hashset;

/* hashset_new — an empty set with sensible defaults (capacity 16, chaining,
 * SipHash-2-4). Returns NULL on allocation failure. */
hashset *hashset_new(void);

/* hashset_new_with — an empty set with an explicit initial capacity, collision
 * strategy, and hash function. Returns NULL on allocation failure. */
hashset *hashset_new_with(size_t capacity, hashmap_strategy strategy,
                          hashmap_hash hash);

/* hashset_free — release the set and every element copy it owns. NULL-safe. */
void hashset_free(hashset *set);

/* hashset_add — insert `elem` (a no-op if already present). Returns 1 on
 * success, 0 on allocation failure. */
int hashset_add(hashset *set, const void *elem, size_t elem_len);

/* hashset_remove — remove `elem`. Returns 1 if it was present, else 0. */
int hashset_remove(hashset *set, const void *elem, size_t elem_len);

/* hashset_contains — 1 if `elem` is a member, else 0. */
int hashset_contains(const hashset *set, const void *elem, size_t elem_len);

/* hashset_size — number of elements. */
size_t hashset_size(const hashset *set);

/* hashset_is_empty — 1 if the set has no elements. */
int hashset_is_empty(const hashset *set);

/* Enumeration: invoke `fn` once per element (borrowed pointer, valid only during
 * the call), in unspecified order. */
typedef void (*hashset_iter_fn)(const void *elem, size_t elem_len, void *user);
void hashset_for_each(const hashset *set, hashset_iter_fn fn, void *user);

/* Set algebra — each returns a NEW set (NULL on allocation failure). The result
 * uses the strategy/hash of `a`. */
hashset *hashset_union(const hashset *a, const hashset *b);
hashset *hashset_intersection(const hashset *a, const hashset *b);
hashset *hashset_difference(const hashset *a, const hashset *b);
hashset *hashset_symmetric_difference(const hashset *a, const hashset *b);

/* Relations. */
int hashset_is_subset(const hashset *a, const hashset *b);
int hashset_is_superset(const hashset *a, const hashset *b);
int hashset_is_disjoint(const hashset *a, const hashset *b);
int hashset_equals(const hashset *a, const hashset *b);

#endif /* HASH_SET_H */
