# NN21 Tiny Message Passing Labs

Status: Draft

NN21 defines a deterministic, language-neutral first graph-neural computation:
one synchronous message-passing round on a tiny undirected graph.

## Contract

Each saved undirected edge `{u, v}` expands into directed messages `u -> v` and
`v -> u`. With scalar features and shared parameters:

```text
message(source -> target) = message_weight * old_feature[source]
aggregate[target] = sum(incoming messages)
preactivation[target] = self_weight * old_feature[target] + aggregate[target] + bias
new_feature[target] = ReLU(preactivation[target])
```

All messages use the original feature snapshot. Updated values cannot influence
another node until the next round. Directed messages are ordered by target and
then source; this order is for trace portability, while the sum itself is
permutation invariant.

## Corpus and Validation

The corpus at `code/specs/fixtures/tiny-message-passing-v1` records original
features, canonical undirected edges, parameters, all directed messages, every
node inbox, aggregates, affine terms, ReLU inputs, and outputs. Duplicate or
unknown keys, non-finite numbers, self edges, duplicate undirected edges,
invalid indices, and mismatched traces are errors.

## Cross-Language and Rust-Core Direction

Implement the three-node scalar loop directly in each host language first. A
Rust core can later accept CSR/COO edge buffers and dense feature buffers,
expand or traverse directed adjacency, reduce messages deterministically, and
apply fused node updates. A stable C ABI plus WASM and idiomatic wrappers can
share the fast path. Trace mode must preserve source/target identities and
per-message values even when production kernels fuse the round.
