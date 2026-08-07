// Tests for the C++ BigDecimal, using the header-only iso_test.h harness (pure
// ISO). Vectors mirror the Rust crate's own unit tests — canonical form,
// display, parsing, exact add/sub/mul, pow, every rounding mode, rounding
// division, ordering, the lossy f64 export, and the MAX_SCALE budget.
#include "iso_test.h"

#include <optional>
#include <string>

#include "bignum_decimal.hpp"
#include "bignum_core.hpp"

using ca::BigDecimal;
using ca::BigInteger;
using ca::ParseDecimalError;
using RM = ca::RoundingMode;

static BigDecimal d(const std::string& s) { return BigDecimal::parse(s); }

int main() {
    // ── canonical form & display ─────────────────────────────────────────
    ISO_CHECK_STR_EQ(d("1.230").to_string().c_str(), "1.23");
    ISO_CHECK_EQ_INT(static_cast<int>(d("1.230").scale()), 2);
    ISO_CHECK_STR_EQ(d("12300").to_string().c_str(), "12300");
    ISO_CHECK_STR_EQ(d("123.45").to_string().c_str(), "123.45");
    ISO_CHECK_STR_EQ(d("0.001").to_string().c_str(), "0.001");
    ISO_CHECK_STR_EQ(d("0.0123").to_string().c_str(), "0.0123");
    ISO_CHECK_STR_EQ(d("-0.5").to_string().c_str(), "-0.5");
    ISO_CHECK_STR_EQ(d("0").to_string().c_str(), "0");
    ISO_CHECK_STR_EQ(d("-0").to_string().c_str(), "0"); // negative zero collapses
    ISO_CHECK_STR_EQ(d("0.00").to_string().c_str(), "0");
    ISO_CHECK_STR_EQ(d("0e5").to_string().c_str(), "0");

    // 100 canonicalizes to mantissa 1, scale -2, but displays "100".
    {
        BigDecimal h = d("100");
        ISO_CHECK_STR_EQ(h.mantissa().to_string().c_str(), "1");
        ISO_CHECK_EQ_INT(static_cast<int>(h.scale()), -2);
        ISO_CHECK_STR_EQ(h.to_string().c_str(), "100");
    }

    // ── from_i64 & from_integer ──────────────────────────────────────────
    ISO_CHECK_STR_EQ(BigDecimal::from_i64(42).to_string().c_str(), "42");
    ISO_CHECK_STR_EQ(BigDecimal::from_i64(-9).to_string().c_str(), "-9");
    ISO_CHECK_STR_EQ(
        BigDecimal::from_integer(BigInteger::from_i64(250)).to_string().c_str(),
        "250");

    // ── parse plain & scientific ─────────────────────────────────────────
    ISO_CHECK_STR_EQ(d("1.5e-3").to_string().c_str(), "0.0015");
    ISO_CHECK_STR_EQ(d("6.022E23").to_string().c_str(),
                     "602200000000000000000000");
    ISO_CHECK_STR_EQ(d("1e3").to_string().c_str(), "1000");
    ISO_CHECK_STR_EQ(d("-0.001").to_string().c_str(), "-0.001");
    ISO_CHECK_STR_EQ(d("+42").to_string().c_str(), "42");
    ISO_CHECK_STR_EQ(d(".5").to_string().c_str(), "0.5");
    ISO_CHECK_STR_EQ(d("5.").to_string().c_str(), "5");

    // ── parse errors are typed (via try_parse and the throwing form) ─────
    {
        using K = ParseDecimalError::Kind;
        ISO_CHECK(!BigDecimal::try_parse("").has_value());
        struct {
            const char* in;
            K kind;
        } cases[] = {
            {"", K::Empty},        {"1.2.3", K::MalformedShape},
            {"1x2", K::InvalidDigit}, {".", K::Empty},
            {"1e", K::InvalidDigit},  {"abc", K::InvalidDigit},
        };
        for (auto& c : cases) {
            bool threw = false;
            try {
                (void)BigDecimal::parse(c.in);
            } catch (const ParseDecimalError& e) {
                threw = true;
                ISO_CHECK(e.kind() == c.kind);
            }
            ISO_CHECK(threw);
        }
    }

    // ── exact +, -, * ────────────────────────────────────────────────────
    ISO_CHECK_STR_EQ((d("0.1") + d("0.2")).to_string().c_str(), "0.3");
    ISO_CHECK_STR_EQ((d("1.23") + d("4.5")).to_string().c_str(), "5.73");
    ISO_CHECK_STR_EQ((d("100") - d("0.01")).to_string().c_str(), "99.99");
    ISO_CHECK_STR_EQ((d("1.5") * d("1.5")).to_string().c_str(), "2.25");
    ISO_CHECK_STR_EQ((d("12345.678") * d("1000")).to_string().c_str(),
                     "12345678");
    ISO_CHECK_STR_EQ((d("-1.5") * d("0.2")).to_string().c_str(), "-0.3");
    // Owned and operator forms agree.
    ISO_CHECK(d("0.1").add(d("0.2")) == (d("0.1") + d("0.2")));
    ISO_CHECK(-d("1.25") == BigDecimal::parse("-1.25"));

    // ── pow is exact ─────────────────────────────────────────────────────
    ISO_CHECK_STR_EQ(d("1.1").pow(2).to_string().c_str(), "1.21");
    ISO_CHECK_STR_EQ(d("2").pow(10).to_string().c_str(), "1024");
    ISO_CHECK_STR_EQ(d("0.5").pow(3).to_string().c_str(), "0.125");
    ISO_CHECK_STR_EQ(d("10").pow(0).to_string().c_str(), "1");

    // ── rounding modes on halves (Python's decimal.quantize truth table) ─
    {
        struct {
            const char* val;
            RM mode;
            const char* want;
        } cases[] = {
            {"2.5", RM::HalfUp, "3"},    {"2.5", RM::HalfEven, "2"},
            {"2.5", RM::HalfDown, "2"},  {"2.5", RM::Down, "2"},
            {"2.5", RM::Up, "3"},        {"2.5", RM::Floor, "2"},
            {"2.5", RM::Ceiling, "3"},   {"-2.5", RM::HalfUp, "-3"},
            {"-2.5", RM::HalfEven, "-2"}, {"-2.5", RM::HalfDown, "-2"},
            {"-2.5", RM::Down, "-2"},    {"-2.5", RM::Up, "-3"},
            {"-2.5", RM::Floor, "-3"},   {"-2.5", RM::Ceiling, "-2"},
        };
        for (auto& c : cases) {
            ISO_CHECK_STR_EQ(
                d(c.val).round_to_scale(0, c.mode).to_string().c_str(), c.want);
        }
    }

    // ── rounding to one place ────────────────────────────────────────────
    ISO_CHECK_STR_EQ(d("1.25").round_to_scale(1, RM::HalfUp).to_string().c_str(),
                     "1.3");
    ISO_CHECK_STR_EQ(
        d("1.25").round_to_scale(1, RM::HalfEven).to_string().c_str(), "1.2");
    ISO_CHECK_STR_EQ(
        d("1.35").round_to_scale(1, RM::HalfEven).to_string().c_str(), "1.4");
    ISO_CHECK(d("1.5").round_to_scale(5, RM::HalfUp) == d("1.5")); // no-op

    // ── rounding division (pinned against Python) ────────────────────────
    {
        struct {
            const char *a, *b;
            std::int64_t scale;
            RM mode;
            const char* want;
        } cases[] = {
            {"10", "3", 4, RM::HalfEven, "3.3333"},
            {"2", "3", 2, RM::HalfUp, "0.67"},
            {"1", "8", 3, RM::HalfEven, "0.125"},
            {"100", "7", 6, RM::Down, "14.285714"},
            {"-10", "3", 2, RM::Floor, "-3.34"},
            {"1", "3", 0, RM::HalfUp, "0"},
            {"1", "4", 10, RM::HalfEven, "0.25"},
        };
        for (auto& c : cases) {
            ISO_CHECK_STR_EQ(
                d(c.a).div_round(d(c.b), c.scale, c.mode).to_string().c_str(),
                c.want);
        }
    }
    // Division by zero: checked returns nullopt; throwing form throws.
    ISO_CHECK(!d("1").checked_div_round(d("0"), 2, RM::HalfUp).has_value());
    ISO_CHECK(d("1").checked_div_round(d("3"), 2, RM::HalfUp).has_value());
    {
        bool threw = false;
        try {
            (void)d("1").div_round(d("0"), 2, RM::HalfUp);
        } catch (const std::domain_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    // An extreme target_scale is rejected up front (the DoS guard), not
    // materialized as a ~gigabyte power of ten.
    {
        bool threw = false;
        try {
            (void)d("1").div_round(d("3"), 2000000000, RM::HalfUp);
        } catch (const std::exception&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    {
        bool threw = false;
        try {
            (void)d("1").round_to_scale(-2000000000, RM::HalfUp);
        } catch (const std::exception&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── ordering & sign ──────────────────────────────────────────────────
    ISO_CHECK(d("0.1") < d("0.2"));
    ISO_CHECK(d("100") > d("99.99"));
    ISO_CHECK(d("-0.5") < d("0"));
    ISO_CHECK(d("1.20") == d("1.2"));
    ISO_CHECK_EQ_INT(d("1.20").cmp(d("1.2")), 0);
    ISO_CHECK_EQ_INT(d("-3.14").signum(), -1);
    ISO_CHECK_EQ_INT(d("0").signum(), 0);
    ISO_CHECK_STR_EQ(d("-3.14").abs().to_string().c_str(), "3.14");
    ISO_CHECK(d("-1").is_negative());
    ISO_CHECK(d("1").is_positive());
    ISO_CHECK(d("0").is_zero());

    // ── lossy f64 export ─────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(d("0.5").to_f64(), 0.5, 0.0);
    ISO_CHECK_EQ_DBL(d("-2.25").to_f64(), -2.25, 0.0);
    ISO_CHECK_EQ_DBL(d("123.456").to_f64(), 123.456, 1e-12);
    ISO_CHECK_EQ_DBL(d("0.1").to_f64(), 0.1, 0.0);

    // ── security: the MAX_SCALE budget rejects amplification payloads ────
    {
        using K = ParseDecimalError::Kind;
        const char* payloads[] = {
            "1e-2000000000", "1e2000000000", "1e99999999999999999999",
            "100e999999", "100e9223372036854775807",
            "1000e9223372036854775806",
        };
        for (const char* p : payloads) {
            ISO_CHECK(!BigDecimal::try_parse(p).has_value());
            bool threw = false;
            try {
                (void)BigDecimal::parse(p);
            } catch (const ParseDecimalError& e) {
                threw = true;
                ISO_CHECK(e.kind() == K::ExponentOverflow);
            }
            ISO_CHECK(threw);
        }
        // A parsed scale exactly at the budget is fine.
        ISO_CHECK(BigDecimal::try_parse("1e-1000000").has_value());
    }

    // ── from_parts enforces the internal ceiling (Rust's checked form) ───
    ISO_CHECK(BigDecimal::checked_from_parts(BigInteger::one(),
                                             BigDecimal::MAX_SCALE + 1)
                  .has_value());
    ISO_CHECK(BigDecimal::checked_from_parts(BigInteger::one(),
                                             BigDecimal::INTERNAL_SCALE_LIMIT)
                  .has_value());
    ISO_CHECK(!BigDecimal::checked_from_parts(BigInteger::one(),
                                              BigDecimal::INTERNAL_SCALE_LIMIT + 1)
                   .has_value());
    ISO_CHECK(!BigDecimal::checked_from_parts(BigInteger::one(), INT64_MIN)
                   .has_value());
    {
        bool threw = false;
        try {
            (void)BigDecimal::from_parts(BigInteger::one(),
                                         BigDecimal::INTERNAL_SCALE_LIMIT + 1);
        } catch (const std::out_of_range&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
