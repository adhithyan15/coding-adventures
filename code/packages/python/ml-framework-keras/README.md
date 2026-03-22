# ml-framework-keras

A Keras 3-compatible high-level neural network API built on top of `ml-framework-core`. Provides the famous "3-line training" experience: define a model, compile it, fit it.

## Where It Fits

This package sits at the top of the ML framework stack:

```
ml-framework-keras   ← YOU ARE HERE (highest-level API)
ml-framework-torch   ← PyTorch-style mid-level API
ml-framework-core    ← Shared tensor/autograd engine
blas-library         ← Linear algebra primitives
```

## Quick Start

```python
import ml_framework_keras as keras
from ml_framework_keras.layers import Dense, Dropout
from ml_framework_core import Tensor

# 1. Define the model
model = keras.Sequential([
    Dense(128, activation="relu"),
    Dropout(0.2),
    Dense(10, activation="softmax"),
])

# 2. Compile (connect optimizer, loss, metrics)
model.compile(
    optimizer="adam",
    loss="categorical_crossentropy",
    metrics=["accuracy"],
)

# 3. Train
history = model.fit(x_train, y_train, epochs=10, batch_size=32,
                    validation_data=(x_val, y_val))
```

## Modules

### Layers (`keras.layers`)
- **Dense** — Fully connected layer with optional activation
- **Dropout** — Regularization by randomly zeroing activations
- **BatchNormalization** — Normalize across the batch dimension
- **LayerNormalization** — Normalize across features (used in transformers)
- **Flatten** — Collapse spatial dimensions into a flat vector
- **Embedding** — Map integer indices to dense vectors
- **Input** — Symbolic placeholder for the Functional API
- **ReLU, Softmax** — Standalone activation layers

### Models (`keras.Sequential`, `keras.Model`)
- **Sequential** — Linear stack of layers
- **Model** — Functional API for complex architectures

Both share: `compile()`, `fit()`, `evaluate()`, `predict()`, `summary()`

### Optimizers (`keras.optimizers`)
- **SGD** — Stochastic Gradient Descent with optional momentum
- **Adam** — Adaptive Moment Estimation (default choice)
- **RMSprop** — Root Mean Square Propagation
- **AdamW** — Adam with decoupled weight decay

### Losses (`keras.losses`)
- **MeanSquaredError** / `"mse"` — Regression
- **MeanAbsoluteError** / `"mae"` — Robust regression
- **BinaryCrossentropy** — Binary classification
- **CategoricalCrossentropy** — Multi-class (one-hot labels)
- **SparseCategoricalCrossentropy** — Multi-class (integer labels)

### Metrics (`keras.metrics`)
- **Accuracy** — General accuracy metric
- **BinaryAccuracy** — With configurable threshold
- **CategoricalAccuracy** — Argmax comparison
- **MeanSquaredError** / **MeanAbsoluteError** — Regression metrics

### Callbacks (`keras.callbacks`)
- **History** — Records training metrics (returned by `fit()`)
- **EarlyStopping** — Stop when metric plateaus
- **ModelCheckpoint** — Save best model weights
- **LearningRateScheduler** — Dynamic learning rate adjustment

### Activations (`keras.activations`)
- `relu`, `sigmoid`, `tanh`, `softmax`, `gelu`, `linear`
- String-based lookup: `get_activation("relu")`

### Backend (`keras.backend`)
- `get_backend()` / `set_backend()` — Backend selection

## String-Based Configuration

All components support string-based lookup for ergonomic model definition:

```python
model.compile(
    optimizer="adam",        # → Adam()
    loss="mse",              # → MeanSquaredError()
    metrics=["accuracy"],    # → [Accuracy()]
)
```

## Running Tests

```bash
mise exec -- uv run pytest tests/ -v --tb=short --cov=ml_framework_keras --cov-report=term-missing
```

## Coverage

98% test coverage across 240 tests.
