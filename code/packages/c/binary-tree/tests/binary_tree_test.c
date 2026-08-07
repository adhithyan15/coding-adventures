/* Tests for the C binary-tree, using the header-only iso_test.h harness (pure
 * ISO). Vectors mirror the Rust crate's own unit tests. */
#include "iso_test.h"

#include <stdlib.h> /* free */
#include <string.h> /* memcmp, strcmp */

#include "binary_tree.h"

/* Build a tree from a level-order layout where every slot is present. */
static BinaryTree *full_levels(const int *vals, size_t n) {
    int present[16];
    size_t i;
    for (i = 0; i < n; i++) {
        present[i] = 1;
    }
    return bt_from_level_order(vals, present, n);
}

int main(void) {
    /* ── perfect tree [1..7] ────────────────────────────────────────────── */
    {
        int vals[] = {1, 2, 3, 4, 5, 6, 7};
        BinaryTree *t = full_levels(vals, 7);
        int buf[7];
        int want_in[] = {4, 2, 5, 1, 6, 3, 7};
        int want_pre[] = {1, 2, 4, 5, 3, 6, 7};
        int want_post[] = {4, 5, 2, 6, 7, 3, 1};
        int want_level[] = {1, 2, 3, 4, 5, 6, 7};
        const BinaryTreeNode *node;

        ISO_CHECK_EQ_UINT(bt_size(t), 7u);
        ISO_CHECK_EQ_INT((int)bt_height(t), 2);
        ISO_CHECK(bt_is_full(t));
        ISO_CHECK(bt_is_complete(t));
        ISO_CHECK(bt_is_perfect(t));

        ISO_CHECK_EQ_UINT(bt_inorder(t, buf, 7), 7u);
        ISO_CHECK_MEM_EQ(buf, want_in, sizeof want_in);
        bt_preorder(t, buf, 7);
        ISO_CHECK_MEM_EQ(buf, want_pre, sizeof want_pre);
        bt_postorder(t, buf, 7);
        ISO_CHECK_MEM_EQ(buf, want_post, sizeof want_post);
        bt_level_order(t, buf, 7);
        ISO_CHECK_MEM_EQ(buf, want_level, sizeof want_level);

        /* find / children. */
        node = bt_find(t, 5);
        ISO_CHECK(node != NULL && node->value == 5);
        ISO_CHECK(bt_find(t, 99) == NULL);
        node = bt_left_child(t, 1);
        ISO_CHECK(node != NULL && node->value == 2);
        node = bt_right_child(t, 1);
        ISO_CHECK(node != NULL && node->value == 3);
        node = bt_left_child(t, 2);
        ISO_CHECK(node != NULL && node->value == 4);
        ISO_CHECK(bt_left_child(t, 4) == NULL); /* leaf has no child */

        /* to_array round trip. */
        {
            int av[7];
            int ap[7];
            size_t len = bt_to_array(t, av, ap, 7);
            size_t i;
            ISO_CHECK_EQ_UINT(len, 7u);
            for (i = 0; i < 7; i++) {
                ISO_CHECK(ap[i] == 1 && av[i] == (int)(i + 1));
            }
        }

        /* to_ascii diagram. */
        {
            char *s = bt_to_ascii(t);
            const char *want =
                "`-- 1\n"
                "    |-- 2\n"
                "    |   |-- 4\n"
                "    |   `-- 5\n"
                "    `-- 3\n"
                "        |-- 6\n"
                "        `-- 7\n";
            ISO_CHECK(s != NULL && strcmp(s, want) == 0);
            free(s);
        }
        bt_free(t);
    }

    /* ── tree with a gap: [1,2,3,_,5] — not full, not complete ──────────── */
    {
        int vals[] = {1, 2, 3, 0, 5};
        int present[] = {1, 1, 1, 0, 1};
        BinaryTree *t = bt_from_level_order(vals, present, 5);
        int buf[8];
        int want_in[] = {2, 5, 1, 3};

        ISO_CHECK_EQ_UINT(bt_size(t), 4u);
        ISO_CHECK(!bt_is_full(t));     /* node 2 has only a right child */
        ISO_CHECK(!bt_is_complete(t)); /* a gap precedes node 5 */
        ISO_CHECK(!bt_is_perfect(t));
        ISO_CHECK_EQ_UINT(bt_inorder(t, buf, 8), 4u);
        ISO_CHECK_MEM_EQ(buf, want_in, sizeof want_in);

        /* to_array keeps the gap. */
        {
            int av[7];
            int ap[7];
            int want_pres[] = {1, 1, 1, 0, 1, 0, 0};
            size_t len = bt_to_array(t, av, ap, 7);
            size_t i;
            ISO_CHECK_EQ_UINT(len, 7u);
            for (i = 0; i < 7; i++) {
                ISO_CHECK(ap[i] == want_pres[i]);
            }
            ISO_CHECK(av[0] == 1 && av[4] == 5);
        }
        bt_free(t);
    }

    /* ── complete but not full/perfect: [1,2,3,4] ───────────────────────── */
    {
        int vals[] = {1, 2, 3, 4};
        BinaryTree *t = full_levels(vals, 4);
        ISO_CHECK_EQ_UINT(bt_size(t), 4u);
        ISO_CHECK(!bt_is_full(t)); /* node 2 has only a left child */
        ISO_CHECK(bt_is_complete(t));
        ISO_CHECK(!bt_is_perfect(t));
        bt_free(t);
    }

    /* ── manual construction via bt_node_new + bt_with_root ─────────────── */
    {
        BinaryTreeNode *root = bt_node_new(10);
        BinaryTree *t;
        root->left = bt_node_new(20);
        root->right = bt_node_new(30);
        t = bt_with_root(root);
        ISO_CHECK_EQ_UINT(bt_size(t), 3u);
        ISO_CHECK(bt_is_full(t));
        ISO_CHECK(bt_is_perfect(t));
        {
            const BinaryTreeNode *n = bt_find(t, 20);
            ISO_CHECK(n != NULL && n->value == 20);
        }
        bt_free(t);
    }

    /* ── empty tree ─────────────────────────────────────────────────────── */
    {
        BinaryTree *t = bt_new();
        char *s;
        ISO_CHECK_EQ_UINT(bt_size(t), 0u);
        ISO_CHECK_EQ_INT((int)bt_height(t), -1);
        ISO_CHECK(bt_is_full(t));
        ISO_CHECK(bt_is_complete(t));
        ISO_CHECK(bt_is_perfect(t));
        ISO_CHECK(bt_root(t) == NULL);
        ISO_CHECK_EQ_UINT(bt_to_array(t, NULL, NULL, 0), 0u);
        s = bt_to_ascii(t);
        ISO_CHECK(s != NULL && strcmp(s, "") == 0);
        free(s);
        bt_free(t);
    }

    return ISO_TEST_RESULT();
}
