// radix_tree.hpp — a radix tree (compressed trie / Patricia trie) for
// string-keyed prefix search, in pure ISO C++17 (header-only). A faithful port
// of the Rust `radix-tree` crate, in namespace `ca`.
// ===========================================================================
//
// A radix tree compresses each chain of single-child trie nodes into one edge
// labelled with the shared substring, keeping the tree small while supporting
// fast prefix queries. Each node may mark the end of a key (with a value) and
// holds edges to children, kept in a std::map keyed by the first byte of the
// edge label so traversals emit keys in sorted order.
//
// Unlike the C sibling (specialised to a `long` value), this header is generic:
// `ca::radix_tree<V>` stores any value type.
//
//   insert / search / remove              — the map operations (remove: `delete`
//                                            is a keyword)
//   starts_with / longest_prefix_match    — prefix queries
//   keys / words_with_prefix              — sorted key enumeration
//   len / node_count                       — introspection
//
// Byte-oriented (the crate splits on Unicode chars); identical to the crate for
// ASCII keys, and a correct prefix map for any byte string.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef RADIX_TREE_HPP
#define RADIX_TREE_HPP

#include <algorithm>
#include <cstddef>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace ca {

template <class V>
class radix_tree {
public:
    radix_tree() : root_(std::make_unique<node>()) {}

    // Insert or update key -> value.
    void insert(std::string_view key, const V &value) {
        if (insert_recursive(*root_, key, value)) {
            size_++;
        }
    }

    // Look up key; returns a pointer to the value, or nullptr if absent.
    const V *search(std::string_view key) const {
        const node *n = descend(root_.get(), key);
        if (n != nullptr && n->is_end) {
            return &(*n->value);
        }
        return nullptr;
    }
    bool contains(std::string_view key) const { return search(key) != nullptr; }

    // Remove key, pruning dead nodes and merging single-child chains. `delete`
    // is a keyword, so the operation is spelled `remove`.
    bool remove(std::string_view key) {
        bool mergeable = false;
        bool deleted = delete_recursive(*root_, key, mergeable);
        if (deleted) {
            size_--;
        }
        return deleted;
    }

    bool starts_with(std::string_view prefix) const {
        if (prefix.empty()) {
            return size_ > 0;
        }
        const node *node = root_.get();
        while (!prefix.empty()) {
            auto it = node->children.find(static_cast<unsigned char>(prefix[0]));
            if (it == node->children.end()) {
                return false;
            }
            const edge &e = it->second;
            std::size_t common = common_prefix_len(prefix, e.label);
            if (common == prefix.size()) {
                return true;
            }
            if (common < e.label.size()) {
                return false;
            }
            prefix = prefix.substr(common);
            node = e.child.get();
        }
        return node->is_end || !node->children.empty();
    }

    std::optional<std::string> longest_prefix_match(std::string_view key) const {
        const node *node = root_.get();
        std::size_t consumed = 0;
        std::optional<std::string> best =
            node->is_end ? std::optional<std::string>(std::string())
                         : std::nullopt;
        std::string_view rest = key;
        while (!rest.empty()) {
            auto it = node->children.find(static_cast<unsigned char>(rest[0]));
            if (it == node->children.end()) {
                break;
            }
            const edge &e = it->second;
            std::size_t common = common_prefix_len(rest, e.label);
            if (common < e.label.size()) {
                break;
            }
            consumed += common;
            rest = rest.substr(common);
            node = e.child.get();
            if (node->is_end) {
                best = std::string(key.substr(0, consumed));
            }
        }
        return best;
    }

    std::vector<std::string> keys() const {
        std::vector<std::string> out;
        std::string path;
        collect(*root_, path, out);
        return out;
    }

    std::vector<std::string> words_with_prefix(std::string_view prefix) const {
        std::vector<std::string> out;
        std::string path;
        const node *node = root_.get();
        std::string_view rest = prefix;
        if (rest.empty()) {
            collect(*node, path, out);
            return out;
        }
        while (!rest.empty()) {
            auto it = node->children.find(static_cast<unsigned char>(rest[0]));
            if (it == node->children.end()) {
                return out;
            }
            const edge &e = it->second;
            std::size_t common = common_prefix_len(rest, e.label);
            if (common == rest.size()) {
                if (common == e.label.size()) {
                    path += e.label;
                    node = e.child.get();
                    rest = std::string_view();
                } else {
                    path += e.label;
                    collect(*e.child, path, out);
                    return out;
                }
            } else if (common < e.label.size()) {
                return out;
            } else {
                path += e.label;
                rest = rest.substr(common);
                node = e.child.get();
            }
        }
        collect(*node, path, out);
        return out;
    }

    std::size_t len() const { return size_; }
    bool empty() const { return size_ == 0; }
    std::size_t node_count() const { return count_nodes(*root_); }

private:
    struct node;
    struct edge {
        std::string label;
        std::unique_ptr<node> child;
    };
    struct node {
        bool is_end = false;
        std::optional<V> value;
        std::map<unsigned char, edge> children; // keyed by first byte, sorted
    };

    std::unique_ptr<node> root_;
    std::size_t size_ = 0;

    static std::size_t common_prefix_len(std::string_view a,
                                         std::string_view b) {
        std::size_t n = std::min(a.size(), b.size());
        std::size_t i = 0;
        while (i < n && a[i] == b[i]) {
            i++;
        }
        return i;
    }

    static const node *descend(const node *nd, std::string_view key) {
        while (!key.empty()) {
            auto it = nd->children.find(static_cast<unsigned char>(key[0]));
            if (it == nd->children.end()) {
                return nullptr;
            }
            const edge &e = it->second;
            std::size_t common = common_prefix_len(key, e.label);
            if (common < e.label.size()) {
                return nullptr;
            }
            key = key.substr(common);
            nd = e.child.get();
        }
        return nd;
    }

    static bool insert_recursive(node &nd, std::string_view key,
                                 const V &value) {
        if (key.empty()) {
            bool added = !nd.is_end;
            nd.is_end = true;
            nd.value = value;
            return added;
        }
        unsigned char first = static_cast<unsigned char>(key[0]);
        auto it = nd.children.find(first);
        if (it == nd.children.end()) {
            edge e;
            e.label = std::string(key);
            e.child = std::make_unique<node>();
            e.child->is_end = true;
            e.child->value = value;
            nd.children.emplace(first, std::move(e));
            return true;
        }
        edge &edge_ref = it->second;
        std::size_t common = common_prefix_len(key, edge_ref.label);
        if (common == edge_ref.label.size()) {
            return insert_recursive(*edge_ref.child, key.substr(common), value);
        }
        // Split the edge at `common`.
        std::string label_rest = edge_ref.label.substr(common);
        std::string_view key_rest = key.substr(common);
        auto split_node = std::make_unique<node>();
        {
            edge sub;
            sub.label = std::move(label_rest);
            sub.child = std::move(edge_ref.child);
            unsigned char sub_first = static_cast<unsigned char>(sub.label[0]);
            split_node->children.emplace(sub_first, std::move(sub));
        }
        if (key_rest.empty()) {
            split_node->is_end = true;
            split_node->value = value;
        } else {
            edge leaf_e;
            leaf_e.label = std::string(key_rest);
            leaf_e.child = std::make_unique<node>();
            leaf_e.child->is_end = true;
            leaf_e.child->value = value;
            unsigned char lf = static_cast<unsigned char>(leaf_e.label[0]);
            split_node->children.emplace(lf, std::move(leaf_e));
        }
        edge_ref.label = edge_ref.label.substr(0, common);
        edge_ref.child = std::move(split_node);
        return true;
    }

    static bool delete_recursive(node &nd, std::string_view key,
                                 bool &mergeable) {
        if (key.empty()) {
            if (!nd.is_end) {
                mergeable = false;
                return false;
            }
            nd.is_end = false;
            nd.value.reset();
            mergeable = nd.children.size() == 1;
            return true;
        }
        unsigned char first = static_cast<unsigned char>(key[0]);
        auto it = nd.children.find(first);
        if (it == nd.children.end()) {
            mergeable = false;
            return false;
        }
        edge &edge_ref = it->second;
        std::size_t common = common_prefix_len(key, edge_ref.label);
        if (common < edge_ref.label.size()) {
            mergeable = false;
            return false;
        }
        bool child_mergeable = false;
        bool deleted = delete_recursive(*edge_ref.child, key.substr(common),
                                        child_mergeable);
        if (!deleted) {
            mergeable = false;
            return false;
        }
        if (child_mergeable) {
            // The child now has exactly one edge → fold it up.
            node &child = *edge_ref.child;
            edge &grand = child.children.begin()->second;
            edge_ref.label += grand.label;
            std::unique_ptr<node> grandchild = std::move(grand.child);
            edge_ref.child = std::move(grandchild);
        } else if (!edge_ref.child->is_end && edge_ref.child->children.empty()) {
            nd.children.erase(it); // prune the dead child
        }
        mergeable = !nd.is_end && nd.children.size() == 1;
        return true;
    }

    static void collect(const node &nd, std::string &path,
                        std::vector<std::string> &out) {
        if (nd.is_end) {
            out.push_back(path);
        }
        for (const auto &kv : nd.children) {
            std::size_t old = path.size();
            path += kv.second.label;
            collect(*kv.second.child, path, out);
            path.resize(old);
        }
    }

    static std::size_t count_nodes(const node &nd) {
        std::size_t total = 1;
        for (const auto &kv : nd.children) {
            total += count_nodes(*kv.second.child);
        }
        return total;
    }
};

} // namespace ca

#endif // RADIX_TREE_HPP
