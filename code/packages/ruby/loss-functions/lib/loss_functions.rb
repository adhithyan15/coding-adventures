module LossFunctions
  # EPSILON is used to clamp probabilities within cross-entropy logarithms.
  # By bounding probabilities to [EPSILON, 1.0 - EPSILON], we mathematically prevent
  # the evaluation of Math.log(0), which would result in NaN or -Infinity and 
  # catastrophically break gradient descent during backpropagation.
  EPSILON = 1e-7

  class LengthMismatchError < StandardError; end

  module_function

  # Calculates Mean Squared Error (MSE).
  #
  # Mean Squared Error is widely used for Regression problems. By squaring the differences,
  # it naturally heavily penalizes predictions that are far away from the true label.
  #
  # @param y_true [Array<Float>] Ground truth original labels
  # @param y_pred [Array<Float>] Model predictions
  # @return [Float] The mean squared error
  #
  # @example
  #   LossFunctions.mse([1.0, 0.0], [0.9, 0.1])
  #   #=> 0.010000000000000002
  def mse(y_true, y_pred)
    validate_lengths!(y_true, y_pred)
    sum = 0.0
    y_true.zip(y_pred).each do |t, p|
      diff = t - p
      sum += diff * diff
    end
    sum / y_true.length
  end

  # Calculates Mean Absolute Error (MAE).
  #
  # MAE measures the absolute magnitude of the errors without considering direction.
  # It is widely used in Robust Regression to ignore extreme outliers.
  #
  # @param y_true [Array<Float>] Ground truth original labels
  # @param y_pred [Array<Float>] Model predictions
  # @return [Float] The mean absolute error
  #
  # @example
  #   LossFunctions.mae([1.0, 0.0], [0.9, 0.1])
  #   #=> 0.1
  def mae(y_true, y_pred)
    validate_lengths!(y_true, y_pred)
    sum = 0.0
    y_true.zip(y_pred).each do |t, p|
      sum += (t - p).abs
    end
    sum / y_true.length
  end

  # Calculates Binary Cross-Entropy (BCE) loss.
  #
  # BCE is used for binary classification tasks. It quantifies the difference
  # between two probability distributions. Predictions must be bound between 0 and 1.
  #
  # @param y_true [Array<Float>] Ground truth labels (0.0 or 1.0)
  # @param y_pred [Array<Float>] Model probabilities
  # @return [Float] The binary cross-entropy loss
  #
  # @example
  #   LossFunctions.bce([1.0, 0.0], [0.9, 0.1])
  #   #=> 0.1053605...
  def bce(y_true, y_pred)
    validate_lengths!(y_true, y_pred)
    sum = 0.0
    y_true.zip(y_pred).each do |t, p|
      p = p.clamp(EPSILON, 1.0 - EPSILON)
      sum += t * Math.log(p) + (1.0 - t) * Math.log(1.0 - p)
    end
    -sum / y_true.length
  end

  # Calculates Categorical Cross-Entropy (CCE) loss.
  #
  # CCE is used for multi-class classification tasks where only one class is correct.
  # It assumes the true labels are one-hot encoded.
  #
  # @param y_true [Array<Float>] One-hot encoded ground truth
  # @param y_pred [Array<Float>] Model probability distribution
  # @return [Float] The categorical cross-entropy loss
  #
  # @example
  #   LossFunctions.cce([1.0, 0.0], [0.9, 0.1])
  #   #=> 0.052680...
  def cce(y_true, y_pred)
    validate_lengths!(y_true, y_pred)
    sum = 0.0
    y_true.zip(y_pred).each do |t, p|
      p = p.clamp(EPSILON, 1.0 - EPSILON)
      sum += t * Math.log(p)
    end
    -sum / y_true.length
  end

  def validate_lengths!(y_true, y_pred)
    if y_true.length != y_pred.length || y_true.empty?
      raise LengthMismatchError, "Arrays must have the same non-zero length"
    end
  end
end
