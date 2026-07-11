/*
 * skip_list.c — implementation of the ordered-map "skip list". Keys are held in
 * a sorted array; lookups/rank use binary search, insert/delete shift to keep
 * the array sorted. Ported from the Rust `skip-list` crate (which is likewise an
 * ordered map with reported skip-list parameters).
 */
#include "skip_list.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, realloc, free */

int skiplist_init_with_params(skiplist *s, size_t max_level, double probability) {
    s->entries = NULL;
    s->size = 0;
    s->cap = 0;
    s->max_level = max_level < 1 ? 1 : max_level;
    /* Valid probability is finite and in (0, 1); otherwise default to 0.5. */
    if (probability > 0.0 && probability < 1.0) {
        s->probability = probability;
    } else {
        s->probability = 0.5;
    }
    s->current_max = 1;
    return 1;
}

int skiplist_init(skiplist *s) {
    return skiplist_init_with_params(s, 32, 0.5);
}

void skiplist_free(skiplist *s) {
    free(s->entries);
    s->entries = NULL;
    s->size = 0;
    s->cap = 0;
    s->current_max = 1;
}

/* lower_bound — index of the first entry whose key is >= `key` (== size if all
 * keys are smaller). If `found` is non-NULL, set it to 1 iff key is present. */
static size_t lower_bound(const skiplist *s, int key, int *found) {
    size_t lo = 0, hi = s->size;
    if (found != NULL) {
        *found = 0;
    }
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (s->entries[mid].key < key) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if (found != NULL && lo < s->size && s->entries[lo].key == key) {
        *found = 1;
    }
    return lo;
}

/* estimated_current_max — ceil(log_base(len)) clamped to [1, max_level], where
 * base = 1/probability. Computed with a multiplicative loop so no <math.h> /
 * -lm is needed: the smallest L with base^L >= len. */
static size_t estimated_current_max(const skiplist *s) {
    double base, acc;
    size_t levels;
    if (s->size == 0) {
        return 1;
    }
    base = 1.0 / s->probability;
    acc = 1.0;
    levels = 0;
    while (acc < (double)s->size) {
        acc *= base;
        levels++;
    }
    if (levels < 1) {
        levels = 1;
    }
    if (levels > s->max_level) {
        levels = s->max_level;
    }
    return levels;
}

static int ensure_capacity(skiplist *s) {
    size_t new_cap;
    skiplist_entry *grown;
    if (s->size < s->cap) {
        return 1;
    }
    new_cap = s->cap == 0 ? 8 : s->cap * 2;
    if (s->cap > SIZE_MAX / 2 || new_cap > SIZE_MAX / sizeof(skiplist_entry)) {
        return 0;
    }
    grown = (skiplist_entry *)realloc(s->entries, new_cap * sizeof(skiplist_entry));
    if (grown == NULL) {
        return 0;
    }
    s->entries = grown;
    s->cap = new_cap;
    return 1;
}

int skiplist_insert(skiplist *s, int key, int value) {
    int found;
    size_t pos = lower_bound(s, key, &found);
    if (found) {
        s->entries[pos].value = value; /* overwrite */
        return 1;
    }
    if (!ensure_capacity(s)) {
        return 0;
    }
    /* Shift the tail up one slot to open room at `pos`, then insert. */
    if (pos < s->size) {
        size_t i;
        for (i = s->size; i > pos; i--) {
            s->entries[i] = s->entries[i - 1];
        }
    }
    s->entries[pos].key = key;
    s->entries[pos].value = value;
    s->size++;
    s->current_max = estimated_current_max(s);
    return 1;
}

int skiplist_delete(skiplist *s, int key) {
    int found;
    size_t pos = lower_bound(s, key, &found);
    size_t i;
    if (!found) {
        return 0;
    }
    for (i = pos; i + 1 < s->size; i++) {
        s->entries[i] = s->entries[i + 1];
    }
    s->size--;
    s->current_max = estimated_current_max(s);
    return 1;
}

int skiplist_search(const skiplist *s, int key, int *out) {
    int found;
    size_t pos = lower_bound(s, key, &found);
    if (found) {
        *out = s->entries[pos].value;
        return 1;
    }
    return 0;
}

int skiplist_contains(const skiplist *s, int key) {
    int found;
    (void)lower_bound(s, key, &found);
    return found;
}

int skiplist_rank(const skiplist *s, int key, size_t *out_rank) {
    int found;
    size_t pos = lower_bound(s, key, &found);
    if (found) {
        *out_rank = pos;
        return 1;
    }
    return 0;
}

int skiplist_by_rank(const skiplist *s, size_t rank, int *out_key) {
    if (rank >= s->size) {
        return 0;
    }
    *out_key = s->entries[rank].key;
    return 1;
}

int skiplist_min(const skiplist *s, int *out) {
    if (s->size == 0) {
        return 0;
    }
    *out = s->entries[0].key;
    return 1;
}

int skiplist_max(const skiplist *s, int *out) {
    if (s->size == 0) {
        return 0;
    }
    *out = s->entries[s->size - 1].key;
    return 1;
}

size_t skiplist_len(const skiplist *s) { return s->size; }
int skiplist_is_empty(const skiplist *s) { return s->size == 0 ? 1 : 0; }
size_t skiplist_max_level(const skiplist *s) { return s->max_level; }
size_t skiplist_current_max(const skiplist *s) { return s->current_max; }
double skiplist_probability(const skiplist *s) { return s->probability; }

void skiplist_foreach(const skiplist *s, skiplist_visit_fn visit, void *ud) {
    size_t i;
    for (i = 0; i < s->size; i++) {
        visit(s->entries[i].key, s->entries[i].value, ud);
    }
}

void skiplist_range(const skiplist *s, int lo, int hi, int inclusive,
                    skiplist_visit_fn visit, void *ud) {
    size_t i;
    if (lo > hi) {
        return;
    }
    for (i = 0; i < s->size; i++) {
        int k = s->entries[i].key;
        if (k < lo || (k == lo && !inclusive)) {
            continue;
        }
        if (k > hi || (k == hi && !inclusive)) {
            break;
        }
        visit(k, s->entries[i].value, ud);
    }
}
