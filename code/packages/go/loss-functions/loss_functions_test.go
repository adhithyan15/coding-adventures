package lossfunctions

import (
	"math"
	"testing"
)

// Shared parity test vector across all languages:
// y_true = [1.0, 0.0]
// y_pred = [0.9, 0.1]

func almostEqual(a, b float64) bool {
	return math.Abs(a-b) <= 1e-6
}

func TestMSE(t *testing.T) {
	yTrue := []float64{1.0, 0.0}
	yPred := []float64{0.9, 0.1}

	result, err := MSE(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	expected := 0.010
	if !almostEqual(result, expected) {
		t.Errorf("MSE: expected %v, got %v", expected, result)
	}
}

func TestMAE(t *testing.T) {
	yTrue := []float64{1.0, 0.0}
	yPred := []float64{0.9, 0.1}

	result, err := MAE(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	expected := 0.100
	if !almostEqual(result, expected) {
		t.Errorf("MAE: expected %v, got %v", expected, result)
	}
}

func TestBCE(t *testing.T) {
	yTrue := []float64{1.0, 0.0}
	yPred := []float64{0.9, 0.1}

	result, err := BCE(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	expected := 0.1053605
	if !almostEqual(result, expected) {
		t.Errorf("BCE: expected %v, got %v", expected, result)
	}
}

func TestCCE(t *testing.T) {
	yTrue := []float64{1.0, 0.0}
	yPred := []float64{0.9, 0.1}

	result, err := CCE(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	expected := 0.0526802
	if !almostEqual(result, expected) {
		t.Errorf("CCE: expected %v, got %v", expected, result)
	}
}

func TestLengthMismatch(t *testing.T) {
	yTrue := []float64{1.0}
	yPred := []float64{0.9, 0.1}

	_, err := MSE(yTrue, yPred)
	if err == nil {
		t.Errorf("expected error for mismatched lengths, got nil")
	}
}

func TestEmptySlices(t *testing.T) {
	var yTrue, yPred []float64

	if _, err := MSE(yTrue, yPred); err == nil {
		t.Errorf("MSE: expected error for empty slices, got nil")
	}
	if _, err := MAE(yTrue, yPred); err == nil {
		t.Errorf("MAE: expected error for empty slices, got nil")
	}
	if _, err := BCE(yTrue, yPred); err == nil {
		t.Errorf("BCE: expected error for empty slices, got nil")
	}
	if _, err := CCE(yTrue, yPred); err == nil {
		t.Errorf("CCE: expected error for empty slices, got nil")
	}
}

func TestIdenticalSlices(t *testing.T) {
	yTrue := []float64{1.0, 0.0, 0.5}
	yPred := []float64{1.0, 0.0, 0.5}

	mseRes, _ := MSE(yTrue, yPred)
	if !almostEqual(mseRes, 0.0) {
		t.Errorf("MSE: identical should be 0, got %v", mseRes)
	}

	maeRes, _ := MAE(yTrue, yPred)
	if !almostEqual(maeRes, 0.0) {
		t.Errorf("MAE: identical should be 0, got %v", maeRes)
	}
}
