# ML Learning Lab Program Parity

The visual lab should be the intuition surface. The repo programs should be the
executable proof that the same idea works in any language a learner already
knows.

## Goal

Every visual machine-learning example should eventually have three matching
artifacts:

1. A visual lab that lets the learner change parameters and watch the model
   move.
2. A portable program spec that defines the dataset, starting parameters,
   primitives, and expected training trace.
3. Language programs with literate notes that run the same experiment through
   the repo's core primitives.

That gives us two useful feedback loops:

- Learners can move from pictures to code without changing the problem.
- CI can stress test activation functions, loss functions, gradient descent,
  matrix math, and future primitives against real teaching examples.

## Translation Shape

Each visual example should translate to a small, deterministic program:

```text
dataset -> model -> activation -> loss -> gradients -> optimizer -> trace
```

The program should print a compact training trace:

```text
epoch, weight(s), bias, prediction sample, loss, gradient summary
```

For tests, each program should also expose or generate a machine-readable golden
trace. A small trace is enough:

```json
[
  {
    "step": 0,
    "weights": [0.5],
    "bias": 0.5,
    "loss": 3628.67
  }
]
```

## Literate Notes

Each language version should include comments or a README that explains the same
concepts in that language's idiom:

- What the inputs represent.
- What the model computes before activation.
- Why the chosen activation function is used.
- How the loss function turns prediction error into one scalar.
- How the derivative points back toward better weights.
- What the learning rate changes in the update step.

The notes should stay close to executable code. A learner should be able to read
from top to bottom and see the same sequence that the visual app animates.

## First Program Families

The repo already has program precedents for these ML examples:

| Example | Existing language coverage | Core primitives stressed |
|---------|----------------------------|--------------------------|
| Celsius to Fahrenheit predictor | Go, Python, Rust, Ruby, TypeScript, Elixir | MSE, MAE, gradient descent, linear activation |
| House price predictor | Go, Python, Rust, Ruby, TypeScript, Elixir | Multi-feature linear regression, MSE |
| Space launch predictor | Go, Python, Rust, Ruby, TypeScript, Elixir | Regression/classification-shaped feature engineering |
| Mansion classifier | Go, Python, Rust, Ruby, TypeScript, Elixir | Classification thresholding, sigmoid-style intuition |
| XOR classifier | Python | Non-linear activation necessity |

These should be the first parity targets because they already exist as programs
and can be tightened into shared, testable teaching examples.

## Program Spec Template

Each lab should get a small JSON or Markdown spec with:

- `id`: stable identifier shared with the visual lab.
- `title`: learner-facing example name.
- `dataset`: inline small dataset or relative path to checked-in CSV/JSON.
- `model`: linear regression, logistic regression, tiny neural network, etc.
- `activations`: required activation functions.
- `losses`: required loss functions.
- `optimizer`: optimizer primitive and learning rate.
- `initial_parameters`: deterministic starting weights and bias.
- `expected_trace`: selected checkpoints for CI.
- `notes`: short explanation of the concept being taught.

## CI Strategy

The build should not require every program in every language to exist on day
one. Instead:

1. Visual labs publish specs.
2. Program implementations declare which specs they satisfy.
3. CI runs only implementations whose language runtime is available.
4. Missing implementations are reported as coverage gaps, not build failures.
5. Existing implementations must match their golden traces within tolerance.

This lets the lab grow toward dozens of examples without blocking every PR on a
complete cross-language matrix.

## Immediate Next Slice

The activation-function package parity work is the right foundation for this.
After the current activation PR, the next implementation slice should be:

1. Extract the Celsius visualizer training loop into a portable lab spec.
2. Update the existing Celsius programs to include literate notes and a golden
   training trace.
3. Add a small cross-language report showing which languages satisfy the Celsius
   lab.
4. Repeat for house price prediction, then classification examples.
