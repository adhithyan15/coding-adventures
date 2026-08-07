// binary_tree.hpp — a generic binary tree with traversals and shape predicates,
// in pure ISO C++17, header-only, in namespace ca. A faithful port of the Rust
// `binary-tree` crate (DT03).
// ===========================================================================
//
// A plain binary tree: each node has a value and up to two children. There is no
// ordering invariant — this is the shared substrate the search-tree family
// reuses for traversal and shape checks.
//
// Shape predicates: FULL (every node has 0 or 2 children), COMPLETE (every level
// filled except possibly the last, left-to-right), PERFECT (full and all leaves
// at the same depth). Traversals: inorder / preorder / postorder (depth-first)
// and level_order (breadth-first). `to_array` lays the tree out in level order
// with gaps (std::nullopt); `to_ascii` renders an indented text diagram.
//
// Value semantics: `BinaryTree<T>` deep-copies on copy (Rust's `Box` clone), so
// trees are independent. `to_ascii` streams each value with `operator<<`.
//
// CAVEAT. The depth-first operations (traversals, height, size, find, clone,
// to_ascii) recurse to the tree's height, so a very deep (degenerate) tree can
// overflow the stack; the breadth-first ones (level_order, is_complete) are
// iterative. `to_array` is dense in the height and can allocate exponentially
// for unbalanced trees. This mirrors the Rust crate.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_BINARY_TREE_HPP
#define CA_BINARY_TREE_HPP

#include <cstddef>
#include <deque>
#include <memory>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {

template <class T>
class BinaryTree {
public:
    struct Node {
        T value;
        std::unique_ptr<Node> left;
        std::unique_ptr<Node> right;
        explicit Node(const T& v) : value(v) {}
    };
    using NodePtr = std::unique_ptr<Node>;

    BinaryTree() = default;
    BinaryTree(const BinaryTree& other) : root_(clone(other.root_)) {}
    BinaryTree& operator=(const BinaryTree& other) {
        if (this != &other) {
            root_ = clone(other.root_);
        }
        return *this;
    }
    BinaryTree(BinaryTree&&) noexcept = default;
    BinaryTree& operator=(BinaryTree&&) noexcept = default;
    ~BinaryTree() = default;

    // A leaf node holding `value`; the caller may attach children before
    // handing the root to `with_root`.
    static NodePtr make_node(const T& value) {
        return std::make_unique<Node>(value);
    }

    // A tree owning the node tree `root` (may be null for an empty tree).
    static BinaryTree with_root(NodePtr root) {
        BinaryTree t;
        t.root_ = std::move(root);
        return t;
    }

    // Build from a level-order layout; std::nullopt marks a gap. Index i has
    // children 2i+1 and 2i+2.
    static BinaryTree from_level_order(const std::vector<std::optional<T>>& v) {
        BinaryTree t;
        t.root_ = build_level_order(v, 0);
        return t;
    }

    const Node* root() const { return root_.get(); }

    // First node (preorder) holding `value`, or nullptr.
    const Node* find(const T& value) const { return find_rec(root_.get(), value); }

    const Node* left_child(const T& value) const {
        const Node* n = find(value);
        return n ? n->left.get() : nullptr;
    }
    const Node* right_child(const T& value) const {
        const Node* n = find(value);
        return n ? n->right.get() : nullptr;
    }

    bool is_full() const { return full_rec(root_.get()); }

    bool is_complete() const {
        std::deque<const Node*> queue;
        queue.push_back(root_.get());
        bool seen_none = false;
        while (!queue.empty()) {
            const Node* node = queue.front();
            queue.pop_front();
            if (!node) {
                seen_none = true;
            } else {
                if (seen_none) {
                    return false;
                }
                queue.push_back(node->left.get());
                queue.push_back(node->right.get());
            }
        }
        return true;
    }

    bool is_perfect() const {
        long h = height();
        std::size_t n = size();
        if (h < 0) {
            return n == 0;
        }
        if (static_cast<std::size_t>(h + 1) >= sizeof(std::size_t) * 8) {
            return false;
        }
        return n == ((static_cast<std::size_t>(1) << (h + 1)) - 1);
    }

    long height() const { return height_rec(root_.get()); }
    std::size_t size() const { return size_rec(root_.get()); }

    std::vector<T> inorder() const {
        std::vector<T> out;
        inorder_rec(root_.get(), out);
        return out;
    }
    std::vector<T> preorder() const {
        std::vector<T> out;
        preorder_rec(root_.get(), out);
        return out;
    }
    std::vector<T> postorder() const {
        std::vector<T> out;
        postorder_rec(root_.get(), out);
        return out;
    }
    std::vector<T> level_order() const {
        std::vector<T> out;
        std::deque<const Node*> queue;
        queue.push_back(root_.get());
        while (!queue.empty()) {
            const Node* node = queue.front();
            queue.pop_front();
            if (node) {
                out.push_back(node->value);
                queue.push_back(node->left.get());
                queue.push_back(node->right.get());
            }
        }
        return out;
    }

    // Level-order layout with gaps. Length is 2^(h+1)-1 (empty -> empty vector).
    // NOTE: the layout is dense in the tree height, so a deep/unbalanced tree
    // produces an exponentially large vector. When 2^(h+1)-1 is not even
    // representable in size_t this throws std::length_error rather than shifting
    // out of range (undefined behaviour) or allocating a bogus size.
    std::vector<std::optional<T>> to_array() const {
        long h = height();
        if (h < 0) {
            return {};
        }
        if (static_cast<std::size_t>(h + 1) >= sizeof(std::size_t) * 8) {
            throw std::length_error("binary-tree: to_array layout too large");
        }
        std::size_t len = (static_cast<std::size_t>(1) << (h + 1)) - 1;
        std::vector<std::optional<T>> out(len);
        fill_array(root_.get(), 0, out);
        return out;
    }

    // Indented text diagram (empty string for an empty tree).
    std::string to_ascii() const {
        std::string out;
        if (root_) {
            render_ascii(root_.get(), "", true, out);
        }
        return out;
    }

private:
    static NodePtr clone(const NodePtr& n) {
        if (!n) {
            return nullptr;
        }
        auto c = std::make_unique<Node>(n->value);
        c->left = clone(n->left);
        c->right = clone(n->right);
        return c;
    }

    static NodePtr build_level_order(const std::vector<std::optional<T>>& v,
                                     std::size_t index) {
        if (index >= v.size() || !v[index].has_value()) {
            return nullptr;
        }
        auto node = std::make_unique<Node>(*v[index]);
        if (index <= (static_cast<std::size_t>(-1) - 1) / 2) {
            node->left = build_level_order(v, 2 * index + 1);
        }
        if (index <= (static_cast<std::size_t>(-1) - 2) / 2) {
            node->right = build_level_order(v, 2 * index + 2);
        }
        return node;
    }

    static const Node* find_rec(const Node* n, const T& value) {
        if (!n) {
            return nullptr;
        }
        if (n->value == value) {
            return n;
        }
        const Node* r = find_rec(n->left.get(), value);
        if (r) {
            return r;
        }
        return find_rec(n->right.get(), value);
    }

    static bool full_rec(const Node* n) {
        if (!n) {
            return true;
        }
        bool has_l = n->left != nullptr;
        bool has_r = n->right != nullptr;
        if (!has_l && !has_r) {
            return true;
        }
        if (has_l && has_r) {
            return full_rec(n->left.get()) && full_rec(n->right.get());
        }
        return false;
    }

    static long height_rec(const Node* n) {
        if (!n) {
            return -1;
        }
        long lh = height_rec(n->left.get());
        long rh = height_rec(n->right.get());
        return 1 + (lh > rh ? lh : rh);
    }

    static std::size_t size_rec(const Node* n) {
        if (!n) {
            return 0;
        }
        return 1 + size_rec(n->left.get()) + size_rec(n->right.get());
    }

    static void inorder_rec(const Node* n, std::vector<T>& out) {
        if (!n) {
            return;
        }
        inorder_rec(n->left.get(), out);
        out.push_back(n->value);
        inorder_rec(n->right.get(), out);
    }
    static void preorder_rec(const Node* n, std::vector<T>& out) {
        if (!n) {
            return;
        }
        out.push_back(n->value);
        preorder_rec(n->left.get(), out);
        preorder_rec(n->right.get(), out);
    }
    static void postorder_rec(const Node* n, std::vector<T>& out) {
        if (!n) {
            return;
        }
        postorder_rec(n->left.get(), out);
        postorder_rec(n->right.get(), out);
        out.push_back(n->value);
    }

    static void fill_array(const Node* n, std::size_t index,
                           std::vector<std::optional<T>>& out) {
        if (!n || index >= out.size()) {
            return;
        }
        out[index] = n->value;
        if (index <= (static_cast<std::size_t>(-1) - 1) / 2) {
            fill_array(n->left.get(), 2 * index + 1, out);
        }
        if (index <= (static_cast<std::size_t>(-1) - 2) / 2) {
            fill_array(n->right.get(), 2 * index + 2, out);
        }
    }

    static void render_ascii(const Node* node, const std::string& prefix,
                             bool is_tail, std::string& out) {
        std::ostringstream os;
        os << node->value;
        out += prefix;
        out += is_tail ? "`-- " : "|-- ";
        out += os.str();
        out += "\n";

        const Node* children[2];
        std::size_t nchildren = 0;
        if (node->left) {
            children[nchildren++] = node->left.get();
        }
        if (node->right) {
            children[nchildren++] = node->right.get();
        }
        std::string next_prefix = prefix + (is_tail ? "    " : "|   ");
        for (std::size_t i = 0; i < nchildren; ++i) {
            bool last = (i + 1 == nchildren);
            render_ascii(children[i], next_prefix, last, out);
        }
    }

    NodePtr root_;
};

}  // namespace ca

#endif  // CA_BINARY_TREE_HPP
