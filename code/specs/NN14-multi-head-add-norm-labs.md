# NN14: Multi-Head Attention, Residual, and Layer-Norm Labs

## Status

Draft specification for deterministic, language-neutral multi-head causal
attention, output projection, residual addition, and layer-normalization traces.

## Purpose

NN14 continues the three-token attention sequence without hiding the block
around attention. Two one-dimensional heads inspect different coordinates,
their contexts are concatenated, an output projection restores the model
width, the original token rejoins through a residual path, and layer
normalization produces the block output.

## Input Contract

V1 takes three two-coordinate token embeddings, exactly two attention heads,
a `2 x 2` output projection, and layer-normalization parameters. Each head has
two-coordinate query, key, and value projection vectors. The projected query,
key, and value for one head are scalars, so its scale divisor is
`sqrt(head_width) = 1`.

The checked-in lab deliberately uses simple projections:

- the `horizontal` head reads the first embedding coordinate;
- the `vertical` head reads the second embedding coordinate.

Different heads therefore produce different causal weight rows for the same
token.

## Multi-Head Contract

For head `h` and token row `i`:

```text
q_h[i] = dot(x[i], Wq_h)
k_h[j] = dot(x[j], Wk_h)
v_h[j] = dot(x[j], Wv_h)
score_h[i,j] = q_h[i] * k_h[j] / sqrt(head_width)
```

Apply the NN13 stable causal softmax independently inside every head, then
reduce its scalar values:

```text
context_h[i] = sum_j(weight_h[i,j] * v_h[j])
```

Concatenate head contexts in fixture order. For two scalar heads this restores
the two-coordinate model width:

```text
concat[i] = [context_horizontal[i], context_vertical[i]]
```

## Output Projection and Residual Contract

Treat vectors as rows. Output coordinate `o` is:

```text
attention[i,o] = sum_h(concat[i,h] * output_projection[h,o])
residual_sum[i,o] = embedding[i,o] + attention[i,o]
```

The V1 output projection is the identity matrix so the first lab exposes the
shape transition without adding arbitrary mixing. Consumers must still perform
and trace the matrix multiplication.

## Layer-Normalization Contract

Normalize each token row independently across its two model coordinates. V1
uses population variance:

```text
mean = sum(residual_sum) / model_width
centered[o] = residual_sum[o] - mean
variance = sum(centered[o]^2) / model_width
denominator = sqrt(variance + epsilon)
normalized[o] = centered[o] / denominator
output[o] = normalized[o] * gamma[o] + beta[o]
```

Epsilon lives inside the square root. The trace retains centered and squared
deviations so the variance can be checked by hand.

## Fixture Layout

```text
code/specs/fixtures/multi-head-attention-v1/
  schema.json
  labs/00-two-head-causal-add-norm.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields, incorrect
shapes, unsupported operations, and values outside `absolute_tolerance`.

## Conformance Levels

1. **Project:** reproduce every scalar query, key, value, and product.
2. **Attend:** reproduce both heads' causal score and stable-softmax traces.
3. **Join:** reproduce concatenation and every output-projection product.
4. **Add:** reproduce the attention-plus-residual coordinates.
5. **Normalize:** reproduce the mean, deviations, variance, denominator,
   affine products, and final output.

## Cross-Language and Rust-Core Direction

Every language should implement the tiny scalar loops first. A performant Rust
core can later expose batched projection, masked-softmax, value reduction,
output projection, residual addition, and layer-normalization kernels without
changing the fixture semantics.

A future C ABI should make batch size, token count, head count, head width,
model width, strides, masks, epsilon, and caller-owned outputs explicit. Trace
mode should optionally return per-head scores and weights plus pre-normalized
rows and normalization statistics. Fused execution is welcome, but it must not
erase the intermediate oracle needed by learning tools and cross-language
parity tests.
