// Tests for the C++ Fenwick tree, using the header-only iso_test.h harness.
// Mirrors the Rust crate and the C port: build from values, point/prefix/range
// queries, updates, find_kth, and the throwing error paths.
#include "iso_test.h"

#include <stdexcept>
#include <vector>

#include "fenwick_tree.hpp"

// Small helper: run `body` and report whether it threw the expected exception.
template <typename Ex, typename F> static bool throws(F body) {
    try {
        body();
    } catch (const Ex &) {
        return true;
    } catch (...) {
        return false;
    }
    return false;
}

int main() {
    ca::fenwick_tree t(std::vector<double>{3, 2, -1, 6, 5, 4, -3, 3});

    ISO_CHECK_EQ_UINT(t.size(), 8);
    ISO_CHECK(!t.empty());

    ISO_CHECK_EQ_DBL(t.prefix_sum(0), 0.0, 1e-9);
    ISO_CHECK_EQ_DBL(t.prefix_sum(5), 15.0, 1e-9);
    ISO_CHECK_EQ_DBL(t.prefix_sum(8), 19.0, 1e-9);

    ISO_CHECK_EQ_DBL(t.point_query(4), 6.0, 1e-9);
    ISO_CHECK_EQ_DBL(t.point_query(3), -1.0, 1e-9);
    ISO_CHECK_EQ_DBL(t.range_sum(3, 6), 14.0, 1e-9);

    t.update(3, 5.0);
    ISO_CHECK_EQ_DBL(t.point_query(3), 4.0, 1e-9);
    ISO_CHECK_EQ_DBL(t.range_sum(3, 6), 19.0, 1e-9);

    // Throwing error paths.
    ISO_CHECK(throws<std::out_of_range>([&] { t.update(0, 1.0); }));
    ISO_CHECK(throws<std::out_of_range>([&] { t.update(9, 1.0); }));
    ISO_CHECK(throws<std::out_of_range>([&] { (void)t.prefix_sum(9); }));
    ISO_CHECK(throws<std::invalid_argument>([&] { (void)t.range_sum(5, 3); }));

    // find_kth cumulative search over [1, 3, 2, 4] (cumulative 1,4,6,10).
    ca::fenwick_tree f(std::vector<double>{1, 3, 2, 4});
    ISO_CHECK_EQ_UINT(f.find_kth(1.0), 1);
    ISO_CHECK_EQ_UINT(f.find_kth(4.0), 2);
    ISO_CHECK_EQ_UINT(f.find_kth(5.0), 3);
    ISO_CHECK_EQ_UINT(f.find_kth(10.0), 4);
    ISO_CHECK(throws<std::invalid_argument>([&] { (void)f.find_kth(0.0); }));
    ISO_CHECK(throws<std::invalid_argument>([&] { (void)f.find_kth(11.0); }));

    ca::fenwick_tree empty(0);
    ISO_CHECK(empty.empty());
    ISO_CHECK(throws<std::out_of_range>([&] { (void)empty.find_kth(1.0); }));

    return ISO_TEST_RESULT();
}
