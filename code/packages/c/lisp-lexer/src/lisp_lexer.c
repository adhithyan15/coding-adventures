/*
 * lisp_lexer.c — implementation of the pure-ISO C Lisp tokenizer.
 * ==============================================================
 *
 * A hand-written scanner: skip whitespace/comments, look at the current byte to
 * choose a token kind, consume the whole token, emit it, repeat. Priority order
 * (whitespace/comments, delimiters, strings, numbers, symbols) resolves every
 * ambiguity, including `-42` (number) vs `-` (symbol).
 */
#include "lisp_lexer.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy */

/* ── Character classes (ASCII, locale-independent — matches Rust) ──────────*/

/* Rust `is_ascii_whitespace`: space, tab, LF, FF, CR (NOT vertical tab). */
static int is_ws(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f';
}
static int is_digit(unsigned char c) { return c >= '0' && c <= '9'; }
static int is_alpha(unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
/* A byte that can start a Lisp symbol: letters, '_', and the operator chars. */
static int is_symbol_start(unsigned char c) {
    return is_alpha(c) || c == '_' || c == '+' || c == '-' || c == '*' ||
           c == '/' || c == '=' || c == '<' || c == '>' || c == '!' ||
           c == '?' || c == '&';
}
/* A byte that can continue a symbol: a symbol-start byte, or a digit. */
static int is_symbol_continue(unsigned char c) {
    return is_symbol_start(c) || is_digit(c);
}

/* ── Token accumulation ────────────────────────────────────────────────────*/

const char *ll_token_type_name(LlTokenType type) {
    switch (type) {
        case LL_NUMBER: return "NUMBER";
        case LL_SYMBOL: return "SYMBOL";
        case LL_STRING: return "STRING";
        case LL_LPAREN: return "LPAREN";
        case LL_RPAREN: return "RPAREN";
        case LL_QUOTE: return "QUOTE";
        case LL_DOT: return "DOT";
        case LL_EOF: return "EOF";
    }
    return "?";
}

/* Append a token copying `len` bytes from `start`. Returns 0 on OOM. */
static int push_tok(LlToken **toks, size_t *count, size_t *cap,
                    LlTokenType type, const char *start, size_t len) {
    if (*count == *cap) {
        size_t nc = *cap ? *cap : 16;
        if (nc > ((size_t)-1) / 2 / sizeof(LlToken)) return 0;
        nc *= 2;
        LlToken *nt = (LlToken *)realloc(*toks, nc * sizeof(LlToken));
        if (nt == NULL) return 0;
        *toks = nt;
        *cap = nc;
    }
    char *v = (char *)malloc(len + 1);
    if (v == NULL) return 0;
    memcpy(v, start, len);
    v[len] = '\0';
    (*toks)[*count].type = type;
    (*toks)[*count].value = v;
    (*count)++;
    return 1;
}

static void free_toks(LlToken *toks, size_t count) {
    for (size_t i = 0; i < count; i++) free(toks[i].value);
    free(toks);
}

static int fail(LlToken *toks, size_t count, LlError *err, const char *message,
                size_t position) {
    free_toks(toks, count);
    /* message is a fixed static string or already-formatted; copy it in. */
    size_t i = 0;
    for (; message[i] != '\0' && i + 1 < sizeof err->message; i++)
        err->message[i] = message[i];
    err->message[i] = '\0';
    err->position = position;
    return 0;
}

/* ── Tokenizer ─────────────────────────────────────────────────────────────*/

int ll_tokenize(const char *source, LlTokenList *out, LlError *err) {
    out->tokens = NULL;
    out->count = 0;

    const size_t n = strlen(source);
    LlToken *toks = NULL;
    size_t count = 0, cap = 0;
    size_t pos = 0;

    while (pos < n) {
        unsigned char c = (unsigned char)source[pos];

        /* Step 1: whitespace and `;` comments. */
        if (is_ws(c)) {
            pos++;
            continue;
        }
        if (c == ';') {
            while (pos < n && source[pos] != '\n') pos++;
            continue;
        }

        /* Step 2: single-character delimiters. */
        if (c == '(' || c == ')' || c == '\'' || c == '.') {
            LlTokenType t = c == '(' ? LL_LPAREN
                            : c == ')' ? LL_RPAREN
                            : c == '\'' ? LL_QUOTE
                            : LL_DOT;
            if (!push_tok(&toks, &count, &cap, t, source + pos, 1))
                return fail(toks, count, err, "out of memory", pos);
            pos++;
            continue;
        }

        /* Step 3: string literals (value includes the surrounding quotes). */
        if (c == '"') {
            size_t start = pos;
            pos++; /* opening quote */
            while (pos < n && source[pos] != '"') {
                if (source[pos] == '\\') pos++; /* skip the escaped byte */
                pos++;
            }
            if (pos >= n)
                return fail(toks, count, err, "Unterminated string literal",
                            start);
            pos++; /* closing quote */
            if (!push_tok(&toks, &count, &cap, LL_STRING, source + start,
                          pos - start))
                return fail(toks, count, err, "out of memory", start);
            continue;
        }

        /* Step 4: numbers, including a leading `-` before a digit. */
        if (is_digit(c) ||
            (c == '-' && pos + 1 < n && is_digit((unsigned char)source[pos + 1]))) {
            size_t start = pos;
            if (c == '-') pos++;
            while (pos < n && is_digit((unsigned char)source[pos])) pos++;
            if (!push_tok(&toks, &count, &cap, LL_NUMBER, source + start,
                          pos - start))
                return fail(toks, count, err, "out of memory", start);
            continue;
        }

        /* Step 5: symbols. */
        if (is_symbol_start(c)) {
            size_t start = pos;
            pos++;
            while (pos < n && is_symbol_continue((unsigned char)source[pos]))
                pos++;
            if (!push_tok(&toks, &count, &cap, LL_SYMBOL, source + start,
                          pos - start))
                return fail(toks, count, err, "out of memory", start);
            continue;
        }

        /* Step 6: unrecognised byte. */
        {
            char msg[64];
            if (c >= 0x20 && c < 0x7f)
                snprintf(msg, sizeof msg, "Unexpected character: '%c'",
                         (char)c);
            else
                snprintf(msg, sizeof msg, "Unexpected character: '\\x%02x'", c);
            return fail(toks, count, err, msg, pos);
        }
    }

    /* Every token stream ends with exactly one EOF (empty value). */
    if (!push_tok(&toks, &count, &cap, LL_EOF, "", 0))
        return fail(toks, count, err, "out of memory", n);

    out->tokens = toks;
    out->count = count;
    return 1;
}

void ll_token_list_free(LlTokenList *list) {
    if (list == NULL) return;
    free_toks(list->tokens, list->count);
    list->tokens = NULL;
    list->count = 0;
}
