# logic-core (C)

The semantic core of a **logic programming engine** (à la Prolog), in pure ISO
C17: terms, substitutions, and first-order **unification**. A faithful port of
the Rust [`logic-core`](../../rust/logic-core) crate.

## What it does

Logic programming reasons over *terms* — atoms, numbers, strings, variables, and
compound `functor(args…)` structures. A **substitution** binds variables to
terms. **Unification** asks: is there a substitution that makes two terms
syntactically equal? `lc_unify` answers, with the occurs-check enabled so
`X = f(X)` fails rather than building a cyclic term.

## API

- Constructors: `lc_atom`, `lc_int`, `lc_float`, `lc_string`, `lc_var_fresh` +
  `lc_term_var`, `lc_compound`, `lc_logic_list` (Prolog `'.'/2` cons cells).
- Terms: `lc_term_clone`, `lc_term_equal`, `lc_term_to_string`, `lc_term_free`.
- Substitutions: `lc_subst_empty`, `lc_subst_extend` (returns a NEW substitution),
  `lc_subst_walk` / `lc_subst_walk_var`, `lc_subst_len`, `lc_subst_equal`,
  `lc_subst_free`.
- `lc_unify(a, b, s)` → a new substitution, or `NULL` if they cannot unify.

## Ownership

An `LcTerm *` is an owned tree; the compound/list constructors take ownership of
the child terms handed to them. Substitutions and the terms returned by `walk` /
`unify` are owned by the caller (`lc_*_free`). `extend` never mutates its input —
it copies, so dropping the new value recovers the old (the shape backtracking
needs).

## Design notes

- **Persistent-in-spirit substitutions.** Every `extend` copies the binding map,
  matching the Rust crate; the API is already shaped for a sharing-aware
  representation later.
- **Faithful divergences.** Rust's `AtomicU64` variable-id counter becomes a
  plain `static` counter (single-threaded pure ISO; only distinct ids matter);
  variable display names live in a fixed inline buffer (cosmetic); float display
  uses `%g` (matches Rust's `1.0` → `"1"`).

## Usage

```c
#include "logic_core.h"

/* father(homer, X) ?= father(homer, bart)  ⇒  X = bart */
LcVar x = lc_var_fresh("X");
LcTerm *qa[2] = {lc_atom("homer"), lc_term_var(x)};
LcTerm *query = lc_compound("father", qa, 2);
LcTerm *fa[2] = {lc_atom("homer"), lc_atom("bart")};
LcTerm *fact = lc_compound("father", fa, 2);

LcSubst *empty = lc_subst_empty();
LcSubst *s = lc_unify(query, fact, empty);   /* non-NULL */
LcTerm *bound = lc_subst_walk_var(s, x);     /* atom "bart" */

lc_term_free(bound); lc_subst_free(s); lc_subst_free(empty);
lc_term_free(query); lc_term_free(fact);
```

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
