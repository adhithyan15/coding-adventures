# Changelog

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
