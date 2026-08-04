# A One-Dimensional GAN Game, by Hand

A generative adversarial network is not one model minimizing one loss. It is a
game between two models:

- a **generator** turns noise into a fake sample;
- a **discriminator** estimates whether a sample came from real data.

The discriminator first gets better at spotting the fake. Then the generator
uses the discriminator's slope to make a more convincing fake. That alternating
schedule is the important idea; larger images and deeper networks only replace
the scalar arithmetic.

This lesson uses one real number, one saved noise value, two scalar affine
models, and one sigmoid. The deterministic oracle is
[`00-discriminator-generator-round.json`](../../specs/fixtures/one-dimensional-gan-v1/labs/00-discriminator-generator-round.json),
and the language-neutral contract is
[`NN18-one-dimensional-gan-labs.md`](../../specs/NN18-one-dimensional-gan-labs.md).

## Put both models on one number line

Use one real sample and one already-drawn noise value:

```text
real = 1
noise = 1
```

The generator starts with weight `0.2` and bias `0`:

```text
fake = generator_weight * noise + generator_bias
     = 0.2 * 1 + 0
     = 0.2
```

The discriminator starts with weight `1` and bias `0`. It turns its affine
logit into a probability with the sigmoid function:

```text
discriminator_probability(x)
  = sigmoid(discriminator_weight * x + discriminator_bias)

real logit = 1 * 1 + 0 = 1
D(real) = sigmoid(1) = 0.7310585786

fake logit = 1 * 0.2 + 0 = 0.2
D(fake) = sigmoid(0.2) = 0.5498339973
```

These values are scores under the current discriminator, not objective facts
about authenticity. An untrained discriminator happens to call both points
more real than fake.

## Give the players different jobs

The discriminator uses mean binary cross-entropy. The real point has target
`1`; the fake point has target `0`:

```text
discriminator_loss
  = -0.5 * (log D(real) + log(1 - D(fake)))
  = -0.5 * (log 0.7310585786 + log(1 - 0.5498339973))
  = 0.5557002784
```

The generator uses the non-saturating objective. It asks for its fake to receive
the real label:

```text
generator_loss = -log D(fake)
               = -log 0.5498339973
               = 0.5981388694
```

These losses disagree on purpose. The discriminator wants `D(fake)` lower; the
generator wants it higher. Do not add them and run one joint update.

## Turn one: train the discriminator

During the discriminator turn, treat the generated value `0.2` as fixed. This
operation is often called **detach** or **stop gradient**. The discriminator may
change, but the generator may not.

For sigmoid plus binary cross-entropy, each logit gradient is prediction minus
target. The mean over two examples adds a factor of `0.5`:

```text
real logit gradient
  = 0.5 * (D(real) - 1)
  = 0.5 * (0.7310585786 - 1)
  = -0.1344707107

fake logit gradient
  = 0.5 * (D(fake) - 0)
  = 0.5 * 0.5498339973
  = 0.2749169987
```

Both examples use the same discriminator weight and bias, so add their
contributions:

```text
discriminator weight gradient
  = real_logit_gradient * real
    + fake_logit_gradient * fake
  = -0.1344707107 * 1 + 0.2749169987 * 0.2
  = -0.0794873110

discriminator bias gradient
  = real_logit_gradient + fake_logit_gradient
  = -0.1344707107 + 0.2749169987
  = 0.1404462880
```

There is no generator gradient in this turn:

```text
gradient into fake sample = 0
generator gradients = 0
```

Apply discriminator learning rate `0.5`:

```text
new discriminator weight
  = 1 - 0.5 * (-0.0794873110)
  = 1.0397436555

new discriminator bias
  = 0 - 0.5 * 0.1404462880
  = -0.0702231440
```

The generator remains unchanged, so the fake is still `0.2`. Rerun the updated
discriminator:

```text
new D(real) = 0.7250239152
new D(fake) = 0.5343770743

discriminator loss: 0.5557002784 -> 0.5429648914
generator loss:     0.5981388694 -> 0.6266535576
```

The discriminator improved its own loss. The generator's loss worsened because
the judge just became harder to fool.

## Turn two: train the generator

Now freeze the updated discriminator parameters. Freezing does **not** mean
cutting the entire graph at the discriminator. Its input slope still tells the
generator which direction makes the fake look more real.

For the non-saturating generator loss, the fake-logit gradient is:

```text
fake logit gradient
  = D(fake) - 1
  = 0.5343770743 - 1
  = -0.4656229257
```

Carry that signal through the updated discriminator weight:

```text
fake sample gradient
  = fake_logit_gradient * updated_discriminator_weight
  = -0.4656229257 * 1.0397436555
  = -0.4841284829
```

Then carry it through the generator. Because the saved noise is `1`:

```text
generator weight gradient
  = fake_sample_gradient * noise
  = -0.4841284829 * 1
  = -0.4841284829

generator bias gradient = -0.4841284829
```

The discriminator parameter gradients are not accumulated or applied in this
turn. Apply generator learning rate `0.25`:

```text
new generator weight
  = 0.2 - 0.25 * (-0.4841284829)
  = 0.3210321207

new generator bias
  = 0 - 0.25 * (-0.4841284829)
  = 0.1210321207
```

Reuse the saved noise to generate the counter-move:

```text
new fake
  = 0.3210321207 * 1 + 0.1210321207
  = 0.4420642414

frozen updated D(new fake) = 0.5961407441

generator loss:     0.6266535576 -> 0.5172784919
discriminator loss: 0.5429648914 -> 0.6141197382
```

The generator moved its point from `0.2` toward the real point at `1` and
lowered its own loss. The discriminator loss rose. That is not a failed update;
it is evidence that the opponent made a successful move.

## Audit two different objectives

A normal supervised model has one objective for a gradient check. This GAN
round requires two checks because the player and the frozen values change.

For each parameter, use centered finite differences with
`epsilon = 0.000001`:

```text
numerical gradient
  = (loss(parameter + epsilon) - loss(parameter - epsilon))
    / (2 * epsilon)
```

The discriminator audit perturbs its initial weight and bias while holding the
initial fake `0.2` detached. Its numerical gradients are:

```text
[-0.0794873109, 0.1404462880]
```

The largest analytical-versus-numerical difference is about `5.38e-11`.

The generator audit perturbs its initial weight and bias while holding the
**updated** discriminator and saved noise fixed. Its numerical gradients are:

```text
[-0.4841284829, -0.4841284829]
```

The largest difference is about `3.34e-12`. A joint finite-difference check over
all four parameters would describe neither turn in the schedule.

## What changes in a real GAN

This scalar trace preserves the training topology, but not the scale:

- real training uses batches and many fresh noise samples;
- generator and discriminator are usually deep networks;
- their learning rates and update counts may differ;
- losses can oscillate because the target keeps changing;
- a low loss does not by itself prove diverse, high-quality generation;
- mode collapse can let a generator fool the critic with too little variety.

The one real point also cannot teach a distribution. It exists so every route
and freeze boundary fits on paper.

## Common implementation bugs

- Updating generator parameters during the discriminator turn.
- Detaching the fake during the generator turn, which removes its learning
  signal.
- Updating discriminator parameters during the generator turn instead of only
  using its input gradient.
- Reusing stale fake probabilities after either model changes.
- Minimizing `log(1 - D(fake))` for the generator without recognizing its weak
  early gradient; this fixture specifies the non-saturating `-log D(fake)`
  objective.
- Treating `D(x)` as a calibrated probability of authenticity outside the
  current game.
- Declaring training broken whenever the opponent's loss rises.
- Drawing different noise for the plus and minus gradient-audit evaluations.

## Play the round

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
places the real and fake samples on one number line. Advance from the initial
forward pass to the discriminator move and generator response. The active
gradient route, detached or frozen boundary, both objectives, audited updates,
and counter-push in the opponent's loss remain visible together.

## Cross-language checkpoint

An NN18 consumer is conformant when it reproduces the initial fake and both
scores, the detached discriminator backward pass and update, the frozen-
discriminator generator backward pass and update, both numerical audits, and
all three objective snapshots.

Implement this scalar schedule directly in every host language first. A Rust
core can later batch stable sigmoid/BCE kernels, generator and discriminator
forwards, backward passes, and optimizer steps behind a stable C ABI. The ABI
must name the active player, frozen parameter buffers, update order, reduction
rule, and saved-noise ownership explicitly. Optional trace buffers should keep
detach boundaries, discriminator input gradients, and per-player losses
inspectable instead of trading understanding for speed.
