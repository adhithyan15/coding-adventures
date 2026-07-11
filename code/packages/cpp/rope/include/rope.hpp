// rope.hpp — a rope (a balanced binary tree of string chunks), in pure ISO
// C++17 (header-only). A faithful port of the Rust `rope` crate (DT16), in
// namespace `ca`.
// ===========================================================================
//
// A rope stores a long string as a binary tree whose leaves hold chunks and
// whose internal nodes carry a `weight` (the length of the left subtree),
// making concatenation O(1) and indexing/splitting cheap.
//
// Where the C port uses a consuming API to mirror Rust's move semantics, this
// header uses value semantics with structural sharing: nodes are immutable and
// held by std::shared_ptr, so a `ca::rope` is cheap to copy and every operation
// returns a new rope while reusing untouched subtrees.
//
// The crate counts Unicode scalar values; this port works on bytes (like
// std::string), so results match for ASCII / single-byte text. Offsets are byte
// offsets.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef ROPE_HPP
#define ROPE_HPP

#include <algorithm>
#include <cstddef>
#include <memory>
#include <optional>
#include <string>
#include <utility>

namespace ca {

class rope {
public:
    rope() = default; // empty

    static rope from_string(std::string s) {
        if (s.empty()) {
            return rope();
        }
        std::size_t n = s.size();
        return rope(make_leaf(std::move(s)), n);
    }

    std::size_t len() const { return len_; }
    bool empty() const { return len_ == 0; }

    std::string to_string() const {
        std::string out;
        collect(root_, out);
        return out;
    }

    // O(1) concatenation, sharing both operands' subtrees.
    static rope concat(const rope &left, const rope &right) {
        if (!left.root_) {
            return right;
        }
        if (!right.root_) {
            return left;
        }
        return rope(make_internal(left.len_, left.root_, right.root_),
                    left.len_ + right.len_);
    }

    // Split at byte offset i into (bytes 0..i, bytes i..end); i is clamped.
    std::pair<rope, rope> split(std::size_t i) const {
        std::string text = to_string();
        std::size_t at = std::min(i, text.size());
        return {from_string(text.substr(0, at)), from_string(text.substr(at))};
    }

    rope insert(std::size_t i, const std::string &s) const {
        auto parts = split(i);
        return concat(concat(parts.first, from_string(s)), parts.second);
    }

    // `delete` is a keyword, so the removal operation is spelled `erase`.
    rope erase(std::size_t start, std::size_t length) const {
        std::string text = to_string();
        std::size_t s = std::min(start, text.size());
        std::size_t end =
            (length > text.size() - s) ? text.size() : s + length;
        return concat(from_string(text.substr(0, s)),
                      from_string(text.substr(end)));
    }

    rope rebalance() const {
        std::string text = to_string();
        return build_balanced(text.data(), text.size());
    }

    // The byte at offset i, or std::nullopt if out of range (weighted descent).
    std::optional<char> index(std::size_t i) const {
        if (i >= len_) {
            return std::nullopt;
        }
        const node *n = root_.get();
        while (n != nullptr) {
            if (n->leaf) {
                return n->chunk[i];
            }
            if (i < n->weight) {
                n = n->left.get();
            } else {
                i -= n->weight;
                n = n->right.get();
            }
        }
        return std::nullopt;
    }

    std::string substring(std::size_t start, std::size_t end) const {
        std::string text = to_string();
        std::size_t s = std::min(start, text.size());
        std::size_t e = std::min(end, text.size());
        if (s >= e) {
            return std::string();
        }
        return text.substr(s, e - s);
    }

    std::size_t depth() const { return node_depth(root_); }
    bool is_balanced() const { return node_balanced(root_); }

private:
    struct node {
        bool leaf = false;
        std::string chunk;      // leaf content
        std::size_t weight = 0; // internal: bytes in the left subtree
        std::shared_ptr<const node> left;
        std::shared_ptr<const node> right;
    };

    std::shared_ptr<const node> root_; // null when empty
    std::size_t len_ = 0;

    rope(std::shared_ptr<const node> root, std::size_t len)
        : root_(std::move(root)), len_(len) {}

    static std::shared_ptr<const node> make_leaf(std::string chunk) {
        auto n = std::make_shared<node>();
        n->leaf = true;
        n->chunk = std::move(chunk);
        return n;
    }
    static std::shared_ptr<const node> make_internal(
        std::size_t weight, std::shared_ptr<const node> left,
        std::shared_ptr<const node> right) {
        auto n = std::make_shared<node>();
        n->leaf = false;
        n->weight = weight;
        n->left = std::move(left);
        n->right = std::move(right);
        return n;
    }

    static void collect(const std::shared_ptr<const node> &n, std::string &out) {
        if (!n) {
            return;
        }
        if (n->leaf) {
            out += n->chunk;
        } else {
            collect(n->left, out);
            collect(n->right, out);
        }
    }

    static std::size_t node_depth(const std::shared_ptr<const node> &n) {
        if (!n || n->leaf) {
            return 0;
        }
        return 1 + std::max(node_depth(n->left), node_depth(n->right));
    }
    static bool node_balanced(const std::shared_ptr<const node> &n) {
        if (!n || n->leaf) {
            return true;
        }
        std::size_t ld = node_depth(n->left);
        std::size_t rd = node_depth(n->right);
        std::size_t diff = ld > rd ? ld - rd : rd - ld;
        return diff <= 1 && node_balanced(n->left) && node_balanced(n->right);
    }

    static rope build_balanced(const char *buf, std::size_t n) {
        if (n == 0) {
            return rope();
        }
        if (n <= 1) {
            return from_string(std::string(buf, n));
        }
        std::size_t mid = n / 2;
        return concat(build_balanced(buf, mid),
                      build_balanced(buf + mid, n - mid));
    }
};

} // namespace ca

#endif // ROPE_HPP
