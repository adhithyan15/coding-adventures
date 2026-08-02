# Normalization, Dropout, and Residual Paths, by Hand

NN23 showed how initialization shapes forward signals. NN24 followed gradients
backward. This lesson keeps one learned branch fixed and changes only the route
around it, so three often-mixed-up tools have visibly different jobs:

- **normalization** makes coordinates share scale information;
- **dropout** samples a smaller training-time subnetwork;
- a **residual path** adds a short identity route.

The language-neutral contract is
[NN25](../../specs/NN25-normalization-dropout-residual-labs.md), with the
canonical fixture in
[`00-four-route-comparison.json`](../../specs/fixtures/training-stabilizers-v1/labs/00-four-route-comparison.json).

## 1. One shared branch

Use a four-coordinate input and one scalar branch weight:

```text
input x       = [1, 1, 3, 3]
branch weight = 0.5
h             = weight * x
              = [0.5, 0.5, 1.5, 1.5]
```

Instead of hiding the reverse pass inside a large loss, seed it with one
visible upstream vector:

```text
g = dS/doutput = [1, 0, 0, -1]
S = dot(g, output)
```

`S` is a scalar score. Its gradient is a vector-Jacobian product: it asks how
the chosen weighted combination of outputs changes when an input or the branch
weight changes.

## 2. The plain route is the control

The plain route returns the branch unchanged:

```text
output = h = [0.5, 0.5, 1.5, 1.5]
S = 1 * 0.5 + 0 * 0.5 + 0 * 1.5 - 1 * 1.5
  = -1
```

The gradient entering `h` is just `g`. The branch multiplies every input by
`0.5`, so the reverse pass multiplies by the same local derivative:

```text
dS/dx = 0.5 * g
      = [0.5, 0, 0, -0.5]
```

The shared branch-weight gradient adds all four coordinate contributions:

```text
dS/dweight = dot(dS/dh, x)
            = 1*1 + 0*1 + 0*3 - 1*3
            = -2
```

## 3. Layer normalization shares statistics

Normalize the four values of `h` together. NN25 uses population variance and
omits the tiny numerical epsilon so every number can be calculated by hand.
The fixture has nonzero variance, so division remains safe.

First find the mean:

```text
mean = (0.5 + 0.5 + 1.5 + 1.5) / 4 = 1
```

Center the coordinates and find their population variance:

```text
centered = [-0.5, -0.5, 0.5, 0.5]
variance = (0.25 + 0.25 + 0.25 + 0.25) / 4 = 0.25
standard deviation = sqrt(0.25) = 0.5
```

Divide every centered coordinate by that shared standard deviation:

```text
normalized output = [-1, -1, 1, 1]
S = 1*(-1) + 0*(-1) + 0*1 - 1*1 = -2
```

The reverse pass is coupled: one output gradient can affect other input
coordinates because mean and variance used all four values. For `N = 4`:

```text
dS/dh[i] =
  (N*g[i] - sum(g) - normalized[i]*sum(g*normalized))
  / (N*standard_deviation)
```

The two shared sums are:

```text
sum(g) = 0
sum(g * normalized) = -2
```

For coordinate 1:

```text
dS/dh[1] = (4*1 - 0 - (-1)*(-2)) / (4*0.5)
          = (4 - 2) / 2
          = 1
```

For coordinate 2, the direct upstream gradient is zero, but shared statistics
still produce a gradient:

```text
dS/dh[2] = (4*0 - 0 - (-1)*(-2)) / 2 = -1
```

The complete reverse vectors are:

```text
dS/dh = [1, -1, 1, -1]
dS/dx = 0.5 * dS/dh
      = [0.5, -0.5, 0.5, -0.5]
```

The shared scalar weight gradient is zero:

```text
1*1 - 1*1 + 1*3 - 1*3 = 0
```

That is not a failed gradient. With zero epsilon and a positive scalar weight,
layer normalization removes that common scale, so changing only the scale does
not change this normalized output.

Real implementations add a small positive epsilon inside the square root and
usually learn per-coordinate `gamma` and `beta`. NN25 isolates the core
centering and scaling calculation.

## 4. Inverted dropout samples a route

Use keep probability `0.5` and pin one training mask:

```text
mask = [1, 0, 1, 0]
mask / keep_probability = [2, 0, 2, 0]
```

Inverted dropout rescales kept values during training:

```text
training output = h * mask / 0.5
                = [1, 0, 3, 0]
S = 1
```

At evaluation time dropout is off:

```text
evaluation output = h = [0.5, 0.5, 1.5, 1.5]
```

For a Bernoulli mask, `E[mask] = keep_probability`, so each expected scaled
mask coordinate is one:

```text
E[mask / keep_probability] = 1
E[training output] = h
```

That equality is an expectation over many masks, not a promise that this one
training pass equals evaluation.

The same scaled mask appears in the reverse pass:

```text
dS/dh = g * mask / 0.5
      = [2, 0, 0, 0]
dS/dx = 0.5 * dS/dh
      = [1, 0, 0, 0]
dS/dweight = dot(dS/dh, x) = 2
```

The fourth upstream coordinate was `-1`, but its mask was zero, so this sampled
subnetwork sends no gradient through that coordinate.

## 5. A residual path adds an identity route

The residual route adds the original input to the learned branch:

```text
output = x + h
       = [1.5, 1.5, 4.5, 4.5]
S = 1.5 - 4.5 = -3
```

The upstream gradient splits. One copy crosses the learned branch and is
multiplied by `0.5`; another crosses the identity skip with derivative `1`:

```text
branch contribution = 0.5 * g = [0.5, 0, 0, -0.5]
skip contribution   = 1.0 * g = [1, 0, 0, -1]
total dS/dx          = [1.5, 0, 0, -1.5]
```

The skip does not replace the learned branch. It gives the forward signal and
the reverse gradient a shorter additional route. Shapes must still match, or a
projection must make them compatible before addition.

## 6. Put the four routes side by side

| Route | Output | `dS/dx` | `dS/dweight` |
| --- | --- | --- | ---: |
| Plain | `[0.5, 0.5, 1.5, 1.5]` | `[0.5, 0, 0, -0.5]` | `-2` |
| Layer normalization | `[-1, -1, 1, 1]` | `[0.5, -0.5, 0.5, -0.5]` | `0` |
| Inverted dropout | `[1, 0, 3, 0]` | `[1, 0, 0, 0]` | `2` |
| Identity residual | `[1.5, 1.5, 4.5, 4.5]` | `[1.5, 0, 0, -1.5]` | `-2` |

Normalization couples coordinates. Dropout changes the sampled training graph.
Residual addition creates a second path. They can coexist in one architecture,
but they are not interchangeable remedies.

## 7. Check every gradient independently

For each input coordinate, perturb only that coordinate:

```text
dS/dx[i] ~= (S(x[i] + epsilon) - S(x[i] - epsilon)) / (2 * epsilon)
```

Perturb the scalar branch weight the same way. NN25 uses epsilon `0.000001` and
checks four input gradients plus one weight gradient for each route. All twenty
analytical values agree with their central finite differences within `1e-8`.

The dropout mask stays fixed during this audit. Changing the mask between the
positive and negative evaluations would compare two different sampled
functions instead of checking one derivative.

## 8. Practical boundaries

- Normalization requires an explicit axis, epsilon, and learned-affine policy.
- Dropout requires a training/evaluation mode and reproducible random-state or
  caller-supplied mask for tests.
- Residual addition requires compatible shapes and clear aliasing rules.
- None guarantees a deep model will train; inspect activations, gradients,
  updates, and data together.

## 9. Validate the deterministic corpus

```text
python code/scripts/validate_training_stabilizer_labs.py
```

The validator rejects unknown or duplicate keys, non-finite vectors, malformed
masks, changed conventions, route reordering, trace drift, and failed input or
weight finite differences.

## 10. Cross-language and Rust-core path

Implement the four vector routes directly in each host language first. That
pins population statistics, inverted-mask scaling, residual gradient splitting,
and vector-Jacobian-product order before tensor broadcasting enters the design.

A Rust core can later fuse normalization, dropout, residual addition, and their
backward kernels over contiguous buffers. A stable C ABI or WASM surface should
accept dimensions, normalization axes and epsilon, training mode, a seed or
caller-provided mask, and residual buffers. An optional trace mode should expose
the saved statistics and split gradients so every language can still unpack
the optimized result.
