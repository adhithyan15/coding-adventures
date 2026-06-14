package neuralnetwork

import "testing"

func TestCreatesNeuralGraphMetadata(t *testing.T) {
	graph := CreateNeuralGraph("tiny-model")
	properties := graph.GraphProperties()

	if properties["nn.version"] != "0" || properties["nn.name"] != "tiny-model" {
		t.Fatalf("unexpected graph properties: %#v", properties)
	}
}

func TestAuthorsPrimitiveMetadata(t *testing.T) {
	graph := CreateNeuralGraph("")

	AddInput(graph, "x0", "", nil)
	AddInput(graph, "x1", "feature", nil)
	if err := AddConstant(graph, "bias", 1, nil); err != nil {
		t.Fatal(err)
	}
	AddWeightedSum(graph, "sum", []WeightedInput{
		{From: "x0", Weight: 0.25, EdgeID: "w0", Properties: PropertyBag{"nn.trainable": true}},
		{From: "x1", Weight: 0.75, EdgeID: "w1"},
		{From: "bias", Weight: -0.1, EdgeID: "bias_to_sum"},
	}, nil)
	AddActivation(graph, "relu", "sum", Relu, nil, "sum_to_relu")
	AddOutput(graph, "out", "relu", "prediction", nil, "relu_to_out")

	if graph.NodeProperties("x0")["nn.input"] != "x0" {
		t.Fatalf("input metadata not authored")
	}
	if graph.NodeProperties("sum")["nn.op"] != "weighted_sum" {
		t.Fatalf("weighted_sum metadata not authored")
	}
	if graph.NodeProperties("relu")["nn.activation"] != "relu" {
		t.Fatalf("activation metadata not authored")
	}
	edgeProperties, ok := graph.EdgeProperties("w0")
	if !ok || edgeProperties["weight"] != 0.25 || edgeProperties["nn.trainable"] != true {
		t.Fatalf("edge properties not authored: %#v", edgeProperties)
	}
}

func TestChainableAuthoringAPI(t *testing.T) {
	network := CreateNeuralNetwork("chain").
		Input("x0").
		Input("x1").
		WeightedSum("sum", []WeightedInput{
			{From: "x0", Weight: 0.5, EdgeID: "w0"},
			{From: "x1", Weight: 0.5, EdgeID: "w1"},
		}, nil).
		Activation("relu", "sum", Relu, nil, "sum_to_relu").
		Output("out", "relu", "prediction", nil, "relu_to_out")

	order, err := network.Graph.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	want := []string{"x0", "x1", "sum", "relu", "out"}
	for index, node := range want {
		if order[index] != node {
			t.Fatalf("topological order[%d] = %s, want %s", index, order[index], node)
		}
	}
}

func TestAuthorsXorNetwork(t *testing.T) {
	network := CreateXorNetwork("")

	if network.Graph.GraphProperties()["nn.name"] != "xor" {
		t.Fatalf("xor name missing")
	}
	if network.Graph.NodeProperties("bias")["nn.value"] != 1.0 {
		t.Fatalf("bias metadata missing")
	}
	edgeProperties, ok := network.Graph.EdgeProperties("bias_to_out")
	if !ok || edgeProperties["weight"] != -30.0 {
		t.Fatalf("output bias edge missing: %#v", edgeProperties)
	}
}
