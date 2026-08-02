# NN17: Scalar Variational Autoencoder Labs

## Status

Draft specification for deterministic, language-neutral scalar variational
autoencoder sampling, beta-weighted KL divergence, backward, gradient-audit,
and update traces.

## Purpose

NN17 adds uncertainty to the smallest autoencoder. Instead of encoding one
fixed latent number, the encoder produces a Gaussian mean and log-variance.
A saved noise value is transformed into one reproducible sample so learners can
see both the stochastic model and a deterministic execution trace.

The objective keeps two pressures separate:

- reconstruction asks the sample to preserve this input;
- KL divergence asks the encoded distribution to stay near a standard normal
  prior that can be sampled later.

## Architecture Contract

V1 uses one scalar input, one scalar Gaussian latent variable, identity decoder,
and one scalar reconstruction:

```text
input -> [mean, log variance] -> saved epsilon -> latent z -> reconstruction
```

```text
mean = input * mean_weight + mean_bias
log_variance = input * log_variance_weight + log_variance_bias
variance = exp(log_variance)
standard_deviation = exp(0.5 * log_variance)
noise_contribution = standard_deviation * epsilon
latent = mean + noise_contribution
reconstruction = latent * decoder_weight + decoder_bias
```

`epsilon` is part of the fixture. Consumers must not call a random-number
generator while replaying a conformance document.

## Objective Contract

The reconstruction term uses half squared error:

```text
error = reconstruction - input
reconstruction_loss = 0.5 * error^2
```

The scalar Gaussian KL divergence against `Normal(0, 1)` is:

```text
kl = 0.5 * (mean^2 + variance - 1 - log_variance)
weighted_kl = beta * kl
total_loss = reconstruction_loss + weighted_kl
```

The canonical lab uses `beta = 0.1`. A larger beta gives the prior-matching
term more influence; beta zero reduces this saved-sample step to reconstruction
training alone.

## Backward Contract

Start at the decoder and follow the saved sample:

```text
reconstruction_gradient = error
decoder_weight_gradient = reconstruction_gradient * latent
decoder_bias_gradient = reconstruction_gradient
latent_gradient = reconstruction_gradient * decoder_weight

reconstruction_mean_gradient = latent_gradient
reconstruction_log_variance_gradient
  = latent_gradient * 0.5 * standard_deviation * epsilon
```

Differentiate the KL term independently:

```text
kl_mean_gradient = mean
kl_log_variance_gradient = 0.5 * (variance - 1)

mean_gradient
  = reconstruction_mean_gradient + beta * kl_mean_gradient
log_variance_gradient
  = reconstruction_log_variance_gradient
  + beta * kl_log_variance_gradient
```

Finish at the two encoder heads:

```text
mean_weight_gradient = mean_gradient * input
mean_bias_gradient = mean_gradient
log_variance_weight_gradient = log_variance_gradient * input
log_variance_bias_gradient = log_variance_gradient
```

## Numerical Audit and Update Contract

Independently perturb all six trainable scalars by `+epsilon` and `-epsilon`
while holding the saved sampling noise fixed:

```text
numerical_gradient
  = (total_loss(parameter + epsilon)
    - total_loss(parameter - epsilon)) / (2 * epsilon)
```

The canonical audit uses `epsilon = 1e-6` and this order:

```text
mean weight, mean bias,
log-variance weight, log-variance bias,
decoder weight, decoder bias
```

Apply one SGD step to all six parameters and rerun the complete distribution,
sample, reconstruction, KL, and total objective. The canonical post-update total
loss must be lower.

## Fixture Layout

```text
code/specs/fixtures/variational-autoencoder-v1/
  schema.json
  labs/00-saved-noise-kl-step.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields, incorrect
shapes, unsupported operation metadata, and trace values outside the declared
absolute tolerance.

## Conformance Levels

1. **Parameterize:** reproduce the mean, log-variance, variance, and standard
   deviation.
2. **Sample:** reproduce the noise contribution and latent value from saved
   epsilon.
3. **Decode:** reproduce the reconstruction and half-squared error.
4. **Regularize:** reproduce KL divergence, its beta weighting, and total loss.
5. **Differentiate:** keep reconstruction and KL gradient routes separate before
   adding them at both encoder outputs.
6. **Audit and update:** reproduce all six central finite differences, one SGD
   step, and the lower post-update total loss.

## Cross-Language and Rust-Core Direction

Every host language should implement the scalar exponential, saved-noise
reparameterization, KL term, and backward equations directly first. A fast Rust
core can later expose batched mean and log-variance heads, caller-supplied random
samples, decoder kernels, KL reductions, backward reductions, and optimizers
through a stable C ABI.

The ABI must make batch size, latent width, strides, beta, reduction rules,
random-number ownership, seed or caller-owned epsilon buffers, and output buffers
explicit. Trace mode must optionally return means, log-variances, standard
deviations, samples, reconstruction losses, KL terms, and both gradient routes so
bindings never hide the stochastic boundary behind a fused kernel.
