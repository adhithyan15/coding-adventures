# ML04 — Activation Functions

## Overview

The Activation Functions specification defines the fundamental non-linear transformations applied to neuron outputs in neural networks. Without activation functions, any composition of linear layers collapses to a single linear transformation — the network could only learn straight lines. Activation functions introduce the non-linearity that allows networks to approximate arbitrarily complex functions.

This is a fundamental mathematical layer. It has absolutely no dependencies other than language-native math libraries (specifically `exp` and `tanh`). **These are purely standalone, composable scalar functions — each takes a single float and returns a single float.**

## Layer Position

```
Inputs → Linear Layer → [YOU ARE HERE] → Loss Functions (ML01)
                              ↑
                    Gradient Descent (ML02) uses the derivatives
```

**Input from:** Raw scalar values (the pre-activation output of a linear layer).
**Output to:** Transformed scalar values fed to the next layer or to a loss function.

## Why These Functions?

Neural network literature has produced dozens of activation functions. This spec keeps the first teaching layer small while covering the functions learners will see immediately in real models:

| Function | Range | Use Case | Why It Matters |
|----------|-------|----------|----------------|
| **Linear / Identity** | $(-\infty, \infty)$ | Regression outputs, baseline comparisons | Leaves the weighted sum unchanged |
| **Sigmoid** | $(0, 1)$ | Output layer for binary classification | Maps any real number to a probability |
| **ReLU** | $[0, \infty)$ | Hidden layers (default choice) | Computationally cheap, avoids vanishing gradients |
| **Leaky ReLU** | $(-\infty, \infty)$ | Hidden layers when ReLU neurons go inactive | Keeps a small gradient for negative inputs |
| **Tanh** | $(-1, 1)$ | Hidden layers, zero-centered data | Like sigmoid but centered at zero |
| **Softplus** | $(0, \infty)$ | Smooth ReLU-like transformations | Differentiable everywhere and has sigmoid as its derivative |

## The Activation Functions

### Linear / Identity

$$\text{Linear}(x) = x$$

Linear activation is the identity function. It is useful for regression output layers where the model should predict any real-valued number, and it is also the clearest baseline for visualizers because it shows the raw weighted sum without any non-linear bend.

### Sigmoid

$$\sigma(x) = \frac{1}{1 + e^{-x}}$$

The sigmoid function squashes any real number into the range $(0, 1)$. It was the original activation function used in neural networks, inspired by the firing rate of biological neurons. For very negative $x$, the output approaches 0; for very positive $x$, it approaches 1; at $x = 0$, it returns exactly 0.5.

**Overflow protection:** For $x < -709$, $e^{-x}$ overflows `float64`. Implementations must clamp: return `0.0` for $x < -709$ and `1.0` for $x > 709$.

### ReLU (Rectified Linear Unit)

$$\text{ReLU}(x) = \max(0, x)$$

ReLU is the simplest and most widely used activation function in modern deep learning. It passes positive values through unchanged and zeros out negative values. Despite its simplicity, it solves the vanishing gradient problem that plagued sigmoid/tanh in deep networks — the gradient is either 0 or 1, never a tiny fraction.

### Leaky ReLU

$$\text{LeakyReLU}(x) = \begin{cases} x & \text{if } x > 0 \\ 0.01x & \text{if } x \leq 0 \end{cases}$$

Leaky ReLU keeps the positive side of ReLU but gives negative inputs a small slope. This reduces the "dying ReLU" problem, where a neuron receiving only negative pre-activations has a zero gradient and stops learning. The canonical default slope in this spec is `0.01`.

### Tanh (Hyperbolic Tangent)

$$\tanh(x) = \frac{e^x - e^{-x}}{e^x + e^{-x}}$$

Tanh is essentially a rescaled sigmoid: $\tanh(x) = 2\sigma(2x) - 1$. Its output is centered at zero (unlike sigmoid's centering at 0.5), which often leads to faster convergence during training because the gradients are better balanced.

### Softplus

$$\text{Softplus}(x) = \log(1 + e^x)$$

Softplus is a smooth approximation of ReLU. For large positive inputs it behaves almost like $x$; for large negative inputs it approaches 0 without a hard kink. Implementations should use the numerically stable equivalent:

$$\text{Softplus}(x) = \log(1 + e^{-|x|}) + \max(x, 0)$$

## Mathematical Derivatives

For backpropagation, implementations must provide the derivative of each activation function with respect to its input $x$:

| Function | Derivative Formula | Notes |
|----------|-------------------|-------|
| Linear' | $1$ | Constant slope everywhere |
| Sigmoid' | $\sigma(x) \cdot (1 - \sigma(x))$ | Maximum value of 0.25 at $x = 0$ |
| ReLU' | $\begin{cases} 1 & \text{if } x > 0 \\ 0 & \text{if } x \leq 0 \end{cases}$ | Technically undefined at $x = 0$; convention is 0 |
| Leaky ReLU' | $\begin{cases} 1 & \text{if } x > 0 \\ 0.01 & \text{if } x \leq 0 \end{cases}$ | Keeps a small gradient on the negative side |
| Tanh' | $1 - \tanh^2(x)$ | Maximum value of 1.0 at $x = 0$ |
| Softplus' | $\sigma(x)$ | The slope is exactly sigmoid |

## Public API

```text
// Package: activation-functions
// All functions take a single float and return a single float.
// These are pure, stateless scalar functions.

func Sigmoid(x: Float) -> Float
func SigmoidDerivative(x: Float) -> Float

func Linear(x: Float) -> Float
func LinearDerivative(x: Float) -> Float

func Relu(x: Float) -> Float
func ReluDerivative(x: Float) -> Float

func LeakyRelu(x: Float) -> Float
func LeakyReluDerivative(x: Float) -> Float

func Tanh(x: Float) -> Float
func TanhDerivative(x: Float) -> Float

func Softplus(x: Float) -> Float
func SoftplusDerivative(x: Float) -> Float
```

## Data Flow & Constraints

1. All functions are **pure** — no side effects, no mutation, no state.
2. All functions operate on **scalars** (single float values), not arrays.
3. Sigmoid must handle overflow gracefully (clamp at $\pm 709$).
4. ReLU derivative at exactly $x = 0$ returns `0.0` by convention.
5. Leaky ReLU uses a default negative slope of `0.01`; its derivative at exactly $x = 0$ returns `0.01` by convention.
6. Softplus must use the stable formula `log1p(exp(-abs(x))) + max(x, 0)` where the language provides `log1p`.
7. Implementations should use the language's native `exp` and `tanh` where available, unless the language philosophy is to build from first principles (e.g., using a custom `trig` package).

## Test Strategy

Activation functions are tested for exact mathematical parity using hardcoded scalar inputs across all language implementations.

### Parity Test Vectors

**Linear**
| Input | Expected Output |
|-------|----------------|
| `-3.0` | `-3.0` |
| `0.0` | `0.0` |
| `5.0` | `5.0` |

**Linear Derivative**
| Input | Expected Output |
|-------|----------------|
| `-3.0` | `1.0` |
| `0.0` | `1.0` |
| `5.0` | `1.0` |

**Sigmoid**
| Input | Expected Output | Notes |
|-------|----------------|-------|
| `0.0` | `0.5` | Midpoint |
| `1.0` | `0.7310585786300049` | Standard positive |
| `-1.0` | `0.2689414213699951` | Symmetry: $\sigma(-x) = 1 - \sigma(x)$ |
| `10.0` | `0.9999546021312976` | Near saturation |
| `-710.0` | `0.0` | Overflow clamp |
| `710.0` | `1.0` | Overflow clamp |

**Sigmoid Derivative**
| Input | Expected Output | Notes |
|-------|----------------|-------|
| `0.0` | `0.25` | Maximum derivative |
| `1.0` | `0.19661193324148185` | |
| `10.0` | `~0.0000453978` | Near-zero (saturated region) |

**ReLU**
| Input | Expected Output |
|-------|----------------|
| `5.0` | `5.0` |
| `-3.0` | `0.0` |
| `0.0` | `0.0` |

**ReLU Derivative**
| Input | Expected Output |
|-------|----------------|
| `5.0` | `1.0` |
| `-3.0` | `0.0` |
| `0.0` | `0.0` |

**Leaky ReLU**
| Input | Expected Output |
|-------|----------------|
| `5.0` | `5.0` |
| `-3.0` | `-0.03` |
| `0.0` | `0.0` |

**Leaky ReLU Derivative**
| Input | Expected Output |
|-------|----------------|
| `5.0` | `1.0` |
| `-3.0` | `0.01` |
| `0.0` | `0.01` |

**Tanh**
| Input | Expected Output | Notes |
|-------|----------------|-------|
| `0.0` | `0.0` | Midpoint |
| `1.0` | `0.7615941559557649` | |
| `-1.0` | `-0.7615941559557649` | Odd function: $\tanh(-x) = -\tanh(x)$ |

**Tanh Derivative**
| Input | Expected Output | Notes |
|-------|----------------|-------|
| `0.0` | `1.0` | Maximum derivative |
| `1.0` | `0.4199743416140261` | |

**Softplus**
| Input | Expected Output | Notes |
|-------|----------------|-------|
| `0.0` | `0.6931471805599453` | $\log(2)$ |
| `1.0` | `1.3132616875182228` | |
| `-1.0` | `0.31326168751822286` | |

**Softplus Derivative**
| Input | Expected Output | Notes |
|-------|----------------|-------|
| `0.0` | `0.5` | Matches sigmoid |
| `1.0` | `0.7310585786300049` | |
| `-1.0` | `0.2689414213699951` | |

### Property-Based Tests

In addition to parity vectors, implementations should verify these mathematical properties:

1. **Sigmoid range:** For any $x$, $0 < \sigma(x) < 1$
2. **Sigmoid symmetry:** $\sigma(-x) = 1 - \sigma(x)$
3. **ReLU idempotence:** $\text{ReLU}(\text{ReLU}(x)) = \text{ReLU}(x)$
4. **Tanh odd symmetry:** $\tanh(-x) = -\tanh(x)$
5. **Tanh range:** For any $x$, $-1 < \tanh(x) < 1$
6. **Softplus derivative identity:** $\text{Softplus}'(x) = \sigma(x)$
7. **All derivatives non-negative:** Linear', Sigmoid', ReLU', Leaky ReLU', Tanh', and Softplus' are all $\geq 0$
