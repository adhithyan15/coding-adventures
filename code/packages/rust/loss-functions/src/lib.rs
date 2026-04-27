//! # Loss Functions
//!
//! Provides pure, mathematical loss functions used in Machine Learning.
//!
//! ## Literate Programming Notes
//! This follows literate programming principles, exposing logic plainly
//! and documenting use-cases clearly. It leverages Rust's safety guarantees
//! and slice types to ensure high performance memory-safe processing of 
//! large floating-point arrays.

/// EPSILON is used to clamp predictions before taking the logarithm.
/// Since log(0) evaluates to negative infinity, we clamp predictions to [EPSILON, 1 - EPSILON].
const EPSILON: f64 = 1e-7;

/// Calculates Mean Squared Error (MSE).
///
/// MSE measures the average of the squares of the errors—that is, the average squared
/// difference between the estimated values and the actual value. It is widely used for
/// standard regression tasks. By squaring the errors, MSE heavily penalizes larger errors.
///
/// # Equation
/// `MSE = (1/N) * Σ(y_true_i - y_pred_i)^2`
///
/// # Examples
/// ```
/// use loss_functions::mse;
/// let loss = mse(&[1.0, 0.0], &[0.9, 0.1]).unwrap();
/// assert!((loss - 0.01).abs() < 1e-6);
/// ```
pub fn mse(y_true: &[f64], y_pred: &[f64]) -> Result<f64, &'static str> {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return Err("Slices must have the same non-zero length");
    }

    let mut sum = 0.0;
    for i in 0..y_true.len() {
        let diff = y_true[i] - y_pred[i];
        sum += diff * diff;
    }
    
    Ok(sum / y_true.len() as f64)
}

/// Calculates Mean Absolute Error (MAE).
///
/// MAE measures the average magnitude of the errors in a set of predictions, without 
/// considering their direction. It is the average over the test sample of the absolute 
/// differences between prediction and actual observation. It's often used for 
/// robust regression to ignore extreme outliers.
///
/// # Equation
/// `MAE = (1/N) * Σ|y_true_i - y_pred_i|`
///
/// # Examples
/// ```
/// use loss_functions::mae;
/// let loss = mae(&[1.0, 0.0], &[0.9, 0.1]).unwrap();
/// assert!((loss - 0.1).abs() < 1e-6);
/// ```
pub fn mae(y_true: &[f64], y_pred: &[f64]) -> Result<f64, &'static str> {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return Err("Slices must have the same non-zero length");
    }

    let mut sum = 0.0;
    for i in 0..y_true.len() {
        sum += (y_true[i] - y_pred[i]).abs();
    }
    
    Ok(sum / y_true.len() as f64)
}

/// Calculates Binary Cross-Entropy (BCE) loss.
///
/// BCE is used for binary classification tasks (e.g., Cat vs. Dog). It quantifies the 
/// difference between two probability distributions. Predictions must be between 0 and 1.
/// We apply a small epsilon clamp to prevent taking the log of 0, which would result
/// in negative infinity and disrupt gradient calculations.
///
/// # Equation
/// `BCE = -(1/n) * Σ[y_true_i * log(y_pred_i) + (1 - y_true_i) * log(1 - y_pred_i)]`
///
/// # Examples
/// ```
/// use loss_functions::bce;
/// let loss = bce(&[1.0, 0.0], &[0.9, 0.1]).unwrap();
/// assert!((loss - 0.1053605).abs() < 1e-6);
/// ```
pub fn bce(y_true: &[f64], y_pred: &[f64]) -> Result<f64, &'static str> {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return Err("Slices must have the same non-zero length");
    }

    let mut sum = 0.0;
    for i in 0..y_true.len() {
        let p = y_pred[i].clamp(EPSILON, 1.0 - EPSILON);
        sum += y_true[i] * p.ln() + (1.0 - y_true[i]) * (1.0 - p).ln();
    }
    
    Ok(-sum / y_true.len() as f64)
}

/// Calculates Categorical Cross-Entropy (CCE) loss.
///
/// CCE is used for multi-class classification tasks (e.g., classifying a digit 0-9).
/// It expects `y_true` to be a one-hot encoded vector representing the true class.
/// Only the probability assigned to the true class affects the loss.
///
/// # Equation
/// `CCE = -(1/n) * Σ[y_true_i * log(y_pred_i)]`
///
/// # Examples
/// ```
/// use loss_functions::cce;
/// let loss = cce(&[1.0, 0.0], &[0.9, 0.1]).unwrap();
/// assert!((loss - 0.0526802).abs() < 1e-6);
/// ```
pub fn cce(y_true: &[f64], y_pred: &[f64]) -> Result<f64, &'static str> {
    if y_true.len() != y_pred.len() || y_true.is_empty() {
        return Err("Slices must have the same non-zero length");
    }

    let mut sum = 0.0;
    for i in 0..y_true.len() {
        let p = y_pred[i].clamp(EPSILON, 1.0 - EPSILON);
        sum += y_true[i] * p.ln();
    }
    
    Ok(-sum / y_true.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almost_equal(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-6
    }
    
    const Y_TRUE: &[f64] = &[1.0, 0.0];
    const Y_PRED: &[f64] = &[0.9, 0.1];

    #[test]
    fn test_mse() {
        assert!(almost_equal(mse(Y_TRUE, Y_PRED).unwrap(), 0.010));
    }

    #[test]
    fn test_mae() {
        assert!(almost_equal(mae(Y_TRUE, Y_PRED).unwrap(), 0.100));
    }

    #[test]
    fn test_bce() {
        assert!(almost_equal(bce(Y_TRUE, Y_PRED).unwrap(), 0.1053605));
    }

    #[test]
    fn test_cce() {
        assert!(almost_equal(cce(Y_TRUE, Y_PRED).unwrap(), 0.0526802));
    }

    #[test]
    fn test_errors() {
        assert!(mse(&[1.0], Y_PRED).is_err());
        assert!(mse(&[], &[]).is_err());
        assert!(mae(&[1.0], Y_PRED).is_err());
        assert!(mae(&[], &[]).is_err());
        assert!(bce(&[1.0], Y_PRED).is_err());
        assert!(bce(&[], &[]).is_err());
        assert!(cce(&[1.0], Y_PRED).is_err());
        assert!(cce(&[], &[]).is_err());
    }

    #[test]
    fn test_identical_slices() {
        let identical = &[1.0, 0.0, 0.5];
        assert!(almost_equal(mse(identical, identical).unwrap(), 0.0));
        assert!(almost_equal(mae(identical, identical).unwrap(), 0.0));
    }
}
