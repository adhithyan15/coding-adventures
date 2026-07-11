// Tests for the C++ binary heap, using the header-only iso_test.h harness.
// Covers min/max heaps, push/pop/peek, empty behavior, heapify-construction,
// heap_sort, and nlargest/nsmallest.
#include "iso_test.h"

#include <vector>

#include "heap.hpp"

int main() {
    // MIN heap: smallest pops first.
    ca::min_heap<int> h;
    ISO_CHECK(h.empty());
    ISO_CHECK(!h.pop().has_value());
    ISO_CHECK(!h.peek().has_value());

    for (int v : {5, 3, 8, 1, 9, 2, 7}) {
        h.push(v);
    }
    ISO_CHECK_EQ_UINT(h.size(), 7);
    ISO_CHECK_EQ_INT(h.peek().value(), 1);

    std::vector<int> drained;
    while (auto v = h.pop()) {
        drained.push_back(*v);
    }
    ISO_CHECK(drained == (std::vector<int>{1, 2, 3, 5, 7, 8, 9}));

    // MAX heap constructed via bulk heapify.
    ca::max_heap<int> m(std::vector<int>{4, 10, 6, 2, 8});
    ISO_CHECK_EQ_INT(m.peek().value(), 10);
    std::vector<int> maxdrain;
    while (auto v = m.pop()) {
        maxdrain.push_back(*v);
    }
    ISO_CHECK(maxdrain == (std::vector<int>{10, 8, 6, 4, 2}));

    // heap_sort ascending (with duplicates and negatives).
    auto sorted = ca::heap_sort(std::vector<int>{5, -3, 5, 0, 9, -3, 1, 2});
    ISO_CHECK(sorted == (std::vector<int>{-3, -3, 0, 1, 2, 5, 5, 9}));
    ISO_CHECK(ca::heap_sort(std::vector<int>{}).empty());

    // nlargest / nsmallest.
    std::vector<int> data{5, 1, 8, 3, 9, 2, 7};
    ISO_CHECK(ca::nlargest(data, 3) == (std::vector<int>{9, 8, 7}));
    ISO_CHECK(ca::nsmallest(data, 3) == (std::vector<int>{1, 2, 3}));
    ISO_CHECK(ca::nlargest(data, 0).empty());
    // n >= size returns everything, sorted.
    ISO_CHECK(ca::nsmallest(std::vector<int>{3, 1, 2}, 9) ==
              (std::vector<int>{1, 2, 3}));

    return ISO_TEST_RESULT();
}
