// trie.hpp — a trie (prefix tree) mapping string keys to values, in pure ISO
// C++17 (header-only). A faithful port of the Rust `trie` crate.
// ===========================================================================
//
// Keys share the path for any prefix they have in common, so lookups and prefix
// queries are O(key length) and enumeration yields keys in sorted order (the
// children live in a std::map, which iterates in ascending key order — matching
// the crate's BTreeMap).
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef TRIE_HPP
#define TRIE_HPP

#include <cstddef>
#include <map>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace ca {

template <typename V> class trie {
public:
    trie() = default;

    std::size_t size() const { return size_; }
    bool empty() const { return size_ == 0; }

    // Associate `value` with `key`, overwriting any previous value.
    void insert(const std::string &key, V value) {
        node *n = &root_;
        for (char ch : key) {
            n = &n->children[ch]; // creates the child if absent
        }
        if (!n->is_end) {
            size_++;
        }
        n->is_end = true;
        n->value = std::move(value);
    }

    // The value stored at `key`, or std::nullopt if `key` is not present.
    std::optional<V> search(const std::string &key) const {
        const node *n = find_node(key);
        if (n != nullptr && n->is_end) {
            return n->value;
        }
        return std::nullopt;
    }

    bool contains_key(const std::string &key) const {
        const node *n = find_node(key);
        return n != nullptr && n->is_end;
    }

    // Remove `key`, pruning now-unused nodes. Returns true iff it was present.
    bool erase(const std::string &key) {
        if (!contains_key(key)) {
            return false;
        }
        erase_rec(root_, key, 0);
        size_--;
        return true;
    }

    // True if any stored key begins with `prefix` (empty prefix ⇒ non-empty).
    bool starts_with(const std::string &prefix) const {
        if (prefix.empty()) {
            return size_ > 0;
        }
        return find_node(prefix) != nullptr;
    }

    // All (key, value) pairs whose key begins with `prefix`, in sorted order.
    std::vector<std::pair<std::string, V>>
    words_with_prefix(const std::string &prefix) const {
        std::vector<std::pair<std::string, V>> out;
        const node *n = prefix.empty() ? &root_ : find_node(prefix);
        if (n != nullptr) {
            std::string key = prefix;
            collect(*n, key, out);
        }
        return out;
    }

    std::vector<std::pair<std::string, V>> all_words() const {
        return words_with_prefix("");
    }

    std::vector<std::string> keys() const {
        std::vector<std::string> out;
        for (auto &kv : all_words()) {
            out.push_back(kv.first);
        }
        return out;
    }

    // The longest stored key that is a prefix of `string`, with its value.
    std::optional<std::pair<std::string, V>>
    longest_prefix_match(const std::string &string) const {
        const node *n = &root_;
        std::optional<std::pair<std::string, V>> best;
        if (n->is_end) {
            best = std::make_pair(std::string(), n->value);
        }
        std::string current;
        for (char ch : string) {
            auto it = n->children.find(ch);
            if (it == n->children.end()) {
                break;
            }
            current.push_back(ch);
            n = &it->second;
            if (n->is_end) {
                best = std::make_pair(current, n->value);
            }
        }
        return best;
    }

private:
    struct node {
        std::map<char, node> children;
        bool is_end = false;
        V value{};
    };

    node root_;
    std::size_t size_ = 0;

    const node *find_node(const std::string &key) const {
        const node *n = &root_;
        for (char ch : key) {
            auto it = n->children.find(ch);
            if (it == n->children.end()) {
                return nullptr;
            }
            n = &it->second;
        }
        return n;
    }

    static void collect(const node &n, std::string &key,
                        std::vector<std::pair<std::string, V>> &out) {
        if (n.is_end) {
            out.emplace_back(key, n.value);
        }
        for (const auto &kv : n.children) { // std::map → ascending key order
            key.push_back(kv.first);
            collect(kv.second, key, out);
            key.pop_back();
        }
    }

    // Returns true if the caller should erase its child link to `n` (n became
    // useless: not a key end and no children).
    static bool erase_rec(node &n, const std::string &key, std::size_t depth) {
        if (depth == key.size()) {
            n.is_end = false;
        } else {
            auto it = n.children.find(key[depth]);
            if (it != n.children.end() && erase_rec(it->second, key, depth + 1)) {
                n.children.erase(it);
            }
        }
        return !n.is_end && n.children.empty();
    }
};

} // namespace ca

#endif // TRIE_HPP
