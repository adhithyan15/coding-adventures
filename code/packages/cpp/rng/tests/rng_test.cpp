// Tests for the C++ rng, using the iso_test.h harness. Pinned to the crate's
// reference values (seed = 1) plus determinism, range, and float checks.
#include "iso_test.h"

#include <cstdint>

#include "rng.hpp"

int main() {
    // Reference values: first three next_u32() for seed = 1.
    {
        ca::rng::Lcg g(1);
        ISO_CHECK_EQ_UINT(g.next_u32(), 1817669548u);
        ISO_CHECK_EQ_UINT(g.next_u32(), 2187888307u);
        ISO_CHECK_EQ_UINT(g.next_u32(), 2784682393u);
    }
    {
        ca::rng::Xorshift64 g(1);
        ISO_CHECK_EQ_UINT(g.next_u32(), 1082269761u);
        ISO_CHECK_EQ_UINT(g.next_u32(), 201397313u);
        ISO_CHECK_EQ_UINT(g.next_u32(), 1854285353u);
    }
    {
        ca::rng::Pcg32 g(1);
        ISO_CHECK_EQ_UINT(g.next_u32(), 1412771199u);
        ISO_CHECK_EQ_UINT(g.next_u32(), 1791099446u);
        ISO_CHECK_EQ_UINT(g.next_u32(), 124312908u);
    }

    // Determinism.
    {
        ca::rng::Pcg32 a(42), b(42);
        bool same = true;
        for (int i = 0; i < 10; i++) {
            if (a.next_u32() != b.next_u32()) {
                same = false;
            }
        }
        ISO_CHECK_MSG(same, "same seed must produce the same sequence");
    }

    // Different seeds diverge.
    {
        ca::rng::Lcg a(1), b(2);
        bool differ = false;
        for (int i = 0; i < 5; i++) {
            if (a.next_u32() != b.next_u32()) {
                differ = true;
            }
        }
        ISO_CHECK_MSG(differ, "different seeds must diverge");
    }

    // Xorshift64 seed 0 -> 1, never zero.
    {
        ca::rng::Xorshift64 z(0), o(1);
        ISO_CHECK_EQ_UINT(z.next_u32(), o.next_u32());
        bool never_zero = true;
        for (int i = 0; i < 100; i++) {
            if (z.next_u32() == 0) {
                never_zero = false;
            }
        }
        ISO_CHECK_MSG(never_zero, "Xorshift64 must not collapse to zero");
    }

    // next_u64 packs two u32 draws.
    {
        ca::rng::Lcg a(7), b(7);
        std::uint64_t combined = a.next_u64();
        std::uint32_t hi = b.next_u32();
        std::uint32_t lo = b.next_u32();
        ISO_CHECK(combined == ((static_cast<std::uint64_t>(hi) << 32) | lo));
    }

    // next_float in [0, 1).
    {
        ca::rng::Pcg32 g(123);
        bool in_range = true;
        for (int i = 0; i < 1000; i++) {
            double f = g.next_float();
            if (!(f >= 0.0 && f < 1.0)) {
                in_range = false;
            }
        }
        ISO_CHECK_MSG(in_range, "next_float must be in [0, 1)");
    }

    // next_int_in_range within bounds and hits the ends.
    {
        ca::rng::Xorshift64 g(55);
        bool in_range = true, saw_min = false, saw_max = false;
        for (int i = 0; i < 5000; i++) {
            std::int64_t v = g.next_int_in_range(-3, 3);
            if (v < -3 || v > 3) {
                in_range = false;
            }
            if (v == -3) {
                saw_min = true;
            }
            if (v == 3) {
                saw_max = true;
            }
        }
        ISO_CHECK_MSG(in_range, "range draws must stay in [min, max]");
        ISO_CHECK(saw_min && saw_max);
        ISO_CHECK_EQ_INT(static_cast<int>(g.next_int_in_range(42, 42)), 42);
    }

    return ISO_TEST_RESULT();
}
