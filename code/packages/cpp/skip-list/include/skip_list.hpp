// skip_list.hpp — an ordered map with skip-list-style reported parameters, in
// pure ISO C++17 (header-only). A faithful port of the Rust `skip-list` crate.
// ===========================================================================
//
// Like the crate, this is internally an ordered map (std::map) that REPORTS
// skip-list parameters (max_level, probability, a derived current_max height).
// It offers insert/erase/search, order statistics (rank, by_rank), min/max,
// ordered iteration, and range queries. current_max is ceil(log_{1/p}(n))
// clamped to [1, max_level], computed without <cmath> so nothing extra links.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef SKIP_LIST_HPP
#define SKIP_LIST_HPP

#include <cstddef>
#include <map>
#include <optional>
#include <utility>
#include <vector>

namespace ca {

template <typename K, typename V> class skip_list {
public:
    skip_list() : skip_list(32, 0.5) {}

    skip_list(std::size_t max_level, double probability)
        : max_level_(max_level < 1 ? 1 : max_level),
          probability_((probability > 0.0 && probability < 1.0) ? probability : 0.5),
          current_max_(1) {}

    // Insert or overwrite key → value.
    void insert(const K &key, const V &value) {
        entries_[key] = value;
        current_max_ = estimated_current_max();
    }

    // Remove key; returns true iff it was present.
    bool erase(const K &key) {
        bool removed = entries_.erase(key) > 0;
        if (removed) {
            current_max_ = estimated_current_max();
        }
        return removed;
    }

    std::optional<V> search(const K &key) const {
        auto it = entries_.find(key);
        if (it == entries_.end()) {
            return std::nullopt;
        }
        return it->second;
    }

    bool contains(const K &key) const {
        return entries_.find(key) != entries_.end();
    }

    // 0-based position of key in ascending order, or nullopt if absent.
    std::optional<std::size_t> rank(const K &key) const {
        std::size_t i = 0;
        for (const auto &kv : entries_) {
            if (kv.first == key) {
                return i;
            }
            i++;
        }
        return std::nullopt;
    }

    // Key at 0-based position `r`, or nullopt if r >= size().
    std::optional<K> by_rank(std::size_t r) const {
        if (r >= entries_.size()) {
            return std::nullopt;
        }
        auto it = entries_.begin();
        std::advance(it, static_cast<std::ptrdiff_t>(r));
        return it->first;
    }

    std::optional<K> min() const {
        if (entries_.empty()) {
            return std::nullopt;
        }
        return entries_.begin()->first;
    }
    std::optional<K> max() const {
        if (entries_.empty()) {
            return std::nullopt;
        }
        return entries_.rbegin()->first;
    }

    // Entries with key in [lo, hi] (inclusive) or (lo, hi) (exclusive), sorted.
    std::vector<std::pair<K, V>> range(const K &lo, const K &hi,
                                       bool inclusive) const {
        std::vector<std::pair<K, V>> out;
        if (lo > hi) {
            return out;
        }
        auto lower = inclusive ? entries_.lower_bound(lo) : entries_.upper_bound(lo);
        for (auto it = lower; it != entries_.end(); ++it) {
            if (inclusive ? (it->first > hi) : !(it->first < hi)) {
                break;
            }
            out.emplace_back(it->first, it->second);
        }
        return out;
    }

    // All entries in ascending key order.
    std::vector<std::pair<K, V>> entries() const {
        return std::vector<std::pair<K, V>>(entries_.begin(), entries_.end());
    }

    std::size_t size() const { return entries_.size(); }
    bool empty() const { return entries_.empty(); }
    std::size_t max_level() const { return max_level_; }
    std::size_t current_max() const { return current_max_; }
    double probability() const { return probability_; }

private:
    std::map<K, V> entries_;
    std::size_t max_level_;
    double probability_;
    std::size_t current_max_;

    // ceil(log_{1/p}(n)) clamped to [1, max_level], without <cmath>: smallest L
    // with (1/p)^L >= n.
    std::size_t estimated_current_max() const {
        if (entries_.empty()) {
            return 1;
        }
        double base = 1.0 / probability_;
        double acc = 1.0;
        std::size_t levels = 0;
        while (acc < static_cast<double>(entries_.size())) {
            acc *= base;
            levels++;
        }
        if (levels < 1) {
            levels = 1;
        }
        if (levels > max_level_) {
            levels = max_level_;
        }
        return levels;
    }
};

} // namespace ca

#endif // SKIP_LIST_HPP
