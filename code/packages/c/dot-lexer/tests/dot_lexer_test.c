/* Tests for the C dot-lexer, using the header-only iso_test.h harness (pure
 * ISO). Vectors mirror the Rust crate's own unit tests. */
#include "iso_test.h"

#include <string.h> /* strcmp */

#include "dot_lexer.h"

/* Assert the token at `i` has the given kind and value. */
static void chk(const DotLexResult *r, size_t i, DotTokenKind kind,
                const char *value) {
    ISO_CHECK(i < r->ntokens);
    if (i < r->ntokens) {
        ISO_CHECK_EQ_INT((int)r->tokens[i].kind, (int)kind);
        ISO_CHECK(strcmp(r->tokens[i].value, value) == 0);
    }
}

int main(void) {
    /* ── the doc example: digraph G { A -> B } ──────────────────────────── */
    {
        DotLexResult *r = dot_tokenise("digraph G { A -> B }");
        ISO_CHECK(r != NULL);
        ISO_CHECK_EQ_UINT(r->nerrors, 0u);
        ISO_CHECK_EQ_UINT(r->ntokens, 8u);
        chk(r, 0, DOT_DIGRAPH, "");
        chk(r, 1, DOT_ID, "G");
        chk(r, 2, DOT_LBRACE, "");
        chk(r, 3, DOT_ID, "A");
        chk(r, 4, DOT_ARROW, "");
        chk(r, 5, DOT_ID, "B");
        chk(r, 6, DOT_RBRACE, "");
        chk(r, 7, DOT_EOF, "");
        /* line/col of the first token. */
        ISO_CHECK(r->tokens[0].line == 1 && r->tokens[0].col == 1);
        dot_lex_result_free(r);
    }

    /* ── keywords are case-insensitive ──────────────────────────────────── */
    {
        DotLexResult *r = dot_tokenise("STRICT Graph node EDGE SubGraph");
        chk(r, 0, DOT_STRICT, "");
        chk(r, 1, DOT_GRAPH, "");
        chk(r, 2, DOT_NODE, "");
        chk(r, 3, DOT_EDGE, "");
        chk(r, 4, DOT_SUBGRAPH, "");
        chk(r, 5, DOT_EOF, "");
        dot_lex_result_free(r);
    }

    /* ── all punctuation and the -- edge operator ───────────────────────── */
    {
        DotLexResult *r = dot_tokenise("{}[]=;,: A -- B");
        chk(r, 0, DOT_LBRACE, "");
        chk(r, 1, DOT_RBRACE, "");
        chk(r, 2, DOT_LBRACKET, "");
        chk(r, 3, DOT_RBRACKET, "");
        chk(r, 4, DOT_EQUALS, "");
        chk(r, 5, DOT_SEMICOLON, "");
        chk(r, 6, DOT_COMMA, "");
        chk(r, 7, DOT_COLON, "");
        chk(r, 8, DOT_ID, "A");
        chk(r, 9, DOT_DASHDASH, "");
        chk(r, 10, DOT_ID, "B");
        dot_lex_result_free(r);
    }

    /* ── numerals: -42, 3.14, .5 ────────────────────────────────────────── */
    {
        DotLexResult *r = dot_tokenise("-42 3.14 .5");
        chk(r, 0, DOT_ID, "-42");
        chk(r, 1, DOT_ID, "3.14");
        chk(r, 2, DOT_ID, ".5");
        chk(r, 3, DOT_EOF, "");
        ISO_CHECK_EQ_UINT(r->nerrors, 0u);
        dot_lex_result_free(r);
    }

    /* ── quoted string: delimiters stripped, escapes resolved ───────────── */
    {
        DotLexResult *r = dot_tokenise("\"hello world\" \"a\\\"b\" \"x\\ny\"");
        chk(r, 0, DOT_ID, "hello world");
        chk(r, 1, DOT_ID, "a\"b");
        chk(r, 2, DOT_ID, "x\ny");
        ISO_CHECK_EQ_UINT(r->nerrors, 0u);
        dot_lex_result_free(r);
    }

    /* ── HTML string with balanced angle brackets ───────────────────────── */
    {
        DotLexResult *r = dot_tokenise("<hello> <<b>x</b>>");
        chk(r, 0, DOT_ID, "hello");
        chk(r, 1, DOT_ID, "<b>x</b>");
        dot_lex_result_free(r);
    }

    /* ── comments (line and block) are skipped ──────────────────────────── */
    {
        DotLexResult *r = dot_tokenise("a // line comment\nb /* block */ c");
        chk(r, 0, DOT_ID, "a");
        chk(r, 1, DOT_ID, "b");
        chk(r, 2, DOT_ID, "c");
        chk(r, 3, DOT_EOF, "");
        dot_lex_result_free(r);
    }

    /* ── error recovery: unexpected char, unterminated string ───────────── */
    {
        DotLexResult *r = dot_tokenise("a @ b");
        ISO_CHECK_EQ_UINT(r->nerrors, 1u); /* '@' is unexpected */
        chk(r, 0, DOT_ID, "a");
        chk(r, 1, DOT_ID, "b"); /* recovered and continued */
        dot_lex_result_free(r);
    }
    {
        DotLexResult *r = dot_tokenise("\"unterminated");
        ISO_CHECK_EQ_UINT(r->nerrors, 1u);
        chk(r, 0, DOT_ID, "unterminated"); /* partial value still emitted */
        dot_lex_result_free(r);
    }
    {
        DotLexResult *r = dot_tokenise("x /* unterminated");
        ISO_CHECK_EQ_UINT(r->nerrors, 1u);
        dot_lex_result_free(r);
    }

    /* ── empty input yields just EOF ────────────────────────────────────── */
    {
        DotLexResult *r = dot_tokenise("");
        ISO_CHECK_EQ_UINT(r->ntokens, 1u);
        chk(r, 0, DOT_EOF, "");
        dot_lex_result_free(r);
    }

    return ISO_TEST_RESULT();
}
