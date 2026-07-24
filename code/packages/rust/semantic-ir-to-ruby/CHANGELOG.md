# Changelog

## 0.13.0 — classes slice 3: instance variables (@ivars) + self

Accepts `Feature::InstanceVars` — an instance variable read and write, plus the
`__self__` builtin. The third OOP slice, and a small one: the slice-2 method
machinery already does the heavy lifting.

- `@v = x` → `Stmt::Assign { scope: Instance }` → native `@v = x`.
- `@v` → `Expr::VarRef { scope: Instance }` → native `@v`.
- `__self__` (a bare `self`) → the native `self` keyword.

The frontend puts the leading `@` in the node's `name`, and the emitter renders
it **verbatim** (not through `sanitize_ident`, which would mangle the `@`). No
runtime support is needed: an instance-method body is installed with
`define_method` (slice 2), which binds `self` to the receiver, so `@v` inside a
method reads/writes **that instance's** own variable, and it persists across
dispatches (a counter mutating `@n` across calls works).

**Injection safety.** Both verbatim-emitted instance-variable positions — a
`Scope::Instance` `Assign` target and `VarRef` — are validated as
`@<identifier>` (a new `is_valid_ivar_name`) in the SAME pre-emit traversal as
the builtin/constant scan (co-total with the emitter), so a crafted name (`@v;
system(...)`, a non-`@` name, `@` + digit) cannot inject source and is rejected
cleanly.

**Still rejects** class variables (`@@x` — `Feature::ClassVars`), inheritance (a
superclass / `__super__`), class methods (`__class_method__` /
`__def_class_method__`), and modules — each a later slice.

## 0.12.0 — classes slice 2: instance methods

Instance-method **definition** and **dispatch** — the second OOP slice. No new
`Feature` (the frontend lowers a method-bearing class to builtins, not a
feature-gated node); this wires two builtins:

- `__def_method__("Class", "method", MakeClosure(fn))` — the frontend's
  registration of a hoisted method — renders as
  `Class.define_method(:sir_um_method, &closure)`. `define_method` binds `self`
  to the receiver at call time, and the closure calls the hoisted top-level
  function, so the method body runs with the instance as `self` (its `@ivars`
  become reachable once slice 3 accepts them).
- `__method__(recv, "method", args…)` — instance dispatch — renders as
  `(recv).public_send(:sir_um_method, args…)`.

**Anti-RCE — the `sir_um_` prefix closes reflection dispatch.** `__method__`
dispatches by a method name taken from the IR, so a naive
`recv.public_send(:name)` would be a remote-code-execution sink: a hand-built
module could pass `"instance_eval"` / `"send"` and reach Ruby's metaprogramming.
Both registration and dispatch instead go through a **reserved `sir_um_`
method-name prefix** — no Ruby built-in is named `sir_um_*`, so `public_send`
with a crafted name can reach *only* a method installed by `__def_method__`,
never `instance_eval`/`send`/`eval`/any reflection sink. This is the codebase's
"explicit dispatch, never reflection" invariant, achieved natively (SIR24 §OOP).
The prefixed name is emitted as a quoted symbol via `emit_symbol` (no injection),
and the class name in `__def_method__` is validated as a constant path like the
slice-1 constant positions.

**Totality / clean rejection.** A `__method__` call to a name the module never
registers via `__def_method__` is a **built-in method call** (`.upcase`, …) — the
separate Collections batch — and is rejected cleanly (a source-positioned
`UnsupportedFeature`) rather than compiling to a runtime `NoMethodError` (the
prefixed `sir_um_upcase` is unbound). The scan collects the module-wide set of
registered method names in a first pass, then the single co-total traversal
validates each dispatch against it. A malformed `__def_method__` (missing its
closure) and the remaining OOP builtins (`__super__`, `__self__`,
`__class_method__`, `__def_class_method__`) stay rejected (later slices). Class
**methods**, **inheritance**, `@ivars`, `@@class vars`, and modules remain
unsupported.

## 0.11.0 — classes (slice 1) + constants

Accepts `Feature::Classes` and `Feature::Constants` — the first slice of the OOP
frontier: an **empty base class** and its **construction**, plus the entangled
**constants** prerequisite.

- **Classes.** `Stmt::ClassDef { name, superclass: None, body: [] }` — an empty
  base class — is accepted, and `Foo.new(args…)` (the frontend's `__new__`
  builtin, whose first argument is the class name) constructs an instance.
- **Constants.** A `Scope::Const` assignment (`PI = 3`) and reference (`PI`,
  `Foo::Bar`) are accepted. Constants ride in with Classes because they are
  **entangled**: a class name IS a Ruby constant, so the frontend records
  `Constants` in the manifest for any `Foo.new` (the receiver `Foo` is a
  constant) — an instantiable class cannot compile without it. Accepting
  Constants also unblocks `raise SomeClass` (a specific exception class is a
  `Const` reference — a form the 0.10 exceptions slice deferred precisely
  because Constants was then unaccepted).

**Reflective definition (why not native `class Foo; end` / `PI = 3`).** The
frontend wraps a program's top-level code in `main`, and Ruby forbids BOTH a
`class` definition and a constant assignment inside a method body ("class
definition in method body" / "dynamic constant assignment"). So a class and a
constant are defined **reflectively**:

- `class Foo; end` → `Object.const_set(:Foo, Class.new)`
- `PI = 3` → `Object.const_set(:PI, 3)`

`const_set` is legal anywhere, executes in place (no fragile hoisting /
reordering), and still names the class (`Foo.name == "Foo"`, so `Foo.new` and
`x.is_a?(Foo)` work). Constant *references* (`Foo.new`, bare `PI`) emit the bare
constant, which resolves at runtime. This dynamic construction also composes
cleanly with the next slice's `define_method` for the frontend's hoisted,
separately-registered methods.

**Injection safety.** Every constant name emitted verbatim — a `ClassDef` name,
a `__new__` class name, a `Const` reference, and a `Const` assignment target —
is validated as a Ruby constant path (`Foo` / `Foo::Bar`) by the SAME single
pre-emit traversal that rejects unlowerable builtins (a unified `ScanHit`,
**co-total with the emitter**), so a hand-built module cannot inject source
through a crafted name.

**Totality — deferred shapes rejected cleanly (never `unreachable!`).** Accepting
`Classes` obligates handling every node it surfaces. This slice supports ONLY an
empty base class; the pre-emit scan rejects, with a source-positioned error,
everything deferred to later slices: a **superclass** (inheritance), a
**non-empty class body** (class-level code / constants), a **namespaced**
(`Foo::Bar`) class or constant *definition* (`const_set` names one namespace), a
**singleton class** (`class << self` — `Stmt::SingletonClassDef`, which also
observes `Feature::Classes`), and every **OOP method builtin** (`__def_method__`,
`__method__`, `__super__`, `__self__`, `__class_method__`, …) — so a
method-bearing, inheriting, or singleton-opening class fails cleanly rather than
mis-emitting. Instance variables (`@x`), class
variables (`@@x`), and modules remain unaccepted features (their own later
slices).

## 0.10.0 — exceptions (SIR17)

Accepts `Feature::Exceptions` — the first of the OOP/exception frontier, and
self-contained (a `rescue` clause matches by exception-class NAME, an advisory
string, so it is separable from `Classes`). Ruby handles exceptions natively, so
this needs no runtime support:

- `Stmt::TryCatch` renders `begin … rescue … ensure … end`. Each `rescue`
  clause lists its exception classes by name, optionally binds the caught
  exception to a local (`rescue Foo => e`), and runs its body; an empty class
  list is a bare catch-all. `ensure`, when present, runs afterwards.
- The `raise` builtin renders the native `raise` — bare (re-raise the exception
  being handled), with a message string (`raise "boom"` → `RuntimeError`), or
  with an exception object. `retry` renders the native `retry`.
- `raise SomeClass` (a specific exception class) lowers to a `Const` reference,
  which observes `Feature::Constants` (not accepted) → such a module is rejected;
  `raise "message"`, a bare re-raise, and `rescue` by a standard class
  (`StandardError`, …) or catch-all are the accepted forms.

**Injection safety**: a `rescue` clause's exception-type name is emitted verbatim
as a Ruby constant reference (it must stay capitalized, so it cannot be routed
through `sanitize_ident`). A `compile`-time gate rejects any module whose rescue
type is not a valid Ruby constant path (`Foo` / `Foo::Bar`) — so a hand-built
module cannot inject source through a crafted type name. Crucially, this check is
folded into the SAME single traversal as the unsupported-builtin pre-check
(a unified `ScanHit`), so it is **co-total with the emitter**: every `TryCatch`
the emitter can reach — including ones nested in a call argument, a function's
trailing value, a `SeqSet`/`MapSet` sub-expression, or any other expression
position — is validated. (Security review caught a first attempt using a
separate, hand-picked walk that missed several of those positions; the unified
walk cannot drift.) The caught exception's binding, being an ordinary local,
goes through `sanitize_ident` as usual; a `raise`d message string is quoted by
`quote_ruby_string`.

Documented limitation: a `rescue` by an advisory class name that is not a live
Ruby constant (a user-defined exception class, which needs the not-yet-accepted
`Classes` feature) raises `NameError` at runtime; standard classes and bare
`rescue` always work.

First of the exceptions parity arc: Go/Rust/Python/JS already accept `Exceptions`
(C is tracked next). Verified through a real `ruby` with hand-built modules: a
bare rescue catching a raised message, `ensure` always running, a rescue binding,
a typed `rescue StandardError`, the native emit shape, and the injectable-type
rejection. Bumps semantic-ir-to-ruby 0.9.0 → 0.10.0.

## 0.9.0 — keyword parameters (SIR19)

Accepts `Feature::KeywordParams`. Ruby has native keyword arguments, so this is
a direct emission — no positional resolution like the Go/C backends' KW6:

- A **keyword parameter** renders `def f(x:)` (required) or `def f(x: <default>)`
  (optional — a keyword default is an optional keyword, riding on
  `KeywordParams`, not `DefaultParams`).
- A **keyword argument** (`Expr::KeywordArg`) renders `x: <value>` in the call's
  argument list; Ruby binds it to the parameter by **name**, so keyword
  arguments are order-independent (`f(b: 2, a: 10)` binds `a`/`b` correctly). The
  label is sanitised identically to the parameter it binds, so the two agree.
- The unsupported-builtin pre-check (`scan_expr`) recurses into a keyword
  argument's value.

While restructuring the parameter loop, made it **total** over every
`ParamKind`: a `Rest` parameter now renders `*rest` and a `KwRest` renders
`**opts` (native Ruby), where both were previously mis-emitted as bare names.
This matters because a `**opts` co-occurs with keyword parameters, so accepting
`KeywordParams` must not leave it broken. (These kinds carry no feature of their
own — a validator matter — but the emitter now spells all four kinds correctly.)

First of the KeywordParams parity arc: Go/Python/JS already accept it; this
brings the Ruby backend up (C is tracked next; the Rust backend is a separate
gap). Verified through a real `ruby` with hand-built modules: a keyword argument
binding by name, order-independent resolution (`f(b: 2, a: 10)` → `8` for `f(a:,
b:) = a - b`), an optional keyword using its default when omitted (`f()` → `7`
for `f(x: 7)`) and overridden when supplied, native `x:` / `x: 5` syntax, and
the `*rest` / `**opts` splat emission. Bumps semantic-ir-to-ruby 0.8.0 → 0.9.0.

## 0.8.0 — default parameters (SIR19)

Accepts `Feature::DefaultParams`. A positional parameter carrying a default
expression renders as Ruby's **native** `def f(a, b = <default>)`. Ruby
evaluates the default at call time when the argument is omitted — exactly the
SIR semantics — so no runtime support is needed; it is a one-line addition to
the function-signature emitter (`name = <emit_expr(default)>` when
`p.default.is_some()`).

- The default may reference an **earlier parameter** (`def f(a, b = a)`): Ruby
  binds parameters left to right, matching the validator, which checks each
  default with the parameters declared before it in scope.
- Only the **positional** case is `DefaultParams`; a keyword default is the
  separate (still-unaccepted) `KeywordParams` feature, so it never reaches here.

Also extends the unsupported-builtin pre-check (`first_unsupported_builtin`) to
scan each parameter default, not just the body — a default is an expression
evaluated at call time, so a deferred builtin hidden in one (`def g(x = foo())`)
must be rejected cleanly rather than slip past the body scan and hit the
emitter's `unreachable!`. This keeps the emitter total for the feature.

Security review additionally caught a pre-existing hole the default scan would
inherit: `scan_expr`'s `IndirectCall` arm scanned only the call arguments, not
the callee `target` — yet the emitter renders the target (`sir_apply(<target>,
…)`), so a deferred builtin in the callee position could reach the
`unreachable!`. The arm now scans the target too.

First of the DefaultParams parity arc: Go/Rust/Python/JS already accept it; this
brings the Ruby backend up (C is the last, tracked next). Verified through a
real `ruby` with hand-built modules (a function with a defaulted parameter and a
`main` that calls it with and without the trailing argument): the default is
used when the argument is omitted (`f(1)` → `6` for `f(a, b = 5) = a + b`) and
overridden when supplied (`f(1, 2)` → `3`), a default referencing an earlier
parameter, and the deferred-builtin-in-default rejection. Bumps
semantic-ir-to-ruby 0.7.0 → 0.8.0.

## 0.7.0 — short-circuit (SIR16)

Accepts `Feature::ShortCircuit`. `Expr::LogicalAnd` / `Expr::LogicalOr`
(`&&` / `||`) render as Ruby's native short-circuit operators, which ARE the
SIR semantics exactly — no runtime helper, no coercion:

- They yield the **deciding operand**, not a coerced boolean: `1 && 2` is `2`,
  `false && 2` is `false`, `nil || 7` is `7`, `1 || 2` is `1`.
- They **skip the right operand** when the left already decides — Ruby `&&`
  does not evaluate its rhs when the lhs is falsy, and `||` does not when the
  lhs is truthy.
- Ruby truthiness is the SIR/Lisp convention (only `nil` and `false` are falsy),
  so the operands need no `sir_truthy` wrapper — unlike the Go/C backends, which
  must lift to an IIFE / hoisted `if` to return the operand value rather than a
  native bool.

These are distinct from the eager `and`/`or` **builtins** (which the emitter
also renders with `&&`/`||`); the `ShortCircuit` feature is specifically the two
short-circuit expression nodes. The unsupported-builtin pre-check
(`scan_expr`) now recurses into both operands, so a deferred builtin nested in a
`&&`/`||` is still reported cleanly. Two nodes, both handled → the emitter stays
total.

First of the ShortCircuit parity arc: Go/Rust/Python/JS already accept it; this
brings the Ruby backend up (C is the last, tracked next). Verified through a
real `ruby` with hand-built modules (the frontend constant-folds a literal
`&&`, so the node is built directly): operand-return for both operators, and a
short-circuit proof where the dead operand is `1 / 0` — a correct lowering skips
it (`false && (1/0)` → `false`, exits clean), a broken eager one would raise.
Bumps semantic-ir-to-ruby 0.6.0 → 0.7.0.

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
