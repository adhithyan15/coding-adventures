// Tests for the C++ Bloom filter, using the iso_test.h harness. Mirrors the Rust
// crate's tests: no false negatives, consistent stats, the sizing formulas, and
// parameter validation (both throwing and std::optional forms).
#include "iso_test.h"

#include <cstdio>
#include <limits>
#include <optional>
#include <stdexcept>
#include <string>

#include "bloom_filter.hpp"

int main() {
    // A fresh filter is empty.
    {
        ca::bloom_filter bf(1000, 0.01);
        ISO_CHECK_EQ_UINT(bf.bits_set(), 0);
        ISO_CHECK_EQ_DBL(bf.fill_ratio(), 0.0, 1e-12);
        ISO_CHECK(!bf.is_over_capacity());
    }

    // add() then contains().
    {
        ca::bloom_filter bf; // defaults: 1000 items, p=0.01
        bf.add(std::string("hello"));
        ISO_CHECK(bf.contains(std::string("hello")));
    }

    // No false negatives across 200 inserted items.
    {
        ca::bloom_filter bf(1000, 0.01);
        char buf[32];
        for (int i = 0; i < 200; i++) {
            std::snprintf(buf, sizeof buf, "item-%d", i);
            bf.add(std::string(buf));
        }
        bool found = true;
        for (int i = 0; i < 200; i++) {
            std::snprintf(buf, sizeof buf, "item-%d", i);
            if (!bf.contains(std::string(buf))) {
                found = false;
            }
        }
        ISO_CHECK_MSG(found, "no inserted item may be missing");
    }

    // Stats are consistent after an insertion.
    {
        ca::bloom_filter bf(100, 0.01);
        bf.add(std::string("alpha"));
        ISO_CHECK(bf.bit_count() > 0);
        ISO_CHECK(bf.hash_count() >= 1);
        ISO_CHECK(bf.bits_set() > 0);
        ISO_CHECK(bf.size_bytes() > 0);
        ISO_CHECK(bf.fill_ratio() > 0.0);
        ISO_CHECK(bf.estimated_false_positive_rate() >= 0.0);
    }

    // Sizing helpers match the reference expectations.
    {
        std::size_t m = ca::bloom_filter::optimal_m(1000000, 0.01);
        std::size_t k = ca::bloom_filter::optimal_k(m, 1000000);
        ISO_CHECK(m > 9000000);
        ISO_CHECK_EQ_UINT(k, 7);
        ISO_CHECK(ca::bloom_filter::capacity_for_memory(1000000, 0.01) > 0);
    }

    // Invalid parameters: try_* returns nullopt, throwing ctor throws.
    {
        ISO_CHECK(!ca::bloom_filter::try_create(0, 0.01).has_value());
        ISO_CHECK(!ca::bloom_filter::try_create(1, 0.0).has_value());
        ISO_CHECK(!ca::bloom_filter::try_create(1, 1.0).has_value());
        ISO_CHECK(!ca::bloom_filter::try_from_params(0, 1).has_value());
        ISO_CHECK(!ca::bloom_filter::try_from_params(1, 0).has_value());

        bool threw = false;
        try {
            ca::bloom_filter bad(0, 0.01);
        } catch (const std::invalid_argument &) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // Explicit-parameter construction.
    {
        auto bf = ca::bloom_filter::from_params(1024, 3);
        ISO_CHECK_EQ_UINT(bf.bit_count(), 1024);
        ISO_CHECK_EQ_UINT(bf.hash_count(), 3);
        ISO_CHECK_EQ_UINT(bf.size_bytes(), 128);
        bf.add(std::string("x"));
        ISO_CHECK(bf.contains(std::string("x")));
        ISO_CHECK(!bf.contains(std::string("definitely-not-present-zzz")));
    }

    // Non-finite sizing inputs are handled gracefully (no hang, no UB).
    {
        constexpr double inf = std::numeric_limits<double>::infinity();
        constexpr double nan = std::numeric_limits<double>::quiet_NaN();
        ISO_CHECK_EQ_UINT(ca::bloom_filter::optimal_m(1000, inf), 0);
        ISO_CHECK_EQ_UINT(ca::bloom_filter::optimal_m(1000, nan), 0);
        ISO_CHECK_EQ_UINT(ca::bloom_filter::capacity_for_memory(1024, inf), 0);
        ISO_CHECK_EQ_UINT(ca::bloom_filter::capacity_for_memory(0, nan), 0);
    }

    return ISO_TEST_RESULT();
}
