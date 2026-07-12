/*
 * javascript_tokens.c — implementation of the shared JS token vocabulary.
 * ===========================================================================
 *
 * Pure lookup tables and small comparisons — no allocation. The EsVersion
 * enum values are laid out in chronological order, so its ordering is just the
 * integer ordering; the string table and the ALL array share that order.
 */
#include "javascript_tokens.h"

#include <string.h>

/* ── EsVersion ──────────────────────────────────────────────────────────── */

/* Indexed by the EsVersion enum value (chronological). */
static const char *const ES_STRINGS[ES_VERSION_COUNT] = {
    "es1",    "es3",    "es5",    "es2015", "es2016", "es2017", "es2018",
    "es2019", "es2020", "es2021", "es2022", "es2023", "es2024", "es2025"};

static const EsVersion ES_ALL[ES_VERSION_COUNT] = {
    ES_ES1,    ES_ES3,    ES_ES5,    ES_ES2015, ES_ES2016, ES_ES2017, ES_ES2018,
    ES_ES2019, ES_ES2020, ES_ES2021, ES_ES2022, ES_ES2023, ES_ES2024, ES_ES2025};

EsVersion es_version_latest(void) { return ES_ES2025; }
EsVersion es_version_default(void) { return es_version_latest(); }

const char *es_version_as_str(EsVersion v) {
    /* Defensive bound: a valid EsVersion is always in range. */
    if ((int)v < 0 || (int)v >= ES_VERSION_COUNT) return "";
    return ES_STRINGS[v];
}

const EsVersion *es_version_all(size_t *count_out) {
    if (count_out) *count_out = ES_VERSION_COUNT;
    return ES_ALL;
}

int es_version_from_str(const char *s, EsVersion *out) {
    if (!s) return -1;
    for (int i = 0; i < ES_VERSION_COUNT; i++) {
        if (strcmp(s, ES_STRINGS[i]) == 0) {
            *out = ES_ALL[i];
            return 0;
        }
    }
    return -1; /* empty string and anything else fall through to here */
}

/* Append `s` at *need, writing only while a byte plus the final NUL still fit;
 * *need always tracks the full required length so truncation is detectable. */
static void msg_put(char *buf, size_t buflen, size_t *need, const char *s) {
    for (size_t i = 0; s[i]; i++) {
        if (*need + 1 < buflen) buf[*need] = s[i];
        (*need)++;
    }
}

int es_version_unknown_message(const char *bad, char *buf, size_t buflen) {
    if (!bad) bad = "";
    size_t need = 0;
    msg_put(buf, buflen, &need, "unknown ECMAScript version \"");
    msg_put(buf, buflen, &need, bad);
    msg_put(buf, buflen, &need, "\"; valid values are ");
    for (int i = 0; i < ES_VERSION_COUNT; i++) {
        if (i) msg_put(buf, buflen, &need, ", ");
        msg_put(buf, buflen, &need, "\"");
        msg_put(buf, buflen, &need, ES_STRINGS[i]);
        msg_put(buf, buflen, &need, "\"");
    }
    if (buflen > 0) buf[need < buflen ? need : buflen - 1] = '\0';
    if (need >= buflen) return -1;            /* truncated */
    if (need > (size_t)2147483647) return -1; /* unrepresentable as int */
    return (int)need;
}

/* ── Span ───────────────────────────────────────────────────────────────── */

JsSpan js_span_new(uint32_t start, uint32_t end) {
    JsSpan s;
    s.start = start;
    s.end = end;
    return s;
}

uint32_t js_span_len(JsSpan s) { return s.end - s.start; }
int js_span_is_empty(JsSpan s) { return s.start == s.end; }
int js_span_eq(JsSpan a, JsSpan b) {
    return a.start == b.start && a.end == b.end;
}

int js_span_cmp(JsSpan a, JsSpan b) {
    if (a.start != b.start) return a.start < b.start ? -1 : 1;
    if (a.end != b.end) return a.end < b.end ? -1 : 1;
    return 0;
}

/* ── TokenKind ──────────────────────────────────────────────────────────── */

JsTokenKind js_token_kind(JsTokenKindTag tag) {
    JsTokenKind k;
    k.tag = tag;
    k.other_name = NULL;
    return k;
}

JsTokenKind js_token_kind_other(const char *name) {
    JsTokenKind k;
    k.tag = JS_TOK_OTHER;
    k.other_name = name;
    return k;
}

int js_token_kind_is_trivia(JsTokenKind k) {
    return k.tag == JS_TOK_COMMENT || k.tag == JS_TOK_WHITESPACE ||
           k.tag == JS_TOK_NEWLINE;
}

int js_token_kind_is_eof(JsTokenKind k) { return k.tag == JS_TOK_EOF; }

int js_token_kind_eq(JsTokenKind a, JsTokenKind b) {
    if (a.tag != b.tag) return 0;
    if (a.tag == JS_TOK_OTHER) {
        const char *an = a.other_name ? a.other_name : "";
        const char *bn = b.other_name ? b.other_name : "";
        return strcmp(an, bn) == 0;
    }
    return 1;
}
