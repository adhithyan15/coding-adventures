/*
 * lisp_parser.c — implementation of the pure-ISO C Lisp parser.
 * ============================================================
 *
 * A recursive-descent parser over the lexer's token stream: one function per
 * grammar rule. The AST is an owned tagged-union tree of `LpSExpr` nodes; a
 * failed parse frees whatever it built and reports a message.
 */
#include "lisp_parser.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* strlen, memcpy */

/* ── String helpers ────────────────────────────────────────────────────────*/

static char *str_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) memcpy(p, s, n);
    return p;
}

void lp_strlist_free(LpStrList *list) {
    if (list == NULL) return;
    for (size_t i = 0; i < list->n; i++) free(list->items[i]);
    free(list->items);
    list->items = NULL;
    list->n = 0;
}

static int strlist_push(LpStrList *l, size_t *cap, const char *s) {
    if (l->n == *cap) {
        size_t nc = *cap ? *cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(char *)) return 0;
        nc *= 2;
        char **ni = (char **)realloc(l->items, nc * sizeof(char *));
        if (ni == NULL) return 0;
        l->items = ni;
        *cap = nc;
    }
    char *dup = str_dup(s);
    if (dup == NULL) return 0;
    l->items[l->n++] = dup;
    return 1;
}

/* ── AST nodes ─────────────────────────────────────────────────────────────*/

struct LpSExpr {
    LpSExprKind kind;
    union {
        struct {
            LpAtomKind kind;
            char *value;
        } atom;
        struct {
            LpSExpr **items;
            size_t n;
        } list; /* also used for the elements of a dotted pair */
        struct {
            LpSExpr **items;
            size_t n;
            LpSExpr *last;
        } dotted;
        LpSExpr *quoted;
    } as;
};

static LpSExpr *node_alloc(LpSExprKind kind) {
    LpSExpr *e = (LpSExpr *)calloc(1, sizeof(LpSExpr));
    if (e != NULL) e->kind = kind;
    return e;
}

static void sexpr_free(LpSExpr *e) {
    if (e == NULL) return;
    switch (e->kind) {
        case LP_ATOM:
            free(e->as.atom.value);
            break;
        case LP_LIST:
            for (size_t i = 0; i < e->as.list.n; i++) sexpr_free(e->as.list.items[i]);
            free(e->as.list.items);
            break;
        case LP_DOTTED_PAIR:
            for (size_t i = 0; i < e->as.dotted.n; i++)
                sexpr_free(e->as.dotted.items[i]);
            free(e->as.dotted.items);
            sexpr_free(e->as.dotted.last);
            break;
        case LP_QUOTED:
            sexpr_free(e->as.quoted);
            break;
    }
    free(e);
}

/* Free an in-progress element array (used on parse error). */
static void free_elems(LpSExpr **elems, size_t n) {
    for (size_t i = 0; i < n; i++) sexpr_free(elems[i]);
    free(elems);
}

static int elems_push(LpSExpr ***arr, size_t *n, size_t *cap, LpSExpr *v) {
    if (*n == *cap) {
        size_t nc = *cap ? *cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(LpSExpr *)) return 0;
        nc *= 2;
        LpSExpr **na = (LpSExpr **)realloc(*arr, nc * sizeof(LpSExpr *));
        if (na == NULL) return 0;
        *arr = na;
        *cap = nc;
    }
    (*arr)[(*n)++] = v;
    return 1;
}

/* ── Parser state ──────────────────────────────────────────────────────────*/

typedef struct {
    const LlToken *tokens;
    size_t n;
    size_t pos;
} Parser;

/* Safe peek: past the end reads as EOF (the lexer always appends one anyway). */
static LlTokenType peek_type(const Parser *p) {
    return p->pos < p->n ? p->tokens[p->pos].type : LL_EOF;
}
static const char *peek_value(const Parser *p) {
    return p->pos < p->n ? p->tokens[p->pos].value : "";
}
static void advance(Parser *p) {
    if (p->pos < p->n) p->pos++;
}

static void set_err(LpError *err, const char *msg) {
    size_t i = 0;
    for (; msg[i] != '\0' && i + 1 < sizeof err->message; i++)
        err->message[i] = msg[i];
    err->message[i] = '\0';
}

static LpSExpr *parse_sexpr(Parser *p, LpError *err);

/* list = '(' { sexpr } [ '.' sexpr ] ')' */
static LpSExpr *parse_list(Parser *p, LpError *err) {
    advance(p); /* consume '(' (already known to be LParen) */

    LpSExpr **elems = NULL;
    size_t n = 0, cap = 0;
    int is_dotted = 0;
    LpSExpr *dot_value = NULL;

    while (peek_type(p) != LL_RPAREN && peek_type(p) != LL_EOF) {
        if (peek_type(p) == LL_DOT) {
            advance(p); /* consume '.' */
            is_dotted = 1;
            dot_value = parse_sexpr(p, err);
            if (dot_value == NULL) {
                free_elems(elems, n);
                return NULL;
            }
            break;
        }
        LpSExpr *child = parse_sexpr(p, err);
        if (child == NULL) {
            free_elems(elems, n);
            return NULL;
        }
        if (!elems_push(&elems, &n, &cap, child)) {
            sexpr_free(child);
            free_elems(elems, n);
            set_err(err, "ParseError: out of memory");
            return NULL;
        }
    }

    if (peek_type(p) != LL_RPAREN) {
        set_err(err, "ParseError: Expected RParen");
        free_elems(elems, n);
        sexpr_free(dot_value);
        return NULL;
    }
    advance(p); /* consume ')' */

    if (is_dotted) {
        LpSExpr *e = node_alloc(LP_DOTTED_PAIR);
        if (e == NULL) {
            free_elems(elems, n);
            sexpr_free(dot_value);
            set_err(err, "ParseError: out of memory");
            return NULL;
        }
        e->as.dotted.items = elems;
        e->as.dotted.n = n;
        e->as.dotted.last = dot_value;
        return e;
    }
    LpSExpr *e = node_alloc(LP_LIST);
    if (e == NULL) {
        free_elems(elems, n);
        set_err(err, "ParseError: out of memory");
        return NULL;
    }
    e->as.list.items = elems;
    e->as.list.n = n;
    return e;
}

static LpSExpr *make_atom(LpAtomKind kind, const char *value, LpError *err) {
    LpSExpr *e = node_alloc(LP_ATOM);
    if (e == NULL) {
        set_err(err, "ParseError: out of memory");
        return NULL;
    }
    e->as.atom.kind = kind;
    e->as.atom.value = str_dup(value);
    if (e->as.atom.value == NULL) {
        free(e);
        set_err(err, "ParseError: out of memory");
        return NULL;
    }
    return e;
}

/* sexpr = atom | list | quoted */
static LpSExpr *parse_sexpr(Parser *p, LpError *err) {
    switch (peek_type(p)) {
        case LL_LPAREN:
            return parse_list(p, err);
        case LL_QUOTE: {
            advance(p); /* consume the quote */
            LpSExpr *inner = parse_sexpr(p, err);
            if (inner == NULL) return NULL;
            LpSExpr *e = node_alloc(LP_QUOTED);
            if (e == NULL) {
                sexpr_free(inner);
                set_err(err, "ParseError: out of memory");
                return NULL;
            }
            e->as.quoted = inner;
            return e;
        }
        case LL_NUMBER: {
            LpSExpr *e = make_atom(LP_NUMBER, peek_value(p), err);
            if (e != NULL) advance(p);
            return e;
        }
        case LL_SYMBOL: {
            LpSExpr *e = make_atom(LP_SYMBOL, peek_value(p), err);
            if (e != NULL) advance(p);
            return e;
        }
        case LL_STRING: {
            LpSExpr *e = make_atom(LP_STRING, peek_value(p), err);
            if (e != NULL) advance(p);
            return e;
        }
        default: {
            char msg[128];
            snprintf(msg, sizeof msg, "ParseError: Unexpected token: %s (%s)",
                     ll_token_type_name(peek_type(p)), peek_value(p));
            set_err(err, msg);
            return NULL;
        }
    }
}

/* ── Public API ────────────────────────────────────────────────────────────*/

int lp_parse_tokens(const LlToken *tokens, size_t n_tokens, LpProgram *out,
                    LpError *err) {
    out->exprs = NULL;
    out->n = 0;

    Parser p = {tokens, n_tokens, 0};
    LpSExpr **exprs = NULL;
    size_t n = 0, cap = 0;

    while (peek_type(&p) != LL_EOF) {
        LpSExpr *e = parse_sexpr(&p, err);
        if (e == NULL) {
            free_elems(exprs, n);
            return 0;
        }
        if (!elems_push(&exprs, &n, &cap, e)) {
            sexpr_free(e);
            free_elems(exprs, n);
            set_err(err, "ParseError: out of memory");
            return 0;
        }
    }

    out->exprs = exprs;
    out->n = n;
    return 1;
}

int lp_parse(const char *source, LpProgram *out, LpError *err) {
    out->exprs = NULL;
    out->n = 0;

    LlTokenList tokens;
    LlError lex_err;
    if (!ll_tokenize(source, &tokens, &lex_err)) {
        char msg[128];
        snprintf(msg, sizeof msg, "ParseError: Lexer error: %s", lex_err.message);
        set_err(err, msg);
        return 0;
    }
    int ok = lp_parse_tokens(tokens.tokens, tokens.count, out, err);
    ll_token_list_free(&tokens);
    return ok;
}

void lp_program_free(LpProgram *program) {
    if (program == NULL) return;
    for (size_t i = 0; i < program->n; i++) sexpr_free(program->exprs[i]);
    free(program->exprs);
    program->exprs = NULL;
    program->n = 0;
}

/* ── Node inspection ───────────────────────────────────────────────────────*/

LpSExprKind lp_sexpr_kind(const LpSExpr *e) { return e->kind; }
LpAtomKind lp_sexpr_atom_kind(const LpSExpr *e) { return e->as.atom.kind; }
const char *lp_sexpr_atom_value(const LpSExpr *e) { return e->as.atom.value; }

static int collect_atoms(const LpSExpr *e, LpStrList *out, size_t *cap) {
    switch (e->kind) {
        case LP_ATOM:
            return strlist_push(out, cap, e->as.atom.value);
        case LP_LIST:
            for (size_t i = 0; i < e->as.list.n; i++)
                if (!collect_atoms(e->as.list.items[i], out, cap)) return 0;
            return 1;
        case LP_DOTTED_PAIR:
            for (size_t i = 0; i < e->as.dotted.n; i++)
                if (!collect_atoms(e->as.dotted.items[i], out, cap)) return 0;
            return collect_atoms(e->as.dotted.last, out, cap);
        case LP_QUOTED:
            return collect_atoms(e->as.quoted, out, cap);
    }
    return 1;
}

LpStrList lp_sexpr_find_atoms(const LpSExpr *e) {
    LpStrList out = {NULL, 0};
    size_t cap = 0;
    if (!collect_atoms(e, &out, &cap)) lp_strlist_free(&out);
    return out;
}

size_t lp_sexpr_count_lists(const LpSExpr *e) {
    switch (e->kind) {
        case LP_LIST: {
            size_t total = 1;
            for (size_t i = 0; i < e->as.list.n; i++)
                total += lp_sexpr_count_lists(e->as.list.items[i]);
            return total;
        }
        case LP_DOTTED_PAIR: {
            size_t total = 1;
            for (size_t i = 0; i < e->as.dotted.n; i++)
                total += lp_sexpr_count_lists(e->as.dotted.items[i]);
            return total + lp_sexpr_count_lists(e->as.dotted.last);
        }
        case LP_QUOTED:
            return lp_sexpr_count_lists(e->as.quoted);
        case LP_ATOM:
            return 0;
    }
    return 0;
}

size_t lp_sexpr_count_quoted(const LpSExpr *e) {
    switch (e->kind) {
        case LP_QUOTED:
            return 1 + lp_sexpr_count_quoted(e->as.quoted);
        case LP_LIST: {
            size_t total = 0;
            for (size_t i = 0; i < e->as.list.n; i++)
                total += lp_sexpr_count_quoted(e->as.list.items[i]);
            return total;
        }
        case LP_DOTTED_PAIR: {
            size_t total = 0;
            for (size_t i = 0; i < e->as.dotted.n; i++)
                total += lp_sexpr_count_quoted(e->as.dotted.items[i]);
            return total + lp_sexpr_count_quoted(e->as.dotted.last);
        }
        case LP_ATOM:
            return 0;
    }
    return 0;
}

LpStrList lp_program_find_atoms(const LpProgram *program) {
    LpStrList out = {NULL, 0};
    size_t cap = 0;
    for (size_t i = 0; i < program->n; i++)
        if (!collect_atoms(program->exprs[i], &out, &cap)) {
            lp_strlist_free(&out);
            return out;
        }
    return out;
}

size_t lp_program_count_lists(const LpProgram *program) {
    size_t total = 0;
    for (size_t i = 0; i < program->n; i++)
        total += lp_sexpr_count_lists(program->exprs[i]);
    return total;
}

size_t lp_program_count_quoted(const LpProgram *program) {
    size_t total = 0;
    for (size_t i = 0; i < program->n; i++)
        total += lp_sexpr_count_quoted(program->exprs[i]);
    return total;
}
