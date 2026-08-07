// Tests for the C++ logic-core library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <string>

#include "logic_core.hpp"

namespace lc = ca::logic_core;
using lc::LogicVar;
using lc::Substitution;
using lc::Term;

int main() {
    // ── term construction & display ──────────────────────────────────────────
    ISO_CHECK(lc::to_string(lc::atom("homer")) == "homer");

    {  // int and float are distinct terms, both display as "1"
        Term a = lc::integer(1);
        Term b = lc::real(1.0);
        ISO_CHECK(a != b);
        ISO_CHECK(lc::to_string(a) == "1");
        ISO_CHECK(lc::to_string(b) == "1");
    }

    ISO_CHECK(lc::to_string(lc::string("hello world")) == "\"hello world\"");

    {  // fresh variables have distinct ids
        LogicVar x = lc::var("X");
        LogicVar y = lc::var("X");  // same name, different identity
        ISO_CHECK(x.id != y.id);
        ISO_CHECK(x != y);
    }

    ISO_CHECK(lc::to_string(lc::compound(
                  "father", {lc::atom("homer"), lc::atom("bart")})) ==
              "father(homer, bart)");

    ISO_CHECK(lc::to_string(lc::logic_list({lc::atom("a"), lc::atom("b")})) ==
              ".(a, .(b, []))");

    // ── unification ──────────────────────────────────────────────────────────
    {  // two identical atoms unify without new bindings
        auto s = lc::unify(lc::atom("a"), lc::atom("a"), Substitution::empty());
        ISO_CHECK(s.has_value() && *s == Substitution::empty());
    }
    {  // two different atoms fail
        auto s = lc::unify(lc::atom("a"), lc::atom("b"), Substitution::empty());
        ISO_CHECK(!s.has_value());
    }
    {  // variable with atom binds it
        LogicVar x = lc::var("X");
        auto s = lc::unify(lc::var_term(x), lc::atom("homer"),
                           Substitution::empty());
        ISO_CHECK(s.has_value());
        ISO_CHECK(s->walk_var(x) == lc::atom("homer"));
    }
    {  // compound unifies argument pairs: father(homer,X) ?= father(homer,bart)
        LogicVar x = lc::var("X");
        Term query = lc::compound("father", {lc::atom("homer"), lc::var_term(x)});
        Term fact = lc::compound("father", {lc::atom("homer"), lc::atom("bart")});
        auto s = lc::unify(query, fact, Substitution::empty());
        ISO_CHECK(s.has_value());
        ISO_CHECK(s->walk_var(x) == lc::atom("bart"));
    }
    {  // mismatched functor fails
        Term a = lc::compound("p", {lc::atom("x")});
        Term b = lc::compound("q", {lc::atom("x")});
        ISO_CHECK(!lc::unify(a, b, Substitution::empty()).has_value());
    }
    {  // mismatched arity fails
        Term a = lc::compound("p", {lc::atom("x")});
        Term b = lc::compound("p", {lc::atom("x"), lc::atom("y")});
        ISO_CHECK(!lc::unify(a, b, Substitution::empty()).has_value());
    }
    {  // int and float do not unify
        ISO_CHECK(!lc::unify(lc::integer(1), lc::real(1.0),
                             Substitution::empty())
                       .has_value());
    }
    {  // occurs-check prevents cyclic binding: X = f(X) fails
        LogicVar x = lc::var("X");
        Term cyclic = lc::compound("f", {lc::var_term(x)});
        ISO_CHECK(!lc::unify(lc::var_term(x), cyclic, Substitution::empty())
                       .has_value());
    }
    {  // two variables become equal
        LogicVar x = lc::var("X"), y = lc::var("Y");
        auto s = lc::unify(lc::var_term(x), lc::var_term(y),
                           Substitution::empty());
        ISO_CHECK(s.has_value());
        ISO_CHECK(s->walk_var(x) == s->walk_var(y));
    }

    // ── substitution semantics ───────────────────────────────────────────────
    {  // extend does not mutate the original
        Substitution s0 = Substitution::empty();
        Substitution s1 = s0.extend(0, lc::atom("a"));
        ISO_CHECK(s0.size() == 0);
        ISO_CHECK(s1.size() == 1);
    }
    {  // walk through chained bindings reaches the root: X -> Y -> homer
        LogicVar x = lc::var("X"), y = lc::var("Y");
        Substitution s = Substitution::empty()
                             .extend(x.id, lc::var_term(y))
                             .extend(y.id, lc::atom("homer"));
        ISO_CHECK(s.walk_var(x) == lc::atom("homer"));
    }

    return ISO_TEST_RESULT();
}
