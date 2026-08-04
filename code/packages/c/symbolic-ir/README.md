# symbolic-ir (C)

The universal symbolic-expression IR, in pure ISO C17 — a faithful port of the
Rust `symbolic-ir` crate. This is the one shared tree every computer-algebra
frontend compiles to and every backend consumes.

## The six node variants

| Variant | Payload | Notes |
|---------|---------|-------|
| `Symbol(name)` | string | variable / constant / operation head |
| `Integer(i64)` | `int64_t` | negative stored directly |
| `Rational(n, d)` | two `int64_t` | **always** reduced, `d > 0` |
| `Float(f64)` | `double` | |
| `Str(text)` | string | string literal |
| `Apply(head, args)` | node + node array | `head(arg0, arg1, …)` |

The single compound form `Apply` covers everything from `x + y` to
`Integrate(f(x), x, 0, 1)` — head and args are themselves nodes.

## API

```c
#include "symbolic_ir.h"

/* Build  Add(Pow(x, 2), 1)  */
SirNode *pow_args[2] = { sir_sym("x"), sir_int(2) };
SirNode *powx = sir_apply(sir_sym(SIR_POW), pow_args, 2);
SirNode *add_args[2] = { powx, sir_int(1) };
SirNode *expr = sir_apply(sir_sym(SIR_ADD), add_args, 2);

char *s = sir_to_string(expr);   /* "Add(Pow(x, 2), 1)" */
free(s);
sir_free(expr);                  /* recursive free */
```

- **Constructors** return a malloc-owned `SirNode *` (NULL on OOM).
- **`sir_apply` consumes** its `head` and argument nodes (ownership transferred,
  on failure too); free the result with `sir_free`.
- **`sir_rational`** reduces via GCD, moves the sign to the numerator, collapses
  to `Integer` when the denominator becomes 1, and returns `SIR_ERR_ZERO_DENOM`
  for a zero denominator.
- **`sir_equals`** is structural (floats by bit pattern); **`sir_hash`** is
  consistent with it.

## Divergence from the Rust crate

Rust's `rational` panics on a zero denominator; this port returns
`SIR_ERR_ZERO_DENOM`. Float `Display` uses the shortest `%g`-style round-tripping
decimal (always with a decimal point), matching Rust's `{:?}` for common values.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors` /
`/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).
