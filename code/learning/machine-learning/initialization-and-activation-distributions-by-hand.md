# Initialization and Activation Distributions, by Hand

A deep network repeats the same two operations many times:

```text
inputs x weights -> preactivation -> activation
```

If the first weights are too small, signals can fade toward zero. If they are
too large, ReLU values can grow rapidly while bounded activations such as
`tanh` can become pinned near their limits. **Initialization** is the choice of
the network's starting parameter values, before any training step occurs.

The language-neutral contract is
[NN23](../../specs/NN23-initialization-activation-distribution-labs.md), with
the canonical fixture in
[`00-three-layer-scale-comparison.json`](../../specs/fixtures/initialization-activation-distribution-v1/labs/00-three-layer-scale-comparison.json).

## 1. Keep the experiment tiny and controlled

Use four two-number inputs:

```text
[ 1,  0]
[ 0,  1]
[-1,  0]
[ 0, -1]
```

Every layer has two inputs and two outputs. Reuse this sign template three
times:

```text
base weights = [[1, -1],
                [1,  1]]
```

Real initializers sample random weights. This lesson fixes the signs so that
only scale changes and every language sees exactly the same arithmetic.

## 2. Four scaling rules

The **fan-in** is the number of values entering one neuron. Here it is `2` in
every layer.

| Initializer | Scale | With fan-in 2 |
| --- | ---: | ---: |
| Tiny control | `0.1` | `0.1` |
| Xavier | `sqrt(1 / fan_in)` | `0.707107` |
| He | `sqrt(2 / fan_in)` | `1` |
| Large control | `2` | `2` |

Xavier scaling is a common match for symmetric activations such as `tanh`. He
scaling allows more spread before ReLU discards negative values. The tiny and
large controls intentionally demonstrate failure directions.

## 3. One Xavier calculation by hand

Scale the template by `1 / sqrt(2)`:

```text
W = [[ 0.707107, -0.707107],
     [ 0.707107,  0.707107]]
```

Open sample `[1, 0]` and the first neuron:

```text
z_0 = 1 * 0.707107 + 0 * 0.707107
    = 0.707107

tanh(z_0) = 0.608859
```

The second neuron receives:

```text
z_1 = 1 * -0.707107 + 0 * 0.707107
    = -0.707107

tanh(z_1) = -0.608859
```

Across all four samples, the eight first-layer activations are four positive
and four negative copies of `0.608859`.

## 4. Summarize a distribution

A **distribution** here means the collection of all eight activation values in
one layer. Use population statistics because these eight values are the whole
toy batch, not a sample estimating a larger batch:

```text
mean     = sum(values) / 8
variance = sum((value - mean)^2) / 8
std dev  = sqrt(variance)
```

For Xavier plus `tanh`, layer 1 has mean `0` and standard deviation
`0.608859`. Continue the same matrix multiplication through all three layers:

```text
layer standard deviations = [0.608859, 0.492713, 0.456367]
```

The signal narrows gently rather than collapsing immediately.

## 5. See four different outcomes

With ReLU, the scale differences accumulate visibly:

| Initializer | Layer 1 std | Layer 2 std | Layer 3 std |
| --- | ---: | ---: | ---: |
| Tiny | `0.05` | `0.006960` | `0.000857` |
| Xavier | `0.353553` | `0.347985` | `0.302980` |
| He | `0.5` | `0.695971` | `0.856957` |
| Large | `1` | `2.783882` | `6.855655` |

The tiny signal nearly disappears. The deliberately large signal grows by
almost seven times the original input standard deviation.

ReLU also turns negative preactivations into exact zeros. The zero fraction in
this fixed experiment is `50%`, `50%`, then `62.5%`.

With large weights and `tanh`, `100%` of layer 1 values, `50%` of layer 2, and
`100%` of layer 3 have absolute magnitude at least `0.95`. Those values are
called **saturated** here: changing the preactivation has little effect on the
bounded output.

## 6. What initialization does and does not do

A useful initializer keeps forward signals informative long enough for
training to begin. It does not guarantee learning, and the best rule depends
on the activation, width, architecture, normalization, and optimizer.

This forward experiment also does not yet trace gradients. The next lesson
will reverse a similar network and show why repeated small or large derivative
factors produce vanishing or exploding gradients.

## 7. Validate the deterministic corpus

```text
python code/scripts/validate_initialization_activation_distribution_labs.py
```

The validator rejects unknown or duplicate keys, non-finite or ragged
matrices, incompatible layer shapes, changed conventions, and any activation or
summary that drifts beyond the fixture tolerance.

## 8. Cross-language and Rust-core path

Implement the fixed sign templates first. That separates matrix and statistics
parity from pseudorandom-generator parity.

A Rust core can accept row-major input and template buffers, derive each layer's
scale, fuse matrix multiplication with the activation, and reduce population
statistics with SIMD. C ABI and WASM bindings can expose the same buffers to
other languages. An explain mode should retain preactivation and activation
buffers even when the fast path fuses those operations.

## 9. Next experiment

Change the layer width while keeping the raw sign pattern. Xavier and He scales
change automatically because fan-in changes; the fixed `0.1` and `2` controls
do not. That experiment shows why useful initializers depend on network shape.
