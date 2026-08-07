# NN35: Stable Rust C ABI

## Status

Implemented.

## Purpose

NN35 moves the NN03 weighted-neuron arithmetic behind a stable foreign-function
boundary. A C, Go, Ruby, Python, Swift, or other FFI-capable caller can reuse
the Rust execution core without depending on Rust layout or ownership rules.

## Contract

`code/specs/fixtures/neural-learning-rust-cabi-v1/` pins the ABI version,
function declarations, status table, ownership rules, hand calculation, and
failure probes. The public header is
`code/packages/rust/neural-learning-capi/include/neural_learning.h`.

The compute function accepts caller-owned binary64 input and weight buffers,
one bias, a caller-owned contribution buffer, and a caller-owned prediction
slot. Success writes the visible paper trace. Failure returns a closed status
code without writing either output.

## Stability rules

- Exported names are versioned when their signatures or semantics may evolve.
- Lengths and statuses use fixed-width integers.
- No Rust type, reference, string, vector, allocator, or unwinding crosses C.
- The version query returns `0x00010000`, meaning ABI major 1, minor 0.
- Panics are caught and mapped to status 6.
- Callers supply live, aligned buffers; mutable buffers may not overlap.

## Direct evidence

From the repository root:

```bash
python code/scripts/validate_neural_learning_rust_cabi.py
cargo test --manifest-path code/packages/rust/Cargo.toml -p neural-learning-capi
```

The validator compiles and dynamically loads the actual shared library. It
does not accept a Rust-only proxy as evidence for the exported calling
convention.

## Interactive trace

The Rust C ABI workbench reads the same catalog. Learners can select success or
one failure probe and inspect which boundary check runs, which status returns,
and whether caller-owned output bytes are allowed to change. The browser
recomputes catalog claims but does not claim to load native code.

## Cross-language direction

NN35 defines the core boundary only. The next tranche can add binding-backed
consumer lanes and track them beside the NN34 native baselines. Both lane types
must continue to read the same fixture and emit the same mathematical receipt.
