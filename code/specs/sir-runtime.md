# sir-runtime — importable runtime packages for SIR backends

## Status

New. Builds on and partially supersedes the runtime portions of
[SIR14](SIR14-semantic-ir-to-python.md) (Python backend),
[SIR16](SIR16-ir-extensions-for-python-and-javascript.md) (IR extensions),
and [SIR20](SIR20-semantic-ir-to-python-v1-extension.md) (Python v1 extension).

Those specs say "every backend accepts the full SIR-v1 feature set," but the
implementation never landed — `semantic-ir-to-python` and
`semantic-ir-to-typescript` still accept only the 8 SIR-v0 features and `panic!`
on every newer node (`Seq*`, `Map*`, `Logical*`, `Assign`, loops, classes,
exceptions, interpolation). This spec defines the architecture that closes that
gap **and** carries it further — to the full surface the Ruby frontend
(`ruby-to-semantic-ir`) already emits — while changing *how* the runtime is
delivered.

### Current state vs. target (2026-06-21 audit — honest baseline)

This document is the **target architecture**, not the shipped state. As of the
2026-06-21 audit:

- **`sir-runtime-core` is the only package that exists** (Python + TypeScript). The
  per-concern packages below (`-pairs`, `-oop`, `-exceptions`, `-regex`, `-shell`)
  are **not yet created**; pairs live inside `sir-runtime-core` for now.
- The **Python backend** accepts the v0 eight plus `Floats, Sequences, Maps,
  ShortCircuit, StringInterpolation` (expressions only). It still `panic!`s on
  `MutableBindings`, `Loops`, `Classes`, `Modules`, `InstanceVars`, `ClassVars`,
  `Constants`, `Exceptions`, and routes `__method__` / `block_pass` to the builtin
  dispatch (runtime "unknown builtin").
- The **TypeScript backend** accepts only the v0 eight — not even the expression
  features above.

The work to reach the target is tracked, phase by phase, in
[`sir-backend-completion-plan.md`](sir-backend-completion-plan.md). Each phase ships
as its own PR; this contract is updated as packages and features land so it never
again drifts ahead of the code.

## Why this exists

A backend turns a `semantic_ir::Module` into target source. Most SIR constructs
have a faithful **native** equivalent in Python and TypeScript/JS — a sequence is
a `list`/`Array`, a map is a `dict`/`Map`, a loop is `for`/`while`, a class is a
`class`. Those should translate **directly to native code**, with nothing
imported.

A handful of SIR semantics have **no faithful native equivalent**:

- **SIR truthiness** — only `false` and `nil` are falsy (the Lisp/Ruby
  convention). Python and JS natively treat `0`, `""`, `[]`, `{}` as falsy too.
  Emitting a bare `and` / `&&` would silently change behaviour.
- **Symbols** (`:foo`) — interned identity objects; no native Python/JS type.
- **Pairs** (`cons`/`car`/`cdr`) — Lisp cons cells; no native type.
- **Exception class matching** against the SIR class model; **regex** flavour/flag
  compatibility; **backtick** shell-out — each needs a small, explicit shim.

These quirks live in **small, per-concern, importable runtime packages**. Emitted
code imports them and calls in. Nothing language-specific is baked into the SIR or
inlined into user code.

### Keyed to SIR, not to any one frontend

The packages implement **SIR** semantics, not Ruby's. That is the whole point of a
narrow waist: the same Python backend + same `sir-runtime-*` packages serve a Ruby
frontend today and a JavaScript or Python frontend tomorrow (the JS → SIR → Python
direction that SIR16/SIR20 set out to enable). A frontend is responsible for
lowering its language's semantics *into* SIR semantics; the backend and runtime
then implement SIR faithfully and uniformly.

## Principle: translate-first, library-only-for-quirks

| SIR construct | Python / TS native emit | Runtime package |
|---|---|---|
| `IntLit` `FloatLit` `BoolLit` `StrLit` | literal | — |
| `NilLit` | `None` / `null` | — |
| `SeqLit` `SeqIndex` `SeqLen` `SeqSet` | `[...]`, `s[i]`, `len(s)`/`.length`, `s[i]=v` | — |
| `MapLit` `MapGet` `MapSet` | `{...}`/`new Map`, `m[k]`, `m[k]=v` | — |
| `Assign` `While` `ForRange` `ForEach` | `=`, `while`, `for` | — |
| `If` / loop conditions | native `if`/`while` (condition wrapped) | `core.truthy(cond)` |
| `LogicalAnd` `LogicalOr` | lazy native ternary (condition wrapped) | `core.truthy` |
| `StrConcat` (interpolation) | f-string / template literal | `core.to_display` for non-`str` parts |
| `ClassDef` `ModuleDef` | native `class` / namespace | — |
| `VarRef`/`Assign` scopes `Instance` `ClassVar` `Const` | `self.x`/`this.x`, class store, `CONST` | — |
| `SingletonClassDef`, `is_a?` | native where possible | `sir-runtime-oop` for the rest |
| `TryCatch` / `raise` | native `try/except`/`finally` | `sir-runtime-exceptions` |
| `SymLit` / symbol ops | — | `sir-runtime-core` |
| `cons` `car` `cdr` (Pairs) | — | `sir-runtime-pairs` |
| `regex` literal | native `re` / `RegExp` engine | `sir-runtime-regex` (flavour/flags) |
| `backtick` shell-out | native subprocess | `sir-runtime-shell` |
| `+ - * / = < > <= >= != %` | native operators where SIR semantics match | `sir-runtime-core` for SIR-specific (variadic, truncating `/`) |

### The truthiness rule (corrects SIR20)

SIR truthiness is canonical and **false/nil-only**:

```text
truthy(v)  ==  v is not false  AND  v is not nil
```

So `0`, `0.0`, `""`, `[]`, `{}`, a symbol, a pair — all **truthy**. This matches
the existing `_sir_truthy` in the shipped runtime and the Ruby/Lisp frontends.
SIR20's note that "Python's native `bool()` matches" is **incorrect** for this
convention and is hereby superseded: backends MUST route every condition through
`core.truthy(...)` and MUST NOT emit a bare `and`/`or`/`if x`.

`LogicalAnd`/`LogicalOr` stay **lazy** (rhs unevaluated when lhs decides it) *and*
use SIR truthiness:

- Python `&&`: `(rhs if core.truthy(__l := lhs) else __l)`
- TS `&&`: `((__l) => core.truthy(__l) ? (rhs) : __l)(lhs)`
- `||` mirrors (return lhs when truthy, else rhs).

## The packages

Each is a standard publishable package under `code/packages/python/<name>/` and
`code/packages/typescript/<name>/` (pyproject.toml / package.json, `src/` src
layout, `tests/`, `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md`,
`required_capabilities.json`; TS also `tsconfig.json`, `vitest.config.ts`).
Python: full type annotations, `mypy --strict`, ruff. TS: strict mode + lint.
Coverage target 95%.

### `sir-runtime-core` (always imported)
The base every emitted program needs.
- `truthy(v) -> bool` — SIR truthiness (false/nil-only).
- `Symbol` + `intern(name) -> Symbol` — interned symbol identity.
- `eq(a, b) -> bool` — SIR equality (symbol-aware).
- `to_display(v) -> str` — SIR display form (`nil`, `#t`/`#f`, symbol name, …);
  used by `print` and by `StrConcat` for non-string parts.
- `print(v)` — display + newline.
- arithmetic/compare that differ from native: variadic `add`/`sub`/`mul`,
  truncating integer `div`, plus `lt`/`gt`/`le`/`ge`/`ne` where coercion matters.
- closure-handle helpers (`Closure`, `apply`, `make_closure`) and the global
  store (`global_set`/`global_get`/`global_get_static`) used by `IndirectCall`,
  builtin-as-value, and `Globals`.

### `sir-runtime-pairs`
`Pair`, `cons`, `car`, `cdr`, `is_pair`; Lisp list display via `Pair.__repr__` /
`toString` (so `core.to_display` needs no dependency on pairs).

### `sir-runtime-oop`
Only the OOP bits with no clean native form: singleton-method attachment
(`define_singleton_method`-style), and SIR-class-model `is_a?` / scoped-constant
(`Foo::Bar`) resolution where native `isinstance`/member access is insufficient.

### `sir-runtime-exceptions`
`SirError` base + a class registry; `raise(cls, msg)`; `rescues(err, types)`
predicate used to dispatch a `TryCatch`'s typed rescue clauses to native
`except`/`catch`.

### `sir-runtime-regex`
Compile a SIR `regex` literal (pattern + flags) to a native engine object with
flavour/flag compatibility (e.g. Ruby `/i`, `/m`, `/x` → `re`/`RegExp` flags),
plus match/scan helpers.

### `sir-runtime-shell`
`backtick(command) -> str` — run a command and capture stdout (Python
`subprocess`, TS `child_process.execSync`), preserving the SIR contract for exit
status / output.

## How backends consume them

`emit_module` emits an import header for exactly the packages the module's
features require, then `emit_expr`/`emit_stmt` translate native-first and call
`core.*` / `pairs.*` / … for the quirks. Example (Python):

```python
from sir_runtime_core import truthy, intern, to_display, print as _print
# (pairs/oop/exceptions/regex/shell imported only when used)

def add(a, b):
    return a + b

xs = [1, 2, 3]
i = 0
while truthy(i < len(xs)):
    _print(xs[i])
    i = i + 1
```

No `_sir_*` prelude is pasted into the file; the semantics live in the imported
packages. The backend chooses, per node, native-or-import per the table above.

`accepts_features()` grows to the full set the Ruby frontend emits
(`Floats, Sequences, Maps, ShortCircuit, MutableBindings, Loops, Classes, Modules,
InstanceVars, ClassVars, Constants, Exceptions, StringInterpolation` on top of the
v0 eight). `TailCalls` and `Intrinsics` remain rejected.

## Verification

- Backend unit + Ruby→backend source-shape tests (native constructs + the right
  imports).
- **Execution tests**: run the emitted code through `python3` / `node` with the
  local `sir-runtime-*` packages on `PYTHONPATH` / module-resolution path; assert
  stdout; skip gracefully when the interpreter is absent.
- Short-circuit laziness + SIR-truthiness tests: `false && side_effect()` must not
  fire; `0 && x` must yield `x` (0 is truthy under SIR), distinguishing SIR from
  native truthiness.
- Each runtime package: own unit tests, `mypy --strict` + ruff / TS-strict + lint,
  coverage ≥ 95%.

## In scope for core changes (updated 2026-06-21)

The original out-of-scope note forbade `semantic-ir` core and frontend changes. That
restriction is **lifted** for the completion effort: where a faithful implementation
requires it, the `semantic-ir` core schema and the `ruby-to-semantic-ir` frontend
**may** change. Known core changes on the roadmap:

- a **variadic `Param` kind** so `*args` / `**kwargs` survive lowering (today the
  splat prefix is dropped);
- a **map has-key primitive** so hash-pattern key-presence is enforced faithfully;
- a **first-class sequence-slice** (or an executed `__seq_slice__`) so array
  one-splat patterns bind the middle.

Core changes affect every frontend/backend, so each is specced and validated at the
SIR boundary before dependent backend work builds on it.

## Out of scope

- **Go and Rust backends** (static typing + Ruby exceptions/OOP make them a separate,
  larger effort). Confirmed out of scope on 2026-06-21.
- Idiomatic-quality / style-transfer of emitted code (correct + readable, not
  hand-written-equivalent). The reconciliation of cross-frontend truthiness
  (a JS frontend's `0 && x`) is a frontend-lowering concern, noted here only so the
  canonical SIR truthiness rule is unambiguous.
