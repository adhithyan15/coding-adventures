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
	return StartNew[float64]("loss-functions.MSE", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(0, errors.New("slices must have the same non-zero length"))
			}
			var sum float64
			for i := range yTrue {
				diff := yTrue[i] - yPred[i]
				sum += diff * diff
			}
			return rf.Generate(true, false, sum/float64(len(yTrue)))
		}).GetResult()
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
	return StartNew[float64]("loss-functions.MAE", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(0, errors.New("slices must have the same non-zero length"))
			}
			var sum float64
			for i := range yTrue {
				sum += math.Abs(yTrue[i] - yPred[i])
			}
			return rf.Generate(true, false, sum/float64(len(yTrue)))
		}).GetResult()
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
	return StartNew[float64]("loss-functions.BCE", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(0, errors.New("slices must have the same non-zero length"))
			}
			var sum float64
			for i := range yTrue {
				p := math.Max(epsilon, math.Min(1-epsilon, yPred[i]))
				sum += yTrue[i]*math.Log(p) + (1-yTrue[i])*math.Log(1-p)
			}
			return rf.Generate(true, false, -sum/float64(len(yTrue)))
		}).GetResult()
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
	return StartNew[float64]("loss-functions.CCE", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(0, errors.New("slices must have the same non-zero length"))
			}
			var sum float64
			for i := range yTrue {
				p := math.Max(epsilon, math.Min(1-epsilon, yPred[i]))
				sum += yTrue[i] * math.Log(p)
			}
			return rf.Generate(true, false, -sum/float64(len(yTrue)))
		}).GetResult()
}

// MSED calculates the derivative of the Mean Squared Error with respect to predictions.
func MSED(yTrue, yPred []float64) ([]float64, error) {
	return StartNew[[]float64]("loss-functions.MSED", nil,
		func(op *Operation[[]float64], rf *ResultFactory[[]float64]) *OperationResult[[]float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(nil, errors.New("slices must have the same non-zero length"))
			}
			n := float64(len(yTrue))
			res := make([]float64, len(yTrue))
			for i := range yTrue {
				res[i] = (2.0 / n) * (yPred[i] - yTrue[i])
			}
			return rf.Generate(true, false, res)
		}).GetResult()
}

// MAED calculates the derivative of the Mean Absolute Error with respect to predictions.
func MAED(yTrue, yPred []float64) ([]float64, error) {
	return StartNew[[]float64]("loss-functions.MAED", nil,
		func(op *Operation[[]float64], rf *ResultFactory[[]float64]) *OperationResult[[]float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(nil, errors.New("slices must have the same non-zero length"))
			}
			n := float64(len(yTrue))
			res := make([]float64, len(yTrue))
			for i := range yTrue {
				if yPred[i] > yTrue[i] {
					res[i] = 1.0 / n
				} else if yPred[i] < yTrue[i] {
					res[i] = -1.0 / n
				} else {
					res[i] = 0.0
				}
			}
			return rf.Generate(true, false, res)
		}).GetResult()
}

// BCED calculates the derivative of the Binary Cross-Entropy with respect to predictions.
func BCED(yTrue, yPred []float64) ([]float64, error) {
	return StartNew[[]float64]("loss-functions.BCED", nil,
		func(op *Operation[[]float64], rf *ResultFactory[[]float64]) *OperationResult[[]float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(nil, errors.New("slices must have the same non-zero length"))
			}
			n := float64(len(yTrue))
			res := make([]float64, len(yTrue))
			for i := range yTrue {
				p := math.Max(epsilon, math.Min(1-epsilon, yPred[i]))
				res[i] = (1.0 / n) * ((p - yTrue[i]) / (p * (1.0 - p)))
			}
			return rf.Generate(true, false, res)
		}).GetResult()
}

// CCED calculates the derivative of the Categorical Cross-Entropy with respect to predictions.
func CCED(yTrue, yPred []float64) ([]float64, error) {
	return StartNew[[]float64]("loss-functions.CCED", nil,
		func(op *Operation[[]float64], rf *ResultFactory[[]float64]) *OperationResult[[]float64] {
			if len(yTrue) != len(yPred) || len(yTrue) == 0 {
				return rf.Fail(nil, errors.New("slices must have the same non-zero length"))
			}
			n := float64(len(yTrue))
			res := make([]float64, len(yTrue))
			for i := range yTrue {
				p := math.Max(epsilon, math.Min(1-epsilon, yPred[i]))
				res[i] = (-1.0 / n) * (yTrue[i] / p)
			}
			return rf.Generate(true, false, res)
		}).GetResult()
}
