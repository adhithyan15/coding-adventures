# Rust neural fixture consumer

This Rust program reads the NN03 weighted-neuron JSON fixture, recomputes its
native binary64 forward pass, and writes one strict JSON receipt.

```bash
cargo run --quiet -- --fixture ../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json
cargo test
```

It is the systems-native lane in NN34. This version executes native Rust math;
it does not yet expose or consume the future stable Rust C ABI.
