# CodingAdventures::ActivationFunctions (Perl)

Pure Perl implementation of neural network activation functions and their
derivatives. No external dependencies beyond `POSIX` (part of Perl's core).

## What Are Activation Functions?

Activation functions introduce non-linearity into neural networks. Without
them, no matter how many layers you stack, the network collapses to a single
linear transformation. Non-linearity allows networks to learn complex patterns.

## Functions Provided

| Function                   | Output Range  | Description                                    |
|----------------------------|--------------|------------------------------------------------|
| `sigmoid($x)`              | (0, 1)       | Classic S-curve, binary classification output  |
| `sigmoid_derivative($x)`   | (0, 0.25]    | σ(x) · (1 − σ(x))                             |
| `relu($x)`                 | [0, ∞)       | Rectified Linear Unit, default hidden layers   |
| `relu_derivative($x)`      | {0, 1}       | 1 if x > 0 else 0                             |
| `tanh_activation($x)`      | (−1, 1)      | Zero-centred, often faster convergence         |
| `tanh_derivative($x)`      | (0, 1]       | 1 − tanh(x)²                                  |
| `leaky_relu($x, $alpha)`   | (−∞, ∞)      | Prevents dying ReLU; default α = 0.01         |
| `leaky_relu_derivative($x)`| {α, 1}       | 1 if x > 0 else α                             |
| `elu($x, $alpha)`          | (−α, ∞)      | Smooth, saturates negatively                   |
| `elu_derivative($x, $alpha)`| (0, 1]      | 1 if x≥0 else α·e^x                          |
| `softmax(@logits)`         | (0,1)^n      | Probability distribution over n classes        |
| `softmax_derivative(@logits)`| (0, 0.25]^n | Diagonal Jacobian entries s_i · (1 − s_i)   |

## Usage

```perl
use CodingAdventures::ActivationFunctions qw(
    sigmoid relu tanh_activation softmax
);

print sigmoid(0);             # 0.5
print relu(-3);               # 0
print tanh_activation(1);     # 0.761594...
my @probs = softmax(1, 2, 3); # (0.090, 0.245, 0.665)
```

## Numerical Stability

- **sigmoid**: clamped at x < −709 → 0.0 and x > 709 → 1.0
- **softmax**: max-subtraction trick prevents exp() overflow

## Naming

`tanh_activation` is used instead of `tanh` to avoid a name conflict with
the POSIX-exported `tanh` symbol.

## Installation

```bash
cpanm --installdeps .
perl Makefile.PL && make && make install
```

## Running Tests

```bash
prove -l -v t/
```

## Dependencies

None (runtime). Uses only Perl built-ins and `POSIX::tanh`.
