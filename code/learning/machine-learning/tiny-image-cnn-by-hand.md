# A Tiny Image CNN, Completely by Hand

A convolutional neural network does not receive a picture as a special object.
It receives stacks of number grids. A color image often has red, green, and
blue grids. This lab uses two simpler channels:

- **vertical position** says how far down a pixel is; and
- **horizontal position** says how far right a pixel is.

```text
vertical-position        horizontal-position

0  0  0                  0  1  2
1  1  1                  0  1  2
2  2  2                  0  1  2
```

The complete language-neutral oracle is
[`00-two-channel-image.json`](../../specs/fixtures/tiny-image-cnn-v1/labs/00-two-channel-image.json).
The formulas and conformance rules are in
[`NN07-tiny-image-cnn-labs.md`](../../specs/NN07-tiny-image-cnn-labs.md).

## What changes when a signal becomes an image?

The one-dimensional lab slid left to right. An image kernel slides across rows
*and* columns. There is one more important loop: a filter has one small kernel
for every input channel.

At one output location, the network:

1. takes a `2 x 2` window from input channel 0;
2. multiplies it by filter 0's channel-0 kernel and adds the products;
3. repeats that for input channel 1;
4. adds the two channel sums and one filter bias.

That final number belongs to one **output channel**, also called a feature map.
Using two filters gives us two output channels.

## Filter 0: larger toward the bottom-right

Filter 0 has these channel-specific kernels and bias `0`:

```text
vertical kernel          horizontal kernel

4  0                     2  0
0  0                     0  0
```

Select output row `1`, column `1`. Its two input windows are:

```text
vertical window          horizontal window

1  1                     1  2
2  2                     1  2
```

The channel calculations are:

```text
vertical:   1x4 + 1x0 + 2x0 + 2x0 = 4
horizontal: 1x2 + 2x0 + 1x0 + 2x0 = 2
bias:                                            0
                                                --
output[0, 1, 1] = 4 + 2 + 0 =                  6
```

Repeating the same weights at all four valid positions gives:

```text
vertical contribution    horizontal contribution    convolution

0  0                     0  2                       0  2
4  4                     0  2                       4  6
```

This is the essential channel rule: **correlate each input channel, then add
the matching spatial positions**. Channels do not make separate final images
unless separate filters learn to preserve them.

## Filter 1: larger toward the top-left

Filter 1 reverses the signs and adds bias `6`:

```text
vertical kernel          horizontal kernel

-4  0                    -2  0
 0  0                     0  0
```

At row `1`, column `1`:

```text
vertical:   1x-4 + 1x0 + 2x0 + 2x0 = -4
horizontal: 1x-2 + 2x0 + 1x0 + 2x0 = -2
bias:                                             +6
                                                   --
output[1, 1, 1] = -4 + -2 + 6 =                    0
```

Its complete feature map points in the opposite direction:

```text
6  4
2  0
```

## Why normalize a feature map?

The two filters use different weights and biases, yet both maps have mean `3`.
For filter 0:

```text
mean = (0 + 2 + 4 + 6) / 4 = 3
```

Population variance measures the average squared distance from that mean:

```text
variance = ((0-3)^2 + (2-3)^2 + (4-3)^2 + (6-3)^2) / 4
         = (9 + 1 + 1 + 9) / 4
         = 5
```

Normalization divides by `sqrt(variance + epsilon)`. This teaching fixture
uses `epsilon = 4`, so the denominator is exactly:

```text
sqrt(5 + 4) = 3
```

With scale `gamma = 1` and shift `beta = 0`, filter 0 becomes:

```text
(value - 3) / 3

-1    -1/3
 1/3   1
```

Filter 1 contains the same values in reverse order, so it has the same mean,
variance, and denominator:

```text
 1     1/3
-1/3  -1
```

The large epsilon is chosen only to keep this first example exact on paper. In
real networks epsilon is usually much smaller. The key idea is unchanged: a
normalization layer defines a group of values that share statistics, then
recenters and rescales that group. Batch, instance, and layer normalization
differ mainly in which axes belong to the group.

## ReLU keeps positive evidence

ReLU replaces every negative number with zero:

```text
filter 0 after ReLU       filter 1 after ReLU

0    0                   1    1/3
1/3  1                   0    0
```

The two channels now say where each learned pattern has positive evidence.

## Max pooling keeps the strongest location

A `2 x 2` max pool covers each entire activated feature map:

```text
filter 0: max(0, 0, 1/3, 1) = 1 at row 1, column 1
filter 1: max(1, 1/3, 0, 0) = 1 at row 0, column 0
```

Pooling throws away detail on purpose. It retains a strong response while
making the next representation smaller. The argmax location matters during
backpropagation because only the winning input receives the gradient through a
max-pool operation.

## The whole pipeline

```text
two 3x3 channels
  -> two filters, each with two 2x2 kernels
  -> two 2x2 convolution maps
  -> normalize each output channel over four positions
  -> ReLU
  -> one max-pooled value per output channel
```

The visualizer in
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
lets you select a filter, spatial output, input channel, and pipeline stage. Use
it to answer three questions without trusting a tensor library:

1. Which exact input values and kernel values made this output?
2. Which values shared normalization statistics?
3. Which location survived pooling, and which locations were discarded?

## Cross-language checkpoint

An implementation is ready for the next lesson when it can load the same JSON
fixture and reproduce:

- every per-input-channel contribution map;
- both biased convolution maps;
- both means, variances, denominators, and normalized maps;
- the ReLU maps; and
- pooled values plus row-major argmax coordinates.

Native loops are enough for this tiny oracle. A performant language binding can
later pass contiguous image, kernel, bias, and output buffers to a Rust core
through a stable C ABI. Either route must keep an inspectable mode so fusion and
acceleration never put the arithmetic back behind “magic.”
