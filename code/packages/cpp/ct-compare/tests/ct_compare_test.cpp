// Tests for the C++ ct-compare, using the iso_test.h harness. These verify
// functional correctness; the every-bit-flip sweeps are what a short-circuit
// bug would fail.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <stdexcept>
#include <vector>

#include "ct_compare.hpp"

namespace ct = ca::ct_compare;

int main() {
    // ct_eq: equal inputs.
    {
        std::vector<std::uint8_t> a = {'a', 'b', 'c', 'd', 'e', 'f'};
        std::vector<std::uint8_t> b = a;
        ISO_CHECK(ct::ct_eq(a, b));
        std::vector<std::uint8_t> z32(32, 0x00);
        ISO_CHECK(ct::ct_eq(z32, z32));
    }

    // ct_eq: every single-bit flip at every byte position is detected.
    {
        std::vector<std::uint8_t> base(32, 0x42);
        bool all_detected = true;
        for (std::size_t i = 0; i < 32; ++i) {
            for (int bit = 0; bit < 8; ++bit) {
                std::vector<std::uint8_t> flipped = base;
                flipped[i] = static_cast<std::uint8_t>(flipped[i] ^ (1u << bit));
                if (ct::ct_eq(base, flipped)) {
                    all_detected = false;
                }
            }
        }
        ISO_CHECK_MSG(all_detected, "every bit flip must be detected");
    }

    // ct_eq: length mismatch, empties, and first/last-byte differences.
    {
        std::vector<std::uint8_t> abc = {'a', 'b', 'c'};
        std::vector<std::uint8_t> abcd = {'a', 'b', 'c', 'd'};
        std::vector<std::uint8_t> empty;
        ISO_CHECK(!ct::ct_eq(abc, abcd));
        ISO_CHECK(!ct::ct_eq(abcd, abc));
        ISO_CHECK(ct::ct_eq(empty, empty));
        std::vector<std::uint8_t> x = {'a', 'b', 'c', 'd', 'e', 'f'};
        std::vector<std::uint8_t> first = {'b', 'b', 'c', 'd', 'e', 'f'};
        std::vector<std::uint8_t> last = {'a', 'b', 'c', 'd', 'e', 'g'};
        ISO_CHECK(!ct::ct_eq(x, first));
        ISO_CHECK(!ct::ct_eq(x, last));
    }

    // ct_eq_fixed over std::array (compile-time length).
    {
        std::array<std::uint8_t, 16> a;
        a.fill(0x11);
        std::array<std::uint8_t, 16> b = a;
        ISO_CHECK(ct::ct_eq_fixed(a, b));
        b[7] = static_cast<std::uint8_t>(b[7] ^ 0x80);
        ISO_CHECK(!ct::ct_eq_fixed(a, b));
    }

    // ct_select_bytes: choice picks the right buffer, byte for byte.
    {
        std::vector<std::uint8_t> a, b;
        for (int i = 0; i < 8; ++i) {
            a.push_back(static_cast<std::uint8_t>(0xA0 + i));
            b.push_back(static_cast<std::uint8_t>(0xB0 + i));
        }
        ISO_CHECK(ct::ct_select_bytes(a, b, true) == a);
        ISO_CHECK(ct::ct_select_bytes(a, b, false) == b);

        // Length mismatch throws.
        std::vector<std::uint8_t> shorter = {1, 2, 3};
        bool threw = false;
        try {
            (void)ct::ct_select_bytes(a, shorter, true);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK_MSG(threw, "ct_select_bytes must throw on length mismatch");
    }

    // ct_eq_u64: equal, every single-bit difference, and the high bit.
    {
        bool all_detected = true;
        ISO_CHECK(ct::ct_eq_u64(0, 0));
        ISO_CHECK(ct::ct_eq_u64(0xFFFFFFFFFFFFFFFFull, 0xFFFFFFFFFFFFFFFFull));
        ISO_CHECK(ct::ct_eq_u64(0xDEADBEEFull, 0xDEADBEEFull));
        for (int bit = 0; bit < 64; ++bit) {
            std::uint64_t base = 0x0123456789ABCDEFull;
            std::uint64_t flipped = base ^ (static_cast<std::uint64_t>(1) << bit);
            if (ct::ct_eq_u64(base, flipped)) {
                all_detected = false;
            }
        }
        ISO_CHECK_MSG(all_detected, "every u64 bit difference must be detected");
        ISO_CHECK(!ct::ct_eq_u64(0, static_cast<std::uint64_t>(1) << 63));
    }

    return ISO_TEST_RESULT();
}
