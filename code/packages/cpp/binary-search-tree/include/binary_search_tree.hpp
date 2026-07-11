// binary_search_tree.hpp — an unbalanced binary search tree with order
// statistics, in pure ISO C++17, header-only, in namespace ca::bst. A faithful
// port of the Rust `binary-search-tree` crate (DT07).
// ===========================================================================
//
// A binary search tree keeps values ordered so that, for every node, everything
// in its left subtree is smaller and everything in its right subtree is larger.
// Search / insert / delete are O(h) where h is the height — O(log n) for a
// balanced tree, O(n) worst case for a degenerate one. Unlike its self-balancing
// cousin (see the `avl-tree` crate), this tree never rotates: insertion order
// alone determines the shape.
//
// Each node caches its subtree `size` (node count); that cache is what makes
// `rank` and `kth_smallest` (order statistics) O(h). Height is NOT cached — it
// is computed by a full traversal, matching the Rust crate.
//
// `from_sorted_array` builds a height-balanced tree from a sorted sequence by
// recursively choosing the middle element as each subtree root.
//
// PERSISTENCE. The Rust crate is *persistent*: `insert`/`delete` take `&self`
// and return a brand-new tree, leaving the original untouched. This port keeps
// that exact behaviour through value semantics: `BST<T>` deep-copies on copy,
// and `insert`/`erase` are `const` — they copy `*this` and mutate the copy, so
// any tree you already hold is unchanged. (Rust's `Box` gives deep-clone
// persistence, not `Rc` structural sharing, which is what value semantics
// mirror.)
//
// The element type `T` must be less-than comparable (equality is derived as
// `!(a < b) && !(b < a)`) and copyable — matching Rust's `T: Ord + Clone`.
//
// CAVEAT. This tree does not self-balance, so its height is bounded only by
// insertion order — inserting already-sorted keys builds a degenerate (linked-
// list) tree of height n. Operations (and the `unique_ptr` teardown) recurse to
// the tree's height, so a very tall tree can overflow the stack. For adversarial
// or large sorted input, prefer `from_sorted_array` (log-depth) or the
// self-balancing `avl-tree` package. This mirrors the Rust crate's behaviour.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_BINARY_SEARCH_TREE_HPP
#define CA_BINARY_SEARCH_TREE_HPP

#include <cstddef>
#include <memory>
#include <optional>
#include <vector>

namespace ca {
namespace bst {

template <class T>
class BST {
public:
    // A node. `size` is the number of nodes in this subtree (>= 1).
    struct Node {
        T value;
        std::unique_ptr<Node> left;
        std::unique_ptr<Node> right;
        std::size_t size = 1;
        explicit Node(const T& v) : value(v) {}
    };

    BST() = default;
    BST(const BST& other) : root_(clone(other.root_)) {}
    BST& operator=(const BST& other) {
        if (this != &other) {
            root_ = clone(other.root_);
        }
        return *this;
    }
    BST(BST&&) noexcept = default;
    BST& operator=(BST&&) noexcept = default;
    ~BST() = default;

    // An empty tree.
    static BST empty() { return BST(); }

    // A balanced tree over the (assumed sorted, ascending) `values`.
    static BST from_sorted_array(const std::vector<T>& values) {
        BST t;
        t.root_ = build_balanced(values, 0, values.size());
        return t;
    }

    // Persistent insert: a new tree with `value` added (duplicates are a no-op
    // on membership). `*this` is left unchanged.
    BST insert(const T& value) const {
        BST copy(*this);
        copy.root_ = ins(std::move(copy.root_), value);
        return copy;
    }

    // Persistent erase: a new tree with `value` removed if present. `*this` is
    // left unchanged.
    BST erase(const T& value) const {
        BST copy(*this);
        copy.root_ = del(std::move(copy.root_), value);
        return copy;
    }

    // Pointer to the node holding `value`, or nullptr. Valid until this tree is
    // destroyed or reassigned.
    const Node* find(const T& value) const {
        const Node* cur = root_.get();
        while (cur) {
            if (value < cur->value) {
                cur = cur->left.get();
            } else if (cur->value < value) {
                cur = cur->right.get();
            } else {
                return cur;
            }
        }
        return nullptr;
    }

    bool contains(const T& value) const { return find(value) != nullptr; }

    std::optional<T> min_value() const {
        const Node* cur = root_.get();
        if (!cur) {
            return std::nullopt;
        }
        while (cur->left) {
            cur = cur->left.get();
        }
        return cur->value;
    }

    std::optional<T> max_value() const {
        const Node* cur = root_.get();
        if (!cur) {
            return std::nullopt;
        }
        while (cur->right) {
            cur = cur->right.get();
        }
        return cur->value;
    }

    // Largest stored value strictly less than `value`.
    std::optional<T> predecessor(const T& value) const {
        const Node* cur = root_.get();
        std::optional<T> best;
        while (cur) {
            if (!(cur->value < value)) {  // value <= cur->value
                cur = cur->left.get();
            } else {  // cur->value < value — a candidate
                best = cur->value;
                cur = cur->right.get();
            }
        }
        return best;
    }

    // Smallest stored value strictly greater than `value`.
    std::optional<T> successor(const T& value) const {
        const Node* cur = root_.get();
        std::optional<T> best;
        while (cur) {
            if (!(value < cur->value)) {  // value >= cur->value
                cur = cur->right.get();
            } else {  // cur->value > value — a candidate
                best = cur->value;
                cur = cur->left.get();
            }
        }
        return best;
    }

    // The k-th smallest value (1-based); nullopt if k == 0 or k > size.
    std::optional<T> kth_smallest(std::size_t k) const {
        if (k == 0) {
            return std::nullopt;
        }
        const Node* cur = root_.get();
        while (cur) {
            std::size_t ls = sub_size(cur->left);
            if (k == ls + 1) {
                return cur->value;
            }
            if (k <= ls) {
                cur = cur->left.get();
            } else {
                k = k - ls - 1;
                cur = cur->right.get();
            }
        }
        return std::nullopt;
    }

    // Number of stored values strictly less than `value`.
    std::size_t rank(const T& value) const {
        const Node* cur = root_.get();
        std::size_t r = 0;
        while (cur) {
            if (value < cur->value) {
                cur = cur->left.get();
            } else if (!(cur->value < value)) {  // equal
                return r + sub_size(cur->left);
            } else {
                r += sub_size(cur->left) + 1;
                cur = cur->right.get();
            }
        }
        return r;
    }

    // Values in ascending order.
    std::vector<T> to_sorted_array() const {
        std::vector<T> out;
        inorder(root_.get(), out);
        return out;
    }

    std::size_t size() const { return sub_size(root_); }

    // Height: -1 when empty, 0 for a single node. Computed by traversal.
    long height() const { return sub_height(root_.get()); }

    const Node* root() const { return root_.get(); }

    bool is_valid() const { return valid_bst(root_.get(), nullptr, nullptr); }

private:
    using NodePtr = std::unique_ptr<Node>;

    static std::size_t sub_size(const NodePtr& n) { return n ? n->size : 0; }

    static long sub_height(const Node* n) {
        if (!n) {
            return -1;
        }
        long lh = sub_height(n->left.get());
        long rh = sub_height(n->right.get());
        return 1 + (lh > rh ? lh : rh);
    }

    static void update_size(const NodePtr& n) {
        n->size = 1 + sub_size(n->left) + sub_size(n->right);
    }

    static NodePtr clone(const NodePtr& n) {
        if (!n) {
            return nullptr;
        }
        auto c = std::make_unique<Node>(n->value);
        c->size = n->size;
        c->left = clone(n->left);
        c->right = clone(n->right);
        return c;
    }

    // Build a balanced subtree from values[lo, hi) by taking the middle element.
    static NodePtr build_balanced(const std::vector<T>& values, std::size_t lo,
                                  std::size_t hi) {
        if (lo >= hi) {
            return nullptr;
        }
        std::size_t mid = lo + (hi - lo) / 2;
        auto node = std::make_unique<Node>(values[mid]);
        node->left = build_balanced(values, lo, mid);
        node->right = build_balanced(values, mid + 1, hi);
        update_size(node);
        return node;
    }

    static NodePtr ins(NodePtr root, const T& value) {
        if (!root) {
            return std::make_unique<Node>(value);
        }
        if (value < root->value) {
            root->left = ins(std::move(root->left), value);
        } else if (root->value < value) {
            root->right = ins(std::move(root->right), value);
        } else {
            return root;  // duplicate — set semantics
        }
        update_size(root);
        return root;
    }

    // Remove the minimum of `node`, writing it to `min_out`.
    static NodePtr extract_min(NodePtr node, T& min_out) {
        if (!node->left) {
            min_out = node->value;
            return std::move(node->right);  // node destroyed on return
        }
        node->left = extract_min(std::move(node->left), min_out);
        update_size(node);
        return node;
    }

    static NodePtr del(NodePtr root, const T& value) {
        if (!root) {
            return nullptr;
        }
        if (value < root->value) {
            root->left = del(std::move(root->left), value);
            update_size(root);
            return root;
        }
        if (root->value < value) {
            root->right = del(std::move(root->right), value);
            update_size(root);
            return root;
        }
        // Equal — remove this node.
        if (!root->left && !root->right) {
            return nullptr;
        }
        if (!root->right) {
            return std::move(root->left);
        }
        if (!root->left) {
            return std::move(root->right);
        }
        // Two children: replace with the in-order successor.
        T successor = root->value;  // placeholder, overwritten below
        root->right = extract_min(std::move(root->right), successor);
        root->value = successor;
        update_size(root);
        return root;
    }

    static void inorder(const Node* n, std::vector<T>& out) {
        if (!n) {
            return;
        }
        inorder(n->left.get(), out);
        out.push_back(n->value);
        inorder(n->right.get(), out);
    }

    static bool valid_bst(const Node* n, const T* min, const T* max) {
        if (!n) {
            return true;
        }
        if (min && !(*min < n->value)) {  // n->value <= *min
            return false;
        }
        if (max && !(n->value < *max)) {  // n->value >= *max
            return false;
        }
        return valid_bst(n->left.get(), min, &n->value) &&
               valid_bst(n->right.get(), &n->value, max);
    }

    NodePtr root_;
};

}  // namespace bst
}  // namespace ca

#endif  // CA_BINARY_SEARCH_TREE_HPP
