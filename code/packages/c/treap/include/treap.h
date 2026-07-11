/*
 * treap.h — a treap (tree + heap), a randomized balanced BST, in pure ISO C17.
 * A faithful port of the Rust `treap` crate (DT10).
 * ===========================================================================
 *
 * A treap stores each key together with a random `priority` and keeps two
 * invariants at once:
 *
 *   - BST order on the KEYS: left subtree < node < right subtree.
 *   - MAX-HEAP order on the PRIORITIES: every node's priority is >= its
 *     children's priorities.
 *
 * Because the priorities are random, the heap constraint forces a shape that is
 * balanced *in expectation* — O(log n) search / insert / delete with high
 * probability — without the explicit rebalancing an AVL or red-black tree needs.
 * Rotations during insert, and a priority-ordered `merge` during delete, restore
 * the heap invariant.
 *
 *   split(key)  -> (<= key, > key)      merge(l, r)  (all l-keys < all r-keys)
 *
 * `split` and `merge` are the treap's signature operations and run in O(log n).
 *
 * Each node caches its subtree `size`, which makes `treap_kth_smallest` and the
 * order statistics O(h).
 *
 * PRIORITIES. `treap_insert` takes an optional priority (`const double *`): pass
 * a pointer to use a specific priority, or NULL to draw one from a built-in
 * deterministic PRNG. NOTE: the Rust crate seeds that PRNG through a global
 * AtomicU32 for cross-thread safety; this port uses a plain `static` counter
 * (identical arithmetic, single-threaded). If you need reproducibility or
 * thread-safety, supply priorities explicitly.
 *
 * PERSISTENCE. Like the Rust crate, updates are *persistent*: `treap_insert`,
 * `treap_delete`, `treap_split`, and `treap_merge` return NEW treaps and leave
 * their inputs untouched (they deep-copy, then work on the copy). Every treap
 * you obtain must be released with `treap_free`.
 *
 * CAVEAT. All operations recurse to the treap's height. With random priorities
 * the height is O(log n) in expectation, but a caller supplying adversarial
 * explicit priorities (e.g. a monotonic sequence) can force a height-n
 * degenerate chain and overflow the stack. Draw priorities from the built-in
 * PRNG (pass NULL) for untrusted input. This mirrors the Rust crate.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Key type is `int`.
 */
#ifndef TREAP_H
#define TREAP_H

#include <stddef.h> /* size_t */

/* A treap node. Fields are exposed (as in the Rust crate) so callers can walk
 * the structure. */
typedef struct TreapNode TreapNode;
struct TreapNode {
    int key;
    double priority;
    TreapNode *left;
    TreapNode *right;
    size_t size; /* number of nodes in this subtree (>= 1) */
};

/* An owning handle to a treap (possibly empty). */
typedef struct {
    TreapNode *root;
} Treap;

/* ---- construction / destruction --------------------------------------- */

/* treap_empty — a new empty treap. Returns NULL on allocation failure. */
Treap *treap_empty(void);

/* treap_free — free a treap and all its nodes. Safe to call with NULL. */
void treap_free(Treap *t);

/* ---- persistent updates (return a NEW treap; inputs are unchanged) ----- */

/* treap_insert — a new treap with `key` added. If `priority` is non-NULL its
 * pointed-to value is used; if NULL, a priority is drawn from the built-in PRNG.
 * Re-inserting an existing key is a no-op. Returns NULL on allocation failure. */
Treap *treap_insert(const Treap *t, int key, const double *priority);

/* treap_delete — a new treap with `key` removed if present. NULL on failure. */
Treap *treap_delete(const Treap *t, int key);

/* treap_split — split into (keys <= `key`) and (keys > `key`). Writes freshly
 * allocated treaps to *left_out and *right_out and returns 1; on allocation
 * failure writes nothing, frees any partial result, and returns 0. */
int treap_split(const Treap *t, int key, Treap **left_out, Treap **right_out);

/* treap_merge — a new treap combining `left` and `right`, which must be
 * key-disjoint with every key of `left` strictly less than every key of
 * `right` (as produced by treap_split). Inputs are unchanged. NULL on failure. */
Treap *treap_merge(const Treap *left, const Treap *right);

/* ---- queries (read-only; do not allocate) ----------------------------- */

const TreapNode *treap_search(const Treap *t, int key);
int treap_contains(const Treap *t, int key);

/* Extremes / neighbours: write to *out and return 1, or return 0 if absent. */
int treap_min_key(const Treap *t, int *out);
int treap_max_key(const Treap *t, int *out);
int treap_predecessor(const Treap *t, int key, int *out);
int treap_successor(const Treap *t, int key, int *out);

/* treap_kth_smallest — the k-th smallest key (k is 1-based); writes *out and
 * returns 1, or 0 if k == 0 or k > size. */
int treap_kth_smallest(const Treap *t, size_t k, int *out);

/* treap_to_sorted_array — copy keys in ascending order into `buf` (capacity
 * buf_len); returns the number written, min(size, buf_len). */
size_t treap_to_sorted_array(const Treap *t, int *buf, size_t buf_len);

size_t treap_size(const Treap *t);

/* treap_height — height of the treap: -1 when empty, 0 for a single node. */
long treap_height(const Treap *t);

/* treap_is_valid — 1 iff both the BST (key) and max-heap (priority) invariants
 * hold and the cached sizes are consistent. */
int treap_is_valid(const Treap *t);

#endif /* TREAP_H */
