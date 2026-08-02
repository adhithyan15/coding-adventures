# A Two-Number Autoencoder Bottleneck, by Hand

An autoencoder tries to reproduce its input after forcing the information
through a constrained middle representation. The encoder compresses. The
decoder reconstructs. Reconstruction error teaches both sides what the middle
number must preserve.

This lesson uses the smallest undercomplete autoencoder: two input coordinates,
one scalar bottleneck, and two reconstructed coordinates. The deterministic
oracle is
[`00-linear-bottleneck-step.json`](../../specs/fixtures/two-number-autoencoder-v1/labs/00-linear-bottleneck-step.json),
and the language-neutral contract is
[`NN16-two-number-autoencoder-labs.md`](../../specs/NN16-two-number-autoencoder-labs.md).

## What the bottleneck changes

A two-number identity function could copy each input directly to its matching
output. That would teach us nothing. Instead, require this shape:

```text
[x0, x1] -> one number z -> [x_hat0, x_hat1]
```

Both original coordinates must influence `z`, and both reconstructions must be
created from that same `z`. The model cannot send two independent values across
the middle boundary.

For this first trace, the input is:

```text
x = [2,-1]
```

The hats in `x_hat` mean reconstructed estimates, not new target labels. In an
autoencoder, the input is also the target.

## Encode two values into one

Use a linear encoder with weights `[0.5,-0.25]` and bias `0`:

```text
z = x0 * encoder_weight0
  + x1 * encoder_weight1
  + encoder_bias

  = 2 * 0.5 + (-1) * (-0.25) + 0
  = 1 + 0.25
  = 1.25
```

The negative second input and negative second weight make a positive
contribution. The bottleneck is now one scalar, `1.25`.

This does not mean `1.25` has a universal human label. A learned representation
is useful because of how the encoder places examples and how the decoder uses
those positions, not because every coordinate has a name.

## Decode the same scalar along two branches

Use decoder weights `[1.2,-0.8]` and biases `[0.1,-0.2]`:

```text
x_hat0 = 1.25 * 1.2 + 0.1
       = 1.5 + 0.1
       = 1.6

x_hat1 = 1.25 * (-0.8) + (-0.2)
       = -1 - 0.2
       = -1.2
```

The reconstruction is:

```text
x_hat = [1.6,-1.2]
```

Both output branches received exactly the same bottleneck value. Their separate
weights and biases interpret it differently.

## Measure what compression failed to preserve

Define error as reconstruction minus input:

```text
error = x_hat - x
      = [1.6 - 2, -1.2 - (-1)]
      = [-0.4,-0.2]
```

Use mean squared error across the two coordinates:

```text
squared errors = [(-0.4)^2, (-0.2)^2]
               = [0.16,0.04]

loss = (0.16 + 0.04) / 2
     = 0.1
```

The model missed the first coordinate by `0.4` and the second by `0.2`. Squaring
makes both misses positive and penalizes the larger miss more strongly.

## Start backward at both reconstructions

For mean squared error over two outputs:

```text
d loss / d x_hat[j]
  = 2 * error[j] / 2
  = error[j]
```

Therefore:

```text
reconstruction gradients = [-0.4,-0.2]
```

Each decoder weight multiplied the bottleneck, so:

```text
d loss / d decoder_weight0 = -0.4 * 1.25 = -0.5
d loss / d decoder_weight1 = -0.2 * 1.25 = -0.25

d loss / d decoder_bias = [-0.4,-0.2]
```

Negative gradients mean that subtracting the gradient will increase these
parameters.

## Make both errors meet at the bottleneck

The scalar `z` fed both decoder branches. Its gradient must add both routes:

```text
from x_hat0: -0.4 * 1.2  = -0.48
from x_hat1: -0.2 * -0.8 =  0.16

d loss / d z = -0.48 + 0.16
             = -0.32
```

The two contributions partially cancel. This is ordinary gradient accumulation:
one saved value influenced several downstream calculations, so its derivative
is the sum of every route back from the loss.

## Continue through the encoder

The encoder used each input as a multiplier:

```text
d loss / d encoder_weight0 = -0.32 * 2    = -0.64
d loss / d encoder_weight1 = -0.32 * (-1) =  0.32
d loss / d encoder_bias                  = -0.32
```

The same bottleneck gradient reaches both encoder weights, scaled by the input
coordinate each weight saw during the forward pass.

## Audit all seven trainable scalars

The model has seven parameters:

```text
2 encoder weights + 1 encoder bias
+ 2 decoder weights + 2 decoder biases
= 7
```

For each parameter, perturb it by `epsilon = 0.000001`, rerun the complete
forward loss on both sides, and estimate a centered numerical slope:

```text
numerical gradient
  = (loss(parameter + epsilon) - loss(parameter - epsilon))
    / (2 * epsilon)
```

The largest absolute difference between backpropagation and the seven numerical
gradients is about `2.03e-11`. That tiny gap is floating-point rounding. The
independent calculation gives us evidence that the encoder and decoder gradient
routes were assembled correctly.

## Apply one SGD step

Use learning rate `0.1`:

```text
parameter_after = parameter_before - 0.1 * gradient
```

The updated parameters are:

```text
encoder weights = [0.564,-0.282]
encoder bias    = 0.032

decoder weights = [1.25,-0.775]
decoder biases  = [0.14,-0.18]
```

Rerun the entire autoencoder:

```text
z_after = 2 * 0.564 + (-1) * (-0.282) + 0.032
        = 1.442

x_hat_after = [1.442 * 1.25 + 0.14,
               1.442 * -0.775 - 0.18]
            = [1.9425,-1.29755]
```

The new errors and loss are:

```text
errors = [-0.0575,-0.29755]

loss = ((-0.0575)^2 + (-0.29755)^2) / 2
     = 0.04592112625
```

One coordinate became much more accurate while the other became less accurate,
yet their mean squared error fell from `0.1` to `0.04592112625`. A shared
bottleneck couples the reconstruction tradeoff.

## What this one-example model does not prove

This arithmetic is a real autoencoder step, but one example is not enough to
learn a generally useful representation. With a dataset, the same small
bottleneck must preserve regularities shared across many inputs rather than
memorize one pair of numbers.

A linear `2 -> 1 -> 2` autoencoder trained across centered data learns a
one-dimensional direction closely related to principal component analysis.
Nonlinear layers can learn curved representations, but only after we understand
this shared compression boundary.

## Common implementation bugs

- Giving the decoder the original input as well as the bottleneck, which lets it
  bypass compression.
- Training against a separate label instead of using the input as the target.
- Summing squared errors while using gradients for a mean reduction, or the
  reverse.
- Sending only one output's gradient into the bottleneck instead of adding both.
- Reusing decoder weights when computing encoder products.
- Applying an update before finishing every gradient from the saved forward pass.
- Trusting the analytical gradient without the independent numerical audit.
- Assuming a lower single-example loss proves a useful dataset-level embedding.

## Explore the bottleneck

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
shows both input coordinates converging into one scalar and the same scalar
branching into two reconstructions. Select either output to isolate its decoder
arithmetic and its contribution to the bottleneck gradient. Toggle the updated
parameters to see the coupled reconstruction tradeoff and the lower total loss.

## Cross-language checkpoint

An NN16 consumer is conformant when it reproduces encoder products, bottleneck,
decoder products, reconstructions, coordinate errors, mean loss, all decoder and
encoder gradients, the two bottleneck-gradient contributions, seven numerical
gradients, updated parameters, and the post-update forward trace.

Implement the scalar operations in every host language first. A Rust core may
later batch and fuse dense layers, reconstruction loss, backward reductions,
and optimizer updates behind a C ABI. Its fast path should retain optional trace
buffers so every binding and learning tool can explain the representation it
executes.
