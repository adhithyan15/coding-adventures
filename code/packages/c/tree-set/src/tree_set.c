/*
 * tree_set.c — implementation of the ordered set (see tree_set.h). A faithful
 * port of the Rust `tree-set` crate: membership/order queries delegate to the
 * avl-tree backend, and the set algebra is the crate's linear merge over the
 * two operands' sorted sequences.
 */
#include "tree_set.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */

/* ---- construction / destruction --------------------------------------- */

TreeSet *tset_empty(void) {
    TreeSet *s = malloc(sizeof *s);
    if (!s) {
        return NULL;
    }
    s->backend = avl_empty();
    if (!s->backend) {
        free(s);
        return NULL;
    }
    return s;
}

void tset_free(TreeSet *s) {
    if (!s) {
        return;
    }
    avl_free(s->backend);
    free(s);
}

/* Wrap an owned backend into a new set handle; frees the backend on failure. */
static TreeSet *set_with_backend(AVLTree *backend) {
    TreeSet *s = malloc(sizeof *s);
    if (!s) {
        avl_free(backend);
        return NULL;
    }
    s->backend = backend;
    return s;
}

TreeSet *tset_from_array(const int *values, size_t n) {
    TreeSet *s = tset_empty();
    size_t i;
    if (!s) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        TreeSet *next = tset_insert(s, values[i]);
        tset_free(s);
        if (!next) {
            return NULL;
        }
        s = next;
    }
    return s;
}

/* ---- persistent updates ----------------------------------------------- */

TreeSet *tset_insert(const TreeSet *s, int value) {
    AVLTree *nb = avl_insert(s->backend, value);
    if (!nb) {
        return NULL;
    }
    return set_with_backend(nb);
}

TreeSet *tset_remove(const TreeSet *s, int value) {
    AVLTree *nb = avl_delete(s->backend, value);
    if (!nb) {
        return NULL;
    }
    return set_with_backend(nb);
}

/* ---- membership & order queries (delegate to the backend) ------------- */

size_t tset_size(const TreeSet *s) { return avl_size(s->backend); }

int tset_is_empty(const TreeSet *s) { return tset_size(s) == 0; }

int tset_contains(const TreeSet *s, int value) {
    return avl_contains(s->backend, value);
}

int tset_min_value(const TreeSet *s, int *out) {
    return avl_min_value(s->backend, out);
}

int tset_max_value(const TreeSet *s, int *out) {
    return avl_max_value(s->backend, out);
}

int tset_predecessor(const TreeSet *s, int value, int *out) {
    return avl_predecessor(s->backend, value, out);
}

int tset_successor(const TreeSet *s, int value, int *out) {
    return avl_successor(s->backend, value, out);
}

int tset_kth_smallest(const TreeSet *s, size_t k, int *out) {
    return avl_kth_smallest(s->backend, k, out);
}

size_t tset_rank(const TreeSet *s, int value) {
    return avl_rank(s->backend, value);
}

size_t tset_to_sorted_array(const TreeSet *s, int *buf, size_t buf_len) {
    return avl_to_sorted_array(s->backend, buf, buf_len);
}

size_t tset_range(const TreeSet *s, int min, int max, int inclusive, int *buf,
                  size_t buf_len) {
    size_t n, k, count = 0;
    if (min > max || !buf || buf_len == 0) {
        return 0;
    }
    n = tset_size(s);
    for (k = 1; k <= n; k++) {
        int v = 0;
        int in;
        avl_kth_smallest(s->backend, k, &v); /* k in [1, n] always found */
        if (inclusive) {
            in = (v >= min && v <= max);
        } else {
            in = (v > min && v < max);
        }
        if (in) {
            if (count < buf_len) {
                buf[count++] = v;
            } else {
                break;
            }
        }
    }
    return count;
}

/* ---- set algebra ------------------------------------------------------ */

/* Materialize a set's elements (ascending). On success returns 1, sets *arr to
 * a malloc'd array (NULL when empty; caller frees) and *n to the count. Returns
 * 0 on allocation failure. */
static int set_sorted(const TreeSet *s, int **arr, size_t *n) {
    size_t sz = tset_size(s);
    *arr = NULL;
    *n = sz;
    if (sz == 0) {
        return 1;
    }
    {
        int *a = calloc(sz, sizeof *a); /* calloc does the checked multiply */
        if (!a) {
            return 0;
        }
        tset_to_sorted_array(s, a, sz);
        *arr = a;
    }
    return 1;
}

typedef enum { OP_UNION, OP_INTERSECT, OP_DIFF, OP_SYMDIFF } SetOp;

/* The shared two-pointer merge behind union / intersection / difference /
 * symmetric-difference, exactly mirroring the crate's *_sorted helpers. */
static TreeSet *set_algebra(const TreeSet *a, const TreeSet *b, SetOp op) {
    int *la = NULL, *lb = NULL, *merged = NULL;
    size_t na = 0, nb = 0, i = 0, j = 0, c = 0;
    TreeSet *result;

    if (!set_sorted(a, &la, &na)) {
        return NULL;
    }
    if (!set_sorted(b, &lb, &nb)) {
        free(la);
        return NULL;
    }
    if (na > SIZE_MAX - nb) { /* guard the result-size addition */
        free(la);
        free(lb);
        return NULL;
    }
    if (na + nb > 0) {
        merged = calloc(na + nb, sizeof *merged);
        if (!merged) {
            free(la);
            free(lb);
            return NULL;
        }
    }

    while (i < na && j < nb) {
        if (la[i] < lb[j]) {
            if (op != OP_INTERSECT) { /* union, diff, symdiff keep left-smaller */
                merged[c++] = la[i];
            }
            i++;
        } else if (la[i] > lb[j]) {
            if (op == OP_UNION || op == OP_SYMDIFF) { /* keep right-smaller */
                merged[c++] = lb[j];
            }
            j++;
        } else { /* equal */
            if (op == OP_UNION || op == OP_INTERSECT) { /* keep the common one */
                merged[c++] = la[i];
            }
            i++;
            j++;
        }
    }
    if (op == OP_UNION || op == OP_DIFF || op == OP_SYMDIFF) {
        while (i < na) {
            merged[c++] = la[i++];
        }
    }
    if (op == OP_UNION || op == OP_SYMDIFF) {
        while (j < nb) {
            merged[c++] = lb[j++];
        }
    }

    free(la);
    free(lb);
    result = tset_from_array(merged, c); /* rebuild through the backend */
    free(merged);
    return result;
}

TreeSet *tset_union(const TreeSet *a, const TreeSet *b) {
    return set_algebra(a, b, OP_UNION);
}

TreeSet *tset_intersection(const TreeSet *a, const TreeSet *b) {
    return set_algebra(a, b, OP_INTERSECT);
}

TreeSet *tset_difference(const TreeSet *a, const TreeSet *b) {
    return set_algebra(a, b, OP_DIFF);
}

TreeSet *tset_symmetric_difference(const TreeSet *a, const TreeSet *b) {
    return set_algebra(a, b, OP_SYMDIFF);
}

/* ---- set relations (enumerate `a` via the backend; no allocation) ----- */

int tset_is_subset(const TreeSet *a, const TreeSet *b) {
    size_t na = tset_size(a), k;
    for (k = 1; k <= na; k++) {
        int v = 0;
        avl_kth_smallest(a->backend, k, &v);
        if (!tset_contains(b, v)) {
            return 0;
        }
    }
    return 1;
}

int tset_is_superset(const TreeSet *a, const TreeSet *b) {
    return tset_is_subset(b, a);
}

int tset_is_disjoint(const TreeSet *a, const TreeSet *b) {
    size_t na = tset_size(a), k;
    for (k = 1; k <= na; k++) {
        int v = 0;
        avl_kth_smallest(a->backend, k, &v);
        if (tset_contains(b, v)) {
            return 0;
        }
    }
    return 1;
}

int tset_equals(const TreeSet *a, const TreeSet *b) {
    size_t na = tset_size(a), nb = tset_size(b), k;
    if (na != nb) {
        return 0;
    }
    for (k = 1; k <= na; k++) {
        int va = 0, vb = 0;
        avl_kth_smallest(a->backend, k, &va);
        avl_kth_smallest(b->backend, k, &vb);
        if (va != vb) {
            return 0;
        }
    }
    return 1;
}
