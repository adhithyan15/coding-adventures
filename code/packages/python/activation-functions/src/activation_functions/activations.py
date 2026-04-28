import math

LEAKY_RELU_SLOPE = 0.01

def linear(x: float) -> float:
    """Return the raw weighted sum unchanged."""
    return float(x)

def linear_derivative(x: float) -> float:
    return 1.0

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

def leaky_relu(x: float) -> float:
    """Keep a small negative slope so inactive ReLU neurons can still learn."""
    return float(x) if x > 0 else LEAKY_RELU_SLOPE * float(x)

def leaky_relu_derivative(x: float) -> float:
    return 1.0 if x > 0 else LEAKY_RELU_SLOPE

def tanh_func(x: float) -> float:
    """Bounds structures cleanly between -1.0 and 1.0"""
    return math.tanh(x)

def tanh_derivative(x: float) -> float:
    """Structurally calculates to 1.0 - cached² flawlessly."""
    t = math.tanh(x)
    return 1.0 - (t * t)

def softplus(x: float) -> float:
    """Smooth ReLU using a numerically stable log1p formulation."""
    return math.log1p(math.exp(-abs(x))) + max(float(x), 0.0)

def softplus_derivative(x: float) -> float:
    return sigmoid(x)
