# Changelog

All notable changes to the `ruby-to-semantic-ir` crate will be documented in this file.

## 0.8.0 — lower bracket-index read/write through `__method__` dispatch

`ruby-parser` 0.7.0 adds real grammar rules for `recv[k]` (read,
`index_suffix`) and `recv[k] = v` (write, `index_assignment`) — previously
neither had a grammar rule at all (see that crate's CHANGELOG). This
release adds the corresponding lowering:

- `recv[k]` → `BuiltinCall("__method__", [recv, StrLit("[]"), k])`
- `recv[k] = v` → `ExprStmt(BuiltinCall("__method__", [recv, StrLit("[]="), k, v]))`

Both ride the SAME narrow-waist `__method__` dispatch every other
Collections built-in uses (`.map`, `.each`, …) — no new IR node, no new
`Feature`. This routes the Array-vs-Hash decision to the BACKEND, at
RUNTIME, based on the receiver's actual tag.

An earlier version of this lowering used a compile-time heuristic instead
(mirroring `python-to-semantic-ir`'s documented convention: a string-
literal-key index lowers to `Feature::Maps`/`Expr::MapGet`/`Stmt::MapSet`,
any other index lowers to `Feature::Sequences`/`Expr::SeqIndex`/
`Stmt::SeqSet`). That heuristic mis-routes a Hash write whose key isn't a
string literal — `h[2] = "b"` on an int-keyed Hash, or `h[:sym] = 1` on a
symbol-keyed Hash — to the Array path regardless of `h`'s actual type,
which crashes at runtime (`_sir_seq_set` exits on a non-sequence receiver).
Both are common, legitimate Ruby. The `__method__` design was chosen
specifically because it cannot mis-route: the C backend's
`_sir_builtin_method_v` checks `recv.tag` itself, so the index's syntactic
shape is irrelevant to which path runs.

v0 scope carries over from `ruby-parser`: `index_assignment`'s left-hand
side must be a bare local/param `NAME` — no dotted or chained receivers.

### Added

- `bracket_index_read_lowers_to_method_dispatch`,
  `bracket_index_write_lowers_to_method_dispatch`,
  `bracket_index_write_with_non_string_key_is_not_a_seq_set`,
  `chained_bracket_index_read_nests_method_dispatch`,
  `bracket_index_read_and_write_pass_sir_validator` — lowering-shape and
  validator regression tests, including the non-string-key case that
  motivated the design above.

## 0.7.1 — regression tests for a `ruby-parser` grammar fix

No behavior change in this crate — the actual fix (a bare comparison/logical
statement, e.g. `x > 2`, mis-parsing as a paren-less call and splitting into
two statements) lives entirely in `coding-adventures-ruby-parser`'s
`ruby.grammar` (see that crate's 0.6.0 CHANGELOG entry). Adds regression
tests here at the lowering/validation layer — the level a consumer would
actually notice the bug at — covering the bare-statement, block-tail, and
`def`-tail positions, plus a control case pinning that the (deliberately
unguarded) bitwise operators remain unchanged.

## 0.7.0 — lift a class CONSTANT to its name for `is_a?` / `when SomeClass`

`case x when Integer` and `x.is_a?(Integer)` compiled on **Python alone**. Both
lower to `is_a?`, and both passed the class as a bare `Const` `VarRef`. Only
Python's backend could cope: Go and Rust REJECT a constant reference at emit
("cannot lower a constant reference" — a `Const` is accepted only as an
exception class in `raise Foo`), and JavaScript emitted an undefined reference
that blew up at run time. Ruby type-dispatch is ordinary code, so this was a
large hole rather than an exotic one.

Both sites now surface the class as a `StrLit` of its NAME — exactly the
convention `lower_class_pattern` (Phase FC) already documented and used
("so no constant declaration is required and no `Constants` feature is pulled
in"):

- `when SomeClass` in a `case`.
- A direct `x.is_a?(C)` / `x.kind_of?(C)` / `x.instance_of?(C)` call.

Every backend's `is_a?` compares class NAMES, so the name is the honest thing
to hand them, and no backend needs general constant-reference support. All four
running backends (Python, JavaScript, Go, Rust) now agree.

## [0.6.3] - 2026-07-03

### Fixed (FC — sequential local assignments: `x = a` where `a` is an earlier local)

Ruby assignments are SEQUENTIAL (`let*`): `a = 5; b = a + 1` binds `b` using
`a`'s value. The frontend, however, lowered every first-sighting `name = value`
to a PARALLEL `Stmt::LetBinding`, and the SIR validator treats a *run* of
consecutive `LetBinding`s as one parallel-`let` group — every RHS is evaluated
BEFORE any of the run's names are bound. So `[LetBinding(a); LetBinding(b = a+1)]`
was rejected with `var-ref scope=local references unknown name 'a'`: any
`newvar = <expr reading an earlier local>` (`x = a`, `b = a + 1`, `v = h["k"]`,
destructuring `a, b = arr`, `y = obj.meth`, `y = -x`, a hash/heredoc reading a
prior local, …) failed to compile on EVERY backend. Discovered while
investigating the array-index conformance gap.

- New `sequentialize_let_bindings` post-pass: after a block's statements are
  lowered, a `LetBinding` whose value reads a name bound by an EARLIER statement
  in the block is rewritten to a `LetStarBinding` (identical fields). `let*` is
  sequential — the validator binds its name immediately and it breaks the
  parallel run — so the reference resolves. Independent bindings (`i = 0;
  sum = 0`) keep `LetBinding`, so nothing else changes. Both variants lower to
  the same sequential variable declaration on every backend, so the rewrite is
  behaviour-preserving.
- Applied at every sequential body: program (main), `if`/`unless`/`case` branch
  bodies, method (`def`) bodies, and block/lambda bodies.
- Shape tests that asserted the buggy `LetBinding` form for such programs updated
  to accept either variant (via new `binding_name`/`binding_value` test helpers);
  verified end-to-end by the sir-conformance `seq_assign` program.

NOTE: array/hash *index reads* themselves (`a[1]`, `h["k"]`) remain a separate,
open Ruby-frontend PARSER-precedence gap — `a[1]` still mis-parses as `a` + a
bare `[1]` array literal; only the assignment-scoping half is fixed here.

## [0.6.2] - 2026-07-02

### Added (FC — implicit return of a trailing `case` from a method body)

Extends the trailing-conditional implicit return (0.6.1's `if`/`unless`) to
`case`. A Ruby method's value is its last evaluated expression, and `case`
(both `case/when` value-matching and `case/in` pattern-matching) is an
expression that already lowers to a chained `Expr::If`. Before this change a
method body ending in a `case` left it as a discarded statement and returned
`nil` on every backend; now the `case` is promoted to the block's `value`.

- `lower_tail_value` gains a `case_statement` arm (routing through
  `lower_case_statement`), so a `def` whose body ends in a `case` returns the
  matched arm's value; because `case` arms are built with
  `lower_clause_statements` (which calls `lower_tail_value`), promotion recurses
  through nested tail conditionals.
- Verified end to end: `def grade(n); case n; when 90 then "A"; when 80 then
  "B"; else "C"; end; end` with `puts grade(90/80/50)` prints `A/B/C` on
  Python, JavaScript, Go, and Rust (was blank/nil before). This is now provable
  on all backends because the `case_eq` builtin the chain relies on was
  implemented across Go/Rust/JS in a prior PR.
- Four new unit tests (tail `case/when`, tail `case/in`,
  leading-stmts-then-tail-`case`, validator pass). All 427 frontend tests pass.

## [0.6.1] - 2026-07-02

### Fixed (FC — implicit return of a trailing `if`/`unless` from a method body)

A Ruby method returns the value of its **last evaluated expression**, and
`if`/`unless` are expressions. The frontend, however, only promoted a bare
`expression_stmt`/`method_call` tail into the SIR `Block.value` slot — a body
ending in a conditional left the `if` as a discarded statement and set
`value = NilLit`. Every backend faithfully reproduced this, so a method like
`def bigger(a, b); if a > b then a else b end; end` returned `nil` on Python,
JavaScript, Go, and Rust alike (verified end to end: `puts bigger(10, 7)`
printed a blank line before the fix, `10` after).

- **Shared `lower_tail_value` helper.** The tail-promotion decision (previously
  duplicated inline in `lower_program`, `lower_clause_statements`, and
  `lower_def_statement`) is now one helper with a documented promotion table.
  It promotes `expression_stmt`, `method_call`/`method_call_no_paren`, and now
  **`if_statement`/`unless_statement`**; everything else (assignments, `while`,
  …) returns `None` and stays a `Stmt`.
- **Method and branch bodies fixed.** `lower_def_statement` and
  `lower_clause_statements` route their tail through the helper, so a `def`
  whose body ends in a conditional returns the branch value, and an `if` branch
  that itself ends in an `if` promotes **recursively** (nested tail conditionals
  each carry their value).
- **Top-level unchanged.** A script's top-level value is not language-visible,
  so `lower_program` keeps a bare trailing `if` as a statement (its pinned
  behavior is intentional); only method/branch implicit returns changed.
- **Still deferred:** implicit return of a trailing `case` or `begin`/`rescue`,
  and of a block/lambda's tail conditional (follow-up milestones).
- Five new unit tests (tail `if`, tail `unless`, leading-stmts-then-tail-`if`,
  recursive nested tail `if`, validator pass); all downstream backend suites
  (Python/TypeScript/JavaScript/Go/Rust — including the native compile-and-run
  execution proofs) pass unchanged.

## [0.6.0] - 2026-07-01

(Cargo manifest minor bump 0.5.0 → 0.6.0.)

### Added (MX1 — mixin syntax → mixin builtins, frontend only, NO core-IR change)

First milestone of the `sir-mixins` cascade. Lowers Ruby's `module`/`include`/
`extend` mixin surface to ordinary `BuiltinCall`s, reusing the OOP milestone's
runtime-method-table pattern. Everything rides the existing `Expr::BuiltinCall`
node — no new IR variant, no `Feature` enum change — so nothing here can create
a cross-PR enum/field hazard. Execution is delivered in later milestones
(MX2–MX6, backend runtimes); MX1 asserts the lowered IR shape only.

- **Module methods now register keyed by the module name.** Before MX1, a
  `module M … end` body's `def`s hoisted to *detached* top-level `Function`s
  (bare names) and recorded nothing — no `include M` could ever find them.
  MX1 routes the module body through the SAME registration-collecting path
  classes use (`lower_class_body`), keyed by the module name:
  `module M; def greet; "hi"; end; end` now emits
  `__def_method__("M", "greet", MakeClosure(M__greet))` right after the
  `ModuleDef`, and hoists the body under the module-qualified name `M__greet`
  (avoiding top-level collisions, exactly like class methods). `def self.m` in
  a module registers as a class method (`__def_class_method__`).

- **`include M` / `extend M` in a class or module body → mixin directives.**
  A paren-less (or parenthesized) call whose callee is `include`/`extend` and
  whose sole argument is a bare module constant `M` now lowers to
  `__include__("Owner", "M")` / `__extend__("Owner", "M")`, where `Owner` is the
  enclosing class/module being defined. The directive is emitted in the same
  registration slot the method `__def_*__` calls use, so it runs in source order
  right after the declaration. The module NAME is extracted as a `StrLit`
  (mirroring how `Foo.new` / class-method dispatch read a `Scope::Const`
  operand's name), keeping dispatch fully table-driven — never reflection on a
  source-derived name (the C3 RCE lesson).

- **Feature gating.** `module`/`include`/`extend` all observe the existing
  `Feature::Modules` (plus `Feature::Strings`/`Feature::Closures` for the
  emitted `StrLit`/`MakeClosure` args). No `Feature::Mixins` was added: that
  would be a core-IR (`semantic-ir`) change, which is out of MX1's frontend-only
  scope — the mixin builtins validate under `Modules` because they are ordinary
  `BuiltinCall`s.

### Deferred / known gaps (MX1)

- **Multi-module `include A, B`** and non-constant operands (`include some_expr`)
  fall through to the ordinary-call path in v0 (documented in
  `try_expand_mixin_call`); the single-module `include M` form is all MX1 needs.
- **Top-level `include M`** (outside any class/module) is unchanged (still a
  `DirectCall`) — the mixin owner is only defined inside a class/module body.
- **`super` inside a module method** has no anchoring class and remains out of
  scope (unchanged from the OOP milestone).

## [0.5.0] - 2026-07-01

(Cargo manifest minor bump 0.4.0 → 0.5.0.)

### Added (Issue #59 — class-method defs `def self.m` + `super` as an expression)

Lowering support for the two grammar features unblocked in
`coding-adventures-ruby-parser` 0.5.0.

- **`def self.m` / `def Recv.m` → class-method registration.** A `def` (or
  endless `def`) carrying a `def_receiver` node now routes to the
  ALREADY-EXISTING `register_class_method` path (previously unreachable — the
  grammar had no receiver production): it emits
  `__def_class_method__("Class", "m", MakeClosure(fn))` instead of
  `__def_method__`, and hoists the body under a `_cm`-suffixed class-qualified
  top-level name (`Counter__zero_cm`) so a class method and an instance method
  of the same name on the same class never collide.

- **Class-method CALL dispatch `Foo.bar` → `__class_method__`.** A non-`new`
  method call on a CONSTANT receiver (`Counter.zero`, `Foo.bar(x)`) now lowers
  to a new `__class_method__("Foo", "bar", …args)` builtin (routed by the
  Python backend to `_sir_oop_call_class_method`, the ancestry-walking lookup
  in the `def self.m` table). `.new` on a const still routes to `__new__` (the
  implicit constructor); a method call on a NON-constant receiver
  (`obj.meth`) still routes through `__method__`.

- **`super` in expression position → `__super__` as an `Expr`.** The
  `super`-lowering logic moved into a shared `lower_super_expr` helper that
  returns an `Expr::BuiltinCall("__super__", …)` (rather than a `Stmt`), so
  `x = super`, `super + 1`, and `puts(super)` slot the `__super__` marker
  anywhere an expression goes. The statement form (a bare `super` line) wraps
  the same helper in an `ExprStmt`. The zsuper param-forwarding / explicit-arg
  behaviour is unchanged.

### Deferred / known gaps

- **String concatenation via `super + "…"`** (or any Ruby `str + str`) is a
  PRE-EXISTING pipeline limitation unrelated to #59: Ruby `+` lowers to the
  numeric-seeded `_sir_plus` (`add` starts `total = 0`), so `"a" + "b"` raises
  `int + str` at runtime. The #59 super-as-expression execution-proof therefore
  uses `super + 1` (numeric); string `+` awaits a polymorphic-`+` follow-up.
- **Class-method CALL dispatch is wired for the Python backend only.** The
  JS/Go/Rust backends have no `call_class_method` runtime yet, so
  `__class_method__` is not emitted there (a per-backend follow-up).

## [0.4.0] - 2026-07-01

(Cargo manifest minor bump 0.3.0 → 0.4.0.  Note: earlier CHANGELOG headers use
a separate 0.9x/0.100 sequence that had drifted from the manifest version; this
entry tracks the actual `Cargo.toml` version.)

### Added (O2 — OOP production: real object-oriented Ruby executes end to end)

The frontend now PRODUCES the OOP wiring so object-oriented Ruby runs end to
end (Ruby → SIR → Python / TypeScript).  Before O2, classes parsed but their
methods were disconnected: `.new` was not wired to `initialize`, `super` /
`self` / `attr_accessor` did nothing.  O2 emits the missing wiring entirely
through the existing `BuiltinCall` envelope — **no core-IR change** — which the
O1 OOP runtime + backend emit arms consume.

- **Method registration.**  For every instance method `def m` in `class C`,
  the frontend now emits — right after the `ClassDef` in program order — a
  `__def_method__("C", "m", MakeClosure { fn_name, captures: [] })` builtin
  call.  The registrations run once at startup so `C.new` later finds an
  `initialize` and dispatch finds the methods.  (Class methods `def self.m`
  would emit `__def_class_method__` analogously; the register path is ready but
  currently unreachable — see the deferred note.)
- **Class-qualified hoisted names.**  A method defined in a class body now
  hoists under a class-qualified top-level name (`Dog__speak`, not the bare
  `speak`).  This is what makes inheritance + `super` work: `Animal#initialize`
  and `Cat#initialize` are two DISTINCT top-level functions (bare `initialize`
  would collide and the validator would reject the duplicate), yet both stay
  reachable so `super` can re-run the parent's.  The runtime method table is
  keyed on `(class, bare_method)`, so *dispatch* is by bare name; only the
  shared top-level symbol is qualified.  Method names ending in `?`/`!`/`=`
  map to `_p`/`_bang`/`_set` suffixes to stay valid identifiers.
- **`Foo.new(args)`.**  A `.new` call on a *constant* receiver lowers to
  `__new__("Foo", …args)` (→ `call_new`: allocate → push self → run inherited
  `initialize` → pop self → return the object), rather than a generic
  `__method__` dispatch.  Chaining falls out for free: `Foo.new(x).meth` nests
  as `__method__(__new__("Foo", x), "meth")` (and longer chains, e.g.
  `c.inc.inc`, the same way).
- **`super(args)` / bare `super`.**  Both now lower to
  `__super__(method_name, class_name, …args)`, threading the enclosing method +
  class names from lowerer context (`current_method` / `current_class`).  The
  runtime walks from `class_name`'s parent to the first ancestor implementation
  and runs it with the *current* self still bound.  Bare `super` (zsuper)
  forwards the enclosing method's parameters by reference (sorted for
  determinism); `super()` forwards nothing.
- **`self`.**  A bare `self` now lowers to `__self__()` (→ `current_self`) — the
  receiver on the runtime self-stack — rather than a plain local `VarRef
  "self"`.  As a dot-chain receiver (`self.count`) and as a method's self-return
  (`c.inc.inc`, where `inc` ends in `self`) it composes correctly.
- **`attr_accessor` / `attr_reader` / `attr_writer`.**  Each symbol argument
  expands into synthesized accessor method(s) — getter `def x; @x; end` and/or
  setter `def x=(v); @x = v; end` — hoisted like a hand-written method AND
  registered via `__def_method__`.  `attr_reader` = getter only, `attr_writer`
  = setter only, `attr_accessor` = both; `attr_accessor :a, :b` expands both.

### Execution proofs (the payoff)

Three golden Ruby programs are lowered and run through the Python backend under
a real CPython interpreter (and P1 additionally through the TypeScript backend
under `node`), asserting stdout:

- **P1** `class Dog; def initialize(name); @name = name; end; def speak;
  "#{@name} says woof"; end; end; print Dog.new("Rex").speak` → `Rex says woof`
  (construction, instance-method dispatch, `@ivar` through the pushed self,
  interpolation through the OOP path).
- **P2** an Animal/Cat inheritance program where `Cat#initialize` `super(name)`s
  into `Animal#initialize` on the shared self → `Tom with 4 legs`.
- **P3** a Counter with `attr_accessor :count`, `@count` mutation, and an
  `inc`-returns-`self` chain (`c.inc.inc; print c.count`) → `2`.

### Deferred (documented v0 limitations)

- **`def self.m` class methods do not parse.**  The ruby-parser `def` rule has
  no receiver production, so `def self.zero` is a parse error today.  The
  `__def_class_method__` register path is implemented and ready; wiring it
  awaits a grammar extension.  (The original P3 used `def self.zero`; it was
  restated using `Counter.new` so it still proves getter/setter + self-chain.)
- **`super` is statement-only.**  `super + expr` (super as a sub-expression)
  does not parse, so `super` used as a value (concatenated, etc.) is out of
  scope; `super(args)` as a bare statement (e.g. in `initialize`) works.
- **`puts` vs `print`.**  The golden programs print with `print`; `puts` is not
  in `sir-runtime-core`'s native `call_builtin` dispatch table (a pre-existing,
  OOP-unrelated backend coverage gap).
- **Cross-class same-name intra-class calls.**  A method that calls another of
  its class's methods by *bare name* lowers to `DirectCall` on that bare name,
  which will not resolve to the class-qualified hoisted function.  The golden
  programs do not do this; a future phase can qualify such intra-class calls.

## [0.100.0] - 2026-06-30

### Added (KW7 — keyword parameter & argument production, the Ruby-1.0 unblock)

The frontend now PRODUCES keyword parameters and keyword arguments — the
single most-requested modern-Ruby gap. `def f(a:)` / `def f(a: 1)` and
`f(a: 1)` previously could not even parse, let alone lower.

- **Def side (`extract_params`).** A `param` whose grammar suffix is a COLON
  (`a:` / `a: expr`) now lowers to `Param { kind: ParamKind::Keyword, .. }`.
  Required-vs-optional rides on the existing `default` field exactly as
  positional optionals do: a keyword param with a trailing `expression` (the
  `a: 1` form) gets `default: Some(_)` (OPTIONAL keyword); one without (`a:`)
  gets `default: None` (REQUIRED keyword). The COLON token is the sole
  discriminator between a keyword param and a positional-default (`a = 1`)
  param.
- **Call side (`lower_call_arg`).** A `call_arg` node carrying a COLON
  (`f(a: 1)`) now lowers to the first-class `Expr::KeywordArg { name, value }`
  — NOT a trailing hash literal. Positional args stay bare and precede the
  keyword (`g(1, y: 2)` → `args: [IntLit(1), KeywordArg{name:"y", ..}]`),
  matching the core's "keywords trail positionals" contract that the SIR
  validator enforces.
- **Feature manifest.** Any keyword param OR keyword arg now observes
  `Feature::KeywordParams`, and the manifest materialiser emits it (mirrors
  how a positional default observes `DefaultParams`), so the SIR validator
  accepts the used feature.
- **Validator round-trip.** A required keyword omitted at a call site is
  rejected by `semantic_ir::validate`; supplying it (or omitting an optional
  keyword) validates. The frontend produces the required-ness; the core
  enforces it.

### Changed

- Compile-compat stub arms for the core `Expr::KeywordArg` variant (KW1) are
  now backed by real production: the swap-safety reference check, the
  `yield`-rewrite and call-normalization `&mut` visitors, and the bound-name
  collector already recurse into the keyword arg's inner `value`.

### Tests

- Eleven new unit tests: `def f(a:)` → `Param{kind:Keyword, default:None}`;
  `def f(a: 1)` → `default:Some(IntLit 1)`; a mixed
  positional+required-keyword+optional-keyword signature lowers in declared
  order; `f(x: 2)` → `Expr::KeywordArg{name:"x", value:IntLit 2}`; a keyword
  arg follows a positional (`g(1, y: 2)`); the `KeywordParams` feature is
  observed on both the def and call sides but not by ordinary positional
  params; a required keyword omitted at a call is rejected by the validator
  while supplying it (or omitting an optional keyword) validates.
- End-to-end execution proof lives in `semantic-ir-to-python` (which already
  depends on this crate as a dev-dependency):
  `def greet(greeting:, name: "world")\n "#{greeting}, #{name}"\nend\n
  print greet(greeting: "hi")\nprint greet(greeting: "hi", name: "ada")` →
  Ruby SIR → Python source → CPython prints `hi, world` then `hi, ada`,
  proving keyword params/args bind BY NAME through the whole pipeline (the
  omitted optional `name` resolves to its default `"world"`).

### Notes

- Cargo crate version bumped `0.2.0` → `0.3.0`.

## [0.99.1] - 2026-06-30

### Fixed (bare-identifier method bodies now lower)

- `def f(a)\n a\nend` now lowers to a `Function` whose body tail is
  `VarRef("a", Param)`. It previously produced no function at all: the
  underlying `ruby-parser` `factor` rule let the bare identifier `a` swallow
  the method's closing `end`, so the `def` never closed and lowering saw no
  `def_statement`. The repair lives in `ruby-parser` (guarded bare-`KEYWORD`
  atom); this crate adds an end-to-end lowering regression test pinning the
  fixed behaviour (382 tests, up from 381).

## [0.99.0] - 2026-06-30

### Added (P7 — default / optional parameters, Ruby-1.0 gap closed)

- `def f(a = 1)` now PRODUCES a default parameter. The frontend used to extract
  the param name but *silently drop* the `= <default>` subtree; worse, the
  grammar `param` rule (`[ "*" | "**" ] NAME`) had **no default-value branch at
  all**, so `def f(a = 1)` did not even parse. Both gaps are now closed:
  - **Grammar.** `param` is extended to `param = [ "*" | "**" ] NAME [ EQUALS
    expression ]` in both `code/grammars/ruby.grammar` and the embedded
    `ruby-parser/src/_grammar.rs` (kept in sync per the grammar-tools rule).
  - **Lowering.** A new `Lowerer::extract_params` helper centralises the
    parameter walk for all three call sites (`def_statement`,
    `endless_def_statement`, `lambda_literal`). When a param carries a default
    `expression` child, it is lowered through the normal bounded
    `lower_expression` path into `Param { default: Some(Box::new(expr)), .. }`.
    Required / rest (`*r`) / kwrest (`**o`) params keep `default: None`.
  - **Call-time, param-scoped.** Ruby defaults evaluate at call time and may
    reference EARLIER params (`def f(a, b = a)` is legal Ruby). `extract_params`
    lowers each default with every prior param already visible as a
    `Scope::Param`, matching the SIR validator's model exactly. The temporary
    scope visibility is snapshotted and restored so it does not leak into the
    method body scope.
  - **Feature manifest.** A defaulted param now observes `Feature::DefaultParams`
    and the manifest materialiser emits it, so the SIR validator accepts the
    used feature.
  - **Partial calls.** A call that omits a defaulted arg (`f(5)` for
    `def f(a, b = 1)`) lowers to a call with FEWER args — the frontend lowers the
    args present and does not pad. The SIR validator (and the Python/JS backends)
    now permit omitting trailing defaulted args.

### Tests

- Six new unit tests: `def f(a = 1)` → `Param.default = Some(IntLit 1)`;
  `def f(a, b = a + 1)` → default referencing param `a` resolves to
  `Scope::Param` and the module validates; required/rest params keep no default;
  the DefaultParams feature is observed only when a default is present; and a
  partial call lowers without padding the omitted default.
- End-to-end execution proof lives in `semantic-ir-to-python` (which already
  depends on this crate as a dev-dependency):
  `def f(a, b = a + 1)\n  b + 0\nend\nprint f(5)\nprint f(5, 10)` → Ruby SIR →
  Python source → CPython prints `6` then `10`, proving the default is genuinely
  call-time and param-scoped through the whole pipeline.

### Notes

- A pre-existing parser quirk (a method body that is a single *bare* identifier,
  e.g. `def f(a)\n  a\nend`, mis-parses as a no-paren call) is unrelated to this
  change; the new tests use honest expression bodies (`a + 0`).
- Cargo crate version bumped `0.1.0` → `0.2.0`.

## [0.98.0] - 2026-06-26

### Changed (M5 — `when` uses case-equality `===`, not `==`)

- A `when` clause now lowers with Ruby case-equality semantics instead of plain
  `==`. Per value:
  - a bare constant (`when Integer` / `when MyClass`) → a class match, lowered
    to `x.is_a?(Const)` via the `__method__` dispatch envelope (the backend
    passes a `Const` operand to `is_a?` as its name string, so a built-in class
    name needs no binding);
  - a range (`when 1..5`), regex (`when /re/`), or any other value → the
    `case_eq(pattern, x)` runtime helper (`sir-runtime-oop`), which dispatches
    Range→membership, Regexp→match, else `==`.
  Multiple values in one `when` still OR-chain. The `case_eq` floor is `==`, so
  a literal `when 5` keeps plain-equality behaviour. Closes the v0 caveat that
  `when` ignored class/range/regex case-equality.

## [0.97.0] - 2026-06-26

### Added (M4 — general outer-local block captures)

- A block that *reads* a local or parameter of its enclosing scope now
  **captures** it. When `hoist_block_to_function` lowers a block body, it
  detects free reads (`VarRef{scope:Local}`) of names bound in the immediate
  enclosing method/block, rewrites them to `Scope::Capture`, and threads the
  enclosing value in as a `MakeClosure` capture (which the Python/TypeScript
  backends prepend as a leading parameter). Previously such references became
  unbound names in the hoisted `__block_<n>` function — invalid SIR — so any
  block closing over an outer variable failed to lower.

  Example now supported end-to-end:

  ```ruby
  def run
    base = 100
    apply { |n| print n + base }   # `base` is captured
  end
  ```

- Capture rule (v0): read-only, single-level. A name is captured iff it is
  read in the body, is bound in the *immediate* enclosing scope, and is NOT
  bound inside the block (block param, block-local, or assigned anywhere in
  the body — an in-block assignment makes it block-local). Capturing a
  variable two scopes up (capture chaining) and write-back to the enclosing
  binding (by-reference capture) remain documented cut-lines, shared with
  RB2's nested-`yield` limitation.

- Internal: `hoist_block_to_function` now returns the `MakeClosure` capture
  values directly (replacing the RB2-only `block_capture_values` helper), so
  the enclosing-block (`__sir_block__`) capture and the new outer-local
  captures are threaded through one consistent, ordered list.

## [0.96.0] - 2026-06-22

### Added (M3 — faithful variadic def parameters)

- `def f(*rest)` and `def g(**opts)` now lower to a `Param` whose new
  `kind` field is `ParamKind::Rest` / `ParamKind::KwRest` instead of dropping
  the `*` / `**` prefix and emitting a bare positional `Param`. All three
  def-param lowering paths (`lower_def_statement`, the singleton-def path, and
  the endless-def path) detect the leading prefix token and set the kind;
  ordinary positionals stay `Required`. The lossy-limitation comment is
  removed. No grammar change (the parser already accepts `*`/`**` on def
  params). Closes the def side of variadics; the call side (`f(*arr)`) was
  already handled by Q9c.

## [0.95.0] - 2026-06-21

### Added (RB2 — `yield` inside a hoisted block captures the enclosing block)

- A `yield` lexically inside a block literal belongs to the *enclosing
  method* (`def outer; helper(2) { yield 99 }; end` — the block, when
  called, invokes `outer`'s block). This is now lowered faithfully, for
  both bare-name (`foo { … }`) and receiver (`recv.each { … }`, RB1)
  blocks:
  - The hoisted block `Function` declares a `__sir_block__` **capture**,
    and each in-block `yield` becomes an `IndirectCall` through
    `VarRef("__sir_block__", Scope::Capture)`.
  - The enclosing `def` gains the trailing `__sir_block__` parameter (via
    `thread_block_param`, now also keyed on a new
    `block_captures_enclosing` signal) and is registered in
    `block_param_methods` so Q9f threads the block at its call sites.
  - The block's `MakeClosure` carries a `CaptureValue { "__sir_block__"
    → VarRef("__sir_block__", Param) }`, binding the enclosing method's
    block into the closure.
- The yield-rewrite walk (`rewrite_yields_in_*`) is now parameterized by
  the target `Scope` (`Param` for method bodies, `Capture` for hoisted
  blocks). `hoist_block_to_function` runs it (scope `Capture`) only when
  inside a method body (new `in_def_body` flag); at the top level the raw
  `yield` is preserved (no enclosing block exists) — `top_level_block_yield_is_not_captured`.
- New tests: `yield_inside_block_captures_enclosing_block`,
  `yield_inside_receiver_block_captures_enclosing_block`,
  `top_level_block_yield_is_not_captured`. Each validates. This reopens
  and resolves the earlier Q10d v0 cut-line (now genuinely reachable
  after RB1's grammar fix). v0: block bodies referencing *other* outer
  locals still aren't captured.

### Notes / v0 cut-lines

- Capture threading fires only for a block lowered **directly** in the
  method body. A `yield` inside a block **nested within another block**
  (`[1].each { [2].each { yield } }`) is *not* threaded — that would need
  the intermediate block to re-capture `__sir_block__` (capture chaining).
  Instead the inner block keeps its raw `yield`, which is valid SIR, so
  the module still validates (`nested_block_yield_still_validates`) rather
  than emitting an invalid cross-level reference.

## [0.94.0] - 2026-06-19

### Added (RB1 — hoist a trailing block on a receiver/dotted method call)

- `fold_one_dot_call` now detects the optional trailing `block` the
  grammar admits on a `dot_call` (`recv.each { … }` / `recv.each do …
  end`), hoists it to a top-level Function via `hoist_block_to_function`,
  and appends the resulting `MakeClosure` as the `__method__` envelope's
  trailing argument — mirroring `method_with_block` for bare-name calls.
  Previously the block was silently dropped (lowered as a plain
  `__method__(recv, "each")`). Captures remain empty in v0 (shared
  block-closure limitation).
- New tests: `receiver_method_brace_block_is_hoisted_and_attached`,
  `receiver_method_do_end_block_is_hoisted_and_attached`,
  `receiver_method_without_block_has_no_closure`. Each validates.

### Notes

- This is RB1 of the receiver-block series. Threading the block as the
  enclosing method's implicit block when its body `yield`s (RB2 / the
  reopened Q10d) and backend execution-proof (RB3) follow as separate PRs.

## [0.93.0] - 2026-06-19

### Added (Q10c — parenless/argless call to a yielding method)

- A bare, parenless reference to a known block-taking method (`foo` with
  no `()`/args) reaches the lowerer as `VarRef { scope: Local }` — the
  method-call parser can't distinguish a zero-arg call from a variable.
  The Q9f call-site pass now rewrites such a `VarRef`, when its name is in
  `block_param_methods`, into `DirectCall { fn_name, args: [NilLit] }`,
  threading a nil block so the call's arity matches the def's trailing
  `__sir_block__` parameter. Previously these calls were left as `VarRef`
  and never threaded.
- **Shadow-safe.** The rewrite fires only when the name is *not* bound as
  a param/local anywhere in the enclosing function. A new
  `collect_bound_names_*` pre-pass gathers each function's param + `let`/
  `let*`/`Assign`/loop-var/rescue-binding names; the normalization walk
  (now carrying a `BlockNormCtx { methods, bound }`) skips any bound name.
  Conservative: a name bound anywhere in the function suppresses the
  rewrite for the whole function — this can only *miss* a rewrite, never
  produce a wrong one (so `t = 1; t` keeps `t` a local `VarRef`).
- New tests:
  `parenless_call_to_yielding_method_becomes_direct_call_with_nil_block`,
  `local_shadowing_a_method_name_stays_a_varref`,
  `parenless_reference_to_non_block_method_is_left_alone`. Each validates.

## [0.92.0] - 2026-06-19

### Added (Q10b — `block_given?`)

- `block_given?` (which reaches the lowerer as a bare `VarRef` named
  `"block_given?"`, being parenless) is now rewritten, inside a method
  body, to `not(null?(__sir_block__))` — i.e. "is the threaded block
  parameter non-nil". Both builtins are already supported (a native
  `not` arm + runtime-core `null?` dispatch), so it emits with no backend
  change and validates.
- The explicit-block-param detection (`thread_block_param`) now fires on
  `block_given?` as well as `yield`, so a method that only queries
  `block_given?` (and never yields) still gains the trailing
  `__sir_block__` parameter and is registered in `block_param_methods`
  (so Q9f threads the block at its call sites). The rewrite reuses the
  existing control-flow-descending walk and still does not descend into
  `MakeClosure`.
- New tests: `block_given_in_yielding_method_becomes_nil_check`,
  `block_given_alone_threads_block_param`,
  `method_without_block_given_or_yield_is_unchanged`. Each threaded
  module re-validates.

## [0.91.0] - 2026-06-19

### Added (Q9f — explicit block-param ABI, part 2: call-site normalization)

- A new post-lowering pass in `compile()` threads the matching block
  argument at every `DirectCall` to a method that gained a trailing
  `__sir_block__` parameter in Q9e (tracked in `Lowerer::block_param_methods`).
  Running after the *whole* program is lowered makes the pass
  order-independent: call-before-def and mutual recursion both thread
  correctly because the method registry is fully populated first.
- For each such call, the trailing argument slot is normalized so call
  arity matches the threaded def:
  - trailing `MakeClosure` (`foo { … }` / `foo do … end`) — already the
    block; left as-is.
  - trailing `BuiltinCall("block_pass", [inner])` (`foo(&p)`) — unwrapped
    to `inner` (the proc/block value).
  - otherwise (`foo(1, 2)`) — append `NilLit` (no block passed; the
    parameter binds nil).
- New `Lowerer::normalize_block_call_args` + recursive
  `normalize_calls_in_{stmt,stmts,expr}` walk every function body
  (user functions + `main`), descending through control flow, nested
  calls, and `MakeClosure` capture values.
- New tests: `call_to_yielding_method_with_block_keeps_makeclosure`,
  `call_to_yielding_method_without_block_appends_nil`,
  `block_pass_to_yielding_method_unwraps_to_inner`,
  `call_to_non_block_method_is_unchanged`, `call_before_def_is_threaded`.
  Every threaded module re-validates.

### Notes / v0 cut-lines

- A **parenless, argless** call to a yielding method (`foo` with no `()`
  and no block) lowers to a `VarRef`, not a `DirectCall`, so it is not
  recognized as a call and is left un-threaded — a pre-existing
  call-detection limitation, not introduced here.
- A `yield` through a nil block (no block passed) raising the exact Ruby
  `LocalJumpError` class is deferred; runtime `apply` on nil surfaces a
  generic error.

## [0.90.0] - 2026-06-19

### Added (Q9e — explicit block-param ABI, part 1: def threading + yield rewrite)

- A method whose body contains a direct `yield` now gains a trailing
  reserved parameter `__sir_block__` (an ordinary untyped `Param`), and
  each in-body `yield` is rewritten from `BuiltinCall("yield", args)` to
  `IndirectCall { target: VarRef("__sir_block__", Scope::Param), args }`.
  This makes Ruby's implicit block channel explicit in the SIR so the
  Python/TS backends can emit it natively (`IndirectCall` already lowers
  to runtime-core `apply`, and ordinary params emit directly) — **no
  backend or `semantic-ir` core change required**.
- New `Lowerer::thread_block_param` runs at the return of both
  `lower_def_statement` and `lower_endless_def_statement`. The recursive
  rewrite (`rewrite_yields_in_{block,stmts,stmt,expr}`) descends through
  control flow (`If`/`While`/`ForRange`/`ForEach`/`TryCatch`/`Block`) and
  ordinary call/expression children, but deliberately **stops at
  `Expr::MakeClosure`** (a `yield` inside a hoisted block belongs to its
  own enclosing method — a documented v0 cut-line) and does not descend
  into class/module/singleton declaration bodies (their `def`s are
  hoisted and threaded in their own right).
- Threading a method records its name in the new
  `Lowerer::block_param_methods` set and requests `Feature::Closures` +
  `Feature::DynamicTyping`, keeping the manifest in sync with the
  introduced `IndirectCall` and untyped param.
- New tests: `def_with_yield_threads_block_param_and_rewrites_yield`,
  `def_without_yield_is_unchanged`, `yield_inside_if_in_def_is_rewritten`.

### Notes / v0 cut-lines

- **Call-site threading is NOT in this release.** Until Q9f wires the
  matching block argument at every call to a `block_param_methods`
  method, a direct call to such a method is arity-short by one. `yield`
  used directly at the top level (`main`) is unaffected and still lowers
  to `BuiltinCall("yield", …)` (it is not a `def` body).
- `yield` inside a block literal, `block_given?`, proc-vs-lambda arity,
  and non-local `return`/`break` from blocks remain deferred.

## [0.89.0] - 2026-06-03

### Added (FC — array splat pattern lowering)

- **One-splat** `in [a, *rest, b]` now lowers structurally via the new
  `lower_array_pattern_one_splat`: a relaxed `len(target) >= fixed_count`
  check (vs the exact `==` of the no-splat path), front-anchored fixed
  elements indexed from the head (`target[i]`), back-anchored fixed
  elements indexed from the tail (`target[len - k]`), and — for a named
  splat — the middle slice bound via a `__seq_slice__(target, pre, len -
  post)` marker `BuiltinCall` (SIR has no first-class sequence slice). A
  bare `*` binds nothing. Each fixed element recurses through
  `lower_in_clause_pattern`, so literal / binding / nested sub-patterns
  compose. Requests `Feature::Sequences` + `Feature::ShortCircuit`.
- **Find** patterns (two splats, `[*, x, *]`) remain a documented v0
  limitation and fall back to the `__pattern_match__` marker — a
  contiguous-window search can't be expressed inline in the current IR.

New `array_pattern_splat_count` helper dispatches: 0 splats → existing
`lower_array_pattern`; 1 splat → `lower_array_pattern_one_splat`; ≥2 →
marker.

New tests: `case_in_array_one_splat_lowers_structurally`,
`case_in_array_one_splat_validates`,
`case_in_array_anonymous_splat_binds_nothing`,
`case_in_array_find_pattern_falls_back_to_marker`.

## [0.88.0] - 2026-06-03

### Added (FC — pin `^x` and class `Foo(x)` pattern lowering)

- **Pin** `in ^x` lowers to `scrutinee == x` (an equality `BuiltinCall`
  over a `Scope::Local` `VarRef`), no binding. The leading `^` lexes as a
  `Name` token (value "^"), so the lowerer skips it to find the pinned
  identifier.
- **Class** `in Foo(p, …)` lowers via new `lower_class_pattern` to
  `is_a?(scrutinee, "Foo") && <positional deconstruction>`: the class is
  a `StrLit` of its name (no `Const` decl / `Constants` feature needed),
  and positional sub-patterns match `scrutinee[i]` (a `len == N` check
  plus recursion through `lower_in_clause_pattern` via `SeqIndex`). v0
  simplification: indexes the scrutinee directly rather than calling
  `#deconstruct`. Requests `Feature::Strings` (+ `Sequences` /
  `ShortCircuit` when positional sub-patterns are present).

New tests: `case_in_pin_pattern_lowers_to_equality_with_local`,
`case_in_class_pattern_lowers_to_is_a_check`,
`case_in_pin_and_class_patterns_validate_e2e`.

## [0.87.0] - 2026-06-03

### Added (FC — `case/in` hash-pattern structural lowering)

Hash patterns in `case/in` (`in {name: n, age: 30}`) now lower to a real
structural match instead of the `__pattern_match__` marker. New
`lower_hash_pattern` mirrors `lower_array_pattern` but keys by symbol:

- Each `hash_pattern_pair` `key: <subpat>` builds a `MapGet(target, :key)`
  sub-scrutinee and recurses through `lower_in_clause_pattern`, so
  literal, binding, nested array, and nested hash sub-patterns all
  compose. Literal pairs AND a `target[:key] == lit` check into the
  condition; binding pairs add a `LetBinding` to the match-arm prefix.
- The Ruby 3.1 shorthand `{name:}` now **binds** `name = target[:name]`
  (previously a documented no-op).
- Array patterns containing hash sub-patterns (`[{a: 1}, 2]`) now lower
  structurally too (the lowerability guard and element loop gained a
  `hash_pattern` arm) instead of falling back to the whole-pattern marker.
- Declares `Feature::Maps` (for `MapGet`), `Feature::Symbols` (symbol
  keys), and `Feature::ShortCircuit` (the `&&` chain).

**v0 limitation:** Ruby hash patterns require each listed key to be
*present*; SIR has no map has-key primitive, so presence is only enforced
indirectly (via literal `==` checks) — a hash pattern of pure bindings
matches on shape alone. Documented on `lower_hash_pattern`.

New tests: `case_in_hash_pattern_binding_emits_mapget_letbinding`,
`case_in_hash_pattern_literal_emits_equality_on_mapget`,
`case_in_hash_pattern_shorthand_binds`,
`case_in_hash_pattern_validates_e2e`,
`case_in_array_with_hash_element_lowers_structurally` (replaces the prior
marker-fallback test). Full lexer+parser+lowerer suites green (338
lowerer tests).
## [0.86.0] - 2026-06-03

### Added (FC — `__END__` end-to-end coverage; tests only)

Coverage test that a program with a trailing `__END__` data section
lowers cleanly from just the code above it (the lexer strips the data
section — see `coding-adventures-ruby-lexer` 0.25.0).  No lowerer code
change: `program_with_end_marker_lowers_only_the_code` pins the
behaviour and round-trips the SIR validator.

## [0.84.0] - 2026-06-01

### Added (Phase 26b (FC) — `refine Class do … end` refinement body)

`refine(Class) do … end` now lowers to a first-class
`BuiltinCall("refine", [<class>, <closure>])` with `EffectSet::PURE`,
rather than falling through to a `DirectCall("refine", …)` that the SIR
validator rejected as an undeclared callee.

`refine` is an ordinary block-taking method, so it arrives as a
`method_with_block` with the target class as its argument and the
refinement body as a block — **no grammar or lexer change** was needed.
The fix adds `"refine"` to the lowerer's `ruby_builtin_effects` table as a
PURE builtin (alongside `using` and the block-taking `lambda`/`proc`); the
target class is lowered through the normal expression path and the
refinement block is hoisted to a `MakeClosure` trailing argument by the
existing `lower_method_with_block` machinery.

This completes the Ruby 3.4 refinement surface (`using` + `refine`) — the
final slice of the Ruby 3.4 frontend full-coverage convergence.

New lowering tests: `refine_lowers_to_builtin_call`,
`refine_block_is_makeclosure_arg`, `refine_is_pure_and_validates_e2e`
(lower → validate round-trip).

## [0.83.0] - 2026-06-01

### Added (Phase 26a (FC) — `using Mod` refinement activation)

`using Mod` (refinement activation) now lowers to a first-class
`BuiltinCall("using", [<module>])` with `EffectSet::PURE`, rather than
falling through to a `DirectCall("using", …)` that the SIR validator
rejected as an undeclared callee.

`using` is an ordinary method (not a keyword), so it arrives as a
`method_call_no_paren` with the refinement module as its sole argument —
**no grammar or lexer change** was needed.  The fix adds `"using"` to the
lowerer's `ruby_builtin_effects` table as a PURE builtin, alongside the
other declaration-style forms; the module operand is lowered through the
normal expression path (e.g. a `Const` reference for `using Foo`).  In
this model the activation carries no runtime data effect.

New lowering tests: `using_lowers_to_builtin_call`,
`using_operand_is_the_module_ref`, `using_is_pure_and_validates_e2e`
(lower → validate round-trip).

## [0.82.0] - 2026-06-01

### Added (Phase 23d (FC) — `__dir__` pseudo-variable lowering)

Ruby's `__dir__` pseudo-variable now lowers to a compile-time `StrLit`
carrying the directory portion of the lowerer's `file_name`: the
substring before the final path separator (`/` or `\`), or `"."` when the
name has no directory component — the closest fixed value for "the
current directory" absent a runtime filesystem, consistent with how
`__FILE__` surfaces the bare module name.

Sibling of Phase 23a `__FILE__` / 23c `__LINE__`: because `__dir__` is
**not** a lexer keyword it arrives as an ordinary `Name` token and the
parser already matches it via `factor`'s bare-`NAME` alternative, so **no
grammar or lexer change** was needed.  The meaning is supplied entirely in
`lower_factor_atom`, intercepting the bare `Name` exactly like the sibling
pseudo-variables: when the token value is `__dir__` and it is **not**
shadowed by a local binding, we emit the `StrLit` and declare
`Feature::Strings`.  A `__dir__` shadowed by a prior local (`__dir__ = 1`)
keeps the local read (a `VarRef`), mirroring the existing shadow guards.
Scope: the bare form; the explicit-call form `__dir__()` is a deliberate
follow-up slice.

New lowering tests: `dir_keyword_lowers_to_strlit`,
`dir_keyword_declares_strings_feature`, `dir_keyword_validates_e2e`
(lower → validate round-trip), `dir_keyword_shadowed_by_local_is_varref`.

## [0.81.0] - 2026-06-01

### Added (Phase 23c (FC) — `__LINE__` pseudo-variable lowering)

Ruby's `__LINE__` pseudo-variable now lowers to a compile-time `IntLit`
carrying the (1-based) source line of the `__LINE__` token itself.

Sibling of Phase 23a `__FILE__`: because `__LINE__` begins with `_` it is
**not** a lexer keyword — it arrives as an ordinary `Name` token and the
parser already matches it via `factor`'s bare-`NAME` alternative, so **no
grammar or lexer change** was needed.  The meaning is supplied entirely in
`lower_factor_atom`, intercepting the bare `Name` exactly like the
`__FILE__` / bare-`raise` cases: when the token value is `__LINE__` and it
is **not** shadowed by a local binding, we emit `IntLit { value:
tok.line }`.  Unlike `__FILE__`'s `StrLit`, **no `Feature` declaration is
required** — integers are a baseline SIR capability.  A `__LINE__`
shadowed by a prior local (`__LINE__ = 7`) keeps the local read (a
`VarRef`), mirroring the existing shadow guards.

New lowering tests: `line_keyword_lowers_to_intlit`,
`line_keyword_tracks_source_line`, `line_keyword_validates_e2e`
(lower → validate round-trip), `line_keyword_shadowed_by_local_is_varref`.

## [0.80.0] - 2026-06-01

### Added (Phase 23a (FC) — `__FILE__` pseudo-variable lowering)

Ruby's `__FILE__` pseudo-variable now lowers to a compile-time `StrLit`
carrying the lowerer's `file_name` (the SIR module identifier the source
was compiled under) — the closest fixed value we have for "the current
source file" absent a runtime filesystem.

Because `__FILE__` begins with `_` it is **not** a lexer keyword: it
arrives as an ordinary `Name` token and the parser already matches it via
`factor`'s bare-`NAME` alternative, so **no grammar or lexer change** was
needed.  The pseudo-variable's meaning is supplied entirely here, in
`lower_factor_atom`, intercepting the bare `Name` exactly like the
existing bare-`raise` case: when the token's value is `__FILE__` and it is
**not** shadowed by a local binding, we emit the `StrLit` and declare
`Feature::Strings` (already permitted by the manifest builder allowlist).
A `__FILE__` shadowed by a prior local (`__FILE__ = 1`) keeps the local
read (a `VarRef`), mirroring the `raise` shadow guard.

New lowering tests: `file_keyword_lowers_to_strlit`,
`file_keyword_declares_strings_feature`, `file_keyword_validates_e2e`
(lower → validate round-trip), `file_keyword_shadowed_by_local_is_varref`.

## [0.79.0] - 2026-06-01

### Added (Phase 24b (FC) — `undef name` lowering)

An `undef_statement` node (`undef name`) lowers to a statement-position
`BuiltinCall("undef", [StrLit(name)])` with `EffectSet::PURE`, mirroring
the Phase 24a `alias` lowering exactly.  The method name is surfaced as
a `StrLit` — it is a method name, **not** a local variable, so emitting
a `VarRef` would be wrong and the SIR validator would reject the
never-bound name.  `Feature::Strings` is declared for the `StrLit`
operand (already permitted by the manifest builder allowlist).  Effects
are `PURE`, like the other declaration-ish keyword statements
(`redo`/`retry`/`defined?`/`alias`).

This first slice covers the canonical single-bare-name form (`undef
foo`); the symbol form (`undef :name`) and the multi-name form (`undef
a, b`) are deliberate follow-ups.  New lowering tests:
`undef_lowers_to_builtin_call`, `undef_operand_is_string_literal`,
`undef_is_pure_and_validates_e2e` (lower → validate round-trip).

## [0.78.0] - 2026-06-01

### Added (Phase 24a (FC) — `alias new old` lowering)

An `alias_statement` node (`alias new old`) lowers to a statement-position
`BuiltinCall("alias", [StrLit(new), StrLit(old)])` with `EffectSet::PURE`.
The two method names are surfaced as `StrLit`s — they are method names,
not local variables, so emitting `VarRef`s would be wrong and the SIR
validator would reject the never-bound names.  Effects are `PURE`: in
this model the alias declaration carries no runtime data effect, mirroring
how the other declaration-ish keyword statements (`redo`/`retry`/`defined?`)
are treated.  Because the operands are `StrLit`s, the lowerer now declares
`Feature::Strings` for an `alias` (already permitted by the manifest
builder allowlist).

This first slice covers the canonical two-bare-name form; symbol operands
(`alias :new :old`) are a deliberate follow-up.  New pins:
`alias_lowers_to_builtin_call`, `alias_operands_are_string_literals`,
`alias_is_pure_and_validates_e2e` (lower → validate round-trip).

## [0.77.0] - 2026-06-01

### Added (Phase 23b (FC) — `defined?` operator lowering)

A `defined_expression` node (`defined?(x)` / `defined? x`) lowers to
`BuiltinCall("defined?", [operand])` with `EffectSet::PURE` — `defined?`
inspects whether its operand is defined, never raises, and has no side
effects.  Handled in both expression position (`lower_expression`, e.g.
an assignment RHS) and statement position (the statement dispatch wraps
it in `Stmt::ExprStmt`).  The operand is carried as a lowered argument so
a downstream emitter can reconstruct the source; a faithful backend does
not evaluate it.

v0 limitation: the operand lowers like any expression, so
`defined?(undefined_local)` produces a `VarRef` the validator rejects as
an unknown name — `defined?` on a never-bound bare local is not
representable yet (operands are bound names, calls, or literals in
practice).

New lowering pins: `defined_with_parens_lowers_to_builtin_call`,
`defined_of_literal_lowers_with_literal_operand`,
`defined_in_statement_position_is_expr_stmt`,
`defined_expression_validates_e2e`. Test count: 307 → 311.

## [0.76.0] - 2026-06-01

### Changed (Phase 13b (FC) — nested array patterns lower structurally)

`case/in` array patterns now lower **recursively**: a nested array
sub-pattern (`in [[1], y]`) is matched structurally instead of dropping
the whole pattern to the `__pattern_match__` marker.
`lower_fixed_array_pattern` was generalized to `lower_array_pattern`,
which takes a `target` expression (the scrutinee, or a `SeqIndex` into it
for a nested level) and recurses on `array_pattern` elements:

```text
in [[1], y]   # cond:   ((len(x)==2) && ((len(x[0])==1) && (x[0][0]==1)))
              # prefix: let y = x[1]
```

Each level's length check leads its sub-match in the short-circuiting
`&&` (`Expr::LogicalAnd`) chain, so every `SeqIndex` is in bounds before
evaluation (outer length → element → inner length → …). A new recursive
`array_pattern_is_lowerable` gate decides marker-vs-structural: literal /
binding / nested-array elements are lowerable; **hash sub-patterns at any
depth** keep the whole pattern on the `__pattern_match__` marker.

Also declares `Feature::ShortCircuit` whenever a structural pattern emits
`Expr::LogicalAnd` (a literal or nested element) and adds it to the
manifest builder allowlist — fixing a latent gap where a literal array
pattern (`in [1, 2]`, Phase 13a) emitted `LogicalAnd` without declaring
the feature (not previously exercised through the validator).

New / updated lowering pins: `case_in_nested_array_pattern_lowers_structurally`
(replaces the 13a marker assertion), `case_in_array_with_hash_element_keeps_marker_fallback`,
`case_in_literal_array_pattern_validates_e2e` (ShortCircuit regression),
`case_in_nested_array_pattern_validates_e2e`. Test count: 304 → 307.

## [0.75.0] - 2026-06-01

### Changed (Phase 13a (FC) — fixed-arity array patterns lower structurally)

`case/in` array patterns whose elements are all simple (`literal_pattern`
or `binding_pattern`) now lower to a real structural match instead of the
v0 `BuiltinCall("__pattern_match__", …)` marker:

```text
case x
in [1, b, 3] then …    # cond:   ((len(x) == 3) && (x[0] == 1)) && (x[2] == 3)
                       # prefix: let b = x[1]   (runs only in the match arm)
```

The length check (`SeqLen == N`) leads the short-circuiting `&&`
(`Expr::LogicalAnd`) chain so every `x[i]` (`Expr::SeqIndex`) is in bounds
before evaluation, and binding `LetBinding`s run only in the match body
(where the whole condition already held).  `Feature::Sequences` is
requested for the new sequence nodes.

Nested sub-patterns (`in [[1], 2]`) and all hash patterns keep the v0
`__pattern_match__` marker (refactored into a shared
`pattern_match_marker` helper) — to be replaced in a later phase per the
Tier-3 marker-replacement convention.

New / updated lowering pins:
`case_in_literal_array_pattern_lowers_to_structural_match` (replaces the
old marker assertion), `case_in_array_pattern_binds_name_elements`
(`in [a, b]` → element bindings), `case_in_nested_array_pattern_keeps_marker_fallback`,
`case_in_array_pattern_validates_e2e` (manifest + validator round-trip).
Test count: 301 → 304.

## [0.74.0] - 2026-06-01

### Changed (Phase 17a (FC) — heredoc bodies interpolate)

A heredoc (`<<EOF` / `<<-EOF` / `<<~EOF`) interpolates `#{…}` like a
double-quoted string, but `lower_heredoc_literal` previously emitted the
extracted body as a single raw `StrLit` — so `<<EOF\nhi #{name}\nEOF`
mis-lowered `#{name}` as literal text.  The lowerer now routes the
extracted body through the shared `lower_string_literal_with_interp`
splitter:

- A body with no `#{…}` still lowers to one `StrLit` (unchanged — the
  existing plain/`<<-`/`<<~` pins still hold).
- An interpolating body lowers to a `StrConcat` of literal runs and the
  lowered `#{…}` expressions, exactly as `"a#{x}b"` does (and so picks up
  Phase 20a recursive expression lowering + Phase 20b `StrConcat` +
  `Feature::StringInterpolation` for free).  A malformed-interp lowering
  error falls back to the verbatim body as a plain `StrLit`, keeping
  heredoc lowering infallible.

New lowering pins (+3): `interpolated_heredoc_lowers_body_to_str_concat`
(`<<EOF\nhi #{name}\nEOF` → 3-part `StrConcat`),
`interpolated_tilde_heredoc_with_expression_lowers_recursively`
(`<<~EOF` body `#{1 + 2}` → real `+` call),
`interpolated_heredoc_validates_e2e_and_declares_feature` (manifest +
validator round-trip).  Test count: 298 → 301.

## [0.73.0] - 2026-06-01

### Changed (Phase 20b (FC) — interpolation lowers to first-class `StrConcat`)

A multi-segment interpolated/concatenated string (`"a#{x}b"`) now lowers
to the new `Expr::StrConcat` node instead of the v0
`BuiltinCall("string_concat", …)` marker.  This is the marker→node
counterpart of Phase 20a (which replaced the `__interp__` body marker
with real recursive lowering): 20a turned each `#{…}` *body* into real
SIR, and 20b turns the *concatenation wrapper* into a real SIR node.

`lower_string_literal_with_interp` (shared by string **and** regex-pattern
interpolation) keeps its result-shape selection — empty → empty `StrLit`,
one segment → the bare segment, two-or-more → concat — but the 2+ case now
emits `StrConcat { parts }` and declares the new
`Feature::StringInterpolation` in the module manifest (alongside `Strings`
for any `StrLit` parts).

New / updated lowering pins:
`interpolated_string_with_bare_name_lowers_to_str_concat`,
`interpolated_string_with_expression_lowers_recursively`,
`interpolated_string_with_multiple_expr_interps_lowers_each_recursively`,
and the regex `regex_interpolation_lowers_pattern_to_concat` /
`regex_interpolation_validates_e2e` all now assert the `StrConcat` shape.
Added `adjacent_interps_with_no_literal_lower_to_str_concat_of_refs`
(`"#{a}#{b}"` → two-`VarRef` concat) and
`str_concat_module_declares_string_interpolation_feature` (manifest
agreement).  Test count: 296 → 298.

## [0.72.0] - 2026-05-31

### Changed (Phase 20a (FC) — string interpolation lowers to real SIR)

Expression interpolations inside double-quoted strings now lower to
genuine SIR instead of the v0 `__interp__` marker.  When a `#{…}` body
re-parses to exactly one expression / method-call statement, the new
`try_lower_interp_body` helper re-invokes `parse_ruby` on the body and
lowers its tail expression **in the current scope** — so `"sum=#{1+2}"`
becomes `string_concat([StrLit("sum="), BuiltinCall("+", [1, 2])])` and
`"#{a + b}"` resolves `a`/`b` as `VarRef`s the same way surrounding code
would.  Bare-name bodies (`"#{name}"`) keep their existing `VarRef`
fast-path.

Per the Tier-3 marker-replacement convention, the `__interp__` marker is
**retained as a fallback** for one phase: empty bodies (`#{}`),
multi-statement bodies (`#{a; b}`), and anything that doesn't parse to a
single expression statement still emit the verbatim marker.

**DoS guard.** Because a `#{…}` body may itself contain a nested
interpolated string (`"#{ "#{x}" }"`), the recursive re-parse path could
otherwise recurse without bound and exhaust the thread stack (an
uncatchable abort) on adversarial input.  A new `interp_depth` counter on
`Lowerer` caps re-parsing at `MAX_INTERP_DEPTH = 8` — far beyond any
legitimate nesting — and falls back to the safe `__interp__` marker past
the cap.

New lowering pins (+4, one existing marker test rewritten):
`interpolated_string_with_expression_lowers_recursively`
(`"sum=#{1+2}"` → real `+` call, not a marker),
`interpolated_string_with_multiple_expr_interps_lowers_each_recursively`
(`"a#{1}b#{2}c"` → five-segment concat),
`interpolated_string_with_binary_var_expr_lowers_recursively`
(`"#{a + b}"` → `+` over two `VarRef`s),
`interpolated_expression_string_validates_e2e`
(`puts("sum is #{1 + 2}")` round-trips the SIR validator),
`deeply_nested_interpolation_terminates_without_stack_overflow`
(200-level nesting terminates via the depth cap).
Test count: 292 → 296.

## [0.71.0] - 2026-05-31

### Added (Phase 21c (FC) — implicit `it` block parameter, Ruby 3.4)

When a block has NO explicit `|...|` header, no block-locals, and no
numbered params, the lowerer now detects a bare `it` reference in the
body and synthesizes a single positional parameter named `it`
(`Scope::Param`).

Detection (`block_uses_implicit_it`) flattens the block body's tokens in
source order — pruning nested `block` nodes — and treats an `it` `Name`
token as the implicit parameter ONLY when it is neither immediately
preceded by `.` (method name: `obj.it`) nor immediately followed by `(`
(call: `it(x)`).  So `it.foo`, `it + 1`, and `puts(it)` qualify, while
`it(1)` and `obj.it` do not.  Numbered params (`_N`) take precedence;
Ruby forbids mixing them with `it` anyway.

New lowering pins (+3): `implicit_it_synthesizes_single_param`
(`each { puts(it) }` → `__block_0.params == [it]`),
`implicit_it_method_call_does_not_synthesize_param`
(`each { it(1) }` → zero params), `implicit_it_passes_sir_validator`
(validator end-to-end).  Test count: 289 → 292.

## [0.70.0] - 2026-05-31

### Added (Phase 21b (FC) — implicit numbered block parameters `_1`..`_9`)

When a block has NO explicit `|...|` header and no block-locals, the
lowerer now scans the block body for `_1`..`_9` `Name` tokens, takes the
highest index used, and synthesizes positional parameters `_1`..`_<max>`
(`Scope::Param`).  Arity follows Ruby semantics: a body using `_2`
implies arity 2 (params `_1, _2`), even if `_1` is unreferenced.

The scan does NOT descend into nested `block` nodes — a `_N` inside an
inner block belongs to that inner block's own implicit parameter scope.
An explicit pipe header (or `;` block-locals) always wins; numbered
params apply only when no header is present.

New lowering pins (+3): `numbered_block_param_synthesizes_single_param`
(`each { puts(_1) }` → `__block_0.params == [_1]`),
`numbered_block_param_arity_is_highest_index` (`each { puts(_2) }` →
params `_1, _2`), `numbered_block_param_passes_sir_validator`
(validator end-to-end).  Test count: 286 → 289.

## [0.69.0] - 2026-05-31

### Added (Phase 21a (FC) — block-local variables `{ |x; y| … }`)

The block lowerer now splits the `block_params` pipe contents at the
`;` (Semicolon) token: names before it are block parameters
(`Scope::Param`), names after it are **block-local variables** — fresh
locals scoped to the block body.

Block-locals are:
- declared in the block's `declared_locals` scope so VarRefs to them
  resolve as `Scope::Local` (NOT `Scope::Param`);
- excluded from the synthetic function's parameter list;
- materialized as explicit `LetBinding <name> = NilLit` statements
  prepended to the block body, so the SIR validator recognizes them
  (Ruby block-locals start unbound / nil).

New lowering pins (+3): `block_local_is_not_a_param`
(`do |x; y|` → `__block_0.params` is just `[x]`),
`block_local_varref_resolves_as_local_not_param`
(`do |x; y| y = x; puts(y) end` → `y` VarRef is `Scope::Local`),
`block_with_block_local_passes_sir_validator` (validator end-to-end).
Test count: 283 → 286.

## [0.68.0] - 2026-05-31

### Added (Phase 11d (FC) — `return` WITH VALUE, coverage-confirmation)

No lowering change.  The Phase 6j arm already folds an optional trailing
expression after `return` into the single `BuiltinCall` argument
(bare → `NilLit`):

```
return [1, 2]  → ExprStmt(BuiltinCall("return", [SeqLit ...], Divergent))
return "ok"    → ExprStmt(BuiltinCall("return", [StrLit ...], Divergent))
```

The pre-existing pins covered `return 42`, bare `return`, `return x + 1`
inside a def, and a def-body validator run.  These pins add new payload
angles so the value-carrying contract stays nailed down.

New lowering pins (+3): `return_with_array_value_lowers_to_seqlit_arg`
(`return [1, 2]` → `SeqLit` arg, Divergent),
`return_with_string_value_lowers_to_strlit_arg` (`return "ok"` →
`StrLit` arg, Divergent),
`return_with_top_level_local_value_passes_sir_validator`
(`x = 5; return x` at top level validates — distinct from the existing
def-body pin).  Test count: 280 → 283.

## [0.67.0] - 2026-05-31

### Added (Phase 11c (FC) — `retry` keyword)

New `retry_statement` lowering, folded into the Phase 11b arm
(`redo_statement | retry_statement`):

```
retry → ExprStmt(BuiltinCall("retry", [], Divergent))
```

`retry` mirrors `redo`: a bare keyword lowering to a **zero-argument**
Divergent `BuiltinCall`.  It re-executes the enclosing `begin` block
from the top inside a `rescue` clause, so it diverges from straight-line
control flow.  No new SIR variant — it reuses the existing `BuiltinCall`
envelope, so the walker, validator, printer, and all four backends
handle it generically by name (same as `redo`/`break`/`next`).

New lowering pins (+3): `retry_lowers_to_zero_arg_divergent_builtin`
(`retry` → `retry`, 0 args, Divergent),
`retry_inside_begin_rescue_lowers` (`begin; x = 1; rescue; retry; end`
— the marker lands inside a `Stmt::TryCatch` rescue-clause body),
`retry_module_passes_sir_validator` (validates).  Test count: 277 → 280.

## [0.66.0] - 2026-05-31

### Added (Phase 11b (FC) — `redo` keyword)

New `redo_statement` lowering arm:

```
redo → ExprStmt(BuiltinCall("redo", [], Divergent))
```

`redo` lowers to a **zero-argument** Divergent `BuiltinCall` — distinct
from `break`/`next`, which always carry an operand (`NilLit` when bare).
It restarts the current loop iteration without re-checking the loop
condition, so it diverges from straight-line control flow.  No new SIR
variant: it reuses the existing `BuiltinCall` envelope, so the walker,
validator, printer, and all four backends handle it generically by name
(same as `break`/`next`/`yield`/`super`).

New lowering pins (+3): `redo_lowers_to_zero_arg_divergent_builtin`
(`redo` → `redo`, 0 args, Divergent),
`redo_inside_while_body_lowers` (`while x; redo; end` — the marker lands
inside the `Stmt::While` body), `redo_module_passes_sir_validator`
(validates).  Test count: 274 → 277.

## [0.65.0] - 2026-05-31

### Added (Phase 11a (FC) — `break`/`next` WITH VALUES, coverage-confirmation)

No lowering change.  The Phase 6j arm
(`return_statement | break_statement | next_statement`) already folds an
optional trailing expression into the single `BuiltinCall` argument and
falls back to `NilLit` when bare:

```
break 5   → ExprStmt(BuiltinCall("break", [IntLit 5], Divergent))
next 7    → ExprStmt(BuiltinCall("next",  [IntLit 7], Divergent))
break     → ExprStmt(BuiltinCall("break", [NilLit],   Divergent))
```

This release adds lowering pins from new angles so the value-carrying
contract is nailed down independently of `return` (whose pins already
existed): a value-carrying `break`, a value-carrying `next`, a bare
`break` (NilLit arg), and a validator end-to-end run where the payload
is a resolved local variable (`x = 1; break x`).

New lowering pins (+4): `break_with_value_lowers_to_int_arg` (`break 5`),
`next_with_value_lowers_to_int_arg` (`next 7`),
`bare_break_lowers_with_nil_arg` (`break` → NilLit),
`break_with_local_var_value_passes_sir_validator` (`x = 1; break x`,
validates).  Test count: 270 → 274.

## [0.64.0] - 2026-05-31

### Added (Phase 22d (FC) — `super` keyword)

New `super_statement` lowering arm, mirroring `yield`.  Two distinct
lowerings keyed on whether a `super_args` node is present:

- bare `super` (absent) → `BuiltinCall("zsuper", [])` — Ruby's implicit
  "zsuper" that forwards ALL of the enclosing method's arguments, so it
  carries no operands.
- `super()` / `super(x)` / `super x` (present) → `BuiltinCall("super",
  lowered_args)`, where `super()` lowers to **zero** args (forwards
  nothing) — semantically distinct from bare zsuper.

`super_args` reuses `lower_call_arg`, so splat / double-splat /
block-pass / `...` envelopes nest inside `super` args for free.  Effects
are PURE (matching `yield`): the dispatched parent method's effects are
accounted for at its own definition/call site, so the marker stays PURE
to avoid double-counting.  No new SIR variant.

New lowering pins (+3): `bare_super_lowers_to_zsuper_builtin` (`super` →
`zsuper`, 0 args), `super_empty_parens_lowers_to_super_builtin_no_args`
(`super()` → `super`, 0 args), `super_with_args_lowers_and_passes_validator`
(`super(1, 2)` → `super` with 2 args, validates).  Test count: 267 → 270.

## [0.63.0] - 2026-05-31

### Added (Phase 22c (FC) — `...` argument forwarding)

`lower_call_arg` now rewrites a bare-name `...` operand into the nullary
marker `BuiltinCall("forward_args", [])`.  Because the lexer fuses `...`
into a single Name-typed token, `n(...)` parses with the bare name `...`
in the call_arg's expression slot, which lowers to `VarRef { name: "..."
}`; the new check (on the no-prefix path only) detects that exact name
and substitutes the marker.  `...` is not a legal Ruby identifier, so a
bare `VarRef("...")` can only have come from forwarding — the rewrite is
unambiguous.  A beginless-range argument `m(...5)` lowers to a `range`
builtin (the `...` is the operator, not a bare name) and is left
untouched.  No new SIR variant.

`def m(...)` lowers to a function with **zero** params (v0 lossy: the
bare `...` is a literal token in `params`, not a `param` node, so the
param collector emits nothing); the call-side `forward_args` marker
carries the forwarding semantics.

New lowering pins (+3):
- `forward_args_call_arg_lowers_to_forward_args_builtin` (`f(...)`) —
  node shape: `BuiltinCall("forward_args", [])` (no operand).
- `forward_all_def_and_call_passes_sir_validator`
  (`def m(...) ; puts(...) ; end`) — round-trip lowers, `m` has 0 params,
  and the module passes `semantic_ir::validate` (`puts` intrinsic).
- `beginless_range_arg_does_not_lower_to_forward_args` (`m(...5)`) —
  regression: lowers to `BuiltinCall("range", …)`, not `forward_args`.

Test count: 264 → 267.

## [0.62.0] - 2026-05-31

### Added (Phase 22b (FC) — `&blk` block-pass call argument)

`lower_call_arg` gained a `&` arm: a block-pass argument lowers to
`BuiltinCall("block_pass", [inner])`, mirroring the `splat` /
`double_splat` marker envelopes.  SIR has no first-class block-argument
slot, so the marker lets downstream emitters reconstruct `&expr` (the
operand may be a Proc, a `&:sym` symbol-to-proc, or any `to_proc`-able
object — all preserved verbatim inside the envelope).  No new SIR
variant; the prefix detector now matches `"*" | "**" | "&"`.

New lowering pins (+3):
- `block_pass_call_arg_lowers_to_block_pass_builtin` (`f(&blk)`) — node
  shape: `block_pass` wrapping the `VarRef` operand.
- `block_pass_call_arg_passes_sir_validator` (`puts(&blk)`) — lowers
  AND passes `semantic_ir::validate` (uses the `puts` intrinsic so the
  unknown-callee check passes).
- `block_pass_after_positional_lowers_in_order` (`f(7, &blk)`) — locks
  the positional-then-block-pass two-arg ordering.

Test count: 261 → 264.

## [0.61.0] - 2026-05-31

### Added (Phase 22a (FC) — `**` double-splat call argument, coverage)

No lowering change.  `lower_call_arg` (Phase 6s) already maps a `**`
double-splat call argument to `BuiltinCall("double_splat", [inner])`,
mirroring the single-splat `BuiltinCall("splat", …)` shape.  This phase
adds three new-angle lowering pins that earlier coverage did not run:

- `double_splat_only_arg_passes_sir_validator` (`puts(**opts)`) — a
  lone double-splat call arg both lowers to one `double_splat` builtin
  AND passes `semantic_ir::validate` (the prior shape pins never ran the
  validator on a double-splat-only call).
- `double_splat_hash_literal_inner_lowers_and_validates`
  (`puts(**{a: 1})`) — the double-splat operand lowers to a `MapLit`
  wrapped by `double_splat`, and the module validates.
- `double_splat_after_leading_positional_lowers_in_order`
  (`f(7, **opts)`) — pins the positional-then-double-splat two-arg
  ordering with no intervening single splat.

(`puts` is used for the validator-backed pins because it is a known
intrinsic — an unknown callee `f` trips the validator's
unknown-function check; `puts` lowers to `BuiltinCall("puts", …)`.)

Test count: 258 → 261.

## [0.60.0] - 2026-05-31

### Added (Phase 19d (FC) — `%r{...}` regex literal)

`lower_factor_atom` gained a `%r{...}` dispatch (placed before the
`/.../` check).  The new free helper `percent_r_pattern_flags` strips
the `%r`, reads the opening delimiter, finds the matching closing
delimiter (the last occurrence — v0 does not track nested brackets,
matching the other percent literals), and splits the body into
`(pattern, flags)` (flags validated as Ruby regex flag letters).  It
then reuses `lower_regex_literal`, so a `%r{...}` produces the SAME
`BuiltinCall("regex", [pattern, StrLit(flags)])` shape as `/.../` — and
gets the pattern interpolation splitter for free.  No new SIR variant or
backend dispatch.

(v0 lexer note: `%r` uses `{}` as the canonical delimiter and does not
slurp trailing flags; the helper is written generally — bracket pairs
`{}`/`[]`/`()`/`<>` and symmetric delimiters, plus trailing flags — so
it already covers the broader forms a future lexer pass may emit.)

New tests (+3): `percent_r_regex_lowers_to_regex_builtin` (`%r{hello}`),
`percent_r_regex_empty_pattern_lowers` (`%r{}`),
`percent_r_regex_validates_e2e` (`%r{x}` + validator E2E).  Test count:
255 → 258.

## [0.59.0] - 2026-05-31

### Added (Phase 19c (FC) — regex interpolation `/a#{b}c/`)

`lower_regex_literal` now runs the regex pattern through the SAME
`#{...}` interpolation splitter string literals use
(`lower_string_literal_with_interp`).  Since the lexer captures the
markers verbatim into the pattern, an interpolated regex's `args[0]`
becomes:

- a `string_concat` over literal + interpolated segments for
  `` /a#{b}c/ `` → `[StrLit("a"), VarRef("b"), StrLit("c")]`;
- a bare `VarRef` for a lone `` /#{b}/ ``;
- a plain `StrLit` when the pattern has no markers (the 19a/19b shape,
  unchanged).

`lower_regex_literal` became fallible (returns `Result`) to propagate
splitter errors.  No new SIR variant or backend dispatch — reuses
`string_concat` / `VarRef` / `StrLit`.  Still pure; requests
`Feature::Strings`.

New tests (+3): `regex_interpolation_lowers_pattern_to_concat`
(`/a#{b}c/`), `regex_interpolation_single_marker_is_bare_varref`
(`/#{b}/`), `regex_interpolation_validates_e2e` (`/x#{b}/i` + validator
E2E).  Test count: 252 → 255.

## [0.58.0] - 2026-05-31

### Added (Phase 19b (FC) — regex flags `/r/i` coverage confirmation)

No lowering change.  The `regex` builtin's `args[1]` already carries the
flag letters verbatim (Phase 19a), so 19b is a coverage-confirmation
phase (cf. 16b/16c) exercising MULTI-flag combinations the 19a tests
didn't (single `i` only).

New tests (+3): `regex_literal_multi_flag_preserves_all_flags`
(`/foo/im` → flags `"im"`, order preserved),
`regex_literal_all_common_flags_lower` (`/a/mix` → `"mix"`),
`regex_literal_multi_flag_validates_e2e` (`(/x/im)` + validator E2E).
Test count: 249 → 252.

## [0.57.0] - 2026-05-31

### Added (Phase 19a (FC) — regex literal `/pattern/flags`)

`lower_factor_atom` gained a regex dispatch: a `String` token whose
verbatim value has regex shape (recognised by the new free helper
`regex_pattern_flags`) lowers via `lower_regex_literal` to
`BuiltinCall("regex", [StrLit(pattern), StrLit(flags)])` (flags = `""`
when none).  Building a regex is pure; the literal requests
`Feature::Strings` (it emits `StrLit`s), matching the backtick/heredoc
stance.  No new SIR variant or backend dispatch — reuses the existing
`BuiltinCall` + `StrLit`.

`regex_pattern_flags` rejects path-shaped strings like `"/usr/bin"`
(lexed value `/usr/bin`) by requiring the trailing segment after the
final `/` to consist only of valid Ruby regex flag letters (`imxounes`)
— `b` is not a flag, so it stays a string.  (Residual v0 ambiguity: a
double-quoted string whose content has true regex shape, e.g. `"/a/i"`,
is read as a regex — the same lexeme-prefix limitation backticks and
heredocs already accept.  v0 does not unescape the body.)

New tests (+3): `regex_literal_lowers_to_regex_builtin` (`/foo/` → empty
flags), `regex_literal_with_flags_carries_flags` (`/foo/i`),
`regex_literal_validates_e2e` (`(/foo/)` + validator E2E).  Test count:
246 → 249.

## [0.56.0] - 2026-05-31

### Added (Phase 10d (FC) — beginless range `..5` / `...5`)

`lower_range` now handles beginless ranges (an end with no start).  A
beginless range has the SAME arity as an endless range (one operand +
one op token), so the lowerer disambiguates by the op token's position
relative to the operand:

- endless  `1..` (child order `[operand, op]`) → `[start, NilLit, excl]`
- beginless `..5` (child order `[op, operand]`) → `[NilLit, end, excl]`

The missing endpoint is encoded as `NilLit` either way, keeping the
`range` builtin's uniform shape.  No new SIR variant, `Feature`, or
backend dispatch.

New tests (+3): `beginless_range_inclusive_lowers_with_nil_start`
(`(..5)`), `beginless_range_exclusive_lowers_with_nil_start` (`(...5)`),
`beginless_range_over_param_validates_e2e` (`(..b)` + validator E2E).
Test count: 243 → 246.

## [0.55.0] - 2026-05-31

### Added (Phase 10c (FC) — endless range `1..` / `1...`)

`lower_range` gained a third case for endless ranges (one operand plus
a range op, no trailing operand).  The open upper bound is encoded as
`NilLit`, keeping the `range` builtin's uniform shape:

- `1..` → `BuiltinCall("range", [start, NilLit, BoolLit(false)])`
- `1...` → `BuiltinCall("range", [start, NilLit, BoolLit(true)])`

A nil end means "unbounded above"; the exclusive flag distinguishes
`..` from `...` exactly as in the two-operand case, so no new SIR
variant, `Feature`, or backend dispatch is required.

New tests (+3): `endless_range_inclusive_lowers_with_nil_end` (`(1..)`),
`endless_range_exclusive_lowers_with_nil_end` (`(1...)`),
`endless_range_over_param_validates_e2e` (`(a..)` + validator E2E).
Test count: 240 → 243.

## [0.54.0] - 2026-05-31

### Added (Phase 10a (FC) — inclusive range `1..5` coverage confirmation)

Inclusive ranges lower to `BuiltinCall("range", [start, end, BoolLit(false)])`
since **Phase 6n** (the third arg is the exclusive-end flag — `false`
for `..`, `true` for `...`).  No lowering change is required for Phase
10a — like Phases 16b/16c it is a coverage-confirmation phase pinning
inclusive ranges in positions the 6n tests skipped.

New tests (+3):

- `inclusive_range_in_assignment_rhs_lowers_with_false_flag` — `x = 1..5`
  at statement level (binding RHS; accepts `LetBinding`/`Assign`).
- `inclusive_range_string_endpoints_lower_with_false_flag` —
  `("a".."z")` lowers to a range over two `StrLit` endpoints.
- `inclusive_range_as_array_element_lowers_and_validates` — `[1..5]`
  lowers to a `SeqLit` whose element is the range builtin, and the
  module passes `semantic_ir::validate`.

Each pins the inclusive flag to `false`.  Test count: 237 → 240.

## [0.53.0] - 2026-05-30

### Added (Phase 16e (FC) — method-level rescue/ensure)

A `def` body carrying trailing `rescue`/`ensure` clauses (no explicit
`begin`) now lowers so the **whole method body** is wrapped in a single
`Stmt::TryCatch` (semantic-ir 0.9.0); the method's value becomes nil.

- Refactored the Phase 16a `begin` lowering: the body / rescue / ensure
  extraction is now shared via two helpers — `lower_flat_statements`
  (direct `statement` children → flat `Vec<Stmt>`) and
  `lower_rescue_ensure_clauses` (→ `(Vec<RescueClause>, Option<Vec<Stmt>>)`).
  `lower_begin_statement` and `lower_def_statement` both call them.
- `lower_def_statement` detects `rescue_clause` / `ensure_clause` children
  and, when present, wraps the method body in a `TryCatch` (requesting
  `Feature::Exceptions`); a plain `def` is unchanged (trailing expression
  still becomes the method value).

New tests (+4): rescue wraps body in TryCatch, ensure wraps body, plain
def unchanged (regression), method-level rescue validator E2E.  Test
count: 233 → 237.

## [0.52.0] - 2026-05-30

### Added (Phase 16d (FC) — `raise` / `raise Foo` / `raise Foo, "msg"`)

`raise` lowers to `BuiltinCall("raise", args)` tagged `MayThrow` +
`Divergent` (it is an expression-position construct, so it stays a
builtin rather than a `Stmt`).  Phase 16d completes and hardens this:

- **Bare `raise`** (re-raise the current exception) previously lowered
  to a plain `VarRef("raise", Local)` — losing the throw/divergent
  effects.  It now lowers to `BuiltinCall("raise", [])` in the factor
  path, unless `raise` is shadowed by a local binding.
- A `raise`-using module now requests `Feature::Exceptions` (both the
  bare-`raise` factor path and the `raise Foo` / `raise Foo, "msg"`
  method-call path), aligning the manifest with begin/rescue (Phase 16a).
- `raise Foo` / `raise Foo, "msg"` already lowered to
  `BuiltinCall("raise", [Foo, …])` via the method-call path (unchanged).

New tests (+4): bare-raise → builtin with MayThrow+Divergent+Exceptions;
`raise Foo` → one Const arg; `raise Foo, "msg"` → class + message args;
validator E2E.  Test count: 229 → 233 (+4).

## [0.51.0] - 2026-05-30

### Added (Phase 16c (FC) — `ensure` clause coverage)

Hardening of the Phase 16a `Stmt::TryCatch.ensure_body` lowering — no
code change.  Phase 16c locks the ensure-clause behaviour in with tests
that 16a didn't cover:

- `ensure_only_lowers_with_no_rescues` — `begin … ensure … end` (no
  rescue) lowers to a `TryCatch` with empty `rescues` and a populated
  `ensure_body`, requesting `Feature::Exceptions`.
- `ensure_body_preserves_statement_order` — a multi-statement ensure
  body keeps its statements in source order.
- `ensure_only_passes_sir_validator` (E2E) — an ensure-only begin
  validates end-to-end (no rescue path).

Test count: 226 → 229 (+3).

## [0.50.0] - 2026-05-30

### Added (Phase 16b (FC) — typed / multi-type / multi-clause rescue)

Hardening of the Phase 16a `Stmt::TryCatch` lowering — no code change,
the 16a `RescueClause` plumbing already handles these forms.  Phase 16b
locks the behaviour in with dedicated tests:

- `rescue_multi_type_lowers_all_exception_types` — `rescue Foo, Bar => e`
  lowers to one `RescueClause` whose `exception_types` lists both classes
  in source order, with binding `e`.
- `multiple_rescue_clauses_lower_to_separate_clauses` — two `rescue`
  clauses lower to two `RescueClause`s, each with its own exception type
  and binding, in source order.
- `multi_clause_rescue_passes_sir_validator` (E2E) — each clause's
  binding resolves inside its own body, confirming per-clause scope.

Test count: 223 → 226 (+3).

## [0.49.0] - 2026-05-30

### Changed (Phase 16a (FC) — `begin/rescue/ensure/end` → `Stmt::TryCatch`)

`begin/rescue/ensure/end` now lowers to the first-class
`Stmt::TryCatch` (semantic-ir 0.9.0) instead of the Phase 6v inline
`__rescue_marker__` / `__ensure_marker__` placeholder builtins:

- `lower_begin_statement` builds a single `Stmt::TryCatch { body,
  rescues, ensure_body }`.  The try body and each clause body are lowered
  with the existing `lower_statement_inner_multi` collector.
- Each `rescue_clause` becomes a `RescueClause` carrying its exception
  class names (`Vec<String>`), the optional `=> e` binding, and its body.
  A bare `rescue` yields an empty `exception_types`.
- The optional `ensure_clause` becomes `ensure_body: Some(..)`.
- Emitting a `TryCatch` requests `Feature::Exceptions` (no longer the
  ad-hoc `Effect::MayThrow`-tagged marker builtins).
- No grammar change — `begin_statement` already parsed (Phase 6v).

The four pre-existing Phase 6v tests were rewritten from
marker-assertions to the `TryCatch` contract
(`begin_without_rescue_lowers_body_inline`,
`begin_with_rescue_lowers_to_rescue_clause`,
`begin_with_ensure_lowers_to_ensure_body`,
`begin_with_rescue_and_ensure_lowers_to_full_trycatch`), plus a new
validator E2E (`begin_rescue_passes_sir_validator`).  Test count:
222 → 223 (+1).

## [0.48.0] - 2026-05-30

### Added (Phase 15d (FC) — scoped lookup `Foo::Bar`)

Ruby's scope-resolution operator now lowers.  A scoped constant lookup
is, semantically, a single constant resolved against a namespace, so it
reuses the Phase 15c `Scope::Const` machinery (no new SIR node):

- `apply_dot_chain` now also folds `scope_resolution` postfix steps.
- New `fold_one_scope_resolution`: `Foo::Bar` folds into a single
  `VarRef { scope: Const, name: "Foo::Bar" }`, and `A::B::C` collapses
  to `VarRef { Const, "A::B::C" }`.  Each step requests
  `Feature::Constants`.
- A non-constant base (`expr::Bar`, uncommon) is preserved structurally
  via a `BuiltinCall("__scope__", [base, StrLit(name)])` marker so no
  structure is silently dropped.

New tests (+3): `scope_resolution_lowers_to_qualified_const`,
`scope_resolution_chain_lowers_to_full_path`,
`scope_resolution_passes_sir_validator` (E2E).  Test count: 219 → 222
(+3).

## [0.47.0] - 2026-05-30

### Changed (Phase 15c (FC) — constants `FOO` / `MyClass`)

Constants now lower to the first-class `Scope::Const` (semantic-ir
0.8.0) instead of the Phase 6x `Scope::Local` placeholder, mirroring
Phases 15a/15b (`@x`, `@@x`):

- A constant **read** (any bare uppercase-initial name) lowers to
  `Expr::VarRef { scope: Const }` — and no longer errors as an
  undefined local when read before any assignment.
- A constant **assignment** (`FOO = …`, including the compound forms)
  lowers to `Stmt::Assign { scope: Const }` — never a `LetBinding` — and
  is not registered in `declared_locals`.  Emitting it requests
  `Feature::Constants` (and `MutableBindings`, since the store is a
  `Stmt::Assign`).
- New `is_constant_name` helper: a name whose first character is an
  uppercase ASCII letter is a constant.  Class/module *names* in
  `class Foo` / `module M` are consumed by their own grammar productions
  and never reach this path, so only constants used as values or
  assignment targets are routed here.

New tests (+4): `const_read_lowers_to_const_scope`,
`const_assignment_lowers_to_const_assign_not_letbinding`,
`const_read_without_assignment_passes_validator` (E2E),
`lowercase_name_stays_local_not_const` (regression).  Five pre-existing
class/module/singleton-body tests that asserted constant assignments as
`LetBinding` were updated to the new `Assign { scope: Const }` contract
(`class_body_preserves_multiple_statements_in_source_order`,
`class_body_preserves_executable_statement_and_hoists_method`,
`module_body_preserves_executable_statement`,
`singleton_class_hoists_methods_and_keeps_statements`,
`subclass_with_body_records_superclass_and_hoists_methods`).  Test
count: 215 → 219 (+4).

## [0.46.0] - 2026-05-30

### Changed (Phase 15b (FC) — class variables `@@x`)

Class variables now lower to the first-class `Scope::ClassVar`
(semantic-ir 0.7.0) instead of the Phase 6x `Scope::Local` placeholder,
mirroring Phase 15a's treatment of `@x`:

- A `@@x` **read** lowers to `Expr::VarRef { scope: ClassVar }` — and,
  crucially, no longer errors as an undefined local when read before any
  assignment (reading an unset `@@x` is nil in Ruby).
- A `@@x` **assignment** (`@@x = …`, `@@x += …`) lowers to
  `Stmt::Assign { scope: ClassVar }` — never a `LetBinding` — and does
  not register `@@x` as a local.  Emitting it requests
  `Feature::ClassVars` (and `MutableBindings`, since the store is a
  `Stmt::Assign`).
- New `is_class_var_name` helper (`starts_with("@@")`).  Because `@@x`
  also begins with `@`, the read/assign paths test for class var
  **before** instance var, so `@@x` → ClassVar and `@x` → Instance stay
  distinct.

New tests (+4): `class_var_read_lowers_to_classvar_scope`,
`class_var_read_without_assignment_passes_validator` (E2E),
`class_var_in_method_roundtrips_through_validator` (E2E),
`instance_and_class_vars_are_distinct_scopes` (regression guard).  Two
pre-existing tests were updated to the new ClassVar contract:
`class_var_double_at_is_not_instance_scope` (now asserts
`Scope::ClassVar` + `ClassVars`/not-`InstanceVars`) and the Phase 6x
`class_var_ref_lowers_with_local_scope_and_double_at_preserved` (now
asserts `Scope::ClassVar`).  Test count: 211 → 215 (+4).

## [0.45.0] - 2026-05-30

### Changed (Phase 15a (FC) — instance variables `@x`)

Instance variables now lower to the first-class `Scope::Instance`
(semantic-ir 0.6.0) instead of the Phase 6x `Scope::Local` placeholder:

- A `@x` **read** lowers to `Expr::VarRef { scope: Instance }` — and,
  crucially, no longer errors as an undefined local when read before any
  assignment (reading an unset `@x` is nil in Ruby).
- A `@x` **assignment** (`@x = …`, `@x += …`) lowers to
  `Stmt::Assign { scope: Instance }` — never a `LetBinding` — and does
  not register `@x` as a local.  Emitting it requests
  `Feature::InstanceVars` (and `MutableBindings`, since the store is a
  `Stmt::Assign`).
- New `is_instance_var_name` helper: a single-`@` sigil name is an
  instance var; `@@x` (class var, Phase 15b) and `$x` (global) keep
  their pre-15a handling.

New tests (+5): `instance_var_read_lowers_to_instance_scope`,
`instance_var_assignment_lowers_to_instance_assign`,
`instance_var_read_without_assignment_passes_validator` (E2E),
`instance_var_in_method_roundtrips_through_validator` (E2E),
`class_var_double_at_is_not_instance_scope` (regression guard).  The
pre-existing Phase 6x test
`instance_var_ref_lowers_with_local_scope_and_sigil_preserved` was
updated to assert the new `Scope::Instance` contract.  Test count:
206 → 211 (+5).

## [0.44.0] - 2026-05-30

### Added (Phase 14e (FC) — singleton class `class << self … end`)

The singleton-class form `class << RECEIVER … end` now lowers to
`Stmt::SingletonClassDef { target, body, span }` (semantic-ir 0.5.0).
`target` is the receiver (`"self"` or a bare name).

- The `class_statement` lowering arm dispatches on the presence of a
  `singleton_receiver` child node: present → `SingletonClassDef`,
  absent → the ordinary `ClassDef` path (Phase 14b/14c).
- New `extract_singleton_receiver` helper returns the receiver token's
  value (or `None` for the ordinary class form).
- Body handling reuses `lower_decl_body_statements` (shared with
  class/module): method `def`s hoist to top-level `Function`s; non-`def`
  statements are preserved in `body`.  Requests `Feature::Classes`.

New tests (+5): `singleton_class_of_self_lowers_to_singleton_class_def`,
`singleton_class_requests_classes_feature`,
`singleton_class_hoists_methods_and_keeps_statements`,
`singleton_class_passes_sir_validator` (E2E lower → validate),
`ordinary_class_still_lowers_to_class_def_not_singleton` (regression
guard).  Test count: 201 → 206 (+5).

## [0.43.0] - 2026-05-30

### Changed (Phase 14d (FC) — `module M … end` → `Stmt::ModuleDef`)

`module M … end` now lowers to a first-class
`Stmt::ModuleDef { name, body, span }` (semantic-ir 0.4.0), replacing
the pre-14d behaviour where a module lowered to a no-op
`ExprStmt(NilLit)` (with its `def`s hoisted as a side effect).

- New `extract_module_name` helper (symmetric with
  `extract_class_name`, module-specific error message).
- Emitting a `ModuleDef` requests `Feature::Modules`, now materialised
  into the module manifest.
- Module body handling is **identical to a class** and shares the
  helper, renamed `lower_class_body_statements` →
  `lower_decl_body_statements`: method `def`s hoist to top-level
  `Function`s; non-`def` statements are preserved in `body` in source
  order.
- Retired the Phase 6f `collect_def_statements_from_body` whole-body
  pre-pass (now dead — both the class and module arms hoist per-direct
  child via `lower_decl_body_statements`, and nested declarations
  hoist their own direct `def`s through the normal dispatch).

The pre-14d `module_still_lowers_to_nil_no_op_in_phase_14a` test is
replaced by the new ModuleDef contract tests. New/updated tests:
`empty_module_lowers_to_module_def_stmt`,
`empty_module_requests_modules_feature`,
`empty_module_passes_sir_validator`,
`module_with_def_hoists_def_to_top_level` (now also asserts ModuleDef),
`module_body_preserves_executable_statement`.  Test count: 198 → 201.

## [0.42.0] - 2026-05-30

### Added (Phase 14c (FC) — inheritance `class Foo < Bar`)

`class Foo < Bar` now lowers to `Stmt::ClassDef` with
`superclass: Some("Bar")` (semantic-ir 0.3.0's new field); a base class
`class Foo` keeps `superclass: None`.

- New `extract_superclass` helper scans the `class_statement` node's
  *direct* child tokens for the `<` separator (a `Name`-type token with
  value `"<"`) and returns the value of the next `Name` token — the
  superclass.  Only direct tokens are inspected, so a `<` comparison
  *inside* a body statement (`a < b`) is never mistaken for the
  superclass separator (body statements are `statement` nodes, not bare
  tokens).
- Inheritance composes with Phase 14b: a subclass body still hoists its
  `def`s to top-level Functions and preserves non-def statements in
  `ClassDef.body`.

New tests (+5): `class_with_superclass_records_parent_name`,
`base_class_has_no_superclass`,
`subclass_with_body_records_superclass_and_hoists_methods`,
`subclass_passes_sir_validator` (E2E lower → validate),
`comparison_in_class_body_is_not_mistaken_for_superclass`.
Test count: 193 → 198 (+5).

## [0.41.0] - 2026-05-30

### Changed (Phase 14b (FC) — class body with method defs + statements)

`class Foo … end` now lowers to `Stmt::ClassDef` with a **populated**
`body` (Phase 14a always emitted `body: vec![]`).  The class body's
*executable* statements — constant/expression assignments, bare
expressions, nested `class`/`module` declarations, loops, … — are
lowered in source order and preserved in `ClassDef.body`, instead of
being silently dropped.

- New `lower_class_body_statements` helper walks the class body's
  `statement` children once:
  - `def_statement` / `endless_def_statement` are **hoisted** to
    top-level `Function`s (unchanged — SIR v0 has no
    method-as-statement node, so a method can't live inside a
    `Vec<Stmt>`), contributing nothing to `body`.
  - every other statement is lowered via the shared
    `lower_statement_inner_multi` dispatch and pushed onto `body`.
- The `class_statement` arm no longer calls the recursive
  whole-body `collect_def_statements_from_body` pre-pass; hoisting is
  now per-direct-child.  A nested `class`/`module` is lowered via the
  normal dispatch (whose own arm hoists *its* direct `def`s), so every
  method is hoisted **exactly once** — no double-registration that
  would trip the validator's function-name-uniqueness check.
- A method-*only* class still produces an empty `body` (the methods
  hoist); the `module_statement` arm is unchanged (still a NilLit
  no-op + def hoist, pending Phase 14d's `ModuleDef`).

New tests (4): `class_body_preserves_executable_statement_and_hoists_method`,
`class_body_preserves_multiple_statements_in_source_order`,
`class_with_body_statements_passes_sir_validator` (E2E lower → validate),
`nested_class_methods_hoisted_exactly_once`.  The existing
`class_with_method_body_still_emits_class_def_and_hoists_method` is
retained (method-only → empty body) with an updated comment.

Test count: 189 → 193 (+4).

## [0.40.0] - 2026-05-29

### Added (Phase 14a (FC) — empty `class Foo; end`)

`class Foo; end` now lowers to a first-class
`Stmt::ClassDef { name: "Foo", body: vec![], span }` (semantic-ir
0.2.0's new SIR17 node), replacing the pre-14a behaviour where a
class declaration lowered to a no-op `ExprStmt(NilLit)`.

- New `extract_class_name` helper pulls the class name from the
  first `TokenType::Name` token of a `class_statement` node (the
  `class` keyword is `TokenType::Keyword`, so it is skipped).
- Emitting a `ClassDef` requests `Feature::Classes`, which is now
  materialised into the module manifest alongside the existing
  feature tally.
- **Empty-body only:** Phase 14a always lowers `body: vec![]`.  The
  pre-existing Phase 6f method-hoisting fallback is preserved — a
  non-empty class body still hoists its `def`s to top-level
  `Function`s, leaving the `ClassDef` body empty — so older fixtures
  with method bodies continue to validate.  Phase 14b will populate
  `body` directly and retire the hoist-as-fallback path.
- `module M; end` is unchanged: it continues to lower to the Phase
  6f `NilLit` no-op until a later phase introduces a module node.

### Tests

- `ruby-to-semantic-ir`: 184 → 189 (+5): empty-class → ClassDef,
  `Feature::Classes` request, validator E2E, verbatim-name
  preservation, class-with-method-body (ClassDef + hoist), and a
  pin that `module` still lowers to NilLit.

## [0.39.0] - 2026-05-28

### Added (Phase 9c (FC) — single-RHS tuple destructure)

`multi_assignment` now accepts the single-RHS shape that Phase 9b's
comments still flagged as deferred:

```ruby
a, b    = arr           # a == arr[0]; b == arr[1]
a, b, c = arr           # a == arr[0]; b == arr[1]; c == arr[2]
a, b    = make_pair()   # make_pair() evaluated once into a temp
```

The lowerer routes 1-RHS / ≥2-LHS / no-splat through a new helper
`lower_multi_assignment_single_rhs_destructure`.  The strategy:

1. Bind the single (already-lowered) RHS to a fresh
   `LetStarBinding(__multi_assign_t<N>_seq, rhs)` — `LetStarBinding`
   keeps the temp visible to the LHS-binding pass and side effects
   in the RHS fire exactly once.
2. For each LHS position `i`, emit
   `Stmt::LetBinding`/`Stmt::Assign` reading
   `Expr::SeqIndex { seq: VarRef(temp), index: IntLit(i) }`.

| Source                | SIR shape (Phase 9c)                                                                                              |
|-----------------------|-------------------------------------------------------------------------------------------------------------------|
| `a, b = arr`          | `LetStarBinding(t0_seq, arr); LetBinding(a, SeqIndex(t0_seq, 0)); LetBinding(b, SeqIndex(t0_seq, 1))`             |
| `a, b, c = arr`       | `LetStarBinding(t0_seq, arr); LetBinding(a, SeqIndex(t0_seq, 0)); LetBinding(b, SeqIndex(t0_seq, 1)); LetBinding(c, SeqIndex(t0_seq, 2))` |
| `a = 0; a, b = arr`   | re-bind path: stmt for `a` is `Assign`, requests `Feature::MutableBindings`                                       |

Out-of-bounds semantics are target-language-defined per
`Expr::SeqIndex`'s docs.  Ruby itself fills missing positions with
`nil`; matching that exactly is left to the backend or a future
phase.

### Arity check (Phase 9c)

The no-splat arity check relaxes from "LHS == RHS strict" to
"LHS == RHS *or* exactly 1 RHS with ≥2 LHS".  All other shapes still
error.  The splat path is unchanged (`a, *b = arr` still uses the
Phase 9b splat lowering and treats the single RHS as one of the
absorbable values — single-RHS-with-splat auto-unpack remains a
future phase).

### Tests

- `ruby-to-semantic-ir`: 177 → **184** (+7)
- `coding-adventures-ruby-parser`: 152 → **155** (+3 — grammar
  coverage tests for `a, b = arr` shape).

## [0.38.0] - 2026-05-28

### Added (Phase 9b (FC) — splat target in multi-assignment LHS)

`multi_assignment` now accepts an optional `*` prefix on each LHS
target via the new `mlhs_target` rule.  At most one splat per LHS is
allowed; the splat absorbs zero or more "extra" RHS values into an
`Expr::SeqLit` while non-splat targets bind to fixed-position RHS
values (counted from the start, or from the end if a splat sits to
the left).

| Source                      | SIR shape (after Phase 9b)                                                                                                                                                                                  |
|-----------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `a, *b = 1, 2, 3`           | `LetStarBinding(t0,1); LetStarBinding(t1,2); LetStarBinding(t2,3); LetBinding(a, VarRef(t0)); LetBinding(b, SeqLit([VarRef(t1), VarRef(t2)]))`                                                              |
| `*a, b = 1, 2, 3`           | `LetStarBinding(t0,1); LetStarBinding(t1,2); LetStarBinding(t2,3); LetBinding(a, SeqLit([VarRef(t0), VarRef(t1)])); LetBinding(b, VarRef(t2))`                                                              |
| `a, *b, c = 1, 2, 3, 4`     | 4 temps + `LetBinding(a, t0); LetBinding(b, SeqLit([t1, t2])); LetBinding(c, t3)`                                                                                                                            |
| `a, *b = 1`                 | 1 temp + `LetBinding(a, t0); LetBinding(b, SeqLit([]))` *(empty splat)*                                                                                                                                     |

The splat path always routes through the swap-safe temp pass (Phase
9a pattern) — every RHS value lands in a fresh
`LetStarBinding(__multi_assign_t<N>_<i>, rhs[i])` first, so the
splat's `SeqLit` and the surrounding non-splat bindings all read
captured values.  `Feature::Sequences` is required.

Arity check:

- No splat → LHS count must equal RHS count (Phase 6r semantics).
- Splat present → RHS count must be `≥ non_splat_count`.  Otherwise
  the lowerer rejects with a clear error.

### Tests

- `ruby-to-semantic-ir`: 170 → **177** (+7):
  - `splat_lhs_at_end_absorbs_trailing_rhs_into_seqlit`
  - `splat_lhs_at_start_absorbs_leading_rhs_into_seqlit`
  - `splat_lhs_in_middle_absorbs_middle_rhs_into_seqlit`
  - `splat_lhs_with_minimum_rhs_count_gives_empty_seqlit`
  - `splat_lhs_requests_sequences_feature`
  - `splat_lhs_module_passes_sir_validator` (E2E for all three splat positions)
  - `splat_lhs_too_few_rhs_is_a_lower_error`

## [0.37.0] - 2026-05-28

### Changed (Phase 9a (FC) — swap-safe parallel multi-assignment)

Phase 6r lowered `a, b = rhs0, rhs1` as a flat sequence of one SIR
statement per pair: `Stmt(a := rhs0); Stmt(b := rhs1)`.  That's
observably correct only when no LHS name appears in any RHS — the
common case.  For the swap `a, b = b, a`, the sequential form reads
the *post-assignment* value of `a` when evaluating the second pair,
producing `a = old_b; b = old_b` instead of the true swap.

Phase 9a introduces a "needs-temps" heuristic.  After lowering every
RHS to an `Expr`, the lowerer scans each one (structural recursion
over the SIR `Expr` tree) for any `VarRef` whose name appears in the
LHS list.  If found:

1. Each RHS value is bound to a fresh `LetStarBinding` temp named
   `__multi_assign_t<N>_<i>` (counter `multi_assign_counter` ensures
   uniqueness across multiple multi-assignments in the same scope).
2. Each LHS is then assigned from its temp via the usual
   first-sighting `LetBinding` / re-binding `Assign` decision.

`LetStarBinding` (sequential semantics) is used for the temps so each
temp's name is visible to the subsequent LHS-binding pass — `LetBinding`
would put them in the same parallel-let validator group and hide them.

If no LHS appears in any RHS, the lowerer keeps Phase 6r's sequential
shape (no temps, no `LetStarBinding`) so the simple case stays cheap.

| Source                  | SIR shape (after Phase 9a)                                                                                                         |
|-------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `a, b = 1, 2`           | `LetBinding(a, 1); LetBinding(b, 2)` *(no temps — fast path)*                                                                      |
| `a = 1; b = 2; a, b = b, a` | `LetBinding(a, 1); LetBinding(b, 2); LetStarBinding(__multi_assign_t0_0, VarRef(b)); LetStarBinding(__multi_assign_t0_1, VarRef(a)); Assign(a, VarRef(__multi_assign_t0_0)); Assign(b, VarRef(__multi_assign_t0_1))` |

### Tests

- `ruby-to-semantic-ir`: 165 → **170** (+5):
  - `multi_assignment_swap_introduces_temps_to_preserve_parallel_semantics`
  - `multi_assignment_simple_case_keeps_fast_path_with_no_temps`
  - `multi_assignment_partial_dependency_still_uses_temps_for_all_positions`
  - `multi_assignment_swap_module_passes_sir_validator` (validator E2E)
  - `multi_assignment_two_swaps_use_distinct_temp_counters`

## [0.36.0] - 2026-05-28

### Changed (Phase 8b (FC) — short-circuit `||=` / `&&=` lowering)

Phase 6p originally lowered `x ||= y` and `x &&= y` eagerly to
`Assign(x, BuiltinCall("or"/"and", [VarRef(x), y]))`.  That form
ALWAYS evaluates `y` and ALWAYS re-binds `x`, which silently breaks
Ruby's documented short-circuit semantics whenever `y` has side
effects.  Phase 8b replaces it with a gated `Expr::If` so the RHS and
the re-bind are skipped when the short-circuit branch fires.

| Source     | SIR shape                                                          |
|------------|--------------------------------------------------------------------|
| `x ||= y`  | `ExprStmt(If(VarRef(x), Block{[], VarRef(x)}, Block{[Assign(x,y)], VarRef(x)}))` |
| `x &&= y`  | `ExprStmt(If(VarRef(x), Block{[Assign(x,y)], VarRef(x)}, Block{[], VarRef(x)}))` |

`Feature::MutableBindings` is still required (the gated branch
re-binds `x`), and `x` is recorded as a declared local so any
subsequent `x = …` doesn't trip the rebinding-into-undeclared-name
error.  All other compound-assign forms (`+=`, `-=`, `*=`, `/=`, `%=`,
`**=`, `<<=`, `>>=`, `&=`, `|=`, `^=`) keep their eager
`Assign + BuiltinCall` lowering — they have no short-circuit
semantics, so the previous shape is correct for them.

### Tests

- `ruby-to-semantic-ir`: 162 → **165** (+3 net):
  - Replaced `logical_compound_assigns_lower_to_or_and_builtins`
    (asserted the old eager shape) with four new tests:
    - `or_assign_lowers_to_short_circuit_if_with_assign_in_else_branch`
    - `and_assign_lowers_to_short_circuit_if_with_assign_in_then_branch`
    - `short_circuit_op_assign_marks_mutable_bindings_feature`
    - `short_circuit_op_assign_module_passes_sir_validator` (validator E2E for both ops)

## [0.35.0] - 2026-05-26

### Added (Phase 8a-2 (FC) — `>>=` right-shift compound-assign lowering)

`lower_assignment` gains one more case arm — `">>="` maps to `BuiltinCall(">>", ...)` with the same `Stmt::Assign` + `Feature::MutableBindings` shape as the rest of the compound-assign family.

| Source     | SIR shape                                                       |
|------------|-----------------------------------------------------------------|
| `x >>= y`  | `Assign(x, BuiltinCall(">>", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |

Combined with Phase 8a, Ruby's complete compound-assignment family on local variables is now fully lowered to first-class SIR.

### Tests

- `ruby-to-semantic-ir`: 160 → **162** (+2):
  - `right_shift_assign_desugars_to_assign_with_rshift_builtin`
  - `right_shift_assign_module_passes_sir_validator` (E2E smoke)

## [0.34.0] - 2026-05-26

### Added (Phase 8a (FC) — additional compound-assignment lowering)

`lower_assignment` learns six more compound forms — `%=`, `**=`, `<<=`, `&=`, `|=`, `^=` — and desugars each identically to `x = x op rhs`:

| Source     | SIR shape                                                       |
|------------|-----------------------------------------------------------------|
| `x %= y`   | `Assign(x, BuiltinCall("%",  [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x **= y`  | `Assign(x, BuiltinCall("**", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x <<= y`  | `Assign(x, BuiltinCall("<<", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x &= y`   | `Assign(x, BuiltinCall("&",  [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x \|= y`  | `Assign(x, BuiltinCall("\|", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x ^= y`   | `Assign(x, BuiltinCall("^",  [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |

Same convention as the pre-existing `+= -= *= /=` family: BuiltinCall name matches the underlying surface operator literally, so downstream emitters that target Ruby can pass the name through unchanged.

### Tests

- `ruby-to-semantic-ir`: 155 → **160** (+5):
  - `modulo_assign_desugars_to_assign_with_modulo_builtin`
  - `power_assign_desugars_to_assign_with_power_builtin`
  - `left_shift_assign_desugars_to_assign_with_lshift_builtin`
  - `bitwise_op_assigns_lower_to_assign_with_bitwise_builtins`
  - `compound_assigns_module_passes_sir_validator` (E2E smoke)

## [0.33.0] - 2026-05-26

### Added (Phase 7f — Ruby 3.1 hash value-omitted shorthand lowering)

`lower_hash_entry` learns a third dispatch arm: when a `hash_entry` node has a `NAME` token, a `COLON` token, and ZERO `expression` children, the entry's value is emitted as `VarRef(name, scope)` — a same-named local variable lookup.  Key remains `SymLit(name)` (matching the existing keyword-style shorthand).

The `scope` follows the same Param-vs-Local dispatch as bare-name factor lowering: if the binding exists in `current_params`, mark it `Param`; otherwise mark it `Local`.  This means `{x:}` inside `def f(x); …; end` correctly emits `VarRef("x", Param)`.

### Lowering dispatch summary

| Source            | Shape                                                                        |
|-------------------|------------------------------------------------------------------------------|
| `{x: 1}`          | `MapEntry { key: SymLit("x"), value: IntLit(1) }` (unchanged)               |
| `{x => 1}`        | `MapEntry { key: <lowered x>, value: IntLit(1) }` (unchanged)               |
| **`{x:}`**        | **`MapEntry { key: SymLit("x"), value: VarRef("x", Local/Param) }` (new)** |

The change is purely additive — no existing SIR shape changes.  Both `Feature::Symbols` (for the key) and (transitively) any feature for the value expression are still recorded as before.

### Tests

- `ruby-to-semantic-ir`: 150 → **155** (+5):
  - `hash_value_shorthand_emits_var_ref_value` — `{name:}` value is `VarRef("name", Local)`.
  - `hash_value_shorthand_inside_method_uses_param_scope` — `def f(x); {x:}; end` value is `VarRef("x", Param)`.
  - `hash_value_shorthand_mixed_with_explicit_form` — `{name:, age: 30}` first entry is VarRef, second is IntLit.
  - `hash_explicit_form_unchanged_after_phase_7f` — `{x: 1, y: 2}` regression (still IntLit values).
  - `hash_value_shorthand_module_passes_sir_validator` — end-to-end validator smoke.

## [0.32.0] - 2026-05-25

### Added (Phase 7e — Ruby 3.0 rightward assignment lowering)

A new helper `lower_rightward_assignment` mirrors the `lower_assignment` LetBinding-on-first-sight / Assign-on-rebind dispatch.  Rightward assignment is purely syntactic — `expr => var` and `var = expr` produce identical SIR.

### Lowering

| Source              | SIR shape                                                |
|---------------------|----------------------------------------------------------|
| `1 + 2 => sum`      | `LetBinding(sum, BuiltinCall("+", [IntLit 1, IntLit 2]))` |
| `42 => x`           | `LetBinding(x, IntLit 42)`                               |
| `[1, 2] => arr`     | `LetBinding(arr, SeqLit([IntLit 1, IntLit 2]))`          |
| (re-bind) `5 => x`  | `Assign(x, IntLit 5)` + `Feature::MutableBindings`       |

`lower_statement_inner` dispatches `rightward_assignment` to the new helper alongside `assignment`.

### Tests

- `ruby-to-semantic-ir`: 146 → **150** (+4):
  - `rightward_assignment_lowers_to_let_binding_on_first_sight` — `1 + 2 => sum`.
  - `rightward_assignment_with_literal_lowers_to_int_let_binding` — `42 => x`.
  - `rightward_assignment_rebind_emits_assign_with_mutable_bindings_feature` — `Assign` + manifest gating.
  - `rightward_assignment_module_passes_sir_validator` — end-to-end smoke.

## [0.31.0] - 2026-05-25

### Added (Phase 7d — Ruby 3.0 case/in pattern matching lowering)

`lower_case_statement` now collects both `when_clause` and `in_clause` subnodes in source order.  Two new helpers:

- `lower_when_clause_condition` — refactored out of the original Phase 6u lowerer for symmetry with `in_clause` dispatch (no behaviour change).
- `lower_in_clause_pattern` — dispatches on pattern kind, returning `(cond, prefix_stmts)`.  Binding-pattern body-prefix stmts are prepended to the clause body so the bound local is visible from the first statement.

### Pattern lowering

| Pattern        | cond                                            | body-prefix stmts        |
|----------------|-------------------------------------------------|--------------------------|
| `in 1`         | `BuiltinCall("==", [scrut, IntLit(1)])`         | `[]`                     |
| `in "s"`       | `BuiltinCall("==", [scrut, StrLit("s")])`       | `[]`                     |
| `in :foo`      | `BuiltinCall("==", [scrut, SymLit("foo")])`     | `[]`                     |
| `in nil`       | `BuiltinCall("==", [scrut, NilLit])`            | `[]`                     |
| `in y`         | `BoolLit(true)`                                 | `[LetBinding(y, scrut)]` |
| `in [1, 2]`    | `BuiltinCall("__pattern_match__", [scrut, StrLit(raw)])` | `[]`            |
| `in {name: y}` | `BuiltinCall("__pattern_match__", [scrut, StrLit(raw)])` | `[]`            |

The `__pattern_match__` marker carries the verbatim pattern text (joined Token values via depth-first walk) so downstream emitters can re-derive the structural matching at codegen time.  Same marker-builtin pattern as Phase 6v rescue/ensure, Phase 6y `__interp__`, Phase 7a `backtick`.

The synthetic `StrLit` triggers `Feature::Strings`.

A new helper `lower_pattern_literal` mirrors the factor-atom Token dispatch but narrowed to the patterns the `literal_pattern` rule admits (NUMBER, STRING, KEYWORD/`nil`/`true`/`false`, symbol_literal).  It reuses Phase 6z's `lower_numeric_literal` so every numeric shape (float/hex/bin/oct/dec) parses identically inside a pattern.

### v0 deferred limitations

- Array / hash patterns are kept as the `__pattern_match__` marker — no structural decomposition, no sub-bindings emitted.  A follow-up phase will walk the inner patterns and emit element comparisons + sub-`LetBinding`s.
- Hash pattern shorthand `{name:}` doesn't bind `name` at SIR level.
- Pin operators (`^x`), find patterns (`[…, *, …]`), and class patterns (`SomeClass(x)`) are not yet parsed.
- Inside an `in` body, a bare-name statement (`in y; y; end`) hits a pre-existing grammar quirk where `method_call_no_paren` greedily consumes the closing `end` keyword as an argument.  Workaround in tests: use `puts(y)` rather than bare `y`.

### Tests

- `ruby-to-semantic-ir`: 141 → **146** (+5):
  - `case_in_literal_pattern_lowers_to_equality_check` — `in 1`.
  - `case_in_binding_pattern_emits_letbinding_prefix` — `in y`.
  - `case_in_array_pattern_lowers_to_pattern_match_marker` — `in [1, 2]`.
  - `case_in_hash_pattern_lowers_to_pattern_match_marker` — `in {name: y}`.
  - `case_in_with_else_clause_emits_else_branch` — `else` fallback.

## [0.30.0] - 2026-05-25

### Added (Phase 7c — Ruby 3.0 endless method definitions)

A new helper `lower_endless_def_statement` lowers `def foo = expr` / `def foo(x, y) = expr` into a top-level `Function`.  The shape:

```
Function {
    name,
    params,
    body: Block { stmts: [], value: <lowered expression> },
    return_type: None,
    captures: [],
    effects: PURE,
    metadata: Metadata::new(),
}
```

The `lower_statement_inner` `def_statement` match now also matches `endless_def_statement` (both hoist to a top-level Function and emit a `NilLit` ExprStmt placeholder in the main body).  Both pre-passes — `collect_def_statements` (program-level) and `collect_def_statements_from_body` (class/module-level) — dispatch on rule name so either form gets hoisted.

### Lowering details

- Parameter extraction reuses the same `params` → `param` Node walk as `lower_def_statement`, with the same lossy-splat v0 limitation: `*args` / `**kw` params drop the splat prefix.
- A fresh `declared_locals` / `current_params` scope is opened for the body expression so a parameter reference inside the body resolves to `Scope::Param` (validator-correct).
- The body is the single `expression` Node child (PEG guarantees exactly one, after the EQUALS token).  `Block.stmts` is empty; `Block.value` is the lowered expression.
- Function `effects` default to `PURE`; if the lowered expression contains effectful calls (e.g. `puts`), the SIR's `effects_of` inference will pick them up at validation time.

### v0 deferred limitations

- Lossy splat (inherited from Phase 6s): `def foo(*args) = ...` loses the `*` prefix.
- Endless defs inside classes / modules are hoisted to top level (no class scoping in SIR v0 — same caveat as the block-bodied def).
- No method visibility markers (`private`, `protected`) — same as block-bodied defs.

### Tests

- `ruby-to-semantic-ir`: 137 → **141** (+4):
  - `endless_def_no_params_hoists_to_top_level_function` — happy path.
  - `endless_def_with_params_carries_param_scope` — asserts `Scope::Param` for body VarRefs.
  - `endless_def_does_not_emit_main_body_stmt` — confirms hoisting + NilLit placeholder.
  - `endless_def_module_passes_sir_validator` — end-to-end validator smoke.

## [0.29.0] - 2026-05-25

### Added (Phase 7b — heredoc literal lowering)

`lower_factor_atom`'s `String` case now dispatches in lexeme-prefix priority order:
- starts with `` ` `` → backtick command literal (Phase 7a)
- starts with `<<` → heredoc (Phase 7b — this phase)
- otherwise → string interpolation lowering (Phase 6y)

### Lowering

| Source                            | SIR shape                            |
|-----------------------------------|--------------------------------------|
| `` `<<EOF\nhello\nEOF` ``         | `StrLit("hello\n")`                  |
| `` `<<-EOF\nhello\n  EOF` ``      | `StrLit("hello\n")`                  |
| `` `<<~EOF\n  hello\n  EOF` ``    | `StrLit("hello\n")` (lexer pre-strips indent) |

The `<<~TAG` common-indent stripping is performed by the lexer's `finalize_heredoc` before the token reaches the lowerer; this routine just removes the opener prefix (`<<`, `<<-`, `<<~`) and the trailing closing-tag suffix.

The synthetic `StrLit` triggers `Feature::Strings`.

### v0 deferred limitations

- Interpolation inside the body (`#{name}`) is NOT split — the body lowers as a single `StrLit` with `#{...}` markers preserved verbatim.  Follow-up will reuse the Phase 6y interpolation splitter.
- Non-interpolating heredocs (`<<'TAG'`) and the `<<"TAG"` form are not yet distinguished from the unquoted form — the lexer doesn't carry the quote state through.
- Escape sequences inside the body are kept literal.

### Tests

- `ruby-to-semantic-ir`: 132 → **137** (+5):
  - `plain_heredoc_lowers_to_strlit_body_only` — happy path.
  - `dash_indent_heredoc_lowers_to_strlit_body_only` — `<<-EOF`.
  - `tilde_indent_heredoc_strips_common_leading_whitespace` — `<<~EOF`.
  - `heredoc_triggers_strings_feature` — manifest gating.
  - `heredoc_module_passes_sir_validator` — end-to-end smoke.

## [0.28.0] - 2026-05-25

### Added (Phase 7a — backtick command literal lowering)

`lower_factor_atom`'s `String` case now dispatches by lexeme prefix:
- starts with `` ` `` → new `lower_backtick_command_literal` helper (Phase 7a).
- otherwise → existing `lower_string_literal_with_interp` (Phase 6y).

### Lowering

| Source       | SIR shape                                                       |
|--------------|-----------------------------------------------------------------|
| `` `ls` ``   | `BuiltinCall("backtick", [StrLit("ls")])` + MayBlock\|MayPrint\|MayThrow |
| `` `` ``     | `BuiltinCall("backtick", [StrLit("")])` + same effects          |

The triple-effect set reflects that command execution may **block** on the child process, **print** stdout/stderr, and **throw** if the command can't be invoked.  Marker-builtin pattern reused from Phase 6v (`__rescue_marker__`), Phase 6w (`lambda`/`proc`), and Phase 6y (`__interp__`).

The synthetic `StrLit` body triggers `Feature::Strings`.

### v0 deferred limitations

- Interpolation inside the body (`` `echo #{name}` ``) is NOT split — the body lowers as a single `StrLit` with any `#{...}` markers preserved verbatim.  Follow-up will reuse the Phase 6y splitter.
- Escape sequences are already resolved by the lexer's `backtick_body` state.

### Tests

- `ruby-to-semantic-ir`: 127 → **132** (+5):
  - `backtick_command_literal_lowers_to_backtick_builtin_call` — happy path.
  - `backtick_command_literal_carries_effect_set` — asserts MayBlock + MayPrint + MayThrow.
  - `empty_backtick_command_literal_lowers_with_empty_body` — `` `` ``.
  - `backtick_command_literal_triggers_strings_feature` — manifest gating.
  - `backtick_command_literal_module_passes_sir_validator` — end-to-end smoke.

## [0.27.0] - 2026-05-25

### Added (Phase 6z — float / hex / bin / oct numeric literal lowering)

`lower_factor_atom` now hands every `TokenType::Number` token to a new helper, `lower_numeric_literal`, which dispatches on shape (radix-prefix → IntLit with chosen radix; float-shape → FloatLit; otherwise → decimal IntLit).

### Lowering

| Source       | SIR shape                                |
|--------------|------------------------------------------|
| `42`         | `IntLit { value: 42 }`                   |
| `1_000_000`  | `IntLit { value: 1000000 }`              |
| `0x1F`       | `IntLit { value: 31 }` (radix 16)        |
| `0xDEAD_BEEF`| `IntLit { value: 3735928559 }`           |
| `0b1010`     | `IntLit { value: 10 }` (radix 2)         |
| `0o17`       | `IntLit { value: 15 }` (radix 8)         |
| `0d42`       | `IntLit { value: 42 }` (radix 10 explicit) |
| `1.5`        | `FloatLit { value: 1.5 }`                |
| `1e10`       | `FloatLit { value: 1e10 }`               |
| `1.5e-3`     | `FloatLit { value: 0.0015 }`             |

Float literals additionally trigger `Feature::Floats` in the module manifest.  The manifest aggregator now propagates `Feature::Floats` alongside the prior set (`Strings`, `Closures`, `Symbols`, etc.).

Underscore separators are stripped before parsing.  Radix detection checks `bytes[1] ∈ {x,X,b,B,o,O,d,D}` after a leading `0`.  Float detection is a single scan for `.` or `e`/`E` — mutually exclusive with radix prefixes in the Ruby grammar.

### v0 deferred limitations

- Rational (`r`) / Complex (`i`) numeric suffixes (lexed by Phase 4f) are rejected by the integer-parse path — a future phase will route those into `BuiltinCall("rational", ...)` / `BuiltinCall("complex", ...)` markers.
- Negative literals continue to flow through the Phase 6k unary-minus path; this routine sees only the magnitude.
- Legacy octal (`017` without `0o` prefix) is not supported by either the lexer or this lowerer.

### Tests

- `ruby-to-semantic-ir`: 121 → **127** (+6):
  - `float_literal_lowers_to_floatlit_and_triggers_floats_feature` — `1.5`.
  - `float_literal_with_signed_exponent_lowers_correctly` — `1.5e-3`.
  - `hex_literal_lowers_to_intlit_with_correct_value` — `0xDEAD_BEEF` + asserts Floats feature NOT triggered.
  - `binary_literal_lowers_to_intlit` — `0b1010`.
  - `octal_literal_lowers_to_intlit` — `0o17`.
  - `float_literal_module_passes_sir_validator` — end-to-end validator smoke.

## [0.26.0] - 2026-05-25

### Added (Phase 6y — string interpolation lowering)

`lower_factor_atom` now hands every `TokenType::String` token to a new helper, `lower_string_literal_with_interp`, which scans the raw content for `#{...}` interpolation markers and emits the appropriate SIR shape.

### Lowering

| Source              | SIR shape                                                                                       |
|---------------------|-------------------------------------------------------------------------------------------------|
| `"plain"`           | `StrLit("plain")` — zero-cost fast path                                                         |
| `"#{x}"`            | `VarRef("x")` — single non-literal segment, no wrapper                                          |
| `"hi #{name}"`      | `BuiltinCall("string_concat", [StrLit("hi "), VarRef("name")])`                                 |
| `"#{a}#{b}"`        | `BuiltinCall("string_concat", [VarRef("a"), VarRef("b")])`                                      |
| `"sum=#{1+2}"`      | `BuiltinCall("string_concat", [StrLit("sum="), BuiltinCall("__interp__", [StrLit("1+2")])])`    |

Bare-identifier interp bodies route to `VarRef` with the same `Scope::Param` / `Scope::Local` dispatch as the regular factor-atom Name case.  Complex bodies emit a marker `BuiltinCall("__interp__", [StrLit(raw)])` — same marker pattern as Phase 6v's `__rescue_marker__` / `__ensure_marker__`.

Brace depth is tracked while scanning the interp body so nested `{...}` (inline hash, block) is balanced correctly, matching the lexer's `interp_brace_depth` state.

### v0 deferred limitations

- Complex interp bodies (arithmetic, method calls, nested strings, sigil vars) are kept as a `__interp__` marker rather than being recursively parsed.  A future phase will invoke the Ruby parser/lowerer on the body so the SIR carries proper semantic info.
- Escape sequences inside the string literal (`\n`, `\t`, `\\`, `\"`) pass through unchanged — the lexer hasn't unescaped them yet.
- Sigil-prefixed vars (`@x`, `$x`, `@@x`) inside an interp body intentionally fall through to the `__interp__` marker; Phase 6x's sigil routing only fires at lex time, not at interp-split time.

### Tests

- `ruby-to-semantic-ir`: 116 → **121** (+5):
  - `plain_string_with_no_interp_remains_a_strlit` — regression for the zero-cost path.
  - `interpolated_string_with_bare_name_lowers_to_string_concat` — happy path.
  - `interpolated_string_that_is_only_interp_unwraps_to_a_single_segment` — `"#{name}"`.
  - `interpolated_string_with_expression_uses_interp_marker` — `"sum=#{1+2}"`.
  - `interpolated_string_module_passes_sir_validator` — end-to-end validator smoke.

## [0.25.0] - 2026-05-25

### Added (Phase 6x — sigil variable refs `@x`, `@@x`, `$x`)

`lower_factor_atom` now documents Ruby's sigil-prefixed variable convention and explicitly routes all three sigil forms to `Scope::Local` with the sigil preserved in the bound name.

### Lowering

| Source | SIR shape |
|---|---|
| `@a` | `VarRef { name: "@a", scope: Local }` |
| `@@count` | `VarRef { name: "@@count", scope: Local }` |
| `$config` | `VarRef { name: "$config", scope: Local }` |

`Scope::Local` is the conservative v0 choice — the SIR validator enforces that `Scope::Global` references have a matching `Global` declaration in the module, and the Ruby lowerer doesn't yet auto-emit those declarations.  Downstream emitters can still recognise the sigil form via the leading `@` / `@@` / `$` in `name`.

### v0 deferred limitations

- No SIR `IVar` / `CVar` / `GVar` scope.  A future phase will add auto-`Global`-declaration for `$x` so the validator-true mapping `$x` → `Scope::Global` becomes usable.
- The sigil convention is purely a name-encoding hint for downstream emitters; the SIR scope machinery treats all three identically.

### Tests

- `ruby-to-semantic-ir`: 112 → **116** (+4):
  - `global_var_ref_preserves_sigil_in_name` — `$config` keeps the `$`.
  - `instance_var_ref_lowers_with_local_scope_and_sigil_preserved` — `@a`.
  - `class_var_ref_lowers_with_local_scope_and_double_at_preserved` — `@@count`.
  - `sigil_vars_module_passes_sir_validator` — end-to-end validator smoke across all three sigils.

## [0.24.0] - 2026-05-25

### Added (Phase 6w — arrow-lambda lowering)

New `lower_lambda_literal` helper handles `->(params){body}`:

- Extracts params from the leading parens-list (Phase 6s — splat names preserved bare).
- Hoists the block body to a top-level `Function` named `__block_<n>` (reusing Phase 6g's counter) via new `hoist_lambda_body` helper.
- Emits `Expr::BuiltinCall { name: "lambda", args: [MakeClosure { fn_name, captures: [] }], effects: PURE }`.
- Auto-sets `Feature::Closures` (and `Feature::DynamicTyping` if any params present).

`ruby_builtin_effects` extended to recognise `lambda` and `proc` so `lambda { ... }` and `proc { ... }` (keyword forms going through `method_with_block`) also emit `BuiltinCall("lambda"|"proc", ...)`.  Downstream emitters see a single closure-construction shape.

### v0 deferred limitations

- Captures from the enclosing scope are NOT computed for arrow lambda bodies (same limitation as Phase 6g blocks).
- `lambda { … }` / `proc { … }` work at statement position only — they can't be used as an expression RHS because `method_with_block` is not part of `factor`.

### Tests

- `ruby-to-semantic-ir`: 108 → **112** (+4):
  - `arrow_lambda_no_params_lowers_to_lambda_builtin` — bare `-> { 1 }`.
  - `arrow_lambda_with_params_hoists_body_with_params` — params propagate to hoisted Function.
  - `lambda_keyword_form_lowers_via_method_with_block` — `lambda { |x| x + 1 }` keyword form.
  - `arrow_lambda_module_passes_sir_validator` — end-to-end validator smoke.

## [0.23.0] - 2026-05-25

### Added (Phase 6v — `begin … rescue … ensure … end` lowering)

New `lower_begin_statement` helper fans the source `begin_statement` into multiple SIR statements (via the existing `lower_statement_inner_multi` Vec<Stmt> dispatch from Phase 6r):

- Body stmts inline.
- ExprStmt(BuiltinCall("__rescue_marker__", [StrLit(exc_types_csv), StrLit(var_name)])) per `rescue_clause`, followed by that clause's body stmts inline.
- ExprStmt(BuiltinCall("__ensure_marker__", [])) before the ensure body stmts inline (if `ensure_clause` present).

Markers carry the `Effect::MayThrow` tag.  Strings feature is auto-set (markers emit StrLits).

### v0 deferred limitations

- SIR has no try/catch primitive — markers only signal the form's presence to downstream emitters that target languages with real exceptions.
- Rescue body is *unreachable* in SIR's effect model; the marker is informational.
- `else` clause inside `begin` (Ruby's "no-exception" branch) is not supported by the grammar.
- Exception class hierarchy is not modelled — `rescue StandardError` (with `=>`) and bare `rescue` lower identically apart from the marker payload.

### Tests

- `ruby-to-semantic-ir`: 104 → **108** (+4):
  - `begin_without_rescue_lowers_body_inline` — no marker for plain `begin … end`.
  - `begin_with_rescue_emits_rescue_marker` — `__rescue_marker__("StandardError", "e")`.
  - `begin_with_ensure_emits_ensure_marker` — `__ensure_marker__()`.
  - `begin_with_rescue_and_ensure_emits_both_markers_in_order` — full sequence shape.

## [0.22.0] - 2026-05-25

### Added (Phase 6u — `case … when … else … end` lowering)

New `lower_case_statement` helper folds the source `case_statement` into a chained `Expr::If`:

```
case x
when v1, v2 then a
when v3     then b
else c
end
```

→

```
if ((x == v1) || (x == v2)) then a
else if (x == v3) then b
else c
```

Each when_clause becomes one nested `If`; multi-value `when 1, 2, 3` lists OR-fold left-to-right using `BuiltinCall("or", ...)`.  The else clause (or implicit `NilLit` block) caps the chain.  Comparisons use `BuiltinCall("==", [scrutinee, value])` — see v0 deferred caveats below.

The result is wrapped in `Stmt::ExprStmt`.

### v0 deferred limitations

- Comparisons use `==` not `===`.  Ruby's case-equality (class-aware: `Integer === 1`, range membership, regex match) is NOT modelled.  Phase 7d adds full `case/in` pattern matching.
- Range/Regex/Class values in `when` lists work syntactically but don't behave as Ruby would.

### Tests

- `ruby-to-semantic-ir`: 100 → **104** (+4):
  - `case_single_when_lowers_to_if_with_eq` — basic shape check.
  - `case_with_multi_value_when_lowers_to_or_chain` — `==` × 3 + `or` × 2.
  - `case_with_else_terminates_chain` — else body lands in chain tail.
  - `case_without_else_uses_nil_tail` — no else → NilLit tail.

## [0.21.0] - 2026-05-25

### Added (Phase 6t — `yield` lowering)

`yield ...` → `Stmt::ExprStmt(Expr::BuiltinCall("yield", lowered_args, EffectSet::PURE))`.

Lowering walks the optional `yield_args` wrapper (when present), extracts its `call_arg` children, and routes each through Phase 6s's `lower_call_arg` helper.  Bare `yield` (no `yield_args` wrapper) lowers to an empty-arg BuiltinCall.

Effects are PURE — `yield` invokes the caller-supplied block, whose effects are tracked at the *block construction* site (via `Expr::MakeClosure`'s captured effect set), not at the yield call site.  Modelling `yield` as PURE keeps the effect lattice from double-counting block effects.

### Tests

- `ruby-to-semantic-ir`: 96 → **100** (+4):
  - `bare_yield_lowers_to_yield_builtin_no_args`
  - `yield_with_one_arg_lowers_to_builtin_with_one_arg`
  - `yield_with_paren_args_lowers_to_two_arg_builtin`
  - `yield_with_splat_arg_lowers_with_splat_envelope` — exercises Phase 6t × Phase 6s composition.

## [0.20.0] - 2026-05-25

### Added (Phase 6s — splat / double-splat lowering)

#### Call args (preserved through SIR)

| Source | Lowered |
|---|---|
| `f(*arr)` | `DirectCall(f, [BuiltinCall("splat", [VarRef(arr)])])` |
| `f(**hsh)` | `DirectCall(f, [BuiltinCall("double_splat", [VarRef(hsh)])])` |
| `f(1, *arr, **hsh)` | three positional args: `IntLit(1)`, `splat(arr)`, `double_splat(hsh)` |

New helper `lower_call_arg` dispatches on the leading `*` / `**` token (if any) and wraps the inner expression in a `BuiltinCall` envelope.  No prefix → return the bare expression.  Downstream emitters can pattern-match the builtin name to convert back to splat syntax in target source.

Renamed `head_call_expression_children` → `head_call_args` (returns `call_arg` Nodes instead of `expression` Nodes).  `lower_method_call` dispatches on the rule name: `method_call` uses the new `call_arg` shape; `method_call_no_paren` keeps the legacy bare-`expression` shape (paren-less splat is a deferred limitation — see ruby-parser changelog).

`fold_one_dot_call` likewise routes through `lower_call_arg`.

#### Params (lossy at SIR level)

`*args` / `**kwargs` lower to regular `Param { name: "args" / "kwargs" }` — SIR's `Param` has no variadic flag, so the splat-ness is dropped.  The parameter-name extractor in `lower_def_statement` skips the splat-prefix tokens (`*` and `**`) when locating the identifier.

**Downstream impact**: target source emitted for variadic functions will treat the parameter as positional.  Calls passing splat args (via the `BuiltinCall("splat", ...)` envelope) still preserve the variadic shape — the asymmetry only matters for definitions of variadic functions.  Tracked as a deferred limitation for a future SIR phase that adds variadic-aware `Param`.

### Tests

- `ruby-to-semantic-ir`: 91 → **96** (+5):
  - `splat_call_arg_lowers_to_splat_builtin`
  - `double_splat_call_arg_lowers_to_double_splat_builtin`
  - `mixed_call_args_with_splats_lower_in_order`
  - `splat_param_lowers_to_bare_name_param` (asserts lossy v0 lowering)
  - `splat_call_arg_module_passes_sir_validator` — end-to-end validator smoke.

## [0.19.0] - 2026-05-25

### Added (Phase 6r — multiple assignment lowering)

`a, b = 1, 2` fans out into one SIR statement per (LHS, RHS) pair — each lowered identically to the single-LHS `assignment` rule (`LetBinding` on first sighting, `Assign` thereafter).

#### Architecture change

New dispatch wrapper `lower_statement_inner_multi(node) → Vec<Stmt>`:
- `multi_assignment` → delegates to `lower_multi_assignment` (returns `Vec<Stmt>`).
- All other statement forms → wraps the single `lower_statement_inner` result in `vec![stmt]`.

The four statement-list walkers (`lower_program`, `lower_clause_statements`, `lower_def_statement` body, `lower_method_with_block` body) updated from `.push(...)` to `.extend(...)`.  The modifier-statement LHS path keeps the single-stmt `lower_statement_inner` call because `multi_assignment` is not an eligible LHS form in `modifier_statement`.

#### v0 restrictions (rejected with `RubyLowerError`)

- LHS count must equal RHS count.
- Single-RHS auto-unpack (`a, b = arr`) — not supported.
- Splat targets (`a, *b = 1, 2, 3`) — Phase 6s.

#### Lowering rule

For each pair `(lhs[i], rhs[i])`:
- First sighting of `lhs[i]` in this scope → `Stmt::LetBinding { name, value: rhs[i], … }`.
- Subsequent sighting → `Stmt::Assign { name, scope: Local, value: rhs[i], … }` (and sets `Feature::MutableBindings`).

RHS expressions are lowered first (in source order), then the LHS bindings happen in source order.  The parallel-binding swap case (`a, b = b, a`) is NOT correctly v0-lowered (would silently mis-evaluate); this is documented as a deferred limitation.

### Tests

- `ruby-to-semantic-ir`: 86 → **91** (+5):
  - `multi_assignment_lowers_to_independent_let_bindings` — basic `a, b = 1, 2` → two `LetBinding`.
  - `multi_assignment_redeclaration_uses_assign` — `a = 1; b = 2; a, b = 3, 4` second multi-assign uses `Assign`.
  - `multi_assignment_three_names_emits_three_stmts` — three LHS / three RHS → three SIR stmts.
  - `multi_assignment_arity_mismatch_errors` — `a, b = 1, 2, 3` returns `RubyLowerError`.
  - `multi_assignment_module_passes_sir_validator` — end-to-end validator smoke.

## [0.18.0] - 2026-05-24

### Added (Phase 6q — modifier conditionals/loops lowering)

New `lower_modifier_statement` handler dispatches on the parser's `modifier_statement` node.

Lowering table:

| Source              | Lowered SIR                                              |
|---------------------|----------------------------------------------------------|
| `lhs if cond`       | `Stmt::ExprStmt(Expr::If(cond, [lhs], Nil))`             |
| `lhs unless cond`   | `Stmt::ExprStmt(Expr::If(not(cond), [lhs], Nil))`        |
| `lhs while cond`    | `Stmt::While(cond, [lhs])`                               |
| `lhs until cond`    | `Stmt::While(not(cond), [lhs])`                          |

The lowering produces the same canonical `Expr::If` / `Stmt::While` shapes as the leading-keyword `if_statement` / `while_statement` lowerings — every downstream emitter (semantic-ir-to-python, -rust, -typescript, -go) handles modifier forms transparently with no new code paths.

`while`/`until` modifier variants set `Feature::Loops` automatically, matching the leading-keyword loop behaviour.

The LHS statement is wrapped in a single-statement `Block` whose `value` is `NilLit` — the modifier form is never tail-promoted to an expression (it sits in statement position only).

### Tests

- `ruby-to-semantic-ir`: 81 → **86** (+5):
  - `if_modifier_lowers_to_expr_if_statement` — produces `ExprStmt(If)` with bare cond.
  - `unless_modifier_wraps_condition_in_not` — cond becomes `BuiltinCall(not, …)`.
  - `while_modifier_lowers_to_stmt_while` — `Stmt::While` with bare cond.
  - `until_modifier_negates_condition_in_while` — `Stmt::While` with `not(cond)`.
  - `modifier_module_passes_sir_validator` — end-to-end validator smoke test across all four forms.

## [0.17.0] - 2026-05-24

### Added (Phase 6p — compound assignment lowering)

SIR encoding (for each `x op= rhs`):
```
Stmt::Assign {
  name: "x",
  scope: Local,
  value: Expr::BuiltinCall {
    name: "<op>",   // "+", "-", "*", "/", "or", "and"
    args: [VarRef("x"), <rhs>],
  },
}
```

| Source | Lowered as |
|---|---|
| `x += y` | `x = x + y` |
| `x -= y` | `x = x - y` |
| `x *= y` | `x = x * y` |
| `x /= y` | `x = x / y` |
| `x \|\|= y` | `x = x or y` |
| `x &&= y` | `x = x and y` |

Lowering identically to `x = x op y` means downstream emitters (semantic-ir-to-python, -rust, -typescript, -go) need no new code path — the existing assignment + binary-op lowering handles both forms.

### Lowerer changes
- `lower_assignment` now reads the operator token (skipping the leading NAME) to dispatch on `EQUALS` vs the six compound forms.
- Compound forms always emit `Stmt::Assign` (never `LetBinding`) even on first sighting — the read of `x` before the write means the binding semantically pre-exists.  Sets `Feature::MutableBindings` automatically.

### Tests (+4 new, total 81)
- `plus_equals_lowers_to_assign_with_plus_builtin`
- `all_arithmetic_compound_assigns_lower_correctly` — `+=`, `-=`, `*=`, `/=`.
- `logical_compound_assigns_lower_to_or_and_builtins` — `||=` → `"or"`, `&&=` → `"and"`.
- `compound_assign_module_passes_sir_validator` — end-to-end validator smoke test.

## [0.16.0] - 2026-05-24

### Added (Phase 6o — ternary lowering)

SIR encoding:
```
cond ? a : b  →  Expr::If {
                   cond,
                   then_branch: Block { stmts: [], value: a },
                   else_branch: Block { stmts: [], value: b },
                 }
```

Lowering identically to `if cond then a else b end` means downstream emitters (semantic-ir-to-python, semantic-ir-to-rust, etc.) need no new code path — the existing if-lowering paths handle both syntactic forms transparently.

**Right-associativity** falls out of the grammar: `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`, so the inner ternary nests inside the outer's else-branch as another `Expr::If`.

### Lowerer changes
- `lower_expression` gained a `"ternary"` dispatch arm.
- New helper `lower_ternary(node)` filters operand sub-nodes: one operand (pass-through) or three (cond/then/else → `Expr::If`).

### Tests (+3 new, total 77)
- `ternary_lowers_to_if_expr_with_branch_blocks` — `x = 1 ? 2 : 3` → `LetBinding { value: If { cond=1, then=2, else=3 } }`.
- `ternary_right_associative_nests_in_else_branch` — `x = 1 ? 2 : 3 ? 4 : 5` produces a nested If in the outer else.
- `ternary_module_passes_sir_validator` — end-to-end validator smoke test.

## [0.15.0] - 2026-05-24

### Added (Phase 6n — range expressions lowering)

SIR encoding:
- `a..b`  →  `BuiltinCall("range", [a, b, BoolLit(false)])` ; inclusive end
- `a...b` →  `BuiltinCall("range", [a, b, BoolLit(true)])`  ; exclusive end

A single builtin name (`range`) handles both forms; the third argument carries the inclusive/exclusive flag so downstream emitters can pattern-match once and read the flag.  Effects default to `PURE` — constructing a range observes nothing.

### Lowerer changes
- `lower_expression` gained a `"range"` dispatch arm.
- New helper `lower_range(node)` filters operand `logical_or` sub-nodes from the `..`/`...` operator token, then either passes through (1 operand, no op) or emits the three-arg `BuiltinCall`.

### Tests (+4 new, total 74)
- `inclusive_range_lowers_to_range_builtin_with_false_flag` — `1..5` → flag = false.
- `exclusive_range_lowers_to_range_builtin_with_true_flag` — `1...5` → flag = true.
- `range_with_variable_operands_uses_var_refs` — `(a..b)` over function params (parens dodge the lessons.md ambiguity).
- `range_module_passes_sir_validator` — end-to-end smoke test through the validator.

### Out of scope (deferred to follow-up phase)
- Endless ranges `(1..)`, `arr[2..]` (lexer 4e already flags these; parser support TBD).
- Beginless ranges `(..5)`.

## [0.14.0] - 2026-05-24

### Added (Phase 6m — logical operators lowering)

SIR encoding:
- `a || b`, `a or b`  →  `BuiltinCall("or",  [a, b])`
- `a && b`, `a and b` →  `BuiltinCall("and", [a, b])`
- `!x`, `not x`       →  `BuiltinCall("not", [x])`
- `!!x`               →  `BuiltinCall("not", [BuiltinCall("not", [x])])`

Both symbol form (`||`/`&&`/`!`) and keyword form (`or`/`and`/`not`) collapse to the same builtin name — v0 simplification.  All effects default to `PURE`.

### Lowerer changes
- `lower_expression` gained dispatch arms for `logical_or`, `logical_and`, `logical_not`, and `comparison` (renamed from the old `expression` arm).  The `expression` arm itself is now a pass-through to the inner `logical_or` node.
- New helpers `lower_logical_chain(node, op_lexemes, builtin_name)` and `lower_logical_not(node)`.
- `lower_logical_chain` matches operators by lexeme (covers both `||`/`&&` Name-classified tokens and `or`/`and` Keyword tokens uniformly).

### Tests (+6 new, total 70)
- `logical_or_symbol_lowers_to_or_builtin`
- `logical_and_symbol_lowers_to_and_builtin`
- `logical_keyword_form_lowers_same_as_symbol`
- `logical_not_symbol_lowers_to_not_builtin`
- `logical_chain_and_then_or_nests_correctly` — `a && b || c` parses & lowers as `(a && b) || c`.
- `logical_module_passes_sir_validator`

All six use the parens workaround (`(a || b)` instead of bare `a || b`) inside def bodies to dodge the `method_call_no_paren` ambiguity (logged in lessons.md, parser CHANGELOG).

## [0.13.0] - 2026-05-23

### Added (Phase 6l — method receiver chains lowering)

Each `.method[(...)]` step in a receiver chain lowers to:

```
BuiltinCall {
  name: "__method__",
  args: [receiver, StrLit(method_name), ...actual_args],
  effects: PURE,
}
```

The receiver stays as a first-class expression so arbitrary nesting works (`a.b.c.d`).  The method name lives as a `StrLit` so backends can dispatch by string.  This avoids growing the shared `semantic-ir::Expr` enum.

**Why BuiltinCall and not DirectCall?**  The validator checks `DirectCall.fn_name` against the module's function table; our synthetic `__method__` envelope is intentionally not a declared function — it's a wire-format tag for backends.  BuiltinCall has no such resolution check.

**Effect set**: defaults to `PURE`.  Receiver-dispatched calls are type-erased at this layer; a later receiver-type analysis pass can widen as needed.

**Feature side-effect**: any dot_call fires `Feature::Strings` (because of the synthesised StrLit).  This is auto-added to the manifest in `lower_program`'s feature-collection pass.

### Lowerer changes
- New helpers: `apply_dot_chain(atom, factor_node)`, `fold_one_dot_call(receiver, dot_node)`, `head_call_expression_children`.
- `lower_factor` split into `lower_factor` (atom extraction + dot-chain application) and `lower_factor_atom` (the pre-6l atom logic).
- `lower_method_call` collects head-call args via `head_call_expression_children` so args inside `dot_call` subtrees don't leak into the head call.
- `lower_expression` gains a dispatch arm for `method_call` (which can now appear in expression position because it's the first atom alternative in `factor`).
- `Feature::Strings` added to the manifest-population loop in `lower_program`.

### Tests (+5 new, total 64)
- `dot_chain_lowers_to_method_builtincall` — `foo.bar` produces `BuiltinCall("__method__", [VarRef(foo), StrLit("bar")])`.
- `dot_chain_two_steps_nests_outer_recv` — `foo.bar.baz` nests as `__method__(__method__(foo, "bar"), "baz")`.
- `dot_call_with_args_includes_them_after_method_name` — `obj.add(1, 2)` → `__method__(obj, "add", 1, 2)`.
- `dot_chain_on_method_call_head` — `puts(1).then_something` keeps the head BuiltinCall("puts") and wraps it in `__method__(_, "then_something")`.
- `dot_chain_module_passes_sir_validator` — full module with a chain inside a function body validates clean.

## [0.12.0] - 2026-05-22

### Added (Phase 6k — unary minus lowering)
- `lower_expression` dispatches `unary_minus` to a new arm emitting `Expr::BuiltinCall { name: "neg", args: [inner], effects: PURE }`.

### Tests (+5 new, total 59)
- `unary_minus_on_number_lowers_to_neg_builtin`, `unary_minus_on_name_carries_scope`, `double_unary_minus_nests_correctly`, `unary_minus_with_binary_plus_resolves_precedence_correctly`, `unary_minus_module_passes_sir_validator`.

## [0.11.0] - 2026-05-22

### Added (Phase 6j — `return` / `break` / `next` lowering)
- `lower_statement_inner` dispatches `return_statement` / `break_statement` / `next_statement` to a common arm that emits `Expr::BuiltinCall` with the keyword name, the optional trailing expression as the sole argument (or `NilLit` when absent), and `Effect::Divergent` declared.

### Tests (+5 new, total 54)
- `return_with_value_lowers_to_divergent_builtin_call`, `bare_return_lowers_with_nil_arg`, `break_and_next_lower_to_their_respective_builtins`, `return_inside_def_body`, `return_module_passes_sir_validator`.

## [0.10.0] - 2026-05-22

### Added (Phase 6i — comparison operator lowering)
- `lower_expression` now dispatches the renamed `sum` rule via the existing `lower_binary_chain(..., ["PLUS", "MINUS"])`.
- New `lower_comparison_chain` helper — left-associative reduce of comparison operators into `BuiltinCall("==", [lhs, rhs])` (and similarly for `!=`, `<`, `>`, `<=`, `>=`).

### Tests (+5 new, total 49)
- `equality_op_lowers_to_builtin_call`, `less_than_op_lowers_to_builtin_call`, `all_six_comparison_operators_lower_with_correct_names`, `comparison_has_lower_precedence_than_arithmetic`, `comparison_used_in_if_condition_passes_validator`.

## [0.9.0] - 2026-05-22

### Added (Phase 6h — no-paren method call lowering)
- `lower_statement_inner` dispatches `method_call_no_paren` to the existing `lower_method_call` helper.

### Tests (+5 new, total 44)
- `no_paren_call_with_single_arg_lowers_to_builtin_call`, `no_paren_call_with_multiple_args`, `no_paren_call_with_binary_expr_arg_groups_correctly`, `no_paren_call_module_passes_sir_validator`, `paren_form_still_lowers_unchanged`.

## [0.8.0] - 2026-05-22

### Added (Phase 6g — method-with-block lowering)
- `lower_method_with_block` lowers the `method_with_block` rule node into:
  1. A `BuiltinCall` / `DirectCall` for the method dispatch.
  2. A hoisted top-level `Function` named `__block_<n>` for the block body.
  3. An `Expr::MakeClosure { fn_name, captures: [] }` appended as the call's trailing argument.
- New `Lowerer.block_counter: usize` field, new `hoist_block_to_function` helper, `Feature::Closures` declared, expanded builtin iterator table.

### Tests (+5 new, total 39)
- `brace_block_hoists_to_synthetic_function_and_make_closure`, `do_block_with_pipe_params_lowers_to_function_with_params`, `multiple_blocks_get_distinct_synthetic_names`, `block_module_declares_closures_feature`, `block_lowering_passes_sir_validator`.

## [0.7.0] - 2026-05-22

### Added (Phase 6f — class/module lowering with nested-def hoisting)
- `lower_statement_inner` dispatches `class_statement` / `module_statement` rule nodes.  In v0, SIR has no native `class` / `namespace` node, so the declaration itself lowers to a no-op `Stmt::ExprStmt(NilLit)` — same shape used for already-hoisted `def_statement`s.
- New `collect_def_statements_from_body(node)` helper recursively walks a class/module body and hoists every nested `def_statement` to a top-level `Function` (same machinery as the program-level pre-pass).  Nested class/module declarations are recursed into so deeply-nested `def`s still hoist.
- Each `def` body is lowered with a fresh `declared_locals` + `current_params` scope (snapshot/restore in `lower_def_statement`), so locals from sibling methods or the surrounding class don't leak across method boundaries.

### Documented v0 caveat
The hoisted methods land at top-level, *not* nested under the class name.  In real Ruby, `class Foo; def bar` makes `bar` an instance method of `Foo`; v0 SIR collapses the namespace.  The validator still accepts the result because every function has a unique name across the lowered module, and `main` remains the only export.  Proper namespace handling lands when SIR grows a `class` / `namespace` node in a future phase.

### Tests (+4 new, total 34)
- `class_with_method_hoists_def_to_top_level` — `class Foo; def greet; end; end` exposes `greet` on `m.functions`.
- `empty_class_lowers_cleanly` — `class Foo; end` produces a module with only `main` plus a no-op `Stmt::ExprStmt(NilLit)` in the main body.
- `module_with_def_hoists_def_to_top_level` — `module M; def helper; end; end` exposes `helper`.
- `class_module_lowering_passes_sir_validator` — a combined class+module module passes `semantic_ir::validate`.

## [0.6.0] - 2026-05-20

### Added (Phase 6e — symbol-literal lowering)
- `lower_symbol_literal` — picks the first Name / Keyword / String token under the `symbol_literal` node and emits an `Expr::SymLit` with that lexeme as the symbol name.  Quoted symbols (`:"hello world"`) work transparently because the String token's value already has the surrounding quotes stripped.
- Declares `Feature::Symbols` on every symbol literal (same feature the hash-shorthand entries already used).

### Tests (+4 new, total 30)
- `:foo` → `SymLit("foo")`.
- `:"hello world"` → `SymLit("hello world")` (spaces preserved).
- `:def` (keyword-shaped name) → `SymLit("def")`.
- Symbol-containing module passes `semantic_ir::validate`.

## [0.5.0] - 2026-05-20

### Added (Phase 6d — array and hash literal lowering)
- `lower_array_literal` — `[a, b, c]` → `Expr::SeqLit` with all element expressions lowered recursively.
- `lower_hash_literal` — `{a: 1, b => 2}` → `Expr::MapLit { entries }`.
- `lower_hash_entry` handles both syntactic forms:
  - **Shorthand** (`NAME COLON expression`) — the Name becomes a `SymLit` key (sugar for `:name =>`).
  - **Hash-rocket** (`expression "=>" expression`) — both sides are lowered as ordinary expressions.
- `lower_expression` now dispatches `array_literal` and `hash_literal` rule nodes alongside `expression`/`term`/`factor`.
- Feature tracking extended: `Sequences` declared on SeqLit, `Maps` on MapLit, `Symbols` on the shorthand hash-entry key.

### Tests (+4 new, total 26)
- `[1, 2, 3]` → `SeqLit` with three items.
- `[]` → empty `SeqLit`.
- `{a: 1, b: 2}` → `MapLit` with two entries whose keys are `SymLit("a")` and `SymLit("b")`.
- Combined array+hash module passes `semantic_ir::validate` (feature manifests align exactly).

## [0.4.0] - 2026-05-20

### Added (Phase 6c — `while … end` / `until … end` lowering)
- New `lower_while_or_until` handler emits a `Stmt::While`.  `until cond` lowers to `while !cond` (condition wrapped in `BuiltinCall("not", ...)`).
- `Feature::Loops` is now added to the module manifest whenever a `Stmt::While` is emitted (the SIR validator requires it).
- Loop body uses the existing `lower_clause_statements` helper, so locals introduced inside the loop don't leak to the outer scope.

### Tests (+3 new, total 22)
- `while_lowers_to_stmt_while` — basic while produces `Stmt::While`.
- `until_negates_condition` — `until cond` wraps cond in `BuiltinCall("not", ...)`.
- `while_module_passes_sir_validator` — a while-loop module passes `semantic_ir::validate` (Feature::Loops gating works).

## [0.3.0] - 2026-05-20

### Added (Phase 6b — `if … else … end` / `unless` lowering)
- New `lower_if_or_unless` handler — both rules produce a single `Expr::If` because SIR treats conditionals as expressions (every branch yields a value).
- `unless cond` lowers to `if !cond` by wrapping the condition in `BuiltinCall("not", [cond])`.
- `elsif` chains lower with right-associative nesting: the outermost `If`'s `else_branch` is itself a `Block` whose `value` is another `If` for the first `elsif`, and so on.  The validator sees one well-formed expression tree.
- New `lower_clause_statements` helper saves/restores `declared_locals` around each branch so locals introduced in one `if`/`elsif`/`else` arm don't leak into siblings (which would have caused spurious `Stmt::Assign` emissions and validator errors).
- `Lowerer.features_used: HashSet<Feature>` — tracks which SIR features the lowering actually exercises.  `compile` now emits a manifest that lists *only* the features in use:
  - `DynamicTyping` whenever a function has at least one untyped param.
  - `MutableBindings` whenever a `Stmt::Assign` re-binds an existing local.
  This swaps the previous "always declare DynamicTyping" approach for an exact-match manifest, which is what the validator requires.

### Tests (+5 new, total 19)
- `if_lowers_to_expr_if` — basic if/end produces `Expr::If`.
- `if_else_lowers_with_else_branch` — explicit else branch is captured.
- `unless_negates_condition` — `unless cond` wraps the cond in `BuiltinCall("not", ...)`.
- `if_elsif_else_chain_nests_right` — elsif chain produces nested `Expr::If` in `else_branch.value`.
- `if_module_passes_sir_validator` — an `if … else … end` containing module passes `semantic_ir::validate`.

## [0.2.0] - 2026-05-20

### Added (Phase 6a — `def name(params) … end` method definitions)
- New `collect_def_statements` pre-pass hoists every `def_statement` from the program to a top-level `semantic_ir::Function` *before* the main-body lowerer runs.
- `lower_def_statement` translates the AST node into a `Function`:
  - The first `Name` token after the leading `def` keyword becomes the function name (the `def` keyword itself is skipped so the function isn't named `"def"`).
  - The optional `params` sub-rule's `Name` tokens become `Param`s.
  - The body is lowered using a *fresh* `declared_locals` set so the outer program's bindings don't leak in.  Params are pre-declared as locals (so `x = 2` inside `def f(x)` routes through `Stmt::Assign`) *and* tracked in a new `current_params` set so `VarRef` to them gets `Scope::Param` (the validator's expectation for parameters).
  - The tail expression (if any) is promoted to the body block's `value`; otherwise `value = NilLit`.
- `Module::manifest` now declares `Feature::DynamicTyping` — Ruby is dynamically typed, and the SIR validator requires this whenever a module produces untyped params or globals.
- `def_statement` nodes left behind in the program body lower to a no-op `Stmt::ExprStmt(NilLit)` so the SIR-level statement stream stays in sync with the source line count.

### Tests (+5 new, total 14)
- `def_lowers_to_top_level_function` — `def add(x, y); x + y; end` produces an `add` function with two params and a `+` builtin call body.
- `def_with_no_params_lowers_cleanly` — `def hello; end` produces a paramless function whose body value is `NilLit`.
- `def_does_not_leak_locals_to_outer_scope` — locals in a method body don't leak into the program-level scope (each gets its own first-occurrence `LetBinding`).
- `def_with_param_reassignment_routes_through_assign` — `def f(x); x = 2; end` re-binds via `Stmt::Assign` (the param is pre-declared as local).
- `module_with_def_passes_sir_validator` — a `def`-containing module passes `semantic_ir::validate`.

## [0.1.0] - 2026-05-20

### Added (Phase 5 — initial Ruby → SIR frontend)
- New crate `ruby-to-semantic-ir`.  Consumes `coding-adventures-ruby-parser`'s `GrammarASTNode` and emits a `semantic_ir::Module`.
- `compile_source(source, module_name) -> Result<Module, RubyLowerError>` — tokenize → parse → lower in one call.
- `compile(ast, module_name) -> Result<Module, RubyLowerError>` — lower an already-parsed AST.
- `RubyLowerError` — carries a human-readable message plus 1-based line/column drawn from the AST node's span.
- v0 lowering covers everything ruby-parser v0 parses:
  - Programs → synthesised `main` function whose body is the sequence of lowered statements.  Last bare-expression statement becomes the block's `value`; otherwise `value = NilLit`.
  - Assignments (`x = expr`) → `LetBinding` for first occurrence, `Assign` for subsequent re-bindings (scope is `Local`).
  - Method calls (`name(args...)`) → `BuiltinCall` for the known builtin set (`puts`, `print`, `p`, `gets`, `raise`); other names lower to `DirectCall` (placeholder; in v0 there are no user-defined functions, so backends will flag these as unresolved).
  - Expressions: integer literals, string literals, name references, binary `+ - * /` lowered to `BuiltinCall("+"...)` etc., parenthesised sub-expressions.
- Tests cover empty program, single assignment, multi-statement programs, arithmetic, method calls, re-assignment routing through `Stmt::Assign`, and tail-expression promotion to block `value`.
