# Changelog

All notable changes to the `sir-conformance` crate will be documented in this file.

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
