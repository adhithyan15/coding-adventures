/*
 * dot_lexer.c — implementation of the Graphviz DOT tokeniser (see dot_lexer.h).
 * A faithful port of the Rust `dot-lexer` crate's byte-oriented state machine:
 * whitespace/comment skipping, quoted/HTML/unquoted/numeral scanning, keyword
 * recognition, and error recovery.
 */
#include "dot_lexer.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, calloc, free */
#include <string.h> /* strlen, memcpy */

/* ---- growable string used while scanning a token's value -------------- */

typedef struct {
    char *data;
    size_t len;
    size_t cap;
    int ok;
} StrBuf;

static void sb_init(StrBuf *s) {
    s->data = NULL;
    s->len = 0;
    s->cap = 0;
    s->ok = 1;
}

static void sb_push(StrBuf *s, char c) {
    if (!s->ok) {
        return;
    }
    if (s->len + 1 >= s->cap) {
        size_t ncap = s->cap ? (s->cap > (size_t)-1 / 2 ? s->cap + 1 : s->cap * 2)
                             : 16;
        char *nd = realloc(s->data, ncap);
        if (!nd) {
            s->ok = 0;
            return;
        }
        s->data = nd;
        s->cap = ncap;
    }
    s->data[s->len++] = c;
}

/* Finish: return a NUL-terminated owned string (never NULL unless OOM). */
static char *sb_finish(StrBuf *s) {
    if (!s->ok) {
        free(s->data);
        return NULL;
    }
    if (!s->data) {
        return calloc(1, 1); /* empty string */
    }
    s->data[s->len] = '\0';
    return s->data;
}

static char *dup_cstr(const char *s) {
    size_t n = strlen(s);
    char *p = malloc(n + 1);
    if (p) {
        memcpy(p, s, n + 1);
    }
    return p;
}

/* ---- lexer state ------------------------------------------------------ */

typedef struct {
    const unsigned char *src;
    size_t len;
    size_t pos;
    uint32_t line;
    uint32_t col;
    DotToken *tokens;
    size_t ntok, tok_cap;
    DotLexError *errors;
    size_t nerr, err_cap;
    int ok;
} Lexer;

static int lx_peek(const Lexer *l, int *out) {
    if (l->pos < l->len) {
        *out = l->src[l->pos];
        return 1;
    }
    return 0;
}

static int lx_peek2(const Lexer *l, int *out) {
    if (l->pos + 1 < l->len) {
        *out = l->src[l->pos + 1];
        return 1;
    }
    return 0;
}

static int lx_advance(Lexer *l, int *out) {
    int ch;
    if (l->pos >= l->len) {
        return 0;
    }
    ch = l->src[l->pos++];
    if (ch == '\n') {
        l->line += 1;
        l->col = 1;
    } else {
        l->col += 1;
    }
    if (out) {
        *out = ch;
    }
    return 1;
}

static int lx_at_end(const Lexer *l) { return l->pos >= l->len; }

/* Emit a token, taking ownership of `value` (a malloc'd string). */
static void lx_emit(Lexer *l, DotTokenKind kind, char *value, uint32_t line,
                    uint32_t col) {
    if (!l->ok || !value) {
        free(value);
        l->ok = 0;
        return;
    }
    if (l->ntok == l->tok_cap) {
        size_t nc = l->tok_cap ? l->tok_cap * 2 : 16;
        DotToken *nt;
        if (l->tok_cap > (size_t)-1 / 2 || nc > (size_t)-1 / sizeof *nt) {
            free(value);
            l->ok = 0;
            return; /* growth would overflow size_t */
        }
        nt = realloc(l->tokens, nc * sizeof *nt);
        if (!nt) {
            free(value);
            l->ok = 0;
            return;
        }
        l->tokens = nt;
        l->tok_cap = nc;
    }
    l->tokens[l->ntok].kind = kind;
    l->tokens[l->ntok].value = value;
    l->tokens[l->ntok].line = line;
    l->tokens[l->ntok].col = col;
    l->ntok++;
}

static void lx_error_at(Lexer *l, const char *message, uint32_t line,
                        uint32_t col) {
    char *msg;
    if (!l->ok) {
        return;
    }
    msg = dup_cstr(message);
    if (!msg) {
        l->ok = 0;
        return;
    }
    if (l->nerr == l->err_cap) {
        size_t nc = l->err_cap ? l->err_cap * 2 : 8;
        DotLexError *ne;
        if (l->err_cap > (size_t)-1 / 2 || nc > (size_t)-1 / sizeof *ne) {
            free(msg);
            l->ok = 0;
            return; /* growth would overflow size_t */
        }
        ne = realloc(l->errors, nc * sizeof *ne);
        if (!ne) {
            free(msg);
            l->ok = 0;
            return;
        }
        l->errors = ne;
        l->err_cap = nc;
    }
    l->errors[l->nerr].message = msg;
    l->errors[l->nerr].line = line;
    l->errors[l->nerr].col = col;
    l->nerr++;
}

static void lx_error(Lexer *l, const char *message) {
    lx_error_at(l, message, l->line, l->col);
}

/* ---- character classes ------------------------------------------------ */

static int is_digit(int c) { return c >= '0' && c <= '9'; }
static int is_alpha(int c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
static int is_alnum(int c) { return is_alpha(c) || is_digit(c); }

/* ---- skip whitespace and comments ------------------------------------- */

static void skip_ws_comments(Lexer *l) {
    for (;;) {
        int c, c2;
        while (lx_peek(l, &c) &&
               (c == ' ' || c == '\t' || c == '\r' || c == '\n')) {
            lx_advance(l, NULL);
        }
        if (lx_peek(l, &c) && c == '/' && lx_peek2(l, &c2) && c2 == '/') {
            int ch;
            lx_advance(l, NULL);
            lx_advance(l, NULL);
            while (lx_advance(l, &ch)) {
                if (ch == '\n') {
                    break;
                }
            }
            continue;
        }
        if (lx_peek(l, &c) && c == '/' && lx_peek2(l, &c2) && c2 == '*') {
            lx_advance(l, NULL);
            lx_advance(l, NULL);
            for (;;) {
                if (lx_at_end(l)) {
                    lx_error(l, "unterminated block comment");
                    break;
                }
                if (lx_peek(l, &c) && c == '*' && lx_peek2(l, &c2) && c2 == '/') {
                    lx_advance(l, NULL);
                    lx_advance(l, NULL);
                    break;
                }
                lx_advance(l, NULL);
            }
            continue;
        }
        break;
    }
}

/* ---- token scanners (return an owned string) -------------------------- */

static char *scan_quoted_string(Lexer *l, uint32_t sl, uint32_t sc) {
    StrBuf sb;
    int ch;
    sb_init(&sb);
    for (;;) {
        if (!lx_advance(l, &ch)) {
            lx_error_at(l, "unterminated string literal", sl, sc);
            break;
        }
        if (ch == '"') {
            break;
        }
        if (ch == '\\') {
            int esc;
            if (!lx_advance(l, &esc)) {
                lx_error(l, "unexpected end of input in string escape");
                break;
            }
            if (esc == '"') {
                sb_push(&sb, '"');
            } else if (esc == '\\') {
                sb_push(&sb, '\\');
            } else if (esc == 'n') {
                sb_push(&sb, '\n');
            } else if (esc == 't') {
                sb_push(&sb, '\t');
            } else {
                sb_push(&sb, '\\');
                sb_push(&sb, (char)esc);
            }
        } else {
            sb_push(&sb, (char)ch);
        }
    }
    return sb_finish(&sb);
}

static char *scan_html_string(Lexer *l, uint32_t sl, uint32_t sc) {
    StrBuf sb;
    int depth = 1;
    int ch;
    sb_init(&sb);
    for (;;) {
        if (!lx_advance(l, &ch)) {
            lx_error_at(l, "unterminated HTML string", sl, sc);
            break;
        }
        if (ch == '<') {
            depth += 1;
            sb_push(&sb, '<');
        } else if (ch == '>') {
            depth -= 1;
            if (depth == 0) {
                break;
            }
            sb_push(&sb, '>');
        } else {
            sb_push(&sb, (char)ch);
        }
    }
    return sb_finish(&sb);
}

static char *scan_unquoted_id(Lexer *l, int first) {
    StrBuf sb;
    int c;
    sb_init(&sb);
    sb_push(&sb, (char)first);
    while (lx_peek(l, &c) && (is_alnum(c) || c == '_' || c >= 0x80)) {
        lx_advance(l, NULL);
        sb_push(&sb, (char)c);
    }
    return sb_finish(&sb);
}

static char *scan_numeral(Lexer *l, int first) {
    StrBuf sb;
    int c;
    sb_init(&sb);
    sb_push(&sb, (char)first);
    while (lx_peek(l, &c) && is_digit(c)) {
        lx_advance(l, NULL);
        sb_push(&sb, (char)c);
    }
    if (lx_peek(l, &c) && c == '.') {
        lx_advance(l, NULL);
        sb_push(&sb, '.');
        while (lx_peek(l, &c) && is_digit(c)) {
            lx_advance(l, NULL);
            sb_push(&sb, (char)c);
        }
    }
    return sb_finish(&sb);
}

/* Map an unquoted word (case-insensitive) to a keyword kind, or DOT_ID. */
static DotTokenKind keyword_or_id(const char *word) {
    char lower[16];
    size_t i, n = strlen(word);
    if (n >= sizeof lower) {
        return DOT_ID; /* longer than any keyword */
    }
    for (i = 0; i < n; i++) {
        char c = word[i];
        lower[i] = (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : c;
    }
    lower[n] = '\0';
    if (strcmp(lower, "strict") == 0) return DOT_STRICT;
    if (strcmp(lower, "graph") == 0) return DOT_GRAPH;
    if (strcmp(lower, "digraph") == 0) return DOT_DIGRAPH;
    if (strcmp(lower, "node") == 0) return DOT_NODE;
    if (strcmp(lower, "edge") == 0) return DOT_EDGE;
    if (strcmp(lower, "subgraph") == 0) return DOT_SUBGRAPH;
    return DOT_ID;
}

/* ---- main scan loop --------------------------------------------------- */

static void scan_all(Lexer *l) {
    for (;;) {
        uint32_t line, col;
        int ch, nc;
        skip_ws_comments(l);
        if (!l->ok) {
            return;
        }
        if (lx_at_end(l)) {
            lx_emit(l, DOT_EOF, dup_cstr(""), l->line, l->col);
            break;
        }
        line = l->line;
        col = l->col;
        lx_advance(l, &ch);

        switch (ch) {
            case '{': lx_emit(l, DOT_LBRACE, dup_cstr(""), line, col); continue;
            case '}': lx_emit(l, DOT_RBRACE, dup_cstr(""), line, col); continue;
            case '[': lx_emit(l, DOT_LBRACKET, dup_cstr(""), line, col); continue;
            case ']': lx_emit(l, DOT_RBRACKET, dup_cstr(""), line, col); continue;
            case '=': lx_emit(l, DOT_EQUALS, dup_cstr(""), line, col); continue;
            case ';': lx_emit(l, DOT_SEMICOLON, dup_cstr(""), line, col); continue;
            case ',': lx_emit(l, DOT_COMMA, dup_cstr(""), line, col); continue;
            case ':': lx_emit(l, DOT_COLON, dup_cstr(""), line, col); continue;
            default: break;
        }

        if (ch == '-' && lx_peek(l, &nc) && nc == '>') {
            lx_advance(l, NULL);
            lx_emit(l, DOT_ARROW, dup_cstr(""), line, col);
        } else if (ch == '-' && lx_peek(l, &nc) && nc == '-') {
            lx_advance(l, NULL);
            lx_emit(l, DOT_DASHDASH, dup_cstr(""), line, col);
        } else if (ch == '-' && lx_peek(l, &nc) && (is_digit(nc) || nc == '.')) {
            lx_emit(l, DOT_ID, scan_numeral(l, ch), line, col);
        } else if (ch == '.' && lx_peek(l, &nc) && is_digit(nc)) {
            lx_emit(l, DOT_ID, scan_numeral(l, ch), line, col);
        } else if (is_digit(ch)) {
            lx_emit(l, DOT_ID, scan_numeral(l, ch), line, col);
        } else if (ch == '"') {
            lx_emit(l, DOT_ID, scan_quoted_string(l, line, col), line, col);
        } else if (ch == '<') {
            lx_emit(l, DOT_ID, scan_html_string(l, line, col), line, col);
        } else if (is_alpha(ch) || ch == '_' || ch >= 0x80) {
            char *word = scan_unquoted_id(l, ch);
            if (!word) {
                l->ok = 0;
                return;
            }
            {
                DotTokenKind kind = keyword_or_id(word);
                if (kind == DOT_ID) {
                    lx_emit(l, DOT_ID, word, line, col);
                } else {
                    free(word);
                    lx_emit(l, kind, dup_cstr(""), line, col);
                }
            }
        } else {
            char buf[64];
            snprintf(buf, sizeof buf, "unexpected character '%c' (0x%02x)",
                     (ch >= 32 && ch < 127) ? ch : '?', (unsigned)ch);
            lx_error_at(l, buf, line, col);
        }
        if (!l->ok) {
            return;
        }
    }
}

/* ---- public API ------------------------------------------------------- */

DotLexResult *dot_tokenise(const char *source) {
    Lexer l;
    DotLexResult *r;
    l.src = (const unsigned char *)source;
    l.len = strlen(source);
    l.pos = 0;
    l.line = 1;
    l.col = 1;
    l.tokens = NULL;
    l.ntok = 0;
    l.tok_cap = 0;
    l.errors = NULL;
    l.nerr = 0;
    l.err_cap = 0;
    l.ok = 1;

    scan_all(&l);

    if (!l.ok) {
        size_t i;
        for (i = 0; i < l.ntok; i++) {
            free(l.tokens[i].value);
        }
        free(l.tokens);
        for (i = 0; i < l.nerr; i++) {
            free(l.errors[i].message);
        }
        free(l.errors);
        return NULL;
    }

    r = malloc(sizeof *r);
    if (!r) {
        size_t i;
        for (i = 0; i < l.ntok; i++) {
            free(l.tokens[i].value);
        }
        free(l.tokens);
        for (i = 0; i < l.nerr; i++) {
            free(l.errors[i].message);
        }
        free(l.errors);
        return NULL;
    }
    r->tokens = l.tokens;
    r->ntokens = l.ntok;
    r->errors = l.errors;
    r->nerrors = l.nerr;
    return r;
}

void dot_lex_result_free(DotLexResult *r) {
    size_t i;
    if (!r) {
        return;
    }
    for (i = 0; i < r->ntokens; i++) {
        free(r->tokens[i].value);
    }
    free(r->tokens);
    for (i = 0; i < r->nerrors; i++) {
        free(r->errors[i].message);
    }
    free(r->errors);
    free(r);
}
