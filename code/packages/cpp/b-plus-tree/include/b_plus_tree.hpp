// b_plus_tree.hpp — a generic B+ tree (minimum degree `t`), in pure ISO C++17
// (header-only). A faithful port of the Rust `b-plus-tree` crate, in namespace
// `ca`.
// ===========================================================================
//
// A B+ tree stores all values in its leaves; internal nodes hold only separator
// keys for routing, and the leaves form a singly-linked list (`next`) so a range
// scan finds one leaf and then walks the chain. This port implements the full
// algorithm — leaf/internal splitting on insert (propagated bottom-up), and
// borrow-from-sibling / merge rebalancing on delete — while keeping the leaf
// chain in sync.
//
// Unlike the C sibling (specialised to long -> long), this header is fully
// generic: `ca::b_plus_tree<K, V>` works for any less-than-comparable key.
// The leaf `next` link is a non-owning raw pointer into the `unique_ptr`-owned
// tree — the C++ analogue of the Rust crate's `*mut` leaf chain.
//
//   insert / remove / search / contains — the map operations
//   range_scan / full_scan              — ordered scans over the leaf chain
//   min_key / max_key / height / is_valid — introspection
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef B_PLUS_TREE_HPP
#define B_PLUS_TREE_HPP

#include <algorithm>
#include <cstddef>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

namespace ca {

template <class K, class V>
class b_plus_tree {
public:
    explicit b_plus_tree(std::size_t t) : t_(t < 2 ? 2 : t) {
        root_ = std::make_unique<node>();
        root_->is_leaf = true;
        first_leaf_ = root_.get();
    }

    void insert(const K &key, const V &value) {
        split_out res = insert_node(*root_, key, value, t_);
        if (res.sep.has_value()) {
            auto new_root = std::make_unique<node>();
            new_root->is_leaf = false;
            new_root->keys.push_back(*res.sep);
            new_root->children.push_back(std::move(root_));
            new_root->children.push_back(std::move(res.right));
            root_ = std::move(new_root);
        }
        if (res.grew) {
            size_++;
        }
        first_leaf_ = leftmost_leaf(root_.get());
    }

    bool remove(const K &key) {
        bool underfull = false;
        if (!delete_node(*root_, key, t_, true, underfull)) {
            return false;
        }
        if (!root_->is_leaf && root_->keys.empty()) {
            root_ = std::move(root_->children[0]);
        }
        size_--;
        first_leaf_ = leftmost_leaf(root_.get());
        return true;
    }

    const V *search(const K &key) const {
        const node *leaf = find_leaf(root_.get(), key);
        auto it = std::lower_bound(leaf->keys.begin(), leaf->keys.end(), key);
        if (it != leaf->keys.end() && !(key < *it)) {
            return &leaf->values[static_cast<std::size_t>(
                it - leaf->keys.begin())];
        }
        return nullptr;
    }
    bool contains(const K &key) const { return search(key) != nullptr; }

    std::optional<K> min_key() const {
        if (size_ == 0) {
            return std::nullopt;
        }
        return first_leaf_->keys.front();
    }
    std::optional<K> max_key() const {
        if (size_ == 0) {
            return std::nullopt;
        }
        const node *nd = root_.get();
        while (!nd->is_leaf) {
            nd = nd->children.back().get();
        }
        return nd->keys.back();
    }

    std::size_t len() const { return size_; }
    bool empty() const { return size_ == 0; }

    std::size_t height() const {
        const node *nd = root_.get();
        std::size_t h = 0;
        while (!nd->is_leaf) {
            h++;
            nd = nd->children[0].get();
        }
        return h;
    }

    // Every entry in ascending order, by walking the leaf chain.
    std::vector<std::pair<K, V>> full_scan() const {
        std::vector<std::pair<K, V>> out;
        const node *cur = first_leaf_;
        while (cur != nullptr) {
            for (std::size_t i = 0; i < cur->keys.size(); i++) {
                out.emplace_back(cur->keys[i], cur->values[i]);
            }
            cur = cur->next;
        }
        return out;
    }

    // Entries with low <= key <= high, in order, via the leaf chain.
    std::vector<std::pair<K, V>> range_scan(const K &low, const K &high) const {
        std::vector<std::pair<K, V>> out;
        const node *cur = find_leaf(root_.get(), low);
        bool emitted = false;
        while (cur != nullptr) {
            bool done = true;
            for (std::size_t i = 0; i < cur->keys.size(); i++) {
                if (high < cur->keys[i]) {
                    break;
                }
                if (!(cur->keys[i] < low)) {
                    out.emplace_back(cur->keys[i], cur->values[i]);
                    done = false;
                    emitted = true;
                }
            }
            if (done && emitted) {
                break;
            }
            cur = cur->next;
        }
        return out;
    }

    bool is_valid() const {
        std::size_t leaf_depth = 0, count = 0;
        if (!validate_node(*root_, t_, true, 0, leaf_depth, count)) {
            return false;
        }
        if (count != size_) {
            return false;
        }
        std::size_t list_count = 0;
        bool have_prev = false;
        K prev{};
        const node *cur = first_leaf_;
        while (cur != nullptr) {
            for (const K &k : cur->keys) {
                if (have_prev && !(prev < k)) {
                    return false;
                }
                prev = k;
                have_prev = true;
                list_count++;
            }
            cur = cur->next;
        }
        return list_count == size_;
    }

private:
    struct node {
        bool is_leaf = false;
        std::vector<K> keys;
        std::vector<V> values;                       // leaf
        std::vector<std::unique_ptr<node>> children; // internal
        node *next = nullptr;                         // leaf chain (non-owning)
    };

    struct split_out {
        std::optional<K> sep;         // set iff the node split
        std::unique_ptr<node> right;  // the new right sibling
        bool grew = false;
    };

    std::unique_ptr<node> root_;
    node *first_leaf_ = nullptr;
    std::size_t t_;
    std::size_t size_ = 0;

    static std::size_t count_le(const std::vector<K> &keys, const K &key) {
        return static_cast<std::size_t>(
            std::upper_bound(keys.begin(), keys.end(), key) - keys.begin());
    }
    static std::size_t count_lt(const std::vector<K> &keys, const K &key) {
        return static_cast<std::size_t>(
            std::lower_bound(keys.begin(), keys.end(), key) - keys.begin());
    }
    static std::size_t child_index(const node &nd, const K &key) {
        std::size_t ci = count_le(nd.keys, key);
        return ci > nd.keys.size() ? nd.keys.size() : ci;
    }

    static const node *find_leaf(const node *nd, const K &key) {
        while (!nd->is_leaf) {
            nd = nd->children[child_index(*nd, key)].get();
        }
        return nd;
    }
    static node *leftmost_leaf(node *nd) {
        while (!nd->is_leaf) {
            nd = nd->children[0].get();
        }
        return nd;
    }

    static split_out insert_node(node &nd, const K &key, const V &value,
                                 std::size_t t) {
        return nd.is_leaf ? insert_leaf(nd, key, value, t)
                          : insert_internal(nd, key, value, t);
    }

    static split_out insert_leaf(node &leaf, const K &key, const V &value,
                                 std::size_t t) {
        auto it = std::lower_bound(leaf.keys.begin(), leaf.keys.end(), key);
        std::size_t pos = static_cast<std::size_t>(it - leaf.keys.begin());
        if (it != leaf.keys.end() && !(key < *it)) {
            leaf.values[pos] = value;
            return {std::nullopt, nullptr, false};
        }
        leaf.keys.insert(leaf.keys.begin() + pos, key);
        leaf.values.insert(leaf.values.begin() + pos, value);
        if (leaf.keys.size() >= 2 * t) {
            auto right = std::make_unique<node>();
            right->is_leaf = true;
            right->keys.assign(
                std::make_move_iterator(leaf.keys.begin() + t),
                std::make_move_iterator(leaf.keys.end()));
            leaf.keys.erase(leaf.keys.begin() + t, leaf.keys.end());
            right->values.assign(
                std::make_move_iterator(leaf.values.begin() + t),
                std::make_move_iterator(leaf.values.end()));
            leaf.values.erase(leaf.values.begin() + t, leaf.values.end());
            K sep = right->keys[0]; // copy of right's first key
            right->next = leaf.next;
            leaf.next = right.get();
            return {std::move(sep), std::move(right), true};
        }
        return {std::nullopt, nullptr, true};
    }

    static split_out insert_internal(node &nd, const K &key, const V &value,
                                     std::size_t t) {
        std::size_t ci = child_index(nd, key);
        split_out res = insert_node(*nd.children[ci], key, value, t);
        if (!res.sep.has_value()) {
            return {std::nullopt, nullptr, res.grew};
        }
        std::size_t pos = count_lt(nd.keys, *res.sep);
        nd.keys.insert(nd.keys.begin() + pos, *res.sep);
        nd.children.insert(nd.children.begin() + pos + 1, std::move(res.right));
        if (nd.keys.size() >= 2 * t) {
            std::size_t mid = t - 1;
            K promote = nd.keys[mid];
            auto right = std::make_unique<node>();
            right->is_leaf = false;
            right->keys.assign(std::make_move_iterator(nd.keys.begin() + mid + 1),
                               std::make_move_iterator(nd.keys.end()));
            for (std::size_t j = mid + 1; j < nd.children.size(); j++) {
                right->children.push_back(std::move(nd.children[j]));
            }
            nd.keys.erase(nd.keys.begin() + mid, nd.keys.end());
            nd.children.erase(nd.children.begin() + mid + 1, nd.children.end());
            return {std::move(promote), std::move(right), res.grew};
        }
        return {std::nullopt, nullptr, res.grew};
    }

    static bool leftmost_key(const node *nd, K &out) {
        while (!nd->is_leaf) {
            nd = nd->children[0].get();
        }
        if (nd->keys.empty()) {
            return false;
        }
        out = nd->keys.front();
        return true;
    }

    static void maybe_update_separator(node &nd, std::size_t ci) {
        if (ci > 0 && ci <= nd.keys.size()) {
            K lk{};
            if (leftmost_key(nd.children[ci].get(), lk)) {
                nd.keys[ci - 1] = lk;
            }
        }
    }

    static void borrow_from_left(node &nd, std::size_t ci) {
        node &left = *nd.children[ci - 1];
        node &right = *nd.children[ci];
        if (right.is_leaf) {
            right.keys.insert(right.keys.begin(), std::move(left.keys.back()));
            right.values.insert(right.values.begin(),
                                std::move(left.values.back()));
            left.keys.pop_back();
            left.values.pop_back();
            nd.keys[ci - 1] = right.keys.front();
        } else {
            K sep = std::move(nd.keys[ci - 1]);
            nd.keys[ci - 1] = std::move(left.keys.back());
            left.keys.pop_back();
            right.keys.insert(right.keys.begin(), std::move(sep));
            right.children.insert(right.children.begin(),
                                  std::move(left.children.back()));
            left.children.pop_back();
        }
    }

    static void borrow_from_right(node &nd, std::size_t ci) {
        node &left = *nd.children[ci];
        node &right = *nd.children[ci + 1];
        if (left.is_leaf) {
            left.keys.push_back(std::move(right.keys.front()));
            left.values.push_back(std::move(right.values.front()));
            right.keys.erase(right.keys.begin());
            right.values.erase(right.values.begin());
            nd.keys[ci] = right.keys.front();
        } else {
            K sep = std::move(nd.keys[ci]);
            nd.keys[ci] = std::move(right.keys.front());
            right.keys.erase(right.keys.begin());
            left.keys.push_back(std::move(sep));
            left.children.push_back(std::move(right.children.front()));
            right.children.erase(right.children.begin());
        }
    }

    // Merge children[l] and children[l+1] with keys[l] as the separator.
    static void merge_pair(node &nd, std::size_t l) {
        std::unique_ptr<node> right = std::move(nd.children[l + 1]);
        nd.children.erase(nd.children.begin() + l + 1);
        K sep = std::move(nd.keys[l]);
        nd.keys.erase(nd.keys.begin() + l);
        node &left = *nd.children[l];
        if (left.is_leaf) {
            for (auto &k : right->keys) {
                left.keys.push_back(std::move(k));
            }
            for (auto &v : right->values) {
                left.values.push_back(std::move(v));
            }
            left.next = right->next; // unlink `right` from the leaf chain
        } else {
            left.keys.push_back(std::move(sep));
            for (auto &k : right->keys) {
                left.keys.push_back(std::move(k));
            }
            for (auto &c : right->children) {
                left.children.push_back(std::move(c));
            }
        }
        // `right` is destroyed here; its children were moved out (internal) and
        // its next was copied (leaf), so nothing owned is lost or double-freed.
    }

    static void fix_underfull(node &nd, std::size_t ci, std::size_t t) {
        std::size_t n_children = nd.children.size();
        if (ci > 0 && nd.children[ci - 1]->keys.size() >= t) {
            borrow_from_left(nd, ci);
            return;
        }
        if (ci + 1 < n_children && nd.children[ci + 1]->keys.size() >= t) {
            borrow_from_right(nd, ci);
            return;
        }
        if (ci > 0) {
            merge_pair(nd, ci - 1);
        } else {
            merge_pair(nd, ci);
        }
    }

    static bool delete_node(node &nd, const K &key, std::size_t t, bool is_root,
                            bool &underfull) {
        std::size_t min_keys = is_root ? 0 : t - 1;
        if (nd.is_leaf) {
            auto it = std::lower_bound(nd.keys.begin(), nd.keys.end(), key);
            if (it == nd.keys.end() || key < *it) {
                return false;
            }
            std::size_t i = static_cast<std::size_t>(it - nd.keys.begin());
            nd.keys.erase(nd.keys.begin() + i);
            nd.values.erase(nd.values.begin() + i);
            underfull = nd.keys.size() < min_keys;
            return true;
        }
        std::size_t ci = child_index(nd, key);
        bool child_underfull = false;
        if (!delete_node(*nd.children[ci], key, t, false, child_underfull)) {
            return false;
        }
        if (child_underfull) {
            fix_underfull(nd, ci, t);
        } else {
            maybe_update_separator(nd, ci);
        }
        underfull = nd.keys.size() < min_keys;
        return true;
    }

    static bool validate_node(const node &nd, std::size_t t, bool is_root,
                              std::size_t depth, std::size_t &out_leaf_depth,
                              std::size_t &out_count) {
        std::size_t min_keys = is_root ? 0 : t - 1;
        std::size_t max_keys = 2 * t - 1;
        if (nd.keys.size() > max_keys ||
            (!is_root && nd.keys.size() < min_keys)) {
            return false;
        }
        for (std::size_t i = 1; i < nd.keys.size(); i++) {
            if (!(nd.keys[i - 1] < nd.keys[i])) {
                return false;
            }
        }
        if (nd.is_leaf) {
            out_leaf_depth = depth;
            out_count = nd.keys.size();
            return true;
        }
        if (nd.children.size() != nd.keys.size() + 1) {
            return false;
        }
        bool have = false;
        std::size_t ld = depth;
        std::size_t total = 0;
        for (const auto &child : nd.children) {
            std::size_t cd = 0, cc = 0;
            if (!validate_node(*child, t, false, depth + 1, cd, cc)) {
                return false;
            }
            if (!have) {
                have = true;
                ld = cd;
            } else if (ld != cd) {
                return false;
            }
            total += cc;
        }
        out_leaf_depth = ld;
        out_count = total;
        return true;
    }
};

} // namespace ca

#endif // B_PLUS_TREE_HPP
