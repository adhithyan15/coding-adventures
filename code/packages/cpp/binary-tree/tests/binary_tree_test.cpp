// Tests for the C++ binary-tree, using the header-only iso_test.h harness (pure
// ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <optional>
#include <string>
#include <vector>

#include "binary_tree.hpp"

using ca::BinaryTree;
using OptVec = std::vector<std::optional<int>>;

int main() {
    // ── perfect tree [1..7] ──────────────────────────────────────────────
    {
        BinaryTree<int> t = BinaryTree<int>::from_level_order(
            {1, 2, 3, 4, 5, 6, 7});

        ISO_CHECK_EQ_UINT(t.size(), 7u);
        ISO_CHECK_EQ_INT(static_cast<int>(t.height()), 2);
        ISO_CHECK(t.is_full());
        ISO_CHECK(t.is_complete());
        ISO_CHECK(t.is_perfect());

        ISO_CHECK(t.inorder() == std::vector<int>({4, 2, 5, 1, 6, 3, 7}));
        ISO_CHECK(t.preorder() == std::vector<int>({1, 2, 4, 5, 3, 6, 7}));
        ISO_CHECK(t.postorder() == std::vector<int>({4, 5, 2, 6, 7, 3, 1}));
        ISO_CHECK(t.level_order() == std::vector<int>({1, 2, 3, 4, 5, 6, 7}));

        const BinaryTree<int>::Node* n = t.find(5);
        ISO_CHECK(n != nullptr && n->value == 5);
        ISO_CHECK(t.find(99) == nullptr);
        ISO_CHECK(t.left_child(1) != nullptr && t.left_child(1)->value == 2);
        ISO_CHECK(t.right_child(1) != nullptr && t.right_child(1)->value == 3);
        ISO_CHECK(t.left_child(2) != nullptr && t.left_child(2)->value == 4);
        ISO_CHECK(t.left_child(4) == nullptr);

        OptVec arr = t.to_array();
        ISO_CHECK(arr == OptVec({1, 2, 3, 4, 5, 6, 7}));

        std::string want =
            "`-- 1\n"
            "    |-- 2\n"
            "    |   |-- 4\n"
            "    |   `-- 5\n"
            "    `-- 3\n"
            "        |-- 6\n"
            "        `-- 7\n";
        ISO_CHECK(t.to_ascii() == want);
    }

    // ── tree with a gap: [1,2,3,_,5] ─────────────────────────────────────
    {
        BinaryTree<int> t = BinaryTree<int>::from_level_order(
            {1, 2, 3, std::nullopt, 5});
        ISO_CHECK_EQ_UINT(t.size(), 4u);
        ISO_CHECK(!t.is_full());
        ISO_CHECK(!t.is_complete());
        ISO_CHECK(!t.is_perfect());
        ISO_CHECK(t.inorder() == std::vector<int>({2, 5, 1, 3}));
        OptVec arr = t.to_array();
        OptVec want = {1, 2, 3, std::nullopt, 5, std::nullopt, std::nullopt};
        ISO_CHECK(arr == want);
    }

    // ── complete but not full/perfect: [1,2,3,4] ─────────────────────────
    {
        BinaryTree<int> t = BinaryTree<int>::from_level_order({1, 2, 3, 4});
        ISO_CHECK_EQ_UINT(t.size(), 4u);
        ISO_CHECK(!t.is_full());
        ISO_CHECK(t.is_complete());
        ISO_CHECK(!t.is_perfect());
    }

    // ── manual construction + deep-copy value semantics ──────────────────
    {
        auto root = BinaryTree<int>::make_node(10);
        root->left = BinaryTree<int>::make_node(20);
        root->right = BinaryTree<int>::make_node(30);
        BinaryTree<int> t = BinaryTree<int>::with_root(std::move(root));
        ISO_CHECK_EQ_UINT(t.size(), 3u);
        ISO_CHECK(t.is_full() && t.is_perfect());

        BinaryTree<int> copy = t;  // deep copy
        ISO_CHECK_EQ_UINT(copy.size(), 3u);
        ISO_CHECK(copy.find(20) != nullptr);
        ISO_CHECK(copy.root() != t.root());  // independent nodes
    }

    // ── empty tree ───────────────────────────────────────────────────────
    {
        BinaryTree<int> t;
        ISO_CHECK_EQ_UINT(t.size(), 0u);
        ISO_CHECK_EQ_INT(static_cast<int>(t.height()), -1);
        ISO_CHECK(t.is_full() && t.is_complete() && t.is_perfect());
        ISO_CHECK(t.root() == nullptr);
        ISO_CHECK(t.to_array().empty());
        ISO_CHECK(t.to_ascii().empty());
    }

    // ── generic over T: std::string ──────────────────────────────────────
    {
        BinaryTree<std::string> t = BinaryTree<std::string>::from_level_order(
            {std::optional<std::string>("root"), std::string("l"),
             std::string("r")});
        ISO_CHECK_EQ_UINT(t.size(), 3u);
        ISO_CHECK(t.level_order() ==
                  std::vector<std::string>({"root", "l", "r"}));
    }

    return ISO_TEST_RESULT();
}
