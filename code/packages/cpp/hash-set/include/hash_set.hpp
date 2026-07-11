// hash_set.hpp — a hash set, in pure ISO C++17 (header-only). A faithful port of
// the Rust `hash-set` crate (DT19), in namespace `ca`.
// ===========================================================================
//
// Exactly as in the Rust crate — "a zero-cost wrapper around the DT18 hash map:
// HashSet<T> is stored as HashMap<T, ()>" — `ca::hash_set<T>` is a thin layer
// over `ca::hash_map<T, unit>` from the sibling `hash-map` package. Membership,
// the full set algebra (union / intersection / difference / symmetric
// difference), and the relations (subset / superset / disjoint / equals) are all
// built on the map's enumeration.
//
// Element type `T` follows the map's key rules: `std::string` or any
// trivially-copyable type.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef HASH_SET_HPP
#define HASH_SET_HPP

#include <cstddef>
#include <vector>

#include "hash_map.hpp"

namespace ca {

template <class T>
class hash_set {
public:
    explicit hash_set(
        std::size_t capacity = 16,
        collision_strategy strategy = collision_strategy::chaining,
        hash_algorithm hash = hash_algorithm::siphash24)
        : map_(capacity, strategy, hash) {}

    // Insert `element` (a no-op if already present).
    void add(const T &element) { map_.set(element, unit{}); }

    // Remove `element`; returns true if it was present.
    bool remove(const T &element) { return map_.remove(element); }

    bool contains(const T &element) const { return map_.has(element); }
    std::size_t size() const { return map_.size(); }
    bool empty() const { return map_.empty(); }

    // The elements, in unspecified order.
    std::vector<T> to_vector() const { return map_.keys(); }

    // ── set algebra (each returns a fresh set) ─────────────────────────────
    hash_set union_with(const hash_set &other) const {
        hash_set result = fresh();
        for (const T &e : to_vector()) {
            result.add(e);
        }
        for (const T &e : other.to_vector()) {
            result.add(e);
        }
        return result;
    }

    hash_set intersection(const hash_set &other) const {
        const hash_set &smaller = size() <= other.size() ? *this : other;
        const hash_set &larger = size() <= other.size() ? other : *this;
        hash_set result = fresh();
        for (const T &e : smaller.to_vector()) {
            if (larger.contains(e)) {
                result.add(e);
            }
        }
        return result;
    }

    hash_set difference(const hash_set &other) const {
        hash_set result = fresh();
        for (const T &e : to_vector()) {
            if (!other.contains(e)) {
                result.add(e);
            }
        }
        return result;
    }

    hash_set symmetric_difference(const hash_set &other) const {
        hash_set result = fresh();
        for (const T &e : to_vector()) {
            if (!other.contains(e)) {
                result.add(e);
            }
        }
        for (const T &e : other.to_vector()) {
            if (!contains(e)) {
                result.add(e);
            }
        }
        return result;
    }

    // ── relations ──────────────────────────────────────────────────────────
    bool is_subset(const hash_set &other) const {
        if (size() > other.size()) {
            return false;
        }
        for (const T &e : to_vector()) {
            if (!other.contains(e)) {
                return false;
            }
        }
        return true;
    }

    bool is_superset(const hash_set &other) const {
        return other.is_subset(*this);
    }

    bool is_disjoint(const hash_set &other) const {
        const hash_set &smaller = size() <= other.size() ? *this : other;
        const hash_set &larger = size() <= other.size() ? other : *this;
        for (const T &e : smaller.to_vector()) {
            if (larger.contains(e)) {
                return false;
            }
        }
        return true;
    }

    bool equals(const hash_set &other) const {
        return size() == other.size() && is_subset(other);
    }

private:
    // The unit value type: an empty, trivially-copyable stand-in for Rust's ().
    struct unit {};

    // A new empty set that inherits this set's map configuration.
    hash_set fresh() const {
        return hash_set(1, map_.strategy(), map_.hash_function());
    }

    hash_map<T, unit> map_;
};

} // namespace ca

#endif // HASH_SET_HPP
