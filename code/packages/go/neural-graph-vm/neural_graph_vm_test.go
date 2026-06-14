package neuralgraphvm

import (
	"errors"
	"testing"

	neuralnetwork "github.com/adhithyan15/coding-adventures/code/packages/go/neural-network"
)

func tinyWeightedSumGraph(t *testing.T) *neuralnetwork.Graph {
	t.Helper()
	graph := neuralnetwork.CreateNeuralGraph("tiny-weighted-sum")
	neuralnetwork.AddInput(graph, "x0", "", nil)
	neuralnetwork.AddInput(graph, "x1", "", nil)
	if err := neuralnetwork.AddConstant(graph, "bias", 1, nil); err != nil {
		t.Fatal(err)
	}
	neuralnetwork.AddWeightedSum(graph, "sum", []neuralnetwork.WeightedInput{
		{From: "bias", Weight: -1, EdgeID: "bias_to_sum"},
		{From: "x0", Weight: 0.25, EdgeID: "w0", Properties: neuralnetwork.PropertyBag{"nn.trainable": true}},
		{From: "x1", Weight: 0.75, EdgeID: "w1", Properties: neuralnetwork.PropertyBag{"nn.trainable": true}},
	}, nil)
	neuralnetwork.AddActivation(graph, "relu", "sum", neuralnetwork.Relu, nil, "sum_to_relu")
	neuralnetwork.AddOutput(graph, "out", "relu", "prediction", nil, "relu_to_out")
	return graph
}

func TestCompilesForwardBytecode(t *testing.T) {
	bytecode, err := CompileNeuralGraphToBytecode(tinyWeightedSumGraph(t))
	if err != nil {
		t.Fatal(err)
	}
	if bytecode.Magic != "CANN" || bytecode.Version != 0 {
		t.Fatalf("unexpected bytecode header: %#v", bytecode)
	}
	ops := []Opcode{}
	for _, instruction := range bytecode.Functions[0].Instructions {
		ops = append(ops, instruction.Op)
	}
	want := []Opcode{LoadConst, LoadInput, LoadInput, LoadEdgeWeight, Mul, LoadEdgeWeight, Mul, LoadEdgeWeight, Mul, Add, Activate, StoreOutput}
	for index, op := range want {
		if ops[index] != op {
			t.Fatalf("op[%d] = %s, want %s", index, ops[index], op)
		}
	}
}

func TestRunsScalarForwardInterpreter(t *testing.T) {
	bytecode, err := CompileNeuralGraphToBytecode(tinyWeightedSumGraph(t))
	if err != nil {
		t.Fatal(err)
	}
	outputs, err := RunNeuralBytecodeForward(bytecode, map[string]float64{"x0": 4, "x1": 8})
	if err != nil {
		t.Fatal(err)
	}
	if outputs["prediction"] != 6 {
		t.Fatalf("prediction = %v, want 6", outputs["prediction"])
	}
}

func TestCompilesChainableNetwork(t *testing.T) {
	network := neuralnetwork.CreateNeuralNetwork("tiny-network").
		Input("x0").
		Input("x1").
		WeightedSum("sum", []neuralnetwork.WeightedInput{
			{From: "x0", Weight: 0.25, EdgeID: "w0"},
			{From: "x1", Weight: 0.75, EdgeID: "w1"},
		}, nil).
		Output("out", "sum", "prediction", nil, "sum_to_out")
	bytecode, err := CompileNeuralNetworkToBytecode(network)
	if err != nil {
		t.Fatal(err)
	}
	outputs, err := RunNeuralBytecodeForward(bytecode, map[string]float64{"x0": 4, "x1": 8})
	if err != nil {
		t.Fatal(err)
	}
	if outputs["prediction"] != 7 {
		t.Fatalf("prediction = %v, want 7", outputs["prediction"])
	}
}

func TestRunsXorNetwork(t *testing.T) {
	bytecode, err := CompileNeuralNetworkToBytecode(neuralnetwork.CreateXorNetwork(""))
	if err != nil {
		t.Fatal(err)
	}
	inputs := []map[string]float64{
		{"x0": 0, "x1": 0},
		{"x0": 0, "x1": 1},
		{"x0": 1, "x1": 0},
		{"x0": 1, "x1": 1},
	}
	predictions := []float64{}
	for _, input := range inputs {
		outputs, err := RunNeuralBytecodeForward(bytecode, input)
		if err != nil {
			t.Fatal(err)
		}
		predictions = append(predictions, outputs["prediction"])
	}
	if predictions[0] >= 0.01 || predictions[1] <= 0.99 || predictions[2] <= 0.99 || predictions[3] >= 0.01 {
		t.Fatalf("xor predictions did not classify: %#v", predictions)
	}
}

func TestRejectsUnsupportedOps(t *testing.T) {
	graph := neuralnetwork.CreateNeuralGraph("")
	graph.AddNode("custom", neuralnetwork.PropertyBag{"nn.op": "custom_kernel"})
	_, err := CompileNeuralGraphToBytecode(graph)
	var compileError CompileError
	if !errors.As(err, &compileError) {
		t.Fatalf("expected CompileError, got %v", err)
	}
}
