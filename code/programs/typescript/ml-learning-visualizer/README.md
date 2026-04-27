# ml-learning-visualizer

Interactive machine learning lab for building intuition around small models.

## What It Shows

This renderer app started as a tiny linear regression visualizer and is growing
into a broader ML learning lab. It now includes 100 selectable examples that
reuse one training shell:

```text
y = weight * x + bias
```

You can tune the learning rate, initial weight, initial bias, loss function, and
activation preview, then step through gradient descent while the fitted line,
prediction points, loss, gradients, and error distances update.

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

## Development

```bash
bash BUILD
```
