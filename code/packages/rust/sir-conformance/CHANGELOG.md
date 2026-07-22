# Changelog

All notable changes to the `sir-conformance` crate will be documented in this file.

## [0.20.0] - Comparison-operator frontier

Adds `comparison_operators_match_ruby_on_every_backend`: `==`, `!=`, `<=`, `>=`
(plus `<`/`>` regression guards) over integers, a cross int/float pair, string
equality, AND string ordering, across every backend. Even `puts(1 == 1)` failed
on Python (`NameError`), JavaScript (`unknown builtin`), Go (`panic`) and Rust
(missing function) — only C and Ruby lowered the operator spellings. String
ordering is included because Go's comparisons gained a lexicographic string
path in this same change (they previously panicked on a string operand), so all
six backends now agree on `"a" < "b"`. Also covers COMPOSITE equality
(`[1,2] == [1,2]` structural, not reference) on the four backends that accept
the `sequences` feature (Python/JavaScript/Go/Rust) — the end-to-end proof that
the JavaScript `eq`→`valEq` fix agrees cross-backend; C and Ruby skip (no
`sequences` feature in their v0 backends).

## [0.19.0] - Exception-reflection frontier

Adds `exception_reflection_matches_ruby_on_every_backend`: `e.class`,
`e.is_a?` (self, ancestor, and a NEGATIVE case), `e.message`, and `puts e`
on a rescued exception, across every backend.

Each was wrong somewhere: **Rust** bound `e` to the message string (so
`e.class` was `String` and `is_a?` false), **Python** had no `SirError` case in
`class_of` (so `e.class` was `Object`), **`e.message` failed on ALL FOUR
backends**, and **JavaScript** printed `ArgumentError: boom` for `puts e` where
Ruby prints `boom` — that last one caught by this test as it was written.

Also pins a **bare** `raise ArgumentError` (no message): Ruby's default
`#message` is then the class name, which all four backends already produce —
but via four different mechanisms (Rust bakes it into `SirError.msg`,
JavaScript into the `SirError` constructor, Python into `args[0]`, Go decides
it at the `message` call site). Four independent spellings of one rule is
exactly what drifts, so the agreement is now guarded.

## [0.18.0] - `is_a?` / `case-when` frontier across the backends

Adds `is_a_and_case_when_match_ruby_on_every_backend` — the `is_a?` family and
`case x when SomeClass`, whose class argument is a bare CONSTANT in the source.

This previously compiled on Python alone: Go and Rust rejected the constant
reference at emit, and JavaScript blew up at run time. With the frontend now
lifting the constant to its name (ruby-to-semantic-ir 0.7.0) and Go implementing
the predicates (semantic-ir-to-go 0.35.0), all four running backends agree —
so it is a live cross-backend assertion, not a per-backend guard.

## [0.17.0] - Reflection frontier goes cross-backend (Rust arm closed)

`tests/reflection.rs` grows from a per-backend guard into a live **all-backend**
assertion, `class_reflection_matches_ruby_on_every_backend`: every backend that
runs a case must produce the same Ruby class-name string, failing by name the
day one diverges.

That was gated on the Rust arm, which previously **panicked at runtime**
(exit 101) because `.class` was undispatched; `semantic-ir-to-rust` 0.37.0
implements it. Adds `rust_class_reflection_is_ruby_faithful` alongside the
existing JavaScript and Go granular guards. Python, Go, JavaScript and Rust all
answer; C does not yet emit reflection and skips.

## [0.16.0] - Type-reflection conformance: `.class` across the backends

Adds `tests/reflection.rs` — every backend must reproduce Ruby's class-NAME
strings (`7.class == Integer`, `7.0.class == Float`, `nil.class == NilClass`,
…). A **per-backend** guard (the `division.rs` `python_division_is_ruby_floor_faithful`
style) rather than an all-backend frontier, because the arms are not yet all
closed:

- JavaScript — **closed by this change** (previously raised `NoMethodError`;
  its `Integer`/`Float` split is representable only via its tagged floats).
- Go — already closed (`_sir_ruby_class_name`); pinned against regression.
- Rust — **`.class` panics at runtime (exit 101)**, tracked separately.
- C — not yet emitted (skips).

Grows into a cross-backend assertion once the Rust arm is implemented.

## [0.15.0] - Float-division frontier: `Float#/` true-divides on every backend

Adds `float_division_true_divides_on_every_backend` — the float half of the
polymorphic-`/` frontier. `Float#/` TRUE-divides (`7.0 / 2 == 3.5`) and an
integral float result still prints its `.0` (`6.0 / 2 == 3.0`), and every
backend must reproduce Ruby's `Float#to_s`. The JS backend gained this via its
tagged-float substrate (semantic-ir-to-javascript 0.39.0 — its numbers are all
f64, so integral floats were previously indistinguishable from integers and
wrongly floored); the tagged-value backends (Rust/Go/C) and Python/Ruby already
carried the distinction. Complements the existing integer-floor frontier.

## [0.14.0] - Division frontier: C arm fully closed (SIR21 §E3)

The C backend now lowers the unary-minus `neg` builtin (semantic-ir-to-c 0.3.0),
so `division_matches_ruby_floor_on_every_backend` now **asserts** C's negative
cases instead of skipping them — the C emitter previously reported `neg` as an
unsupported builtin, so `run_c` returned `Skipped` for every negative literal.
All six backends (Python, JavaScript, Go, Rust, Ruby, C) now floor every sign
combination end-to-end. Module docs and the test doc-comment updated to record
the C arm as closed rather than tracked.

## [0.13.0] - Division frontier CLOSED on every backend (SIR21 §E3)

With Rust, Go and JavaScript now flooring integer division (their runtime
`divide` helpers, this release's sibling backend bumps) alongside Python — and
Ruby flooring natively (it transpiles to Ruby) and C already flooring
(`_sir_ifloordiv`) — `division_matches_ruby_floor_on_every_backend` is **no
longer `#[ignore]`d**. It is a live conformance assertion that emits
`puts(lhs / rhs)`, runs it through every backend's real toolchain, and asserts
the output equals the oracle's `DivOp::Floor` on all sign combinations. It fails
(naming the backend) if any backend regresses to truncation or true-division on
integer operands. `Skipped` outcomes are not asserted: the C *emitter* does not
yet lower unary `neg`, so its negative cases skip (positive division is
asserted), and Ruby skips without a `ruby` toolchain. Verified locally: Python,
JavaScript, Go, Rust and Ruby all floor every sign combination; C floors the
positive cases.
`python_division_is_ruby_floor_faithful` remains as a granular per-backend guard;
module docs updated to record all four arms closed.

Un-ignoring the frontier also surfaced (and this release's Go/Rust bumps fixed)
a second bug: the negative test cases (`-7 / 2`) crashed Go and Rust with
`unknown builtin: neg` — unary minus (`-x`) lowers to a `neg` builtin those two
runtimes never implemented. That is the "Go/Rust crash on negatives" the
frontier doc long recorded; it was never a division bug at all. The frontier is
what forced it to the surface, since floor and truncation only *differ* on
negative operands.

## [0.11.0] - Division frontier: Python arm closed (SIR21 §E3)

### Added — `python_division_is_ruby_floor_faithful` (non-ignored)

The Python backend's `Integer#/` now floors toward −∞ (fixed in
`coding-adventures-sir-runtime-core` 0.1.9), so it reproduces Ruby's `/` on every
sign combination. This new **non-ignored** test in `tests/division.rs` is the
live regression guard: it emits `puts(lhs / rhs)`, runs it through `python3`, and
asserts the output equals the oracle's `DivOp::Floor` — it would go red the day
the Python `div` helper is reverted to truncation. Runs whenever `python3` and
the `sir-runtime-*` packages are present; inert (not falsely green) otherwise.

### Changed

The all-backend `division_matches_ruby_floor_on_every_backend` stays `#[ignore]`d
— Python passes now, but JavaScript still true-divides and Go/Rust crash on the
negative path, so the *every-backend* promise isn't met yet. Its `#[ignore]`
message and the module docs were updated to reflect that the Python arm is
closed. The frontier flips to a passing assertion once the remaining three
backends are made floor-faithful.

## [0.10.0] - Division frontier captured (oracle-judged), §E3

### Added — `tests/division.rs`

Probing division through the current pipeline (with the T3b-1 `DivOp` oracle as
judge) surfaced a **confirmed, multi-way cross-backend divergence** — the exact
"one overloaded `divide` that does the Ruby thing on Tuesdays" bug §E3 exists to
kill. Ruby's integer `/` floors (`-7 / 2 == -4`); today:

- **Python** prints `-3` — its runtime `div` truncates toward zero (`int(a/b)`),
  and is deliberately documented/unit-tested that way ("to match SIR semantics").
- **JavaScript** prints `-3.5` — true (float) division, not integer at all
  (even `7 / 2` prints `3.5`).
- **Go / Rust** — the emitted program **crashes** on the negative path.

So besides bugs, there is a genuine **semantics conflict** to resolve: the Python
runtime's truncating `div` vs. the SIR21 oracle's (and Ruby's) floor `/`. That is
exactly why §E3 splits `/` into explicit `div_floor` / `div_trunc`; resolving it
(flip vs. split, plus `Integer#/` floors while `Float#/` true-divides) is a
design decision tracked separately, not made here.

This slice **captures** the frontier so it is oracle-judged and tracked (the way
`arithmetic.rs` captures the 10²⁴ bignum frontier):

- `oracle_floor_matches_ruby_integer_division` — a toolchain-free control
  pinning the oracle's floor expectations to Ruby (`7/2=3`, `-7/2=-4`, …).
- `division_matches_ruby_floor_on_every_backend` — the cross-backend frontier,
  `#[ignore]`d so the suite stays green; verified to genuinely fail today
  (`javascript computed 7/2 = 3.5, Ruby floor = 3`) and it flips to a passing
  assertion once division is made floor-faithful everywhere.

## [0.9.0] - SIR21 T3b (T3b-1) — division reference semantics (floor vs trunc, §E3)

### Added — `oracle::DivOp` + `Outcome::DivByZero`

First slice of the milestone-T3 **division split**: the reference semantics for
integer division, so the oracle can later judge a frontend/backend that emits
explicit `div_floor` / `div_trunc` instead of one overloaded `_sir_divide`
(SIR21 §E3 "division semantics are explicit, not guessed").

- `DivOp { Floor, Trunc }` with `ALL`, `name()` (canonical IR op names
  `div_floor` / `div_trunc`), and `eval(lhs, rhs) -> Outcome` at arbitrary
  precision. `Floor` rounds toward −∞ (Ruby `/`, Python `//`); `Trunc` rounds
  toward 0 (C / Rust / Go / Java `/`). They agree on positive operands and exact
  division, and differ exactly when a negative quotient has a non-zero
  remainder — the canonical `−7 / 2` = `−4` (floor) vs `−3` (trunc).
- New `Outcome::DivByZero` — division by zero (a faithful backend *raises*, so
  the harness asserts failure, never a value). The one overflow case
  `i128::MIN / -1` reports `BeyondOracle` rather than panicking.
- `div_true` (Python `/`, `7 / 2 == 3.5`) is intentionally **not** modelled: it
  yields a float, so it belongs to a future float oracle. Documented in `DivOp`.
- 6 unit tests pin floor-vs-trunc across all four sign combinations, exact
  division agreement, div-by-zero, and the `MIN / -1` overflow.

Pure and additive: `DivOp` is a new type kept *out* of `IntOp::ALL`, so the
T2c coverage gate is untouched and the existing `_sir_divide` path is unchanged
(no frontend emits these ops yet). Wiring the frontend to emit `div_floor` and
each backend to lower it — and running division conformance cases — is the next
slice (T3b-2).

## [0.8.0] - SIR21 T2c — the coverage gate (§P5)

### Added — structural coverage guard over the integer-op × backend grid

The third slice of T2. Where the differential runner proves the cases it *has*,
the **coverage gate** proves there are no *gaps*: for every operation the oracle
can evaluate (`IntOp::ALL` — the set a frontend could emit), at least one
conformance case must exist and pass on every backend that accepts it. This is
the structural fix for the "a construct is emittable but a backend never
implemented it, and no test noticed" class of bug — the same shape as the
`case_eq` gap (missing from three runtimes). An op that grows the oracle but
gains no case now fails CI as a *coverage* error, not silently (SIR21 §P5).

- `oracle::IntOp::ALL` enumerates every op (with a compile-time exhaustiveness
  nudge in the tests), plus `IntOp::tag()` for stable coverage keys. Adding an
  `IntOp` variant forces adding it to `ALL`, which forces a conformance case.
- `coverage_gate_every_op_has_a_case` (toolchain-free): every op in `ALL` is
  exercised by ≥1 arithmetic case — runs even on a host with no backends.
- `coverage_gate_every_op_backend_cell_is_proven` (toolchain-gated): for each
  op, a representative case runs on every available backend and matches the
  oracle; and every backend that ran *any* op must have proven *all* of them —
  no accepted-but-untested `(op, backend)` cell.

This is the arithmetic slice of the gate (op × backend); extending it to the
full `SirType`/feature surface of the golden corpus is a later slice (T2d).

## [0.7.0] - SIR21 T2b — the oracle-derived differential runner (§P1)

### Added — `tests/arithmetic.rs`, oracle-derived expectations

The second slice of SIR21 T2. Wires the T2a reference oracle into a **differential
runner**: for each integer-arithmetic case it asks the oracle for the answer,
generates the equivalent Ruby (`puts(lhs op rhs)`), runs it through **every**
backend's real toolchain, and asserts each backend's stdout equals the oracle's
answer byte-for-byte. **No expected value is hand-typed** — the oracle is the
single source of truth, so a disagreeing backend is localised as `(case, backend)`
and can't hide behind another backend sharing its bug (§P1).

- 9 arithmetic cases × 4 backends = 36 real toolchain runs, all green on the
  **Phase-1 net** (values within the range every backend represents exactly
  today — Python is bignum, JS `Number` is f64-exact to 2⁵³, Go/Rust are 64-bit).
- A toolchain-free `every_case_has_a_well_formed_oracle_expectation` guard
  cross-checks the oracle against native `i128` math and runs even with no
  backends present.

### Confirmed — a real cross-backend faithfulness gap (the bignum frontier)

Probing beyond the Phase-1 net surfaced the harness's first genuine finding:
`10¹² * 10¹²` (= 10²⁴) prints
- `1000000000000000000000000` on **Python** (correct — Ruby ints are arbitrary
  precision),
- `1e+24` on **JavaScript** (f64 precision loss),
- `2003764205206896640` on **Go** and **Rust** (64-bit wraparound).

Only Python honours Ruby's arbitrary-precision semantics — precisely the "the
type is the semantics" problem SIR21 exists to fix. It is captured as the
`#[ignore]`d `frontier_large_arbitrary_diverges` test (executable documentation
that flips to a passing assertion once the per-backend `Bignum` lowering, T4–T8,
lands).

### Refactored

`lib.rs` gained source-level primitives `lower_source(name, src)` /
`run_source(name, src, target)` (the internals now key off a `&str` name instead
of a `&'static Program`), so runtime-generated programs can drive the harness;
the static `Program`-based `lower`/`run` are thin wrappers, unchanged for callers.

## [0.6.0] - SIR21 T2a — the integer reference oracle (§P2)

### Added — `oracle` module (pure, toolchain-free)

The first slice of SIR21 milestone **T2**: a **reference oracle** for integer
semantics — the independent authority every backend is later measured against,
so two backends that share a bug can no longer agree with each other and hide it
(SIR21 §P2). It runs no toolchain and touches no backend.

- `oracle::eval(op, lhs, rhs, spec) -> Outcome` computes the observable result
  of a binary integer op (`Add`/`Sub`/`Mul`) under an `IntSpec`'s
  `(width, signed, overflow)`, and `oracle::reduce(exact, spec)` applies the
  width + overflow policy to an already-exact value (decoupled from any op).
- `Outcome` = `Value(i128)` · `Trapped` (overflow-trap raises) · `NoValue`
  (checked → none) · `Unspecified` (UB — oracle asserts nothing) ·
  `BeyondOracle` (exact result / `W128` modulus exceeds the oracle's `i128`
  working range — a documented, honest limit rather than a wrong number).
- Overflow policy per the SIR21 faithfulness table: `Wrap` → mod 2ⁿ re-centred
  for signedness, `Saturate` → clamp, `Trap` → raise, `Checked` → none,
  `Undefined` → unspecified, `Arbitrary` → grows.
- 13 unit tests pin the canonical constants (`INT32_MAX+1 == INT32_MIN`,
  `0u32-1 == 4294967295`, `255+1 (u8) == 0`, `10¹²·10¹² == 10²⁴`), every
  overflow mode, in-range passthrough, and the honest `W128`/`>i128` limits.

This is the semantic core the differential runner (P1) and coverage gate (P5)
will consume in T2b to derive expected outputs programmatically instead of
hand-typing them. The existing Ruby-source golden matrix is unchanged.

## [0.5.0] - 2026-07-03

### Added — `seq_assign` (14 -> 15 programs)

Sequential local assignments that read an earlier local (`a = 5; b = a + 1;
c = b + a`) — previously rejected by the SIR validator as a parallel-`let`
violation, now fixed in the frontend. Verified across all four backends.

## [0.4.0] - 2026-07-03

### Added — `string_case` (13 -> 14 programs)

The JS String-method rename gap this corpus tracked is fixed, so a `string_case`
program joins the suite: `"hello".upcase` -> `HELLO`, `"WORLD".downcase` ->
`world`, `"  hi  ".strip` -> `hi`. Verified 14 corpus x 4 backends = 56 runs,
0 skipped, all agree.

## [0.3.0] - 2026-07-03

### Added — `logical_ops` + `multi_when` (11 -> 13 programs)

The `or`/`and` cross-backend gap this corpus surfaced is now fixed (JS/Go/Rust
emitters implement short-circuit `||`/`&&`), so two programs join the suite:

- `logical_ops` — `"a" || "b"` -> `a`, `nil || "b"` -> `b`, `"x" && "y"` -> `y`
  (short-circuit, returning the deciding operand).
- `multi_when` — a multi-value `when 1, 2, 3` (folds through the `or` builtin),
  re-enabled from the tracked-gap list.

Verified: **13 corpus x 4 backends = 52 runs, 0 skipped, all agree.**

## [0.2.0] - 2026-07-02

### Added — corpus expansion (6 -> 11 programs) + three latent gaps found

Broadened the conformance corpus to exercise loops, arrays, string methods,
instance state, and mixins. Five new programs, all green on every backend:

- `while_loop` — a `while` with a mutable accumulator (0+1+2+3+4 = 10).
- `array_length` — array literal + `.length`.
- `string_length` — `String#length`.
- `counter_state` — `@ivar` state mutated across method calls (2).
- `mixin_include` — a `module` mixed into a class via `include`.

Verified locally: **11 corpus x 4 backends = 44 runs, 0 skipped, all agree.**

### Found (documented, kept OUT of the corpus until fixed; see lessons.md)

Expanding coverage immediately surfaced three real `case_eq`-style gaps — caught
only because each backend is compared to the reference, not to backend-consensus:

- **Frontend array/hash index** — `puts a[1]` mis-parses as `(puts a)[1]`,
  `puts(a[1])` fails to parse, and `x = a[1]` fails SIR validation (index base
  mis-scoped). Index-based programs are therefore excluded for now.
- **JS string-method rename** — the JS backend renames Ruby method names to
  native JS at emit time but is missing `upcase`/`downcase` → `NoMethodError`
  on JS only.
- **JS `or` builtin** — a multi-value `when 1, 2, 3` lowers to a
  `BuiltinCall("or", …)` the JS runtime doesn't implement → `unknown builtin`.

Each is tracked for a separate focused fix; adding a program that can't pass
would only mask them.

## [0.1.0] - 2026-07-02

### Added — cross-backend golden conformance harness

The first test that drives real Ruby **source** all the way to Python,
JavaScript, Go, and Rust and proves the four agree — closing the long-standing
gap where no single test exercised Ruby → {Go, Rust} end-to-end, and where a
frontend-emitted construct missing from a backend runtime (like the `case_eq`
builtin) could pass every per-backend unit test yet crash at runtime.

- **`Program` corpus + reference oracle.** Each program is Ruby source paired
  with the exact stdout a Ruby interpreter would produce; every backend is
  measured against that reference, never against another backend.
- **`run(program, target)` / `lower(program)`.** Public plumbing that lowers a
  program through `ruby_to_semantic_ir`, emits it via a backend, and runs it
  through the real toolchain (`python3` with the `sir-runtime-*` packages
  auto-discovered onto `PYTHONPATH`; `node`; `go run`; `rustc` + execute),
  returning normalised stdout. Reusable by future conformance suites (e.g. the
  SIR21 typed-integer matrix).
- **Graceful skip + hollow-pass guard.** A backend whose toolchain is absent is
  skipped (logged), not failed; the matrix test asserts at least one backend
  actually ran so a toolchain-less host cannot report a hollow pass. The Go
  toolchain is probed with `go version` (not `--version`, which errors).
- **Initial corpus (6 programs):** operator precedence, method-with-params
  implicit return, trailing-`if` return, trailing-`case` return (the `case_eq`
  regression oracle), string concatenation, and a `class`/`.new`/method call.
  Verified locally: 6 corpus × 4 backends = 24 runs, 0 skipped, all agree.

This is the first slice of
[`SIR21` §Provability](../../../specs/SIR21-type-system-and-integer-semantics.md)'s
conformance matrix (P1).
