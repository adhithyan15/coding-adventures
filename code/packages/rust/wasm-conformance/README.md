# wasm-conformance

Runs the official [WebAssembly/testsuite](https://github.com/WebAssembly/testsuite)'s
`.wast` scripts against this repo's `wasm-execution` interpreter (via
`wasm-runtime` and `wasm-wast-parser`) and reports a real, git-pinned
conformance baseline. Phase A of the `wasm-execution`-as-good-as-wasmtime
arc; see [`code/specs/W05-wasm-conformance-harness.md`](../../../specs/W05-wasm-conformance-harness.md)
for the full design.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo,
a ground-up implementation of the computing stack from transistors to operating systems.

## Why this exists

Every claim this repo has made about `wasm-execution`'s WASM coverage —
"the complete WASM instruction set", "~182 instruction handlers" — was
self-reported by the crate's own doc comments, never independently
verified against the real testsuite or a real engine. This crate is that
independent measurement.

## Where it fits in the stack

```
wasm-wast-parser  ←── .wat/.wast text -> WasmModule + script directives
wasm-runtime      ←── parse -> validate -> instantiate -> execute
wasm-validator    ←── structural (not instruction-level) validation
wasm-conformance  ←── THIS CRATE: runs .wast scripts, grades outcomes
```

## Usage

```bash
# The actual day-to-day deliverable: where does wasm-execution stand today?
cargo run --bin wasm_conformance_report -p wasm-conformance

# Catches drift from the committed baseline (regression OR improvement).
cargo test -p wasm-conformance

# After a deliberate, reviewed change that moves the numbers:
cargo run --bin wasm_conformance_report -p wasm-conformance -- --write-baseline
cargo test -p wasm-conformance   # confirm it's green again
```

## How grading works

Every directive in a `.wast` script is graded one of four ways:

- **`Pass`** — the interpreter did exactly what the spec says it should.
- **`Fail`** — the interpreter got a real answer wrong. This is a genuine
  bug report.
- **`Trap`** — an unexpected trap where a normal result was expected (or
  vice versa for `assert_trap`).
- **`NotYetSupported`** — grading this directive correctly needs a
  capability this repo's WASM stack doesn't have yet, and claiming `Fail`
  would misattribute the gap. Three specific cases:
  - `assert_invalid` needs an instruction-level type-checker
    `wasm-validator` doesn't have (`W02`'s own spec already designs it).
  - `assert_unlinkable` needs `WasmRuntime::instantiate` to actually be
    able to fail on an unresolved import — today it always falls back to
    a default value.
  - `assert_exhaustion` is **never executed at all** — `wasm-execution`
    has no call-depth guard, so the deliberately unbounded recursion these
    cases trigger would overflow the real host stack (an uncatchable
    process abort), not produce a gradeable trap.

Every `NotYetSupported` case is expected to flip to a real `Pass`/`Fail`
once the missing capability ships, with **zero changes to this harness**.

A handful of `.wast` files in the vendored slice fail to parse entirely —
tracked separately from directive-level outcomes as `parse_failures` in
the report, not folded into a misleading all-zero tally. These are
legitimate, out-of-scope gaps (`select`'s explicit `(result T)`-annotated
opcode, the reference-types proposal's extended/generalized `elem`-segment
syntax, concrete `(ref null $t)` heap types, and a named global
inline-import shorthand — WASM17's spec, `code/specs/
W08-wasm-funcref-externref.md`, tracks exactly which reference-types
pieces are and aren't in scope) — see this crate's own report output and
`code/specs/W05-wasm-conformance-harness.md` section 6 for the exact,
current breakdown.

## The golden baseline

`tests/fixtures/testsuite-status.json` is a checked-in snapshot of every
vendored file's per-directive-kind tallies. `tests/testsuite_conformance.rs`
runs the corpus fresh on every `cargo test` and fails loudly — naming the
exact file and kind that changed — on any drift from that snapshot,
improvement or regression. This means every change to the number is a
deliberate, reviewed commit, never silent.

## Vendored corpus

`tests/fixtures/testsuite/` vendors 48 `.wast` files from the official
testsuite at a pinned commit SHA (never `main`, which interleaves
post-MVP proposal files with MVP-core ones). See
`tests/fixtures/testsuite/NOTICE` for the exact pin and
`tests/fixtures/fetch_testsuite.py` to re-fetch (a no-op unless the pin is
bumped on purpose).
