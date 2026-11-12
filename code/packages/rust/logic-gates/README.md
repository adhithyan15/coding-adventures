# logic-gates (Rust)

Logic gates and sequential circuits — the foundation of all digital hardware.

## What This Is

This crate implements the seven fundamental logic gates (AND, OR, NOT, XOR, NAND, NOR, XNOR), proves that all gates can be built from NAND alone (functional completeness), and provides multi-input variants. It also implements sequential logic elements (SR latch, D latch, D flip-flop, register, shift register, counter) that give circuits the ability to remember.

## How It Fits in the Stack

This is **Layer 1** — the absolute foundation. Every other crate in the accelerator stack depends on these gates. The `arithmetic` crate uses them to build adders and ALUs. The `clock` crate drives the sequential elements. Higher layers (cache, branch predictor, hazard detection) all ultimately reduce to combinations of these gates.

## Modules

- **`gates`** — 7 fundamental gates, NAND-derived gates, multi-input AND/OR/XOR
- **`combinational`** — mux2/mux4/mux_n, demux, decoder, encoder, priority encoder, tri-state
- **`sequential`** — SR latch, D latch, D flip-flop, register, shift register, counter

The stateless operations (`gates` and `combinational`) are also exported from
the crate root: `logic_gates::and_gate` is `logic_gates::gates::and_gate`.

## The bit domain

A wire is a `u8` that is **0 or 1**. Any other value is rejected by a **panic,
in every build profile** (it is never masked and never passed through), and so
is a slice of the wrong length, an `encoder` input that is not one-hot, and a
`decoder` or `demux` wider than `MAX_DECODER_WIDTH` = 16 inputs (its output has
2^n lines). Before this was pinned, a release build accepted a non-bit silently
in `tri_state`, `priority_encoder` and `encoder` (finding SEC-007) and a decoder
had no width bound (SEC-008); see [LG01](../../../specs/LG01-logic-gates-contract.md).

## Usage

```rust
use logic_gates::gates::{and_gate, or_gate, not_gate, xor_gate};
use logic_gates::sequential::{d_flip_flop, FlipFlopState};

// Basic gates
assert_eq!(and_gate(1, 1), 1);
assert_eq!(xor_gate(1, 0), 1);

// Flip-flop (edge-triggered memory)
let mut state = FlipFlopState::default();
d_flip_flop(1, 0, &mut state); // clock low: absorb
let (q, _) = d_flip_flop(1, 1, &mut state); // clock high: output
assert_eq!(q, 1);
```

## Verification

The stateless operations (22 of them) are specified by
[`code/specs/logic-gates.warrant`](../../../specs/logic-gates.warrant), a Warrant
v1 document, with the prose contract in
[LG01](../../../specs/LG01-logic-gates-contract.md). It is layer 1 of the Intel
4004 verification chain (VER27 section 4.15). `tests/generated/warrant_evidence.rs`
is *generated* from it and `evidence.bindings` (never edit it; `BUILD` fails if
it drifts) and runs against the real exports:

- **exhaustive suites, 3,770,123 cases in 88 claims:** every truth table of the
  seven primitive and four NAND-built gates against an oracle made of Warrant's
  own Boolean operators; every vector of width 2..=8 for the n-input gates;
  every input of `mux2`, `mux4` and `mux_n` (2, 4, 8 and 16 inputs, every data
  word and select); the decoder, demux and encoder at every width 1..=8; the
  priority encoder at 2, 4, 8 and 16 inputs; `tri_state`; and, for every
  operation, the rejection of every non-bit value and of every bad shape. That is
  proof by enumeration of the Rust function as compiled; it assumes the host
  language and toolchain, and the `transistors` crate is under test, not an
  oracle (LG01 "Trust boundary");
- **property and fuzz suites** for every claim, and **sabotage controls** that
  inject wrong results and a swallowed rejection, and must fail.

The claim names (`and_truth_table`, `mux_n_16_inputs_exhaustive`, ...) are
stable identifiers a later layer links to (LG01, "Contracts this layer
publishes"). `evidence/test-manifest.json` is written by that run;
`assurance_report.json` is derived from it (verdict `report-only`, audit
profile `boolean-logic`). What is *not* claimed: `sequential.rs` (the next
slice), timing, the step-bound proofs (**not live**: the WIR front end refuses
the sources; `evidence/proof-manifest.json` records why), and CWE-208 is
recorded as not applicable because every operand is public.

## Runtime capability conformance

The library is pure (`required_capabilities.json` declares no capabilities).
`cargo test -p logic-gates --test runtime_capability` proves it at run time on
Linux: every public stateless operation runs inside a seccomp/Landlock sandbox
derived from that manifest while an independent `strace` observer confirms
nothing but `write`/`exit_group` happens (VER08b). Set `RCG_STRICT=1` to fail
instead of reporting NOT-LIVE where the sandbox is unavailable.

## Running Tests

```bash
# Unit, doc and hand-written integration tests (the generated evidence target is separate)
cargo test -p logic-gates --lib
cargo test -p logic-gates --doc
cargo test -p logic-gates --test gates_test --test combinational_test --test sequential_test
# The bit domain, in both profiles (SEC-007)
cargo test -p logic-gates --test bit_domain --test decoder_width
cargo test -p logic-gates --release --test bit_domain --test decoder_width

# Regenerate the Warrant evidence harness after editing the .warrant document
cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- logic-gates

# Everything CI runs, including the evidence, sabotage and report checks: see BUILD
```
