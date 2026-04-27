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

const leakyReluSlope = 0.01

// Linear returns the raw weighted sum unchanged.
func Linear(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.Linear", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			return rf.Generate(true, false, x)
		}).GetResult()
	return result
}

func LinearDerivative(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.LinearDerivative", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			return rf.Generate(true, false, 1.0)
		}).GetResult()
	return result
}

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

// LeakyRelu keeps a small negative slope so inactive ReLU neurons can still learn.
func LeakyRelu(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.LeakyRelu", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			if x > 0 {
				return rf.Generate(true, false, x)
			}
			return rf.Generate(true, false, leakyReluSlope*x)
		}).GetResult()
	return result
}

func LeakyReluDerivative(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.LeakyReluDerivative", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			if x > 0 {
				return rf.Generate(true, false, 1.0)
			}
			return rf.Generate(true, false, leakyReluSlope)
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

// Softplus is a smooth ReLU approximation using a stable log1p formulation.
func Softplus(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.Softplus", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			value := math.Log1p(math.Exp(-math.Abs(x))) + math.Max(x, 0.0)
			return rf.Generate(true, false, value)
		}).GetResult()
	return result
}

func SoftplusDerivative(x float64) float64 {
	result, _ := StartNew[float64]("activation-functions.SoftplusDerivative", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)
			return rf.Generate(true, false, Sigmoid(x))
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
