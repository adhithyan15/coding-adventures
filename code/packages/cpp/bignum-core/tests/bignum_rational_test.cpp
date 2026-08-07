// Tests for the C++ BigRational, using the header-only iso_test.h harness (pure
// ISO). Vectors mirror the Rust crate's own unit tests — canonical/lowest-terms
// form, sign placement, exact add/sub/mul/div (including big operands pinned
// against Python's fractions.Fraction), ordering, reciprocal, integer powers
// (with the try_pow DoS guard), parsing, and the lossy f64 export.
#include "iso_test.h"

#include <exception>
#include <optional>
#include <string>

#include "bignum_core.hpp"
#include "bignum_rational.hpp"

using ca::BigInteger;
using ca::BigRational;
using ca::ParseRatioError;
using ca::PowTooLargeError;

static BigRational r(std::int64_t n, std::int64_t d) {
    return BigRational::from_ints(n, d);
}
static BigRational p(const std::string& s) { return BigRational::parse(s); }

int main() {
    // ── canonical form: lowest terms, sign in numerator, zero == 0/1 ─────
    ISO_CHECK_STR_EQ(r(50, 100).to_string().c_str(), "1/2");
    ISO_CHECK_STR_EQ(r(462, 1071).to_string().c_str(), "22/51"); // gcd 21
    ISO_CHECK_STR_EQ(r(6, 3).to_string().c_str(), "2");
    ISO_CHECK_STR_EQ(r(3, -4).to_string().c_str(), "-3/4");
    ISO_CHECK_STR_EQ(r(-3, -4).to_string().c_str(), "3/4");
    ISO_CHECK_STR_EQ(r(-3, 4).to_string().c_str(), "-3/4");
    ISO_CHECK_STR_EQ(r(0, 5).to_string().c_str(), "0");
    ISO_CHECK_STR_EQ(r(0, -7).denominator().to_string().c_str(), "1");
    ISO_CHECK(r(0, 5).is_zero());
    // Different spellings are equal by value.
    ISO_CHECK(r(2, 4) == r(1, 2));
    ISO_CHECK(r(10, 20) == r(1, 2));
    ISO_CHECK(r(1, 2) != r(1, 3));

    // ── zero denominator is rejected, not accepted ──────────────────────
    ISO_CHECK(!BigRational::checked_make(BigInteger::one(), BigInteger::zero())
                   .has_value());
    ISO_CHECK(BigRational::checked_make(BigInteger::one(),
                                        BigInteger::from_i64(2))
                  .has_value());
    {
        bool threw = false;
        try {
            (void)BigRational::make(BigInteger::one(), BigInteger::zero());
        } catch (const std::domain_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── small exact arithmetic ───────────────────────────────────────────
    ISO_CHECK_STR_EQ((r(1, 3) + r(1, 6)).to_string().c_str(), "1/2");
    ISO_CHECK_STR_EQ((r(2, 7) * r(14, 3)).to_string().c_str(), "4/3");
    ISO_CHECK_STR_EQ((r(355, 113) - r(22, 7)).to_string().c_str(), "-1/791");
    ISO_CHECK_STR_EQ((r(1, 2) / r(3, 4)).to_string().c_str(), "2/3");
    ISO_CHECK_STR_EQ((r(1, 10) + r(2, 10)).to_string().c_str(), "3/10");
    // Operator and method forms agree.
    ISO_CHECK(r(1, 3).add(r(1, 6)) == (r(1, 3) + r(1, 6)));
    ISO_CHECK(-r(3, 4) == r(-3, 4));

    // ── big operands, pinned against Python's fractions.Fraction ─────────
    {
        BigRational a =
            p("1000000000000000000000000000001/100000000000000000000");
        BigRational b = p("6366805760909027985741435139224001/847288609443");

        ISO_CHECK_STR_EQ(
            (a + b).to_string().c_str(),
            "636680576091750087183586513922400100000000847288609443/"
            "84728860944300000000000000000000");
        ISO_CHECK_STR_EQ(
            (a - b).to_string().c_str(),
            "-636680576090055509964700513922400099999999152711390557/"
            "84728860944300000000000000000000");
        ISO_CHECK_STR_EQ(
            (a * b).to_string().c_str(),
            "6366805760909027985741435139230367805760909027985741435139224001/"
            "84728860944300000000000000000000");
        ISO_CHECK_STR_EQ(
            (a / b).to_string().c_str(),
            "847288609443000000000000000000847288609443/"
            "636680576090902798574143513922400100000000000000000000");
        ISO_CHECK_STR_EQ(
            a.pow(3).to_string().c_str(),
            "1000000000000000000000000000003000000000000000000000000000003000000"
            "000000000000000000000001/"
            "1000000000000000000000000000000000000000000000000000000000000");
        ISO_CHECK_STR_EQ(
            b.pow(-2).to_string().c_str(),
            "717897987691852588770249/"
            "40536215597144386832065866109016673800875222251012083746192454448001");
    }

    // ── ordering ─────────────────────────────────────────────────────────
    ISO_CHECK(r(22, 7) > r(355, 113)); // 3.142857… > 3.14159…
    ISO_CHECK(r(-1, 3) > r(-1, 2));    // -0.333… > -0.5
    ISO_CHECK_EQ_INT(r(2, 4).cmp(r(1, 2)), 0);

    // ── sign, reciprocal, predicates ─────────────────────────────────────
    ISO_CHECK_STR_EQ(r(-3, 4).abs().to_string().c_str(), "3/4");
    ISO_CHECK_EQ_INT(r(-3, 4).signum(), -1);
    ISO_CHECK_EQ_INT(r(0, 1).signum(), 0);
    ISO_CHECK_EQ_INT(r(3, 4).signum(), 1);
    ISO_CHECK(r(-3, 4).is_negative());
    ISO_CHECK(r(3, 4).is_positive());
    ISO_CHECK(r(6, 3).is_integer());
    ISO_CHECK(!r(1, 2).is_integer());
    ISO_CHECK_STR_EQ(r(-3, 4).recip().to_string().c_str(), "-4/3");
    ISO_CHECK_STR_EQ(r(7, 1).recip().to_string().c_str(), "1/7");
    ISO_CHECK(!r(0, 1).checked_recip().has_value());
    {
        bool threw = false;
        try {
            (void)r(0, 1).recip();
        } catch (const std::domain_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    {
        bool threw = false;
        try {
            (void)(r(1, 2) / r(0, 1));
        } catch (const std::domain_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    ISO_CHECK(!r(1, 2).checked_div(r(0, 1)).has_value());

    // ── pow: positive, negative, zero exponents ──────────────────────────
    ISO_CHECK_STR_EQ(r(2, 3).pow(0).to_string().c_str(), "1");
    ISO_CHECK_STR_EQ(r(2, 3).pow(3).to_string().c_str(), "8/27");
    ISO_CHECK_STR_EQ(r(2, 3).pow(-3).to_string().c_str(), "27/8");
    ISO_CHECK_STR_EQ(r(-2, 3).pow(2).to_string().c_str(), "4/9");
    ISO_CHECK_STR_EQ(r(-2, 3).pow(3).to_string().c_str(), "-8/27");
    ISO_CHECK_STR_EQ(r(0, 1).pow(5).to_string().c_str(), "0");
    {
        bool threw = false;
        try {
            (void)r(0, 1).pow(-2); // 1/0
        } catch (const std::domain_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── try_pow guards oversized results ─────────────────────────────────
    ISO_CHECK_STR_EQ(r(2, 1).try_pow(10, 64).to_string().c_str(), "1024");
    {
        bool threw = false;
        try {
            (void)r(10, 3).try_pow(1000000, 4096);
        } catch (const PowTooLargeError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    {
        bool threw = false;
        try {
            (void)r(10, 3).try_pow(-1000000, 4096);
        } catch (const PowTooLargeError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── parsing & display round-trips ────────────────────────────────────
    ISO_CHECK_STR_EQ(p("22/7").to_string().c_str(), "22/7");
    ISO_CHECK_STR_EQ(p("-3/4").to_string().c_str(), "-3/4");
    ISO_CHECK_STR_EQ(p("5").to_string().c_str(), "5");
    ISO_CHECK_STR_EQ(p("0").to_string().c_str(), "0");
    ISO_CHECK_STR_EQ(p("42").to_string().c_str(), "42");      // bare int → n/1
    ISO_CHECK_STR_EQ(p("50/100").to_string().c_str(), "1/2"); // normalized
    ISO_CHECK_STR_EQ(p("3/-4").to_string().c_str(), "-3/4");
    ISO_CHECK_STR_EQ(p("1/1000000000000000000000").to_string().c_str(),
                     "1/1000000000000000000000");

    // ── parse errors are typed ───────────────────────────────────────────
    {
        using K = ParseRatioError::Kind;
        struct {
            const char* in;
            K kind;
        } cases[] = {
            {"", K::Empty},
            {"/3", K::Empty},
            {"5/", K::Empty},
            {"1/2/3", K::TooManySlashes},
            {"5/0", K::ZeroDenominator},
            {"x/2", K::InvalidInteger},
            {"1/y", K::InvalidInteger},
        };
        for (auto& c : cases) {
            ISO_CHECK(!BigRational::try_parse(c.in).has_value());
            bool threw = false;
            try {
                (void)BigRational::parse(c.in);
            } catch (const ParseRatioError& e) {
                threw = true;
                ISO_CHECK(e.kind() == c.kind);
            }
            ISO_CHECK(threw);
        }
    }

    // ── conversions & constants ──────────────────────────────────────────
    ISO_CHECK_STR_EQ(BigRational::from_i64(5).to_string().c_str(), "5");
    ISO_CHECK_STR_EQ(BigRational::from_u64(5).to_string().c_str(), "5");
    ISO_CHECK_STR_EQ(
        BigRational::from_integer(BigInteger::from_i64(9)).to_string().c_str(),
        "9");
    ISO_CHECK_STR_EQ(BigRational::one().to_string().c_str(), "1");
    ISO_CHECK_STR_EQ(BigRational::zero().to_string().c_str(), "0");

    // ── lossy f64 export ─────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(r(0, 1).to_f64(), 0.0, 0.0);
    ISO_CHECK_EQ_DBL(r(1, 2).to_f64(), 0.5, 0.0);
    ISO_CHECK_EQ_DBL(r(-3, 4).to_f64(), -0.75, 0.0);
    ISO_CHECK_EQ_DBL(r(10, 1).to_f64(), 10.0, 0.0);
    ISO_CHECK_EQ_DBL(r(160, 7).to_f64(), 160.0 / 7.0, 0.0);
    ISO_CHECK_EQ_DBL(r(1, 3).to_f64(), 1.0 / 3.0, 0.0);
    ISO_CHECK_EQ_DBL(r(2, 3).to_f64(), 2.0 / 3.0, 0.0);
    {
        // Extreme magnitudes narrow cleanly (saturate / underflow), no crash.
        BigRational huge =
            BigRational::from_integer(BigInteger::from_i64(10).pow(400));
        ISO_CHECK(huge.to_f64() > 1e308); // +inf
        ISO_CHECK_EQ_DBL(huge.recip().to_f64(), 0.0, 0.0); // 10^-400
    }

    return ISO_TEST_RESULT();
}
