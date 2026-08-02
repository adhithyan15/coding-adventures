# Multi-Head Attention, Add, and Norm, by Hand

One attention head gives every token one way to ask what matters. A transformer
usually runs several heads in parallel so different learned projections can
look for different relationships. The results still have to rejoin one model
stream, preserve the original token, and keep feature scales controlled.

This lesson builds the smallest complete version: two tokens-wide features,
two scalar heads, a causal mask, an output projection, a residual addition, and
layer normalization. The language-neutral oracle is
[`00-two-head-causal-add-norm.json`](../../specs/fixtures/multi-head-attention-v1/labs/00-two-head-causal-add-norm.json),
and the contract is
[`NN14-multi-head-add-norm-labs.md`](../../specs/NN14-multi-head-add-norm-labs.md).

## Keep the same three tokens

The token embeddings are the value vectors from the previous attention lesson:

```text
x_red    = [2,0]
x_blue   = [0,1]
x_purple = [2,1]
```

The model width is two. Split that width across two heads, so each head produces
one scalar context:

```text
head count = 2
head width = model width / head count = 2 / 2 = 1
```

This is intentionally tiny. Real models use many coordinates per head, but the
control flow is the same.

## Give the heads different views

The horizontal head projects query and key with `[0.5,0]` and value with
`[1,0]`. It sees only the first coordinate. The vertical head uses `[0,1]` for
all three projections and sees only the second coordinate.

For the blue query:

```text
horizontal query = [0,1] dot [0.5,0] = 0
horizontal keys  = [1,0,1]
horizontal values = [2,0,2]

vertical query = [0,1] dot [0,1] = 1
vertical keys  = [0,1,1]
vertical values = [0,1,1]
```

The head width is one, so dividing scores by `sqrt(head width)` divides by one.
The scale step is still explicit because it stops being trivial as head width
grows.

## Work both blue-query heads

Blue may read red and itself, but not future purple.

The horizontal query is zero, so both allowed scores are zero:

```text
scores       = [0 * 1, 0 * 0, blocked]
             = [0, 0, blocked]
exponentials = [1, 1, 0]
weights      = [0.5, 0.5, 0]
context      = 0.5 * 2 + 0.5 * 0 + 0 * 2
             = 1
```

The vertical head finds a stronger match with blue itself:

```text
scores        = [1 * 0, 1 * 1, blocked]
              = [0, 1, blocked]
shifted       = [-1, 0, blocked]
exponentials  = [exp(-1), 1, 0]
denominator   = 0.367879 + 1 = 1.367879
weights       = [0.268941, 0.731059, 0]
context       = 0.268941 * 0 + 0.731059 * 1 + 0 * 1
              = 0.731059
```

The heads used the same token row and causal boundary, yet learned projections
made them attend differently. That is the reason to have multiple heads.

## Concatenate, then project

Put the head contexts beside each other in head order:

```text
concatenated blue context = [1, 0.731059]
```

Concatenation restores the model width. A learned output matrix can now mix
information across heads. This first lab uses the identity matrix so every
multiply remains easy to audit:

```text
W_output = [[1,0],
            [0,1]]

projected attention = [1 * 1 + 0.731059 * 0,
                       1 * 0 + 0.731059 * 1]
                    = [1, 0.731059]
```

Do not omit the output projection from an implementation just because the toy
matrix is the identity. The fixture records all four products.

## Add the residual path

The attention path should not replace the original token. Add the input back:

```text
attention path = [1, 0.731059]
identity path  = [0, 1]
residual sum   = [1 + 0, 0.731059 + 1]
               = [1, 1.731059]
```

The short route helps information and gradients cross a deep stack even when
the learned attention path is temporarily unhelpful.

## Normalize the two features

This lab uses post-attention layer normalization: attention, add residual, then
normalize. Statistics belong to one token row and span its feature coordinates.
They do not mix tokens.

```text
mean = (1 + 1.731059) / 2
     = 1.365529

centered = [1 - 1.365529, 1.731059 - 1.365529]
         = [-0.365529, 0.365529]

population variance
  = ((-0.365529)^2 + 0.365529^2) / 2
  = 0.133612

denominator = sqrt(variance + 0.00001)
            = 0.365543

normalized = centered / denominator
           = [-0.999963, 0.999963]
```

With `gamma = [1,1]` and `beta = [0,0]`, the affine step leaves those two
normalized coordinates unchanged. Gamma and beta are learned in a real block.

## Follow the complete block

For one token, the post-norm block is:

```text
embedding
  -> two independent causal attention heads
  -> concatenate head contexts
  -> output projection
  -> add the original embedding
  -> layer normalization
```

Some modern decoder stacks use pre-normalization, placing layer normalization
before attention. That is a different block order, not a different definition
of multi-head attention. NN14 pins post-normalization so every consumer checks
the same graph.

## Common implementation bugs

- Sharing one softmax denominator across heads instead of normalizing each head
  independently.
- Applying one set of query, key, and value projections to every head, which
  makes the heads identical by construction.
- Concatenating in a different order than the output projection expects.
- Adding the residual before restoring the model width.
- Normalizing across tokens instead of across one token's feature coordinates.
- Using sample variance and dividing by `width - 1`; layer normalization here
  uses population variance and divides by `width`.
- Adding epsilon after the square root rather than inside it.
- Forgetting the learned gamma and beta affine step.

## Try the complete attention block

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
keeps both heads visible at once, aligns their three weights and scalar value
contributions, then follows the concatenated result through projection,
residual addition, and normalization. Select another token or remove the
residual and normalization stages to see which part of the block caused each
number.

## Cross-language checkpoint

An NN14 consumer is conformant when it reproduces every scalar projection
product, per-head score, mask, stable-softmax intermediate, value contribution,
context, output-projection product, residual coordinate, normalization
statistic, affine product, and final output.

Implement the scalar loops directly in each language first. A Rust core may
later fuse projection, masked softmax, value reduction, residual addition, and
normalization for performance. Its stable C ABI should expose dimensions,
strides, masks, epsilon, caller-owned buffers, and optional trace buffers. The
fast path and the teaching path should share one numerical contract.
