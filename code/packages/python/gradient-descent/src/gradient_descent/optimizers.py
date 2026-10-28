def sgd(weights: list[float], gradients: list[float], learning_rate: float) -> list[float]:
    """Return one pure stochastic-gradient-descent update.

    Every output coordinate follows ``weight - learning_rate * gradient``. The
    comprehension constructs a new list, preserving both caller-owned inputs.

    Raises:
        ValueError: If the vectors are empty or have different lengths.
    """
    if len(weights) != len(gradients) or len(weights) == 0:
        raise ValueError("Weights and gradients must have the same non-zero length")

    return [w - (learning_rate * g) for w, g in zip(weights, gradients)]
