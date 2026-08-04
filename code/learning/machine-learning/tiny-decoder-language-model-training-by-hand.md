# A Tiny Decoder-Only Language Model Training Step, by Hand

A decoder-only language model learns one deceptively simple task: given every
token it is allowed to see so far, predict the token that comes next. The
attention block from the previous lessons builds a context-aware state. A
shared vocabulary head turns that state into guesses, cross-entropy measures
the surprise, and gradients make the next guess a little less surprising.

This lesson traces that full boundary with two positions and three vocabulary
items. The deterministic oracle is
[`00-two-position-next-token-step.json`](../../specs/fixtures/tiny-decoder-training-v1/labs/00-two-position-next-token-step.json),
and the language-neutral contract is
[`NN15-tiny-decoder-training-labs.md`](../../specs/NN15-tiny-decoder-training-labs.md).

## Shift one sequence into training examples

Start with a three-token sequence:

```text
red blue purple
```

A decoder does not copy the token at its current target position into its
input. Shift the same sequence by one:

```text
position 0: input prefix [red]       -> target blue
position 1: input prefix [red, blue] -> target purple
```

The second position may use red and blue, but never future purple. That causal
boundary is what makes training match left-to-right generation.

## Start at a visible decoder boundary

Suppose a tiny causal decoder block has already produced these saved states:

```text
h_red  = [1,0]
h_blue = [0,1]
```

They are deliberately simple so the vocabulary-head arithmetic fits on paper.
NN15 freezes the decoder body for one step, updates only the head, and records
the gradient flowing into each saved state. Later, automatic differentiation
can continue that gradient through layer normalization, residual paths,
attention, and embeddings.

## Turn a state into vocabulary logits

The vocabulary order is `[red, blue, purple]`. Use one shared unembedding
matrix and one shared bias at both positions:

```text
                 red  blue  purple
unembedding = [[  1,    0,    -1],   <- state coordinate 0
               [  0,    1,    -1]]   <- state coordinate 1

bias = [0,0,0]
```

For the first state `[1,0]`, compute one dot product per vocabulary item:

```text
red logit    = 1 * 1  + 0 * 0  + 0 =  1
blue logit   = 1 * 0  + 0 * 1  + 0 =  0
purple logit = 1 * -1 + 0 * -1 + 0 = -1

logits = [1,0,-1]
```

A logit is an unnormalized preference. It is not yet a probability.

## Apply stable softmax

Subtract the largest logit before exponentiating:

```text
row maximum = 1
shifted     = [0,-1,-2]
exp         = [1,0.367879,0.135335]
denominator = 1.503215

probabilities = [0.665241,0.244728,0.090031]
```

The target is blue, so this position assigned its correct answer probability
`0.244728`.

At the second position, state `[0,1]` produces logits `[0,1,-1]` and
probabilities:

```text
[0.244728,0.665241,0.090031]
```

That distribution prefers blue, but the target is purple. The model is quite
confident in the wrong direction.

## Convert target probability into loss

Cross-entropy for one one-hot target is just negative log target probability:

```text
loss at red  = -ln(P(blue))   = -ln(0.244728) = 1.407606
loss at blue = -ln(P(purple)) = -ln(0.090031) = 2.407606

mean loss = (1.407606 + 2.407606) / 2
          = 1.907606
```

The second prediction contributes more loss because it gave the correct token
less probability. Cross-entropy is a smooth measure of surprise.

## Differentiate logits without magic

Softmax plus cross-entropy has a compact derivative. For a mean over two
positions:

```text
d mean loss / d logit
  = (probability - one_hot_target) / 2
```

For the first target, blue:

```text
one-hot target = [0,1,0]
logit gradient = ([0.665241,0.244728,0.090031] - [0,1,0]) / 2
               = [0.332620,-0.377636,0.045015]
```

The blue coordinate is negative. SGD subtracts the gradient, so the blue logit
will rise. Positive non-target gradients make those logits fall.

For the second target, purple:

```text
logit gradient = [0.122364,0.332620,-0.454985]
```

Every row sums to zero. Softmax redistributes probability; increasing one share
requires decreasing others.

## Send error into the decoder state

Multiply the logit gradients by the transpose of the original unembedding:

```text
d loss / d h_red  = [ 0.287605,-0.422651]
d loss / d h_blue = [ 0.577349, 0.787605]
```

NN15 stops parameter updates at this boundary but does not hide these values.
They are the exact signals a full decoder backward pass would continue through
the causal block.

## Reduce gradients into shared parameters

The same unembedding and bias served both positions. Each position contributes
an outer product of its state and its logit gradient. Add those contributions:

```text
unembedding gradient =
  [[ 0.332620,-0.377636, 0.045015],
   [ 0.122364, 0.332620,-0.454985]]

bias gradient =
  [0.454985,-0.045015,-0.409969]
```

Do not average these values a second time. The factor `1/2` is already inside
each logit gradient because the objective is the mean loss.

## Audit the gradient independently

Backpropagation and a numerical gradient should reach the same answer by
different routes. Perturb each trainable scalar by `epsilon = 0.000001`, rerun
the mean loss on both sides, and measure the centered slope:

```text
numerical gradient
  = (loss(parameter + epsilon) - loss(parameter - epsilon))
    / (2 * epsilon)
```

Across all nine unembedding and bias parameters, the largest absolute
difference from the analytical gradients is about `1.98e-10`. That tiny error
is expected floating-point rounding and gives us an independent reason to
trust the backward trace before changing parameters.

## Apply one SGD update

With learning rate `0.5`:

```text
parameter_after = parameter_before - 0.5 * gradient
```

The updated bias is:

```text
[-0.227492,0.022508,0.204985]
```

Rerun both forward passes with the updated head:

```text
P(blue | red)        0.244728 -> 0.351913
P(purple | red blue) 0.090031 -> 0.154460

mean loss            1.907606 -> 1.456094
```

Both correct-token probabilities rose, and the objective fell. That is one
complete language-model training step.

## What this tiny model leaves out

The arithmetic is real, but the scope is controlled:

- The vocabulary contains three items rather than tens of thousands.
- There are two training positions rather than a packed batch of sequences.
- Decoder states have width two and are saved rather than recomputed here.
- The output head is trained while the decoder body is frozen for this step.
- SGD has no momentum, adaptive moments, weight decay, or gradient clipping.

Each later feature scales one boundary without changing the basic loop:

```text
causal prefixes -> decoder states -> logits -> probabilities -> loss
                <- state gradient <- logit gradient <- target
```

## Common implementation bugs

- Pairing each input token with itself instead of the next token.
- Letting the second training position see future purple.
- Applying softmax across positions instead of across vocabulary items.
- Computing `log(softmax)` from unstable raw exponentials.
- Using the largest probability rather than the target probability in the loss.
- Forgetting the `1 / position_count` factor for a mean objective.
- Averaging shared gradients again after the mean factor is already applied.
- Trusting a hand-derived gradient without an independent numerical audit.
- Updating each position's private copy instead of one shared unembedding.
- Recomputing state gradients with the updated weights instead of the weights
  used in the saved forward pass.

## Explore the training trace

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
keeps the two shifted positions aligned, lets you select either prediction, and
follows its state through logits, stable probabilities, target surprise, and
the compact logit gradient. Switch to the post-update view to see both target
probabilities rise and the mean loss fall while the causal prefixes stay fixed.

## Cross-language checkpoint

An NN15 consumer is conformant when it reproduces the sequence shift, causal
prefixes, every unembedding product, stable-softmax intermediate, target
probability, position and mean losses, state gradients, per-position and
reduced parameter gradients, updated parameters, and post-update loss.

Implement those scalar loops in every host language first. A Rust core may
later fuse matrix multiplication and softmax cross-entropy or update batched
buffers through a C ABI. Its optimized path must retain an optional trace mode
so language bindings and teaching tools can explain the same step they execute.
