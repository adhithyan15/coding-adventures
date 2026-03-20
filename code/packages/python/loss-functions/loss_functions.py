import math

# EPSILON is used to clamp probabilities before taking the logarithm.
# Since log(0) evaluates to negative infinity, we clamp predictions to [EPSILON, 1 - EPSILON].
EPSILON = 1e-7

def mse(y_true: list[float], y_pred: list[float]) -> float:
    """
    Calculates Mean Squared Error (MSE).

    MSE measures the average of the squares of the errors-that is, the average squared
    difference between the estimated values and the actual value. It is widely used for
    standard regression tasks. By squaring the errors, MSE heavily penalizes larger errors.

    Equation:
        MSE = (1/n) * Σ(y_true_i - y_pred_i)^2

    Args:
        y_true: A list of the ground truth actual values.
        y_pred: A list of the predicted values from the model.

    Returns:
        The mean squared error as a float.

    Example:
        >>> mse([1.0, 0.0], [0.9, 0.1])
        0.010000000000000002
    """
    if len(y_true) != len(y_pred) or len(y_true) == 0:
        raise ValueError("Lists must have the same non-zero length")
    
    total_error = 0.0
    for true_val, pred_val in zip(y_true, y_pred):
        diff = true_val - pred_val
        total_error += diff * diff
        
    return total_error / len(y_true)

def mae(y_true: list[float], y_pred: list[float]) -> float:
    """
    Calculates Mean Absolute Error (MAE).

    MAE measures the average magnitude of the errors in a set of predictions, without 
    considering their direction. It is the average over the test sample of the absolute 
    differences between prediction and actual observation where all individual differences 
    have equal weight. It's often used for robust regression to ignore extreme outliers.

    Equation:
        MAE = (1/n) * Σ|y_true_i - y_pred_i|

    Args:
        y_true: A list of the ground truth actual values.
        y_pred: A list of the predicted values from the model.

    Returns:
        The mean absolute error as a float.

    Example:
        >>> mae([1.0, 0.0], [0.9, 0.1])
        0.1
    """
    if len(y_true) != len(y_pred) or len(y_true) == 0:
        raise ValueError("Lists must have the same non-zero length")
        
    total_error = 0.0
    for true_val, pred_val in zip(y_true, y_pred):
        total_error += abs(true_val - pred_val)
        
    return total_error / len(y_true)

def bce(y_true: list[float], y_pred: list[float]) -> float:
    """
    Calculates Binary Cross-Entropy (BCE) loss.

    BCE is used for binary classification tasks (e.g., Cat vs. Dog). It quantifies the 
    difference between two probability distributions. Predictions must be between 0 and 1.
    We apply a small epsilon clamp to prevent taking the log of 0, which would result
    in negative infinity and disrupt gradient calculations during backpropagation.

    Equation:
        BCE = -(1/n) * Σ[y_true_i * log(y_pred_i) + (1 - y_true_i) * log(1 - y_pred_i)]

    Args:
        y_true: A list of the ground truth actual values (usually 0.0 or 1.0).
        y_pred: A list of the predicted probabilities from the model (between 0.0 and 1.0).

    Returns:
        The binary cross-entropy loss as a float.

    Example:
        >>> bce([1.0, 0.0], [0.9, 0.1])
        0.10536051545782785
    """
    if len(y_true) != len(y_pred) or len(y_true) == 0:
        raise ValueError("Lists must have the same non-zero length")
        
    total_error = 0.0
    for true_val, pred_val in zip(y_true, y_pred):
        # Clamp prediction to avoid log(0)
        p = max(EPSILON, min(1 - EPSILON, pred_val))
        total_error += true_val * math.log(p) + (1 - true_val) * math.log(1 - p)
        
    return -total_error / len(y_true)

def cce(y_true: list[float], y_pred: list[float]) -> float:
    """
    Calculates Categorical Cross-Entropy (CCE) loss.

    CCE is used for multi-class classification tasks (e.g., classifying a digit 0-9).
    It expects `y_true` to be a one-hot encoded vector representing the true class.
    Only the probability assigned to the true class affects the loss.
    
    Like BCE, we clamp predictions using epsilon to avoid negative infinity from log(0).

    Equation:
        CCE = -(1/n) * Σ[y_true_i * log(y_pred_i)]

    Args:
        y_true: A list of one-hot encoded ground truth values.
        y_pred: A list of predicted probabilities for each class.

    Returns:
        The categorical cross-entropy loss as a float.

    Example:
        >>> cce([1.0, 0.0], [0.9, 0.1])
        0.05268025772891392
    """
    if len(y_true) != len(y_pred) or len(y_true) == 0:
        raise ValueError("Lists must have the same non-zero length")
        
    total_error = 0.0
    for true_val, pred_val in zip(y_true, y_pred):
        # Clamp prediction to avoid log(0)
        p = max(EPSILON, min(1 - EPSILON, pred_val))
        total_error += true_val * math.log(p)
        
    return -total_error / len(y_true)
