package lossfunctions

import (
	"testing"
)

func TestMSED(t *testing.T) {
	yTrue := []float64{1.0, 0.0}
	yPred := []float64{0.8, 0.2}
	result, err := MSED(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !almostEqual(result[0], -0.2) || !almostEqual(result[1], 0.2) {
		t.Errorf("MSED: expected [-0.2, 0.2], got %v", result)
	}
}

func TestMAED(t *testing.T) {
	yTrue := []float64{1.0, 0.0, 0.5}
	yPred := []float64{0.8, 0.2, 0.5}
	result, err := MAED(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !almostEqual(result[0], -1.0/3.0) || !almostEqual(result[1], 1.0/3.0) || !almostEqual(result[2], 0.0) {
		t.Errorf("MAED: expected [-0.33, 0.33, 0.0], got %v", result)
	}
}

func TestBCED(t *testing.T) {
	yTrue := []float64{1.0, 0.0}
	yPred := []float64{0.8, 0.2}
	result, err := BCED(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !almostEqual(result[0], -0.625) || !almostEqual(result[1], 0.625) {
		t.Errorf("BCED: expected [-0.625, 0.625], got %v", result)
	}
}

func TestCCED(t *testing.T) {
	yTrue := []float64{1.0, 0.0}
	yPred := []float64{0.8, 0.2}
	result, err := CCED(yTrue, yPred)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !almostEqual(result[0], -0.625) || !almostEqual(result[1], 0.0) {
		t.Errorf("CCED: expected [-0.625, 0.0], got %v", result)
	}
}
