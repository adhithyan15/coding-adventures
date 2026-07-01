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

## Usage

```rust
let module = ruby_to_semantic_ir::compile_source(
    "x = 1\ny = 2\nputs(x + y)\n",
    "demo",
)?;

// `module` is a `semantic_ir::Module` — pass it to any SIR backend.
```

## What's deferred

- `def` / `end` method definitions (the v0 ruby-parser grammar
  doesn't accept them; Phase 6+ will extend the grammar).
- Control flow (`if` / `while` / `case`).
- Blocks (`do...end`, `{...}`).
- Modules, classes, mixins.
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
