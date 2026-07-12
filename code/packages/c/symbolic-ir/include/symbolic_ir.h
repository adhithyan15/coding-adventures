/*
 * symbolic_ir.h — the universal symbolic-expression IR, in pure ISO C17.
 * A faithful port of the Rust `symbolic-ir` crate.
 * ===========================================================================
 *
 * A computer-algebra system needs one shared tree every frontend compiles to
 * and every backend consumes. That tree is `SirNode`, one of six variants:
 *
 *   Symbol(name)     named atom: variable, constant, or operation head
 *   Integer(i64)     64-bit integer literal
 *   Rational(n, d)   exact fraction, ALWAYS reduced with d > 0
 *   Float(f64)       double-precision literal
 *   Str(text)        string literal
 *   Apply(head,args) compound: head(arg0, arg1, ...)
 *
 * The single compound form `Apply` covers everything from `x + y` to
 * `Integrate(f(x), x, 0, 1)` — head and args are themselves nodes, so
 * higher-order expressions work naturally.
 *
 * OWNERSHIP. Constructors return a malloc-owned `SirNode *` (NULL on OOM).
 * `sir_apply` CONSUMES its head and argument nodes (it takes ownership on both
 * success and failure). Free any node with `sir_free`, which recurses.
 *
 * EQUALITY. Structural, matching the Rust `PartialEq`: floats compare by raw bit
 * pattern (so two identical-bit NaNs are equal), Apply compares recursively.
 * `sir_hash` is consistent with equality (equal nodes hash equal).
 *
 * DIVERGENCE FROM RUST. Rust's `rational` panics on a zero denominator; this
 * port returns `SIR_ERR_ZERO_DENOM` from `sir_rational`. Float `Display` uses
 * the shortest `%g`-style round-tripping decimal (always with a decimal point),
 * which matches Rust's `{:?}` for the common cases.
 *
 * PORTABILITY. Pure ISO C17 — no compiler extensions. Builds clean under GCC,
 * Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_SYMBOLIC_IR_H
#define CA_SYMBOLIC_IR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SIR_VERSION "0.2.0"

/* The six node variants. */
typedef enum {
    SIR_SYMBOL,
    SIR_INTEGER,
    SIR_RATIONAL,
    SIR_FLOAT,
    SIR_STR,
    SIR_APPLY
} SirKind;

/* Status of a fallible operation. */
typedef enum {
    SIR_OK = 0,
    SIR_ERR_NOMEM,
    SIR_ERR_ZERO_DENOM /* rational with a zero denominator */
} SirStatus;

typedef struct SirNode SirNode;

/* ── Constructors (malloc-owned; NULL on OOM) ────────────────────────────── */
SirNode *sir_sym(const char *name);
SirNode *sir_int(int64_t n);
SirNode *sir_flt(double v);
SirNode *sir_str(const char *s);

/* Build a Rational in fully reduced form (sign in the numerator, denominator
 * positive), collapsing to Integer when the denominator reduces to 1. Writes
 * the node to *out and returns SIR_OK, or SIR_ERR_ZERO_DENOM (denom == 0) /
 * SIR_ERR_NOMEM. */
SirStatus sir_rational(int64_t numer, int64_t denom, SirNode **out);

/* Build Apply(head, args[0..n_args)). CONSUMES `head` and every element of
 * `args` (ownership is transferred, on failure too). The `args` array itself is
 * borrowed — the caller still frees that container. Returns NULL on OOM (having
 * freed head and the args). */
SirNode *sir_apply(SirNode *head, SirNode **args, size_t n_args);

/* ── Accessors (borrowed) ────────────────────────────────────────────────── */
SirKind sir_kind(const SirNode *n);
const char *sir_symbol_name(const SirNode *n);           /* SIR_SYMBOL */
int64_t sir_integer_value(const SirNode *n);             /* SIR_INTEGER */
void sir_rational_parts(const SirNode *n, int64_t *numer, int64_t *denom);
double sir_float_value(const SirNode *n);                /* SIR_FLOAT */
const char *sir_str_value(const SirNode *n);             /* SIR_STR */
const SirNode *sir_apply_head(const SirNode *n);         /* SIR_APPLY */
size_t sir_apply_arity(const SirNode *n);                /* SIR_APPLY */
const SirNode *sir_apply_arg(const SirNode *n, size_t i);/* SIR_APPLY */

/* ── Operations ──────────────────────────────────────────────────────────── */
/* Structural equality (1 = equal). Floats compared by bit pattern. */
int sir_equals(const SirNode *a, const SirNode *b);
/* A hash consistent with sir_equals (equal nodes hash equal). */
uint64_t sir_hash(const SirNode *n);
/* Pretty-print to a malloc'd string (caller frees); NULL on OOM. */
char *sir_to_string(const SirNode *n);
/* Recursively free a node and all it owns. */
void sir_free(SirNode *n);

/* ── Standard head-name constants ────────────────────────────────────────── */
/* Arithmetic */
#define SIR_ADD "Add"
#define SIR_SUB "Sub"
#define SIR_MUL "Mul"
#define SIR_DIV "Div"
#define SIR_POW "Pow"
#define SIR_NEG "Neg"
#define SIR_INV "Inv"
/* Elementary functions */
#define SIR_EXP "Exp"
#define SIR_LOG "Log"
#define SIR_SIN "Sin"
#define SIR_COS "Cos"
#define SIR_TAN "Tan"
#define SIR_SQRT "Sqrt"
#define SIR_ATAN "Atan"
#define SIR_ASIN "Asin"
#define SIR_ACOS "Acos"
/* Hyperbolic functions */
#define SIR_SINH "Sinh"
#define SIR_COSH "Cosh"
#define SIR_TANH "Tanh"
#define SIR_ASINH "Asinh"
#define SIR_ACOSH "Acosh"
#define SIR_ATANH "Atanh"
#define SIR_COTH "Coth"
#define SIR_SECH "Sech"
#define SIR_CSCH "Csch"
/* Calculus */
#define SIR_D "D"
#define SIR_INTEGRATE "Integrate"
/* Relations */
#define SIR_EQUAL "Equal"
#define SIR_NOT_EQUAL "NotEqual"
#define SIR_LESS "Less"
#define SIR_GREATER "Greater"
#define SIR_LESS_EQUAL "LessEqual"
#define SIR_GREATER_EQUAL "GreaterEqual"
/* Logic */
#define SIR_AND "And"
#define SIR_OR "Or"
#define SIR_NOT "Not"
#define SIR_IF "If"
/* Containers and binding */
#define SIR_LIST "List"
#define SIR_ASSIGN "Assign"
#define SIR_DEFINE "Define"
#define SIR_RULE "Rule"

#ifdef __cplusplus
}
#endif

#endif /* CA_SYMBOLIC_IR_H */
