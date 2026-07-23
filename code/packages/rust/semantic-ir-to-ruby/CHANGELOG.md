# Changelog

## 0.6.0 — floats (SIR16)

Accepts `Feature::Floats`. Ruby has a native `Float`, so this is a one-arm
addition: `Expr::FloatLit` renders directly as a Ruby float literal. The
feature gates ONLY `FloatLit` (float arithmetic reuses the existing
`+`/`-`/`*`/`/` builtins, which already fold to native Ruby operators), and the
runtime's `sir_fmt_float` already rendered every float — so accepting the
feature plus the one emit arm keeps the emitter total.

The literal is produced by a new `float_to_ruby_literal` helper, which fixes two
ways a naive `value.to_string()` would be wrong:

- **Integral floats must keep their point.** Rust's `f64::to_string` renders
  `7.0` as `"7"` — which Ruby parses as an *Integer* (a different type, with
  floor `/` instead of true divide, and `7` instead of `7.0` on display). The
  helper uses `{:?}` (Debug), whose shortest round-tripping form always carries
  a decimal point or exponent (`7.0`, `-0.0`, `1e300`) — every one a valid Ruby
  *Float* literal.
- **Non-finite values have no numeric token.** Ruby has no `inf`/`nan` literal;
  the values are `Float::INFINITY` / `-Float::INFINITY` / `Float::NAN`. A
  `FloatLit` carrying one (rare — it usually arises at runtime from `1.0 / 0.0`)
  now emits the named constant.

Because display routes through the runtime's `sir_fmt_float` (Ruby's own
`to_s`/`nan?`/`infinite?`), the printed form is native regardless of how the
literal was spelled — the helper only has to preserve the numeric value.
Verified end-to-end through a real `ruby` with hand-built modules (the frontend
masks `FloatLit`): integral floats keep `.0` (`7.0`, not `7`), `-0.0` keeps its
sign, `1.5 + 2.5 == 4.0` and `2.0 * 3.0 == 6.0` (integral results stay Float),
`7.0 / 2 == 3.5` while `7 / 2 == 3` (division frontier preserved — a Float
operand promotes, two Integers floor), `1.0 / 0.0 == Infinity` and `0.0 / 0.0 ==
NaN` (Float division by zero does not raise), and `7.0 == 7` is true.

## 0.5.0 — maps (SIR16)

Accepts `Feature::Maps`. Ruby has a native Hash, so the three map nodes render
directly — no runtime value-boxing like the Go/Rust backends' `_sir_map_*`:

- `Expr::MapLit` (`{k => v, …}`) → a native Hash literal.
- `Expr::MapGet` (`h[k]`) → `(h)[k]`: a missing key yields nil (no raise),
  matching `_sir_map_get`.
- `Stmt::MapSet` (`h[k] = v`) → `(h)[k] = v`: insert-or-update, mutating the
  shared Hash (a write through one binding is visible through every alias). A
  map has no bounds, so — unlike `SeqSet` — no guard helper is needed.

Ruby's Hash preserves insertion order and compares keys with `eql?`/`hash`,
which is STRUCTURAL for composite keys — so `{[1, 2] => x}[[1, 2]]` finds the
entry, matching the reference's `_sir_value_eq` key comparison. (One documented
divergence: `eql?` is type-strict for numbers, so a Ruby `{1 => x}[1.0]` is nil
where the reference's cross-representation `_sir_value_eq` would match; a
mixed int/float map key is rare and not exercised by any conformance case.)

`ForEach` over a Hash needs no new arm — the existing `(iter).each { |x| … }`
works on a Hash (yielding `[k, v]`) as well as an Array — so accepting Maps
keeps the emitter total. Every node verified by hand-built modules (bypassing
the frontend, which does not yet produce these), run against a real `ruby`.

## 0.4.0 — sequences (SIR16)

Accepts `Feature::Sequences`. Ruby has native arrays, so the SIR16 sequence
nodes render directly — no runtime value-boxing like the Go/Rust backends'
`_sir_seq_*`:

- `Expr::SeqLit` (`[1, 2, 3]`) → a native array literal. Structural `Array#==`
  makes `[1, 2] == [1, 2]` true, matching every backend that carries sequences.
- `Expr::SeqIndex` (`a[i]`) → `(a)[i]`. Ruby's `Array#[]` already matches the
  SIR reference exactly: a negative index counts from the end, an out-of-range
  index returns `nil` (never raises — that is `fetch`).
- `Expr::SeqLen` (`len a`) → `(a).length`.
- `Stmt::SeqSet` (`a[i] = v`) → `sir_seq_set(a, i, v)`, a new runtime helper
  that enforces the reference's bounds rule (RAISES on a negative or
  out-of-range index, unlike Ruby's native `[]=` which pads with nils / counts
  from the end) and returns the assigned value.
- `Stmt::ForEach` (`for x in a`) → `(a).each { |x| … }` — reachable once
  `Loops` is also accepted. A BLOCK, so `x` (and any body-local) is
  block-scoped, matching the validator (which rewinds the loop body) and the
  Go reference (`for _, x := range`, block-local via `:=`); a leaking `for …
  in` would instead clobber an enclosing same-named local. `ForRange` is
  block-scoped the same way, via a hoisted `->(x) { … }` body called from the
  `while`. Safe as blocks because SIR loop bodies have no break/next/return.

Also fixes a **pre-existing** panic surfaced while making the emitter total:
`Stmt::ForRange` (`for i in 0...3`) is gated by `Feature::Loops` alone
(accepted since 0.3.0) and is produced by the Ruby frontend, yet was sent to
the same `unreachable!` — so a numeric `for` loop crashed the backend. It now
desugars to a `while` mirroring the Go/Rust backends: bounds evaluated once
into nesting-safe `sir_`-prefixed temporaries, a direction-aware exclusive stop
(`step >= 0 ? i < stop : i > stop`, so a descending loop works), and a
block-scoped loop var (the body runs inside a hoisted `->(i) { … }`, so `i`
does not clobber an enclosing same-named local).

Handling all five sequence nodes plus `ForRange` keeps the emitter TOTAL for
its accepted feature set: no conforming producer (Ruby, C→SIR, Twig→SIR, …) can
reach an `unreachable!`. **This was
caught by security review** — an earlier revision handled only `SeqLit` on the
false premise that it was the only `Sequences`-gated node; in fact `SeqIndex`/
`SeqLen`/`SeqSet` are also gated by `Sequences` (the `NDArrays`-gated
`IndexGet`/`IndexSet` are the different SIR22 nodes), and `ForEach` becomes
reachable once `Loops` is accepted — all four would have panicked the emitter
for a non-Ruby producer. Verified with hand-built modules (bypassing the Ruby
frontend, which masks these nodes) for each of the five.

Array *indexing via `Expr::IndexGet`* and slicing are a DIFFERENT feature
(`NDArrays`, not accepted); array-*pattern* destructuring needs `ShortCircuit`
(not accepted) — so those stay rejected at the feature gate.

The `scan_expr`/`scan_stmt` unsupported-builtin pre-check recurses into the new
nodes' sub-expressions too, so an unsupported builtin nested in `[foo()]`,
`a[foo()]`, or `for x in [foo()]` is reported cleanly. It also gains a `While`
arm — a pre-existing hole (also found by the review): an unsupported builtin in
a `while` body previously escaped the pre-check and hit the emitter, so it now
rejects cleanly instead of panicking.

## 0.3.0 — control flow & mutation (SIR16)

Accepts `Feature::Loops` and `Feature::MutableBindings`, and renders the two
statements the C frontend's milestone-2 `if`/`while`/`for` produce:

- `Stmt::While { cond, body }` → Ruby `while sir_truthy(<cond>) … end` (the
  condition, already a bool, is re-tested each iteration).
- `Stmt::Assign { name, value }` → `name = value` (Ruby locals are mutable).

`Expr::If` and the comparison builtins were already rendered, so a C `for`-loop
now round-trips to running Ruby.

## 0.2.0 — render SIR26 integer conversions

Accepts `Feature::Conversions` (plus the SIR21 type-implied `SizedIntegers`,
`Unsigned`, `WrappingArithmetic`) and renders `Expr::Convert` — the C→SIR→Ruby
payoff.

- A conversion emits an inlined mask helper chosen by target width + signedness:
  `sir_u8`/`sir_u16`/`sir_u32`/`sir_u64`/`sir_u128` (mask) and
  `sir_i8`/`sir_i16`/`sir_i32`/`sir_i64`/`sir_i128` (mask then two's-complement
  sign-fold).  A target width of `Arbitrary` is the identity (a widen into
  Ruby's already-unbounded `Integer`) and emits no wrapper.
- The masking is exact for every width because Ruby's `Integer` is arbitrary
  precision and its bitwise ops use a two's-complement model — so `sir_u8(-1)
  == 255`, `sir_i32(4_000_000_000) == -294_967_296`.
- Verified end-to-end through a real `ruby`: `sir_u8(300)==44`,
  `sir_i32(4e9)==-294_967_296`, `(uint32_t)-1==4_294_967_295`,
  `(int8_t)200==-56`, arbitrary-width identity.

## 0.1.0 — v0 core (SIR25)

First release of the Ruby backend — the seventh SIR backend and the first Ruby
*target* (Ruby was previously only a frontend).

### Added

- `compile(module)` / `RubyBackend` implementing `semantic_ir::Backend`
  (`target_tag() == "ruby"`).
- **Self-contained** emission: a single `.rb` file with a small inlined runtime
  preamble (`SirPair`, a `$sir_globals` store, `sir_truthy`, display helpers
  that honour the display convention, `sir_eq`, `sir_apply`, and a
  builtin-as-value dispatcher).  Runs with `ruby <file>.rb`, no gems.
- **Expression-oriented lowering**: because Ruby's `if`/`begin…end` yield values
  and a method returns its last expression, `Block`/`If` render directly — no
  IIFE or statement-hoisting.  `MakeClosure` renders as a native lambda that
  binds the capture values and splats the call arguments; `IndirectCall` is
  `target.call(*args)`.
- v0 capability set (`Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
  `OptionalTypeAnnotations`, `MutualRecursion`, `Globals`) plus the core
  builtins `+ - * / % neg = == != < > <= >= not and or cons car cdr null? pair?
  number? symbol? print puts global_get global_set` (mostly native Ruby, whose
  semantics are the reference).
- A structural gate rejecting builtins the v0 backend cannot lower (e.g. the
  `__method__`/`case_eq` collection-dispatch protocol), so a module using a
  later feature fails cleanly rather than emitting a call with no lowering.
- Identifier sanitisation (Ruby keywords, the `sir_` runtime namespace, and
  leading-uppercase locals) and string/symbol escaping that neutralises `#{…}`
  interpolation so no source text can inject.
- Display-convention substitution (`__SIR_DISPLAY_RUBY__` → a boolean-selected
  literal, never source text).

### Wiring

- Added to the Rust workspace `members`.
- `sir-conformance` gains a `Target::Ruby` arm (`run_ruby`, `ruby` toolchain,
  skip-if-absent); a program whose feature set v0 does not accept is *skipped*
  (a declared gap), not failed — mirroring the C backend.

### Verified

- `cargo test -p semantic-ir-to-ruby` green (emit-shape + end-to-end via `ruby`).
- `cargo test -p sir-conformance` green: the Ruby cells run every v0-accepted
  corpus program and match the reference oracle byte-for-byte.
