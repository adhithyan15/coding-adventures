# Softmax Attention and Causal Masking, by Hand

The previous lesson produced one row of match scores for every query. Those
numbers can be negative, positive, or larger than one. They are not yet
fractions and they do not yet mix any information.

This lesson turns each score row into attention weights, prevents left-to-right
language models from looking ahead, and finally uses the weights to blend value
vectors. The language-neutral oracle is
[`00-three-token-causal-softmax.json`](../../specs/fixtures/attention-softmax-v1/labs/00-three-token-causal-softmax.json),
and the contract is
[`NN13-attention-softmax-labs.md`](../../specs/NN13-attention-softmax-labs.md).

## Start with scaled scores and values

NN12 gave the tokens `red`, `blue`, and `purple` these scaled query-key scores:

```text
                         key
                 red       blue     purple
query red     0.707107  -0.707107   0
      blue    0.707107   0.707107   1.414214
      purple  1.414214   0          1.414214
```

The value payloads are:

```text
v_red    = [2,0]
v_blue   = [0,1]
v_purple = [2,1]
```

Softmax operates across one query row. It never mixes rows together.

## Why a causal mask comes before softmax

Imagine generating the sequence from left to right. When the model is working
on `blue`, `purple` is still in the future. Letting blue read purple would leak
the answer during training.

For query position `i` and key position `j`, the causal rule is:

```text
allowed when j <= i
```

That makes the three allowed-key rows:

```text
red query:     [yes, no,  no ]
blue query:    [yes, yes, no ]
purple query:  [yes, yes, yes]
```

Mathematically, implementations add negative infinity to blocked scores before
softmax. JSON cannot safely store infinity, so the NN13 trace records blocked
scores as `null`, then records their exponentials and weights as zero.

## Work the blue row by hand

The scaled blue-query row is:

```text
[0.707107, 0.707107, 1.414214]
```

First block the future purple key:

```text
[0.707107, 0.707107, blocked]
```

Softmax uses exponentials, but exponentiating very large scores directly can
overflow. Subtract the largest allowed score first. Both allowed scores are
equal, so:

```text
row maximum      = 0.707107
shifted scores   = [0, 0, blocked]
exponentials     = [exp(0), exp(0), 0]
                 = [1, 1, 0]
denominator      = 1 + 1 + 0 = 2
attention weight = [1/2, 1/2, 0/2]
                 = [0.5, 0.5, 0]
```

The weights are non-negative and sum to one. The future position receives
exactly zero.

## Use the weights to mix values

Multiply each key's value by its blue-query weight:

```text
red contribution    = 0.5 * [2,0] = [1,0]
blue contribution   = 0.5 * [0,1] = [0,0.5]
purple contribution = 0.0 * [2,1] = [0,0]
```

Add the three contribution rows:

```text
blue context = [1,0] + [0,0.5] + [0,0]
             = [1,0.5]
```

The blue output now contains a weighted blend of information it was allowed to
read. This is where the value vectors, deliberately unused in NN12's score
calculation, finally enter the computation.

## Compare with the mask removed

Without a causal mask, the blue query can read purple. Stable softmax gives:

```text
unmasked blue weights
  = [0.248255, 0.248255, 0.503490]

unmasked blue context
  = [1.503490, 0.751745]
```

The largest score sends slightly more than half the weight to the future purple
value. The arithmetic is valid for an encoder that may read the whole input,
but it violates autoregressive generation. The mask changes the model's
information boundary, not merely its presentation.

## Read the full causal matrix

The causal weight matrix is approximately:

```text
                         key
                 red       blue     purple
query red     1.000000   0          0
      blue    0.500000   0.500000   0
      purple  0.445808   0.108383   0.445808
```

Each row sums to one over allowed keys. The upper-right triangle is zero because
those keys are in the future. The last row matches its unmasked version because
no key lies to the right of the final token.

## Common implementation bugs

- Applying softmax before the mask, then setting blocked weights to zero without
  renormalizing the remaining positions.
- Masking the lower-left triangle and allowing tokens to read the future.
- Using one maximum or denominator for the entire matrix instead of one per
  query row.
- Exponentiating large raw scores without subtracting the allowed row maximum.
- Treating a blocked score as numeric zero; `exp(0) = 1`, so it would still
  receive weight.
- Letting a masked position contribute a value because its displayed weight
  looks close to zero instead of being exactly zero.
- Multiplying weights by keys rather than by values.

## Try the causal-softmax mixer

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
keeps the triangular weight matrix, selected row normalization, and weighted
value contributions aligned. Select any query and switch the causal mask off to
see exactly which weights and context coordinates change.

## Cross-language checkpoint

An NN13 consumer is conformant when it reproduces every allowed position, row
maximum, shifted score, exponential, denominator, weight, value contribution,
and context for both masked and unmasked modes.

Implement stable softmax and the weighted sum directly in each language first.
A Rust core may later fuse these kernels for performance, but a stable C ABI
should keep shapes, strides, masks, input buffers, caller-owned outputs, and
optional trace buffers explicit. Fusing work must not make the independently
checkable attention weights disappear.
