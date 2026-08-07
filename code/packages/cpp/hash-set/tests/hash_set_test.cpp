// Tests for the C++ hash set, using the iso_test.h harness. Mirrors the Rust
// crate's tests: membership, duplicate handling, set algebra, relations.
#include "iso_test.h"

#include <algorithm>
#include <string>
#include <vector>

#include "hash_set.hpp"

static ca::hash_set<std::string> make(std::initializer_list<const char *> xs) {
    ca::hash_set<std::string> s;
    for (const char *x : xs) {
        s.add(std::string(x));
    }
    return s;
}

int main() {
    // Basic membership.
    {
        auto s = make({"one", "two", "three"});
        ISO_CHECK_EQ_UINT(s.size(), 3);
        ISO_CHECK(s.contains("one"));
        ISO_CHECK(!s.contains("four"));
        ISO_CHECK(!s.empty());
    }

    // Duplicates ignored.
    {
        auto s = make({"x", "x", "y", "y", "z"});
        ISO_CHECK_EQ_UINT(s.size(), 3);
    }

    // Remove and redundant remove.
    {
        auto s = make({"a", "b"});
        ISO_CHECK(s.remove("b"));
        ISO_CHECK(!s.contains("b"));
        ISO_CHECK(!s.remove("b"));
        ISO_CHECK_EQ_UINT(s.size(), 1);
    }

    // Set algebra: A={1..5}, B={3..7}.
    {
        auto a = make({"1", "2", "3", "4", "5"});
        auto b = make({"3", "4", "5", "6", "7"});

        ISO_CHECK_EQ_UINT(a.union_with(b).size(), 7);
        ISO_CHECK_EQ_UINT(a.intersection(b).size(), 3);
        ISO_CHECK_EQ_UINT(a.difference(b).size(), 2);
        ISO_CHECK_EQ_UINT(a.symmetric_difference(b).size(), 4);

        auto inter = a.intersection(b);
        ISO_CHECK(inter.contains("3") && inter.contains("4") &&
                  inter.contains("5"));
        auto diff = a.difference(b);
        ISO_CHECK(diff.contains("1") && diff.contains("2") &&
                  !diff.contains("3"));
        auto sym = a.symmetric_difference(b);
        ISO_CHECK(sym.contains("1") && sym.contains("6") && !sym.contains("3"));

        // to_vector() returns exactly the members.
        auto v = a.union_with(b).to_vector();
        std::sort(v.begin(), v.end());
        ISO_CHECK_EQ_UINT(v.size(), 7);
        ISO_CHECK(v.front() == "1" && v.back() == "7");
    }

    // Relations: A={1,2,3}, B={1,2,3,4,5}, C={10,20}.
    {
        auto a = make({"1", "2", "3"});
        auto b = make({"1", "2", "3", "4", "5"});
        auto c = make({"10", "20"});
        auto a2 = make({"1", "2", "3"});

        ISO_CHECK(a.is_subset(b));
        ISO_CHECK(!a.is_subset(c));
        ISO_CHECK(b.is_superset(a));
        ISO_CHECK(a.is_disjoint(c));
        ISO_CHECK(!a.is_disjoint(b));
        ISO_CHECK(a.equals(a2));
        ISO_CHECK(!a.equals(b));
    }

    // Integer elements with a non-default configuration.
    {
        ca::hash_set<int> s(4, ca::collision_strategy::open_addressing,
                            ca::hash_algorithm::murmur3_32);
        for (int i = 0; i < 100; i++) {
            s.add(i);
            s.add(i); // duplicate
        }
        ISO_CHECK_EQ_UINT(s.size(), 100);
        ISO_CHECK(s.contains(42));
        ISO_CHECK(!s.contains(1000));
    }

    return ISO_TEST_RESULT();
}
