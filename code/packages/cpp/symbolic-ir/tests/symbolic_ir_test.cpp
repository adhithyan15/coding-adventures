// Tests for the C++ symbolic-ir library, using the header-only iso_test.h
// harness (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <cstring>
#include <stdexcept>
#include <vector>

#include "symbolic_ir.hpp"

namespace s = ca::symbolic_ir;
using s::Node;
using s::Kind;

int main() {
    ISO_CHECK_STR_EQ(s::VERSION, "0.2.0");

    // ── rational: reduce, collapse, sign, zero numerator ──────────────────
    {
        Node r = s::rat(2, 4);
        ISO_CHECK(r.kind() == Kind::Rational);
        auto rp = r.rational_parts();
        ISO_CHECK(rp.first == 1 && rp.second == 2);

        Node c = s::rat(6, 3);
        ISO_CHECK(c.kind() == Kind::Integer && c.integer_value() == 2);
        ISO_CHECK(s::rat(10, 5).integer_value() == 2);

        auto np = s::rat(1, -2).rational_parts();
        ISO_CHECK(np.first == -1 && np.second == 2);  // sign -> numerator
        auto nnp = s::rat(-3, -4).rational_parts();
        ISO_CHECK(nnp.first == 3 && nnp.second == 4);

        ISO_CHECK(s::rat(0, 5).kind() == Kind::Integer);
        ISO_CHECK(s::rat(0, 5).integer_value() == 0);
    }

    // zero denominator throws (the Rust panic).
    {
        bool threw = false;
        try {
            (void)s::rat(1, 0);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── standard heads ────────────────────────────────────────────────────
    ISO_CHECK_STR_EQ(s::ADD, "Add");
    ISO_CHECK_STR_EQ(s::MUL, "Mul");
    ISO_CHECK_STR_EQ(s::POW, "Pow");
    ISO_CHECK_STR_EQ(s::SIN, "Sin");
    ISO_CHECK_STR_EQ(s::DEFINE, "Define");
    ISO_CHECK(s::sym(s::COTH) == Node::symbol("Coth"));
    ISO_CHECK(s::sym(s::SECH) == Node::symbol("Sech"));
    ISO_CHECK(s::sym(s::CSCH) == Node::symbol("Csch"));

    // ── equality ──────────────────────────────────────────────────────────
    ISO_CHECK(s::sym("x") == s::sym("x"));
    ISO_CHECK(s::sym("x") != s::sym("X"));  // case-sensitive
    ISO_CHECK(s::integer(42) == s::integer(42));
    ISO_CHECK(s::integer(1) != s::integer(2));
    ISO_CHECK(s::flt(1.0) == s::flt(1.0));
    ISO_CHECK(s::flt(1.0) != s::flt(2.0));
    ISO_CHECK(s::integer(1) != s::flt(1.0));    // different variants
    ISO_CHECK(s::sym("1") != s::integer(1));

    // NaN with identical bits compares equal.
    {
        volatile double zero = 0.0;
        double nan = zero / zero;
        Node n = s::flt(nan);
        ISO_CHECK(n == n);  // same bits
    }

    // ── hash: equal nodes hash equal ──────────────────────────────────────
    ISO_CHECK(s::sym("x").hash() == s::sym("x").hash());
    ISO_CHECK(s::integer(7).hash() == s::integer(7).hash());
    ISO_CHECK(s::flt(2.5).hash() == s::flt(2.5).hash());
    ISO_CHECK(s::rat(1, 2).hash() == s::rat(1, 2).hash());
    ISO_CHECK(s::integer(7).hash() != s::sym("7").hash());  // variant matters

    // ── display ───────────────────────────────────────────────────────────
    ISO_CHECK_STR_EQ(s::sym("x").to_string().c_str(), "x");
    ISO_CHECK_STR_EQ(s::integer(-7).to_string().c_str(), "-7");
    ISO_CHECK_STR_EQ(s::rat(1, 3).to_string().c_str(), "1/3");
    ISO_CHECK_STR_EQ(s::flt(1.5).to_string().c_str(), "1.5");
    ISO_CHECK_STR_EQ(s::flt(3.0).to_string().c_str(), "3.0");
    ISO_CHECK_STR_EQ(s::str_node("hello").to_string().c_str(), "\"hello\"");

    // Apply: Add(x, 1) and nested Pow(x, 2).
    {
        Node e = s::apply(s::sym(s::ADD), {s::sym("x"), s::integer(1)});
        ISO_CHECK_STR_EQ(e.to_string().c_str(), "Add(x, 1)");

        Node p = s::apply(s::sym(s::POW), {s::sym("x"), s::integer(2)});
        ISO_CHECK_STR_EQ(p.to_string().c_str(), "Pow(x, 2)");
    }

    // Doubly-nested: Add(Pow(x, 2), 1) — recursion + equality + accessors.
    {
        Node e = s::apply(
            s::sym(s::ADD),
            {s::apply(s::sym(s::POW), {s::sym("x"), s::integer(2)}),
             s::integer(1)});
        Node e2 = s::apply(
            s::sym(s::ADD),
            {s::apply(s::sym(s::POW), {s::sym("x"), s::integer(2)}),
             s::integer(1)});

        ISO_CHECK(e == e2);
        ISO_CHECK(e.hash() == e2.hash());
        ISO_CHECK_EQ_UINT(e.apply_args().size(), 2u);
        ISO_CHECK(e.apply_args()[0].kind() == Kind::Apply);
        ISO_CHECK_STR_EQ(e.to_string().c_str(), "Add(Pow(x, 2), 1)");
        ISO_CHECK_STR_EQ(e.apply_head().symbol_name().c_str(), "Add");
    }

    return ISO_TEST_RESULT();
}
