# ml-learning-visualizer

Interactive machine learning lab for building intuition around small models.

## What It Shows

The app has three workbenches that move from one arithmetic update to small
hidden-layer networks.

### Training-step microscope

The default view pauses one neuron on one training example and reveals seven
phases:

```text
example -> multiply -> add bias -> activate -> loss -> backprop -> update
```

Future values stay hidden until their phase is selected. The learner can change
the input, target, weight, bias, activation, and learning rate, inspect the
chain-rule factors, and apply exactly one proposed update.

### Linear lab

The linear workbench includes 100 selectable examples that reuse one training
shell:

```text
y = weight * x + bias
```

You can tune the learning rate, initial weight, initial bias, loss function, and
activation preview, then step through gradient descent while the fitted line,
prediction points, loss, gradients, and error distances update.

### Hidden-layer lab

The hidden workbench traces XOR-like logic, absolute value, piecewise pricing,
circle classification, two moons, and feature interactions. It exposes selected
hidden activations, matrix shapes, gradients, parameter updates, and the shared
neural graph VM execution path.

## Lab Families

- Basics: clean linear relationships such as Celsius to Fahrenheit.
- Learning rate: examples tuned to show slow, useful, and unstable step sizes.
- Loss functions: MSE vs MAE with clean points and outliers.
- Scaling: normalized, centered, and wide input ranges.
- Noise: increasingly noisy observations around the same underlying trend.
- Generalization: sparse and curved data where a line has limits.
- Real data: a checked-in CC0 Palmer Penguins CSV sample.

## Dataset Policy

The browser app should not live-load Hugging Face, Kaggle, or other remote
datasets. Small teaching datasets can be checked in as local JSON or CSV only
when their license and source are clear. Dataset notes live in
`src/data/SOURCES.md`.

The smallest canonical examples also live in the language-neutral NN03 corpus
under `code/specs/fixtures/neural-learning-v1`. That corpus pins forward values
and first-step gradients so other language implementations can reproduce the
same visualized arithmetic.

## Development

```bash
bash BUILD
```
