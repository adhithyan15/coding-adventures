// Tests for the C++ segment tree, using the header-only iso_test.h harness.
// Covers sum/min/max factories, a custom combine (gcd), range queries, point
// updates, and safe empty / out-of-range behavior.
#include "iso_test.h"

#include <vector>

#include "segment_tree.hpp"

static int gcd(int a, int b) {
    while (b != 0) {
        int t = a % b;
        a = b;
        b = t;
    }
    return a < 0 ? -a : a;
}

int main() {
    std::vector<int> values{1, 3, 5, 7, 9, 11};

    // sum tree
    auto sum = ca::segment_tree<int>::sum_tree(values);
    ISO_CHECK_EQ_UINT(sum.size(), 6);
    ISO_CHECK(!sum.empty());
    ISO_CHECK_EQ_INT(sum.query(0, 5), 36);
    ISO_CHECK_EQ_INT(sum.query(1, 3), 15);
    sum.update(2, 10); // 5 → 10
    ISO_CHECK_EQ_INT(sum.query(1, 3), 20);
    ISO_CHECK_EQ_INT(sum.query(3, 2), 0);  // inverted → identity
    ISO_CHECK_EQ_INT(sum.query(0, 99), 0); // out of range → identity

    // min / max trees
    auto mn = ca::segment_tree<int>::min_tree(values);
    ISO_CHECK_EQ_INT(mn.query(0, 5), 1);
    mn.update(4, -2);
    ISO_CHECK_EQ_INT(mn.query(2, 4), -2);

    auto mx = ca::segment_tree<int>::max_tree(values);
    ISO_CHECK_EQ_INT(mx.query(0, 5), 11);
    mx.update(0, 100);
    ISO_CHECK_EQ_INT(mx.query(0, 2), 100);

    // custom combine: range gcd. gcd(4,6,9)=1; gcd(12,18)=6.
    auto g = ca::segment_tree<int>(std::vector<int>{12, 18, 6, 4, 9},
                                   [](const int &a, const int &b) { return gcd(a, b); }, 0);
    ISO_CHECK_EQ_INT(g.query(0, 1), 6);
    ISO_CHECK_EQ_INT(g.query(0, 4), 1);

    // empty tree
    auto e = ca::segment_tree<int>::sum_tree(std::vector<int>{});
    ISO_CHECK(e.empty());
    ISO_CHECK_EQ_INT(e.query(0, 0), 0);
    e.update(0, 5); // ignored, no crash

    return ISO_TEST_RESULT();
}
