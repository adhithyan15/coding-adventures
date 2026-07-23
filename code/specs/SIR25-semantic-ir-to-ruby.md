# SIR25 — Semantic IR → Ruby (Rust backend)

## Status

Seventh backend for the narrow-waist Semantic IR
([SIR10](SIR10-narrow-waist-semantic-ir.md)).  Joins
[SIR12](SIR12-semantic-ir-to-typescript.md) (TypeScript),
[SIR13](SIR13-semantic-ir-to-rust.md) (Rust),
[SIR14](SIR14-semantic-ir-to-python.md) (Python),
[SIR15](SIR15-semantic-ir-to-go.md) (Go),
[SIR18](SIR18-semantic-ir-to-javascript.md) (JavaScript), and
[SIR24](SIR24-semantic-ir-to-c.md) (C).  Implemented as the Rust crate
`semantic-ir-to-ruby`.

Ruby was previously **only a frontend** ([`ruby-to-semantic-ir`](../packages/rust/ruby-to-semantic-ir/)).
This spec adds the matching **backend**, so SIR can now *emit* Ruby — enabling
Ruby↔SIR round-trips, Twig/Python/JavaScript→Ruby, and (the motivating goal)
**C→SIR→Ruby**, where C's sized-integer / wrapping semantics are rendered
faithfully in Ruby (the [`Convert`](SIR26-integer-conversions.md) node, a later
batch).

The crate consumes a [`semantic_ir::Module`] and emits a **self-contained Ruby
source file** — a small inlined runtime plus the program; it runs with
`ruby <file>.rb`, no gems.  This spec covers the **v0 core**; later feature
batches land through the same cascade the other backends followed.

## Why Ruby is the simplest target in the family

Ruby's semantics already line up with the SIR's:

- **Truthiness matches exactly.** Only `nil` and `false` are falsy in Ruby —
  precisely the SIR/Lisp convention.  A SIR condition maps to a native Ruby
  `if` with no coercion (unlike Python/JS/C, where `0`/`""`/`[]` are falsy and
  every test must route through a `truthy` helper).
- **Everything is an expression.** `if … then … else … end` yields a value, a
  `begin … end` yields its last expression, and a method returns its last
  expression.  So a SIR `Block`/`If` renders **directly** — none of the
  IIFE/statement-hoisting the Go/C backends need.
- **Native values for most SIR primitives.** Arbitrary-precision `Integer`,
  `Float`, `true`/`false`/`nil`, `String`, `Symbol` (`:foo`), and first-class
  `Proc`/lambda closures are all built in.  Only cons-`Pair`s need a shim.

So the emitter is thin and the inlined runtime is tiny.

## Public API

```rust
use semantic_ir::{Backend, Module, Artifact, BackendError};

pub struct RubyBackend;
impl RubyBackend { pub fn new() -> Self; }

impl Backend for RubyBackend {
    fn target_tag(&self) -> &'static str { "ruby" }
    fn accepts_features(&self) -> &'static [Feature] { /* ACCEPTED_FEATURES */ }
    fn accepts_intrinsics(&self) -> &'static [&'static str] { &[] }  // v0
    fn compile(&self, module: &Module) -> Result<Artifact, BackendError>;
}

pub fn compile(module: &Module) -> Result<Artifact, BackendError>;
```

`compile` runs `validate`, then the trait-default `check_module`, then
`emit::emit_module`.

## Capability declaration

**v0 accepts** the SIR-v0 feature set: `Closures`, `Pairs`, `Symbols`,
`Strings`, `DynamicTyping`, `OptionalTypeAnnotations`, `MutualRecursion`,
`Globals`.

**Landed since v0** (one version-bumped batch at a time — see the roadmap): the
SIR26 integer conversions (`Conversions`, `SizedIntegers`, `Unsigned`,
`WrappingArithmetic`); and the SIR16 batches `Loops` + `MutableBindings`
(0.3.0), `Sequences` (native `Array`, 0.4.0), `Maps` (native `Hash`, 0.5.0), and
`Floats` (native `Float`, 0.6.0).

**Still rejects** `TailCalls`, `Intrinsics`, `ShortCircuit`, `NDArrays`,
exceptions/OOP, and every not-yet-landed feature (each rejection is a clean,
source-positioned `UnsupportedFeature`).

## Value model

Emitted Ruby uses **native values**: `Integer`, `Float`, `true`/`false`/`nil`,
`String`, `Symbol`.  Closures are Ruby **lambdas** (`->(a, b) { … }`).  The one
non-native concept, a cons-`Pair`, is a tiny inlined struct:

```ruby
SirPair = Struct.new(:car, :cdr)
```

Globals live in a runtime `Hash` (`$sir_globals`), matching the other backends'
name-keyed store, so a `VarRef { Global }` reads through it and the `_init`
function's `global_set` calls write to it.

## Expression-oriented rendering

Because Ruby is expression-oriented, value production needs no helper functions
or temporaries:

| SIR node | Emitted Ruby |
|---|---|
| `IntLit { value }` | `<value>` |
| `FloatLit { value }` | `<value>` as a Ruby `Float` — `7.0` (never the Integer `7`), `Float::INFINITY` / `Float::NAN` |
| `BoolLit { value }` | `true` / `false` |
| `NilLit` | `nil` |
| `SymLit { name }` | `:<name>` (or `:"<escaped>"`) |
| `StrLit { value }` | `"<escaped>"` |
| `VarRef { Local\|Param\|Capture }` | `<name>` |
| `VarRef { Global }` | `sir_global_get("<name>")` |
| `VarRef { Builtin }` | `sir_builtin_closure("<name>")` |
| `If` | `(if sir_truthy(<cond>) then <then> else <else> end)` |
| `Block` (stmts + value) | method body: `<stmts>` then `<value>`; as an expression: `(begin; <stmts>; <value>; end)` |
| `LetBinding` / `LetStarBinding` | `<name> = <value>` |
| `ExprStmt` | `<expr>` |
| `DirectCall` | `<fn>(<args>)` |
| `IndirectCall` | `sir_apply(<target>, <args>)` (`target.call(*args)`) |
| `BuiltinCall` | native operator / runtime helper (below) |
| `MakeClosure { fn_name, captures }` | `sir_make_closure(method(:<fn>), <cap-values>)` |

`sir_truthy(v)` is `!v.nil? && v != false` — but since that *is* Ruby's own
`if` test, conditions may render as a bare `if <cond>`; the helper is kept for
uniformity and for values the runtime boxes.  A **trivial** block (no stmts)
renders its value inline.

## Builtins

Most builtins map to **native Ruby**, whose semantics are the reference:

| builtin | Ruby |
|---|---|
| `+` `-` `*` | native `+ - *` (numeric; `+` also concatenates strings/arrays, as Ruby does) |
| `/` | `sir_div` (Ruby `Integer#/` already floors; kept as a helper for the float/zero split) |
| `=` | `sir_eq(a, b)` (structural, symbol-aware) |
| `<` `>` | native `< >` |
| `cons` `car` `cdr` | `SirPair.new(a, b)` / `a.car` / `a.cdr` |
| `null?` `pair?` `number?` `symbol?` | `sir_is_null` / `…_pair` / `…_number` / `…_symbol` |
| `print` `puts` | `sir_print(...)` / `sir_puts(...)` (route through the display convention) |
| `global_get` `global_set` | `sir_global_get` / `sir_global_set` |

## Display convention

Output routes through `sir_puts`/`sir_print`/`sir_fmt` so the
[display-convention](sir-display-convention.md) is honoured: a **Ruby-sourced**
module renders booleans as `true`/`false` (Ruby's native form); any other source
keeps the Lisp `#t`/`#f`.  As in the Go/Rust/C backends, the emitter substitutes
the single placeholder `__SIR_DISPLAY_RUBY__` in the runtime with `true` or
`false`, **selected by a boolean** — never source-derived text (anti-injection).

## Identifier sanitisation

Ruby local/method identifiers match `[a-z_][A-Za-z0-9_]*` (locals must not start
uppercase — that is a constant).  `sanitize_ident` lowercases a leading
uppercase, escapes other characters, and suffixes Ruby keywords (`def`, `end`,
`class`, `do`, …) and the runtime's own `sir_`/`$sir_` namespace with `_`.  SIR
`main` renders as `sir_user_main` (Ruby has no reserved `main`, but the rename
keeps the entry explicit and uniform with the other backends); `_init` renders
as `sir_user_init`.  String/symbol escaping (`quote_ruby_string`) neutralises
`"`, `\`, `#{` (interpolation), and control characters so no source text can
break out of a literal or inject an interpolation.

## Module layout of the emitted `.rb`

```ruby
# Generated by semantic-ir-to-ruby (SIR25) — do not edit.
# ── inlined SIR runtime ──
SirPair = Struct.new(:car, :cdr)
$sir_globals = {}
def sir_truthy(v) … end
def sir_puts(*xs) … end        # display-convention aware
# … helpers …

# ── user functions (mutual recursion needs no ordering in Ruby) ──
def <fn>(<params>) … end

def sir_user_init … end        # if the module has _init
def sir_user_main … end        # SIR `main`

sir_user_init if defined?(...)  # emitted only when present
sir_user_main
```

## Tests

`cargo test -p semantic-ir-to-ruby` covers per-node lowering, identifier
sanitisation, deterministic output, and end-to-end *emit* from Ruby and Twig
source (asserting the emitted Ruby text).  A `tests/run_*.rs` set runs the
emitted Ruby with a discovered `ruby` and asserts stdout, **skipping when
`ruby` is absent** (the toolchain-gated convention).  The cross-backend proof is
[`sir-conformance`](../packages/rust/sir-conformance/): a `Target::Ruby` arm runs
the emitted Ruby and asserts byte-identical stdout versus the reference oracle
for every corpus program the backend accepts.

## Roadmap to parity

Mirrors the other backends' landed cascade; each item is one version-bumped PR
growing `ACCEPTED_FEATURES`, the runtime, and the conformance corpus in lockstep:

1. **v0 core** (this spec).
2. **SIR16** — `Floats`, `ShortCircuit`, `MutableBindings`, `Loops`,
   `Sequences` (native `Array`), `Maps` (native `Hash`).
3. **Params** — `DefaultParams`, `KeywordParams` (Ruby has both natively).
4. **[`Convert`](SIR26-integer-conversions.md)** — render integer
   narrow/reinterpret via mask helpers (`sir_u8`/`sir_i32`/…) — the C→Ruby
   faithfulness payoff.
5. **Collections / exceptions / OOP** — the `__method__` catalog (Ruby methods
   are largely native), `begin/rescue`, `class`/`module` (native).

## Out of scope (v0)

- Any feature past the SIR-v0 set (deferred to its roadmap batch, rejected
  cleanly until then).
- Integer `Convert`/sized-int rendering (SIR26 batch).
- Source maps; raw-Ruby intrinsic injection.
