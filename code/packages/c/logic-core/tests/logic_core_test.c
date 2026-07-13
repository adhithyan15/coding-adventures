/*
 * Tests for logic-core, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests.
 */
#include "iso_test.h"

#include <stdlib.h> /* free */
#include <string.h> /* strcmp */

#include "logic_core.h"

/* Display `t`, compare to `expected`, free both. Returns 1 on match. */
static int disp_eq(LcTerm *t, const char *expected) {
    char *s = lc_term_to_string(t);
    int ok = s != NULL && strcmp(s, expected) == 0;
    free(s);
    lc_term_free(t);
    return ok;
}

/* Walk `v` under `s` and compare to owned `expected`; frees the walk result and
 * `expected`. */
static int walk_is(const LcSubst *s, LcVar v, LcTerm *expected) {
    LcTerm *w = lc_subst_walk_var(s, v);
    int ok = lc_term_equal(w, expected);
    lc_term_free(w);
    lc_term_free(expected);
    return ok;
}

int main(void) {
    /* ── term construction & display ─────────────────────────────────────────*/
    ISO_CHECK(disp_eq(lc_atom("homer"), "homer"));

    { /* int and float are distinct terms, both display as "1" */
        LcTerm *a = lc_int(1);
        LcTerm *b = lc_float(1.0);
        ISO_CHECK(!lc_term_equal(a, b));
        lc_term_free(a);
        lc_term_free(b);
        ISO_CHECK(disp_eq(lc_int(1), "1"));
        ISO_CHECK(disp_eq(lc_float(1.0), "1"));
    }

    ISO_CHECK(disp_eq(lc_string("hello world"), "\"hello world\""));

    { /* fresh variables have distinct ids */
        LcVar x = lc_var_fresh("X");
        LcVar y = lc_var_fresh("X"); /* same name, different identity */
        ISO_CHECK(x.id != y.id);
    }

    { /* compound displays in functional form */
        LcTerm *args[2] = {lc_atom("homer"), lc_atom("bart")};
        ISO_CHECK(disp_eq(lc_compound("father", args, 2), "father(homer, bart)"));
    }

    { /* logic_list uses cons-cell encoding: .(a, .(b, [])) */
        LcTerm *items[2] = {lc_atom("a"), lc_atom("b")};
        ISO_CHECK(disp_eq(lc_logic_list(items, 2), ".(a, .(b, []))"));
    }

    /* ── unification ─────────────────────────────────────────────────────────*/
    { /* two identical atoms unify without new bindings */
        LcSubst *empty = lc_subst_empty();
        LcTerm *a = lc_atom("a"), *b = lc_atom("a");
        LcSubst *s = lc_unify(a, b, empty);
        ISO_CHECK(s != NULL && lc_subst_equal(s, empty));
        lc_term_free(a);
        lc_term_free(b);
        lc_subst_free(s);
        lc_subst_free(empty);
    }

    { /* two different atoms fail */
        LcSubst *empty = lc_subst_empty();
        LcTerm *a = lc_atom("a"), *b = lc_atom("b");
        LcSubst *s = lc_unify(a, b, empty);
        ISO_CHECK(s == NULL);
        lc_term_free(a);
        lc_term_free(b);
        lc_subst_free(empty);
    }

    { /* variable with atom binds it */
        LcSubst *empty = lc_subst_empty();
        LcVar x = lc_var_fresh("X");
        LcTerm *tx = lc_term_var(x), *homer = lc_atom("homer");
        LcSubst *s = lc_unify(tx, homer, empty);
        ISO_CHECK(s != NULL);
        ISO_CHECK(walk_is(s, x, lc_atom("homer")));
        lc_term_free(tx);
        lc_term_free(homer);
        lc_subst_free(s);
        lc_subst_free(empty);
    }

    { /* compound unifies argument pairs: father(homer,X) ?= father(homer,bart) */
        LcSubst *empty = lc_subst_empty();
        LcVar x = lc_var_fresh("X");
        LcTerm *qa[2] = {lc_atom("homer"), lc_term_var(x)};
        LcTerm *query = lc_compound("father", qa, 2);
        LcTerm *fa[2] = {lc_atom("homer"), lc_atom("bart")};
        LcTerm *fact = lc_compound("father", fa, 2);
        LcSubst *s = lc_unify(query, fact, empty);
        ISO_CHECK(s != NULL);
        ISO_CHECK(walk_is(s, x, lc_atom("bart")));
        lc_term_free(query);
        lc_term_free(fact);
        lc_subst_free(s);
        lc_subst_free(empty);
    }

    { /* mismatched functor fails */
        LcSubst *empty = lc_subst_empty();
        LcTerm *xa[1] = {lc_atom("x")};
        LcTerm *a = lc_compound("p", xa, 1);
        LcTerm *ya[1] = {lc_atom("x")};
        LcTerm *b = lc_compound("q", ya, 1);
        ISO_CHECK(lc_unify(a, b, empty) == NULL);
        lc_term_free(a);
        lc_term_free(b);
        lc_subst_free(empty);
    }

    { /* mismatched arity fails */
        LcSubst *empty = lc_subst_empty();
        LcTerm *xa[1] = {lc_atom("x")};
        LcTerm *a = lc_compound("p", xa, 1);
        LcTerm *ya[2] = {lc_atom("x"), lc_atom("y")};
        LcTerm *b = lc_compound("p", ya, 2);
        ISO_CHECK(lc_unify(a, b, empty) == NULL);
        lc_term_free(a);
        lc_term_free(b);
        lc_subst_free(empty);
    }

    { /* int and float do not unify (distinct ground terms) */
        LcSubst *empty = lc_subst_empty();
        LcTerm *a = lc_int(1), *b = lc_float(1.0);
        ISO_CHECK(lc_unify(a, b, empty) == NULL);
        lc_term_free(a);
        lc_term_free(b);
        lc_subst_free(empty);
    }

    { /* occurs-check prevents cyclic binding: X = f(X) fails */
        LcSubst *empty = lc_subst_empty();
        LcVar x = lc_var_fresh("X");
        LcTerm *tx = lc_term_var(x);
        LcTerm *fa[1] = {lc_term_var(x)};
        LcTerm *cyclic = lc_compound("f", fa, 1);
        ISO_CHECK(lc_unify(tx, cyclic, empty) == NULL);
        lc_term_free(tx);
        lc_term_free(cyclic);
        lc_subst_free(empty);
    }

    { /* two variables become equal */
        LcSubst *empty = lc_subst_empty();
        LcVar x = lc_var_fresh("X"), y = lc_var_fresh("Y");
        LcTerm *tx = lc_term_var(x), *ty = lc_term_var(y);
        LcSubst *s = lc_unify(tx, ty, empty);
        ISO_CHECK(s != NULL);
        LcTerm *wx = lc_subst_walk_var(s, x);
        LcTerm *wy = lc_subst_walk_var(s, y);
        ISO_CHECK(lc_term_equal(wx, wy));
        lc_term_free(wx);
        lc_term_free(wy);
        lc_term_free(tx);
        lc_term_free(ty);
        lc_subst_free(s);
        lc_subst_free(empty);
    }

    /* ── substitution semantics ──────────────────────────────────────────────*/
    { /* extend does not mutate the original */
        LcSubst *s0 = lc_subst_empty();
        LcTerm *a = lc_atom("a");
        LcSubst *s1 = lc_subst_extend(s0, 0, a);
        ISO_CHECK(lc_subst_len(s0) == 0);
        ISO_CHECK(lc_subst_len(s1) == 1);
        lc_term_free(a);
        lc_subst_free(s0);
        lc_subst_free(s1);
    }

    { /* walk through chained bindings reaches the root: X -> Y -> homer */
        LcVar x = lc_var_fresh("X"), y = lc_var_fresh("Y");
        LcSubst *empty = lc_subst_empty();
        LcTerm *ty = lc_term_var(y);
        LcSubst *s1 = lc_subst_extend(empty, x.id, ty);
        LcTerm *homer = lc_atom("homer");
        LcSubst *s = lc_subst_extend(s1, y.id, homer);
        ISO_CHECK(walk_is(s, x, lc_atom("homer")));
        lc_term_free(ty);
        lc_term_free(homer);
        lc_subst_free(empty);
        lc_subst_free(s1);
        lc_subst_free(s);
    }

    return ISO_TEST_RESULT();
}
