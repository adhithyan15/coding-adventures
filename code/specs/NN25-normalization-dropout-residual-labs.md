# NN25 Normalization, Dropout, and Residual Labs

Status: Draft

NN25 compares four routes through one four-coordinate branch. Every route starts
with the same input and scalar branch weight:

```text
h[i] = branch_weight * input[i]
score = sum(upstream_gradient[i] * output[i])
```

The plain route returns `h`. The normalization route applies population layer
normalization across all four coordinates. V1 deliberately uses zero numerical
epsilon with a nonzero-variance fixture so the mean, variance, standard
deviation, normalized values, and backward formula remain hand-calculable.
Production implementations must accept and apply a caller-visible positive
epsilon.

The dropout route applies a pinned binary mask with inverted training-time
scaling:

```text
output[i] = h[i] * mask[i] / keep_probability
```

The fixture also records dropout's evaluation-time output and the exact
training-mask expectation. The residual route returns `input + h` and splits
its input gradient into the learned branch contribution and the identity skip
contribution.

Every route records its output, scalar score, gradient entering the shared
branch, skip gradient, input gradient, and branch-weight gradient. Central
finite differences independently check every input coordinate and the scalar
branch weight with epsilon `1e-6`.

## Cross-Language and Rust-Core Direction

Direct consumers should reproduce the four small vector traces before adding
tensor broadcasting or random mask generation. A Rust core can later expose
normalization statistics, caller-supplied dropout masks or seeds, residual
aliases, and vector-Jacobian products through a stable C ABI and WASM surface.
Optimized paths may fuse the operations, but an optional trace mode should keep
the saved mean, standard deviation, scaled mask, and split residual gradients
available to every host language.
