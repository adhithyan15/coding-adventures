# Graph Convolution and Attention, by Hand

NN21 separated graph computation into message, aggregate, and update. This
lesson compares two rules for weighting the same inbox: graph convolution uses
the graph's degrees, while graph attention uses the current features.

The language-neutral contract is
[NN22](../../specs/NN22-graph-convolution-attention-labs.md), with the canonical
fixture in
[`00-three-node-gcn-gat.json`](../../specs/fixtures/graph-convolution-attention-v1/labs/00-three-node-gcn-gat.json).

## 1. Reuse the three-node path

```text
node 0 ----- node 1 ----- node 2
  1            2           -1
```

Add one self-loop to every node. The neighborhoods become:

```text
N(0) = [0, 1]       degree 2
N(1) = [0, 1, 2]    degree 3
N(2) = [1, 2]       degree 2
```

A self-loop lets each node retain its own feature inside the same aggregation
used for neighbors.

## 2. Graph convolution: weight by structure

For source `j` entering target `i`, use symmetric degree normalization:

```text
coefficient(j -> i) = 1 / sqrt(degree_i * degree_j)
```

For middle target `1`:

```text
source 0: 1/sqrt(3*2) *  1 =  0.408248
source 1: 1/sqrt(3*3) *  2 =  0.666667
source 2: 1/sqrt(3*2) * -1 = -0.408248
                                      --------
sum                                   0.666667
ReLU                                  0.666667
```

The positive and negative outside neighbors cancel because their degree
coefficients are equal. All outputs are:

```text
GCN -> [1.316497, 0.666667, 0.316497]
```

The word **convolution** here means applying one shared local mixing rule across
the graph. Unlike an image grid, each node can have a different neighbor count,
so degrees normalize the mixing.

## 3. Graph attention: weight by features

Keep the same neighborhood but score each source by its scalar feature:

```text
scores for target 1 = [1, 2, -1]
```

Use stable softmax. Subtract the row maximum `2` before exponentiating:

```text
shifted scores = [-1, 0, -3]
exponentials   = [0.367879, 1, 0.049787]
denominator    = 1.417667
weights        = [0.259496, 0.705385, 0.035119]
```

The weights sum to one. Mix source features with those weights:

```text
0.259496*1 + 0.705385*2 + 0.035119*(-1)
= 0.259496 + 1.410769 - 0.035119
= 1.635146
```

All outputs are:

```text
GAT -> [1.731059, 1.635146, 1.857722]
```

Attention gives the high-feature node `1` most of the weight. The graph did not
change; the data-dependent weighting rule did.

## 4. What the comparison teaches

| Property | GCN | GAT |
| --- | --- | --- |
| Neighborhood | self plus graph neighbors | self plus graph neighbors |
| Weight source | endpoint degrees | learned/data-dependent scores |
| Normalization | square-root degree factors | softmax inside each inbox |
| Sum-to-one | not required | yes |
| Changes when features change | contributions only | weights and contributions |

Our GAT score is intentionally simple. Real graph-attention layers first
transform features and learn a scoring function involving source and target.
The stable-softmax and weighted-sum mechanics remain the same.

## 5. Validate the deterministic corpus

```text
python code/scripts/validate_graph_convolution_attention_labs.py
```

The validator rejects unknown or duplicate keys, non-finite features,
neighborhoods without self-loops, asymmetry, invalid indices, trace mismatches,
and attention rows that do not sum to one.

## 6. Cross-language and Rust-core path

Implement these three scalar neighborhoods directly in every host language.
The fixture aligns every degree coefficient, softmax intermediate, weight, and
contribution.

A Rust core can traverse CSR neighborhoods, cache degree factors, run segmented
stable softmax, and fuse reductions. A C ABI and WASM layer can expose the same
buffers to other languages. Trace mode should remain available even when the
production path fuses edge scoring and aggregation.

## 7. Next experiment

Change node `2` from `-1` to `3`. GCN coefficients stay fixed because degrees
do not change. GAT weights shift immediately because the scores changed. That
counterfactual isolates the central difference between structural and
feature-dependent weighting.
