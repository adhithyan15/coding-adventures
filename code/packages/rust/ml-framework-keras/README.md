# ml-framework-keras

Keras 3-compatible high-level neural network API built on ml-framework-core.

## Philosophy

Keras was created with a clear mission: "Being able to go from idea to result
with the least possible delay is key to doing good research."

The design principles:
1. **User-friendly**: consistent API, clear error messages
2. **Modular**: layers, losses, optimizers are interchangeable pieces
3. **Extensible**: easy to create new components
4. **Multi-backend**: write once, run on any engine

## Usage

```rust
use ml_framework_keras::*;

let mut model = models::Sequential::new();
model.add(Box::new(layers::Dense::new(128, Some("relu"), true)));
model.add(Box::new(layers::Dropout::new(0.2)));
model.add(Box::new(layers::Dense::new(10, Some("softmax"), true)));

model.compile("adam", "categorical_crossentropy", &["accuracy"]);
let history = model.fit(&x_train, &y_train, 10, 32, None, 1);
```

## Module Structure

- `layers`: Building blocks (Dense, Dropout, BatchNorm, etc.)
- `models`: Sequential and Functional API containers
- `optimizers`: SGD, Adam, RMSprop, AdamW
- `losses`: MSE, CrossEntropy, etc.
- `metrics`: Accuracy, MSE, MAE (for monitoring)
- `callbacks`: EarlyStopping, ModelCheckpoint, etc.
- `activations`: relu, sigmoid, softmax, etc.
- `backend`: Backend selection (ml_framework_core)
