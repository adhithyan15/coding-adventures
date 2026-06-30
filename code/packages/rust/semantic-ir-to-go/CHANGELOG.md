# Changelog

## 0.2.0 — SIR16 Floats + ShortCircuit (A4)

First two SIR16 (v1) features land in the Go backend, mirroring the
just-merged Rust backend equivalent.  Before this release the Go backend
declared *none* of the six SIR16 features, so every SIR16 IR node hit a
`panic!` reject arm.  This release wires up two of them end-to-end.

### Added

- `Feature::Floats` and `Feature::ShortCircuit` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **Floats** — the inlined Go runtime's `Value` (`interface{}`) now
  accepts a `float64` arm:
  - New helpers `_sir_as_float`, `_sir_any_float`, `_sir_is_number_val`,
    and `_sir_format_float`.
  - Arithmetic (`+ - * /`) keeps the exact int64 fast-path while every
    operand is an integer, and promotes the whole fold to `float64` the
    moment any operand is a float ("int op float ⇒ float").  Integer
    division keeps its divide-by-zero panic; float division follows
    IEEE-754 (`1.0/0.0 ⇒ +Inf`).
  - `=` is cross-type for numbers (`1 == 1.0` is true) and uses IEEE
    equality for floats (`NaN != NaN`).  `<` / `>` compare numerically,
    staying on the int path when both operands are int64.
  - `number?` is true for both integers and floats.
  - `FloatLit` emits `Value(float64(<lit>))`; integral values spell out
    `3.0` (never `3`) so the runtime type-switch hits the float arm.
    Non-finite values route through `math.NaN()` / `math.Inf(±1)` since
    Go has no float literal for them.
  - Display: `_sir_format_float` prints integral floats with a trailing
    `.0` (`3.0`, not Go's default `%v`-style `3`), fractional values via
    `strconv.FormatFloat(x, 'g', -1, 64)`, and non-finite values as
    `NaN` / `inf` / `-inf` — matching the Rust backend's intent.
- **ShortCircuit** — `LogicalAnd` / `LogicalOr` emit a truthy-guarded
  immediately-invoked func literal:
  `func() Value { __l := <lhs>; if _sir_truthy(__l) { return <rhs> } else { return __l } }()`
  (and the mirror for `or`).  The operand value is returned (not a
  coerced bool), `lhs` is evaluated exactly once, and each IIFE scopes
  its own `__l` so nesting never collides.  Pure emit — no runtime
  change.
- The emitter now imports `"math"` (alongside `"fmt"` and `"strconv"`);
  the runtime always references it via the float `NaN`/`Inf` checks, so
  Go's unused-import rule stays satisfied.
- Integration test `tests/compile_and_run_floats.rs`: hand-builds a SIR
  module exercising floats, short-circuit, and cross-type equality;
  emits Go, runs it with `go run`, and asserts stdout
  (`4.0 / 4.0 / 5 / 7 / #f / #t`).  Gated on `go version` — skips with a
  log line if the Go toolchain is absent.

### Notes

- The remaining four SIR16 features (MutableBindings, Loops, Sequences,
  Maps) are still **not** declared, so the corresponding emit arms
  (`SeqLit`, `MapLit`, `Assign`, `While`, …) remain reachable only as
  internal-bug `panic!`s — the capability check rejects such modules
  before emit.  They land in later Go PRs.

## 0.1.2 — SIR18 exhaustiveness (no behaviour change)

semantic-ir 0.10.0 adds `Expr::StrConcat` (the SIR18 string-concat
node).  This backend gains a `StrConcat` arm in its expression emitter
so it stays exhaustive.  The arm joins the existing SIR16+ reject group
and `panic!`s with a "capability check should have rejected it"
message: `Feature::StringInterpolation` is not in this backend's
accepted-feature set, so a concat-using module is rejected at the
capability check before emit, making the arm unreachable.  No output or
accepted-feature changes.

## 0.1.1 — SIR17 exhaustiveness (no behaviour change)

semantic-ir 0.2.0 adds `Stmt::ClassDef` (the SIR17 class node).  This
backend gains a `ClassDef` match arm in its statement emitter so it
stays exhaustive.  The arm `panic!`s with a "capability check should
have rejected it" message: `Feature::Classes` is not in this
backend's accepted-feature set, so a class-using module is rejected
at the capability check before emit, making the arm unreachable.  No
output or accepted-feature changes.

## 0.1.0 — initial release (SIR15 v0)

Fourth backend for the narrow-waist Semantic IR.  Emits
self-contained Go source from a `semantic_ir::Module`.

### Added

- `GoBackend` implementing `semantic_ir::Backend` with
  `target_tag = "go"`; accepts the v0 feature set minus
  `TailCalls` and `Intrinsics`.
- Per-node lowering per SIR15.  Notable Go-isms:
  - `If` and non-trivial `Block` lower to immediately-invoked
    function expressions (`func() Value { ... }()`) since Go has
    no expression-position blocks.
  - `MakeClosure` emits an adapter `func([]Value) Value` that
    splats the runtime args into the synthesised lambda's
    positional parameters; the per-function arity table is
    threaded through TLS so the splat is sized correctly.
  - `LetBinding` emits `name := value` followed by a defensive
    `_ = name` so unused bindings don't break Go's strict
    unused-variable rule.
  - `ExprStmt` emits `_ = expr` for the same reason.
- Inlined Go runtime (~280 lines) covering `Value` (`interface{}`),
  `Symbol`, `Pair`, `Closure`, all 15 Twig builtins, symbol
  interning, module globals, `_sir_format` and `_sir_truthy` and
  `_sir_apply` and `_sir_make_closure`, plus a `_sir_call_builtin_by_name`
  dispatch table for `VarRef Builtin`.
- Identifier sanitisation handles Go keywords (`for`, `func`,
  `chan`, etc.) and predeclared builtins (`int`, `string`,
  `print`, `len`, etc.) by appending `_`.  Other invalid chars
  encode as `_<hex>`.  Empty → `_sir_empty`.  SIR's `main` is
  renamed to `_sir_user_main` so the emitter's own `main()`
  doesn't collide.
- `sanitize_comment` strips line terminators from external
  strings written into `//` comments — same defence as SIR12 /
  SIR13 / SIR14.
- Pre-lowering validation via `semantic_ir::validate`; capability
  check via `Backend::check_module`.

### Notes

- The runtime always imports both `"fmt"` and `"strconv"` — both
  are referenced inside the runtime block, so Go's strict
  unused-import rule never fires regardless of what the user
  module uses.
