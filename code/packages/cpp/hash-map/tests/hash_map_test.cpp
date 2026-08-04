// Tests for the C++ hash map, using the iso_test.h harness. Exercises both
// collision strategies, all four hash functions, overwrite/delete semantics,
// tombstone reuse, resizing, and both string and integer keys.
#include "iso_test.h"

#include <algorithm>
#include <cstdio>
#include <string>
#include <vector>

#include "hash_map.hpp"

// Run the core behavioural suite against a freshly built string→string map.
static void exercise(ca::collision_strategy strat, ca::hash_algorithm hash) {
    ca::hash_map<std::string, std::string> m(4, strat, hash);
    ISO_CHECK(m.strategy() == strat);
    ISO_CHECK(m.hash_function() == hash);
    ISO_CHECK(m.empty());

    m.set("one", "1");
    m.set("two", "2");
    m.set("three", "3");
    ISO_CHECK_EQ_UINT(m.size(), 3);
    ISO_CHECK(m.get("one") == std::optional<std::string>("1"));
    ISO_CHECK(m.get("two") == std::optional<std::string>("2"));
    ISO_CHECK(m.get("three") == std::optional<std::string>("3"));
    ISO_CHECK(m.has("one"));
    ISO_CHECK(!m.has("missing"));
    ISO_CHECK(!m.get("missing").has_value());

    // Overwrite.
    m.set("two", "22");
    ISO_CHECK_EQ_UINT(m.size(), 3);
    ISO_CHECK(m.get("two") == std::optional<std::string>("22"));

    // Delete, and a redundant delete.
    ISO_CHECK(m.remove("two"));
    ISO_CHECK_EQ_UINT(m.size(), 2);
    ISO_CHECK(!m.has("two"));
    ISO_CHECK(!m.remove("two"));

    // Reinsert (tombstone reuse for open addressing).
    m.set("two", "200");
    ISO_CHECK_EQ_UINT(m.size(), 3);
    ISO_CHECK(m.get("two") == std::optional<std::string>("200"));
}

int main() {
    ca::collision_strategy strategies[2] = {
        ca::collision_strategy::chaining,
        ca::collision_strategy::open_addressing};
    ca::hash_algorithm hashes[4] = {
        ca::hash_algorithm::siphash24, ca::hash_algorithm::fnv1a32,
        ca::hash_algorithm::murmur3_32, ca::hash_algorithm::djb2};

    for (auto s : strategies) {
        for (auto h : hashes) {
            exercise(s, h);
        }
    }

    // Integer keys, integer values, with resizing.
    {
        ca::hash_map<int, int> m(4, ca::collision_strategy::open_addressing,
                                 ca::hash_algorithm::murmur3_32);
        for (int i = 0; i < 300; i++) {
            m.set(i, i * i);
        }
        ISO_CHECK_EQ_UINT(m.size(), 300);
        bool all_found = true;
        for (int i = 0; i < 300; i++) {
            auto v = m.get(i);
            if (!v.has_value() || v.value() != i * i) {
                all_found = false;
            }
        }
        ISO_CHECK_MSG(all_found, "every integer key survives resizing");
        ISO_CHECK(m.load_factor() <= 0.75);
    }

    // Chaining resize preserves all string entries.
    {
        ca::hash_map<std::string, int> m(4, ca::collision_strategy::chaining,
                                         ca::hash_algorithm::siphash24);
        char buf[16];
        for (int i = 0; i < 500; i++) {
            std::snprintf(buf, sizeof buf, "k-%d", i);
            m.set(std::string(buf), i);
        }
        ISO_CHECK_EQ_UINT(m.size(), 500);
        ISO_CHECK(m.capacity() > 4);
        bool all_found = true;
        for (int i = 0; i < 500; i++) {
            std::snprintf(buf, sizeof buf, "k-%d", i);
            auto v = m.get(std::string(buf));
            if (!v.has_value() || v.value() != i) {
                all_found = false;
            }
        }
        ISO_CHECK_MSG(all_found, "every string key survives resizing");

        // keys() returns exactly the inserted set.
        auto ks = m.keys();
        ISO_CHECK_EQ_UINT(ks.size(), 500);
    }

    return ISO_TEST_RESULT();
}
