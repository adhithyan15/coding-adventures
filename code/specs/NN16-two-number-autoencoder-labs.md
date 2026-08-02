# NN16: Two-Number Autoencoder Bottleneck Labs

## Status

Draft specification for deterministic, language-neutral `2 -> 1 -> 2` linear
autoencoder forward, reconstruction-loss, backward, gradient-audit, and update
traces.

## Purpose

NN16 begins representation learning with the smallest undercomplete network.
Two input coordinates must pass through one scalar bottleneck before the model
can reconstruct both coordinates. The trace exposes what is compressed, what
is lost, how both reconstruction errors meet at the bottleneck, and how one SGD
step changes every encoder and decoder parameter.

## Architecture Contract

V1 uses one two-number example, identity activations, one scalar bottleneck,
and a two-number reconstruction:

```text
input width 2 -> bottleneck width 1 -> reconstruction width 2
```

For input coordinate `i` and output coordinate `j`:

```text
encoder_product[i] = input[i] * encoder_weight[i]
bottleneck = sum_i(encoder_product[i]) + encoder_bias

decoder_product[j] = bottleneck * decoder_weight[j]
reconstruction[j] = decoder_product[j] + decoder_bias[j]
```

Identity activations keep the compression boundary visible. Later labs may add
nonlinear encoders, larger datasets, and higher-dimensional bottlenecks without
changing the meaning of reconstruction.

## Loss Contract

The error uses `reconstruction - input`. The objective is mean squared error
across the two reconstructed coordinates:

```text
error[j] = reconstruction[j] - input[j]
squared_error[j] = error[j]^2
loss = sum_j(squared_error[j]) / output_width
```

Because `output_width = 2`, the reconstruction gradient simplifies to the
error itself:

```text
d_loss/d_reconstruction[j] = error[j]
```

## Backward Contract

The decoder gradients are:

```text
decoder_weight_gradient[j]
  = reconstruction_gradient[j] * bottleneck
decoder_bias_gradient[j] = reconstruction_gradient[j]

bottleneck_gradient_contribution[j]
  = reconstruction_gradient[j] * decoder_weight[j]
bottleneck_gradient = sum_j(bottleneck_gradient_contribution[j])
```

Both reconstructed coordinates therefore meet at the one-number bottleneck.
Continue that combined signal through the encoder:

```text
encoder_weight_gradient[i] = bottleneck_gradient * input[i]
encoder_bias_gradient = bottleneck_gradient
```

## Numerical Audit and Update Contract

Independently perturb all seven trainable scalars by `+epsilon` and `-epsilon`
and estimate the centered slope of the same mean loss:

```text
numerical_gradient
  = (loss(parameter + epsilon) - loss(parameter - epsilon)) / (2 * epsilon)
```

The canonical lab uses `epsilon = 1e-6` and records analytical and numerical
gradients in this order:

```text
encoder weights, encoder bias, decoder weights, decoder biases
```

Apply one SGD step to all parameters:

```text
parameter_after = parameter_before - learning_rate * gradient
```

The post-update trace reruns the complete encoder, bottleneck, decoder, and
loss. Its loss must be lower for the canonical lab.

## Fixture Layout

```text
code/specs/fixtures/two-number-autoencoder-v1/
  schema.json
  labs/00-linear-bottleneck-step.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields, incorrect
shapes, unsupported operation metadata, and trace values outside the declared
absolute tolerance.

## Conformance Levels

1. **Encode:** reproduce both encoder products and the scalar bottleneck.
2. **Decode:** reproduce both decoder products and reconstructed coordinates.
3. **Measure:** reproduce errors, squared errors, and mean reconstruction loss.
4. **Differentiate:** reproduce decoder, bottleneck, and encoder gradients.
5. **Audit:** reproduce all seven central finite-difference gradients.
6. **Update:** apply SGD and reproduce the lower post-update loss.

## Cross-Language and Rust-Core Direction

Every language should implement the two scalar dense layers directly first.
The fixture indexes weights mathematically and does not require a particular
row-major or column-major in-memory layout.

A performant Rust core can later expose batched encoder and decoder matrix
multiplication, activation, reconstruction loss, backward reduction, and
optimizer kernels through a stable C ABI. The ABI should make batch size,
input width, bottleneck width, output width, strides, activation identifiers,
loss reduction, learning rate, and caller-owned buffers explicit. Trace mode
must optionally return encoded values, reconstructions, coordinate errors,
bottleneck gradients, and parameter gradients even when the fast path fuses
operations.
