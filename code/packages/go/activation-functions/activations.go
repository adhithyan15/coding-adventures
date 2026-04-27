package activation

import "math"

// Sigmoid seamlessly clamps any numerical vector precisely between 0.0 and 1.0 representing strict probabilities.
func Sigmoid(x float64) float64 {
	if x < -709 {
		return 0.0
	}
	if x > 709 {
		return 1.0
	}
	return 1.0 / (1.0 + math.Exp(-x))
}

func SigmoidDerivative(x float64) float64 {
	sig := Sigmoid(x)
	return sig * (1.0 - sig)
}

// Relu annihilates completely negative values while permitting pure linear traversal.
func Relu(x float64) float64 {
	if x > 0 {
		return x
	}
	return 0.0
}

func ReluDerivative(x float64) float64 {
	if x > 0 {
		return 1.0
	}
	return 0.0
}

// Tanh evaluates structurally across negative constraints (-1.0 to 1.0).
func Tanh(x float64) float64 {
	return math.Tanh(x)
}

func TanhDerivative(x float64) float64 {
	t := math.Tanh(x)
	return 1.0 - (t * t)
}
