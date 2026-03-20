from typing import List

def sgd(weights: List[float], gradients: List[float], learning_rate: float) -> List[float]:
    """
    Stochastic Gradient Descent (SGD)
    Updates weights by moving them in the opposite direction of the gradient.
    """
    if len(weights) != len(gradients) or len(weights) == 0:
        raise ValueError("Weights and gradients must have the same non-zero length")
    
    return [w - (learning_rate * g) for w, g in zip(weights, gradients)]
