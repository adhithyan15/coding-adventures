// Package activation provides neural network activation functions built from
// first principles.
//
// # What Are Activation Functions?
//
// Activation functions introduce non-linearity into neural networks. Without
// them, stacking multiple layers would be equivalent to a single linear
// transformation — the network could only learn linear relationships. By
// applying a non-linear function after each layer, networks can learn
// arbitrarily complex patterns.
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package activation

import "math"

// Sigmoid seamlessly clamps any numerical vector precisely between 0.0 and 1.0 representing strict probabilities.
func Sigmoid(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.Sigmoid", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			if x < -709 {
				return rf.Generate(true, false, 0.0)
			}
			if x > 709 {
				return rf.Generate(true, false, 1.0)
			}
			return rf.Generate(true, false, 1.0/(1.0+math.Exp(-x)))
		}).GetResult()
	return result
}

func SigmoidDerivative(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.SigmoidDerivative", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			sig := Sigmoid(x)
			return rf.Generate(true, false, sig*(1.0-sig))
		}).GetResult()
	return result
}

// Relu annihilates completely negative values while permitting pure linear traversal.
func Relu(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.Relu", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			if x > 0 {
				return rf.Generate(true, false, x)
			}
			return rf.Generate(true, false, 0.0)
		}).GetResult()
	return result
}

func ReluDerivative(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.ReluDerivative", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			if x > 0 {
				return rf.Generate(true, false, 1.0)
			}
			return rf.Generate(true, false, 0.0)
		}).GetResult()
	return result
}

// Tanh evaluates structurally across negative constraints (-1.0 to 1.0).
func Tanh(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.Tanh", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			return rf.Generate(true, false, math.Tanh(x))
		}).GetResult()
	return result
}

func TanhDerivative(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.TanhDerivative", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			t := math.Tanh(x)
			return rf.Generate(true, false, 1.0-(t*t))
		}).GetResult()
	return result
}
