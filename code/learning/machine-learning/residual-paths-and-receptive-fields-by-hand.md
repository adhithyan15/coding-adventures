# Residual Paths and Receptive Fields, by Hand

A deep spatial network is still made from local arithmetic, but two things
become harder to see as layers accumulate:

1. **How much of the original input can one output see?**
2. **Is there a short route through the network as well as the deep route?**

The first question is about the **receptive field**. The second is why residual
networks add **skip connections**.

This lesson uses five values and two three-tap layers. The complete
language-neutral oracle is
[`00-two-layer-residual.json`](../../specs/fixtures/residual-receptive-v1/labs/00-two-layer-residual.json).
The formulas and conformance rules are in
[`NN08-residual-receptive-labs.md`](../../specs/NN08-residual-receptive-labs.md).

## The tiny residual block

Start with:

```text
index:  0  1  2  3  4
input:  1  0  2  0  1
```

Both convolution layers use kernel `[1, 1, 1]`, stride one, and same zero
padding. At each position, add the left neighbor, the position itself, and the
right neighbor. Values outside the signal are zero.

The block has two routes:

```text
main path: input -> convolution 1 -> convolution 2 -> + -> ReLU
skip path: input -------------------------------------> +
```

The skip is the identity: it copies `input[i]` directly to the addition at the
same index.

## First local layer

At the center, index `2`:

```text
hidden[2] = input[1] + input[2] + input[3]
          = 0 + 2 + 0
          = 2
```

Doing the same operation at every position gives:

```text
hidden: 1  3  2  3  1
```

The boundary calculation includes padding. For example:

```text
hidden[0] = padding[-1] + input[0] + input[1]
          = 0 + 1 + 0
          = 1
```

Same padding preserves five positions; it does not invent five real input
values at the edge.

## Second local layer

The second layer performs the same operation on `hidden`:

```text
main[2] = hidden[1] + hidden[2] + hidden[3]
        = 3 + 2 + 3
        = 8
```

Across all positions:

```text
main: 4  6  8  6  4
```

The center output only reads three values from the immediately previous layer,
but those three hidden values were each made from three input values. We need
to expand the graph one more step to see the true receptive field.

## Open the center output all the way back to the input

The three hidden dependencies expand as:

```text
hidden[1] = input[0] + input[1] + input[2] = 1 + 0 + 2 = 3
hidden[2] = input[1] + input[2] + input[3] = 0 + 2 + 0 = 2
hidden[3] = input[2] + input[3] + input[4] = 2 + 0 + 1 = 3
```

Now count how many computational paths connect each input to `main[2]`:

```text
input index:       0  1  2  3  4
path count:        1  2  3  2  1
input value:       1  0  2  0  1
value x paths:     1  0  6  0  1
```

The expanded contributions still sum to the same main-path value:

```text
1 + 0 + 6 + 0 + 1 = 8
```

Two zeros make no numerical contribution, but they remain inside the
**structural receptive field**. If training or a new example changes them,
`main[2]` can change. A receptive field describes possible dependency, not only
the nonzero arithmetic in one example.

For `L` stride-one layers with odd kernel width `K`, no dilation, and the same
spatial resolution, an interior receptive-field width grows as:

```text
width = 1 + L * (K - 1)
```

Here `1 + 2 * (3 - 1) = 5`.

## Add the identity skip

The skip path does not use either convolution. At center index `2`, it copies:

```text
skip[2] = input[2] = 2
```

The residual addition combines the deep and short routes:

```text
preactivation[2] = main[2] + skip[2]
                 = 8 + 2
                 = 10
```

ReLU keeps `10` because it is positive. The whole block produces:

```text
main:          4  6   8  6  4
identity skip: 1  0   2  0  1
               ----------------
residual sum:  5  6  10  6  5
after ReLU:    5  6  10  6  5
```

Toggling the skip off does not change the main path or its receptive field. It
only removes the direct contribution, so the center falls from `10` back to
`8`.

## Why a short path helps deep networks

The identity route lets information and gradients travel through one addition
instead of relying entirely on every transformation in the main path. The main
path can learn a useful **residual change** while the skip preserves a simple
baseline.

This example does not prove that every residual network trains well. It exposes
the mechanism a larger network repeats:

```text
output = activation(transform(input) + compatible_skip(input))
```

If shapes differ, the skip cannot be a raw identity; it needs a projection or
resampling step. Shape compatibility is part of the residual contract, not a
minor implementation detail.

## What happens at the boundary?

Select output `0`. Layer two reads only `hidden[0]` and `hidden[1]`; its left
neighbor is padding. Those hidden positions reach actual input indices
`[0, 1, 2]`. The in-range receptive field is three positions rather than the
center's five.

The visualizer in
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
lets you select all five outputs. It keeps three facts separate:

- the immediate hidden positions read by layer two;
- the union of original input positions they can reach; and
- the direct identity value added by the skip.

## Cross-language checkpoint

An implementation is ready for the next architecture when it can load the same
JSON fixture and reproduce:

- both same-padded convolution outputs;
- every identity contribution, residual sum, and ReLU output;
- each output's hidden dependencies;
- each input's path multiplicity and numerical contribution; and
- the clipped receptive-field indices at both boundaries.

The Rust `dsp-conv` package can reproduce these two main-path layers directly
with zero boundaries because `[1, 1, 1]` is symmetric. Native language loops
and Rust-backed consumers must agree with the fixture before the block is fused
or accelerated. A debug path should remain available after fusion so “residual
block” never becomes another name for hidden arithmetic.
