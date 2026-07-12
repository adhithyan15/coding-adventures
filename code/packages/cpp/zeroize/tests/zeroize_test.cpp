// Tests for the C++ zeroize library, using the header-only iso_test.h harness
// (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "zeroize.hpp"

namespace z = ca::zeroize;

// A user type whose zeroize() wipes a *borrowed* buffer. Because the buffer
// outlives the Zeroizing guard, we can inspect it after the guard drops. The
// overload lives in the global namespace so ADL finds it.
struct BorrowedZeroizer {
    unsigned char* buf;
    std::size_t len;
};
void zeroize(BorrowedZeroizer& b) { z::zeroize_bytes(b.buf, b.len); }

template <class T>
static bool all_zero(const T* p, std::size_t n) {
    for (std::size_t i = 0; i < n; i++)
        if (p[i] != 0) return false;
    return true;
}

int main() {
    ISO_CHECK_STR_EQ(z::VERSION, "0.1.0");

    // ── fixed-size byte array ─────────────────────────────────────────────
    {
        std::array<std::uint8_t, 16> arr;
        arr.fill(0xFF);
        z::zeroize(arr);
        ISO_CHECK(all_zero(arr.data(), arr.size()));
    }

    // ── integers ──────────────────────────────────────────────────────────
    {
        std::uint64_t x = 0xDEADBEEFCAFEF00Dull;
        z::zeroize(x);
        ISO_CHECK(x == 0u);
        std::int32_t y = -12345;
        z::zeroize(y);
        ISO_CHECK(y == 0);
        std::uint8_t b = 0xAA;
        z::zeroize(b);
        ISO_CHECK(b == 0u);
    }

    // ── byte vector: live bytes wiped, length cleared ─────────────────────
    {
        // The overload wipes then empties (only the live length; see the note
        // in the header on why C++ can't reach capacity bytes).
        std::vector<std::uint8_t> v(16, 0xFF);
        z::zeroize(v);
        ISO_CHECK(v.empty());
        // Prove the byte wipe directly on live vector storage (in bounds).
        std::vector<std::uint8_t> w(16, 0xFF);
        z::zeroize_bytes(w.data(), w.size());
        ISO_CHECK(all_zero(w.data(), w.size()));
    }

    // ── string: live bytes wiped, emptied ─────────────────────────────────
    {
        std::string s = "hunter2";
        z::zeroize(s);
        ISO_CHECK(s.empty());
        std::string t = "hunter2";
        z::zeroize_bytes(&t[0], t.size());
        ISO_CHECK(all_zero(reinterpret_cast<const unsigned char*>(t.data()),
                           t.size()));
    }

    // ── optional: payload wiped, reset to nullopt ─────────────────────────
    {
        std::optional<std::array<std::uint8_t, 8>> opt =
            std::array<std::uint8_t, 8>{};
        opt->fill(0xAA);
        z::zeroize(opt);
        ISO_CHECK(!opt.has_value());

        std::optional<std::uint64_t> none;
        z::zeroize(none);
        ISO_CHECK(!none.has_value());
    }

    // ── Zeroizing derefs (read + mutate the protected value) ──────────────
    {
        std::array<std::uint8_t, 32> forty_two;
        forty_two.fill(0x42);
        z::Zeroizing<std::array<std::uint8_t, 32>> key(forty_two);
        ISO_CHECK((*key)[0] == 0x42);
        ISO_CHECK_EQ_UINT(key->size(), 32u);

        z::Zeroizing<std::array<std::uint8_t, 4>> key2(
            std::array<std::uint8_t, 4>{});
        (*key2)[0] = 1;
        (*key2)[1] = 2;
        ISO_CHECK((*key2)[0] == 1 && (*key2)[1] == 2 && (*key2)[2] == 0);
    }

    // ── Zeroizing::drop wipes an observable (borrowed) buffer ─────────────
    {
        unsigned char owned[48];
        for (unsigned char& c : owned) c = 0xCD;
        {
            z::Zeroizing<BorrowedZeroizer> guard(BorrowedZeroizer{owned, 48});
            (void)guard;  // wipes `owned` when it drops at end of scope
        }
        ISO_CHECK(all_zero(owned, 48));
    }

    // ── into_inner opts out of the wipe ───────────────────────────────────
    {
        std::array<std::uint8_t, 16> seed;
        seed.fill(0x77);
        z::Zeroizing<std::array<std::uint8_t, 16>> key(seed);
        std::array<std::uint8_t, 16> taken = key.into_inner();
        // The caller now holds the live bytes (not wiped).
        bool all_77 = true;
        for (std::uint8_t b : taken)
            if (b != 0x77) all_77 = false;
        ISO_CHECK(all_77);
    }

    return ISO_TEST_RESULT();
}
