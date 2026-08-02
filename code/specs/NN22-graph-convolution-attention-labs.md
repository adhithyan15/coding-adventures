# NN22 Graph Convolution and Attention Labs

Status: Draft

NN22 compares two ways to weight the same self-looped graph neighborhood.

For GCN, each source contribution uses:

```text
coefficient(source -> target) = 1 / sqrt(degree_target * degree_source)
contribution = coefficient * source_feature
output = ReLU(sum(contributions))
```

For the first GAT trace, the score is the transformed scalar source feature.
Scores are normalized only across the selected target's neighborhood with
stable softmax. The fixture records the row maximum, shifted scores,
exponentials, denominator, weights, weighted values, and output.

All neighborhoods include self and must be symmetric, unique, and index-valid.
The corpus at `code/specs/fixtures/graph-convolution-attention-v1` is strict and
language neutral.

## Cross-Language and Rust-Core Direction

Direct host implementations should reproduce the scalar oracle first. A Rust
core can later traverse CSR neighborhoods, precompute degree factors, execute
segmented stable softmax, and fuse weighted reductions. C ABI, WASM, and other
bindings should share buffers and retain an optional per-edge trace mode for
explaining optimized kernels.
