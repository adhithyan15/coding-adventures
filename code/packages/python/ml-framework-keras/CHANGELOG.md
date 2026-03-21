# Changelog

All notable changes to `ml-framework-keras` will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- **Layers**: Dense, Dropout, BatchNormalization, LayerNormalization, Flatten, Embedding, Input, ReLU, Softmax with lazy weight initialization (Keras-style `build()` on first call)
- **Models**: Sequential API and Functional API (Model) with shared compile/fit/evaluate/predict/summary interface
- **Training loop**: `model.fit()` with mini-batch SGD, epoch-level metrics, validation data/split support, and verbose output
- **Optimizers**: SGD (with momentum), Adam, RMSprop, AdamW (decoupled weight decay) with Keras-style `apply_gradients()` API
- **Losses**: MeanSquaredError, MeanAbsoluteError, BinaryCrossentropy, CategoricalCrossentropy, SparseCategoricalCrossentropy with `from_logits` support
- **Metrics**: Accuracy, BinaryAccuracy, CategoricalAccuracy, MeanSquaredError, MeanAbsoluteError with stateful update/result/reset protocol
- **Callbacks**: History, EarlyStopping (with restore_best_weights), ModelCheckpoint, LearningRateScheduler
- **Activations**: relu, sigmoid, tanh, softmax, gelu, linear with string-based lookup via `get_activation()`
- **Backend**: `get_backend()` / `set_backend()` for multi-backend API compatibility
- **String registries**: All optimizers, losses, and metrics can be specified as strings (e.g., `"adam"`, `"mse"`, `"accuracy"`)
- **Literate programming**: All source files include extensive inline documentation explaining ML concepts for newcomers
- 240 tests with 98% code coverage
