/*
 * binary_tree.c — implementation of the generic binary tree (see binary_tree.h).
 * A faithful port of the Rust `binary-tree` crate: the same level-order build,
 * traversals, shape predicates, and indented ASCII rendering.
 */
#include "binary_tree.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, strlen */

/* ---- node helpers ----------------------------------------------------- */

BinaryTreeNode *bt_node_new(int value) {
    BinaryTreeNode *n = malloc(sizeof *n);
    if (!n) {
        return NULL;
    }
    n->value = value;
    n->left = NULL;
    n->right = NULL;
    return n;
}

void bt_free_node(BinaryTreeNode *n) {
    if (!n) {
        return;
    }
    bt_free_node(n->left);
    bt_free_node(n->right);
    free(n);
}

/* ---- construction / destruction --------------------------------------- */

BinaryTree *bt_new(void) {
    BinaryTree *t = malloc(sizeof *t);
    if (!t) {
        return NULL;
    }
    t->root = NULL;
    return t;
}

BinaryTree *bt_with_root(BinaryTreeNode *root) {
    BinaryTree *t = malloc(sizeof *t);
    if (!t) {
        return NULL; /* leave `root` for the caller to free/retry */
    }
    t->root = root;
    return t;
}

/* Recursively build the node at level-order `index`; NULL for a gap or an
 * out-of-range index. On allocation failure sets *ok = 0. */
static BinaryTreeNode *build_level_order(const int *values, const int *present,
                                        size_t n, size_t index, int *ok) {
    BinaryTreeNode *node;
    if (index >= n || !present[index]) {
        return NULL;
    }
    node = bt_node_new(values[index]);
    if (!node) {
        *ok = 0;
        return NULL;
    }
    /* 2*index+1 / +2 cannot overflow here: index < n <= SIZE_MAX and a real
     * level-order array is far smaller, but guard anyway. */
    if (index <= (SIZE_MAX - 1) / 2) {
        node->left = build_level_order(values, present, n, 2 * index + 1, ok);
    }
    if (!*ok) {
        bt_free_node(node);
        return NULL;
    }
    if (index <= (SIZE_MAX - 2) / 2) {
        node->right = build_level_order(values, present, n, 2 * index + 2, ok);
    }
    if (!*ok) {
        bt_free_node(node);
        return NULL;
    }
    return node;
}

BinaryTree *bt_from_level_order(const int *values, const int *present,
                                size_t n) {
    BinaryTree *t = malloc(sizeof *t);
    int ok = 1;
    if (!t) {
        return NULL;
    }
    t->root = build_level_order(values, present, n, 0, &ok);
    if (!ok) {
        bt_free_node(t->root);
        free(t);
        return NULL;
    }
    return t;
}

void bt_free(BinaryTree *t) {
    if (!t) {
        return;
    }
    bt_free_node(t->root);
    free(t);
}

/* ---- accessors -------------------------------------------------------- */

const BinaryTreeNode *bt_root(const BinaryTree *t) {
    return t ? t->root : NULL;
}

static const BinaryTreeNode *find_rec(const BinaryTreeNode *n, int value) {
    const BinaryTreeNode *r;
    if (!n) {
        return NULL;
    }
    if (n->value == value) {
        return n;
    }
    r = find_rec(n->left, value);
    if (r) {
        return r;
    }
    return find_rec(n->right, value);
}

const BinaryTreeNode *bt_find(const BinaryTree *t, int value) {
    return find_rec(t ? t->root : NULL, value);
}

const BinaryTreeNode *bt_left_child(const BinaryTree *t, int value) {
    const BinaryTreeNode *n = bt_find(t, value);
    return n ? n->left : NULL;
}

const BinaryTreeNode *bt_right_child(const BinaryTree *t, int value) {
    const BinaryTreeNode *n = bt_find(t, value);
    return n ? n->right : NULL;
}

/* ---- height / size ---------------------------------------------------- */

static long height_rec(const BinaryTreeNode *n) {
    long lh, rh;
    if (!n) {
        return -1;
    }
    lh = height_rec(n->left);
    rh = height_rec(n->right);
    return 1 + (lh > rh ? lh : rh);
}

long bt_height(const BinaryTree *t) { return height_rec(t ? t->root : NULL); }

static size_t size_rec(const BinaryTreeNode *n) {
    if (!n) {
        return 0;
    }
    return 1 + size_rec(n->left) + size_rec(n->right);
}

size_t bt_size(const BinaryTree *t) { return size_rec(t ? t->root : NULL); }

/* ---- shape predicates ------------------------------------------------- */

static int is_full_rec(const BinaryTreeNode *n) {
    if (!n) {
        return 1;
    }
    if (!n->left && !n->right) {
        return 1;
    }
    if (n->left && n->right) {
        return is_full_rec(n->left) && is_full_rec(n->right);
    }
    return 0; /* exactly one child */
}

int bt_is_full(const BinaryTree *t) { return is_full_rec(t ? t->root : NULL); }

/* Breadth-first: once a gap (NULL) is seen, no real node may follow. The queue
 * holds node pointers (NULL entries included), like the Rust VecDeque. */
int bt_is_complete(const BinaryTree *t) {
    const BinaryTreeNode *root = t ? t->root : NULL;
    size_t sz = size_rec(root);
    const BinaryTreeNode **queue;
    size_t head = 0, tail = 0;
    size_t cap;
    int seen_none = 0;
    int result = 1;
    if (!root) {
        return 1;
    }
    /* Each real node enqueues 2 children; total enqueues <= 1 + 2*size. */
    cap = sz > (SIZE_MAX - 2) / 2 ? SIZE_MAX : 2 * sz + 2;
    queue = malloc(cap * sizeof *queue);
    if (!queue) {
        return 1; /* degrade gracefully; cannot allocate to check */
    }
    queue[tail++] = root;
    while (head < tail) {
        const BinaryTreeNode *node = queue[head++];
        if (!node) {
            seen_none = 1;
        } else {
            if (seen_none) {
                result = 0;
                break;
            }
            if (tail < cap) {
                queue[tail++] = node->left;
            }
            if (tail < cap) {
                queue[tail++] = node->right;
            }
        }
    }
    free(queue);
    return result;
}

int bt_is_perfect(const BinaryTree *t) {
    const BinaryTreeNode *root = t ? t->root : NULL;
    long h = height_rec(root);
    size_t n = size_rec(root);
    if (h < 0) {
        return n == 0;
    }
    /* A perfect tree of height h has 2^(h+1)-1 nodes. If h+1 >= bit-width the
     * count is unrepresentable (and the tree could never be allocated), so it
     * cannot be perfect. */
    if ((size_t)(h + 1) >= sizeof(size_t) * 8) {
        return 0;
    }
    return n == (((size_t)1 << (h + 1)) - 1);
}

/* ---- depth-first traversals ------------------------------------------- */

static void inorder_rec(const BinaryTreeNode *n, int *buf, size_t buf_len,
                        size_t *idx) {
    if (!n) {
        return;
    }
    inorder_rec(n->left, buf, buf_len, idx);
    if (*idx < buf_len) {
        buf[(*idx)++] = n->value;
    }
    inorder_rec(n->right, buf, buf_len, idx);
}

size_t bt_inorder(const BinaryTree *t, int *buf, size_t buf_len) {
    size_t idx = 0;
    inorder_rec(t ? t->root : NULL, buf, buf_len, &idx);
    return idx;
}

static void preorder_rec(const BinaryTreeNode *n, int *buf, size_t buf_len,
                         size_t *idx) {
    if (!n) {
        return;
    }
    if (*idx < buf_len) {
        buf[(*idx)++] = n->value;
    }
    preorder_rec(n->left, buf, buf_len, idx);
    preorder_rec(n->right, buf, buf_len, idx);
}

size_t bt_preorder(const BinaryTree *t, int *buf, size_t buf_len) {
    size_t idx = 0;
    preorder_rec(t ? t->root : NULL, buf, buf_len, &idx);
    return idx;
}

static void postorder_rec(const BinaryTreeNode *n, int *buf, size_t buf_len,
                          size_t *idx) {
    if (!n) {
        return;
    }
    postorder_rec(n->left, buf, buf_len, idx);
    postorder_rec(n->right, buf, buf_len, idx);
    if (*idx < buf_len) {
        buf[(*idx)++] = n->value;
    }
}

size_t bt_postorder(const BinaryTree *t, int *buf, size_t buf_len) {
    size_t idx = 0;
    postorder_rec(t ? t->root : NULL, buf, buf_len, &idx);
    return idx;
}

/* ---- breadth-first traversal ------------------------------------------ */

size_t bt_level_order(const BinaryTree *t, int *buf, size_t buf_len) {
    const BinaryTreeNode *root = t ? t->root : NULL;
    size_t sz = size_rec(root);
    const BinaryTreeNode **queue;
    size_t head = 0, tail = 0, out = 0, cap;
    if (!root) {
        return 0;
    }
    cap = sz > (SIZE_MAX - 2) / 2 ? SIZE_MAX : 2 * sz + 2;
    queue = malloc(cap * sizeof *queue);
    if (!queue) {
        return 0;
    }
    queue[tail++] = root;
    while (head < tail) {
        const BinaryTreeNode *node = queue[head++];
        if (node) {
            if (out < buf_len) {
                buf[out] = node->value;
            }
            out++;
            if (tail < cap) {
                queue[tail++] = node->left;
            }
            if (tail < cap) {
                queue[tail++] = node->right;
            }
        }
    }
    free(queue);
    return out;
}

/* ---- level-order array with gaps -------------------------------------- */

static void fill_array(const BinaryTreeNode *n, size_t index, int *vals,
                       int *pres, size_t buf_len) {
    if (!n || index >= buf_len) {
        return;
    }
    vals[index] = n->value;
    pres[index] = 1;
    if (index <= (SIZE_MAX - 1) / 2) {
        fill_array(n->left, 2 * index + 1, vals, pres, buf_len);
    }
    if (index <= (SIZE_MAX - 2) / 2) {
        fill_array(n->right, 2 * index + 2, vals, pres, buf_len);
    }
}

size_t bt_to_array(const BinaryTree *t, int *values_out, int *present_out,
                   size_t buf_len) {
    const BinaryTreeNode *root = t ? t->root : NULL;
    long h = height_rec(root);
    size_t len;
    size_t clamp;
    size_t i;
    if (h < 0) {
        return 0;
    }
    if ((size_t)(h + 1) >= sizeof(size_t) * 8) {
        len = SIZE_MAX; /* unrepresentable in practice */
    } else {
        len = ((size_t)1 << (h + 1)) - 1;
    }
    clamp = len < buf_len ? len : buf_len;
    for (i = 0; i < clamp; i++) {
        present_out[i] = 0; /* gaps by default */
    }
    fill_array(root, 0, values_out, present_out, buf_len);
    return len;
}

/* ---- ASCII rendering -------------------------------------------------- */

typedef struct {
    char *data;
    size_t len;
    size_t cap;
    int ok;
} StrBuf;

static void sb_reserve(StrBuf *s, size_t extra) {
    size_t need;
    size_t ncap;
    char *nd;
    if (!s->ok) {
        return;
    }
    if (extra > SIZE_MAX - 1 - s->len) {
        s->ok = 0;
        return;
    }
    need = s->len + extra + 1; /* +1 for the NUL */
    if (need <= s->cap) {
        return;
    }
    ncap = s->cap ? s->cap : 16;
    while (ncap < need) {
        if (ncap > SIZE_MAX / 2) {
            ncap = need;
            break;
        }
        ncap *= 2;
    }
    nd = realloc(s->data, ncap);
    if (!nd) {
        s->ok = 0;
        return;
    }
    s->data = nd;
    s->cap = ncap;
}

static void sb_puts(StrBuf *s, const char *str) {
    size_t n = strlen(str);
    sb_reserve(s, n);
    if (!s->ok) {
        return;
    }
    memcpy(s->data + s->len, str, n);
    s->len += n;
    s->data[s->len] = '\0';
}

static void sb_putint(StrBuf *s, int v) {
    char tmp[32];
    snprintf(tmp, sizeof tmp, "%d", v);
    sb_puts(s, tmp);
}

/* Render `node` with the given prefix. `is_tail` selects the branch glyph. */
static void render_ascii(const BinaryTreeNode *node, const char *prefix,
                         int is_tail, StrBuf *sb) {
    const BinaryTreeNode *children[2];
    size_t nchildren = 0;
    size_t plen;
    char *next_prefix;
    size_t i;
    if (!sb->ok) {
        return;
    }
    sb_puts(sb, prefix);
    sb_puts(sb, is_tail ? "`-- " : "|-- ");
    sb_putint(sb, node->value);
    sb_puts(sb, "\n");

    if (node->left) {
        children[nchildren++] = node->left;
    }
    if (node->right) {
        children[nchildren++] = node->right;
    }
    if (nchildren == 0 || !sb->ok) {
        return;
    }

    /* next_prefix = prefix + (is_tail ? "    " : "|   "). */
    plen = strlen(prefix);
    if (plen > SIZE_MAX - 5) {
        sb->ok = 0;
        return;
    }
    next_prefix = malloc(plen + 5);
    if (!next_prefix) {
        sb->ok = 0;
        return;
    }
    memcpy(next_prefix, prefix, plen);
    memcpy(next_prefix + plen, is_tail ? "    " : "|   ", 4);
    next_prefix[plen + 4] = '\0';

    for (i = 0; i < nchildren; i++) {
        int last = (i + 1 == nchildren);
        render_ascii(children[i], next_prefix, last, sb);
    }
    free(next_prefix);
}

char *bt_to_ascii(const BinaryTree *t) {
    const BinaryTreeNode *root = t ? t->root : NULL;
    StrBuf sb;
    sb.data = NULL;
    sb.len = 0;
    sb.cap = 0;
    sb.ok = 1;
    if (root) {
        render_ascii(root, "", 1, &sb);
    }
    if (!sb.ok) {
        free(sb.data);
        return NULL;
    }
    if (!sb.data) {
        return calloc(1, 1); /* empty tree -> "" */
    }
    return sb.data;
}
