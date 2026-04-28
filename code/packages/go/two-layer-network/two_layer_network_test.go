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
