# Vanishing and Exploding Gradients, by Hand

NN23 followed activation values forward. Training must also send information in
the opposite direction: a loss gradient travels backward through every layer.
At each step the chain rule multiplies by another local slope.

If those factors are repeatedly smaller than one, the gradient can
**vanish**. If they are repeatedly larger than one, it can **explode**.

The language-neutral contract is
[NN24](../../specs/NN24-gradient-flow-labs.md), with the canonical fixture in
[`00-four-layer-gradient-flow.json`](../../specs/fixtures/gradient-flow-v1/labs/00-four-layer-gradient-flow.json).

## 1. One scalar layer

Use a bias-free scalar layer:

```text
z = weight * input
a = activation(z)
```

The slope from the layer's output back to its input is its **local Jacobian**:

```text
da/dinput = weight * activation'(z)
```

For a chain of four layers, the output-to-input slope is the product of four
local Jacobians:

```text
da4/da0 = J1 * J2 * J3 * J4
```

## 2. An exploding ReLU chain

Start with input `1`, use four weights equal to `2`, and apply ReLU after each
multiply. Every preactivation is positive, so every ReLU derivative is `1`.

The forward pass is exact:

| Layer | Input | Weight | Activation |
| ---: | ---: | ---: | ---: |
| 1 | `1` | `2` | `2` |
| 2 | `2` | `2` | `4` |
| 3 | `4` | `2` | `8` |
| 4 | `8` | `2` | `16` |

Use target `0` and half squared error:

```text
L = 0.5 * (16 - 0)^2 = 128
dL/da4 = 16
```

Each local Jacobian is:

```text
weight * ReLU'(z) = 2 * 1 = 2
```

So the chain product is:

```text
da4/da0 = 2 * 2 * 2 * 2 = 16
```

Multiply the loss's output gradient by that path:

```text
dL/da0 = dL/da4 * da4/da0
        = 16 * 16
        = 256
```

The reverse values grow as they move toward the first layer:

```text
16 -> 32 -> 64 -> 128 -> 256
```

That is an exploding gradient in miniature.

## 3. Open one reverse step

At layer 3, the upstream gradient is `32`, the ReLU derivative is `1`, the
weight is `2`, and the saved forward input is `4`:

```text
dL/dz3     = 32 * 1 = 32
dL/dinput3 = 32 * 2 = 64
dL/dweight3 = 32 * 4 = 128
```

The saved forward value matters for the parameter gradient. Reverse-mode
automatic differentiation stores or recomputes such values for exactly this
reason.

## 4. A small-weight tanh chain vanishes

Now use four weights equal to `0.5` and `tanh`. The four local Jacobians are:

```text
[0.393224, 0.474228, 0.493612, 0.498406]
```

Their product is only:

```text
0.393224 * 0.474228 * 0.493612 * 0.498406
= 0.045877
```

The output error is `0.056456`, so the gradient reaching the input is:

```text
0.056456 * 0.045877 = 0.002590
```

Early layers receive a much quieter learning signal than the output layer.

## 5. Large weights can still vanish

Four `tanh` weights equal to `3` produce activations near `0.995`. Near the
flat ends of `tanh`, its derivative is only about `0.01`.

The first local Jacobian is therefore:

```text
3 * 0.009866 = 0.029598
```

All four local Jacobians are close to `0.03`, so their product is only:

```text
0.000000840045
```

This is **saturation**: large weights did not create a useful large gradient.
They pushed `tanh` into a flat region whose derivative overwhelms the weight.

## 6. A stable control

With positive ReLU activations and four weights equal to `1`, every local
Jacobian equals `1`:

```text
1 * 1 * 1 * 1 = 1
```

The input gradient stays equal to the output gradient. Real networks do not
stay this perfectly balanced, but the control makes the comparison concrete.

## 7. Check reverse mode independently

Use a central finite difference on the input:

```text
dL/dinput ~= (L(input + epsilon) - L(input - epsilon)) / (2 * epsilon)
epsilon = 0.000001
```

All four NN24 traces compare this numerical slope with the analytical reverse
pass. The absolute errors stay below `1e-8`.

## 8. Practical responses

Initialization matched to the activation helps at the first step. Residual
paths create shorter gradient routes. Normalization can keep internal scales
manageable, and gradient clipping can cap an exploding update. None is a magic
replacement for inspecting the actual signals.

The next lesson will place normalization, dropout, and a residual route beside
the same tiny network so their different jobs remain visible.

## 9. Validate the deterministic corpus

```text
python code/scripts/validate_gradient_flow_labs.py
```

The validator rejects unknown or duplicate keys, non-finite scenarios,
unsupported activations, changed conventions, trace drift, and failed input
finite differences.

## 10. Cross-language and Rust-core path

Implement the four scalar chains directly in each host language first. They
pin the order and meaning of every reverse-mode value without tensor-layout
questions.

A Rust core can later store activations and derivative masks in a contiguous
tape, traverse it backward, and fuse vector-Jacobian products with parameter-
gradient reductions. A C ABI or WASM layer can return the final gradient
buffers for speed and optionally expose the unfused tape for teaching and
debugging.
