# NN12: Three-Token Query, Key, and Value Labs

## Status

Draft specification for deterministic, language-neutral Q/K/V projection and
dot-product traces.

## Purpose

NN12 opens the first arithmetic stage of self-attention. Three two-dimensional
token embeddings are projected into queries, keys, and values. Every query is
then dotted with every key. Softmax, masking, and value mixing are deliberately
reserved for later specifications.

## Input and Projection Contract

V1 uses row vectors and right-multiplies each embedding by a `2 x 2` matrix:

```text
Q = X W_q
K = X W_k
V = X W_v
```

The tokens are `red = [1, 0]`, `blue = [0, 1]`, and
`purple = [1, 1]`. The matrices are:

```text
W_q = [[ 1, 0], [ 0, 1]]
W_k = [[ 1, 1], [-1, 1]]
W_v = [[ 2, 0], [ 0, 1]]
```

## Dot-Product Contract

For every query row `i` and key row `j`:

```text
raw_score[i, j] = Q[i] dot K[j]
scaled_score[i, j] = raw_score[i, j] / sqrt(key_dimension)
```

The key dimension is `2`. Scaling is included because it is the input to the
next attention stage, but NN12 does not normalize scores into probabilities.

The raw score matrix is:

```text
[[ 1, -1, 0],
 [ 1,  1, 2],
 [ 2,  0, 2]]
```

Each of its nine cells includes the two element-wise products that were summed.

## Value Boundary

Values are payload vectors. They do not participate in query-key score
calculation. NN12 projects and displays `V`, then stops. A later specification
will apply normalized attention weights to these rows.

## Fixture Layout

```text
code/specs/fixtures/attention-qkv-v1/
  schema.json
  labs/00-three-token-qkv.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields, incorrect
matrix shapes, unsupported operations, and trace values outside
`absolute_tolerance`.

## Conformance Levels

1. **Projection:** reproduce all Q, K, and V rows.
2. **Dot product:** reproduce all nine product pairs and raw scores.
3. **Scaling:** reproduce the `1 / sqrt(2)` score matrix.
4. **Trace:** expose the selected query, key, product pair, raw score, scaled
   score, and unused value payload to a learner or debugger.

## Cross-Language and Rust-Core Direction

Every language should implement V1 with explicit loops first. A Rust core can
later lower `XW` and `QK^T` to the repository's matrix backend, while trace mode
retains row identities and per-coordinate products.

A future C ABI should describe row-major shapes, strides, element type, and
caller-owned Q/K/V and score buffers. Projection matrices must remain separate
logical parameters even if a fast implementation packs them into one fused
allocation.
