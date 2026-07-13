/*
 * Tests for lisp-lexer, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests.
 */
#include "iso_test.h"

#include <string.h> /* strcmp, strstr */

#include "lisp_lexer.h"

/* True iff tokenizing `src` yields exactly `want` token types (excluding EOF). */
static int types_eq(const char *src, const LlTokenType *want, size_t nwant) {
    LlTokenList list;
    LlError err;
    if (!ll_tokenize(src, &list, &err)) return 0;
    size_t got = 0;
    for (size_t i = 0; i < list.count; i++)
        if (list.tokens[i].type != LL_EOF) got++;
    int ok = got == nwant;
    size_t j = 0;
    for (size_t i = 0; i < list.count && ok; i++) {
        if (list.tokens[i].type == LL_EOF) continue;
        if (list.tokens[i].type != want[j++]) ok = 0;
    }
    ll_token_list_free(&list);
    return ok;
}

/* True iff tokenizing `src` yields exactly `want` token values (excluding EOF). */
static int values_eq(const char *src, const char *const *want, size_t nwant) {
    LlTokenList list;
    LlError err;
    if (!ll_tokenize(src, &list, &err)) return 0;
    size_t got = 0;
    for (size_t i = 0; i < list.count; i++)
        if (list.tokens[i].type != LL_EOF) got++;
    int ok = got == nwant;
    size_t j = 0;
    for (size_t i = 0; i < list.count && ok; i++) {
        if (list.tokens[i].type == LL_EOF) continue;
        if (strcmp(list.tokens[i].value, want[j++]) != 0) ok = 0;
    }
    ll_token_list_free(&list);
    return ok;
}

int main(void) {
    /* ── basic atoms ─────────────────────────────────────────────────────────*/
    {
        LlTokenType w[] = {LL_NUMBER};
        const char *v[] = {"42"};
        ISO_CHECK(types_eq("42", w, 1));
        ISO_CHECK(values_eq("42", v, 1));
    }
    {
        LlTokenType w[] = {LL_NUMBER};
        const char *v[] = {"-7"};
        ISO_CHECK(types_eq("-7", w, 1));
        ISO_CHECK(values_eq("-7", v, 1));
    }
    {
        LlTokenType w[] = {LL_NUMBER};
        ISO_CHECK(types_eq("0", w, 1));
    }
    {
        LlTokenType w[] = {LL_SYMBOL};
        const char *v[] = {"define"};
        ISO_CHECK(types_eq("define", w, 1));
        ISO_CHECK(values_eq("define", v, 1));
    }
    {
        LlTokenType w[] = {LL_STRING};
        ISO_CHECK(types_eq("\"hello world\"", w, 1));
    }
    { /* string with escaped quotes: "hello \"world\"" */
        LlTokenType w[] = {LL_STRING};
        ISO_CHECK(types_eq("\"hello \\\"world\\\"\"", w, 1));
    }

    /* ── operator symbols ────────────────────────────────────────────────────*/
    {
        LlTokenType w[] = {LL_SYMBOL};
        const char *v[] = {"+"};
        ISO_CHECK(types_eq("+", w, 1));
        ISO_CHECK(values_eq("+", v, 1));
    }
    { /* (- 3 1): the '-' is a Symbol (followed by space, not a digit) */
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_NUMBER, LL_NUMBER,
                           LL_RPAREN};
        ISO_CHECK(types_eq("(- 3 1)", w, 5));
    }
    {
        LlTokenType w[] = {LL_SYMBOL};
        ISO_CHECK(types_eq("*", w, 1));
        ISO_CHECK(types_eq("/", w, 1));
        ISO_CHECK(types_eq("=", w, 1));
    }
    {
        LlTokenType w[] = {LL_SYMBOL, LL_SYMBOL, LL_SYMBOL, LL_SYMBOL};
        ISO_CHECK(types_eq("< > <= >=", w, 4));
    }
    {
        LlTokenType w[] = {LL_SYMBOL};
        const char *v1[] = {"set!"};
        const char *v2[] = {"null?"};
        ISO_CHECK(types_eq("set!", w, 1));
        ISO_CHECK(values_eq("set!", v1, 1));
        ISO_CHECK(types_eq("null?", w, 1));
        ISO_CHECK(values_eq("null?", v2, 1));
    }

    /* ── delimiters ──────────────────────────────────────────────────────────*/
    {
        LlTokenType w[] = {LL_LPAREN, LL_RPAREN};
        ISO_CHECK(types_eq("()", w, 2));
    }
    {
        LlTokenType w[] = {LL_QUOTE, LL_SYMBOL};
        ISO_CHECK(types_eq("'x", w, 2));
    }
    {
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_DOT, LL_SYMBOL, LL_RPAREN};
        ISO_CHECK(types_eq("(a . b)", w, 5));
    }

    /* ── whitespace and comments ─────────────────────────────────────────────*/
    {
        LlTokenType w1[] = {LL_NUMBER};
        LlTokenType w2[] = {LL_SYMBOL, LL_SYMBOL};
        ISO_CHECK(types_eq("  42  ", w1, 1));
        ISO_CHECK(types_eq("a\tb", w2, 2));
        ISO_CHECK(types_eq("a\nb", w2, 2));
    }
    {
        LlTokenType w[] = {LL_NUMBER};
        ISO_CHECK(types_eq("; this is a comment\n42", w, 1));
    }
    {
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_NUMBER, LL_NUMBER,
                           LL_RPAREN};
        ISO_CHECK(types_eq("(+ 1 2) ; add them", w, 5));
    }

    /* ── full expressions ────────────────────────────────────────────────────*/
    {
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_NUMBER, LL_NUMBER,
                           LL_RPAREN};
        ISO_CHECK(types_eq("(+ 1 2)", w, 5));
    }
    {
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_LPAREN, LL_SYMBOL,
                           LL_NUMBER, LL_NUMBER, LL_RPAREN, LL_NUMBER,
                           LL_RPAREN};
        ISO_CHECK(types_eq("(+ (* 2 3) 4)", w, 9));
    }
    {
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_SYMBOL, LL_NUMBER,
                           LL_RPAREN};
        ISO_CHECK(types_eq("(define x 42)", w, 5));
    }
    {
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_LPAREN, LL_SYMBOL,
                           LL_RPAREN, LL_LPAREN, LL_SYMBOL, LL_SYMBOL,
                           LL_SYMBOL, LL_RPAREN, LL_RPAREN};
        ISO_CHECK(types_eq("(lambda (x) (* x x))", w, 11));
    }
    {
        LlTokenType w[] = {LL_QUOTE, LL_SYMBOL};
        ISO_CHECK(types_eq("'foo", w, 2));
    }
    {
        LlTokenType w[] = {LL_QUOTE, LL_LPAREN, LL_NUMBER, LL_NUMBER,
                           LL_NUMBER, LL_RPAREN};
        ISO_CHECK(types_eq("'(1 2 3)", w, 6));
    }
    {
        LlTokenType w[] = {LL_LPAREN, LL_NUMBER, LL_DOT, LL_NUMBER, LL_RPAREN};
        ISO_CHECK(types_eq("(1 . 2)", w, 5));
    }
    { /* (cond ((eq x 0) 1) (t x)) */
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_LPAREN, LL_LPAREN,
                           LL_SYMBOL, LL_SYMBOL, LL_NUMBER, LL_RPAREN,
                           LL_NUMBER, LL_RPAREN, LL_LPAREN, LL_SYMBOL,
                           LL_SYMBOL, LL_RPAREN, LL_RPAREN};
        ISO_CHECK(types_eq("(cond ((eq x 0) 1) (t x))", w, 15));
    }
    { /* factorial: > 20 non-EOF tokens; first three checked directly */
        const char *src =
            "\n        (define factorial\n          (lambda (n)\n            "
            "(cond ((eq n 0) 1)\n                  (t (* n (factorial (- n "
            "1)))))))\n        ";
        LlTokenList list;
        LlError err;
        ISO_CHECK(ll_tokenize(src, &list, &err));
        size_t non_eof = 0;
        for (size_t i = 0; i < list.count; i++)
            if (list.tokens[i].type != LL_EOF) non_eof++;
        ISO_CHECK(non_eof > 20);
        ISO_CHECK(list.tokens[0].type == LL_LPAREN);
        ISO_CHECK_STR_EQ(list.tokens[1].value, "define");
        ISO_CHECK_STR_EQ(list.tokens[2].value, "factorial");
        ll_token_list_free(&list);
    }

    /* ── EOF / empties ───────────────────────────────────────────────────────*/
    {
        LlTokenList list;
        LlError err;
        ISO_CHECK(ll_tokenize("", &list, &err));
        ISO_CHECK(list.count == 1 && list.tokens[0].type == LL_EOF);
        ll_token_list_free(&list);
    }
    {
        LlTokenList list;
        LlError err;
        ISO_CHECK(ll_tokenize("; just a comment\n; another one", &list, &err));
        ISO_CHECK(list.count == 1 && list.tokens[0].type == LL_EOF);
        ll_token_list_free(&list);
    }
    {
        LlTokenList list;
        LlError err;
        ISO_CHECK(ll_tokenize("(+ 1 2)", &list, &err));
        ISO_CHECK(list.tokens[list.count - 1].type == LL_EOF);
        ll_token_list_free(&list);
    }

    /* ── number vs symbol disambiguation ─────────────────────────────────────*/
    {
        LlTokenType w[] = {LL_NUMBER};
        const char *v[] = {"-42"};
        ISO_CHECK(types_eq("-42", w, 1));
        ISO_CHECK(values_eq("-42", v, 1));
    }
    {
        LlTokenType w[] = {LL_LPAREN, LL_SYMBOL, LL_NUMBER, LL_NUMBER,
                           LL_RPAREN};
        const char *v[] = {"(", "-", "3", "1", ")"};
        ISO_CHECK(types_eq("(- 3 1)", w, 5));
        ISO_CHECK(values_eq("(- 3 1)", v, 5));
    }

    /* ── error cases ─────────────────────────────────────────────────────────*/
    {
        LlTokenList list;
        LlError err;
        ISO_CHECK(!ll_tokenize("\"hello", &list, &err));
        ISO_CHECK(strstr(err.message, "Unterminated string") != NULL);
    }
    {
        LlTokenList list;
        LlError err;
        ISO_CHECK(!ll_tokenize("@", &list, &err));
        ISO_CHECK(strstr(err.message, "Unexpected character") != NULL);
    }

    /* ── token type names (Display parity) ───────────────────────────────────*/
    {
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_NUMBER), "NUMBER");
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_SYMBOL), "SYMBOL");
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_STRING), "STRING");
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_LPAREN), "LPAREN");
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_RPAREN), "RPAREN");
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_QUOTE), "QUOTE");
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_DOT), "DOT");
        ISO_CHECK_STR_EQ(ll_token_type_name(LL_EOF), "EOF");
    }

    return ISO_TEST_RESULT();
}
