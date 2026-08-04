// treap.hpp — a treap (tree + heap), a randomized balanced BST, in pure ISO
// C++17, header-only, in namespace ca::treap. A faithful port of the Rust
// `treap` crate (DT10).
// ===========================================================================
//
// A treap stores each key together with a random `priority` and keeps two
// invariants at once:
//
//   - BST order on the KEYS:        left subtree < node < right subtree.
//   - MAX-HEAP order on PRIORITIES:  node priority >= its children's.
//
// Because the priorities are random, the heap constraint forces a shape that is
// balanced *in expectation* — O(log n) search / insert / delete with high
// probability — without the explicit rebalancing an AVL or red-black tree needs.
// Rotations during insert, and a priority-ordered `merge` during erase, restore
// the heap invariant.
//
//   split(key) -> (<= key, > key)      merge(l, r)  (all l-keys < all r-keys)
//
// `split` and `merge` are the treap's signature operations and run in O(log n).
// Each node caches its subtree `size`, making `kth_smallest` / order statistics
// O(h).
//
// PRIORITIES. `insert` takes `std::optional<double>`: a value uses that exact
// priority; std::nullopt draws one from a built-in deterministic PRNG. NOTE: the
// Rust crate seeds that PRNG through a global AtomicU32 for cross-thread safety;
// this port uses a plain function-local `static` counter (identical arithmetic,
// single-threaded). Supply priorities explicitly for reproducibility.
//
// PERSISTENCE. The Rust crate is *persistent*: `insert`/`erase`/`split`/`merge`
// return brand-new treaps and leave their inputs untouched. This port keeps that
// through value semantics: `Treap<K>` deep-copies on copy, and `insert`/`erase`/
// `split` are `const` — they copy `*this` and work on the copy. (Rust's `Box`
// gives deep-clone persistence, which is what value semantics mirror.)
//
// The key type `K` must be less-than comparable (equality derived as
// `!(a < b) && !(b < a)`) and copyable — matching Rust's `K: Ord + Clone`.
//
// CAVEAT. All operations (and the `unique_ptr` teardown) recurse to the treap's
// height. With random priorities that is O(log n) in expectation, but a caller
// supplying adversarial explicit priorities (e.g. a monotonic sequence) can
// force a height-n degenerate chain and overflow the stack. Use nullopt (the
// PRNG) for untrusted input. This mirrors the Rust crate.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_TREAP_HPP
#define CA_TREAP_HPP

#include <cstddef>
#include <cstdint>
#include <limits>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

namespace ca {
namespace treap {

// A value in [0, 1] from a deterministic xorshift generator. The Rust crate
// holds the seed in an AtomicU32 (fetch_add) for cross-thread safety; a pure-ISO
// single-threaded port uses a function-local static with identical arithmetic.
// uint32 multiply/add wrap mod 2^32 by definition, so there is no UB.
inline double next_priority() {
    static std::uint32_t seed = 0x9E3779B9u;
    std::uint32_t state = seed;  // fetch_add returns the value *before* the add
    seed += 0x9E3779B9u;
    state ^= state >> 13;
    state ^= state << 17;
    state ^= state >> 5;
    std::uint32_t mixed = state * 0x85EBCA6Bu;  // wrapping multiply
    return static_cast<double>(mixed) /
           static_cast<double>(std::numeric_limits<std::uint32_t>::max());
}

template <class K>
class Treap {
public:
    // A node. `size` is the number of nodes in this subtree (>= 1).
    struct Node {
        K key;
        double priority;
        std::unique_ptr<Node> left;
        std::unique_ptr<Node> right;
        std::size_t size = 1;
        Node(const K& k, double p) : key(k), priority(p) {}
    };

    Treap() = default;
    Treap(const Treap& other) : root_(clone(other.root_)) {}
    Treap& operator=(const Treap& other) {
        if (this != &other) {
            root_ = clone(other.root_);
        }
        return *this;
    }
    Treap(Treap&&) noexcept = default;
    Treap& operator=(Treap&&) noexcept = default;
    ~Treap() = default;

    // An empty treap.
    static Treap empty() { return Treap(); }

    // Persistent insert. A specific `priority`, or nullopt to draw from the
    // PRNG. Re-inserting an existing key is a no-op. `*this` is left unchanged.
    Treap insert(const K& key,
                 std::optional<double> priority = std::nullopt) const {
        double p = priority.has_value() ? *priority : next_priority();
        Treap copy(*this);
        copy.root_ = ins(std::move(copy.root_), key, p);
        return copy;
    }

    // Persistent erase: a new treap with `key` removed if present.
    Treap erase(const K& key) const {
        Treap copy(*this);
        copy.root_ = del(std::move(copy.root_), key);
        return copy;
    }

    // Persistent split into (keys <= `key`, keys > `key`). `*this` unchanged.
    std::pair<Treap, Treap> split(const K& key) const {
        NodePtr root = clone(root_);
        NodePtr l, r;
        split_nodes(std::move(root), key, l, r);
        Treap left, right;
        left.root_ = std::move(l);
        right.root_ = std::move(r);
        return {std::move(left), std::move(right)};
    }

    // Merge two key-disjoint treaps (every key of `left` < every key of
    // `right`, as produced by `split`). Inputs are taken by value (copies).
    static Treap merge(Treap left, Treap right) {
        Treap out;
        out.root_ = merge_nodes(std::move(left.root_), std::move(right.root_));
        return out;
    }

    const Node* find(const K& key) const {
        const Node* cur = root_.get();
        while (cur) {
            if (key < cur->key) {
                cur = cur->left.get();
            } else if (cur->key < key) {
                cur = cur->right.get();
            } else {
                return cur;
            }
        }
        return nullptr;
    }

    bool contains(const K& key) const { return find(key) != nullptr; }

    std::optional<K> min_key() const {
        const Node* cur = root_.get();
        if (!cur) {
            return std::nullopt;
        }
        while (cur->left) {
            cur = cur->left.get();
        }
        return cur->key;
    }

    std::optional<K> max_key() const {
        const Node* cur = root_.get();
        if (!cur) {
            return std::nullopt;
        }
        while (cur->right) {
            cur = cur->right.get();
        }
        return cur->key;
    }

    // Largest stored key strictly less than `key`.
    std::optional<K> predecessor(const K& key) const {
        const Node* cur = root_.get();
        std::optional<K> best;
        while (cur) {
            if (!(cur->key < key)) {  // key <= cur->key
                cur = cur->left.get();
            } else {
                best = cur->key;
                cur = cur->right.get();
            }
        }
        return best;
    }

    // Smallest stored key strictly greater than `key`.
    std::optional<K> successor(const K& key) const {
        const Node* cur = root_.get();
        std::optional<K> best;
        while (cur) {
            if (!(key < cur->key)) {  // key >= cur->key
                cur = cur->right.get();
            } else {
                best = cur->key;
                cur = cur->left.get();
            }
        }
        return best;
    }

    // The k-th smallest key (1-based); nullopt if k == 0 or k > size.
    std::optional<K> kth_smallest(std::size_t k) const {
        if (k == 0) {
            return std::nullopt;
        }
        const Node* cur = root_.get();
        while (cur) {
            std::size_t ls = sub_size(cur->left);
            if (k == ls + 1) {
                return cur->key;
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

    std::vector<K> to_sorted_array() const {
        std::vector<K> out;
        inorder(root_.get(), out);
        return out;
    }

    std::size_t size() const { return sub_size(root_); }
    long height() const { return sub_height(root_.get()); }
    const Node* root() const { return root_.get(); }

    // True iff both the BST and max-heap invariants hold and sizes are correct.
    bool is_valid() const {
        return validate(root_.get(), nullptr, nullptr, nullptr) >= 0;
    }

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

    static void update_metadata(const NodePtr& n) {
        n->size = 1 + sub_size(n->left) + sub_size(n->right);
    }

    static NodePtr clone(const NodePtr& n) {
        if (!n) {
            return nullptr;
        }
        auto c = std::make_unique<Node>(n->key, n->priority);
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
        update_metadata(root);
        new_root->left = std::move(root);
        update_metadata(new_root);
        return new_root;
    }

    static NodePtr rotate_right(NodePtr root) {
        if (!root->left) {
            return root;
        }
        NodePtr new_root = std::move(root->left);
        root->left = std::move(new_root->right);
        update_metadata(root);
        new_root->right = std::move(root);
        update_metadata(new_root);
        return new_root;
    }

    static NodePtr ins(NodePtr root, const K& key, double priority) {
        if (!root) {
            return std::make_unique<Node>(key, priority);
        }
        if (key < root->key) {
            root->left = ins(std::move(root->left), key, priority);
            if (root->left && root->left->priority > root->priority) {
                root = rotate_right(std::move(root));
            }
        } else if (root->key < key) {
            root->right = ins(std::move(root->right), key, priority);
            if (root->right && root->right->priority > root->priority) {
                root = rotate_left(std::move(root));
            }
        } else {
            return root;  // duplicate key — no-op
        }
        update_metadata(root);
        return root;
    }

    // Merge two key-disjoint subtrees (all left keys < all right keys).
    static NodePtr merge_nodes(NodePtr left, NodePtr right) {
        if (!left) {
            return right;
        }
        if (!right) {
            return left;
        }
        if (left->priority >= right->priority) {
            left->right = merge_nodes(std::move(left->right), std::move(right));
            update_metadata(left);
            return left;
        }
        right->left = merge_nodes(std::move(left), std::move(right->left));
        update_metadata(right);
        return right;
    }

    static NodePtr del(NodePtr root, const K& key) {
        if (!root) {
            return nullptr;
        }
        if (key < root->key) {
            root->left = del(std::move(root->left), key);
            update_metadata(root);
            return root;
        }
        if (root->key < key) {
            root->right = del(std::move(root->right), key);
            update_metadata(root);
            return root;
        }
        // Equal — drop this node, merging its children.
        return merge_nodes(std::move(root->left), std::move(root->right));
    }

    static void split_nodes(NodePtr node, const K& key, NodePtr& left_out,
                            NodePtr& right_out) {
        if (!node) {
            left_out = nullptr;
            right_out = nullptr;
            return;
        }
        if (key < node->key) {
            NodePtr l, r;
            split_nodes(std::move(node->left), key, l, r);
            node->left = std::move(r);
            update_metadata(node);
            left_out = std::move(l);
            right_out = std::move(node);
        } else {  // key >= node->key: this node belongs on the left
            NodePtr l, r;
            split_nodes(std::move(node->right), key, l, r);
            node->right = std::move(l);
            update_metadata(node);
            left_out = std::move(node);
            right_out = std::move(r);
        }
    }

    static void inorder(const Node* n, std::vector<K>& out) {
        if (!n) {
            return;
        }
        inorder(n->left.get(), out);
        out.push_back(n->key);
        inorder(n->right.get(), out);
    }

    // Returns the subtree size, or -1 on any invariant violation.
    static long validate(const Node* n, const K* min, const K* max,
                         const double* parent_prio) {
        if (!n) {
            return 0;
        }
        if (min && !(*min < n->key)) {  // n->key <= *min
            return -1;
        }
        if (max && *max < n->key) {  // n->key > *max
            return -1;
        }
        if (parent_prio && n->priority > *parent_prio) {
            return -1;
        }
        long left = validate(n->left.get(), min, &n->key, &n->priority);
        if (left < 0) {
            return -1;
        }
        long right = validate(n->right.get(), &n->key, max, &n->priority);
        if (right < 0) {
            return -1;
        }
        if (n->size != static_cast<std::size_t>(1 + left + right)) {
            return -1;
        }
        return 1 + left + right;
    }

    NodePtr root_;
};

}  // namespace treap
}  // namespace ca

#endif  // CA_TREAP_HPP
