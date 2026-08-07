# Convolution, Window by Window

A dense neuron looks at every input. A convolutional neuron starts smaller: it
looks at one local window, then reuses the same weights at the next position.
That repeated little calculation is the foundation of convolutional networks.

## The Smallest Example

Use this six-value signal and three-value kernel:

```text
signal = [2,  1, 3, 0, 4, 2]
kernel = [1, -1, 2]
```

Start with the kernel over the first three signal values:

```text
window   = [2,  1, 3]
kernel   = [1, -1, 2]
products = [2, -1, 6]

accumulator: 0 -> 2 -> 1 -> 7
```

So the first output is `7`. Move the same kernel one position right and repeat:

| Output | Signal window | Products | Running sum | Result |
| --- | --- | --- | --- | ---: |
| `y[0]` | `[2, 1, 3]` | `[2, -1, 6]` | `0 -> 2 -> 1 -> 7` | 7 |
| `y[1]` | `[1, 3, 0]` | `[1, -3, 0]` | `0 -> 1 -> -2 -> -2` | -2 |
| `y[2]` | `[3, 0, 4]` | `[3, 0, 8]` | `0 -> 3 -> 3 -> 11` | 11 |
| `y[3]` | `[0, 4, 2]` | `[0, -4, 4]` | `0 -> 0 -> -4 -> 0` | 0 |

The feature map is therefore:

```text
[7, -2, 11, 0]
```

For signal length `N` and kernel length `K`, valid stride-one mode produces
`N - K + 1` outputs. Here that is `6 - 3 + 1 = 4`.

## What the Kernel Means

The three kernel numbers are a tiny feature detector. Positive weights reward
values in particular positions; negative weights subtract evidence. The
important architectural idea is **weight sharing**: all four outputs use the
same `[1, -1, 2]`. A dense layer would need separate weights for each position.

Sharing gives the model a useful bias: if a pattern matters at the left of a
signal, a similar pattern can matter at the right. It also reduces the number
of trainable parameters.

Each output sees only three values. That local window is its **receptive
field**. Later layers combine neighboring features, so their effective
receptive fields become larger.

## “Convolution” Without a Kernel Flip

Mathematical convolution reverses the kernel before sliding it. Most neural
network libraries do not; they compute cross-correlation but conventionally
call the operation convolution.

NN05 follows the neural convention:

```text
y[i] = sum(signal[i + j] * kernel[j])
```

The asymmetric kernel makes the convention visible. Reversing it to
`[2, -1, 1]` would produce `[6, -1, 10, -2]`, not the canonical
`[7, -2, 11, 0]`.

## From One Dimension to an Image

Nothing mysterious changes for an image. The kernel gains a height as well as
a width, so it slides across rows and columns. Color images add an input-channel
loop. Multiple learned detectors add an output-channel loop. Batches add one
more outer loop.

Before adding those axes, be able to point to every value in this 1D trace and
answer:

1. Which signal values were visible to this output?
2. Which shared weight multiplied each value?
3. What were the products and partial sums?
4. Why are there four outputs instead of six?

The interactive **Spatial** view in the
[ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md)
lets you slide the kernel, edit both arrays, and inspect those answers.

## Portable and Fast Implementations

The language-neutral fixture at
`code/specs/fixtures/convolution-learning-v1` pins the signal, kernel,
convention, complete trace, and tolerance. Any language can implement the
small loop directly and validate it with:

```text
python code/scripts/validate_convolution_learning_labs.py
```

Performance-oriented consumers can later call a Rust core through a stable C
ABI. The existing Rust `dsp-conv` package performs centered mathematical
convolution, so an adapter must reverse the neural kernel and select only the
valid outputs implied by its centering convention. A native cross-correlation
kernel is also reasonable. In either case, the NN05 fixture remains the oracle:
acceleration is correct only when its outputs match the simple loop.

The next tranche makes the kernel trainable and traces how every shared weight
receives gradient contributions from all output positions.
