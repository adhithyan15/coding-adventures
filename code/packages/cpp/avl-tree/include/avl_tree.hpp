// avl_tree.hpp — a self-balancing AVL tree with order statistics, in pure ISO
// C++17, header-only, in namespace ca::avl. A faithful port of the Rust
// `avl-tree` crate (DT08).
// ===========================================================================
//
// An AVL tree is a binary search tree that keeps itself balanced: after every
// insert or delete it restores the invariant that, for every node, the heights
// of its two subtrees differ by at most one. That bound guarantees O(log n)
// search / insert / delete regardless of key arrival order.
//
//   balance factor = height(left) - height(right)      must be in {-1, 0, +1}
//
// When an insert/delete pushes a node's balance factor to +2 or -2, one or two
// rotations bring it back (an "LR"/"RL" case first rotates the child to reduce
// to the simple "LL"/"RR" case).
//
// PERSISTENCE. The Rust crate is *persistent*: `insert`/`delete` take `&self`
// and return a brand-new tree, leaving the original untouched. This port keeps
// that exact behaviour through value semantics: `AVLTree<T>` deep-copies on
// copy, and `insert`/`erase` are `const` — they copy `*this` and mutate the
// copy, so any tree you already hold is unchanged. (The Rust `Box` gives
// deep-clone persistence, not `Rc` structural sharing, which is what value
// semantics mirror.)
//
// Each node caches its subtree height and size; the size cache is what makes
// `rank` and `kth_smallest` (order statistics) O(log n).
//
// The element type `T` must be less-than comparable (equality is derived as
// `!(a < b) && !(b < a)`) and copyable — matching Rust's `T: Ord + Clone`.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_AVL_TREE_HPP
#define CA_AVL_TREE_HPP

#include <cstddef>
#include <memory>
#include <optional>
#include <vector>

namespace ca {
namespace avl {

template <class T>
class AVLTree {
public:
    // A node. `height` is 0 for a leaf; an absent child counts as height -1.
    struct Node {
        T value;
        std::unique_ptr<Node> left;
        std::unique_ptr<Node> right;
        long height = 0;
        std::size_t size = 1;
        explicit Node(const T& v) : value(v) {}
    };

    AVLTree() = default;
    AVLTree(const AVLTree& other) : root_(clone(other.root_)) {}
    AVLTree& operator=(const AVLTree& other) {
        if (this != &other) {
            root_ = clone(other.root_);
        }
        return *this;
    }
    AVLTree(AVLTree&&) noexcept = default;
    AVLTree& operator=(AVLTree&&) noexcept = default;
    ~AVLTree() = default;

    // An empty tree.
    static AVLTree empty() { return AVLTree(); }

    // Persistent insert: a new tree with `value` added (duplicates are a no-op
    // on membership). `*this` is left unchanged.
    AVLTree insert(const T& value) const {
        AVLTree copy(*this);
        copy.root_ = ins(std::move(copy.root_), value);
        return copy;
    }

    // Persistent erase: a new tree with `value` removed if present. `*this` is
    // left unchanged.
    AVLTree erase(const T& value) const {
        AVLTree copy(*this);
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
    long height() const { return sub_height(root_); }
    const Node* root() const { return root_.get(); }

    // height(left) - height(right) for a node.
    static long balance_factor(const Node& node) {
        return sub_height(node.left) - sub_height(node.right);
    }

    bool is_valid_bst() const {
        return valid_bst(root_.get(), nullptr, nullptr);
    }

    bool is_valid_avl() const {
        long h;
        std::size_t s;
        return valid_avl(root_.get(), nullptr, nullptr, h, s);
    }

private:
    using NodePtr = std::unique_ptr<Node>;

    static long sub_height(const NodePtr& n) { return n ? n->height : -1; }
    static std::size_t sub_size(const NodePtr& n) { return n ? n->size : 0; }
    static long bf(const NodePtr& n) {
        return sub_height(n->left) - sub_height(n->right);
    }

    static void update_meta(const NodePtr& n) {
        long lh = sub_height(n->left);
        long rh = sub_height(n->right);
        n->height = 1 + (lh > rh ? lh : rh);
        n->size = 1 + sub_size(n->left) + sub_size(n->right);
    }

    static NodePtr clone(const NodePtr& n) {
        if (!n) {
            return nullptr;
        }
        auto c = std::make_unique<Node>(n->value);
        c->height = n->height;
        c->size = n->size;
        c->left = clone(n->left);
        c->right = clone(n->right);
        return c;
    }

    static NodePtr rotate_left(NodePtr root) {
        if (!root->right) {
            return root;
        }
        NodePtr new_root = std::move(root->right);
        root->right = std::move(new_root->left);
        update_meta(root);
        new_root->left = std::move(root);
        update_meta(new_root);
        return new_root;
    }

    static NodePtr rotate_right(NodePtr root) {
        if (!root->left) {
            return root;
        }
        NodePtr new_root = std::move(root->left);
        root->left = std::move(new_root->right);
        update_meta(root);
        new_root->right = std::move(root);
        update_meta(new_root);
        return new_root;
    }

    static NodePtr rebalance(NodePtr node) {
        long b = bf(node);
        if (b > 1) {  // left-heavy
            if (node->left && bf(node->left) < 0) {
                node->left = rotate_left(std::move(node->left));  // LR -> LL
            }
            return rotate_right(std::move(node));
        }
        if (b < -1) {  // right-heavy
            if (node->right && bf(node->right) > 0) {
                node->right = rotate_right(std::move(node->right));  // RL -> RR
            }
            return rotate_left(std::move(node));
        }
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
            return root;  // duplicate
        }
        update_meta(root);
        return rebalance(std::move(root));
    }

    // Remove the minimum of `node`, writing it to `min_out`.
    static NodePtr extract_min(NodePtr node, T& min_out) {
        if (!node->left) {
            min_out = node->value;
            return std::move(node->right);  // node destroyed on return
        }
        node->left = extract_min(std::move(node->left), min_out);
        update_meta(node);
        return rebalance(std::move(node));
    }

    static NodePtr del(NodePtr root, const T& value) {
        if (!root) {
            return nullptr;
        }
        if (value < root->value) {
            root->left = del(std::move(root->left), value);
        } else if (root->value < value) {
            root->right = del(std::move(root->right), value);
        } else {
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
        }
        update_meta(root);
        return rebalance(std::move(root));
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

    static bool valid_avl(const Node* n, const T* min, const T* max, long& h,
                          std::size_t& s) {
        if (!n) {
            h = -1;
            s = 0;
            return true;
        }
        if (min && !(*min < n->value)) {
            return false;
        }
        if (max && !(n->value < *max)) {
            return false;
        }
        long lh = 0, rh = 0;
        std::size_t ls = 0, rs = 0;
        if (!valid_avl(n->left.get(), min, &n->value, lh, ls)) {
            return false;
        }
        if (!valid_avl(n->right.get(), &n->value, max, rh, rs)) {
            return false;
        }
        long height_val = 1 + (lh > rh ? lh : rh);
        std::size_t size_val = 1 + ls + rs;
        if (n->height != height_val || n->size != size_val) {
            return false;
        }
        long diff = lh - rh;
        if (diff < 0) {
            diff = -diff;
        }
        if (diff > 1) {
            return false;
        }
        h = height_val;
        s = size_val;
        return true;
    }

    NodePtr root_;
};

}  // namespace avl
}  // namespace ca

#endif  // CA_AVL_TREE_HPP
