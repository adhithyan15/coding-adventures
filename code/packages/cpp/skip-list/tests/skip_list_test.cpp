// Tests for the C++ skip-list (ordered map), using the iso_test.h harness.
// Covers insert/overwrite/search/erase, order statistics, min/max, range
// queries, ordered entries, and reported parameters.
#include "iso_test.h"

#include <string>
#include <utility>
#include <vector>

#include "skip_list.hpp"

int main() {
    ca::skip_list<int, int> s;
    ISO_CHECK(s.empty());
    ISO_CHECK_EQ_UINT(s.max_level(), 32);
    ISO_CHECK_EQ_DBL(s.probability(), 0.5, 1e-12);
    ISO_CHECK_EQ_UINT(s.current_max(), 1);
    ISO_CHECK(!s.search(5).has_value());

    for (auto kv : std::vector<std::pair<int, int>>{
             {5, 50}, {1, 10}, {9, 90}, {3, 30}, {7, 70}}) {
        s.insert(kv.first, kv.second);
    }
    ISO_CHECK_EQ_UINT(s.size(), 5);
    ISO_CHECK(s.search(7).has_value());
    ISO_CHECK_EQ_INT(s.search(7).value(), 70);
    ISO_CHECK(s.contains(3));
    ISO_CHECK(!s.contains(4));

    // Overwrite.
    s.insert(3, 33);
    ISO_CHECK_EQ_UINT(s.size(), 5);
    ISO_CHECK_EQ_INT(s.search(3).value(), 33);

    // Order statistics: keys 1,3,5,7,9 at ranks 0..4.
    ISO_CHECK_EQ_UINT(s.rank(5).value(), 2);
    ISO_CHECK(!s.rank(4).has_value());
    ISO_CHECK_EQ_INT(s.by_rank(0).value(), 1);
    ISO_CHECK_EQ_INT(s.by_rank(4).value(), 9);
    ISO_CHECK(!s.by_rank(5).has_value());

    ISO_CHECK_EQ_INT(s.min().value(), 1);
    ISO_CHECK_EQ_INT(s.max().value(), 9);

    // Ordered entries.
    std::vector<std::pair<int, int>> expected = {
        {1, 10}, {3, 33}, {5, 50}, {7, 70}, {9, 90}};
    ISO_CHECK(s.entries() == expected);

    // Inclusive vs exclusive range.
    std::vector<std::pair<int, int>> incl = {{3, 33}, {5, 50}, {7, 70}};
    ISO_CHECK(s.range(3, 7, true) == incl);
    std::vector<std::pair<int, int>> excl = {{5, 50}};
    ISO_CHECK(s.range(3, 7, false) == excl);
    ISO_CHECK(s.range(7, 3, true).empty()); // inverted

    // erase.
    ISO_CHECK(s.erase(5));
    ISO_CHECK_EQ_UINT(s.size(), 4);
    ISO_CHECK(!s.contains(5));
    ISO_CHECK(!s.erase(5));
    ISO_CHECK_EQ_UINT(s.rank(7).value(), 2); // 1,3,7,9

    ISO_CHECK(s.current_max() >= 1);

    return ISO_TEST_RESULT();
}
