# Changelog

All notable changes to `ml-framework-tf` will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **tf.constant** and **tf.Variable** — immutable and mutable tensor creation
  with name tracking and trainability flags
- **tf.GradientTape** — explicit gradient tracking context manager with
  persistent and one-shot modes
- **tf.nn** — activation functions: relu, sigmoid, softmax, gelu
- **tf.math** — element-wise operations: log, exp, sqrt, abs
- **tf.random.normal** — random tensor generation from normal distribution
- **tf.data.Dataset** — data pipeline with from_tensor_slices, batch, shuffle
- **tf.keras.layers** — Dense, Flatten, Dropout, BatchNormalization,
  LayerNormalization, Embedding, ReLU, Softmax, Input
- **tf.keras.models** — Sequential and Model with compile/fit/evaluate/predict
- **tf.keras.optimizers** — SGD, Adam, RMSprop, AdamW
- **tf.keras.losses** — MeanSquaredError, MeanAbsoluteError,
  BinaryCrossentropy, CategoricalCrossentropy, SparseCategoricalCrossentropy
- **tf.keras.metrics** — Accuracy, BinaryAccuracy, CategoricalAccuracy,
  MeanSquaredError, MeanAbsoluteError
- **tf.keras.callbacks** — History, EarlyStopping, ModelCheckpoint,
  LearningRateScheduler
- **tf.keras.activations** — Function lookup by string name for layer
  activation parameters
- Top-level operations: matmul, add, multiply, reduce_sum, reduce_mean,
  reshape, transpose, clip_by_value, zeros, ones, eye, range_
- 241 tests with 99% code coverage
- Literate programming style with extensive inline documentation
