// Package lossfunctions provides pure, composable mathematical operations
// for calculating standard machine learning loss (error) metrics.
//
// Literate Programming Notes:
// As a teaching toolkit, this package eschews complex object-oriented patterns
// in favor of pure functions that operate directly on plain floating-point slices.
package lossfunctions

import (
	"errors"
	"math"
)

// epsilon is used to clamp probabilities within cross-entropy logarithms.
// By bounding probabilities to [epsilon, 1-epsilon], we mathematically prevent
// the evaluation of math.Log(0), which would result in NaN or -Inf and catastrophically
// break gradient descent during backpropagation.
const epsilon = 1e-7

// MSE calculates the Mean Squared Error between true labels and predictions.
//
// Mean Squared Error is widely used for Regression problems. By squaring the differences,
// it naturally heavily penalizes predictions that are far away from the true label.
//
// Equation:
//
//	MSE = (1/N) * Σ(yTrue_i - yPred_i)^2
//
// Example:
//
//	loss, _ := MSE([]float64{1.0, 0.0}, []float64{0.9, 0.1})
//	// loss == 0.01
func MSE(yTrue, yPred []float64) (float64, error) {
	if len(yTrue) != len(yPred) || len(yTrue) == 0 {
		return 0, errors.New("slices must have the same non-zero length")
	}
	var sum float64
	for i := range yTrue {
		diff := yTrue[i] - yPred[i]
		sum += diff * diff
	}
	return sum / float64(len(yTrue)), nil
}

// MAE calculates the Mean Absolute Error between true labels and predictions.
//
// MAE measures the absolute magnitude of the errors without considering direction.
// It is widely used in Robust Regression to ignore extreme outliers.
//
// Equation:
//
//	MAE = (1/N) * Σ|yTrue_i - yPred_i|
//
// Example:
//
//	loss, _ := MAE([]float64{1.0, 0.0}, []float64{0.9, 0.1})
//	// loss == 0.1
func MAE(yTrue, yPred []float64) (float64, error) {
	if len(yTrue) != len(yPred) || len(yTrue) == 0 {
		return 0, errors.New("slices must have the same non-zero length")
	}
	var sum float64
	for i := range yTrue {
		sum += math.Abs(yTrue[i] - yPred[i])
	}
	return sum / float64(len(yTrue)), nil
}

// BCE calculates the Binary Cross-Entropy loss between true labels and predictions.
//
// BCE is used for binary classification tasks. It quantifies the difference
// between two probability distributions. Predictions must be between 0 and 1.
//
// Equation:
//
//	BCE = -(1/n) * Σ[yTrue_i * log(yPred_i) + (1 - yTrue_i) * log(1 - yPred_i)]
//
// Example:
//
//	loss, _ := BCE([]float64{1.0, 0.0}, []float64{0.9, 0.1})
//	// loss == 0.1053605
func BCE(yTrue, yPred []float64) (float64, error) {
	if len(yTrue) != len(yPred) || len(yTrue) == 0 {
		return 0, errors.New("slices must have the same non-zero length")
	}
	var sum float64
	for i := range yTrue {
		p := math.Max(epsilon, math.Min(1-epsilon, yPred[i]))
		sum += yTrue[i]*math.Log(p) + (1-yTrue[i])*math.Log(1-p)
	}
	return -sum / float64(len(yTrue)), nil
}

// CCE calculates the Categorical Cross-Entropy loss between true labels and predictions.
//
// CCE is used for multi-class classification tasks where only one class is correct.
// It assumes the true labels are one-hot encoded.
//
// Equation:
//
//	CCE = -(1/n) * Σ[yTrue_i * log(yPred_i)]
//
// Example:
//
//	loss, _ := CCE([]float64{1.0, 0.0}, []float64{0.9, 0.1})
//	// loss == 0.0526802
func CCE(yTrue, yPred []float64) (float64, error) {
	if len(yTrue) != len(yPred) || len(yTrue) == 0 {
		return 0, errors.New("slices must have the same non-zero length")
	}
	var sum float64
	for i := range yTrue {
		p := math.Max(epsilon, math.Min(1-epsilon, yPred[i]))
		sum += yTrue[i] * math.Log(p)
	}
	return -sum / float64(len(yTrue)), nil
}
