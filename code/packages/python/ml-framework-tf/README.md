# ml-framework-tf

A TensorFlow-compatible API built on top of `ml-framework-core`. This package
provides the familiar TensorFlow/Keras programming model while delegating all
tensor operations and automatic differentiation to the shared core engine.

## What This Package Provides

### TensorFlow Core API

```python
import ml_framework_tf as tf

# Constants (immutable) and Variables (mutable)
x = tf.constant([1.0, 2.0, 3.0])
w = tf.Variable([0.5, 0.5, 0.5], name="weights")

# GradientTape for explicit gradient tracking
with tf.GradientTape() as tape:
    y = w * x
    loss = tf.reduce_sum(y)
grads = tape.gradient(loss, [w])
# grads[0].data == [1.0, 2.0, 3.0]
```

### tf.nn — Activation Functions

```python
tf.nn.relu(x)        # max(0, x)
tf.nn.sigmoid(x)     # 1 / (1 + exp(-x))
tf.nn.softmax(x)     # probability distribution
tf.nn.gelu(x)        # transformer activation
```

### tf.math — Mathematical Operations

```python
tf.math.log(x)
tf.math.exp(x)
tf.math.sqrt(x)
tf.math.abs(x)
```

### tf.keras — High-Level Training API

```python
# Build a model
model = tf.keras.Sequential([
    tf.keras.layers.Dense(128, activation='relu', input_dim=784),
    tf.keras.layers.Dropout(0.3),
    tf.keras.layers.Dense(10, activation='softmax'),
])

# Compile with optimizer, loss, and metrics
model.compile(
    optimizer='adam',
    loss='sparse_categorical_crossentropy',
    metrics=['accuracy'],
)

# Train
history = model.fit(x_train, y_train, epochs=10, batch_size=32,
                    validation_data=(x_val, y_val))

# Evaluate and predict
loss, accuracy = model.evaluate(x_test, y_test)
predictions = model.predict(x_new)
```

### tf.data — Data Pipelines

```python
dataset = tf.data.Dataset.from_tensor_slices((x_train, y_train))
dataset = dataset.shuffle(1000).batch(32)

for x_batch, y_batch in dataset:
    # training step
    pass
```

## How It Fits in the Stack

```
ml-framework-tf  (this package — TensorFlow API)
        |
        v
ml-framework-core  (shared tensor + autograd engine)
        |
        v
   blas-library  (linear algebra: sgemm, saxpy, etc.)
```

The core engine handles:
- Tensor storage and operations
- Computation graph construction
- Backward pass (reverse-mode autodiff)

This package adds the TensorFlow-specific API on top:
- `tf.GradientTape` for explicit gradient tracking
- `tf.Variable` for mutable, named tensors
- `tf.keras` for the high-level compile/fit/evaluate workflow

## Key Differences from PyTorch API

| Feature              | TensorFlow (this)           | PyTorch (ml-framework-torch) |
|----------------------|-----------------------------|------------------------------|
| Gradient tracking    | Explicit (GradientTape)     | Implicit (requires_grad)     |
| Training loop        | model.fit()                 | Manual loop                  |
| Loss argument order  | (y_true, y_pred)            | (prediction, target)         |
| Axis naming          | axis= parameter             | dim= parameter               |
| Layer naming         | Dense(units=64)             | Linear(out_features=64)      |

## Package Structure

```
src/ml_framework_tf/
├── __init__.py        # tf.constant, tf.Variable, tf.zeros, etc.
├── variable.py        # Variable class
├── gradient_tape.py   # GradientTape
├── nn.py              # tf.nn activations
├── math_ops.py        # tf.math operations
├── random.py          # tf.random.normal
├── data.py            # tf.data.Dataset
└── keras/
    ├── layers.py      # Dense, Dropout, BatchNorm, Embedding, etc.
    ├── models.py      # Sequential, Model
    ├── optimizers.py   # SGD, Adam, RMSprop, AdamW
    ├── losses.py      # MSE, BCE, CCE, SCCE, MAE
    ├── metrics.py     # Accuracy, BinaryAccuracy, MSE, MAE
    ├── callbacks.py   # EarlyStopping, ModelCheckpoint, History
    └── activations.py # relu, sigmoid, softmax, tanh, gelu
```

## Development

```bash
# Run tests
mise exec -- uv run pytest tests/ -v --tb=short --cov=ml_framework_tf --cov-report=term-missing

# Lint
mise exec -- uv run ruff check src/ tests/

# Format
mise exec -- uv run ruff format src/ tests/
```
