/*
 * javascript_tokens.h — shared JavaScript/TypeScript token vocabulary, in pure
 * ISO C17. A faithful port of the Rust `javascript-tokens` crate.
 * ===========================================================================
 *
 * This is the shared vocabulary every layer of a JS toolchain talks in — the
 * lexer, parser, AST, and tooling — without depending on any layer above:
 *
 *   - EsVersion : the ECMAScript editions with a grammar (ES1..ES2025), with
 *                 string round-tripping and a chronological order.
 *   - Span      : a half-open [start, end) byte range within one source file.
 *   - JsTokenKind : the broad classification of a token (Name, Number, Keyword,
 *                 …), plus an Other tag carrying a grammar-specific token name.
 *
 * DIVERGENCE FROM RUST. `EsVersion::from_str` returns a status here (the error
 * text is formatted on demand by `es_version_unknown_message`). The `Other`
 * token name is BORROWED (`const char *`) rather than an owned `String`: a
 * JsTokenKind is a plain trivially-copyable value with no ownership, so the
 * caller keeps the name alive (grammar token names are static in practice).
 *
 * PORTABILITY. Pure ISO C17 — no extensions. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_JAVASCRIPT_TOKENS_H
#define CA_JAVASCRIPT_TOKENS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── EsVersion ──────────────────────────────────────────────────────────── */

/* The ECMAScript editions with a grammar file, in CHRONOLOGICAL order — so a
 * plain integer comparison of two values is a chronological comparison. */
typedef enum {
    ES_ES1 = 0,
    ES_ES3,
    ES_ES5,
    ES_ES2015,
    ES_ES2016,
    ES_ES2017,
    ES_ES2018,
    ES_ES2019,
    ES_ES2020,
    ES_ES2021,
    ES_ES2022,
    ES_ES2023,
    ES_ES2024,
    ES_ES2025
} EsVersion;

#define ES_VERSION_COUNT 14

/* The most recent edition (ES2025) — also the default. */
EsVersion es_version_latest(void);
EsVersion es_version_default(void);

/* The grammar-file basename: "es1", "es3", "es5", "es2015" … "es2025". */
const char *es_version_as_str(EsVersion v);

/* Every version in chronological order; writes the count to *count_out. */
const EsVersion *es_version_all(size_t *count_out);

/* Parse from the same strings `es_version_as_str` emits (no leading/trailing
 * space, exact case). Returns 0 (fills *out) or -1 if unrecognized. The empty
 * string is rejected. */
int es_version_from_str(const char *s, EsVersion *out);

/* Format the "unknown version" error message for a rejected input `bad`:
 *   unknown ECMAScript version "bad"; valid values are "es1", "es3", …
 * Writes into `buf`; returns the length (excluding the NUL) or -1 if truncated. */
int es_version_unknown_message(const char *bad, char *buf, size_t buflen);

/* ── Span ───────────────────────────────────────────────────────────────── */

/* A half-open [start, end) byte range within one source file. `start <= end`
 * is the caller's invariant (not enforced), matching the Rust type. */
typedef struct {
    uint32_t start;
    uint32_t end;
} JsSpan;

JsSpan js_span_new(uint32_t start, uint32_t end);
uint32_t js_span_len(JsSpan s);    /* end - start */
int js_span_is_empty(JsSpan s);    /* start == end */
int js_span_eq(JsSpan a, JsSpan b);
/* Lexicographic compare (start, then end): -1, 0, or +1. */
int js_span_cmp(JsSpan a, JsSpan b);

/* ── TokenKind ──────────────────────────────────────────────────────────── */

/* The broad token classification. `JS_TOK_OTHER` carries a grammar-specific
 * token name (e.g. "OPTIONAL_CHAIN") in the value's `other_name`. */
typedef enum {
    JS_TOK_NAME = 0,
    JS_TOK_NUMBER,
    JS_TOK_STRING,
    JS_TOK_REGEX,
    JS_TOK_TEMPLATE_NO_SUB,
    JS_TOK_TEMPLATE_HEAD,
    JS_TOK_TEMPLATE_MIDDLE,
    JS_TOK_TEMPLATE_TAIL,
    JS_TOK_BIGINT,
    JS_TOK_PRIVATE_NAME,
    JS_TOK_KEYWORD,
    JS_TOK_OPERATOR,
    JS_TOK_PUNCTUATION,
    JS_TOK_COMMENT,
    JS_TOK_WHITESPACE,
    JS_TOK_NEWLINE,
    JS_TOK_HASHBANG,
    JS_TOK_ERROR,
    JS_TOK_EOF,
    JS_TOK_OTHER
} JsTokenKindTag;

/* A token kind value: a tag, plus a borrowed name when the tag is
 * JS_TOK_OTHER (NULL otherwise). Trivially copyable; no ownership. */
typedef struct {
    JsTokenKindTag tag;
    const char *other_name;
} JsTokenKind;

/* A non-Other token kind (other_name NULL). */
JsTokenKind js_token_kind(JsTokenKindTag tag);
/* An Other token kind wrapping the (borrowed) grammar token `name`. */
JsTokenKind js_token_kind_other(const char *name);

/* Trivia — Comment / Whitespace / Newline (a hint, not a hard rule). */
int js_token_kind_is_trivia(JsTokenKind k);
/* The end-of-input sentinel. */
int js_token_kind_is_eof(JsTokenKind k);
/* Equality: tags equal, and for JS_TOK_OTHER the names equal too. */
int js_token_kind_eq(JsTokenKind a, JsTokenKind b);

#ifdef __cplusplus
}
#endif

#endif /* CA_JAVASCRIPT_TOKENS_H */
