"""
================================================================
BACKEND — MULTI-BACKEND SELECTION FOR KERAS
================================================================

Keras 3's defining feature is backend agnosticism. You can write your
model once and run it on any backend: PyTorch, TensorFlow, JAX, etc.

In our implementation, we only have one backend — our custom
ml-framework-core — but we expose the same API so code written
for real Keras can work here with minimal changes.

=== How It Works ===

    import ml_framework_keras as keras

    keras.backend.set_backend("ml_framework_core")  # our only backend
    print(keras.backend.get_backend())               # "ml_framework_core"

In real Keras 3, this switches the entire tensor/autograd engine.
For us, it's a thin wrapper that validates the backend name but
always uses ml-framework-core under the hood.

================================================================
"""

from __future__ import annotations

# =========================================================================
# Global backend state
# =========================================================================

# The only backend we support. In real Keras 3, this could be
# "torch", "tensorflow", "jax", etc.
_BACKEND = "ml_framework_core"

# Valid backends we recognize (even though we only implement one)
_VALID_BACKENDS = {"ml_framework_core", "torch", "tensorflow", "jax"}


def get_backend() -> str:
    """Return the name of the currently active backend.

    Returns:
        A string identifying the backend, e.g. "ml_framework_core".

    Example:
        >>> from ml_framework_keras.backend import get_backend
        >>> get_backend()
        'ml_framework_core'
    """
    return _BACKEND


def set_backend(name: str) -> None:
    """Set the Keras backend.

    In our implementation, only "ml_framework_core" is functional.
    Setting any other backend will raise a ValueError — but we
    validate the name so users get a helpful error message if they
    try to use a real Keras backend name.

    Args:
        name: Backend identifier string.

    Raises:
        ValueError: If the backend name is not recognized, or if
            it's a real Keras backend we don't support.

    Example:
        >>> from ml_framework_keras.backend import set_backend
        >>> set_backend("ml_framework_core")  # works
        >>> set_backend("torch")              # raises ValueError
    """
    global _BACKEND

    if name not in _VALID_BACKENDS:
        raise ValueError(
            f"Unknown backend '{name}'. Valid backends: {sorted(_VALID_BACKENDS)}"
        )

    if name != "ml_framework_core":
        raise ValueError(
            f"Backend '{name}' is recognized but not supported in this "
            f"implementation. Only 'ml_framework_core' is available."
        )

    _BACKEND = name
