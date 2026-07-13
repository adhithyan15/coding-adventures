/*
 * lisp_compiler.c — implementation of the pure-ISO C Lisp bytecode compiler.
 * =========================================================================
 *
 * A recursive-descent compiler over the parser's S-expression AST. It walks the
 * tree and emits bytecode into growable buffers, dispatching each list on its
 * first element (special form / operator / call). Compilation errors — bad
 * syntax or allocation failure — set a `failed` flag; the emit/add helpers
 * become guarded no-ops once it is set, so the walk unwinds cleanly and the
 * partial output is discarded.
 */
#include "lisp_compiler.h"

#include <errno.h>  /* errno, ERANGE */
#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free, strtoll */
#include <string.h> /* strcmp, strlen, memcpy, memset */

static char *str_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) memcpy(p, s, n);
    return p;
}

/* ── Values ────────────────────────────────────────────────────────────────*/

static LcValue val_zero(LcValueKind kind) {
    LcValue v;
    v.kind = kind;
    v.integer = 0;
    v.str = NULL;
    v.boolean = 0;
    v.addr = 0;
    v.code = NULL;
    return v;
}

int lc_value_is_falsy(const LcValue *v) {
    return v->kind == LC_VAL_NIL ||
           (v->kind == LC_VAL_BOOL && v->boolean == 0) ||
           (v->kind == LC_VAL_INTEGER && v->integer == 0);
}

static int code_equal(const LcCodeObject *a, const LcCodeObject *b);

static int value_equal(const LcValue *a, const LcValue *b) {
    if (a->kind != b->kind) return 0;
    switch (a->kind) {
        case LC_VAL_INTEGER: return a->integer == b->integer;
        case LC_VAL_STRING:
        case LC_VAL_SYMBOL: return strcmp(a->str, b->str) == 0;
        case LC_VAL_BOOL: return a->boolean == b->boolean;
        case LC_VAL_NIL: return 1;
        case LC_VAL_CONS_ADDR:
        case LC_VAL_CLOSURE_ADDR: return a->addr == b->addr;
        case LC_VAL_CODE: return code_equal(a->code, b->code);
    }
    return 0;
}

/* Free the heap a value owns (its string and/or nested code). A nested code
 * object is itself heap-allocated, so free its contents AND the struct. */
static void value_free_contents(LcValue *v) {
    free(v->str);
    v->str = NULL;
    if (v->code != NULL) {
        lc_code_object_free(v->code); /* contents */
        free(v->code);                /* the heap struct */
        v->code = NULL;
    }
}

/* Release the CONTENTS of a code object (instructions, constant pool including
 * nested code, name pool). The `code` struct itself is caller-owned (typically
 * a stack variable filled by lc_compile) and is NOT freed here. */
void lc_code_object_free(LcCodeObject *code) {
    if (code == NULL) return;
    free(code->instructions);
    for (size_t i = 0; i < code->n_constants; i++)
        value_free_contents(&code->constants[i]);
    free(code->constants);
    for (size_t i = 0; i < code->n_names; i++) free(code->names[i]);
    free(code->names);
    code->instructions = NULL;
    code->constants = NULL;
    code->names = NULL;
    code->n_instructions = code->n_constants = code->n_names = 0;
}

static int instr_equal(const LcInstruction *a, const LcInstruction *b) {
    return a->opcode == b->opcode && a->has_operand == b->has_operand &&
           a->operand == b->operand;
}

static int code_equal(const LcCodeObject *a, const LcCodeObject *b) {
    if (a->n_instructions != b->n_instructions ||
        a->n_constants != b->n_constants || a->n_names != b->n_names)
        return 0;
    for (size_t i = 0; i < a->n_instructions; i++)
        if (!instr_equal(&a->instructions[i], &b->instructions[i])) return 0;
    for (size_t i = 0; i < a->n_constants; i++)
        if (!value_equal(&a->constants[i], &b->constants[i])) return 0;
    for (size_t i = 0; i < a->n_names; i++)
        if (strcmp(a->names[i], b->names[i]) != 0) return 0;
    return 1;
}

/* ── Operator tables ───────────────────────────────────────────────────────*/

/* Returns the opcode for an arithmetic symbol, or -1. */
static int arithmetic_op(const char *sym) {
    if (strcmp(sym, "+") == 0) return LC_ADD;
    if (strcmp(sym, "-") == 0) return LC_SUB;
    if (strcmp(sym, "*") == 0) return LC_MUL;
    if (strcmp(sym, "/") == 0) return LC_DIV;
    return -1;
}
static int comparison_op(const char *sym) {
    if (strcmp(sym, "=") == 0) return LC_CMP_EQ;
    if (strcmp(sym, "<") == 0) return LC_CMP_LT;
    if (strcmp(sym, ">") == 0) return LC_CMP_GT;
    return -1;
}

/* ── Compiler state ────────────────────────────────────────────────────────*/

typedef struct {
    char *name;
    size_t slot;
} ScopeEntry;
typedef struct {
    ScopeEntry *entries;
    size_t n, cap;
} Scope;

typedef struct {
    LcInstruction *instructions;
    size_t n_instr, cap_instr;
    LcValue *constants;
    size_t n_const, cap_const;
    char **names;
    size_t n_names, cap_names;
    int tail_position;
    int in_function;
    Scope *scopes;
    size_t n_scopes, cap_scopes;
    int failed;
    char errmsg[128];
} Compiler;

static void fail(Compiler *c, const char *msg) {
    if (c->failed) return;
    c->failed = 1;
    size_t i = 0;
    for (; msg[i] != '\0' && i + 1 < sizeof c->errmsg; i++) c->errmsg[i] = msg[i];
    c->errmsg[i] = '\0';
}

/* ── Emit helpers (guarded no-ops once failed) ─────────────────────────────*/

static void emit_instr(Compiler *c, LcOp op, int has_operand, size_t operand) {
    if (c->failed) return;
    if (c->n_instr == c->cap_instr) {
        size_t nc = c->cap_instr ? c->cap_instr : 16;
        if (nc > ((size_t)-1) / 2 / sizeof(LcInstruction)) {
            fail(c, "CompileError: out of memory");
            return;
        }
        nc *= 2;
        LcInstruction *ni = (LcInstruction *)realloc(
            c->instructions, nc * sizeof(LcInstruction));
        if (ni == NULL) {
            fail(c, "CompileError: out of memory");
            return;
        }
        c->instructions = ni;
        c->cap_instr = nc;
    }
    c->instructions[c->n_instr].opcode = op;
    c->instructions[c->n_instr].has_operand = has_operand;
    c->instructions[c->n_instr].operand = operand;
    c->n_instr++;
}
static void emit(Compiler *c, LcOp op) { emit_instr(c, op, 0, 0); }
static void emit_with(Compiler *c, LcOp op, size_t operand) {
    emit_instr(c, op, 1, operand);
}
static size_t emit_jump(Compiler *c, LcOp op) {
    size_t idx = c->n_instr;
    emit_instr(c, op, 1, 0); /* placeholder target, patched later */
    return idx;
}
static void patch_jump(Compiler *c, size_t jump_idx) {
    if (c->failed || jump_idx >= c->n_instr) return;
    c->instructions[jump_idx].operand = c->n_instr;
}

/* Append a value to the constant pool without dedup; takes ownership. */
static size_t push_constant(Compiler *c, LcValue v) {
    if (c->failed) {
        value_free_contents(&v);
        return 0;
    }
    if (c->n_const == c->cap_const) {
        size_t nc = c->cap_const ? c->cap_const : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(LcValue)) {
            value_free_contents(&v);
            fail(c, "CompileError: out of memory");
            return 0;
        }
        nc *= 2;
        LcValue *nv = (LcValue *)realloc(c->constants, nc * sizeof(LcValue));
        if (nv == NULL) {
            value_free_contents(&v);
            fail(c, "CompileError: out of memory");
            return 0;
        }
        c->constants = nv;
        c->cap_const = nc;
    }
    c->constants[c->n_const] = v;
    return c->n_const++;
}

/* Add a value to the pool with deduplication; takes ownership of `v`. */
static size_t add_constant(Compiler *c, LcValue v) {
    if (c->failed) {
        value_free_contents(&v);
        return 0;
    }
    for (size_t i = 0; i < c->n_const; i++) {
        if (value_equal(&c->constants[i], &v)) {
            value_free_contents(&v);
            return i;
        }
    }
    return push_constant(c, v);
}

static size_t add_name(Compiler *c, const char *name) {
    if (c->failed) return 0;
    for (size_t i = 0; i < c->n_names; i++)
        if (strcmp(c->names[i], name) == 0) return i;
    if (c->n_names == c->cap_names) {
        size_t nc = c->cap_names ? c->cap_names : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(char *)) {
            fail(c, "CompileError: out of memory");
            return 0;
        }
        nc *= 2;
        char **nn = (char **)realloc(c->names, nc * sizeof(char *));
        if (nn == NULL) {
            fail(c, "CompileError: out of memory");
            return 0;
        }
        c->names = nn;
        c->cap_names = nc;
    }
    char *dup = str_dup(name);
    if (dup == NULL) {
        fail(c, "CompileError: out of memory");
        return 0;
    }
    c->names[c->n_names] = dup;
    return c->n_names++;
}

/* Look up a local in the current (topmost) scope only. Returns 1 + *slot. */
static int get_local(const Compiler *c, const char *name, size_t *slot) {
    if (c->n_scopes == 0) return 0;
    const Scope *s = &c->scopes[c->n_scopes - 1];
    for (size_t i = 0; i < s->n; i++)
        if (strcmp(s->entries[i].name, name) == 0) {
            *slot = s->entries[i].slot;
            return 1;
        }
    return 0;
}

/* ── AST helpers ───────────────────────────────────────────────────────────*/

/* Is `e` the symbol atom `name`? */
static int is_symbol(const LpSExpr *e, const char *name) {
    return lp_sexpr_kind(e) == LP_ATOM &&
           lp_sexpr_atom_kind(e) == LP_SYMBOL &&
           strcmp(lp_sexpr_atom_value(e), name) == 0;
}

/* Materialise a LIST/DOTTED node's elements (for dotted, elements + the final
 * cdr) into a fresh pointer array. Returns 1 (setting *out and *n_out), else
 * 0 on OOM. */
static int list_children(const LpSExpr *node, const LpSExpr ***out,
                         size_t *n_out) {
    LpSExprKind k = lp_sexpr_kind(node);
    size_t base = lp_sexpr_child_count(node);
    size_t total = (k == LP_DOTTED_PAIR) ? base + 1 : base;
    *out = NULL;
    *n_out = total;
    if (total == 0) return 1;
    const LpSExpr **arr =
        (const LpSExpr **)malloc(total * sizeof(const LpSExpr *));
    if (arr == NULL) return 0;
    for (size_t i = 0; i < base; i++) arr[i] = lp_sexpr_child(node, i);
    if (k == LP_DOTTED_PAIR) arr[base] = lp_sexpr_dotted_last(node);
    *out = arr;
    return 1;
}

/* ── Forward declarations ──────────────────────────────────────────────────*/

static void compile_sexpr(Compiler *c, const LpSExpr *e);
static void compile_list(Compiler *c, const LpSExpr **elements, size_t n);
static void compile_quoted_datum(Compiler *c, const LpSExpr *e);

/* Parse a `-?[0-9]+` atom into an i64. Returns 1 + *out, 0 on failure
 * (including out-of-range, matching Rust's `parse::<i64>()`). */
static int parse_int(const char *s, int64_t *out) {
    char *end = NULL;
    errno = 0;
    long long v = strtoll(s, &end, 10);
    if (end == s || *end != '\0' || errno == ERANGE) return 0;
    *out = (int64_t)v;
    return 1;
}

/* Strip surrounding quotes from a string literal, returning an owned copy. */
static char *strip_quotes(const char *value) {
    size_t len = strlen(value);
    if (len >= 2 && value[0] == '"' && value[len - 1] == '"') {
        char *p = (char *)malloc(len - 1);
        if (p == NULL) return NULL;
        memcpy(p, value + 1, len - 2);
        p[len - 2] = '\0';
        return p;
    }
    return str_dup(value);
}

/* ── Atom compilation ──────────────────────────────────────────────────────*/

static void compile_atom(Compiler *c, LpAtomKind kind, const char *value) {
    if (c->failed) return;
    if (kind == LP_NUMBER) {
        int64_t n;
        if (!parse_int(value, &n)) {
            char m[128];
            snprintf(m, sizeof m, "CompileError: Invalid number: %s", value);
            fail(c, m);
            return;
        }
        LcValue v = val_zero(LC_VAL_INTEGER);
        v.integer = n;
        emit_with(c, LC_LOAD_CONST, add_constant(c, v));
    } else if (kind == LP_STRING) {
        LcValue v = val_zero(LC_VAL_STRING);
        v.str = strip_quotes(value);
        if (v.str == NULL) {
            fail(c, "CompileError: out of memory");
            return;
        }
        emit_with(c, LC_LOAD_CONST, add_constant(c, v));
    } else { /* LP_SYMBOL */
        if (strcmp(value, "nil") == 0) {
            emit(c, LC_LOAD_NIL);
        } else if (strcmp(value, "t") == 0) {
            emit(c, LC_LOAD_TRUE);
        } else {
            size_t slot;
            if (get_local(c, value, &slot)) {
                emit_with(c, LC_LOAD_LOCAL, slot);
            } else {
                emit_with(c, LC_LOAD_NAME, add_name(c, value));
            }
        }
    }
}

/* ── Special forms ─────────────────────────────────────────────────────────*/

static void compile_define(Compiler *c, const LpSExpr **e, size_t n) {
    if (n != 3) {
        char m[128];
        snprintf(m, sizeof m,
                 "CompileError: define expects 2 arguments, got %zu", n - 1);
        fail(c, m);
        return;
    }
    if (!(lp_sexpr_kind(e[1]) == LP_ATOM &&
          lp_sexpr_atom_kind(e[1]) == LP_SYMBOL)) {
        fail(c, "CompileError: define name must be a symbol");
        return;
    }
    const char *name = lp_sexpr_atom_value(e[1]);
    int saved_tail = c->tail_position;
    c->tail_position = 0;
    compile_sexpr(c, e[2]);
    c->tail_position = saved_tail;
    emit_with(c, LC_STORE_NAME, add_name(c, name));
    emit(c, LC_LOAD_NIL);
}

static void compile_lambda(Compiler *c, const LpSExpr **e, size_t n) {
    if (n < 3) {
        fail(c, "CompileError: lambda needs params and body");
        return;
    }
    if (lp_sexpr_kind(e[1]) != LP_LIST) {
        fail(c, "CompileError: lambda params must be a list");
        return;
    }
    /* Extract parameter names. */
    size_t n_params = lp_sexpr_child_count(e[1]);
    char **params = NULL;
    if (n_params > 0) {
        params = (char **)calloc(n_params, sizeof(char *));
        if (params == NULL) {
            fail(c, "CompileError: out of memory");
            return;
        }
    }
    for (size_t i = 0; i < n_params; i++) {
        const LpSExpr *p = lp_sexpr_child(e[1], i);
        if (!(lp_sexpr_kind(p) == LP_ATOM &&
              lp_sexpr_atom_kind(p) == LP_SYMBOL)) {
            fail(c, "CompileError: lambda parameter must be a symbol");
            for (size_t k = 0; k < i; k++) free(params[k]);
            free(params);
            return;
        }
        params[i] = str_dup(lp_sexpr_atom_value(p));
        if (params[i] == NULL) {
            fail(c, "CompileError: out of memory");
            for (size_t k = 0; k < i; k++) free(params[k]);
            free(params);
            return;
        }
    }

    /* Enter a new scope with the parameters. */
    if (c->n_scopes == c->cap_scopes) {
        size_t ncap = c->cap_scopes ? c->cap_scopes * 2 : 4;
        Scope *ns = (Scope *)realloc(c->scopes, ncap * sizeof(Scope));
        if (ns == NULL) {
            fail(c, "CompileError: out of memory");
            for (size_t k = 0; k < n_params; k++) free(params[k]);
            free(params);
            return;
        }
        c->scopes = ns;
        c->cap_scopes = ncap;
    }
    Scope *scope = &c->scopes[c->n_scopes];
    scope->entries = NULL;
    scope->n = 0;
    scope->cap = 0;
    if (n_params > 0) {
        scope->entries = (ScopeEntry *)malloc(n_params * sizeof(ScopeEntry));
        if (scope->entries == NULL) {
            fail(c, "CompileError: out of memory");
            for (size_t k = 0; k < n_params; k++) free(params[k]);
            free(params);
            return;
        }
        for (size_t i = 0; i < n_params; i++) {
            scope->entries[i].name = params[i]; /* transfer ownership */
            scope->entries[i].slot = i;
        }
        scope->n = n_params;
        scope->cap = n_params;
    }
    c->n_scopes++;
    free(params); /* names now owned by the scope */

    /* Save the outer buffers and compile the body into fresh ones. */
    LcInstruction *si = c->instructions;
    size_t sni = c->n_instr, sci = c->cap_instr;
    LcValue *sco = c->constants;
    size_t snc = c->n_const, scc = c->cap_const;
    char **snm = c->names;
    size_t snn = c->n_names, scn = c->cap_names;
    int saved_tail = c->tail_position, saved_in = c->in_function;
    c->instructions = NULL;
    c->n_instr = c->cap_instr = 0;
    c->constants = NULL;
    c->n_const = c->cap_const = 0;
    c->names = NULL;
    c->n_names = c->cap_names = 0;
    c->in_function = 1;

    size_t n_body = n - 2;
    for (size_t i = 0; i < n_body; i++) {
        int is_last = (i == n_body - 1);
        c->tail_position = is_last;
        compile_sexpr(c, e[2 + i]);
        if (!is_last) emit(c, LC_POP);
    }
    emit(c, LC_RETURN);

    /* Store each parameter name as a string constant in the body pool. */
    {
        Scope *ours = &c->scopes[c->n_scopes - 1];
        for (size_t i = 0; i < ours->n; i++) {
            LcValue v = val_zero(LC_VAL_STRING);
            v.str = str_dup(ours->entries[i].name);
            if (v.str == NULL) {
                fail(c, "CompileError: out of memory");
                break;
            }
            push_constant(c, v);
        }
    }

    /* Capture the compiled body. */
    LcCodeObject *body = (LcCodeObject *)malloc(sizeof(LcCodeObject));
    if (body == NULL) {
        fail(c, "CompileError: out of memory");
        free(c->instructions);
        for (size_t i = 0; i < c->n_const; i++)
            value_free_contents(&c->constants[i]);
        free(c->constants);
        for (size_t i = 0; i < c->n_names; i++) free(c->names[i]);
        free(c->names);
    } else {
        body->instructions = c->instructions;
        body->n_instructions = c->n_instr;
        body->constants = c->constants;
        body->n_constants = c->n_const;
        body->names = c->names;
        body->n_names = c->n_names;
    }

    /* Restore the outer buffers. */
    c->instructions = si;
    c->n_instr = sni;
    c->cap_instr = sci;
    c->constants = sco;
    c->n_const = snc;
    c->cap_const = scc;
    c->names = snm;
    c->n_names = snn;
    c->cap_names = scn;
    c->tail_position = saved_tail;
    c->in_function = saved_in;

    /* Exit the scope. */
    {
        Scope *ours = &c->scopes[c->n_scopes - 1];
        for (size_t i = 0; i < ours->n; i++) free(ours->entries[i].name);
        free(ours->entries);
        c->n_scopes--;
    }

    if (body == NULL) return; /* OOM already flagged */

    LcValue codev = val_zero(LC_VAL_CODE);
    codev.code = body;
    size_t idx = add_constant(c, codev);
    emit_with(c, LC_LOAD_CONST, idx);
    emit_with(c, LC_MAKE_CLOSURE, n_params);
}

static void compile_cond(Compiler *c, const LpSExpr **e, size_t n) {
    size_t n_clauses = n - 1; /* skip 'cond' */
    size_t *end_jumps = NULL;
    size_t n_end = 0, cap_end = 0;

    for (size_t ci = 0; ci < n_clauses && !c->failed; ci++) {
        const LpSExpr *clause = e[1 + ci];
        if (lp_sexpr_kind(clause) != LP_LIST) {
            fail(c, "CompileError: cond clause must be a list");
            break;
        }
        size_t parts = lp_sexpr_child_count(clause);
        if (parts < 2) {
            fail(c, "CompileError: cond clause needs predicate and expression");
            break;
        }
        const LpSExpr *predicate = lp_sexpr_child(clause, 0);
        const LpSExpr *expression = lp_sexpr_child(clause, parts - 1);

        if (is_symbol(predicate, "t")) {
            int saved_tail = c->tail_position;
            compile_sexpr(c, expression);
            c->tail_position = saved_tail;
        } else {
            int saved_tail = c->tail_position;
            c->tail_position = 0;
            compile_sexpr(c, predicate);
            c->tail_position = saved_tail;

            size_t false_jump = emit_jump(c, LC_JUMP_IF_FALSE);

            int saved_tail2 = c->tail_position;
            compile_sexpr(c, expression);
            c->tail_position = saved_tail2;

            size_t end_jump = emit_jump(c, LC_JUMP);
            if (!c->failed) {
                if (n_end == cap_end) {
                    size_t ncap = cap_end ? cap_end * 2 : 4;
                    size_t *nj =
                        (size_t *)realloc(end_jumps, ncap * sizeof(size_t));
                    if (nj == NULL)
                        fail(c, "CompileError: out of memory");
                    else {
                        end_jumps = nj;
                        cap_end = ncap;
                    }
                }
                if (!c->failed) end_jumps[n_end++] = end_jump;
            }
            patch_jump(c, false_jump);
        }
    }

    /* If there is no else clause, push NIL as the default. */
    int has_else = 0;
    if (n_clauses > 0) {
        const LpSExpr *last = e[n - 1];
        if (lp_sexpr_kind(last) == LP_LIST && lp_sexpr_child_count(last) > 0)
            has_else = is_symbol(lp_sexpr_child(last, 0), "t");
    }
    if (n_clauses == 0 || !has_else) emit(c, LC_LOAD_NIL);

    for (size_t i = 0; i < n_end; i++) patch_jump(c, end_jumps[i]);
    free(end_jumps);
}

static void compile_quote_form(Compiler *c, const LpSExpr **e, size_t n) {
    if (n != 2) {
        fail(c, "CompileError: quote takes exactly 1 argument");
        return;
    }
    compile_quoted_datum(c, e[1]);
}

static void compile_quoted_datum(Compiler *c, const LpSExpr *e) {
    if (c->failed) return;
    LpSExprKind k = lp_sexpr_kind(e);
    if (k == LP_ATOM) {
        LpAtomKind ak = lp_sexpr_atom_kind(e);
        const char *value = lp_sexpr_atom_value(e);
        if (ak == LP_NUMBER) {
            int64_t num;
            if (!parse_int(value, &num)) {
                char m[128];
                snprintf(m, sizeof m,
                         "CompileError: Invalid number in quote: %s", value);
                fail(c, m);
                return;
            }
            LcValue v = val_zero(LC_VAL_INTEGER);
            v.integer = num;
            emit_with(c, LC_LOAD_CONST, add_constant(c, v));
        } else if (ak == LP_STRING) {
            LcValue v = val_zero(LC_VAL_STRING);
            v.str = strip_quotes(value);
            if (v.str == NULL) {
                fail(c, "CompileError: out of memory");
                return;
            }
            emit_with(c, LC_LOAD_CONST, add_constant(c, v));
        } else { /* symbol */
            if (strcmp(value, "nil") == 0) {
                emit(c, LC_LOAD_NIL);
            } else {
                LcValue v = val_zero(LC_VAL_STRING);
                v.str = str_dup(value);
                if (v.str == NULL) {
                    fail(c, "CompileError: out of memory");
                    return;
                }
                emit_with(c, LC_MAKE_SYMBOL, add_constant(c, v));
            }
        }
    } else if (k == LP_LIST) {
        size_t count = lp_sexpr_child_count(e);
        emit(c, LC_LOAD_NIL);
        for (size_t i = count; i > 0 && !c->failed; i--) {
            compile_quoted_datum(c, lp_sexpr_child(e, i - 1));
            emit(c, LC_CONS);
        }
    } else if (k == LP_DOTTED_PAIR) {
        compile_quoted_datum(c, lp_sexpr_dotted_last(e));
        size_t count = lp_sexpr_child_count(e);
        for (size_t i = count; i > 0 && !c->failed; i--) {
            compile_quoted_datum(c, lp_sexpr_child(e, i - 1));
            emit(c, LC_CONS);
        }
    } else { /* quoted */
        compile_quoted_datum(c, lp_sexpr_quoted_inner(e));
    }
}

static void compile_cons(Compiler *c, const LpSExpr **e, size_t n) {
    if (n != 3) {
        fail(c, "CompileError: cons takes exactly 2 arguments");
        return;
    }
    int saved_tail = c->tail_position;
    c->tail_position = 0;
    compile_sexpr(c, e[2]); /* cdr goes below car */
    compile_sexpr(c, e[1]); /* car */
    c->tail_position = saved_tail;
    emit(c, LC_CONS);
}

static void compile_unary_op(Compiler *c, const LpSExpr **e, size_t n,
                             LcOp opcode) {
    if (n != 2) {
        char m[128];
        snprintf(m, sizeof m,
                 "CompileError: Unary op expects 1 argument, got %zu", n - 1);
        fail(c, m);
        return;
    }
    int saved_tail = c->tail_position;
    c->tail_position = 0;
    compile_sexpr(c, e[1]);
    c->tail_position = saved_tail;
    emit(c, opcode);
}

static void compile_binary_op(Compiler *c, const LpSExpr **e, size_t n,
                              LcOp opcode) {
    if (n != 3) {
        char m[128];
        snprintf(m, sizeof m,
                 "CompileError: Binary op expects 2 arguments, got %zu", n - 1);
        fail(c, m);
        return;
    }
    int saved_tail = c->tail_position;
    c->tail_position = 0;
    compile_sexpr(c, e[1]);
    compile_sexpr(c, e[2]);
    c->tail_position = saved_tail;
    emit(c, opcode);
}

static void compile_call(Compiler *c, const LpSExpr **e, size_t n) {
    const LpSExpr *func = e[0];
    size_t argc = n - 1;

    int saved_tail = c->tail_position;
    c->tail_position = 0;
    for (size_t i = 0; i < argc; i++) compile_sexpr(c, e[1 + i]);
    compile_sexpr(c, func);
    c->tail_position = saved_tail;

    if (c->tail_position && c->in_function)
        emit_with(c, LC_TAIL_CALL, argc);
    else
        emit_with(c, LC_CALL_FUNCTION, argc);
}

/* ── List dispatch ─────────────────────────────────────────────────────────*/

static void compile_list(Compiler *c, const LpSExpr **elements, size_t n) {
    if (c->failed) return;
    if (n == 0) {
        emit(c, LC_LOAD_NIL);
        return;
    }
    const char *sym = NULL;
    if (lp_sexpr_kind(elements[0]) == LP_ATOM &&
        lp_sexpr_atom_kind(elements[0]) == LP_SYMBOL)
        sym = lp_sexpr_atom_value(elements[0]);

    if (sym != NULL) {
        if (strcmp(sym, "define") == 0)
            compile_define(c, elements, n);
        else if (strcmp(sym, "lambda") == 0)
            compile_lambda(c, elements, n);
        else if (strcmp(sym, "cond") == 0)
            compile_cond(c, elements, n);
        else if (strcmp(sym, "quote") == 0)
            compile_quote_form(c, elements, n);
        else if (strcmp(sym, "cons") == 0)
            compile_cons(c, elements, n);
        else if (strcmp(sym, "car") == 0)
            compile_unary_op(c, elements, n, LC_CAR);
        else if (strcmp(sym, "cdr") == 0)
            compile_unary_op(c, elements, n, LC_CDR);
        else if (strcmp(sym, "atom") == 0)
            compile_unary_op(c, elements, n, LC_IS_ATOM);
        else if (strcmp(sym, "eq") == 0)
            compile_binary_op(c, elements, n, LC_CMP_EQ);
        else if (strcmp(sym, "print") == 0)
            compile_unary_op(c, elements, n, LC_PRINT);
        else if (strcmp(sym, "is-nil") == 0)
            compile_unary_op(c, elements, n, LC_IS_NIL);
        else {
            int aop = arithmetic_op(sym);
            int cop = comparison_op(sym);
            if (aop >= 0)
                compile_binary_op(c, elements, n, (LcOp)aop);
            else if (cop >= 0)
                compile_binary_op(c, elements, n, (LcOp)cop);
            else
                compile_call(c, elements, n);
        }
        return;
    }
    compile_call(c, elements, n);
}

static void compile_sexpr(Compiler *c, const LpSExpr *e) {
    if (c->failed) return;
    LpSExprKind k = lp_sexpr_kind(e);
    if (k == LP_ATOM) {
        compile_atom(c, lp_sexpr_atom_kind(e), lp_sexpr_atom_value(e));
    } else if (k == LP_LIST || k == LP_DOTTED_PAIR) {
        const LpSExpr **children;
        size_t nch;
        if (!list_children(e, &children, &nch)) {
            fail(c, "CompileError: out of memory");
            return;
        }
        compile_list(c, children, nch);
        free((void *)children);
    } else { /* quoted */
        compile_quoted_datum(c, lp_sexpr_quoted_inner(e));
    }
}

/* ── Public API ────────────────────────────────────────────────────────────*/

static void compiler_free_buffers(Compiler *c) {
    free(c->instructions);
    for (size_t i = 0; i < c->n_const; i++) value_free_contents(&c->constants[i]);
    free(c->constants);
    for (size_t i = 0; i < c->n_names; i++) free(c->names[i]);
    free(c->names);
    for (size_t i = 0; i < c->n_scopes; i++) {
        for (size_t j = 0; j < c->scopes[i].n; j++)
            free(c->scopes[i].entries[j].name);
        free(c->scopes[i].entries);
    }
    free(c->scopes);
}

int lc_compile_ast(const LpProgram *program, LcCodeObject *out,
                   LcCompileError *err) {
    Compiler c;
    memset(&c, 0, sizeof c);

    for (size_t i = 0; i < program->n && !c.failed; i++) {
        compile_sexpr(&c, program->exprs[i]);
        if (i + 1 < program->n) emit(&c, LC_POP);
    }
    emit(&c, LC_HALT);

    if (c.failed) {
        size_t k = 0;
        for (; c.errmsg[k] != '\0' && k + 1 < sizeof err->message; k++)
            err->message[k] = c.errmsg[k];
        err->message[k] = '\0';
        compiler_free_buffers(&c);
        return 0;
    }

    out->instructions = c.instructions;
    out->n_instructions = c.n_instr;
    out->constants = c.constants;
    out->n_constants = c.n_const;
    out->names = c.names;
    out->n_names = c.n_names;
    free(c.scopes); /* empty by now */
    return 1;
}

int lc_compile(const char *source, LcCodeObject *out, LcCompileError *err) {
    LpProgram program;
    LpError perr;
    if (!lp_parse(source, &program, &perr)) {
        snprintf(err->message, sizeof err->message,
                 "CompileError: Parse error: %s", perr.message);
        return 0;
    }
    int ok = lc_compile_ast(&program, out, err);
    lp_program_free(&program);
    return ok;
}
