# NN19: One-Dimensional Diffusion Labs

## Status

Draft specification for deterministic, language-neutral scalar diffusion
forward-noising, noise-prediction training, gradient-audit, and reverse-mean
traces.

## Purpose

NN19 turns diffusion into two visible operations: deliberately mix a clean
sample with saved noise at known levels, then train a model to predict that
noise so the process can run backward. One clean scalar and two timesteps are
enough to expose signal decay, noise growth, timestep conditioning, the
noise-prediction objective, and iterative denoising.

## Forward Schedule Contract

For every schedule row:

```text
alpha_t = 1 - beta_t
alpha_bar_t = product(alpha_1 through alpha_t)
signal_scale_t = sqrt(alpha_bar_t)
noise_scale_t = sqrt(1 - alpha_bar_t)

x_t = signal_scale_t * clean_sample
    + noise_scale_t * saved_noise
```

V1 uses the same saved noise at both levels so learners can compare how the
coefficients trade signal for noise. The rows are direct closed-form samples
from `x_0`; they are not claimed to be one Markov forward trajectory.

## Noise Predictor Contract

The scalar denoiser sees the noisy sample and a normalized timestep:

```text
predicted_noise
  = sample_weight * x_t
  + timestep_weight * normalized_t
  + bias
```

Its target is the saved noise used to construct that `x_t`. For two schedule
rows, V1 uses mean half-squared error:

```text
row_loss = 0.5 * (predicted_noise - saved_noise)^2
mean_loss = (row_loss_1 + row_loss_2) / 2
```

The prediction gradient reported for each row already includes the mean
reduction factor.

## Update and Numerical Audit Contract

Add both rows' contributions into the three shared parameter gradients, audit
the initial mean objective with centered finite differences and
`epsilon = 1e-6`, then apply one SGD update. The post-update mean loss must be
lower than the initial mean loss.

## Reverse Mean Contract

Start from the noisiest forward sample. Iterate the schedule in reverse using
the updated denoiser and the deterministic DDPM mean:

```text
noise_coefficient_t = beta_t / sqrt(1 - alpha_bar_t)

corrected_sample
  = current_sample - noise_coefficient_t * predicted_noise

previous_mean = corrected_sample / sqrt(alpha_t)
```

Feed each `previous_mean` into the next lower timestep. V1 adds no fresh reverse
noise, so every consumer replays the same mean path. The final reconstruction
need not equal the clean sample exactly; its remaining error makes model quality
visible.

## Fixture Layout

```text
code/specs/fixtures/one-dimensional-diffusion-v1/
  schema.json
  labs/00-two-level-forward-and-reverse.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields,
unsupported metadata, invalid timestep order, schedule inconsistencies, and
trace values outside the declared absolute tolerance.

## Conformance Levels

1. **Noise:** reproduce every alpha, cumulative alpha, scale, contribution, and
   noisy sample.
2. **Predict:** reproduce both timestep-conditioned noise predictions and the
   initial mean loss.
3. **Differentiate:** reproduce both rows' gradient contributions and their
   three shared reductions.
4. **Audit:** reproduce the three-parameter centered finite-difference check.
5. **Learn:** apply SGD, rerun both rows, and lower mean noise-prediction loss.
6. **Reverse:** replay both deterministic reverse means and the final clean
   reconstruction error.

## Cross-Language and Rust-Core Direction

Every host language should implement the scalar schedule directly first. A
Rust core can later batch schedule coefficient generation, saved-noise mixing,
timestep-conditioned denoiser calls, MSE reductions, backward kernels, and
reverse steps behind a stable C ABI.

The ABI must make tensor shape, dtype, beta schedule, timestep normalization,
random-number ownership, reduction rule, reverse variance policy, and
caller-owned output buffers explicit. Trace mode should optionally return
per-timestep signal/noise contributions, predicted noise, gradient
contributions, reverse corrections, and reconstruction estimates.
