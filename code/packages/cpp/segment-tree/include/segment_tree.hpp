// segment_tree.hpp — a segment tree with a caller-supplied associative combine
// operation, in pure ISO C++17 (header-only). A faithful port of the Rust
// `segment-tree` crate.
// ===========================================================================
//
// Answers "combine all elements in [left, right]" and point updates in
// O(log n). The combine op is any associative binary function with an identity
// element (sum/min/max/gcd/…); pass it to the constructor, or use the sum_tree /
// min_tree / max_tree factories. Ranges are INCLUSIVE and 0-based.
//
// Internally a 1-indexed std::vector of up to 4n nodes: node k covers a segment,
// children 2k / 2k+1 cover the halves.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef SEGMENT_TREE_HPP
#define SEGMENT_TREE_HPP

#include <algorithm>
#include <cstddef>
#include <functional>
#include <limits>
#include <vector>

namespace ca {

template <typename T> class segment_tree {
public:
    using combine_fn = std::function<T(const T &, const T &)>;

    segment_tree(std::vector<T> values, combine_fn combine, T identity)
        : n_(values.size()), combine_(std::move(combine)),
          identity_(std::move(identity)) {
        if (n_ == 0) {
            tree_.assign(1, identity_);
            return;
        }
        tree_.assign(4 * n_ + 4, identity_);
        build(values, 1, 0, n_ - 1);
    }

    // Factories for the three common operations (require an arithmetic T).
    static segment_tree sum_tree(std::vector<T> values) {
        return segment_tree(std::move(values),
                            [](const T &a, const T &b) { return a + b; }, T{});
    }
    static segment_tree min_tree(std::vector<T> values) {
        return segment_tree(std::move(values),
                            [](const T &a, const T &b) { return std::min(a, b); },
                            std::numeric_limits<T>::max());
    }
    static segment_tree max_tree(std::vector<T> values) {
        return segment_tree(std::move(values),
                            [](const T &a, const T &b) { return std::max(a, b); },
                            std::numeric_limits<T>::lowest());
    }

    std::size_t size() const { return n_; }
    bool empty() const { return n_ == 0; }

    // Combine over the inclusive range [left, right]. Returns the identity for an
    // empty tree or an out-of-range / inverted range (never reads out of bounds).
    T query(std::size_t left, std::size_t right) const {
        if (n_ == 0 || left > right || right >= n_) {
            return identity_;
        }
        return query(1, 0, n_ - 1, left, right);
    }

    // Set element `index` to `value`. Out-of-range indices are ignored.
    void update(std::size_t index, const T &value) {
        if (n_ == 0 || index >= n_) {
            return;
        }
        update(1, 0, n_ - 1, index, value);
    }

private:
    std::vector<T> tree_;
    std::size_t n_;
    combine_fn combine_;
    T identity_;

    void build(const std::vector<T> &values, std::size_t node, std::size_t left,
               std::size_t right) {
        if (left == right) {
            tree_[node] = values[left];
            return;
        }
        std::size_t mid = (left + right) / 2;
        build(values, node * 2, left, mid);
        build(values, node * 2 + 1, mid + 1, right);
        tree_[node] = combine_(tree_[node * 2], tree_[node * 2 + 1]);
    }

    T query(std::size_t node, std::size_t left, std::size_t right,
            std::size_t ql, std::size_t qr) const {
        if (right < ql || left > qr) {
            return identity_;
        }
        if (ql <= left && right <= qr) {
            return tree_[node];
        }
        std::size_t mid = (left + right) / 2;
        T l = query(node * 2, left, mid, ql, qr);
        T r = query(node * 2 + 1, mid + 1, right, ql, qr);
        return combine_(l, r);
    }

    void update(std::size_t node, std::size_t left, std::size_t right,
                std::size_t index, const T &value) {
        if (left == right) {
            tree_[node] = value;
            return;
        }
        std::size_t mid = (left + right) / 2;
        if (index <= mid) {
            update(node * 2, left, mid, index, value);
        } else {
            update(node * 2 + 1, mid + 1, right, index, value);
        }
        tree_[node] = combine_(tree_[node * 2], tree_[node * 2 + 1]);
    }
};

} // namespace ca

#endif // SEGMENT_TREE_HPP
