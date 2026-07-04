# sir-conformance

**Cross-backend golden conformance harness for the Semantic IR.**

Lowers real Ruby *source* through the actual frontend and runs the emitted
program through **every** backend's **real** toolchain, asserting the output is
identical — byte for byte — on each.

## Why

The Semantic IR (SIR) is a narrow waist: one Ruby frontend
(`ruby-to-semantic-ir`) lowers to a language-agnostic IR, and several backends
emit target source (`semantic-ir-to-{python,javascript,go,rust}`). The whole
point is **behavioural equivalence** — a program's result must not depend on
which backend emitted it.

Nothing enforced that end-to-end. Each backend's own `compile_and_run_*` test
*hand-builds* an SIR module and checks one feature in isolation, so a construct
the frontend emits but a backend never implemented can pass every unit test and
still crash at runtime. That is exactly what happened with the `case_eq` builtin
(Ruby's `===`): it was missing from three of five runtimes, so every `case`
program panicked on Go, Rust, and JavaScript while passing on Python — and no
test caught it (see the repo `lessons.md`).

This harness closes the gap. It is the first concrete slice of the conformance
matrix specified in
[`SIR21` §Provability](../../../specs/SIR21-type-system-and-integer-semantics.md).

## How it works

For every program in the corpus, and every backend:

1. **Lower** the Ruby source through `ruby_to_semantic_ir::compile_source`
   (the frontend runs once per program, as in production).
2. **Emit** target source via the backend's `compile`.
3. **Run** it through the real toolchain — `python3` (with the `sir-runtime-*`
   packages auto-discovered onto `PYTHONPATH`), `node`, `go run`, or
   `rustc` + execute.
4. **Assert** the normalised stdout equals the corpus's **reference** output.

Comparison is always against the reference (the value a Ruby interpreter would
print), never against another backend — so two backends agreeing on a *wrong*
answer cannot hide a bug.

A backend whose toolchain is absent on the host is **skipped** (logged), not
failed — mirroring the per-backend exec-test convention. The matrix test also
asserts that *at least one* backend actually ran, so a toolchain-less CI box
cannot report a hollow pass.

## The corpus

Small and behavioural (strings/integers only — booleans render differently
across backends, a separate formatting concern). Current programs:

| program | exercises | reference output |
|---------|-----------|------------------|
| `arithmetic` | operator precedence | `14` |
| `def_params` | method with params + implicit return | `5` |
| `tail_if` | implicit return of a trailing `if` | `10` |
| `tail_case` | implicit return of a trailing `case` (the `case_eq` oracle) | `A\nB\nC` |
| `string_concat` | polymorphic `+` | `abcd` |
| `oop_method` | `class` / `.new` / instance method | `woof` |
| `while_loop` | `while` + mutable accumulator | `10` |
| `array_length` | array literal + `.length` | `3` |
| `string_length` | `String#length` | `5` |
| `counter_state` | `@ivar` state across method calls | `2` |
| `mixin_include` | `module` mixed into a class via `include` | `hi` |
| `logical_ops` | short-circuit `||`/`&&` returning the operand | `a\nb\ny` |
| `multi_when` | multi-value `when 1, 2, 3` (folds through `or`) | `small\nbig` |
| `string_case` | `String#upcase`/`#downcase`/`#strip` (Ruby→JS renames) | `HELLO\nworld\nhi` |
| `seq_assign` | sequential assignment reading an earlier local (`b = a + 1`) | `5\n6\n11` |

Adding a program is one `Program { name, ruby, expected }` entry in
`tests/conformance.rs`.

### Gaps the corpus has surfaced (not yet in the suite)

Every added feature is a chance to catch a `case_eq`-style "works on some
backends" bug. Programs that hit an **unfixed** gap are kept *out* of the corpus
(with a pointer to `lessons.md`) so the suite stays green while the gap stays
visible. Currently tracked:

- **Array/hash index reads** (`a[i]`, `h[k]`) — a frontend PARSER-precedence
  gap: `a[1]` mis-parses as `a` followed by a bare `[1]` array literal
  (`puts a[1]` → `(puts a)[1]`; `puts(a[1])` fails to parse). Needs a grammar
  fix. (The *scoping* half — `x = a[1]` failing SIR validation — turned out to
  be the general sequential-assignment bug and is now fixed; see `seq_assign`.)

Fixed since (now IN the suite):
- The `or`/`and` builtin gap — `||`/`&&` and multi-value `when` threw
  `unknown builtin` on Go/Rust/JS; the emitters now emit the short-circuit form.
  Covered by `logical_ops` + `multi_when`.
- The JS `String#upcase`/`#downcase` rename gap — the JS runtime now aliases
  Ruby method names to their native equivalents before the allowlist check.
  Covered by `string_case`.

## Usage

```bash
# Run the whole matrix (skips backends whose toolchain is absent):
cargo test -p sir-conformance -- --nocapture

# The summary line reports coverage, e.g.:
#   conformance matrix: 6 corpus x 4 backends = 24 ran, 0 skipped
```

As a library, `run(&program, target) -> RunOutcome` and `lower(&program)` are
public so other harnesses (e.g. the eventual typed-integer conformance suite)
can reuse the emit-and-run plumbing.

## The reference oracle (`oracle` module, SIR21 §P2)

`oracle::eval(op, lhs, rhs, spec)` is the **independent authority** for integer
arithmetic: given operands, an op (`Add`/`Sub`/`Mul`), and an `IntSpec`
(`width` × `signed` × `overflow`), it returns the one observable `Outcome` the
[SIR21 faithfulness contract](../../../specs/SIR21-type-system-and-integer-semantics.md)
prescribes — `Value`, `Trapped`, `NoValue`, `Unspecified` (UB), or
`BeyondOracle` (a documented `i128` range limit, never a wrong number). It runs
no toolchain and no backend, so every backend can be measured against it rather
than against each other (which would let a shared bug hide). Examples it is
pinned to: `INT32_MAX + 1 == INT32_MIN` (i32 wrap), `0u32 - 1 == 4294967295`,
`10¹² · 10¹² == 10²⁴` (arbitrary). The differential runner and coverage gate
(T2b) will consume it to derive expected outputs instead of hand-typing them.

## The oracle-derived differential runner (`tests/arithmetic.rs`, SIR21 §P1)

For integer arithmetic the expected value is **not** hand-typed — it is computed
by the oracle and every backend is measured against *that*. Each case asks
`oracle::eval(op, lhs, rhs, arbitrary)` for the answer, generates the equivalent
Ruby (`puts(lhs op rhs)`), runs it through every backend's real toolchain, and
asserts the stdout matches byte-for-byte (9 cases × 4 backends = 36 runs, green
on the Phase-1 net).

**Frontier surfaced:** `10¹² * 10¹²` diverges — Python prints the exact `10²⁴`,
but JavaScript (`1e+24`, f64) and Go/Rust (`2003764205206896640`, 64-bit wrap)
do not. Only Python honours Ruby's arbitrary precision today; closing the gap is
the per-backend `Bignum` work (T4–T8), tracked by the `#[ignore]`d
`frontier_large_arbitrary_diverges` test.

## The coverage gate (SIR21 §P5)

The runner proves the cases it *has*; the coverage gate proves there are no
*gaps*. For every op the oracle can evaluate (`IntOp::ALL` — the set a frontend
could emit), it requires at least one conformance case that **passes on every
backend that accepts it**. An op that grows the oracle but gains no case fails
CI as a coverage error — the structural fix for the "emittable but never
implemented, and no test noticed" bug (the `case_eq` class). Two tests:
`coverage_gate_every_op_has_a_case` (toolchain-free) and
`coverage_gate_every_op_backend_cell_is_proven` (no accepted-but-untested cell).

## Where it fits

```
Ruby source ──► ruby-to-semantic-ir ──► SIR ──► semantic-ir-to-{py,js,go,rust} ──► run
                                                         ▲
                              sir-conformance drives this whole path and
                              checks all four outputs agree with the reference.
```
