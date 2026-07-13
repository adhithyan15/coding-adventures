/*
 * logic_core.h — terms, substitutions, and first-order unification, pure ISO C17.
 * ==============================================================================
 *
 * A faithful port of the Rust `logic-core` crate — the data layer of a logic
 * programming engine (à la Prolog). It provides the *term universe*, a
 * *substitution* (variable → term map), and *unification* with the occurs-check.
 *
 * ## Terms
 *
 *   Atom       a zero-arity symbolic constant:  homer, []
 *   Num        an integer or float:             42, 3.14
 *   Str        a quoted string (distinct from an atom)
 *   Var        a bindable logic variable (identity is its numeric id)
 *   Compound   a functor applied to argument terms:  father(homer, bart)
 *
 * Lists use the canonical Prolog `'.'/2` cons-cell encoding:
 * `lc_logic_list([a, b])` builds `.(a, .(b, []))`.
 *
 * ## Substitution & unification
 *
 * A substitution binds variable ids to terms. It is *persistent in spirit*:
 * `lc_subst_extend` returns a NEW substitution and never mutates the old one, so
 * backtracking (a later layer) is just dropping references. `lc_unify` returns a
 * new substitution making two terms syntactically equal, or NULL if impossible.
 * The occurs-check is enabled, so `X = f(X)` fails rather than looping.
 *
 * ## Ownership
 *
 * An `LcTerm *` is an owned tree; constructors that take child terms or an args
 * array TAKE OWNERSHIP of them. Release a term with `lc_term_free`. Functions
 * returning `LcTerm *` / `LcSubst *` hand back owned values the caller frees.
 *
 * ## Faithful divergences
 *
 * - Rust's process-wide `AtomicU64` id counter becomes a plain `static` counter:
 *   pure ISO C is single-threaded here, and only *distinct ids* are observable.
 * - Variable display names live in a fixed inline buffer (cosmetic only; never
 *   affects identity or unification).
 * - Float display uses C `%g` (matches the Rust `{}` output for integer-valued
 *   floats such as `1.0` → `"1"`).
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef LOGIC_CORE_H
#define LOGIC_CORE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int64_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Numbers ───────────────────────────────────────────────────────────────*/

typedef enum { LC_INT, LC_FLOAT } LcNumKind;

/* A numeric term. Integers and floats are distinct: `1` and `1.0` are NOT
 * equal (Prolog tradition). */
typedef struct {
    LcNumKind kind;
    int64_t i; /* valid when kind == LC_INT */
    double f;  /* valid when kind == LC_FLOAT */
} LcNumber;

/* ── Variables ─────────────────────────────────────────────────────────────*/

#define LC_VAR_NAME_CAP 32

/* A bindable variable whose identity is its `id`. Two variables are equal iff
 * their ids match; the display name is cosmetic. Trivially copyable. */
typedef struct {
    uint64_t id;
    char display_name[LC_VAR_NAME_CAP]; /* "" means "no name" (prints _G<id>) */
} LcVar;

/* Allocate a brand-new variable with a fresh, unique id. `display_name` may be
 * NULL; a long name is truncated to fit the inline buffer. */
LcVar lc_var_fresh(const char *display_name);

/* ── Terms ─────────────────────────────────────────────────────────────────*/

typedef struct LcTerm LcTerm;

/* Constructors. Each returns a freshly-owned term, or NULL on allocation
 * failure (freeing any child terms/args they were handed). */
LcTerm *lc_atom(const char *name);
LcTerm *lc_int(int64_t value);
LcTerm *lc_float(double value);
LcTerm *lc_string(const char *value);
LcTerm *lc_term_var(LcVar v);
/* Consume `args[0..n)` (owned) into a compound `functor(args...)`. The `args`
 * array itself is not freed. */
LcTerm *lc_compound(const char *functor, LcTerm **args, size_t n);
/* Consume `items[0..n)` into the cons-cell list `.(i0, .(i1, … []))`. */
LcTerm *lc_logic_list(LcTerm **items, size_t n);

/* Deep-copy a term. NULL on OOM. */
LcTerm *lc_term_clone(const LcTerm *t);
/* Structural equality (variables compare by id). */
int lc_term_equal(const LcTerm *a, const LcTerm *b);
/* Human-readable form (owned string; caller frees). NULL on OOM. */
char *lc_term_to_string(const LcTerm *t);
/* Release a term and, recursively, everything it owns. NULL-safe. */
void lc_term_free(LcTerm *t);

/* ── Substitutions ─────────────────────────────────────────────────────────*/

typedef struct LcSubst LcSubst;

/* The empty substitution. NULL on OOM. */
LcSubst *lc_subst_empty(void);
/* Return a NEW substitution binding `var_id` to a copy of `term`; the original
 * is unchanged. NULL on OOM. */
LcSubst *lc_subst_extend(const LcSubst *s, uint64_t var_id, const LcTerm *term);
/* Chase variable bindings until a non-variable or an unbound variable is
 * reached; returns an owned copy of that term. NULL on OOM. */
LcTerm *lc_subst_walk(const LcSubst *s, const LcTerm *term);
/* Convenience: walk a variable. */
LcTerm *lc_subst_walk_var(const LcSubst *s, LcVar v);
/* Number of bindings. */
size_t lc_subst_len(const LcSubst *s);
/* Equality: same set of (id → term) bindings. */
int lc_subst_equal(const LcSubst *a, const LcSubst *b);
/* Release a substitution and its bound terms. NULL-safe. */
void lc_subst_free(LcSubst *s);

/* First-order unification with occurs-check. Returns a NEW substitution making
 * `a` and `b` syntactically equal under some extension of `s`, or NULL if they
 * cannot be unified (or on OOM). The caller frees the result. */
LcSubst *lc_unify(const LcTerm *a, const LcTerm *b, const LcSubst *s);

#ifdef __cplusplus
}
#endif

#endif /* LOGIC_CORE_H */
