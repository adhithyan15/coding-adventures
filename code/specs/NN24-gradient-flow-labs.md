# NN24 Gradient Flow Labs

Status: Draft

NN24 traces one scalar loss backward through four scalar layers. Each layer
records its saved forward input, weight, preactivation, activation, activation
derivative, and local Jacobian:

```text
local_jacobian = weight * activation_derivative
```

The backward pass starts with the half-squared-error derivative at the output.
For each layer in reverse order it records:

```text
dL/dz       = upstream_gradient * activation_derivative
dL/dinput   = dL/dz * weight
dL/dweight  = dL/dz * saved_input
```

V1 compares small-weight tanh, saturated tanh, unit-weight ReLU, and
large-weight ReLU. A chain-Jacobian magnitude below `0.1` is labeled vanishing;
a magnitude above `10` is labeled exploding. These thresholds describe the toy
experiment rather than a universal production rule.

The fixture also checks `dL/dinput` with a central finite difference using
epsilon `1e-6`.

## Cross-Language and Rust-Core Direction

Direct consumers should reproduce the four scalar chains before generalizing
to tensors. A Rust core can store saved activations and derivative masks in a
contiguous tape, traverse that tape in reverse, and fuse vector-Jacobian
products and parameter-gradient reductions. C ABI and WASM bindings should
expose both an optimized gradient buffer and an optional per-operation trace so
other languages can explain the same reverse pass.
