// heap.hpp — a binary heap (priority queue), in pure ISO C++17 (header-only).
// A faithful port of the Rust `heap` crate (MinHeap / MaxHeap + helpers).
// ===========================================================================
//
// Elements live in a std::vector laid out as a complete binary tree (children
// of i at 2i+1 / 2i+2, parent at (i-1)/2). A comparator decides what floats to
// the root: min_heap<T> keeps the smallest on top, max_heap<T> the largest.
// push and pop restore the order in O(log n) by sifting up / down.
//
// Also provides the crate's free functions: heap_sort (ascending),
// nlargest / nsmallest (the n most extreme elements).
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef HEAP_HPP
#define HEAP_HPP

#include <algorithm>
#include <cstddef>
#include <functional>
#include <optional>
#include <vector>

namespace ca {

// binary_heap<T, Compare> — the shared engine. `higher(a, b)` returns true when
// `a` should sit above `b`. min_heap/max_heap below pick the comparator.
template <typename T, typename Compare> class binary_heap {
public:
    binary_heap() = default;

    explicit binary_heap(std::vector<T> items) : data_(std::move(items)) {
        // Heapify bottom-up in O(n): sift down every internal node.
        for (std::size_t i = data_.size() / 2; i-- > 0;) {
            sift_down(i);
        }
    }

    std::size_t size() const { return data_.size(); }
    bool empty() const { return data_.empty(); }

    void push(const T &value) {
        data_.push_back(value);
        sift_up(data_.size() - 1);
    }

    // Remove and return the root, or std::nullopt if empty.
    std::optional<T> pop() {
        if (data_.empty()) {
            return std::nullopt;
        }
        T root = std::move(data_.front());
        data_.front() = std::move(data_.back());
        data_.pop_back();
        if (!data_.empty()) {
            sift_down(0);
        }
        return root;
    }

    // Read the root without removing it, or std::nullopt if empty.
    std::optional<T> peek() const {
        if (data_.empty()) {
            return std::nullopt;
        }
        return data_.front();
    }

private:
    std::vector<T> data_;
    Compare higher_{};

    void sift_up(std::size_t index) {
        while (index > 0) {
            std::size_t parent = (index - 1) / 2;
            if (higher_(data_[index], data_[parent])) {
                std::swap(data_[index], data_[parent]);
                index = parent;
            } else {
                break;
            }
        }
    }

    void sift_down(std::size_t index) {
        for (;;) {
            std::size_t left = 2 * index + 1;
            std::size_t right = 2 * index + 2;
            std::size_t best = index;
            if (left < data_.size() && higher_(data_[left], data_[best])) {
                best = left;
            }
            if (right < data_.size() && higher_(data_[right], data_[best])) {
                best = right;
            }
            if (best == index) {
                return;
            }
            std::swap(data_[index], data_[best]);
            index = best;
        }
    }
};

template <typename T> using min_heap = binary_heap<T, std::less<T>>;
template <typename T> using max_heap = binary_heap<T, std::greater<T>>;

// heap_sort — return `items` sorted ascending (drains a min-heap).
template <typename T> std::vector<T> heap_sort(std::vector<T> items) {
    min_heap<T> heap(std::move(items));
    std::vector<T> result;
    while (auto value = heap.pop()) {
        result.push_back(std::move(*value));
    }
    return result;
}

// nlargest — the `n` largest elements of `items`, in descending order.
template <typename T>
std::vector<T> nlargest(std::vector<T> items, std::size_t n) {
    if (n == 0) {
        return {};
    }
    if (n >= items.size()) {
        std::sort(items.begin(), items.end(), std::greater<T>());
        return items;
    }
    std::partial_sort(items.begin(), items.begin() + static_cast<std::ptrdiff_t>(n),
                      items.end(), std::greater<T>());
    items.resize(n);
    return items;
}

// nsmallest — the `n` smallest elements of `items`, in ascending order.
template <typename T>
std::vector<T> nsmallest(std::vector<T> items, std::size_t n) {
    if (n == 0) {
        return {};
    }
    if (n >= items.size()) {
        std::sort(items.begin(), items.end());
        return items;
    }
    std::partial_sort(items.begin(), items.begin() + static_cast<std::ptrdiff_t>(n),
                      items.end());
    items.resize(n);
    return items;
}

} // namespace ca

#endif // HEAP_HPP
