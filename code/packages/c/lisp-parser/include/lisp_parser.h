/*
 * lisp_parser.h — parse token streams into S-expression ASTs, pure ISO C17.
 * ========================================================================
 *
 * A faithful port of the Rust `lisp-parser` crate. It sits on top of the
 * sibling `lisp-lexer`: tokens in, a tree of S-expressions out. Lisp's grammar
 * is tiny — just 6 rules — so this is a small recursive-descent parser.
 *
 *   program = { sexpr }
 *   sexpr   = atom | list | quoted
 *   atom    = NUMBER | SYMBOL | STRING
 *   list    = '(' { sexpr } ')'         (may end '. sexpr' → a dotted pair)
 *   quoted  = "'" sexpr                 ('x is sugar for (quote x))
 *
 * An `LpSExpr` is one of: an Atom (a kind + its source text), a List, a
 * DottedPair (elements + a final cdr), or a Quoted form.
 *
 * ## Ownership
 *
 * A successful `lp_parse` fills an `LpProgram` you release with
 * `lp_program_free`. Query helpers that return an `LpStrList` hand back owned
 * storage you release with `lp_strlist_free`.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef LISP_PARSER_H
#define LISP_PARSER_H

#include <stddef.h> /* size_t */

#include "lisp_lexer.h" /* LlToken, ll_tokenize */

#ifdef __cplusplus
extern "C" {
#endif

/* The kind of atom (terminal value) in an S-expression. */
typedef enum { LP_NUMBER, LP_SYMBOL, LP_STRING } LpAtomKind;

/* The kind of S-expression node. */
typedef enum { LP_ATOM, LP_LIST, LP_DOTTED_PAIR, LP_QUOTED } LpSExprKind;

/* An S-expression node (opaque; inspect via the accessors below). */
typedef struct LpSExpr LpSExpr;

/* A parsed program: the top-level forms, in source order. */
typedef struct {
    LpSExpr **exprs; /* owned array of owned nodes */
    size_t n;
} LpProgram;

/* A parse (or lexer) error. `message` is a fixed, self-contained description. */
typedef struct {
    char message[128];
} LpError;

/* An owned list of strings (e.g. the atom values collected from a tree). */
typedef struct {
    char **items;
    size_t n;
} LpStrList;

void lp_strlist_free(LpStrList *list);

/* Parse Lisp `source` into a program. Returns 1 on success (fills *out; release
 * with lp_program_free), 0 on a lexer or parse error (fills *err). */
int lp_parse(const char *source, LpProgram *out, LpError *err);

/* Parse a pre-tokenized stream (the tokens are borrowed, not consumed).
 * `tokens` should end with an LL_EOF token, as ll_tokenize produces. */
int lp_parse_tokens(const LlToken *tokens, size_t n_tokens, LpProgram *out,
                    LpError *err);

void lp_program_free(LpProgram *program);

/* ── Node inspection ───────────────────────────────────────────────────────*/

LpSExprKind lp_sexpr_kind(const LpSExpr *e);
/* For an LP_ATOM node: its atom kind and source text. */
LpAtomKind lp_sexpr_atom_kind(const LpSExpr *e);
const char *lp_sexpr_atom_value(const LpSExpr *e);

/* Recursively collect every atom value in `e` (owned; free with
 * lp_strlist_free). */
LpStrList lp_sexpr_find_atoms(const LpSExpr *e);
/* Number of List / DottedPair nodes in `e`. */
size_t lp_sexpr_count_lists(const LpSExpr *e);
/* Number of Quoted nodes in `e`. */
size_t lp_sexpr_count_quoted(const LpSExpr *e);

/* Program-level convenience: apply the above across all top-level forms. */
LpStrList lp_program_find_atoms(const LpProgram *program);
size_t lp_program_count_lists(const LpProgram *program);
size_t lp_program_count_quoted(const LpProgram *program);

#ifdef __cplusplus
}
#endif

#endif /* LISP_PARSER_H */
