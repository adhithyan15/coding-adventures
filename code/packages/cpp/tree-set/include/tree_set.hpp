// tree_set.hpp — an ordered set built on a balanced-tree backend, in pure ISO
// C++17, header-only, in namespace ca::tree_set. A faithful port of the Rust
// `tree-set` crate.
// ===========================================================================
//
// A set stores each value at most once and keeps its elements sorted. Like the
// Rust crate, `TreeSet<T, Backend>` is *generic over its backend* — any ordered
// balanced tree that provides the small interface below works. The default
// backend is the sibling `ca::avl::AVLTree<T>`; `ca::rb::RBTree<T>` (also in
// this repo) satisfies the same interface, so `TreeSet<int, ca::rb::RBTree<int>>`
// works too.
//
// A backend must provide (all const, value-returning — i.e. persistent):
//   insert(v) erase(v) contains(v) min_value() max_value()
//   predecessor(v) successor(v) kth_smallest(k) to_sorted_array() size()
//
// On top of the backend the set offers the usual algebra — union, intersection,
// difference, symmetric difference — plus subset / superset / disjoint tests and
// range queries, all computed from the operands' sorted sequences (a linear
// merge), exactly as the crate does, so the result is backend-independent.
//
// PERSISTENCE. Following the crate (and the underlying trees), `insert`,
// `remove`, and the algebra operations are `const` and return a NEW set, leaving
// their inputs untouched — the set has value semantics.
//
// NOTE: the union operation is named `union_with` because `union` is a C++
// keyword.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Depends on the sibling `avl-tree`
// package for the default backend (`# build-tool: deps=cpp/avl-tree`).
#ifndef CA_TREE_SET_HPP
#define CA_TREE_SET_HPP

#include <cstddef>
#include <optional>
#include <utility>
#include <vector>

#include "avl_tree.hpp"  // the default ordered-tree backend

namespace ca {
namespace tree_set {

template <class T, class Backend = ca::avl::AVLTree<T>>
class TreeSet {
public:
    TreeSet() = default;
    explicit TreeSet(Backend backend) : backend_(std::move(backend)) {}

    static TreeSet empty() { return TreeSet(); }

    // A set of the distinct values of `values` (duplicates collapse).
    static TreeSet from_list(const std::vector<T>& values) {
        TreeSet s;
        for (const T& v : values) {
            s = s.insert(v);
        }
        return s;
    }

    const Backend& backend() const { return backend_; }

    // ---- membership & order queries -----------------------------------
    std::size_t size() const { return backend_.size(); }
    bool is_empty() const { return size() == 0; }
    bool contains(const T& v) const { return backend_.contains(v); }
    std::optional<T> min_value() const { return backend_.min_value(); }
    std::optional<T> max_value() const { return backend_.max_value(); }
    std::optional<T> first() const { return min_value(); }
    std::optional<T> last() const { return max_value(); }
    std::optional<T> predecessor(const T& v) const {
        return backend_.predecessor(v);
    }
    std::optional<T> successor(const T& v) const {
        return backend_.successor(v);
    }
    std::optional<T> kth_smallest(std::size_t k) const {
        return backend_.kth_smallest(k);
    }
    std::vector<T> to_sorted_array() const { return backend_.to_sorted_array(); }

    // Number of elements strictly less than `v` (the trait's default rank,
    // a partition point over the sorted elements).
    std::size_t rank(const T& v) const {
        std::vector<T> a = backend_.to_sorted_array();
        std::size_t r = 0;
        for (const T& x : a) {
            if (x < v) {
                ++r;
            } else {
                break;
            }
        }
        return r;
    }

    // ---- persistent updates -------------------------------------------
    TreeSet insert(const T& v) const { return TreeSet(backend_.insert(v)); }
    TreeSet remove(const T& v) const { return TreeSet(backend_.erase(v)); }
    TreeSet erase(const T& v) const { return remove(v); }

    // Elements between `min` and `max`, inclusive or strictly between.
    std::vector<T> range(const T& min, const T& max, bool inclusive) const {
        std::vector<T> out;
        if (max < min) {
            return out;
        }
        std::vector<T> a = backend_.to_sorted_array();
        for (const T& v : a) {
            bool in = inclusive ? (!(v < min) && !(max < v))
                                : (min < v && v < max);
            if (in) {
                out.push_back(v);
            }
        }
        return out;
    }

    // ---- set algebra (named union_with; `union` is a keyword) ---------
    TreeSet union_with(const TreeSet& other) const {
        return from_list(
            merge_op(to_sorted_array(), other.to_sorted_array(), Op::Union));
    }
    TreeSet intersection(const TreeSet& other) const {
        return from_list(
            merge_op(to_sorted_array(), other.to_sorted_array(), Op::Intersect));
    }
    TreeSet difference(const TreeSet& other) const {
        return from_list(
            merge_op(to_sorted_array(), other.to_sorted_array(), Op::Diff));
    }
    TreeSet symmetric_difference(const TreeSet& other) const {
        return from_list(
            merge_op(to_sorted_array(), other.to_sorted_array(), Op::SymDiff));
    }

    // ---- set relations ------------------------------------------------
    bool is_subset(const TreeSet& other) const {
        std::vector<T> a = to_sorted_array();
        for (const T& v : a) {
            if (!other.contains(v)) {
                return false;
            }
        }
        return true;
    }
    bool is_superset(const TreeSet& other) const {
        return other.is_subset(*this);
    }
    bool is_disjoint(const TreeSet& other) const {
        std::vector<T> a = to_sorted_array();
        for (const T& v : a) {
            if (other.contains(v)) {
                return false;
            }
        }
        return true;
    }
    bool equals(const TreeSet& other) const {
        return to_sorted_array() == other.to_sorted_array();
    }

private:
    enum class Op { Union, Intersect, Diff, SymDiff };

    // The shared two-pointer merge behind the four algebra operations,
    // mirroring the crate's *_sorted helpers.
    static std::vector<T> merge_op(const std::vector<T>& la,
                                   const std::vector<T>& lb, Op op) {
        std::vector<T> out;
        std::size_t i = 0, j = 0;
        while (i < la.size() && j < lb.size()) {
            if (la[i] < lb[j]) {
                if (op != Op::Intersect) {
                    out.push_back(la[i]);
                }
                ++i;
            } else if (lb[j] < la[i]) {
                if (op == Op::Union || op == Op::SymDiff) {
                    out.push_back(lb[j]);
                }
                ++j;
            } else {  // equal
                if (op == Op::Union || op == Op::Intersect) {
                    out.push_back(la[i]);
                }
                ++i;
                ++j;
            }
        }
        if (op == Op::Union || op == Op::Diff || op == Op::SymDiff) {
            while (i < la.size()) {
                out.push_back(la[i++]);
            }
        }
        if (op == Op::Union || op == Op::SymDiff) {
            while (j < lb.size()) {
                out.push_back(lb[j++]);
            }
        }
        return out;
    }

    Backend backend_;
};

}  // namespace tree_set
}  // namespace ca

#endif  // CA_TREE_SET_HPP
