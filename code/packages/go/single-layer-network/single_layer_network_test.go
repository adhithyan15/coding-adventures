package singlelayernetwork

import "testing"

func near(t *testing.T, actual float64, expected float64) {
	t.Helper()
	if actual < expected-1e-6 || actual > expected+1e-6 {
		t.Fatalf("got %v, want %v", actual, expected)
	}
}

func TestTrainOneEpochExposesMatrixGradients(t *testing.T) {
	step, err := TrainOneEpochWithMatrices(
		Matrix{{1, 2}},
		Matrix{{3, 5}},
		Matrix{{0, 0}, {0, 0}},
		[]float64{0, 0},
		0.1,
		Linear,
	)
	if err != nil {
		t.Fatal(err)
	}
	near(t, step.WeightGradients[0][0], -3)
	near(t, step.WeightGradients[0][1], -5)
	near(t, step.WeightGradients[1][0], -6)
	near(t, step.WeightGradients[1][1], -10)
	near(t, step.NextWeights[0][0], 0.3)
	near(t, step.NextWeights[1][1], 1.0)
	near(t, step.NextBiases[0], 0.3)
}

func TestFitLearnsMInputsToNOutputs(t *testing.T) {
	network := New(3, 2, Linear)
	history, err := network.Fit(
		Matrix{{0, 0, 1}, {1, 2, 1}, {2, 1, 1}},
		Matrix{{1, -1}, {3, 2}, {4, 1}},
		0.05,
		500,
	)
	if err != nil {
		t.Fatal(err)
	}
	if history[len(history)-1].Loss >= history[0].Loss {
		t.Fatalf("loss did not improve")
	}
	prediction, err := network.Predict(Matrix{{1, 1, 1}})
	if err != nil {
		t.Fatal(err)
	}
	if len(prediction[0]) != 2 {
		t.Fatalf("expected two outputs")
	}
}
