/**
 * Provides pure, mathematical loss functions used in Machine Learning.
 * This TypeScript implementation adheres strictly to literate programming principles.
 */

// EPSILON prevents taking the logarithm of 0, avoiding -Infinity
const EPSILON = 1e-7;

/**
 * Calculates Mean Squared Error (MSE).
 *
 * MSE measures the average of the squares of the errors. It heavily penalizes larger errors
 * because of the squaring operation. It is most commonly used for regression tasks.
 *
 * Equation:
 * MSE = (1/N) * Σ(y_true_i - y_pred_i)^2
 *
 * @param yTrue - Ground truth actual values
 * @param yPred - Predicted values from the model
 * @returns The mean squared error
 *
 * @example
 * mse([1.0, 0.0], [0.9, 0.1]); // 0.01000...
 */
export function mse(yTrue: number[], yPred: number[]): number {
  if (yTrue.length !== yPred.length || yTrue.length === 0) {
    throw new Error("Arrays must have the same non-zero length");
  }

  let sum = 0.0;
  for (let i = 0; i < yTrue.length; i++) {
    const diff = yTrue[i] - yPred[i];
    sum += diff * diff;
  }

  return sum / yTrue.length;
}

/**
 * Calculates Mean Absolute Error (MAE).
 *
 * MAE measures the absolute magnitude of the errors without considering direction.
 * It is widely used in robust regression to ignore extreme outliers.
 *
 * Equation:
 * MAE = (1/N) * Σ|y_true_i - y_pred_i|
 *
 * @param yTrue - Ground truth actual values
 * @param yPred - Predicted values from the model
 * @returns The mean absolute error
 *
 * @example
 * mae([1.0, 0.0], [0.9, 0.1]); // 0.1
 */
export function mae(yTrue: number[], yPred: number[]): number {
  if (yTrue.length !== yPred.length || yTrue.length === 0) {
    throw new Error("Arrays must have the same non-zero length");
  }

  let sum = 0.0;
  for (let i = 0; i < yTrue.length; i++) {
    sum += Math.abs(yTrue[i] - yPred[i]);
  }

  return sum / yTrue.length;
}

/**
 * Calculates Binary Cross-Entropy (BCE) loss.
 *
 * BCE is used for binary classification tasks. It quantifies the difference
 * between two probability distributions. Predictions must be between 0 and 1.
 *
 * Equation:
 * BCE = -(1/n) * Σ[y_true_i * log(y_pred_i) + (1 - y_true_i) * log(1 - y_pred_i)]
 *
 * @param yTrue - Ground truth labels (usually 0.0 or 1.0)
 * @param yPred - Model predicted probabilities
 * @returns The binary cross-entropy loss
 *
 * @example
 * bce([1.0, 0.0], [0.9, 0.1]); // 0.1053605...
 */
export function bce(yTrue: number[], yPred: number[]): number {
  if (yTrue.length !== yPred.length || yTrue.length === 0) {
    throw new Error("Arrays must have the same non-zero length");
  }

  let sum = 0.0;
  for (let i = 0; i < yTrue.length; i++) {
    const p = Math.max(EPSILON, Math.min(1.0 - EPSILON, yPred[i]));
    sum += yTrue[i] * Math.log(p) + (1.0 - yTrue[i]) * Math.log(1 - p);
  }

  return -sum / yTrue.length;
}

/**
 * Calculates Categorical Cross-Entropy (CCE) loss.
 *
 * CCE is used for multi-class classification tasks. It expects one-hot encoded labels.
 *
 * Equation:
 * CCE = -(1/n) * Σ[y_true_i * log(y_pred_i)]
 *
 * @param yTrue - One-hot encoded ground truth
 * @param yPred - Predicted probability distribution
 * @returns The categorical cross-entropy loss
 *
 * @example
 * cce([1.0, 0.0], [0.9, 0.1]); // 0.05268...
 */
export function cce(yTrue: number[], yPred: number[]): number {
  if (yTrue.length !== yPred.length || yTrue.length === 0) {
    throw new Error("Arrays must have the same non-zero length");
  }

  let sum = 0.0;
  for (let i = 0; i < yTrue.length; i++) {
    const p = Math.max(EPSILON, Math.min(1.0 - EPSILON, yPred[i]));
    sum += yTrue[i] * Math.log(p);
  }

  return -sum / yTrue.length;
}
