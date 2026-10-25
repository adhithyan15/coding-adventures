# Native Rust Neural Framework

NN40's shared checkpoint corpus is executed directly by the native package
tests: 3 accepted round-trip controls and 52 rejected malformed documents.
NN43 adds six numeric-null cases, including mixed arrays and later layers.
It supplements, rather than replaces, numerical and checkpoint-resume tests.

NN38 independently implements NN37 dense training above the existing Rust
`matrix` foundation. It is native Rust, not an FFI binding or a teaching-only
warm start. Configurable widths support linear, sigmoid, tanh, and ReLU layers,
full-network MSE gradients, deterministic initialization, and full-batch SGD.

```rust
use neural_framework::{Activation, DenseNetwork};

let mut model = DenseNetwork::seeded(&[2, 4, 1],
    &[Activation::Tanh, Activation::Sigmoid], 7)?;
let inputs = vec![vec![0.0, 1.0]];
let targets = vec![vec![1.0]];
let before_update = model.train_step(&inputs, &targets, 0.1)?;
let checkpoint = model.to_json()?;
let mut resumed = DenseNetwork::from_json(&checkpoint)?;
resumed.train_step(&inputs, &targets, 0.1)?;
# Ok::<(), neural_framework::Error>(())
```

Layers/checkpoints/traces own their arrays. Private model fields prevent a
caller from bypassing shape and finite-number checks. Updates are transactional:
even post-update prediction or loss overflow leaves all parameters and the
portable step counter unchanged. `fit` returns the initial loss followed by one
post-update loss per epoch; earlier successful steps remain if a later fails.

Checkpoints use the closed `coding-adventures-dense` V1 JSON format. Parsing
uses the repository's bounded-JSON foundation, rejects duplicate/unknown fields,
requires integer version/step metadata, and never loads executable code. The
JSON reader caps input at 4 MiB and 128 nesting levels; callers with larger
already-owned records may use `from_checkpoint`. Seeds are wrapping u32 LCG
streams for reproducibility, not cryptography. Training/model sizes are
caller-owned: this is a small CPU reference framework, not a hostile workload
scheduler or a measured large-model backend.

From this package, in the real Rust workspace:

```sh
cargo test -p neural-framework
cargo run -p neural-framework --example xor
python ../../../scripts/validate_neural_framework_rust.py
```

The example trains seeded XOR for exactly 3000 updates and emits checkpoint,
predictions, and losses as JSON. `--resume` reads a checkpoint on stdin and runs
one more update. The validator actually executes both processes and checks step
3001. Numerical tests read the shared paper/XOR fixture, differentiate every
parameter in deep multiple-output networks, and verify boundary and atomicity
contracts. Linux BUILD enforces 95% library line coverage with LLVM tarpaulin.

Sparse local validation may copy the exact package, matrix, bounded-JSON, and
fixture sources into a disposable minimal workspace. This does not replace the
real monorepo workspace validation in CI. No unrelated workspace members should
be materialized merely to run a small local oracle.

NN46's `python code/scripts/validate_neural_framework_interchange.py` from the
repository root executes twenty directed exchanges with Python, TypeScript, Go and Ruby,
including return-trip comparison at step 3002. Its selected reusable CI job must
pass before claiming required conformance delivery. Native save/reload/resume
alone is not proof of interchange. C ABI,
bindings, CNN/RNN/transformer training, GPU execution, and large-model performance
are not provided here. See [NN38](../../../specs/NN38-native-rust-dense-framework.md)
and the [framework roadmap](../../../learning/machine-learning/FRAMEWORK-ROADMAP.md).
