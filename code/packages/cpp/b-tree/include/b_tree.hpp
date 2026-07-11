// b_tree.hpp — a generic B-tree (minimum degree `t`), in pure ISO C++17
// (header-only). A faithful port of the Rust `b-tree` crate, in namespace `ca`.
// ===========================================================================
//
// A B-tree is a balanced search tree with high fan-out: every node holds many
// keys, so the tree stays short. With minimum degree `t`, every non-root node
// holds `t-1`..`2t-1` keys and `t`..`2t` children, and all leaves are at the
// same depth. This port implements the full CLRS algorithm — proactive
// top-down splitting on insert, pre-fill (rotate or merge) on delete.
//
// Unlike the C sibling (which specialises to long -> long), this header is fully
// generic: `ca::b_tree<K, V>` works for any less-than-comparable key `K`.
//
//   insert / remove / search / contains  — the map operations
//   min_key / max_key                     — extremes
//   inorder / range_query                 — ordered traversal (vectors of pairs)
//   len / empty / height / is_valid       — introspection
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef B_TREE_HPP
#define B_TREE_HPP

#include <algorithm>
#include <cstddef>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

namespace ca {

template <class K, class V>
class b_tree {
public:
    explicit b_tree(std::size_t t) : t_(t < 2 ? 2 : t) {}

    // Insert or overwrite key -> value.
    void insert(const K &key, const V &value) {
        if (!root_) {
            root_ = std::make_unique<node>();
            root_->is_leaf = true;
            root_->keys.push_back(key);
            root_->values.push_back(value);
            size_++;
            return;
        }
        if (root_->keys.size() == 2 * t_ - 1) {
            auto new_root = std::make_unique<node>();
            new_root->is_leaf = false;
            new_root->children.push_back(std::move(root_));
            split_child(*new_root, 0, t_);
            bool grew = insert_non_full(*new_root, key, value, t_);
            root_ = std::move(new_root);
            if (grew) {
                size_++;
            }
        } else {
            if (insert_non_full(*root_, key, value, t_)) {
                size_++;
            }
        }
    }

    // Remove key; returns true if it was present.
    bool remove(const K &key) {
        if (!root_) {
            return false;
        }
        bool deleted = node_delete(*root_, key, t_);
        if (deleted) {
            size_--;
            if (root_->keys.empty()) {
                if (!root_->children.empty()) {
                    root_ = std::move(root_->children[0]);
                } else {
                    root_.reset();
                }
            }
        }
        return deleted;
    }

    // Look up key; returns a pointer to the value, or nullptr if absent.
    const V *search(const K &key) const {
        const node *nd = root_.get();
        while (nd != nullptr) {
            auto fp = find_pos(*nd, key);
            if (fp.first) {
                return &nd->values[fp.second];
            }
            if (nd->is_leaf) {
                return nullptr;
            }
            nd = nd->children[fp.second].get();
        }
        return nullptr;
    }
    bool contains(const K &key) const { return search(key) != nullptr; }

    std::optional<K> min_key() const {
        if (!root_) {
            return std::nullopt;
        }
        const node *nd = root_.get();
        while (!nd->is_leaf) {
            nd = nd->children[0].get();
        }
        return nd->keys.front();
    }
    std::optional<K> max_key() const {
        if (!root_) {
            return std::nullopt;
        }
        const node *nd = root_.get();
        while (!nd->is_leaf) {
            nd = nd->children.back().get();
        }
        return nd->keys.back();
    }

    std::vector<std::pair<K, V>> inorder() const {
        std::vector<std::pair<K, V>> out;
        if (root_) {
            node_inorder(*root_, out);
        }
        return out;
    }

    std::vector<std::pair<K, V>> range_query(const K &low, const K &high) const {
        std::vector<std::pair<K, V>> out;
        if (root_) {
            node_range(*root_, low, high, out);
        }
        return out;
    }

    std::size_t len() const { return size_; }
    bool empty() const { return size_ == 0; }

    std::size_t height() const {
        if (!root_) {
            return 0;
        }
        const node *nd = root_.get();
        std::size_t h = 0;
        while (!nd->is_leaf) {
            h++;
            nd = nd->children[0].get();
        }
        return h;
    }

    bool is_valid() const {
        if (!root_ || root_->keys.empty()) {
            return true;
        }
        std::size_t depth = 0;
        return validate(*root_, t_, true, 0, depth);
    }

private:
    struct node {
        std::vector<K> keys;
        std::vector<V> values;
        std::vector<std::unique_ptr<node>> children;
        bool is_leaf = true;
    };

    std::unique_ptr<node> root_;
    std::size_t t_;
    std::size_t size_ = 0;

    // {found, index}: index is the match position or the descent child index.
    static std::pair<bool, std::size_t> find_pos(const node &nd, const K &key) {
        auto it = std::lower_bound(nd.keys.begin(), nd.keys.end(), key);
        std::size_t i = static_cast<std::size_t>(it - nd.keys.begin());
        if (it != nd.keys.end() && !(key < *it)) {
            return {true, i}; // *it == key
        }
        return {false, i};
    }

    static void split_child(node &parent, std::size_t ci, std::size_t t) {
        node &full = *parent.children[ci];
        std::size_t median = t - 1;
        auto right = std::make_unique<node>();
        right->is_leaf = full.is_leaf;

        right->keys.assign(std::make_move_iterator(full.keys.begin() + median + 1),
                           std::make_move_iterator(full.keys.end()));
        full.keys.erase(full.keys.begin() + median + 1, full.keys.end());
        right->values.assign(
            std::make_move_iterator(full.values.begin() + median + 1),
            std::make_move_iterator(full.values.end()));
        full.values.erase(full.values.begin() + median + 1, full.values.end());
        if (!full.is_leaf) {
            for (std::size_t j = median + 1; j < full.children.size(); j++) {
                right->children.push_back(std::move(full.children[j]));
            }
            full.children.erase(full.children.begin() + median + 1,
                                full.children.end());
        }

        K mk = std::move(full.keys[median]);
        V mv = std::move(full.values[median]);
        full.keys.pop_back();
        full.values.pop_back();

        parent.keys.insert(parent.keys.begin() + ci, std::move(mk));
        parent.values.insert(parent.values.begin() + ci, std::move(mv));
        parent.children.insert(parent.children.begin() + ci + 1,
                               std::move(right));
    }

    static bool insert_non_full(node &nd, const K &key, const V &value,
                                std::size_t t) {
        auto fp = find_pos(nd, key);
        if (fp.first) {
            nd.values[fp.second] = value;
            return false;
        }
        if (nd.is_leaf) {
            nd.keys.insert(nd.keys.begin() + fp.second, key);
            nd.values.insert(nd.values.begin() + fp.second, value);
            return true;
        }
        std::size_t idx = fp.second;
        if (nd.children[idx]->keys.size() == 2 * t - 1) {
            split_child(nd, idx, t);
            if (!(key < nd.keys[idx]) && !(nd.keys[idx] < key)) {
                nd.values[idx] = value; // promoted median equals key
                return false;
            }
            if (nd.keys[idx] < key) {
                idx++;
            }
        }
        return insert_non_full(*nd.children[idx], key, value, t);
    }

    static std::pair<K, V> predecessor(node &nd, std::size_t idx) {
        node *n = nd.children[idx].get();
        while (!n->is_leaf) {
            n = n->children.back().get();
        }
        return {n->keys.back(), n->values.back()};
    }
    static std::pair<K, V> successor(node &nd, std::size_t idx) {
        node *n = nd.children[idx + 1].get();
        while (!n->is_leaf) {
            n = n->children[0].get();
        }
        return {n->keys.front(), n->values.front()};
    }

    static void merge_children(node &nd, std::size_t idx) {
        std::unique_ptr<node> right = std::move(nd.children[idx + 1]);
        nd.children.erase(nd.children.begin() + idx + 1);
        K sk = std::move(nd.keys[idx]);
        V sv = std::move(nd.values[idx]);
        nd.keys.erase(nd.keys.begin() + idx);
        nd.values.erase(nd.values.begin() + idx);
        node &left = *nd.children[idx];
        left.keys.push_back(std::move(sk));
        left.values.push_back(std::move(sv));
        for (auto &k : right->keys) {
            left.keys.push_back(std::move(k));
        }
        for (auto &v : right->values) {
            left.values.push_back(std::move(v));
        }
        for (auto &c : right->children) {
            left.children.push_back(std::move(c));
        }
    }

    static void rotate_left(node &nd, std::size_t idx) {
        node &left = *nd.children[idx];
        node &right = *nd.children[idx + 1];
        left.keys.push_back(std::move(nd.keys[idx]));
        left.values.push_back(std::move(nd.values[idx]));
        nd.keys[idx] = std::move(right.keys.front());
        nd.values[idx] = std::move(right.values.front());
        right.keys.erase(right.keys.begin());
        right.values.erase(right.values.begin());
        if (!right.is_leaf) {
            left.children.push_back(std::move(right.children.front()));
            right.children.erase(right.children.begin());
        }
    }

    static void rotate_right(node &nd, std::size_t idx) {
        node &right = *nd.children[idx];
        node &left = *nd.children[idx - 1];
        right.keys.insert(right.keys.begin(), std::move(nd.keys[idx - 1]));
        right.values.insert(right.values.begin(), std::move(nd.values[idx - 1]));
        nd.keys[idx - 1] = std::move(left.keys.back());
        nd.values[idx - 1] = std::move(left.values.back());
        left.keys.pop_back();
        left.values.pop_back();
        if (!left.is_leaf) {
            right.children.insert(right.children.begin(),
                                  std::move(left.children.back()));
            left.children.pop_back();
        }
    }

    static std::size_t ensure_child_has_t_keys(node &nd, std::size_t idx,
                                               std::size_t t) {
        if (nd.children[idx]->keys.size() >= t) {
            return idx;
        }
        bool has_left = idx > 0;
        bool has_right = idx + 1 < nd.children.size();
        if (has_left && nd.children[idx - 1]->keys.size() >= t) {
            rotate_right(nd, idx);
            return idx;
        }
        if (has_right && nd.children[idx + 1]->keys.size() >= t) {
            rotate_left(nd, idx);
            return idx;
        }
        if (has_left) {
            merge_children(nd, idx - 1);
            return idx - 1;
        }
        merge_children(nd, idx);
        return idx;
    }

    static bool node_delete(node &nd, const K &key, std::size_t t) {
        auto fp = find_pos(nd, key);
        if (fp.first) {
            std::size_t i = fp.second;
            if (nd.is_leaf) {
                nd.keys.erase(nd.keys.begin() + i);
                nd.values.erase(nd.values.begin() + i);
                return true;
            }
            if (nd.children[i]->keys.size() >= t) {
                auto pred = predecessor(nd, i);
                nd.keys[i] = pred.first;
                nd.values[i] = pred.second;
                return node_delete(*nd.children[i], pred.first, t);
            }
            if (nd.children[i + 1]->keys.size() >= t) {
                auto succ = successor(nd, i);
                nd.keys[i] = succ.first;
                nd.values[i] = succ.second;
                return node_delete(*nd.children[i + 1], succ.first, t);
            }
            merge_children(nd, i);
            return node_delete(*nd.children[i], key, t);
        }
        if (nd.is_leaf) {
            return false;
        }
        std::size_t ni = ensure_child_has_t_keys(nd, fp.second, t);
        return node_delete(*nd.children[ni], key, t);
    }

    static void node_inorder(const node &nd,
                             std::vector<std::pair<K, V>> &out) {
        if (nd.is_leaf) {
            for (std::size_t i = 0; i < nd.keys.size(); i++) {
                out.emplace_back(nd.keys[i], nd.values[i]);
            }
        } else {
            for (std::size_t i = 0; i < nd.keys.size(); i++) {
                node_inorder(*nd.children[i], out);
                out.emplace_back(nd.keys[i], nd.values[i]);
            }
            node_inorder(*nd.children[nd.keys.size()], out);
        }
    }

    static void node_range(const node &nd, const K &low, const K &high,
                           std::vector<std::pair<K, V>> &out) {
        if (nd.is_leaf) {
            for (std::size_t i = 0; i < nd.keys.size(); i++) {
                if (!(nd.keys[i] < low) && !(high < nd.keys[i])) {
                    out.emplace_back(nd.keys[i], nd.values[i]);
                }
            }
            return;
        }
        for (std::size_t i = 0; i < nd.keys.size(); i++) {
            if (low < nd.keys[i]) {
                node_range(*nd.children[i], low, high, out);
            }
            if (!(nd.keys[i] < low) && !(high < nd.keys[i])) {
                out.emplace_back(nd.keys[i], nd.values[i]);
            }
            if (!(nd.keys[i] < high)) {
                return;
            }
        }
        node_range(*nd.children[nd.keys.size()], low, high, out);
    }

    static bool validate(const node &nd, std::size_t t, bool is_root,
                         std::size_t depth, std::size_t &out_leaf_depth) {
        std::size_t min_keys = is_root ? 1 : t - 1;
        std::size_t max_keys = 2 * t - 1;
        if (!nd.keys.empty() &&
            (nd.keys.size() < min_keys || nd.keys.size() > max_keys)) {
            out_leaf_depth = depth;
            return false;
        }
        for (std::size_t i = 1; i < nd.keys.size(); i++) {
            if (!(nd.keys[i - 1] < nd.keys[i])) {
                out_leaf_depth = depth;
                return false;
            }
        }
        if (nd.values.size() != nd.keys.size()) {
            out_leaf_depth = depth;
            return false;
        }
        if (nd.is_leaf) {
            if (!nd.children.empty()) {
                out_leaf_depth = depth;
                return false;
            }
            out_leaf_depth = depth;
            return true;
        }
        if (nd.children.size() != nd.keys.size() + 1) {
            out_leaf_depth = depth;
            return false;
        }
        bool have = false;
        std::size_t ld = depth;
        for (const auto &child : nd.children) {
            std::size_t cd = 0;
            if (!validate(*child, t, false, depth + 1, cd)) {
                out_leaf_depth = depth;
                return false;
            }
            if (!have) {
                have = true;
                ld = cd;
            } else if (ld != cd) {
                out_leaf_depth = depth;
                return false;
            }
        }
        out_leaf_depth = ld;
        return true;
    }
};

} // namespace ca

#endif // B_TREE_HPP
