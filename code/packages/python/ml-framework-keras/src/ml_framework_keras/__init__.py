"""
================================================================
ML FRAMEWORK KERAS — HIGH-LEVEL NEURAL NETWORK API
================================================================

This is a Keras 3-compatible high-level API built on top of
ml-framework-core. It provides the famous "3-line training"
experience:

    model = keras.Sequential([
        keras.layers.Dense(128, activation="relu"),
        keras.layers.Dropout(0.2),
        keras.layers.Dense(10, activation="softmax"),
    ])
    model.compile(optimizer="adam", loss="categorical_crossentropy",
                  metrics=["accuracy"])
    history = model.fit(x_train, y_train, epochs=10, batch_size=32)

=== Philosophy ===

Keras was created by Francois Chollet with a clear mission:
"Being able to go from idea to result with the least possible
delay is key to doing good research."

The design principles:
1. User-friendly: consistent API, clear error messages
2. Modular: layers, losses, optimizers are interchangeable pieces
3. Extensible: easy to create new components
4. Multi-backend: write once, run on any engine

=== Module Structure ===

- layers:      Building blocks (Dense, Dropout, BatchNorm, etc.)
- models:      Sequential and Functional API containers
- optimizers:  SGD, Adam, RMSprop, AdamW
- losses:      MSE, CrossEntropy, etc.
- metrics:     Accuracy, MSE, MAE (for monitoring)
- callbacks:   EarlyStopping, ModelCheckpoint, etc.
- activations: relu, sigmoid, softmax, etc.
- backend:     Backend selection (ml_framework_core)

================================================================
"""

from . import activations, backend, callbacks, layers, losses, metrics, optimizers
from .models import Model, Sequential

__all__ = [
    # Models
    "Sequential",
    "Model",
    # Submodules (accessed as keras.layers, keras.optimizers, etc.)
    "layers",
    "optimizers",
    "losses",
    "metrics",
    "callbacks",
    "activations",
    "backend",
]
