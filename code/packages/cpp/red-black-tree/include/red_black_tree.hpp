// red_black_tree.hpp — a left-leaning red-black (LLRB) tree with order
// statistics, in pure ISO C++17, header-only, in namespace ca::rb. A faithful
// port of the Rust `red-black-tree` crate (DT09).
// ===========================================================================
//
// A red-black tree is a binary search tree that colours its nodes red or black
// and maintains, through rotations and colour flips, two invariants that force
// the height to stay O(log n):
//
//   1. No red node has a red child.
//   2. Every root-to-leaf path passes through the same number of black nodes.
//
// This is the *left-leaning* variant (Sedgewick): red links always lean left,
// so a single `fix_up` on the way back up handles every insert and delete. It
// is exactly equivalent to a 2-3 tree. Each node caches its subtree size, so
// `kth_smallest` (order statistic) is O(log n).
//
// PERSISTENCE. The Rust crate is *persistent*: `insert`/`delete` take `&self`
// and return a brand-new tree, leaving the original untouched. This port keeps
// that behaviour through value semantics: `RBTree<T>` deep-copies on copy, and
// `insert`/`erase` are `const` — they copy `*this` and mutate the copy. (The
// Rust `Box` gives deep-clone persistence, not `Rc` structural sharing, which is
// what value semantics mirror.)
//
// The element type `T` must be less-than comparable (equality is derived as
// `!(a < b) && !(b < a)`) and copyable — matching Rust's `T: Ord + Clone`.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_RED_BLACK_TREE_HPP
#define CA_RED_BLACK_TREE_HPP

#include <cstddef>
#include <memory>
#include <optional>
#include <vector>

namespace ca {
namespace rb {

template <class T>
class RBTree {
public:
    enum class Color { Red, Black };

    struct Node {
        T value;
        Color color;
        std::unique_ptr<Node> left;
        std::unique_ptr<Node> right;
        std::size_t size = 1;
        Node(const T& v, Color c) : value(v), color(c) {}
    };

    RBTree() = default;
    RBTree(const RBTree& other) : root_(clone(other.root_)) {}
    RBTree& operator=(const RBTree& other) {
        if (this != &other) {
            root_ = clone(other.root_);
        }
        return *this;
    }
    RBTree(RBTree&&) noexcept = default;
    RBTree& operator=(RBTree&&) noexcept = default;
    ~RBTree() = default;

    static RBTree empty() { return RBTree(); }

    // Persistent insert: a new tree with `value` added (duplicates are a no-op
    // on membership). `*this` is left unchanged.
    RBTree insert(const T& value) const {
        RBTree copy(*this);
        copy.root_ = insert_rec(std::move(copy.root_), value);
        if (copy.root_) {
            copy.root_->color = Color::Black;  // root is always black
        }
        return copy;
    }

    // Persistent erase: a new tree with `value` removed if present (`delete` is
    // a keyword). `*this` is left unchanged.
    RBTree erase(const T& value) const {
        RBTree copy(*this);
        copy.root_ = delete_rec(std::move(copy.root_), value);
        if (copy.root_) {
            copy.root_->color = Color::Black;
        }
        return copy;
    }

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

    std::optional<T> predecessor(const T& value) const {
        const Node* cur = root_.get();
        std::optional<T> best;
        while (cur) {
            if (!(cur->value < value)) {  // value <= cur->value
                cur = cur->left.get();
            } else {
                best = cur->value;
                cur = cur->right.get();
            }
        }
        return best;
    }

    std::optional<T> successor(const T& value) const {
        const Node* cur = root_.get();
        std::optional<T> best;
        while (cur) {
            if (!(value < cur->value)) {  // value >= cur->value
                cur = cur->right.get();
            } else {
                best = cur->value;
                cur = cur->left.get();
            }
        }
        return best;
    }

    std::optional<T> kth_smallest(std::size_t k) const {
        if (k == 0) {
            return std::nullopt;
        }
        const Node* cur = root_.get();
        while (cur) {
            std::size_t ls = cur->left ? cur->left->size : 0;
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

    std::vector<T> to_sorted_array() const {
        std::vector<T> out;
        inorder(root_.get(), out);
        return out;
    }

    std::size_t size() const { return node_size(root_); }
    const Node* root() const { return root_.get(); }

    // Number of black nodes down the left spine (0 for an empty tree).
    std::size_t black_height() const {
        const Node* cur = root_.get();
        std::size_t h = 0;
        while (cur) {
            if (cur->color == Color::Black) {
                ++h;
            }
            cur = cur->left.get();
        }
        return h;
    }

    bool is_valid_rb() const {
        if (root_ && root_->color != Color::Black) {
            return false;
        }
        std::size_t bh = 0;
        return valid(root_.get(), nullptr, nullptr, bh);
    }

private:
    using NodePtr = std::unique_ptr<Node>;

    static bool eq(const T& a, const T& b) { return !(a < b) && !(b < a); }

    static std::size_t node_size(const NodePtr& n) { return n ? n->size : 0; }
    static void update_size(const NodePtr& n) {
        n->size = 1 + node_size(n->left) + node_size(n->right);
    }
    static bool is_red(const NodePtr& n) {
        return n && n->color == Color::Red;
    }
    static bool is_red_left(const NodePtr& n) {
        return n && n->left && n->left->color == Color::Red;
    }
    static Color flip(Color c) {
        return c == Color::Red ? Color::Black : Color::Red;
    }
    static void flip_colors(const NodePtr& n) {
        n->color = flip(n->color);
        if (n->left) {
            n->left->color = flip(n->left->color);
        }
        if (n->right) {
            n->right->color = flip(n->right->color);
        }
    }

    static NodePtr clone(const NodePtr& n) {
        if (!n) {
            return nullptr;
        }
        auto c = std::make_unique<Node>(n->value, n->color);
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
        Color root_color = root->color;
        root->right = std::move(new_root->left);
        root->color = Color::Red;
        update_size(root);
        new_root->left = std::move(root);
        new_root->color = root_color;
        update_size(new_root);
        return new_root;
    }

    static NodePtr rotate_right(NodePtr root) {
        if (!root->left) {
            return root;
        }
        NodePtr new_root = std::move(root->left);
        Color root_color = root->color;
        root->left = std::move(new_root->right);
        root->color = Color::Red;
        update_size(root);
        new_root->right = std::move(root);
        new_root->color = root_color;
        update_size(new_root);
        return new_root;
    }

    static NodePtr fix_up(NodePtr node) {
        if (is_red(node->right) && !is_red(node->left)) {
            node = rotate_left(std::move(node));
        }
        if (is_red(node->left) && is_red_left(node->left)) {
            node = rotate_right(std::move(node));
        }
        if (is_red(node->left) && is_red(node->right)) {
            flip_colors(node);
        }
        update_size(node);
        return node;
    }

    static NodePtr insert_rec(NodePtr root, const T& value) {
        if (!root) {
            return std::make_unique<Node>(value, Color::Red);
        }
        if (value < root->value) {
            root->left = insert_rec(std::move(root->left), value);
        } else if (root->value < value) {
            root->right = insert_rec(std::move(root->right), value);
        } else {
            return root;  // duplicate
        }
        return fix_up(std::move(root));
    }

    static NodePtr move_red_left(NodePtr node) {
        flip_colors(node);
        if (is_red_left(node->right)) {
            if (node->right) {
                node->right = rotate_right(std::move(node->right));
            }
            node = rotate_left(std::move(node));
            flip_colors(node);
        }
        return node;
    }

    static NodePtr move_red_right(NodePtr node) {
        flip_colors(node);
        if (is_red_left(node->left)) {
            node = rotate_right(std::move(node));
            flip_colors(node);
        }
        return node;
    }

    static NodePtr delete_min(NodePtr node, T& min_out) {
        if (!node->left) {
            min_out = node->value;
            return std::move(node->right);  // node destroyed on return
        }
        if (!is_red(node->left) && !is_red_left(node->left)) {
            node = move_red_left(std::move(node));
        }
        node->left = delete_min(std::move(node->left), min_out);
        return fix_up(std::move(node));
    }

    static NodePtr delete_rec(NodePtr node, const T& value) {
        if (!node) {
            return nullptr;
        }
        if (value < node->value) {
            if (!is_red(node->left) && !is_red_left(node->left)) {
                node = move_red_left(std::move(node));
            }
            node->left = delete_rec(std::move(node->left), value);
        } else {
            if (is_red(node->left)) {
                node = rotate_right(std::move(node));
            }
            if (eq(value, node->value) && !node->right) {
                return nullptr;  // leaf being removed
            }
            if (!is_red(node->right) && !is_red_left(node->right)) {
                node = move_red_right(std::move(node));
            }
            if (eq(value, node->value)) {
                T successor = node->value;  // placeholder, overwritten below
                node->right = delete_min(std::move(node->right), successor);
                node->value = successor;
            } else {
                node->right = delete_rec(std::move(node->right), value);
            }
        }
        return fix_up(std::move(node));
    }

    static void inorder(const Node* n, std::vector<T>& out) {
        if (!n) {
            return;
        }
        inorder(n->left.get(), out);
        out.push_back(n->value);
        inorder(n->right.get(), out);
    }

    static bool valid(const Node* n, const T* min, const T* max,
                      std::size_t& bh) {
        if (!n) {
            bh = 1;
            return true;
        }
        if (min && !(*min < n->value)) {  // n->value <= *min
            return false;
        }
        if (max && !(n->value < *max)) {  // n->value >= *max
            return false;
        }
        bool left_red = n->left && n->left->color == Color::Red;
        bool right_red = n->right && n->right->color == Color::Red;
        if (n->color == Color::Red && (left_red || right_red)) {
            return false;
        }
        std::size_t lh = 0, rh = 0;
        if (!valid(n->left.get(), min, &n->value, lh)) {
            return false;
        }
        if (!valid(n->right.get(), &n->value, max, rh)) {
            return false;
        }
        if (lh != rh) {
            return false;
        }
        std::size_t sz = 1 + (n->left ? n->left->size : 0) +
                         (n->right ? n->right->size : 0);
        if (sz != n->size) {
            return false;
        }
        bh = lh + (n->color == Color::Black ? 1u : 0u);
        return true;
    }

    NodePtr root_;
};

}  // namespace rb
}  // namespace ca

#endif  // CA_RED_BLACK_TREE_HPP
