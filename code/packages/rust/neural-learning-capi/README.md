# neural-learning-capi

This crate exposes the first neural-learning Rust core through a stable C ABI.
It computes one identity-activated weighted neuron while preserving the NN03
paper trace: every weighted contribution and the final prediction are returned
to the caller.

The ABI is deliberately small:

- fixed-width integer lengths and status codes;
- caller-owned input and output buffers;
- no Rust allocation crosses the boundary;
- no Rust type or layout appears in the header;
- every exported operation catches panics before they can unwind into C;
- a version query and versioned function name make evolution explicit.

The public declarations are in `include/neural_learning.h`. On success,
`neural_learning_weighted_sum_f64_v1` returns `NEURAL_LEARNING_OK`, writes
exactly `input_count` contributions, and writes one prediction. On failure it
returns a closed status code and does not write either output.

Callers must provide live, aligned buffers for the declared lengths. Input
buffers may overlap each other because they are read-only. Output buffers must
not overlap the inputs or each other. The function rejects detectable overlap,
null pointers, zero or excessive lengths, short output buffers, misalignment,
and non-finite arithmetic.

From `code/packages/rust`:

```bash
cargo test -p neural-learning-capi
cargo build -p neural-learning-capi --release
```

The build produces a shared library, static library, and Rust library so both
foreign consumers and direct Rust tests exercise the same implementation.
