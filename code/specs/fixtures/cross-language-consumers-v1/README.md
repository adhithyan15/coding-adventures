# Cross-language neural fixture consumers V1

This contract makes the smallest NN03 forward pass executable in three native
language families without translating the source fixture. Go, Ruby, and Rust
all read `neural-learning-v1/labs/00-weighted-neuron.json`, recompute the two
weighted contributions and bias, and emit the same closed JSON receipt shape.

Run the complete gate from the repository root:

```bash
python code/scripts/validate_cross_language_fixture_consumers.py
```

The orchestrator validates this catalog before it starts a child process. It
uses fixed executable names and source paths, substitutes the already-resolved
fixture path as one argument, never invokes a shell, and rejects timeouts,
oversized output, invalid UTF-8, explanatory stdout, and dishonest receipts.

Each lane is marked `native`. The future Rust C ABI tranche may add
`rust-core-binding` lanes, but it must preserve this fixture, receipt, and
tolerance contract so execution strategy cannot redefine correctness.
