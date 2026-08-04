/*
 * suffix_tree.c — implementation of the suffix index (see suffix_tree.h). Like
 * the Rust crate, the "tree" is a simple root-with-one-leaf-per-suffix, so the
 * only state we keep is a copy of the text; the queries are direct scans and
 * dynamic-programming routines over it.
 */
#include "suffix_tree.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy, memcmp */

struct suffix_tree {
    char *text;
    size_t len;
};

suffix_tree *suffix_tree_build(const char *text, size_t len) {
    suffix_tree *t = (suffix_tree *)malloc(sizeof *t);
    if (t == NULL) {
        return NULL;
    }
    t->text = (char *)malloc(len ? len : 1);
    if (t->text == NULL) {
        free(t);
        return NULL;
    }
    if (len) {
        memcpy(t->text, text, len);
    }
    t->len = len;
    return t;
}

void suffix_tree_free(suffix_tree *tree) {
    if (tree == NULL) {
        return;
    }
    free(tree->text);
    free(tree);
}

size_t suffix_tree_text_len(const suffix_tree *tree) { return tree->len; }

size_t suffix_tree_node_count(const suffix_tree *tree) {
    /* Root plus one leaf per byte (matches the crate's 1 + children count). */
    return 1 + tree->len;
}

size_t suffix_tree_search(const suffix_tree *tree, const char *pattern,
                          size_t plen, size_t *out, size_t out_cap) {
    size_t count = 0;
    size_t start;

    if (plen == 0) {
        /* An empty pattern matches at every position 0..=len. */
        for (start = 0; start <= tree->len; start++) {
            if (out != NULL && count < out_cap) {
                out[count] = start;
            }
            count++;
        }
        return count;
    }
    if (plen > tree->len) {
        return 0;
    }
    /* start ranges 0..=(len-plen); len-plen is safe since plen <= len, and this
     * form avoids the start+plen overflow a naive bound would risk. */
    for (start = 0; start <= tree->len - plen; start++) {
        if (memcmp(tree->text + start, pattern, plen) == 0) {
            if (out != NULL && count < out_cap) {
                out[count] = start;
            }
            count++;
        }
    }
    return count;
}

size_t suffix_tree_count_occurrences(const suffix_tree *tree,
                                     const char *pattern, size_t plen) {
    return suffix_tree_search(tree, pattern, plen, NULL, 0);
}

size_t suffix_tree_longest_repeated_substring(const suffix_tree *tree, char *out,
                                              size_t out_cap) {
    size_t n = tree->len;
    size_t best_len = 0;
    size_t best_start = 0;
    size_t i, j, w;

    /* For every pair of suffixes (i < j), measure their common prefix; keep the
     * longest. Strict '>' means ties keep the earliest, as in the crate. */
    for (i = 0; i < n; i++) {
        for (j = i + 1; j < n; j++) {
            size_t k = 0;
            while (j + k < n && tree->text[i + k] == tree->text[j + k]) {
                k++;
            }
            if (k > best_len) {
                best_len = k;
                best_start = i;
            }
        }
    }

    w = best_len < out_cap ? best_len : out_cap;
    if (out != NULL && w > 0) {
        memcpy(out, tree->text + best_start, w);
    }
    return best_len;
}

int suffix_tree_suffix(const suffix_tree *tree, size_t i, const char **ptr,
                       size_t *suffix_len) {
    if (i >= tree->len) {
        return 0;
    }
    if (ptr != NULL) {
        *ptr = tree->text + i;
    }
    if (suffix_len != NULL) {
        *suffix_len = tree->len - i;
    }
    return 1;
}

size_t suffix_longest_common_substring(const char *a, size_t alen, const char *b,
                                       size_t blen, char *out, size_t out_cap) {
    size_t *prev;
    size_t *cur;
    size_t best_len = 0;
    size_t best_end = 0; /* 1-based end index in `a` of the best match */
    size_t i, j, w;

    if (alen == 0 || blen == 0) {
        return 0;
    }
    if (blen > SIZE_MAX - 1) {
        return 0; /* blen + 1 would overflow */
    }
    /* Rolling dynamic programming: dp[i][j] = length of the common suffix of
     * a[..i] and b[..j]. Two rows suffice; every cell of `cur` is written each
     * row, so no stale values survive the swap. */
    prev = (size_t *)calloc(blen + 1, sizeof(size_t));
    cur = (size_t *)calloc(blen + 1, sizeof(size_t));
    if (prev == NULL || cur == NULL) {
        free(prev);
        free(cur);
        return 0;
    }

    for (i = 1; i <= alen; i++) {
        for (j = 1; j <= blen; j++) {
            if (a[i - 1] == b[j - 1]) {
                cur[j] = prev[j - 1] + 1;
                if (cur[j] > best_len) {
                    best_len = cur[j];
                    best_end = i;
                }
            } else {
                cur[j] = 0;
            }
        }
        {
            size_t *tmp = prev;
            prev = cur;
            cur = tmp; /* prev now holds row i; cur is overwritten next row */
        }
    }

    free(prev);
    free(cur);

    w = best_len < out_cap ? best_len : out_cap;
    if (out != NULL && w > 0) {
        memcpy(out, a + (best_end - best_len), w);
    }
    return best_len;
}
