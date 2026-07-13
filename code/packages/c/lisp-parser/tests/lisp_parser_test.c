/*
 * Tests for lisp-parser, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests.
 */
#include "iso_test.h"

#include <string.h> /* strcmp */

#include "lisp_parser.h"

/* True iff parsing `src` yields exactly the atom values in `want`. */
static int atoms_exact(const char *src, const char *const *want, size_t nwant) {
    LpProgram prog;
    LpError err;
    if (!lp_parse(src, &prog, &err)) return 0;
    LpStrList atoms = lp_program_find_atoms(&prog);
    int ok = atoms.n == nwant;
    for (size_t i = 0; i < nwant && ok; i++)
        if (strcmp(atoms.items[i], want[i]) != 0) ok = 0;
    lp_strlist_free(&atoms);
    lp_program_free(&prog);
    return ok;
}

/* True iff parsing `src` yields an atom list containing `needle`. */
static int atoms_contain(const char *src, const char *needle) {
    LpProgram prog;
    LpError err;
    if (!lp_parse(src, &prog, &err)) return 0;
    LpStrList atoms = lp_program_find_atoms(&prog);
    int found = 0;
    for (size_t i = 0; i < atoms.n; i++)
        if (strcmp(atoms.items[i], needle) == 0) found = 1;
    lp_strlist_free(&atoms);
    lp_program_free(&prog);
    return found;
}

int main(void) {
    /* ── basic structure ─────────────────────────────────────────────────────*/
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("", &prog, &err) && prog.n == 0);
        lp_program_free(&prog);
    }
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("1 2 3", &prog, &err) && prog.n == 3);
        lp_program_free(&prog);
    }

    /* ── atoms ───────────────────────────────────────────────────────────────*/
    {
        const char *w[] = {"42"};
        ISO_CHECK(atoms_exact("42", w, 1));
    }
    {
        const char *w[] = {"-7"};
        ISO_CHECK(atoms_exact("-7", w, 1));
    }
    {
        const char *w[] = {"define"};
        ISO_CHECK(atoms_exact("define", w, 1));
    }
    {
        const char *w[] = {"+"};
        ISO_CHECK(atoms_exact("+", w, 1));
    }
    { /* string: one atom */
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("\"hello\"", &prog, &err));
        LpStrList atoms = lp_program_find_atoms(&prog);
        ISO_CHECK(atoms.n == 1);
        lp_strlist_free(&atoms);
        lp_program_free(&prog);
    }

    /* ── lists ───────────────────────────────────────────────────────────────*/
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("()", &prog, &err));
        ISO_CHECK(lp_program_count_lists(&prog) == 1);
        lp_program_free(&prog);
    }
    {
        const char *w[] = {"1", "2", "3"};
        ISO_CHECK(atoms_exact("(1 2 3)", w, 3));
    }
    { /* nested list: outer + 2 inner = 3 */
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("((1 2) (3 4))", &prog, &err));
        ISO_CHECK(lp_program_count_lists(&prog) == 3);
        lp_program_free(&prog);
    }
    {
        const char *w[] = {"+", "1", "2"};
        ISO_CHECK(atoms_exact("(+ 1 2)", w, 3));
    }
    {
        const char *w[] = {"define", "x", "42"};
        ISO_CHECK(atoms_exact("(define x 42)", w, 3));
    }
    {
        const char *w[] = {"+", "*", "2", "3", "-", "10", "4"};
        ISO_CHECK(atoms_exact("(+ (* 2 3) (- 10 4))", w, 7));
    }

    /* ── quoted forms ────────────────────────────────────────────────────────*/
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("'foo", &prog, &err));
        ISO_CHECK(lp_program_count_quoted(&prog) == 1);
        LpStrList a = lp_program_find_atoms(&prog);
        ISO_CHECK(a.n == 1 && strcmp(a.items[0], "foo") == 0);
        lp_strlist_free(&a);
        lp_program_free(&prog);
    }
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("'(1 2 3)", &prog, &err));
        ISO_CHECK(lp_program_count_quoted(&prog) == 1);
        const char *w[] = {"1", "2", "3"};
        LpStrList a = lp_program_find_atoms(&prog);
        int ok = a.n == 3;
        for (size_t i = 0; i < 3 && ok; i++)
            if (strcmp(a.items[i], w[i]) != 0) ok = 0;
        ISO_CHECK(ok);
        lp_strlist_free(&a);
        lp_program_free(&prog);
    }
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("(eq 'foo 'bar)", &prog, &err));
        ISO_CHECK(lp_program_count_quoted(&prog) == 2);
        lp_program_free(&prog);
    }

    /* ── dotted pairs ────────────────────────────────────────────────────────*/
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse("(a . b)", &prog, &err));
        const char *w[] = {"a", "b"};
        ISO_CHECK(atoms_exact("(a . b)", w, 2));
        ISO_CHECK(prog.n == 1 &&
                  lp_sexpr_kind(prog.exprs[0]) == LP_DOTTED_PAIR);
        lp_program_free(&prog);
    }
    {
        const char *w[] = {"1", "2"};
        ISO_CHECK(atoms_exact("(1 . 2)", w, 2));
    }

    /* ── complex expressions ─────────────────────────────────────────────────*/
    ISO_CHECK(atoms_contain("(lambda (x) (* x x))", "lambda"));
    ISO_CHECK(atoms_contain("(lambda (x) (* x x))", "x"));
    ISO_CHECK(atoms_contain("(lambda (x) (* x x))", "*"));
    ISO_CHECK(atoms_contain("(cond ((eq x 0) 1) (t x))", "cond"));
    ISO_CHECK(atoms_contain("(cond ((eq x 0) 1) (t x))", "eq"));
    ISO_CHECK(atoms_contain("(cond ((eq x 0) 1) (t x))", "t"));
    { /* factorial: one top-level form, contains the key symbols */
        const char *src =
            "\n        (define factorial\n          (lambda (n)\n            "
            "(cond ((eq n 0) 1)\n                  (t (* n (factorial (- n "
            "1)))))))\n        ";
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse(src, &prog, &err) && prog.n == 1);
        LpStrList a = lp_program_find_atoms(&prog);
        int has_define = 0, has_fact = 0, has_lambda = 0, has_cond = 0;
        for (size_t i = 0; i < a.n; i++) {
            if (strcmp(a.items[i], "define") == 0) has_define = 1;
            if (strcmp(a.items[i], "factorial") == 0) has_fact = 1;
            if (strcmp(a.items[i], "lambda") == 0) has_lambda = 1;
            if (strcmp(a.items[i], "cond") == 0) has_cond = 1;
        }
        ISO_CHECK(has_define && has_fact && has_lambda && has_cond);
        lp_strlist_free(&a);
        lp_program_free(&prog);
    }
    { /* multiple top-level definitions */
        const char *src =
            "\n        (define x 10)\n        (define y 20)\n        (+ x y)\n"
            "        ";
        LpProgram prog;
        LpError err;
        ISO_CHECK(lp_parse(src, &prog, &err) && prog.n == 3);
        lp_program_free(&prog);
    }
    {
        const char *w[] = {"car", "cons", "1", "2"};
        ISO_CHECK(atoms_exact("(car (cons 1 2))", w, 4));
    }

    /* ── error cases ─────────────────────────────────────────────────────────*/
    {
        LpProgram prog;
        LpError err;
        ISO_CHECK(!lp_parse("(+ 1 2", &prog, &err)); /* unmatched '(' */
        ISO_CHECK(!lp_parse(")", &prog, &err));      /* unexpected ')' */
    }

    return ISO_TEST_RESULT();
}
