package perceptron

import "testing"

func TestNewPreservesTrainingConfiguration(t *testing.T) {
	model := New(0.25, 12)

	if model.LearningRate != 0.25 {
		t.Fatalf("LearningRate = %v, want 0.25", model.LearningRate)
	}
	if model.Epochs != 12 {
		t.Fatalf("Epochs = %d, want 12", model.Epochs)
	}
	if model.Weights != nil {
		t.Fatalf("Weights = %#v before Fit, want nil", model.Weights)
	}
	if model.Bias != 0 {
		t.Fatalf("Bias = %v before Fit, want 0", model.Bias)
	}
}

func TestPredictBeforeFitReturnsNil(t *testing.T) {
	model := New(0.1, 1)

	if predictions := model.Predict([][]float64{{0, 0}}); predictions != nil {
		t.Fatalf("Predict before Fit = %#v, want nil", predictions)
	}
}

func TestFitLearnsAndGate(t *testing.T) {
	features := [][]float64{
		{0, 0},
		{0, 1},
		{1, 0},
		{1, 1},
	}
	labels := [][]float64{{0}, {0}, {0}, {1}}
	model := New(0.8, 5000)

	// logSteps is deliberately larger than the epoch count. Fit still emits its
	// deterministic epoch-zero progress line, without dividing by zero or
	// flooding the test output.
	model.Fit(features, labels, 5001)

	if model.Weights == nil {
		t.Fatal("Weights remain nil after Fit")
	}
	if model.Weights.Rows != 2 || model.Weights.Cols != 1 {
		t.Fatalf("Weights shape = %dx%d, want 2x1", model.Weights.Rows, model.Weights.Cols)
	}

	predictions := model.Predict(features)
	if len(predictions) != len(labels) {
		t.Fatalf("prediction count = %d, want %d", len(predictions), len(labels))
	}
	for index, prediction := range predictions[:3] {
		if prediction >= 0.2 {
			t.Fatalf("negative AND row %d prediction = %v, want < 0.2", index, prediction)
		}
	}
	if predictions[3] <= 0.8 {
		t.Fatalf("positive AND row prediction = %v, want > 0.8", predictions[3])
	}
}
