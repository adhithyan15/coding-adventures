# Changelog

## 0.1.0 — 2026-03-21

### Added
- `Variable` struct with mutable tensor, trainable flag, and in-place mutation (assign, assign_add, assign_sub)
- `GradientTape` for explicit gradient tracking with persistent mode
- `constant()`, `zeros()`, `ones()`, `eye()`, `range_()` tensor factory functions
- `reduce_sum()`, `reduce_mean()`, `matmul()`, `add()`, `multiply()`, `reshape()`, `transpose()`, `clip_by_value()`
- `nn` module with relu, sigmoid, softmax, gelu activation functions
- `math_ops` module with log, exp, sqrt, abs operations
- `random` module with normal distribution sampling
- `keras` submodule: layers (Dense, Flatten, Dropout, BatchNormalization, LayerNormalization, Embedding, ReLU, Softmax, Input)
- `keras` models: Sequential and Model with compile/fit/evaluate/predict
- `keras` optimizers: SGD, Adam, RMSprop, AdamW
- `keras` losses: MSE, MAE, BinaryCrossentropy, CategoricalCrossentropy, SparseCategoricalCrossentropy
- `keras` metrics: Accuracy, BinaryAccuracy, CategoricalAccuracy, MeanSquaredError, MeanAbsoluteError
- `keras` callbacks: History, EarlyStopping, ModelCheckpoint, LearningRateScheduler
- `keras` activations: relu, sigmoid, tanh, softmax, gelu, linear with string lookup
- `Dataset` with from_tensor_slices, batch, shuffle
- Comprehensive test suite
