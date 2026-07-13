// Tests for the C++ lisp-parser library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <algorithm>
#include <string>
#include <vector>

#include "lisp_parser.hpp"

namespace lp = ca::lisp_parser;
using lp::SExpr;

// All atom values across a program (flatten each top-level form's find_atoms).
static std::vector<std::string> all_atoms(const std::vector<SExpr>& program) {
    std::vector<std::string> out;
    for (const auto& e : program) {
        auto a = e.find_atoms();
        out.insert(out.end(), a.begin(), a.end());
    }
    return out;
}
static std::size_t count_lists(const std::vector<SExpr>& program) {
    std::size_t n = 0;
    for (const auto& e : program) n += e.count_lists();
    return n;
}
static std::size_t count_quoted(const std::vector<SExpr>& program) {
    std::size_t n = 0;
    for (const auto& e : program) n += e.count_quoted();
    return n;
}
static bool contains(const std::vector<std::string>& v, const std::string& s) {
    return std::find(v.begin(), v.end(), s) != v.end();
}

int main() {
    using V = std::vector<std::string>;

    // ── basic structure ──────────────────────────────────────────────────────
    ISO_CHECK(lp::parse("").empty());
    ISO_CHECK(lp::parse("1 2 3").size() == 3);

    // ── atoms ────────────────────────────────────────────────────────────────
    ISO_CHECK((all_atoms(lp::parse("42")) == V{"42"}));
    ISO_CHECK((all_atoms(lp::parse("-7")) == V{"-7"}));
    ISO_CHECK((all_atoms(lp::parse("define")) == V{"define"}));
    ISO_CHECK((all_atoms(lp::parse("+")) == V{"+"}));
    ISO_CHECK(all_atoms(lp::parse("\"hello\"")).size() == 1);

    // ── lists ────────────────────────────────────────────────────────────────
    ISO_CHECK(count_lists(lp::parse("()")) == 1);
    ISO_CHECK((all_atoms(lp::parse("(1 2 3)")) == V{"1", "2", "3"}));
    ISO_CHECK(count_lists(lp::parse("((1 2) (3 4))")) == 3);
    ISO_CHECK((all_atoms(lp::parse("(+ 1 2)")) == V{"+", "1", "2"}));
    ISO_CHECK((all_atoms(lp::parse("(define x 42)")) == V{"define", "x", "42"}));
    ISO_CHECK((all_atoms(lp::parse("(+ (* 2 3) (- 10 4))")) ==
               V{"+", "*", "2", "3", "-", "10", "4"}));

    // ── quoted forms ─────────────────────────────────────────────────────────
    {
        auto p = lp::parse("'foo");
        ISO_CHECK(count_quoted(p) == 1);
        ISO_CHECK((all_atoms(p) == V{"foo"}));
    }
    {
        auto p = lp::parse("'(1 2 3)");
        ISO_CHECK(count_quoted(p) == 1);
        ISO_CHECK((all_atoms(p) == V{"1", "2", "3"}));
    }
    ISO_CHECK(count_quoted(lp::parse("(eq 'foo 'bar)")) == 2);

    // ── dotted pairs ─────────────────────────────────────────────────────────
    {
        auto p = lp::parse("(a . b)");
        ISO_CHECK((all_atoms(p) == V{"a", "b"}));
        ISO_CHECK(p.size() == 1 && p[0].kind() == lp::SExprKind::DottedPair);
    }
    ISO_CHECK((all_atoms(lp::parse("(1 . 2)")) == V{"1", "2"}));

    // ── complex expressions ──────────────────────────────────────────────────
    {
        auto a = all_atoms(lp::parse("(lambda (x) (* x x))"));
        ISO_CHECK(contains(a, "lambda") && contains(a, "x") && contains(a, "*"));
    }
    {
        auto a = all_atoms(lp::parse("(cond ((eq x 0) 1) (t x))"));
        ISO_CHECK(contains(a, "cond") && contains(a, "eq") && contains(a, "t"));
    }
    {
        std::string src = R"(
        (define factorial
          (lambda (n)
            (cond ((eq n 0) 1)
                  (t (* n (factorial (- n 1))))))))";
        auto p = lp::parse(src);
        ISO_CHECK(p.size() == 1);
        auto a = all_atoms(p);
        ISO_CHECK(contains(a, "define") && contains(a, "factorial") &&
                  contains(a, "lambda") && contains(a, "cond"));
    }
    {
        std::string src = R"(
        (define x 10)
        (define y 20)
        (+ x y))";
        ISO_CHECK(lp::parse(src).size() == 3);
    }
    ISO_CHECK((all_atoms(lp::parse("(car (cons 1 2))")) ==
               V{"car", "cons", "1", "2"}));

    // ── error cases ──────────────────────────────────────────────────────────
    {
        bool threw = false;
        try {
            lp::parse("(+ 1 2");  // unmatched '('
        } catch (const lp::ParseError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    {
        bool threw = false;
        try {
            lp::parse(")");  // unexpected ')'
        } catch (const lp::ParseError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
