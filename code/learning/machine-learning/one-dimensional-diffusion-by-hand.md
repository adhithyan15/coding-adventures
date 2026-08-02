# One-Dimensional Diffusion, by Hand

A diffusion model learns generation by practicing a reversible-looking task:

1. mix clean data with a known amount of noise;
2. train a neural network to predict the noise that was mixed in;
3. subtract predicted noise a little at a time.

Large diffusion models do this with images, thousands of dimensions, and many
timesteps. This lesson uses one clean number, one saved noise value, two noise
levels, and a three-parameter denoiser. Every coefficient and update fits on
paper.

The deterministic oracle is
[`00-two-level-forward-and-reverse.json`](../../specs/fixtures/one-dimensional-diffusion-v1/labs/00-two-level-forward-and-reverse.json),
and the language-neutral contract is
[`NN19-one-dimensional-diffusion-labs.md`](../../specs/NN19-one-dimensional-diffusion-labs.md).

## Start with one clean sample and one saved noise value

Use:

```text
x0 = 1
epsilon = -0.5
```

In real training, `epsilon` is usually drawn from a standard normal
distribution. Saving one already-drawn value makes this trace reproducible and
lets every language audit identical arithmetic.

## Turn beta into signal and noise scales

Each timestep has a noise rate `beta_t` and retained-signal rate:

```text
alpha_t = 1 - beta_t
```

The cumulative retained signal through timestep `t` is:

```text
alpha_bar_t = alpha_1 * alpha_2 * ... * alpha_t
```

Use this two-step schedule:

| timestep | beta | alpha | alpha_bar |
| ---: | ---: | ---: | ---: |
| `1` | `0.36` | `0.64` | `0.64` |
| `2` | `0.4375` | `0.5625` | `0.36` |

The closed-form forward sample is:

```text
x_t
  = sqrt(alpha_bar_t) * x0
  + sqrt(1 - alpha_bar_t) * epsilon
```

The first square root scales clean signal. The second scales noise. Their
squared coefficients add to `1`, so the mixture shifts smoothly rather than
growing without bound.

## Forward level 1

At `t = 1`:

```text
signal scale = sqrt(0.64) = 0.8
noise scale = sqrt(1 - 0.64) = sqrt(0.36) = 0.6

signal contribution = 0.8 * 1 = 0.8
noise contribution = 0.6 * (-0.5) = -0.3

x1 = 0.8 + (-0.3)
   = 0.5
```

The clean value has moved from `1` to `0.5`, but its signal coefficient is
still larger than its noise coefficient.

## Forward level 2

At `t = 2`:

```text
signal scale = sqrt(0.36) = 0.6
noise scale = sqrt(1 - 0.36) = sqrt(0.64) = 0.8

signal contribution = 0.6 * 1 = 0.6
noise contribution = 0.8 * (-0.5) = -0.4

x2 = 0.6 + (-0.4)
   = 0.2
```

Now noise has the larger coefficient. Both levels reused the same saved
`epsilon` so the changing coefficients are easy to compare. They are direct
closed-form samples from `x0`, not consecutive draws from one Markov forward
trajectory. A production trainer may sample an arbitrary timestep directly in
exactly this way.

## Ask the model to predict noise

The denoiser must know both the noisy value and how far along the schedule it
is. Give it normalized times `0.5` and `1`:

```text
epsilon_hat
  = sample_weight * x_t
  + timestep_weight * normalized_t
  + bias
```

Start all three parameters at zero. The prediction at both timesteps is zero:

```text
epsilon_hat_1 = 0
epsilon_hat_2 = 0
```

The target is the saved noise `-0.5`. Use half-squared error per row:

```text
error = predicted_noise - target_noise
      = 0 - (-0.5)
      = 0.5

row_loss = 0.5 * error^2
         = 0.5 * 0.25
         = 0.125
```

Both row losses are `0.125`, so their mean is also `0.125`.

Predicting noise may look indirect. Its advantage is that training always has a
known target: the program just generated `epsilon`. Once the model estimates
that corruption, the reverse formula converts the estimate into a cleaner
sample.

## Backpropagate both timesteps

The objective is the mean of two half-squared errors. Therefore each row's
prediction gradient includes a factor of `1 / 2`:

```text
d mean_loss / d predicted_noise
  = error / 2
  = 0.5 / 2
  = 0.25
```

At level 1, `x1 = 0.5` and normalized time is `0.5`:

```text
sample-weight contribution = 0.25 * 0.5 = 0.125
time-weight contribution = 0.25 * 0.5 = 0.125
bias contribution = 0.25
```

At level 2, `x2 = 0.2` and normalized time is `1`:

```text
sample-weight contribution = 0.25 * 0.2 = 0.05
time-weight contribution = 0.25 * 1 = 0.25
bias contribution = 0.25
```

Add contributions because both rows used the same denoiser parameters:

```text
sample_weight gradient = 0.125 + 0.05 = 0.175
timestep_weight gradient = 0.125 + 0.25 = 0.375
bias gradient = 0.25 + 0.25 = 0.5
```

The timestep feature matters. The same network must interpret a moderately
noisy value differently from a heavily noisy one.

## Audit the three gradients

For each parameter, perturb the initial value with
`audit_epsilon = 0.000001` and recompute the mean loss:

```text
numerical gradient
  = (loss(parameter + audit_epsilon)
     - loss(parameter - audit_epsilon))
    / (2 * audit_epsilon)
```

The numerical gradients are:

```text
[0.174999999998, 0.374999999997, 0.499999999994]
```

The largest analytical-versus-numerical difference is about `6.44e-12`, which
is floating-point rounding.

## Apply one SGD step

Use learning rate `0.5`:

```text
new sample_weight = 0 - 0.5 * 0.175
                  = -0.0875

new timestep_weight = 0 - 0.5 * 0.375
                    = -0.1875

new bias = 0 - 0.5 * 0.5
         = -0.25
```

Rerun both saved-noise examples.

At level 1:

```text
epsilon_hat_1
  = -0.0875 * 0.5 + -0.1875 * 0.5 + -0.25
  = -0.3875

loss_1 = 0.5 * (-0.3875 - -0.5)^2
       = 0.006328125
```

At level 2:

```text
epsilon_hat_2
  = -0.0875 * 0.2 + -0.1875 * 1 + -0.25
  = -0.455

loss_2 = 0.5 * (-0.455 - -0.5)^2
       = 0.0010125
```

The new mean loss is:

```text
(0.006328125 + 0.0010125) / 2
  = 0.0036703125
```

One step lowered the objective from `0.125` to `0.0036703125`.

## Reverse from level 2 to level 1

For this deterministic audit, use the DDPM reverse mean without adding fresh
reverse noise:

```text
noise_coefficient_t
  = beta_t / sqrt(1 - alpha_bar_t)

previous_mean
  = (current_sample
     - noise_coefficient_t * predicted_noise)
    / sqrt(alpha_t)
```

At `t = 2`:

```text
noise coefficient = 0.4375 / 0.8
                  = 0.546875

scaled noise correction = 0.546875 * (-0.455)
                        = -0.248828125

corrected sample = 0.2 - (-0.248828125)
                 = 0.448828125

mean1 = 0.448828125 / sqrt(0.5625)
      = 0.448828125 / 0.75
      = 0.5984375
```

This generated `mean1` becomes the next denoiser input. Do not replace it with
the saved forward value `x1 = 0.5`; reverse generation must follow its own path.

## Reverse from level 1 to clean data

Predict noise again using the generated mean and normalized time `0.5`:

```text
epsilon_hat
  = -0.0875 * 0.5984375 + -0.1875 * 0.5 + -0.25
  = -0.39611328125
```

Apply the `t = 1` reverse mean:

```text
noise coefficient = 0.36 / 0.6 = 0.6

scaled noise correction = 0.6 * (-0.39611328125)
                        = -0.23766796875

corrected sample = 0.5984375 - (-0.23766796875)
                 = 0.83610546875

mean0 = 0.83610546875 / sqrt(0.64)
      = 0.83610546875 / 0.8
      = 1.0451318359375
```

The noisiest input `0.2` was `0.8` away from the clean value. The final reverse
mean is only `0.0451318359375` away. It is not exact because the tiny denoiser
still predicts imperfect noise.

## What this trace leaves out

- Production schedules use many more, usually much smaller, beta values.
- A real denoiser consumes tensors and a learned timestep embedding.
- Training samples many clean examples, timesteps, and fresh noise values.
- A stochastic sampler may add controlled reverse variance at intermediate
  steps; this fixture follows the mean only.
- Better noise-prediction loss often helps generation, but sample quality and
  diversity still need direct evaluation.
- Other parameterizations predict clean data, velocity, or score rather than
  raw noise.

## Common implementation bugs

- Using `alpha_t` where the closed-form forward equation needs cumulative
  `alpha_bar_t`.
- Scaling noise by `sqrt(beta_t)` in the direct-from-`x0` formula instead of
  `sqrt(1 - alpha_bar_t)`.
- Forgetting timestep conditioning.
- Drawing unrelated noise for the analytical and finite-difference passes.
- Forgetting the mean-reduction factor when accumulating shared gradients.
- Feeding the saved forward `x1` into the second reverse step instead of the
  generated `mean1`.
- Subtracting `predicted_noise` directly without the schedule coefficient.
- Adding random reverse noise during a fixture that specifies the deterministic
  mean path.

## Explore the round trip

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
keeps the clean value, both forward mixtures, denoiser objective, per-timestep
gradient contributions, audited update, and both reverse means in one view.
Advance the six phases to see which value becomes the next input and when the
initial denoiser switches to its trained parameters.

## Cross-language checkpoint

An NN19 consumer is conformant when it reproduces both forward noise levels,
the initial and updated predictions, mean losses, per-row and reduced gradients,
three numerical slopes, updated parameters, both reverse means, and final
reconstruction error.

Implement this scalar loop directly in every host language first. A Rust core
can later vectorize coefficient generation, noise mixing, denoiser kernels,
loss reductions, backward passes, and samplers behind a stable C ABI. Keep beta
schedules, timestep encoding, random-number ownership, reduction rules, reverse
variance policy, and optional trace buffers explicit so performance does not
hide the forward/reverse contract.
