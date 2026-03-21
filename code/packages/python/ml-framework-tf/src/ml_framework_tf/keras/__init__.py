"""
================================================================
TF.KERAS — HIGH-LEVEL NEURAL NETWORK API
================================================================

Keras is TensorFlow's high-level API for building and training
neural networks. It provides a clean, intuitive interface that
abstracts away the complexity of gradient computation, batching,
and optimization.

The Keras API is organized into:
- **layers**: Building blocks (Dense, Dropout, BatchNorm, etc.)
- **models**: Containers (Sequential, Model)
- **optimizers**: Training algorithms (Adam, SGD, etc.)
- **losses**: Error functions (MSE, cross-entropy, etc.)
- **metrics**: Performance measures (accuracy, MAE, etc.)
- **callbacks**: Training hooks (EarlyStopping, etc.)
- **activations**: Activation functions (relu, softmax, etc.)

Example:
    import ml_framework_tf as tf

    model = tf.keras.Sequential([
        tf.keras.layers.Dense(128, activation='relu', input_dim=784),
        tf.keras.layers.Dense(10, activation='softmax'),
    ])
    model.compile(optimizer='adam', loss='sparse_categorical_crossentropy')
    model.fit(x_train, y_train, epochs=5)

================================================================
"""

from . import activations, callbacks, layers, losses, metrics, models, optimizers
from .models import Model, Sequential

__all__ = [
    "activations",
    "callbacks",
    "layers",
    "losses",
    "metrics",
    "models",
    "optimizers",
    "Model",
    "Sequential",
]
