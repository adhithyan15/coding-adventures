// fenwick_tree.hpp — a Fenwick tree (Binary Indexed Tree) over doubles, in pure
// ISO C++17 (header-only). A faithful port of the Rust `fenwick-tree` crate.
// ===========================================================================
//
// A Fenwick tree answers update(i, delta) and prefix_sum(i) in O(log n), from
// which range sums, point queries, and cumulative-frequency search follow. Each
// slot bit[i] holds the sum of a run of length lowbit(i) = (i & -i) ending at i.
//
// Indexing is 1-based (valid element indices 1..=n), matching the crate;
// prefix_sum also accepts 0 (the empty prefix). Out-of-range indices, inverted
// ranges, and bad find_kth targets throw std::out_of_range / std::invalid_argument
// — the idiomatic C++ analogue of the crate's Result errors.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef FENWICK_TREE_HPP
#define FENWICK_TREE_HPP

#include <cstddef>
#include <stdexcept>
#include <vector>

namespace ca {

class fenwick_tree {
public:
    // Construct an all-zero tree of `n` elements.
    explicit fenwick_tree(std::size_t n) : n_(n), bit_(n + 1, 0.0) {}

    // Construct from initial element values (1..=size()).
    explicit fenwick_tree(const std::vector<double> &values)
        : n_(values.size()), bit_(values.size() + 1, 0.0) {
        for (std::size_t index = 1; index <= n_; index++) {
            std::size_t parent = index + lowbit(index);
            bit_[index] += values[index - 1];
            if (parent <= n_) {
                bit_[parent] += bit_[index];
            }
        }
    }

    std::size_t size() const { return n_; }
    bool empty() const { return n_ == 0; }

    // Add `delta` to element `index` (1..=n).
    void update(std::size_t index, double delta) {
        if (index < 1 || index > n_) {
            throw std::out_of_range("fenwick_tree::update index out of range");
        }
        for (std::size_t current = index; current <= n_;
             current += lowbit(current)) {
            bit_[current] += delta;
        }
    }

    // Sum of elements 1..=index (index may be 0 for the empty prefix).
    double prefix_sum(std::size_t index) const {
        if (index > n_) {
            throw std::out_of_range("fenwick_tree::prefix_sum index out of range");
        }
        double total = 0.0;
        for (std::size_t current = index; current > 0;
             current -= lowbit(current)) {
            total += bit_[current];
        }
        return total;
    }

    // Sum of elements left..=right (both 1..=n).
    double range_sum(std::size_t left, std::size_t right) const {
        if (left > right) {
            throw std::invalid_argument("fenwick_tree::range_sum left > right");
        }
        check_index(left);
        check_index(right);
        if (left == 1) {
            return prefix_sum(right);
        }
        return prefix_sum(right) - prefix_sum(left - 1);
    }

    // Value of element `index` (1..=n).
    double point_query(std::size_t index) const {
        check_index(index);
        return range_sum(index, index);
    }

    // Smallest 1-based index whose prefix sum is >= `target` (assumes all
    // elements are non-negative). Throws on empty tree, non-positive target, or
    // a target exceeding the total.
    std::size_t find_kth(double target) const {
        if (n_ == 0) {
            throw std::out_of_range("fenwick_tree::find_kth on empty tree");
        }
        if (target <= 0.0) {
            throw std::invalid_argument("fenwick_tree::find_kth target <= 0");
        }
        double total = prefix_sum(n_);
        if (target > total) {
            throw std::invalid_argument("fenwick_tree::find_kth target > total");
        }
        std::size_t index = 0;
        for (std::size_t step = highest_power_of_two_at_most(n_); step > 0;
             step >>= 1) {
            std::size_t next = index + step;
            if (next <= n_ && bit_[next] < target) {
                index = next;
                target -= bit_[index];
            }
        }
        return index + 1;
    }

private:
    std::size_t n_;
    std::vector<double> bit_; // length n+1, 1-indexed (bit_[0] unused)

    static std::size_t lowbit(std::size_t i) { return i & (~i + 1u); }

    static std::size_t highest_power_of_two_at_most(std::size_t n) {
        if (n == 0) {
            return 0;
        }
        std::size_t p = 1;
        while (p <= n / 2) {
            p *= 2;
        }
        return p;
    }

    void check_index(std::size_t index) const {
        if (index < 1 || index > n_) {
            throw std::out_of_range("fenwick_tree index out of range");
        }
    }
};

} // namespace ca

#endif // FENWICK_TREE_HPP
