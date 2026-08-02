# Variational Sampling and the KL Tradeoff, by Hand

A regular autoencoder turns an input into one fixed bottleneck value. A
variational autoencoder turns the input into a *distribution* of possible
bottleneck values. The encoder describes that distribution, we draw a sample,
and the decoder reconstructs from the sample.

Why add uncertainty? We want nearby, sampleable regions in latent space rather
than an arbitrary collection of isolated codes. That requires two learning
pressures:

- reconstruction keeps information about the input;
- KL divergence keeps the encoded distribution near a simple prior that we can
  sample without already having an input.

This lesson uses one input, one Gaussian latent variable, one saved noise value,
and one output. The deterministic oracle is
[`00-saved-noise-kl-step.json`](../../specs/fixtures/variational-autoencoder-v1/labs/00-saved-noise-kl-step.json),
and the language-neutral contract is
[`NN17-variational-autoencoder-labs.md`](../../specs/NN17-variational-autoencoder-labs.md).

## Encode a distribution, not a point

The input is:

```text
x = 1
```

The encoder has two scalar heads. One produces the Gaussian mean. The other
produces the log of its variance:

```text
mean = x * mean_weight + mean_bias
     = 1 * 0.4 + 0
     = 0.4

log_variance
  = x * log_variance_weight + log_variance_bias
  = 1 * 0 + 0
  = 0
```

Networks usually predict log-variance because any real log-variance becomes a
positive variance after exponentiation:

```text
variance = exp(log_variance)
         = exp(0)
         = 1

standard_deviation = exp(0.5 * log_variance)
                   = exp(0)
                   = 1
```

The encoded distribution is therefore `Normal(mean=0.4, variance=1)`.

## Save the noise and reparameterize

A direct random draw looks like a dead end for backpropagation: how do we
differentiate through an instruction that says "pick a random value"?

The reparameterization trick moves randomness into a separate input:

```text
epsilon ~ Normal(0, 1)
z = mean + standard_deviation * epsilon
```

For this trace, save one already-drawn noise value:

```text
epsilon = 0.5

noise contribution = 1 * 0.5
                   = 0.5

z = 0.4 + 0.5
  = 0.9
```

Now every operation after `epsilon` is ordinary differentiable arithmetic.
Training still uses fresh noise samples in general, but a fixture holds one
sample fixed so every language can replay and audit the same step.

## Reconstruct from the sampled value

Use decoder weight `1` and bias `0`:

```text
x_hat = z * decoder_weight + decoder_bias
      = 0.9 * 1 + 0
      = 0.9
```

Use half squared reconstruction error:

```text
error = x_hat - x
      = 0.9 - 1
      = -0.1

reconstruction_loss = 0.5 * error^2
                    = 0.5 * 0.01
                    = 0.005
```

The factor `0.5` makes the derivative with respect to the reconstruction equal
to the error.

## Measure distance from the prior

We want the encoded Gaussian to stay near the standard normal prior
`Normal(0, 1)`. For one scalar Gaussian, the KL divergence is:

```text
KL = 0.5 * (mean^2 + variance - 1 - log_variance)
```

Substitute this trace:

```text
KL = 0.5 * (0.4^2 + 1 - 1 - 0)
   = 0.5 * 0.16
   = 0.08
```

The variance already matches the prior variance of `1`, but the mean is shifted
away from the prior mean of `0`. That is why the KL value is positive.

## Use beta to expose the tradeoff

The complete objective is:

```text
total_loss = reconstruction_loss + beta * KL
```

Use `beta = 0.1`:

```text
weighted_KL = 0.1 * 0.08
            = 0.008

total_loss = 0.005 + 0.008
           = 0.013
```

Beta is not a probability. It is a knob controlling how loudly prior matching
speaks relative to reconstruction:

| beta | weighted objective before the update | meaning in this trace |
| ---: | ---: | --- |
| `0` | `0.005` | reconstruction speaks alone |
| `0.1` | `0.013` | gentle pull toward the prior |
| `0.25` | `0.025` | the two mean-gradient routes exactly cancel |
| `1` | `0.085` | prior matching dominates the mean direction |

These values compare objectives with different definitions, so do not rank
models by the raw totals across beta settings. Inspect how beta changes the
gradient directions and the eventual reconstruction-versus-regularity balance.

## Backpropagate through reconstruction

Start at the output:

```text
d reconstruction_loss / d x_hat = error = -0.1

d loss / d decoder_weight = -0.1 * z
                            = -0.09
d loss / d decoder_bias = -0.1

d loss / d z = -0.1 * decoder_weight
             = -0.1
```

The sample equation was:

```text
z = mean + standard_deviation * epsilon
```

So the reconstruction route into the mean is direct:

```text
reconstruction mean gradient = -0.1
```

For log-variance:

```text
d standard_deviation / d log_variance
  = 0.5 * standard_deviation

reconstruction log-variance gradient
  = d loss/dz * 0.5 * standard_deviation * epsilon
  = -0.1 * 0.5 * 1 * 0.5
  = -0.025
```

This is the key payoff of reparameterization: the decoder's reconstruction
signal reaches both distribution parameters through normal calculus.

## Backpropagate through KL independently

Differentiate the scalar KL formula:

```text
d KL / d mean = mean
              = 0.4

d KL / d log_variance = 0.5 * (variance - 1)
                       = 0.5 * (1 - 1)
                       = 0
```

Scale those routes by beta `0.1`:

```text
weighted KL mean gradient = 0.1 * 0.4 = 0.04
weighted KL log-variance gradient = 0.1 * 0 = 0
```

The KL variance gradient is zero here because the encoded variance already
equals the prior variance. The mean still needs to move toward zero.

## Add both objectives at the encoder

The encoder outputs influenced both reconstruction and KL, so add the routes:

```text
total mean gradient = -0.1 + 0.04
                    = -0.06

total log-variance gradient = -0.025 + 0
                            = -0.025
```

Because `x = 1`, each encoder weight gradient equals its output gradient:

```text
mean weight gradient = -0.06 * 1 = -0.06
mean bias gradient = -0.06

log-variance weight gradient = -0.025 * 1 = -0.025
log-variance bias gradient = -0.025
```

Notice the mean conflict. Reconstruction contributes `-0.1`, asking the sample
to move upward toward the input. KL contributes `+0.04`, asking the mean to move
down toward zero. Beta controls the result after those signals meet.

## Audit all six trainable scalars

The model has six parameters:

```text
2 mean-head parameters
+ 2 log-variance-head parameters
+ 2 decoder parameters
= 6
```

Perturb each parameter by `epsilon = 0.000001`, keep the *sampling noise* fixed
at `0.5`, and rerun the whole total loss:

```text
numerical gradient
  = (loss(parameter + epsilon) - loss(parameter - epsilon))
    / (2 * epsilon)
```

Holding sampling noise fixed matters. If the plus and minus evaluations used
different random draws, noise could overwhelm the tiny slope we are trying to
measure. The largest analytical-versus-numerical difference is about
`7.66e-12`, which is floating-point rounding.

## Apply one SGD step

Use learning rate `0.1`:

```text
parameter_after = parameter_before - 0.1 * gradient
```

The updated parameters are:

```text
mean weight = 0.406
mean bias = 0.006

log-variance weight = 0.0025
log-variance bias = 0.0025

decoder weight = 1.009
decoder bias = 0.01
```

Rerun the same saved-noise trace:

```text
mean_after = 1 * 0.406 + 0.006
           = 0.412

log_variance_after = 1 * 0.0025 + 0.0025
                   = 0.005

standard_deviation_after = exp(0.5 * 0.005)
                         = 1.0025031276

z_after = 0.412 + 1.0025031276 * 0.5
        = 0.9132515638

x_hat_after = 0.9132515638 * 1.009 + 0.01
            = 0.9314708279
```

The post-update terms are:

```text
reconstruction_loss_after = 0.0023481237
KL_after = 0.0848782604
weighted_KL_after = 0.0084878260
total_loss_after = 0.0108359498
```

Reconstruction improved while KL increased slightly. The weighted sum still
fell from `0.013` to `0.0108359498`. A VAE step optimizes the combined objective,
not either term in isolation.

## What this one-sample trace does not prove

This fixture explains one pathwise gradient. A useful VAE requires a dataset,
many latent dimensions, many noise samples over training, and careful evaluation
of both reconstruction and generated samples.

The saved `epsilon = 0.5` is an audit tool, not a recommendation to reuse one
noise value forever. Production training should draw fresh standard-normal
noise while reproducible runs control the generator seed or supply explicit
noise buffers.

## Common implementation bugs

- Predicting variance directly and allowing it to become negative.
- Using `exp(log_variance)` where the standard deviation requires
  `exp(0.5 * log_variance)`.
- Drawing unrelated noise for the plus and minus finite-difference evaluations.
- Stopping the gradient at the sampled latent value.
- Forgetting to multiply KL gradients by beta.
- Minimizing KL alone and losing input-specific information.
- Comparing raw total losses across different beta values as if the objective
  had not changed.
- Updating parameters before both reconstruction and KL routes are accumulated.

## Explore the tradeoff

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
shows the two encoder heads, saved noise, reparameterized sample, decoder, split
objective, and both gradient routes. Change beta to watch the mean direction
move from reconstruction toward the prior; beta `0.25` makes those mean routes
cancel exactly in this hand-sized example. Toggle the update to rerun the full
distribution and objective.

## Cross-language checkpoint

An NN17 consumer is conformant when it reproduces the Gaussian parameters,
saved-noise sample, reconstruction, KL and beta-weighted objective, separate
gradient routes, six numerical gradients, updated parameters, and post-update
trace.

Implement the scalar exponential and reparameterization directly in every host
language first. A Rust core may later batch the encoder heads, accept explicit
epsilon buffers or a documented generator, fuse decoder and loss kernels, and
return optional trace buffers through a stable C ABI. Random-number ownership
must stay explicit so performance never makes cross-language reproducibility or
the sampling boundary mysterious.
