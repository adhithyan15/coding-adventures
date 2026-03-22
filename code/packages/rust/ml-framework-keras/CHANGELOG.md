# Changelog

## 0.1.0 — 2026-03-21

### Added
- `backend` module with get/set backend support
- `activations` module: relu, sigmoid, tanh, softmax, gelu, linear with string lookup
- `layers` module: Layer trait, Dense, Dropout, BatchNormalization, LayerNormalization, Flatten, Embedding, ReLU, Softmax, Input
- `models` module: Sequential and Model with compile/fit/evaluate/predict
- `optimizers` module: SGD, Adam, RMSprop, AdamW with string lookup
- `losses` module: MeanSquaredError, MeanAbsoluteError, BinaryCrossentropy, CategoricalCrossentropy, SparseCategoricalCrossentropy
- `metrics` module: Accuracy, BinaryAccuracy, CategoricalAccuracy, MeanSquaredError, MeanAbsoluteError
- `callbacks` module: History, EarlyStopping, ModelCheckpoint, LearningRateScheduler
- Comprehensive test suite
