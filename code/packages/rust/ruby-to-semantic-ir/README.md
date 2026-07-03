# ruby-to-semantic-ir

Ruby AST → narrow-waist [Semantic IR](../semantic-ir).  Phase 5 of the
[Ruby parser project](../../../specs/ruby-parser.md) — the first
frontend that consumes `ruby-parser`'s `GrammarASTNode` and emits a
[`semantic_ir::Module`](../semantic-ir).

## How it fits in the stack

```
Ruby source
   │
   ▼  ruby-lexer  →  ruby-parser
GrammarASTNode (program → statement+)
   │
   ▼  ruby-to-semantic-ir   ← THIS CRATE
semantic_ir::Module
   │
   ▼  semantic-ir-to-{rust, typescript, go, python} (existing backends)
target source
```

This is what lets a Ruby program reach any SIR backend.  Code written
in Ruby can be re-emitted as Rust / TypeScript / Go / Python by going
through the narrow-waist IR — the per-language backends already exist
and don't need to know Ruby.

## v0 scope

The lowering targets the same grammar subset that `ruby-parser` v0
parses (see [ruby-parser/src/_grammar.rs](../ruby-parser/src/_grammar.rs)):

- **Assignments** — `x = expr` becomes a `LetBinding` (first occurrence)
  or `Assign` (subsequent re-binding to the same name).
- **Method calls** — `name(args...)` becomes a `BuiltinCall` for the
  small list of Ruby builtins we recognise (`puts`, `print`, `p`,
  `gets`, `raise`); everything else lowers to a `DirectCall` and
  surfaces an "unresolved" diagnostic if no matching top-level
  function exists.  In v0 there are no `def`s, so all named calls
  are effectively builtins or unresolved.
- **Expressions** — integer / string literals, name references,
  binary `+`, `-`, `*`, `/` (lowered to `BuiltinCall("+", ...)` etc.,
  matching the convention `twig-to-semantic-ir` established).
- **Programs** — wrapped in a synthesised `main` function whose body
  is the sequence of lowered statements.  If the final source-level
  statement is a bare expression, it becomes the block's *value*;
  otherwise the value is `NilLit`.
- **Parameters** — positional, splat (`*rest`), and double-splat
  (`**kwrest`), plus **default / optional parameters** (P7, Ruby-1.0):
  `def f(a = 1)` lowers `a`'s default to `Param.default = Some(IntLit 1)`.
  Ruby defaults are call-time and may reference earlier params
  (`def f(a, b = a + 1)`), so the default expression is lowered in the
  parameter scope and resolves earlier params as `Scope::Param`.  A
  defaulted param observes `Feature::DefaultParams`; a call that omits a
  defaulted arg (`f(5)`) lowers to a call with fewer args (no padding).
- **Method bodies — implicit return (FC).**  Ruby has no explicit `return`;
  a method's value is its **last evaluated expression**.  The tail statement
  of a `def` body (and of each `if`/`unless` branch) is promoted into the SIR
  `Block.value` slot the backends emit as the implicit return.  Besides bare
  expressions and calls, a trailing **`if`/`unless`** and **`case`** (both
  `case/when` and `case/in`, which lower to a chained `if`) are now promoted
  too: `def bigger(a, b); if a > b then a else b end; end` returns the winning
  branch (previously the conditional was left as a discarded statement and the
  method returned `nil` on every backend).  Promotion recurses — a branch (or
  `case` arm) that itself ends in a conditional carries its own value — so
  arbitrarily nested tail conditionals all return correctly.  (A *script's* top-level value is not
  language-visible, so a bare trailing `if` at program scope stays a
  statement; only method/branch bodies implicitly return.)

## Usage

```rust
let module = ruby_to_semantic_ir::compile_source(
    "x = 1\ny = 2\nputs(x + y)\n",
    "demo",
)?;

// `module` is a `semantic_ir::Module` — pass it to any SIR backend.
```

## Object orientation (O2)

Real object-oriented Ruby now lowers to executable SIR — the frontend PRODUCES
the OOP wiring (all via the existing `BuiltinCall` envelope; no core-IR change),
which the `sir-runtime-oop` runtime + Python/TypeScript backend emit arms
consume:

- **Method registration.**  Each `def m` in `class C` hoists to a
  class-qualified top-level function (`C__m`) and is registered right after the
  `ClassDef` with `__def_method__("C", "m", MakeClosure { fn_name })`.  The
  runtime table is keyed on `(class, bare_method)`.
- **`Foo.new(args)`** → `__new__("Foo", …args)` (allocate → run inherited
  `initialize` under a pushed self → return the object).  Chains:
  `Foo.new(x).meth` = `__method__(__new__("Foo", x), "meth")`.
- **`super(args)` / bare `super`** → `__super__(method, class, …args)`, threading
  the enclosing method + class; bare `super` forwards the method's params.
- **`self`** → `__self__()` (the receiver on the runtime self-stack).
- **`attr_reader` / `attr_writer` / `attr_accessor :x`** expand into synthesized
  getter (`def x; @x; end`) and/or setter (`def x=(v); @x = v; end`) methods,
  hoisted and registered like hand-written ones.

Three golden programs (a `Dog#speak`, an Animal/Cat inheritance+`super`, and a
`Counter` with `attr_accessor`/`self`-chaining) are proven end to end through
the Python backend (and P1 through TypeScript/node).

**Deferred within OOP:** `def self.m` class methods (the grammar's `def` rule
has no receiver production yet — the `__def_class_method__` path is implemented
and ready); `super` as a sub-expression (statement-only today); and cross-class
same-name *intra-class* bare-name calls (which resolve to the qualified hoisted
function).

## What's deferred

- Control flow beyond v0 and refinements.
- Implicit return of a trailing **`begin`/`rescue`** (a trailing `if`/`unless`
  and `case` — both `case/when` and `case/in` — now promote to the block value),
  and of a block/lambda's tail conditional.
- The full set of Ruby's literal forms (regex, ranges, arrays,
  hashes, symbols, heredocs as runtime values — heredocs ARE lexed
  per Phase 3c, just not yet lowered to IR shape).
- Effect inference — every `BuiltinCall` currently lowers with an
  empty `EffectSet`; richer effect analysis arrives once we have
  more of Ruby's semantics modelled.

## Versioning vs. Ruby eras

The lowering itself is era-agnostic: it consumes whatever
`GrammarASTNode` the parser produces.  Era-specific syntax (lambda
`->`, hash shorthand, `&.`, …) lands at the **parser** layer (see
[ruby-version-evolution.md](../../../specs/ruby-version-evolution.md));
this crate then routes the resulting AST shapes to the right SIR
nodes.
