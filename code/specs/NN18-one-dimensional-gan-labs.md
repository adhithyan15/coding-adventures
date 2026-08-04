# NN18: One-Dimensional GAN Game Labs

## Status

Draft specification for deterministic, language-neutral one-dimensional
generative adversarial network forward, alternating-update, gradient-audit, and
counter-move traces.

## Purpose

NN18 turns adversarial training into the smallest possible game. A scalar
generator maps one saved noise value to one fake point. A scalar discriminator
assigns real probabilities to one real point and the fake point. The
discriminator moves first; the generator then responds against the updated,
frozen discriminator.

The trace exposes three facts that large GANs can hide:

- discriminator gradients combine a real example and a detached fake example;
- generator gradients travel through the discriminator without updating it;
- improving one player can worsen the other player's objective.

## Model Contract

V1 uses scalar affine functions and a sigmoid discriminator:

```text
fake = generator_weight * noise + generator_bias
discriminator_logit(x) = discriminator_weight * x + discriminator_bias
discriminator_probability(x) = sigmoid(discriminator_logit(x))
```

The fixture provides one real sample and one saved noise value. Consumers do not
draw new noise while replaying the document.

## Loss Contract

The discriminator uses mean binary cross-entropy over the real label `1` and
fake label `0`:

```text
discriminator_loss
  = -0.5 * (log(real_probability) + log(1 - fake_probability))
```

The generator uses the non-saturating objective, asking the discriminator to
classify the fake point as real:

```text
generator_loss = -log(fake_probability)
```

Probabilities are derived with the stable sigmoid and losses are evaluated from
finite canonical logits.

## Discriminator Step Contract

Hold the fake point fixed and detached. For mean binary cross-entropy:

```text
real_logit_gradient = 0.5 * (real_probability - 1)
fake_logit_gradient = 0.5 * fake_probability

discriminator_weight_gradient
  = real_logit_gradient * real_sample
  + fake_logit_gradient * fake_sample
discriminator_bias_gradient
  = real_logit_gradient + fake_logit_gradient
```

Apply the discriminator learning rate, then rerun both classifications and both
reported losses without moving the generator.

## Generator Counter-Step Contract

Freeze the updated discriminator. The non-saturating generator loss gives:

```text
fake_logit_gradient = fake_probability - 1
fake_value_gradient
  = fake_logit_gradient * updated_discriminator_weight

generator_weight_gradient = fake_value_gradient * noise
generator_bias_gradient = fake_value_gradient
```

Apply the generator learning rate and rerun the fake point through the frozen
updated discriminator. The canonical generator loss must fall. The reported
discriminator loss may rise because the generator has just made its fake harder
to reject.

## Numerical Audit Contract

Use centered finite differences with `epsilon = 1e-6`:

1. Audit discriminator weight and bias on the initial discriminator objective,
   holding the fake sample detached and fixed.
2. Audit generator weight and bias on the generator objective, holding the
   updated discriminator fixed and reusing the saved noise.

The two audits deliberately target different objectives and moments in the
alternating schedule. There is no single joint gradient for this game round.

## Fixture Layout

```text
code/specs/fixtures/one-dimensional-gan-v1/
  schema.json
  labs/00-discriminator-generator-round.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields,
unsupported metadata, and trace values outside the declared absolute tolerance.

## Conformance Levels

1. **Generate:** reproduce the scalar fake point from saved noise.
2. **Judge:** reproduce both logits, probabilities, and initial losses.
3. **Discriminate:** detach the fake point, reproduce both classification
   gradient routes, update only the discriminator, and lower its loss.
4. **Counter:** freeze the updated discriminator, pass its input gradient into
   the generator, update only the generator, and lower generator loss.
5. **Audit:** reproduce both two-parameter centered finite-difference checks.
6. **Replay:** reproduce the final fake point and the counter-move's effect on
   both players' reported losses.

## Cross-Language and Rust-Core Direction

Every host language should implement the scalar schedule directly first. A fast
Rust core can later expose batched generator and discriminator forwards, stable
binary-cross-entropy kernels, detach boundaries, backward kernels, and optimizer
steps through a stable C ABI.

The ABI must make batch sizes, real/fake labels, saved-noise ownership, player
being updated, frozen parameter buffers, update order, reduction rules, and
caller-owned outputs explicit. Trace mode must optionally return real/fake
logits, probabilities, per-example gradient routes, detached fake values, input
gradients crossing into the generator, and both player losses at every phase.
