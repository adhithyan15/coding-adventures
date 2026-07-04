# Changelog

All notable changes to the `sir-conformance` crate will be documented in this file.

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
