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
| `range` literal (`a..b` `a...b` `a..` `..b`) | — (Python `range` is half-open/integer-only; JS has none) | `sir-runtime-range` |
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

### `sir-runtime-range`
The SIR first-class `Range` value (a Ruby `a..b` / `a...b` literal lowers to
`BuiltinCall("range", [start, stop, exclusive])`). No faithful native form
exists — Python's `range` is half-open and integer-only and can't express the
inclusive or begin/endless forms, and JavaScript has no range type at all.
- `range(start, stop, exclusive) -> Range` — the constructor the backends emit
  (`_sir_range(...)` / `__SirRange.range(...)`). Either bound may be nil/`null`
  for the beginless (`..b`) / endless (`a..`) forms; `exclusive` selects `...`.
- `Range` is **iterable** (integers upward from `start`; an endless range yields
  forever — consume lazily; a beginless range raises on iteration, matching
  Ruby's `(..5).each`), supports **membership** (`includes` / Ruby `include?`),
  materialises with `to_list` / `toList` (Ruby `to_a`; raises on an unbounded
  range), and renders in Ruby notation (`1..5`, `1...5`, `1..`, `..5`).
- Zero dependencies (numeric ranges need no richer display). v0 covers integer
  ranges; non-integer stride / float ranges are out of scope.

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

## Out of scope

- Go and Rust backends (static typing + Ruby exceptions/OOP make them a separate,
  larger effort).
- **TypeScript call-position `**h` (double-splat).** Ruby `*x` / `**x` reach the
  backend as `BuiltinCall("splat"/"double_splat", [x])`; `splat` lowers natively
  to `*x` (Python) / `...x` (TS), and `double_splat` to `**x` in Python.
  TypeScript has no faithful form for `**h` in call position — JS has no
  keyword-argument call form and an SIR map is a `Map` (no object-literal/call
  spread) — so it is a documented v0 cut-line: it falls through to the eager
  dispatch, which raises a clear unknown-builtin error rather than emitting
  silently wrong code.
- **`defined?` runtime-presence fidelity.** Ruby `defined?(x)` reaches the
  backend as `BuiltinCall("defined?", [operand])` and must never evaluate its
  operand. Both backends inspect the operand's SIR shape at emit time and emit a
  constant description string (local→`"local-variable"`, const→`"constant"`,
  `@x`→`"instance-variable"`, `@@x`→`"class variable"`, `$x`→`"global-variable"`,
  builtin→`"method"`, anything else→`"expression"`). The **non-evaluation
  contract is honoured for every shape**. v0 simplifications: an instance/class/
  global variable reports its static description rather than the runtime
  `nil`-when-unset Ruby would give (no presence predicate in the per-concern
  runtimes yet), and a general/method-call operand reports the generic
  `"expression"` rather than Ruby's exact category (`"method"`, `"assignment"`,
  …). Both are non-evaluating and documented.
- Idiomatic-quality / style-transfer of emitted code (correct + readable, not
  hand-written-equivalent).
- Changes to `semantic-ir` core or any frontend — this is purely backend
  translation + runtime packaging. The reconciliation of cross-frontend truthiness
  (a JS frontend's `0 && x`) is a frontend-lowering concern, noted here only so the
  canonical SIR truthiness rule is unambiguous.
