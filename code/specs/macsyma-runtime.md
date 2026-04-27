# macsyma-runtime — MACSYMA-Specific Conventions and Built-ins

> **Status**: New spec. Defines a small Python package that holds
> everything that is genuinely MACSYMA/Maxima-specific and would NOT
> belong in a general CAS substrate.
> Parent: `symbolic-computation.md`. Companion: `macsyma-repl.md`.

## Why this package exists

The symbolic VM is designed as the 80–90% common substrate for any CAS:
arithmetic, calculus, simplification, factoring, solving, lists,
matrices. What's left over — the conventions of a particular language
— belongs in a thin per-language runtime. This is that package for
MACSYMA.

If we ever build a `mathematica-runtime`, a `maple-runtime`, a
`matlab-runtime`, or an `octave-runtime`, each of them sits at the same
position in the layer cake: above `symbolic-vm`, alongside
`<lang>-{lexer,parser,compiler}`, consumed by `<lang>-repl`.

## Reuse story

`macsyma-runtime` is **the only deliberately-non-reusable package** in
this work. Every other new package is generic. Things that a future
`matlab-runtime` would have to write from scratch:

- The MATLAB session's `ans` variable (analogue of MACSYMA's `%`).
- MATLAB's semicolon-suppress vs. echo-on-newline output rule.
- MATLAB's `clear` (analogue of MACSYMA's `kill`).
- MATLAB's `format long` etc. preferences.
- MATLAB's `script.m` batch loading.

Everything underneath — `Subst`, `Simplify`, `Factor`, etc. — is shared.

## Scope

In:

- **History**: `%`, `%i1`, `%o1`, `%i2`, `%o2`, ... — input and output
  history. The runtime owns this state; the REPL writes to it.
- **Statement-terminator semantics**: `;` (display), `$` (suppress).
  Lives here because the parser preserves the terminator token; the
  runtime is what acts on it.
- **`kill(symbols)`**: clear bindings. `kill(all)`, `kill(values)`,
  `kill(functions)`.
- **`ev(expr, ...flags)`**: re-evaluate `expr` with options
  (`numer`, `simp`, `expand`, `factor`, ...).
- **`block([locals], stmt1, stmt2, ...)`**: lexical scope.
- **MACSYMA globals**: `numer`, `simp`, `keepfloat`, `domain`,
  `prederror`, `algebraic`, `radexpand`. These are option flags
  consulted by the substrate (which receives them via the Backend).
- **Predicate aliases**: `is(predicate)`, `assume(facts)`,
  `forget(facts)` (skeletal).
- **Built-in name table**: maps every MACSYMA identifier the user
  types to the canonical IR head. Examples: `expand` → `Expand`,
  `factor` → `Factor`, `subst` → `Subst`, `solve` → `Solve`,
  `taylor` → `Taylor`, `limit` → `Limit`, `length` → `Length`,
  `first` → `First`, `rest` → `Rest`, `map` → `Map`. The
  `macsyma-compiler` consults this table.
- **Error formatting**: pretty MACSYMA-style error messages
  (`"Improper argument: …"`, `"Could not find a fixed point"`).

Out:

- The actual algorithms behind `Subst`/`Simplify`/`Factor`/etc. —
  those live in their own substrate packages (`cas-substitution`,
  `cas-simplify`, `cas-factor`, ...).
- The interactive loop itself — that's `macsyma-repl`.

## Public API

```python
from macsyma_runtime import (
    MacsymaBackend,            # subclass of SymbolicBackend with MACSYMA flags
    History,                   # the %i / %o table
    StatementTerminator,       # enum: DISPLAY, SUPPRESS
    register_macsyma_names,    # extends macsyma-compiler's name table
    MacsymaError,              # base error class
)
```

### `MacsymaBackend`

Subclasses `SymbolicBackend` from `symbolic-vm`. Adds:

- A reference to a `History` object that the REPL has access to.
- The MACSYMA option flags (`numer`, `simp`, `keepfloat`, ...) as
  attributes that downstream handlers can inspect.
- Handler overrides only where MACSYMA differs from the default
  symbolic semantics — e.g., `Equal(a, b)` returns `True` / `False`
  for fully-numeric arguments instead of staying symbolic.

### `History`

```python
class History:
    def record_input(self, ir: IRNode) -> int: ...   # returns index n
    def record_output(self, ir: IRNode) -> int: ...
    def get_input(self, n: int) -> IRNode: ...
    def get_output(self, n: int) -> IRNode: ...
    def last_output(self) -> IRNode | None: ...
    def reset(self) -> None: ...
```

The REPL calls `record_input`/`record_output` on each turn. The VM
resolves `%`, `%i1`, `%o1`, etc. via a lookup hook installed on the
backend.

### `kill`, `ev`, `block`, `assume`

Each is a head registered in `MacsymaBackend.handlers()`. The
implementations are short:

- `kill(name)` removes `name` from the binding environment.
  `kill(all)` resets the whole environment and the history.
- `ev(expr, *opts)` is `Simplify(expr)` plus per-flag re-evaluation —
  e.g., `ev(expr, numer)` evaluates with the `numer` flag set, which
  forces float collapse on transcendental constants.
- `block([locals], s1, s2, ..., sN)` evaluates statements in order
  inside a fresh scope, returning the last statement's value.

## Heads added

| Head      | Arity | Meaning                                      |
|-----------|-------|----------------------------------------------|
| `Kill`    | n     | Remove bindings.                             |
| `Ev`      | 1+    | Re-evaluate with flags.                      |
| `Block`   | 2+    | Lexical scope, sequential statements.        |
| `Assume`  | 1+    | Add fact to the assumption database.         |
| `Forget`  | 1+    | Remove a fact.                               |
| `Display` | 1     | Statement that should be printed (`;`).      |
| `Suppress`| 1     | Statement that should NOT be printed (`$`).  |

`Display` and `Suppress` are wrappers the compiler emits around each
top-level statement so the REPL can distinguish them. The VM unwraps
them; the REPL inspects the wrapper before evaluation to decide what
to print.

## Built-in name table

Extends `macsyma-compiler`'s `_STANDARD_FUNCTIONS` map. New entries:

| MACSYMA name | Canonical head |
|--------------|----------------|
| `expand`     | `Expand`       |
| `factor`     | `Factor`       |
| `simplify`   | `Simplify`     |
| `subst`      | `Subst`        |
| `solve`      | `Solve`        |
| `taylor`     | `Taylor`       |
| `limit`      | `Limit`        |
| `length`     | `Length`       |
| `first`      | `First`        |
| `rest`       | `Rest`         |
| `last`       | `Last`         |
| `append`     | `Append`       |
| `map`        | `Map`          |
| `apply`      | `Apply`        |
| `matrix`     | `Matrix`       |
| `transpose`  | `Transpose`    |
| `determinant`| `Determinant`  |
| `invert`     | `Inverse`      |
| `kill`       | `Kill`         |
| `ev`         | `Ev`           |
| `block`      | `Block`        |
| `assume`     | `Assume`       |
| `forget`     | `Forget`       |
| `is`         | `Is`           |
| `gcd`        | `Gcd`          |
| `lcm`        | `Lcm`          |
| `mod`        | `Mod`          |
| `floor`      | `Floor`        |
| `ceiling`    | `Ceiling`      |
| `abs`        | `Abs`          |

These heads are introduced by their respective substrate packages; the
runtime is the lookup table that says "when the MACSYMA user types
`factor`, route to `Factor`".

## Test strategy

- History records I/O and resolves `%`, `%i1`, `%o3` to the right IR.
- `kill(x)` removes `x` from a `MacsymaBackend` environment.
- `ev(2 + 3, numer)` returns `5.0` (or `5`, depending on flag policy).
- `block([x], x: 5, x + 1)` returns `6` and does NOT leak `x` to outer.
- `Display(expr)` evaluates to `expr`; `Suppress(expr)` evaluates to
  `expr` but the REPL must NOT print it.
- Name-table override: typing `factor(x^2-1)` reaches the `Factor`
  head.
- Coverage target: ≥80%.

## Package layout

```
code/packages/python/macsyma-runtime/
  pyproject.toml
  BUILD / BUILD_windows
  README.md
  CHANGELOG.md
  required_capabilities.json
  src/macsyma_runtime/
    __init__.py
    backend.py          # MacsymaBackend
    history.py          # %i / %o table
    handlers.py         # Kill, Ev, Block, Assume, Forget, Display, Suppress
    name_table.py       # Built-in name → IR head map
    errors.py           # MacsymaError + formatters
    py.typed
  tests/
    test_history.py
    test_kill.py
    test_ev.py
    test_block.py
    test_display_suppress.py
    test_name_table.py
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-symbolic-vm`,
`coding-adventures-cas-pretty-printer` (for error messages).

## Phasing

- **Phase A** (this PR): `History`, `Display`/`Suppress`, `Kill`,
  basic `Ev` (numer flag only), name-table additions for `Subst`,
  `Simplify`, `Expand`, `Factor`, `Solve`, `Taylor`, `Limit` even
  though those substrate packages are not yet implemented (so the
  parser/compiler accept the input).
- **Phase B+**: `Block`, full `Ev`, `Assume`/`Forget`/`Is`.

## Future extensions

- A `mathematica-runtime` would mirror this layout: `MathematicaBackend`,
  a `Hold`/`HoldComplete` system instead of `Display`/`Suppress`,
  `OwnValues`/`DownValues` rules, `MessageHandler`, `Print[]` semantics.
- A `matlab-runtime` would look very different: `ans`, `disp`, `clear`,
  workspace variables, `format` modes — but it would still consume
  `cas-substitution`/`cas-simplify`/`cas-matrix` underneath.
