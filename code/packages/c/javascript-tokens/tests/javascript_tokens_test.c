/*
 * Tests for the C javascript-tokens vocabulary, using the header-only iso_test.h
 * harness (pure ISO). Vectors mirror the Rust crate's own unit tests — EsVersion
 * strings/parsing/ordering, the unknown-version message, Span arithmetic and
 * ordering, and TokenKind trivia/eof/equality classification.
 */
#include "iso_test.h"

#include <string.h>

#include "javascript_tokens.h"

int main(void) {
    /* ── EsVersion: latest / default / as_str / ALL ─────────────────────── */
    ISO_CHECK(es_version_latest() == ES_ES2025);
    ISO_CHECK(es_version_default() == es_version_latest());
    ISO_CHECK_STR_EQ(es_version_as_str(ES_ES2020), "es2020");
    ISO_CHECK_STR_EQ(es_version_as_str(ES_ES1), "es1");
    {
        static const char *expected[ES_VERSION_COUNT] = {
            "es1",    "es3",    "es5",    "es2015", "es2016", "es2017", "es2018",
            "es2019", "es2020", "es2021", "es2022", "es2023", "es2024", "es2025"};
        size_t n = 0;
        const EsVersion *all = es_version_all(&n);
        ISO_CHECK_EQ_UINT(n, (unsigned)ES_VERSION_COUNT);
        for (size_t i = 0; i < n; i++) {
            ISO_CHECK_STR_EQ(es_version_as_str(all[i]), expected[i]);
        }
    }

    /* ── round-trip through strings ─────────────────────────────────────── */
    {
        size_t n = 0;
        const EsVersion *all = es_version_all(&n);
        for (size_t i = 0; i < n; i++) {
            EsVersion v;
            ISO_CHECK(es_version_from_str(es_version_as_str(all[i]), &v) == 0);
            ISO_CHECK(v == all[i]);
        }
    }

    /* ── the empty string and unknowns are rejected ─────────────────────── */
    {
        EsVersion v;
        ISO_CHECK(es_version_from_str("", &v) == -1);
        const char *bad[] = {"es2",    "es5.1", "latest",
                             "ES2025", "es2026", " es2025"};
        for (size_t i = 0; i < sizeof bad / sizeof bad[0]; i++) {
            ISO_CHECK_MSG(es_version_from_str(bad[i], &v) == -1, bad[i]);
        }
    }

    /* ── the unknown-version message names the input and the valid set ──── */
    {
        char buf[512];
        int n = es_version_unknown_message("nope", buf, sizeof buf);
        ISO_CHECK(n > 0);
        ISO_CHECK(strstr(buf, "\"nope\"") != NULL);
        ISO_CHECK(strstr(buf, "\"es2025\"") != NULL);
        ISO_CHECK(strstr(buf, "\"es1\"") != NULL);
        /* A too-small buffer reports truncation. */
        char tiny[8];
        ISO_CHECK(es_version_unknown_message("nope", tiny, sizeof tiny) == -1);
    }

    /* ── ordering is chronological (integer order) ──────────────────────── */
    ISO_CHECK(ES_ES1 < ES_ES3);
    ISO_CHECK(ES_ES5 < ES_ES2015);
    ISO_CHECK(ES_ES2015 < ES_ES2025);

    /* ── Span: construction, len, is_empty ──────────────────────────────── */
    {
        JsSpan s = js_span_new(10, 20);
        ISO_CHECK_EQ_UINT(s.start, 10u);
        ISO_CHECK_EQ_UINT(s.end, 20u);
        ISO_CHECK_EQ_UINT(js_span_len(s), 10u);
        ISO_CHECK_EQ_UINT(js_span_len(js_span_new(0, 1)), 1u);
        ISO_CHECK_EQ_UINT(js_span_len(js_span_new(42, 42)), 0u);
        ISO_CHECK(js_span_is_empty(js_span_new(0, 0)));
        ISO_CHECK(js_span_is_empty(js_span_new(99, 99)));
        ISO_CHECK(!js_span_is_empty(js_span_new(0, 1)));
    }

    /* ── Span: equality over both fields ────────────────────────────────── */
    ISO_CHECK(js_span_eq(js_span_new(3, 7), js_span_new(3, 7)));
    ISO_CHECK(!js_span_eq(js_span_new(3, 7), js_span_new(3, 8)));

    /* ── Span: lexicographic ordering (start, then end) ─────────────────── */
    ISO_CHECK(js_span_cmp(js_span_new(0, 5), js_span_new(0, 6)) < 0);
    ISO_CHECK(js_span_cmp(js_span_new(0, 5), js_span_new(1, 2)) < 0);
    ISO_CHECK(js_span_cmp(js_span_new(5, 10), js_span_new(5, 5)) > 0);
    ISO_CHECK(js_span_cmp(js_span_new(3, 7), js_span_new(3, 7)) == 0);

    /* ── TokenKind: is_trivia is exhaustively specified ─────────────────── */
    {
        struct {
            JsTokenKindTag tag;
            int trivia;
        } cases[] = {
            {JS_TOK_NAME, 0},          {JS_TOK_NUMBER, 0},
            {JS_TOK_STRING, 0},        {JS_TOK_REGEX, 0},
            {JS_TOK_TEMPLATE_NO_SUB, 0}, {JS_TOK_TEMPLATE_HEAD, 0},
            {JS_TOK_TEMPLATE_MIDDLE, 0}, {JS_TOK_TEMPLATE_TAIL, 0},
            {JS_TOK_BIGINT, 0},        {JS_TOK_PRIVATE_NAME, 0},
            {JS_TOK_KEYWORD, 0},       {JS_TOK_OPERATOR, 0},
            {JS_TOK_PUNCTUATION, 0},   {JS_TOK_COMMENT, 1},
            {JS_TOK_WHITESPACE, 1},    {JS_TOK_NEWLINE, 1},
            {JS_TOK_HASHBANG, 0},      {JS_TOK_ERROR, 0},
            {JS_TOK_EOF, 0},
        };
        for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
            ISO_CHECK_EQ_INT(js_token_kind_is_trivia(js_token_kind(cases[i].tag)),
                             cases[i].trivia);
        }
        /* Other is never trivia. */
        ISO_CHECK(!js_token_kind_is_trivia(js_token_kind_other("anything")));
    }

    /* ── TokenKind: is_eof only for Eof ─────────────────────────────────── */
    ISO_CHECK(js_token_kind_is_eof(js_token_kind(JS_TOK_EOF)));
    ISO_CHECK(!js_token_kind_is_eof(js_token_kind(JS_TOK_NAME)));
    ISO_CHECK(!js_token_kind_is_eof(js_token_kind(JS_TOK_NEWLINE)));
    ISO_CHECK(!js_token_kind_is_eof(js_token_kind_other("EOF")));

    /* ── TokenKind: equality (including Other by name) ──────────────────── */
    ISO_CHECK(js_token_kind_eq(js_token_kind(JS_TOK_NAME),
                               js_token_kind(JS_TOK_NAME)));
    ISO_CHECK(!js_token_kind_eq(js_token_kind(JS_TOK_NAME),
                                js_token_kind(JS_TOK_NUMBER)));
    ISO_CHECK(js_token_kind_eq(js_token_kind_other("X"),
                               js_token_kind_other("X")));
    ISO_CHECK(!js_token_kind_eq(js_token_kind_other("X"),
                                js_token_kind_other("Y")));
    /* A bare tag and an Other never compare equal. */
    ISO_CHECK(!js_token_kind_eq(js_token_kind(JS_TOK_NAME),
                                js_token_kind_other("Name")));

    return ISO_TEST_RESULT();
}
