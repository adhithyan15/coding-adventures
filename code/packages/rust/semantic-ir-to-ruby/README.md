# semantic-ir-to-ruby

Seventh backend for the narrow-waist Semantic IR.  Lowers
[semantic-ir](../semantic-ir/) modules into **self-contained** Ruby source
code — every emitted `.rb` file carries a small inlined runtime, so
`ruby <file>.rb` runs it with no gems.

Implements [SIR25](../../../specs/SIR25-semantic-ir-to-ruby.md).

Ruby was previously only a *frontend* ([ruby-to-semantic-ir](../ruby-to-semantic-ir/));
this crate adds the matching **backend**, so SIR can now *emit* Ruby — enabling
Ruby↔SIR round-trips, Twig/Python/JavaScript→Ruby, and the motivating
**C→SIR→Ruby** path (where C's sized-integer / wrapping semantics render
faithfully via the later [`Convert`](../../../specs/SIR26-integer-conversions.md)
node).

## Public API

```rust
use semantic_ir_to_ruby::{compile, RubyBackend};
use semantic_ir::Backend;

let artifact = compile(&sir_module)?;            // convenience
let artifact = RubyBackend::new().compile(&sir_module)?;  // via the trait
```

## Why Ruby is the simplest target in the family

Ruby's semantics already match the SIR's:

- **Truthiness matches exactly** — only `nil` and `false` are falsy, precisely
  the SIR/Lisp convention, so a condition is a native `if` with no coercion.
- **Everything is an expression** — `if`/`begin…end` yield values and a method
  returns its last expression, so a SIR `Block`/`If` renders **directly** with
  no IIFE or statement-hoisting (unlike the Go/C backends).
- **Native values** — arbitrary-precision `Integer`, `Float`,
  `true`/`false`/`nil`, `String`, `Symbol`, and `Proc`/lambda closures are all
  built in.  Only a cons-`Pair` needs a shim (`SirPair = Struct.new(:car, :cdr)`).

So the emitter is thin and the inlined runtime is tiny (a `Pair`, a global
store, a display path, equality, and a builtin-as-value dispatcher).

## Capability declaration (v0)

Accepts `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`; the SIR26 integer
conversions (`Conversions`, `SizedIntegers`, `Unsigned`, `WrappingArithmetic`);
SIR16 control flow and mutation (`Loops` — `While`, `ForRange` (numeric
`for`, direction-aware), `ForEach`; and `MutableBindings`); and SIR16
`Sequences` — native arrays for all five sequence nodes: `SeqLit` (`[1, 2, 3]`,
structural `Array#==`), `SeqIndex` (`a[i]`, nil on OOB), `SeqLen` (`a.length`),
`SeqSet` (`a[i] = v`, bounds-checked via `sir_seq_set`), and `ForEach`
(`for x in a`); SIR16 `Maps` — a native Hash for `MapLit` (`{k => v}`),
`MapGet` (`h[k]`, nil on miss), and `MapSet` (`h[k] = v`), with structural
composite keys; SIR16 `Floats` — a native `Float` for `FloatLit` (rendered
so `7.0` stays a Float, not the Integer `7`; `Infinity`/`NaN` are named), with
native float arithmetic and division; SIR16 `ShortCircuit` — `LogicalAnd`
(`&&`) and `LogicalOr` (`||`) rendered as Ruby's native short-circuit
operators, which yield the deciding operand and skip the dead branch exactly as
SIR requires; SIR19 `DefaultParams` — a positional parameter with a default
renders as native `def f(a, b = <default>)` (evaluated at call time when the
argument is omitted; may reference an earlier parameter); and SIR19
`KeywordParams` — a keyword parameter (`def f(x:)` / `def f(x: 1)`) and keyword
argument (`f(x: 5)`) render as Ruby's native keyword forms, matched by name (so
order-independent).
Rejects `TailCalls`, `Intrinsics`, and every not-yet-wired feature (array
indexing / slicing via `IndexGet` — `NDArrays`; array-pattern destructuring;
collection methods, exceptions, OOP) until its cascade batch
lands — each a clean, source-positioned `UnsupportedFeature`.

## Verification

- `cargo test -p semantic-ir-to-ruby` — per-node emit shape, identifier
  sanitisation, string-escaping safety, determinism, and end-to-end runs
  (skipped when no `ruby` is on `PATH`).
- The cross-backend proof is [`sir-conformance`](../sir-conformance/): a
  `Target::Ruby` arm runs the emitted Ruby and asserts byte-identical stdout
  versus the reference oracle for every corpus program the backend accepts.
