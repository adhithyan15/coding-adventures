# ML Learning Lab: Normalization and Learning-Rate Sweeps

This note explains the first program parity slice for the ML learning lab.

## What Changed

The house-price predictor now follows the same learning pattern in Python,
TypeScript, Go, Rust, Ruby, and Elixir:

1. Start with raw feature rows like `[square_feet, bedrooms]`.
2. Fit a standard scaler on the feature columns.
3. Transform each feature row into normalized coordinates.
4. Try several learning rates on a short 120-epoch run.
5. Pick the stable rate with the lowest short-run loss.
6. Train the final model and predict the first house price.

## Why Normalize First?

Without normalization, one feature can dominate the gradient only because its
numbers are larger. Square footage might be `2000`, while bedroom count might
be `3`. During a weight update, the square-footage gradient can become huge,
forcing the learning rate to be tiny.

After standard scaling, both columns speak in comparable units:

```text
scaled = (value - column_mean) / column_standard_deviation
```

Now the learning-rate knob is easier to reason about because each feature
contributes on a similar numeric scale.

## What the Sweep Teaches

The current house-price data is deliberately tiny and clean. The learning-rate
sweep prints a pattern like this:

```text
lr=0.001  -> loss=80456.4265
lr=0.003  -> loss=29575.3243
lr=0.01   -> loss=960.9546
lr=0.03   -> loss=0.0435
lr=0.1    -> loss=0.0000
lr=0.3    -> loss=0.0000
lr=0.6    -> loss=diverged
```

That output is the lesson:

- `0.001` is stable but slow.
- `0.03` is much faster.
- `0.1` and `0.3` are excellent on this toy dataset.
- `0.6` overshoots so hard that the loss explodes.

This is a practical way to teach learning rate without turning it into a purely
abstract calculus topic.

## Mental Model

For a model with many inputs and one output:

```text
prediction = x1*w1 + x2*w2 + ... + xn*wn + bias
error = prediction - actual
gradient_for_wj = average(error * xj) * loss_derivative_scale
new_wj = old_wj - learning_rate * gradient_for_wj
```

Each weight gets its own gradient. Normalization makes those per-weight
gradients more comparable, which makes the single learning-rate value less
fragile.

## Where This Goes Next

The next useful examples are:

- Compare standard scaling vs min-max scaling on the same house-price loop.
- Add noisy data so the model converges near, not exactly on, the target.
- Add a train/test split to show overfitting.
- Add a small binary classifier with sigmoid activation.
- Add a multi-output example only after the single-output examples feel boring.
