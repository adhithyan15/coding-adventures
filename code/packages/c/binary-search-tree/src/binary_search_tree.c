/*
 * binary_search_tree.c — implementation of the persistent BST (see
 * binary_search_tree.h). A faithful port of the Rust `binary-search-tree` crate:
 * the same insert / delete (extract-min) / balanced-build algorithms and the
 * same persistence (updates deep-copy then mutate the copy).
 */
#include "binary_search_tree.h"

#include <stdlib.h> /* malloc, free */

/* ---- node helpers ----------------------------------------------------- */

static size_t node_size(const BSTNode *n) { return n ? n->size : 0; }

static void update_size(BSTNode *n) {
    n->size = 1 + node_size(n->left) + node_size(n->right);
}

static BSTNode *node_new(int value) {
    BSTNode *n = malloc(sizeof *n);
    if (!n) {
        return NULL;
    }
    n->value = value;
    n->left = NULL;
    n->right = NULL;
    n->size = 1;
    return n;
}

static void node_free(BSTNode *n) {
    if (!n) {
        return;
    }
    node_free(n->left);
    node_free(n->right);
    free(n);
}

/* Deep-copy a subtree; NULL on allocation failure (partial copy is freed). */
static BSTNode *node_clone_deep(const BSTNode *n) {
    BSTNode *c = malloc(sizeof *c);
    if (!c) {
        return NULL;
    }
    c->value = n->value;
    c->size = n->size;
    c->left = NULL;
    c->right = NULL;
    if (n->left) {
        c->left = node_clone_deep(n->left);
        if (!c->left) {
            node_free(c);
            return NULL;
        }
    }
    if (n->right) {
        c->right = node_clone_deep(n->right);
        if (!c->right) {
            node_free(c);
            return NULL;
        }
    }
    return c;
}

/* Build a balanced subtree from values[lo, hi). NULL on empty range OR on
 * allocation failure — the caller distinguishes via *ok. */
static BSTNode *build_balanced(const int *values, size_t lo, size_t hi,
                               int *ok) {
    size_t mid;
    BSTNode *node;
    if (lo >= hi) {
        return NULL;
    }
    mid = lo + (hi - lo) / 2;
    node = node_new(values[mid]);
    if (!node) {
        *ok = 0;
        return NULL;
    }
    if (mid > lo) {
        node->left = build_balanced(values, lo, mid, ok);
        if (!*ok) {
            node_free(node);
            return NULL;
        }
    }
    if (mid + 1 < hi) {
        node->right = build_balanced(values, mid + 1, hi, ok);
        if (!*ok) {
            node_free(node);
            return NULL;
        }
    }
    update_size(node);
    return node;
}

/* ---- insert / delete -------------------------------------------------- */

/* Insert into subtree `root` (owned); returns the new root. On allocation
 * failure sets *ok = 0. NULL is only returned for an originally-empty subtree,
 * so a parent writing `child = insert_rec(child, ...)` never clobbers a real
 * subtree. */
static BSTNode *insert_rec(BSTNode *root, int value, int *ok) {
    if (!root) {
        BSTNode *n = node_new(value);
        if (!n) {
            *ok = 0;
        }
        return n;
    }
    if (value < root->value) {
        root->left = insert_rec(root->left, value, ok);
        if (!*ok) {
            return root;
        }
    } else if (value > root->value) {
        root->right = insert_rec(root->right, value, ok);
        if (!*ok) {
            return root;
        }
    } else {
        return root; /* duplicate — set semantics */
    }
    update_size(root);
    return root;
}

/* Remove and return the minimum of subtree `node` (owned), writing it to
 * *min_out and returning the remaining subtree. */
static BSTNode *extract_min(BSTNode *node, int *min_out) {
    if (!node->left) {
        BSTNode *right = node->right;
        *min_out = node->value;
        free(node);
        return right;
    }
    node->left = extract_min(node->left, min_out);
    update_size(node);
    return node;
}

/* Delete `value` from subtree `root` (owned); returns the new root. No
 * allocation, so this cannot fail. */
static BSTNode *delete_rec(BSTNode *root, int value) {
    if (!root) {
        return NULL;
    }
    if (value < root->value) {
        root->left = delete_rec(root->left, value);
        update_size(root);
        return root;
    }
    if (value > root->value) {
        root->right = delete_rec(root->right, value);
        update_size(root);
        return root;
    }
    /* Equal: remove this node. */
    {
        BSTNode *left = root->left;
        BSTNode *right = root->right;
        root->left = NULL;
        root->right = NULL;
        if (!left && !right) {
            free(root);
            return NULL;
        }
        if (left && !right) {
            free(root);
            return left;
        }
        if (!left && right) {
            free(root);
            return right;
        }
        {
            int successor;
            BSTNode *new_right = extract_min(right, &successor);
            root->value = successor;
            root->left = left;
            root->right = new_right;
            update_size(root);
            return root;
        }
    }
}

/* ---- public: construction / destruction ------------------------------- */

BST *bst_empty(void) {
    BST *t = malloc(sizeof *t);
    if (!t) {
        return NULL;
    }
    t->root = NULL;
    return t;
}

BST *bst_from_sorted_array(const int *values, size_t n) {
    BST *t = malloc(sizeof *t);
    int ok = 1;
    if (!t) {
        return NULL;
    }
    if (!values) {
        n = 0; /* treat a NULL array as empty rather than dereferencing it */
    }
    t->root = build_balanced(values, 0, n, &ok);
    if (!ok) {
        node_free(t->root);
        free(t);
        return NULL;
    }
    return t;
}

void bst_free(BST *t) {
    if (!t) {
        return;
    }
    node_free(t->root);
    free(t);
}

/* ---- public: persistent updates --------------------------------------- */

BST *bst_insert(const BST *t, int value) {
    BST *nt = malloc(sizeof *nt);
    int ok = 1;
    BSTNode *cloned = NULL;
    if (!nt) {
        return NULL;
    }
    if (t && t->root) {
        cloned = node_clone_deep(t->root);
        if (!cloned) {
            free(nt);
            return NULL;
        }
    }
    nt->root = insert_rec(cloned, value, &ok);
    if (!ok) {
        node_free(nt->root);
        free(nt);
        return NULL;
    }
    return nt;
}

BST *bst_delete(const BST *t, int value) {
    BST *nt = malloc(sizeof *nt);
    BSTNode *cloned = NULL;
    if (!nt) {
        return NULL;
    }
    if (t && t->root) {
        cloned = node_clone_deep(t->root);
        if (!cloned) {
            free(nt);
            return NULL;
        }
    }
    nt->root = delete_rec(cloned, value);
    return nt;
}

/* ---- public: queries -------------------------------------------------- */

const BSTNode *bst_search(const BST *t, int value) {
    const BSTNode *cur = t ? t->root : NULL;
    while (cur) {
        if (value < cur->value) {
            cur = cur->left;
        } else if (value > cur->value) {
            cur = cur->right;
        } else {
            return cur;
        }
    }
    return NULL;
}

int bst_contains(const BST *t, int value) {
    return bst_search(t, value) != NULL;
}

int bst_min_value(const BST *t, int *out) {
    const BSTNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->left) {
        cur = cur->left;
    }
    *out = cur->value;
    return 1;
}

int bst_max_value(const BST *t, int *out) {
    const BSTNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->right) {
        cur = cur->right;
    }
    *out = cur->value;
    return 1;
}

int bst_predecessor(const BST *t, int value, int *out) {
    const BSTNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (value <= cur->value) {
            cur = cur->left;
        } else {
            best = cur->value;
            found = 1;
            cur = cur->right;
        }
    }
    if (found) {
        *out = best;
    }
    return found;
}

int bst_successor(const BST *t, int value, int *out) {
    const BSTNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (value >= cur->value) {
            cur = cur->right;
        } else {
            best = cur->value;
            found = 1;
            cur = cur->left;
        }
    }
    if (found) {
        *out = best;
    }
    return found;
}

int bst_kth_smallest(const BST *t, size_t k, int *out) {
    const BSTNode *cur = t ? t->root : NULL;
    if (k == 0) {
        return 0;
    }
    while (cur) {
        size_t ls = node_size(cur->left);
        if (k == ls + 1) {
            *out = cur->value;
            return 1;
        }
        if (k <= ls) {
            cur = cur->left;
        } else {
            k = k - ls - 1;
            cur = cur->right;
        }
    }
    return 0;
}

size_t bst_rank(const BST *t, int value) {
    const BSTNode *cur = t ? t->root : NULL;
    size_t rank = 0;
    while (cur) {
        if (value < cur->value) {
            cur = cur->left;
        } else if (value == cur->value) {
            return rank + node_size(cur->left);
        } else {
            rank += node_size(cur->left) + 1;
            cur = cur->right;
        }
    }
    return rank;
}

static void inorder_fill(const BSTNode *n, int *buf, size_t buf_len,
                         size_t *idx) {
    if (!n || *idx >= buf_len) {
        return;
    }
    inorder_fill(n->left, buf, buf_len, idx);
    if (*idx < buf_len) {
        buf[*idx] = n->value;
        (*idx)++;
    }
    inorder_fill(n->right, buf, buf_len, idx);
}

size_t bst_to_sorted_array(const BST *t, int *buf, size_t buf_len) {
    size_t idx = 0;
    if (!buf || buf_len == 0) {
        return 0;
    }
    inorder_fill(t ? t->root : NULL, buf, buf_len, &idx);
    return idx;
}

size_t bst_size(const BST *t) { return node_size(t ? t->root : NULL); }

static long node_height(const BSTNode *n) {
    long lh, rh;
    if (!n) {
        return -1;
    }
    lh = node_height(n->left);
    rh = node_height(n->right);
    return 1 + (lh > rh ? lh : rh);
}

long bst_height(const BST *t) { return node_height(t ? t->root : NULL); }

static int validate(const BSTNode *n, const int *min, const int *max) {
    if (!n) {
        return 1;
    }
    if (min && n->value <= *min) {
        return 0;
    }
    if (max && n->value >= *max) {
        return 0;
    }
    return validate(n->left, min, &n->value) &&
           validate(n->right, &n->value, max);
}

int bst_is_valid(const BST *t) {
    return validate(t ? t->root : NULL, NULL, NULL);
}
