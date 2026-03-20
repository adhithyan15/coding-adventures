package gradientdescent

import (
	"math"
	"testing"
)

func almostEqual(a, b float64) bool {
	return math.Abs(a-b) <= 1e-6
}

func TestSGD(t *testing.T) {
	weights := []float64{1.0, -0.5, 2.0}
	gradients := []float64{0.1, -0.2, 0.0}
	lr := 0.1

	res, err := SGD(weights, gradients, lr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !almostEqual(res[0], 0.99) || !almostEqual(res[1], -0.48) || !almostEqual(res[2], 2.0) {
		t.Errorf("SGD: expected [0.99, -0.48, 2.0], got %v", res)
	}
}

func TestSGDErrors(t *testing.T) {
	if _, err := SGD([]float64{1.0}, []float64{}, 0.1); err == nil {
		t.Errorf("expected length mismatch error")
	}
	if _, err := SGD([]float64{}, []float64{}, 0.1); err == nil {
		t.Errorf("expected empty array error")
	}
}
