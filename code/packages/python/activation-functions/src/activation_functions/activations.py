import math

def sigmoid(x: float) -> float:
    """Clamps numbers beautifully between 0.0 and 1.0 for Absolute Probabilities."""
    if x < -709: return 0.0
    if x > 709: return 1.0
    return 1.0 / (1.0 + math.exp(-x))

def sigmoid_derivative(x: float) -> float:
    """The cached backpropagation derivative flawlessly evaluates to sig * (1 - sig)."""
    sig = sigmoid(x)
    return sig * (1.0 - sig)

def relu(x: float) -> float:
    """Destroys negative connections dynamically; maintains pure positive values."""
    return max(0.0, float(x))

def relu_derivative(x: float) -> float:
    return 1.0 if x > 0 else 0.0

def tanh_func(x: float) -> float:
    """Bounds structures cleanly between -1.0 and 1.0"""
    return math.tanh(x)

def tanh_derivative(x: float) -> float:
    """Structurally calculates to 1.0 - cached² flawlessly."""
    t = math.tanh(x)
    return 1.0 - (t * t)
