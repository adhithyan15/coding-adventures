// Tests for the C++ bignum-core, using the header-only iso_test.h harness (pure
// ISO). The known "big" values are cross-checked against Python's
// arbitrary-precision integers, matching the Rust crate's oracle tests.
#include "iso_test.h"

#include <stdexcept>
#include <string>

#include "bignum_core.hpp"

using ca::BigInteger;

int main() {
    // ── construction / to_string / signum ────────────────────────────────
    {
        ISO_CHECK(BigInteger::from_u64(255).to_string() == "255");
        ISO_CHECK(BigInteger::from_i64(-255).to_string() == "-255");
        ISO_CHECK(BigInteger::zero().to_string() == "0");
        ISO_CHECK(BigInteger::one().to_string() == "1");
        BigInteger n = BigInteger::from_i64(-42);
        ISO_CHECK(n.is_negative() && !n.is_positive());
        ISO_CHECK_EQ_INT(n.signum(), -1);
        ISO_CHECK(n.abs().to_string() == "42");
        ISO_CHECK(BigInteger::zero().is_zero());
        ISO_CHECK_EQ_UINT((unsigned)BigInteger::from_u64(255).bit_len(), 8u);
        ISO_CHECK_EQ_UINT((unsigned)BigInteger::from_u64(256).bit_len(), 9u);
    }

    // ── factorial 50 ─────────────────────────────────────────────────────
    {
        BigInteger acc = BigInteger::one();
        for (int i = 2; i <= 50; ++i) {
            acc = acc * BigInteger::from_i64(i);
        }
        ISO_CHECK(acc.to_string() ==
                  "30414093201713378043612608166064768844377641568960512"
                  "000000000000");
    }

    // ── powers ───────────────────────────────────────────────────────────
    {
        ISO_CHECK(BigInteger::from_u64(2).pow(128).to_string() ==
                  "340282366920938463463374607431768211456");
        ISO_CHECK(BigInteger::from_u64(10).pow(50).to_string() ==
                  "100000000000000000000000000000000000000000000000000");
        ISO_CHECK(BigInteger::from_i64(-2).pow(7).to_string() == "-128");
        ISO_CHECK(BigInteger::from_i64(-2).pow(8).to_string() == "256");
        ISO_CHECK(BigInteger::zero().pow(0).to_string() == "1");
        ISO_CHECK(BigInteger::zero().pow(5).to_string() == "0");
    }

    // ── Python-oracle: mul / div_rem / gcd / radix ───────────────────────
    {
        BigInteger a = BigInteger::parse_radix(
            "123456789012345678901234567890123456789", 10);
        BigInteger b =
            BigInteger::parse_radix("98765432109876543210987654321", 10);

        ISO_CHECK((a * b).to_string() ==
                  "121932631137021795226185032733744855963362292333223746"
                  "38011112635269");

        auto qr = a.div_rem(b);
        ISO_CHECK(qr.first.to_string() == "1249999988");
        ISO_CHECK(qr.second.to_string() == "60185185206018518520725308641");
        ISO_CHECK((qr.first * b + qr.second) == a);  // reconstruction

        auto nqr = (-a).div_rem(b);
        ISO_CHECK(nqr.first.to_string() == "-1249999988");
        ISO_CHECK(nqr.second.to_string() == "-60185185206018518520725308641");

        ISO_CHECK(a.gcd(b).to_string() == "9");
        ISO_CHECK(b.to_str_radix(16) == "13f20d9c2fff89d38e1c70cb1");
        ISO_CHECK(b.to_str_radix(36) == "9kpsz865lt7jkxk0gq9");
        ISO_CHECK(BigInteger::parse_radix("13f20d9c2fff89d38e1c70cb1", 16) == b);
    }

    // ── 7^99 and 2^200 in base 36 ────────────────────────────────────────
    {
        ISO_CHECK(BigInteger::from_u64(7).pow(99).to_string() ==
                  "462068072803536855906378252728602401551029028414946485847"
                  "699333055955922805275437143");
        ISO_CHECK(BigInteger::from_u64(2).pow(200).to_str_radix(36) ==
                  "bnklg118comha6gqury14067gur54n8won6guf4");
    }

    // ── radix renderings + parse ─────────────────────────────────────────
    {
        ISO_CHECK(BigInteger::from_u64(255).to_str_radix(2) == "11111111");
        ISO_CHECK(BigInteger::from_i64(-42).to_str_radix(2) == "-101010");
        ISO_CHECK(BigInteger::from_u64(35).to_str_radix(36) == "z");
        ISO_CHECK(BigInteger::parse_radix("FF", 16).to_string() == "255");
        ISO_CHECK(BigInteger::parse_radix("+7B", 16).to_string() == "123");
        ISO_CHECK(BigInteger::parse_radix("-0", 10).is_zero());
    }

    // ── parse errors (throw ParseBigIntError) ────────────────────────────
    {
        auto err_kind = [](const std::string& s, std::uint32_t radix) -> int {
            try {
                BigInteger::parse_radix(s, radix);
            } catch (const ca::ParseBigIntError& e) {
                return (int)e.kind;
            }
            return -1;
        };
        using K = ca::ParseBigIntError::Kind;
        ISO_CHECK_EQ_INT(err_kind("", 10), (int)K::Empty);
        ISO_CHECK_EQ_INT(err_kind("-", 10), (int)K::Empty);
        ISO_CHECK_EQ_INT(err_kind("12x3", 10), (int)K::InvalidDigit);
        ISO_CHECK_EQ_INT(err_kind("102", 2), (int)K::InvalidDigit);
        ISO_CHECK_EQ_INT(err_kind("10", 1), (int)K::InvalidRadix);
        ISO_CHECK_EQ_INT(err_kind("10", 37), (int)K::InvalidRadix);
    }

    // ── div by zero throws; try_pow guards ───────────────────────────────
    {
        bool threw = false;
        try {
            (void)BigInteger::from_u64(5).div_rem(BigInteger::zero());
        } catch (const std::domain_error&) {
            threw = true;
        }
        ISO_CHECK(threw);

        BigInteger two = BigInteger::from_u64(2);
        bool pow_threw = false;
        try {
            (void)two.try_pow(4000000000u, (std::uint64_t)1 << 20);
        } catch (const ca::PowTooLargeError& e) {
            pow_threw = true;
            ISO_CHECK(e.projected_bits > e.max_bits);
        }
        ISO_CHECK(pow_threw);
        ISO_CHECK(two.try_pow(200, 4096) == two.pow(200));
    }

    // ── ordering / operators ─────────────────────────────────────────────
    {
        ISO_CHECK(BigInteger::from_i64(-5) < BigInteger::zero());
        ISO_CHECK(BigInteger::zero() < BigInteger::from_u64(1000000));
        ISO_CHECK(BigInteger::from_i64(7) == BigInteger::from_i64(7));
        ISO_CHECK(BigInteger::from_i64(7) != BigInteger::from_i64(-7));
        ISO_CHECK((BigInteger::from_i64(-5) + BigInteger::from_i64(3)) ==
                  BigInteger::from_i64(-2));
        ISO_CHECK((BigInteger::from_i64(-17) % BigInteger::from_i64(5)) ==
                  BigInteger::from_i64(-2));
    }

    return ISO_TEST_RESULT();
}
