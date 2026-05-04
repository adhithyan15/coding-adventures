# two-layer-network

Small TypeScript neural-network primitives for the first hidden-layer lessons.

The package models one hidden layer:

```text
X -> hidden weighted sum -> hidden activation -> output weighted sum -> output activation
```

That is enough to learn XOR, which a single linear layer cannot separate.

The package also exposes a neuron-level trace API:

```typescript
const trace = network.trace([[0, 1]], 0, [1]);
```

Each trace shows the weighted terms, bias, raw sum, activation output, and
optional backprop delta for every hidden and output neuron. This lets examples
treat a neuron like a tiny service that receives values, applies its weights,
and emits a transformed value for the next layer.
