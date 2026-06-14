package twolayernetwork

import "testing"

var xorInputs = Matrix{{0, 0}, {0, 1}, {1, 0}, {1, 1}}
var xorTargets = Matrix{{0}, {1}, {1}, {0}}

func TestForwardPassExposesHiddenActivations(t *testing.T) {
	pass, err := Forward(xorInputs, XorWarmStartParameters(), Sigmoid, Sigmoid)
	if err != nil {
		t.Fatal(err)
	}
	if len(pass.HiddenActivations) != 4 || len(pass.HiddenActivations[0]) != 2 {
		t.Fatalf("unexpected hidden activation shape")
	}
	if pass.Predictions[1][0] <= 0.7 {
		t.Fatalf("expected XOR true row to be high")
	}
	if pass.Predictions[0][0] >= 0.3 {
		t.Fatalf("expected XOR false row to be low")
	}
}

func TestTrainingStepExposesBothLayerGradients(t *testing.T) {
	step, err := TrainOneEpoch(xorInputs, xorTargets, XorWarmStartParameters(), 0.5, Sigmoid, Sigmoid)
	if err != nil {
		t.Fatal(err)
	}
	if len(step.InputToHiddenWeightGradients) != 2 || len(step.InputToHiddenWeightGradients[0]) != 2 {
		t.Fatalf("unexpected input-to-hidden gradient shape")
	}
	if len(step.HiddenToOutputWeightGradients) != 2 || len(step.HiddenToOutputWeightGradients[0]) != 1 {
		t.Fatalf("unexpected hidden-to-output gradient shape")
	}
}

func TestWarmStartSolvesXor(t *testing.T) {
	network := New(XorWarmStartParameters(), 0.5)
	predictions, err := network.Predict(xorInputs)
	if err != nil {
		t.Fatal(err)
	}
	if predictions[0][0] >= 0.2 || predictions[1][0] <= 0.7 || predictions[2][0] <= 0.7 || predictions[3][0] >= 0.2 {
		t.Fatalf("warm start did not solve XOR: %#v", predictions)
	}
}

func sampleParameters(inputCount int, hiddenCount int) Parameters {
	weights := make(Matrix, inputCount)
	for feature := 0; feature < inputCount; feature++ {
		weights[feature] = make([]float64, hiddenCount)
		for hidden := 0; hidden < hiddenCount; hidden++ {
			weights[feature][hidden] = 0.17*float64(feature+1) - 0.11*float64(hidden+1)
		}
	}
	hiddenBiases := make([]float64, hiddenCount)
	hiddenToOutput := make(Matrix, hiddenCount)
	for hidden := 0; hidden < hiddenCount; hidden++ {
		hiddenBiases[hidden] = 0.05 * float64(hidden-1)
		hiddenToOutput[hidden] = []float64{0.13*float64(hidden+1) - 0.25}
	}
	return Parameters{
		InputToHiddenWeights:  weights,
		HiddenBiases:          hiddenBiases,
		HiddenToOutputWeights: hiddenToOutput,
		OutputBiases:          []float64{0.02},
	}
}

func TestHiddenLayerTeachingExamplesRunOneTrainingStep(t *testing.T) {
	cases := []struct {
		name        string
		inputs      Matrix
		targets     Matrix
		hiddenCount int
	}{
		{"XNOR", xorInputs, Matrix{{1}, {0}, {0}, {1}}, 3},
		{"absolute value", Matrix{{-1}, {-0.5}, {0}, {0.5}, {1}}, Matrix{{1}, {0.5}, {0}, {0.5}, {1}}, 4},
		{"piecewise pricing", Matrix{{0.1}, {0.3}, {0.5}, {0.7}, {0.9}}, Matrix{{0.12}, {0.25}, {0.55}, {0.88}, {0.88}}, 4},
		{"circle classifier", Matrix{{0, 0}, {0.5, 0}, {1, 1}, {-0.5, 0.5}, {-1, 0}}, Matrix{{1}, {1}, {0}, {1}, {0}}, 5},
		{"two moons", Matrix{{1, 0}, {0, 0.5}, {0.5, 0.85}, {0.5, -0.35}, {-1, 0}, {2, 0.5}}, Matrix{{0}, {1}, {0}, {1}, {0}, {1}}, 5},
		{"interaction features", Matrix{{0.2, 0.25, 0}, {0.6, 0.5, 1}, {1, 0.75, 1}, {1, 1, 0}}, Matrix{{0.08}, {0.72}, {0.96}, {0.76}}, 5},
	}

	for _, tc := range cases {
		step, err := TrainOneEpoch(tc.inputs, tc.targets, sampleParameters(len(tc.inputs[0]), tc.hiddenCount), 0.4, Sigmoid, Sigmoid)
		if err != nil {
			t.Fatalf("%s failed: %v", tc.name, err)
		}
		if step.Loss < 0 {
			t.Fatalf("%s returned a negative loss", tc.name)
		}
		if len(step.InputToHiddenWeightGradients) != len(tc.inputs[0]) {
			t.Fatalf("%s returned unexpected input-to-hidden gradient shape", tc.name)
		}
		if len(step.HiddenToOutputWeightGradients) != tc.hiddenCount {
			t.Fatalf("%s returned unexpected hidden-to-output gradient shape", tc.name)
		}
	}
}
