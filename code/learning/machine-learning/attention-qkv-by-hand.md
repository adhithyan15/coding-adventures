# Query, Key, and Value Dot Products, by Hand

Attention starts by giving every token three different jobs:

- a **query** asks what this token is looking for;
- a **key** describes what this token can match; and
- a **value** carries the information that may be retrieved later.

This lesson projects three tiny token embeddings and computes all nine
query-key comparisons. It stops before softmax, masking, or value mixing. The
language-neutral oracle is
[`00-three-token-qkv.json`](../../specs/fixtures/attention-qkv-v1/labs/00-three-token-qkv.json),
and the contract is
[`NN12-attention-qkv-labs.md`](../../specs/NN12-attention-qkv-labs.md).

## Three two-number tokens

Use embeddings that are easy to picture:

```text
red    = [1, 0]
blue   = [0, 1]
purple = [1, 1]
```

Stacking them produces a `3 x 2` matrix `X`: three token rows and two features.

## One embedding, three projections

Each role has its own learned matrix:

```text
W_q = [[ 1, 0],    W_k = [[ 1, 1],    W_v = [[2, 0],
       [ 0, 1]]           [-1, 1]]           [0, 1]]
```

The row-vector convention is:

```text
Q = X W_q
K = X W_k
V = X W_v
```

For example, blue `[0,1]` becomes:

```text
blue query
  [0,1] W_q = [0*1 + 1*0, 0*0 + 1*1] = [0,1]

blue key
  [0,1] W_k = [0*1 + 1*(-1), 0*1 + 1*1] = [-1,1]

blue value
  [0,1] W_v = [0*2 + 1*0, 0*0 + 1*1] = [0,1]
```

All projected rows are:

| token | query `Q` | key `K` | value `V` |
| --- | --- | --- | --- |
| red | `[1,0]` | `[1,1]` | `[2,0]` |
| blue | `[0,1]` | `[-1,1]` | `[0,1]` |
| purple | `[1,1]` | `[0,2]` | `[2,1]` |

The original embedding is the same, but three parameter matrices give it three
different roles.

## A query compares with every key

Select the blue query `q_blue = [0,1]`. Dot it with each key:

```text
blue query · red key
  [0,1] · [1,1] = 0*1 + 1*1 = 1

blue query · blue key
  [0,1] · [-1,1] = 0*(-1) + 1*1 = 1

blue query · purple key
  [0,1] · [0,2] = 0*0 + 1*2 = 2
```

The blue query has raw scores `[1,1,2]`. A larger score means stronger
query-key alignment at this stage. It is not a probability.

Repeat that calculation for every query:

```text
                      key
              red  blue  purple
query red       1    -1       0
      blue      1     1       2
      purple    2     0       2
```

Rows answer “what does this query match?” Columns answer “which queries match
this key?” Swapping those axes changes the meaning.

## Why divide by the square root of the key size?

Transformer attention normally scales each raw score:

```text
scaled score = raw score / sqrt(d_k)
```

Here `d_k = 2`, so the blue row becomes:

```text
[1,1,2] / sqrt(2)
= [0.707106781187, 0.707106781187, 1.414213562373]
```

As vector dimensions grow, unscaled dot products can grow in magnitude. The
square-root factor keeps the next softmax stage from becoming unnecessarily
sharp. NN12 records both forms but applies neither softmax nor masking.

## Where values fit—and where they do not

Values are not used to calculate query-key scores:

```text
score = query dot key
```

The next attention stage will normalize scores and use them to mix value rows.
Keeping `V` out of the current arithmetic makes the role separation testable:
queries ask, keys match, values carry.

## Inspect one score cell

Every matrix cell retains its element-wise products. The purple-query,
blue-key score is:

```text
q_purple = [1,1]
k_blue   = [-1,1]

products = [1*(-1), 1*1] = [-1,1]
raw score = -1 + 1 = 0
```

The zero came from cancellation, not from two zero vectors. A trace that stores
only the final score would hide that distinction.

## Common implementation bugs

- Multiplying matrices in the wrong row/column orientation.
- Comparing queries with values instead of keys.
- Transposing the score matrix and silently swapping query and key identities.
- Dividing by `d_k` instead of `sqrt(d_k)`.
- Applying softmax too early and losing access to raw score arithmetic.
- Fusing Q/K/V projections without preserving their logical identities in
  traces and checkpoints.
- Treating a raw or scaled score as an attention probability.

## Try the score microscope

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
keeps the three projection rows aligned with a `3 x 3` score matrix. Select any
query row and key column to open its two products. Toggle square-root scaling to
see exactly which operation changes while values remain payload-only.

## Cross-language checkpoint

An NN12 consumer is conformant when it reproduces all Q/K/V rows, nine product
pairs, the raw score matrix, and the scaled score matrix.

Implement the scalar loops directly in every language first. A Rust core can
later lower `XW` and `QK^T` to matrix kernels, but a stable C ABI should keep
shapes, strides, row identities, projection ownership, and optional trace
buffers explicit. Fused storage is an optimization, not a reason to make Q, K,
and V indistinguishable outside Rust.
