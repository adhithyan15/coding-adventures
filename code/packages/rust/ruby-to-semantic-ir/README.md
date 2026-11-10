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
  are effectively builtins or unresolved.  `print`/`puts` specifically
  lower to `BuiltinCall("__sys_write__", ...)`, not a bare
  `BuiltinCall("print"/"puts", ...)` — see
  [SIR28-syscall-primitives.md](../../specs/SIR28-syscall-primitives.md)
  §2.1; `p` is unaffected (reserved for a future `__sys_write_inspect__`).
- **Expressions** — integer / string literals, name references,
  binary `+`, `-`, `*`, `/`, `<<` (lowered to `BuiltinCall("+", ...)` etc.,
  matching the convention `twig-to-semantic-ir` established). `<<` folds
  left-associatively over the grammar's `shift` chain the same way `+`/`-`
  fold over `sum` — it's a binary-operator EXPRESSION (`BuiltinCall("<<",
  [lhs, rhs])`), not the `__method__` dispatch protocol Collections
  methods use.
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

- **Bare method names (#18778).**  Ruby decides at parse time what a bare
  `g` is: a local if an assignment (or parameter) `g` appears earlier in the
  scope, otherwise a call `g()`.  So `def g; 42; end; puts g` prints 42.  The
  lowerer follows that rule with the locals it has seen so far (and, inside a
  block or lambda, the locals of the scopes around it).  A name that a class
  or module also defines is left as a variable reference, because inside the
  class it would mean `self.g`; the validator then refuses it rather than a
  backend running the wrong method.
- **`for x in xs [do] … end` (#18778).**  No new scope, as in Ruby: `x` is a
  local of the enclosing scope (declared `nil` first if new) and keeps the
  last item.  The SIR loop iterates a fresh `__for_<n>` and stores it into
  `x` first thing in the body.  Locals first assigned in the body are
  locals of the enclosing scope too, declared `nil` before the loop, so they
  are still there after it.  `a...b` and `a..b` with an integer-literal `b`
  become a `ForRange`; `a..b` with any other `b` counts with `<=` in a
  `While` (`b + 1` is wrong for `1..2.5`); a Float-literal start is refused,
  as Ruby cannot iterate from a Float; anything else is a `ForEach`.  A
  parameter as the loop variable is refused.
- **`rescue => e` (#18778)** binds `e` with no class list (a catch-all).
- **Loop bodies** are statements only: a trailing `puts` in a `while` or
  `for` body is an `ExprStmt`, never the body's value (the C and Ruby
  backends emit only a loop body's statements).
- **`x if c` as a method's last statement** returns `x` when `c` holds and
  `nil` otherwise.
- **Where a statement ends (#18798).**  As in Ruby, a newline after a
  complete expression ends the statement, even before a line that starts
  with `[`, `(`, `-` or `+`: `x = 5` ⏎ `-1` leaves `x` at 5, and `x = foo` ⏎
  `(1)` does not pass 1 to `foo`.  An operator at the end of a line, or a
  `.` at the start of the next, still continues it.  A name alone on its
  line takes nothing from the next line (`puts` ⏎ `puts 2`), a `rescue`
  line's class list ends at the line break, and `;` separates statements.
  (The `ruby-lexer` crate marks these places; see its `mark_statement_ends`.)
- **A bare `puts` statement (#18798)** prints a newline, like `puts()`.
  (As a value, `x = puts`, it is refused.)  `super`, `yield`, `return`,
  `break` and `next` take nothing from the next line or past a `;`.
- **A method-level `rescue` keeps its value (#18798).**  `def risky; raise
  "x"; rescue => e; "rescued"; end` returns `"rescued"`: the last value of
  the body, or of the clause that caught the exception, goes through an
  invented local `__rescue_value_<n>`.  `ensure` never changes it.

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
- `for a, b in pairs` (one loop variable only), the value of a `for` loop
  (a method that ends in one returns `nil`, not the iterable), locals a
  `while` body introduces staying visible after the loop, and a `rescue =>
  e` binding staying visible after its clause (#18798).
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
