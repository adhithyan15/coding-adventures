// Tests for the C++ B+ tree, using the iso_test.h harness. Mirrors the Rust
// crate's checks (bulk insert → sorted full scan → validity → range scan →
// delete) across several degrees, plus a generic string-value check.
#include "iso_test.h"

#include <string>
#include <utility>
#include <vector>

#include "b_plus_tree.hpp"

static void torture(std::size_t t, int n) {
    ca::b_plus_tree<int, int> tree(t);
    for (int i = 0; i < n; i++) {
        int key = (i * 617 + 3) % n;
        tree.insert(key, key * 10);
    }
    ISO_CHECK_EQ_UINT(tree.len(), static_cast<std::size_t>(n));
    ISO_CHECK(tree.is_valid());

    bool ok_search = true;
    for (int i = 0; i < n; i++) {
        const int *v = tree.search(i);
        if (v == nullptr || *v != i * 10) {
            ok_search = false;
        }
    }
    ISO_CHECK_MSG(ok_search, "every inserted key must be found");
    ISO_CHECK(!tree.contains(n));

    ISO_CHECK(tree.min_key() == std::optional<int>(0));
    ISO_CHECK(tree.max_key() == std::optional<int>(n - 1));

    // Full scan (leaf-chain walk) must be sorted and complete.
    auto fs = tree.full_scan();
    ISO_CHECK_EQ_UINT(fs.size(), static_cast<std::size_t>(n));
    bool ok_sorted = true;
    for (int i = 0; i < n; i++) {
        if (fs[static_cast<std::size_t>(i)].first != i ||
            fs[static_cast<std::size_t>(i)].second != i * 10) {
            ok_sorted = false;
        }
    }
    ISO_CHECK_MSG(ok_sorted, "full scan must be sorted and complete");

    auto rs = tree.range_scan(n / 4, n / 2);
    ISO_CHECK_EQ_UINT(rs.size(), static_cast<std::size_t>(n / 2 - n / 4 + 1));
    ISO_CHECK(rs.front().first == n / 4);

    for (int i = 0; i < n; i += 2) {
        ISO_CHECK(tree.remove(i));
    }
    ISO_CHECK(tree.is_valid());
    ISO_CHECK_EQ_UINT(tree.len(), static_cast<std::size_t>(n - (n + 1) / 2));
    bool ok_remain = true;
    for (int i = 0; i < n; i++) {
        if (tree.contains(i) != (i % 2 != 0)) {
            ok_remain = false;
        }
    }
    ISO_CHECK_MSG(ok_remain, "after deleting evens, exactly the odds remain");
    ISO_CHECK(!tree.remove(0));
}

int main() {
    // Small hand-checkable example with string values (generic).
    {
        ca::b_plus_tree<int, std::string> tree(2);
        ISO_CHECK(tree.empty());
        tree.insert(10, "ten");
        tree.insert(5, "five");
        tree.insert(20, "twenty");
        const std::string *v = tree.search(10);
        ISO_CHECK(v != nullptr && *v == "ten");
        auto rs = tree.range_scan(5, 15);
        ISO_CHECK_EQ_UINT(rs.size(), 2);
        ISO_CHECK(rs[0].first == 5 && rs[1].first == 10);
        ISO_CHECK(tree.is_valid());
        tree.insert(10, "TEN"); // overwrite
        ISO_CHECK_EQ_UINT(tree.len(), 3);
        ISO_CHECK(*tree.search(10) == "TEN");
    }

    // Empty tree.
    {
        ca::b_plus_tree<int, int> tree(3);
        ISO_CHECK(!tree.min_key().has_value());
        ISO_CHECK(!tree.max_key().has_value());
        ISO_CHECK(!tree.remove(1));
        ISO_CHECK(tree.is_valid());
        ISO_CHECK_EQ_UINT(tree.height(), 0);
    }

    torture(2, 1000);
    torture(3, 1500);
    torture(6, 2000);

    return ISO_TEST_RESULT();
}
