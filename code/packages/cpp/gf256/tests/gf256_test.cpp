// Tests for the C++ gf256, using the iso_test.h harness. Pinned to known GF(2^8)
// values (Reed-Solomon 0x11D and AES 0x11B fields) plus algebraic round-trips.
#include "iso_test.h"

#include <cstdint>

#include "gf256.hpp"

int main() {
    namespace gf = ca::gf256;

    // Addition / subtraction are XOR.
    ISO_CHECK_EQ_UINT(gf::add(0x53, 0xCA), 0x99);
    ISO_CHECK_EQ_UINT(gf::subtract(0x53, 0xCA), 0x99);
    ISO_CHECK_EQ_UINT(gf::add(0xAB, 0xAB), 0x00);

    // Multiplication in the default (0x11D) field.
    ISO_CHECK_EQ_UINT(gf::multiply(3, 7), 9);
    ISO_CHECK_EQ_UINT(gf::multiply(2, 0x80), 0x1D);
    ISO_CHECK_EQ_UINT(gf::multiply(0, 0x42), 0);
    ISO_CHECK_EQ_UINT(gf::multiply(1, 0x42), 0x42);

    // Power.
    ISO_CHECK_EQ_UINT(gf::power(2, 8), 0x1D);
    ISO_CHECK_EQ_UINT(gf::power(0x42, 0), 1);
    ISO_CHECK_EQ_UINT(gf::power(0, 0), 1);
    ISO_CHECK_EQ_UINT(gf::power(0, 5), 0);

    // Inverse round-trip.
    {
        bool ok = true;
        for (int a = 1; a < 256; a++) {
            std::uint8_t inv = gf::inverse(static_cast<std::uint8_t>(a));
            if (gf::multiply(static_cast<std::uint8_t>(a), inv) != 1) {
                ok = false;
            }
        }
        ISO_CHECK_MSG(ok, "a * inverse(a) == 1 for all non-zero a");
    }

    // Division inverts multiplication.
    ISO_CHECK_EQ_UINT(gf::divide(9, 3), 7);
    {
        bool ok = true;
        for (int b = 1; b < 256; b += 17) {
            for (int a = 0; a < 256; a += 13) {
                std::uint8_t prod = gf::multiply(static_cast<std::uint8_t>(a),
                                                 static_cast<std::uint8_t>(b));
                if (gf::divide(prod, static_cast<std::uint8_t>(b)) !=
                    static_cast<std::uint8_t>(a)) {
                    ok = false;
                }
            }
        }
        ISO_CHECK_MSG(ok, "divide(multiply(a,b), b) == a");
    }
    ISO_CHECK_EQ_UINT(gf::divide(0x42, 0), 0);
    ISO_CHECK_EQ_UINT(gf::inverse(0), 0);

    // Field(0x11D) matches the module functions.
    {
        gf::Field rs(gf::PRIMITIVE_POLYNOMIAL);
        bool ok = true;
        for (int a = 0; a < 256; a += 11) {
            for (int b = 0; b < 256; b += 11) {
                if (rs.multiply(static_cast<std::uint8_t>(a),
                                static_cast<std::uint8_t>(b)) !=
                    gf::multiply(static_cast<std::uint8_t>(a),
                                 static_cast<std::uint8_t>(b))) {
                    ok = false;
                }
            }
        }
        ISO_CHECK_MSG(ok, "Field(0x11D) matches the module-level multiply");
    }

    // AES field (0x11B): classic FIPS-197 values.
    {
        gf::Field aes(0x11B);
        ISO_CHECK_EQ_UINT(aes.multiply(0x57, 0x83), 0xC1);
        ISO_CHECK_EQ_UINT(aes.multiply(0x53, 0xCA), 0x01);
        ISO_CHECK_EQ_UINT(aes.inverse(0x53), 0xCA);
        ISO_CHECK_EQ_UINT(aes.divide(0x01, 0x53), 0xCA);
        ISO_CHECK_EQ_UINT(aes.power(0x03, 0), 1);
        bool ok = true;
        for (int a = 1; a < 256; a++) {
            std::uint8_t inv = aes.inverse(static_cast<std::uint8_t>(a));
            if (aes.multiply(static_cast<std::uint8_t>(a), inv) != 1) {
                ok = false;
            }
        }
        ISO_CHECK_MSG(ok, "AES-field inverse round-trip");
    }

    return ISO_TEST_RESULT();
}
