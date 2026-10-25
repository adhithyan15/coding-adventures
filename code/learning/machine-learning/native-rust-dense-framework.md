# Native Rust dense training

A reusable dense model is a chain of small machines. Each machine multiplies
its input by an owned weight matrix, adds a bias, and applies an activation.
Rust's `DenseNetwork` now computes the forward pass and every derivative itself
above the existing Rust matrix arithmetic foundation. It does not call the
Python/TypeScript frameworks or a Rust-core binding.

## A calculation you can check on paper

For input `[1, 2]`, the shared two-layer linear example computes:

```text
hidden[0] = 1 * 0.5 + 2 * 1.0 = 2.5
hidden[1] = 1 * -0.5 + 2 * 0.5 = 0.5
prediction = 2.5 * 1.0 + 0.5 * -1.0 + 0.25 = 2.25
target = 1; squared error = (2.25 - 1)^2 = 1.5625
```

The derivative entering the output is `2 * (2.25 - 1) = 2.5`. Multiplying it by
the hidden activations gives output-weight gradients `[6.25, 1.25]`. Propagating
through the output weights gives hidden derivatives `[2.5, -2.5]`. The shared
fixture pins every remaining weight/bias gradient and the post-update loss
`0.5929` at learning rate `0.01`. The Rust tests read that fixture directly.

## Inspect and train

`gradients` returns an owned trace without changing the model. `train_step`
returns that same pre-update trace, then adopts the complete candidate model
only after its parameters, predictions, gradients, and loss pass finite-number
checks. A rejected candidate cannot leave only half the layers updated.

The [package README](../../packages/rust/neural-framework/README.md) shows the
public API. Try `[2, 4, 1]` with tanh/sigmoid for XOR, or unequal-width deep
models with several outputs. Central finite differences check every weight and
bias in those deep models, independently of the analytical derivative code.

```sh
cd code/packages/rust/neural-framework
cargo run -p neural-framework --example xor
python ../../../scripts/validate_neural_framework_rust.py
```

The validator executes native training, takes its checkpoint, supplies that
data to a second Rust process on stdin, and checks step 3001. The checkpoint is
closed JSON data, never a class name or executable program. Rust can load/save
the NN37 format, but verified three-language interchange is still the next
conformance tranche, not a result of this native-only validator.

See [NN38](../../specs/NN38-native-rust-dense-framework.md) and the
[framework roadmap](./FRAMEWORK-ROADMAP.md) for delivery boundaries. CNNs,
recurrent networks, transformers, GPU kernels, and large-model measurements
still need their own reusable training implementations and execution evidence.
