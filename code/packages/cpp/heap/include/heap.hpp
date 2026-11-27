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
#include <iterator>
#include <limits>
#include <stdexcept>
#include <type_traits>
#include <utility>
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

// A full bounded push is an ordinary result. Range construction uses this
// exception instead, because a constructor cannot return a partial object.
// accepted_count() tells the caller how many values the one-pass source had
// already yielded before the extra value made the requested limit impossible.
class range_capacity_error : public std::length_error {
public:
    explicit range_capacity_error(std::size_t accepted_count)
        : std::length_error("bounded priority queue range exceeds its limit"),
          accepted_count_(accepted_count) {}

    std::size_t accepted_count() const noexcept { return accepted_count_; }

private:
    std::size_t accepted_count_;
};

// A push distinguishes a normal capacity rejection from an inserted value.
// A caller can handle `full` without catching an exception or losing an rvalue.
enum class push_result { inserted, full };

// A bounded queue is a heap with a logical item ceiling. It stores the same
// value that binary_heap stores, but it refuses a new value when the caller's
// limit has been reached. `Higher(a, b)` means that `a` belongs closer to the
// root, matching binary_heap's convention: std::less makes a min-heap and
// std::greater makes a max-heap.
//
// Storage wraps each T in a slot. This matters for vector<bool>: a plain
// vector<bool> returns a proxy object instead of a bool reference, while
// vector<slot<bool>> stores ordinary bool values like every other T.
template <typename T, typename Higher = std::less<T>>
class bounded_priority_queue {
    struct slot {
        T value;
    };

    using storage_type = std::vector<slot>;

    template <typename InputIt, typename = void>
    struct is_input_iterator : std::false_type {};

    template <typename InputIt>
    struct is_input_iterator<
        InputIt,
        typename std::enable_if<
            std::is_base_of<
                std::input_iterator_tag,
                typename std::iterator_traits<InputIt>::iterator_category>::value>::type>
        : std::true_type {};

    static_assert(std::is_nothrow_move_constructible<T>::value,
                  "bounded heap values must be nothrow move-constructible");
    static_assert(std::is_nothrow_move_assignable<T>::value,
                  "bounded heap values must be nothrow move-assignable");
    static_assert(std::is_nothrow_destructible<T>::value,
                  "bounded heap values must be nothrow destructible");
    static_assert(std::is_nothrow_move_constructible<Higher>::value,
                  "bounded heap comparators must be nothrow move-constructible");
    static_assert(std::is_nothrow_destructible<Higher>::value,
                  "bounded heap comparators must be nothrow destructible");
    static_assert(!std::is_member_pointer<Higher>::value,
                  "member pointers are not binary heap comparators");

public:
    template <typename H = Higher,
              typename std::enable_if<std::is_default_constructible<H>::value,
                                      int>::type = 0>
    explicit bounded_priority_queue(std::size_t limit)
        : limit_(checked_limit(limit)), higher_() {
        // Validation happens before the queue reads an iterator or compares a
        // value. It does not reserve memory for the configured limit.
        validate_comparator();
    }

    bounded_priority_queue(std::size_t limit, Higher higher)
        : limit_(checked_limit(limit)), higher_(std::move(higher)) {
        validate_comparator();
    }

    template <typename InputIt, typename H = Higher,
              typename std::enable_if<std::is_default_constructible<H>::value &&
                                          is_input_iterator<InputIt>::value,
                                      int>::type = 0>
    bounded_priority_queue(std::size_t limit, InputIt first, InputIt last)
        : limit_(checked_limit(limit)), higher_() {
        validate_comparator();
        consume_range(std::move(first), std::move(last));
    }

    template <typename InputIt,
              typename std::enable_if<is_input_iterator<InputIt>::value,
                                      int>::type = 0>
    bounded_priority_queue(std::size_t limit, InputIt first, InputIt last,
                           Higher higher)
        : limit_(checked_limit(limit)), higher_(std::move(higher)) {
        validate_comparator();
        consume_range(std::move(first), std::move(last));
    }

    bounded_priority_queue(const bounded_priority_queue &) = delete;
    bounded_priority_queue &operator=(const bounded_priority_queue &) = delete;
    bounded_priority_queue &operator=(bounded_priority_queue &&) = delete;

    bounded_priority_queue(bounded_priority_queue &&other) noexcept
        : data_(std::move(other.data_)), limit_(other.limit_),
          higher_(std::move(other.higher_)) {
        other.data_.clear();
        other.limit_ = 0;
    }

    std::size_t size() const noexcept { return data_.size(); }
    bool empty() const noexcept { return data_.empty(); }
    std::size_t limit() const noexcept { return limit_; }

    // A full result is returned before copying, moving, comparing, or growing.
    // This makes capacity a decision the caller can check in ordinary control
    // flow, and leaves a rejected rvalue in the caller's hands.
    template <typename U = T,
              typename std::enable_if<std::is_copy_constructible<U>::value,
                                      int>::type = 0>
    push_result try_push(const T &value) {
        if (data_.size() == limit_) {
            return push_result::full;
        }
        // Snapshot first: value may be a reference returned by peek(), and a
        // later reserve could invalidate that reference. Copying can throw,
        // but it happens before any queue storage changes.
        T candidate(value);
        return push_candidate(candidate, [&candidate]() noexcept -> T && {
            return std::move(candidate);
        });
    }

    push_result try_push(T &&value) {
        if (data_.size() == limit_) {
            return push_result::full;
        }
        // Comparisons use the caller's value. The move is delayed until all
        // throwing work has completed; if a comparison or reserve fails, the
        // caller still owns the same value. Passing an rvalue that aliases an
        // existing slot is a documented precondition violation.
        return push_candidate(value, [&value]() noexcept -> T && {
            return std::move(value);
        });
    }

    // The reference is borrowed from this queue. Keep the queue alive and
    // unmoved, and do not use the borrow after a successful mutation.
    std::optional<std::reference_wrapper<const T>> peek() const noexcept {
        if (data_.empty()) {
            return std::nullopt;
        }
        return std::cref(data_.front().value);
    }

    std::optional<T> pop() {
        if (data_.empty()) {
            return std::nullopt;
        }
        if (data_.size() == 1) {
            std::optional<T> result(std::in_place,
                                    std::move(data_.front().value));
            data_.pop_back();
            return result;
        }

        // Plan the down-sift before moving the root. At each level choose the
        // higher-priority child, then decide whether the last item can stop
        // above it. Comparator or temporary path-allocation failure therefore
        // leaves both the queue and outstanding peek references untouched.
        const std::size_t remaining = data_.size() - 1;
        std::vector<std::size_t> path;
        std::size_t hole = 0;
        for (;;) {
            std::size_t left = 0;
            if (!left_child(hole, remaining, left)) {
                break;
            }
            std::size_t best = left;
            const std::size_t right = left + 1;
            if (right < remaining &&
                higher_(data_[right].value, data_[left].value)) {
                best = right;
            }
            if (!higher_(data_[best].value, data_.back().value)) {
                break;
            }
            path.push_back(best);
            hole = best;
        }

        std::optional<T> result(std::in_place,
                                std::move(data_.front().value));
        hole = 0;
        for (std::size_t child : path) {
            data_[hole].value = std::move(data_[child].value);
            hole = child;
        }
        data_[hole].value = std::move(data_.back().value);
        data_.pop_back();
        return result;
    }

    void clear() noexcept { data_.clear(); }

private:
    storage_type data_;
    std::size_t limit_;
    Higher higher_;

    static std::size_t checked_limit(std::size_t limit) {
        const storage_type empty_storage;
        if (limit > empty_storage.max_size()) {
            throw std::length_error("bounded priority queue limit exceeds max_size");
        }
        return limit;
    }

    void validate_comparator() const {
        if constexpr (std::is_pointer<Higher>::value &&
                      std::is_function<typename std::remove_pointer<Higher>::type>::value) {
            if (higher_ == nullptr) {
                throw std::invalid_argument("bounded priority queue comparator is null");
            }
        }
    }

    static bool left_child(std::size_t index, std::size_t count,
                           std::size_t &left) noexcept {
        // Prove a child fits before doubling the parent index. This is the
        // same complete-tree arithmetic as 2*i+1, written without overflow.
        if (count < 2 || index > (count - 2) / 2) {
            return false;
        }
        left = index * 2 + 1;
        return true;
    }

    std::size_t growth_target(std::size_t needed) const noexcept {
        std::size_t target = data_.capacity();
        if (target == 0) {
            target = 1;
        }
        while (target < needed) {
            if (target > limit_ / 2) {
                target = limit_;
            } else {
                target *= 2;
            }
        }
        return target;
    }

    template <typename Candidate, typename MoveCandidate>
    push_result push_candidate(const Candidate &candidate,
                               MoveCandidate move_candidate) {
        // First remember where the new item would rise. This path uses
        // O(log n) temporary indices; comparisons happen before mutation, so a
        // throwing comparator cannot leave half-shifted elements behind.
        std::vector<std::size_t> path;
        std::size_t child = data_.size();
        while (child > 0) {
            const std::size_t parent = (child - 1) / 2;
            if (!higher_(candidate, data_[parent].value)) {
                break;
            }
            path.push_back(parent);
            child = parent;
        }

        const std::size_t needed = data_.size() + 1;
        if (needed > data_.capacity()) {
            // Reserve is deliberately last among throwing operations. Growth
            // is geometric but capped by both logical and physical limits.
            // A growth-triggering push can still move Theta(n) slots once;
            // geometric growth amortizes that storage work over many pushes.
            data_.reserve(growth_target(needed));
        }

        // From here, T moves and destruction cannot throw. Move the candidate
        // into the new slot, shift its ancestors down, and fill the final hole.
        // Thus commit does not call the comparator or allocate a path.
        T staged = move_candidate();
        data_.push_back(slot{std::move(staged)});
        T pending(std::move(data_.back().value));
        std::size_t hole = data_.size() - 1;
        for (std::size_t parent : path) {
            data_[hole].value = std::move(data_[parent].value);
            hole = parent;
        }
        data_[hole].value = std::move(pending);
        return push_result::inserted;
    }

    template <typename InputIt>
    void consume_range(InputIt first, InputIt last) {
        // This loop consumes only values inside the logical limit. It builds
        // incrementally, so its comparison work is O(k log k), not Floyd's
        // O(k) heapify; input iterators may still prefetch when incremented.
        while (first != last) {
            if (data_.size() == limit_) {
                throw range_capacity_error(data_.size());
            }
            T value(*first);
            // The size check above guarantees a slot is available, so this
            // move must succeed unless a contract is violated.
            try_push(std::move(value));
            ++first;
        }
    }
};

template <typename T> using min_heap = binary_heap<T, std::less<T>>;
template <typename T> using max_heap = binary_heap<T, std::greater<T>>;

template <typename T>
using bounded_min_heap = bounded_priority_queue<T, std::less<T>>;
template <typename T>
using bounded_max_heap = bounded_priority_queue<T, std::greater<T>>;

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
