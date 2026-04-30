package neuralgraphvm

import (
	"fmt"
	"math"
	"sort"

	neuralnetwork "github.com/adhithyan15/coding-adventures/code/packages/go/neural-network"
)

type Opcode string

const (
	LoadInput      Opcode = "LOAD_INPUT"
	LoadConst      Opcode = "LOAD_CONST"
	LoadEdgeWeight Opcode = "LOAD_EDGE_WEIGHT"
	Mul            Opcode = "MUL"
	Add            Opcode = "ADD"
	Activate       Opcode = "ACTIVATE"
	StoreOutput    Opcode = "STORE_OUTPUT"
)

type Instruction struct {
	Op         Opcode
	Dst        string
	InputName  string
	OutputName string
	EdgeID     string
	Value      float64
	Left       string
	Right      string
	Inputs     []string
	Input      string
	Activation string
	SourceNode string
	SourceEdge string
}

type Function struct {
	ID           string
	Kind         string
	Instructions []Instruction
}

type GraphEdge struct {
	ID     string
	From   string
	To     string
	Weight float64
}

type Module struct {
	Magic     string
	Version   int
	Nodes     []string
	Edges     []GraphEdge
	Functions []Function
}

type CompileError struct {
	Message string
}

func (e CompileError) Error() string {
	return e.Message
}

func CompileNeuralNetworkToBytecode(network *neuralnetwork.NeuralNetwork) (Module, error) {
	return CompileNeuralGraphToBytecode(network.Graph)
}

func CompileNeuralGraphToBytecode(graph *neuralnetwork.Graph) (Module, error) {
	order, err := graph.TopologicalSort()
	if err != nil {
		return Module{}, err
	}
	instructions := []Instruction{}
	values := map[string]string{}
	nextValueID := 0
	allocateValue := func() string {
		valueID := fmt.Sprintf("v%d", nextValueID)
		nextValueID++
		return valueID
	}

	for _, node := range order {
		properties := graph.NodeProperties(node)
		op, _ := properties["nn.op"].(string)
		if op == "" {
			op = "weighted_sum"
		}

		switch op {
		case "input":
			dst := allocateValue()
			values[node] = dst
			inputName, _ := properties["nn.input"].(string)
			if inputName == "" {
				inputName = node
			}
			instructions = append(instructions, Instruction{Op: LoadInput, Dst: dst, InputName: inputName, SourceNode: node})
		case "constant":
			dst := allocateValue()
			values[node] = dst
			value, ok := numberProperty(properties["nn.value"])
			if !ok {
				return Module{}, CompileError{Message: "constant node is missing nn.value"}
			}
			instructions = append(instructions, Instruction{Op: LoadConst, Dst: dst, Value: value, SourceNode: node})
		case "weighted_sum":
			incoming := graph.IncomingEdges(node)
			sort.Slice(incoming, func(i, j int) bool { return incoming[i].ID < incoming[j].ID })
			terms := []string{}
			for _, edge := range incoming {
				sourceValue, ok := values[edge.From]
				if !ok {
					return Module{}, CompileError{Message: fmt.Sprintf("source node has no value: %s", edge.From)}
				}
				weightValue := allocateValue()
				termValue := allocateValue()
				instructions = append(instructions, Instruction{Op: LoadEdgeWeight, Dst: weightValue, EdgeID: edge.ID, SourceEdge: edge.ID})
				instructions = append(instructions, Instruction{Op: Mul, Dst: termValue, Left: sourceValue, Right: weightValue, SourceEdge: edge.ID})
				terms = append(terms, termValue)
			}
			dst := allocateValue()
			values[node] = dst
			if len(terms) == 0 {
				instructions = append(instructions, Instruction{Op: LoadConst, Dst: dst, Value: 0, SourceNode: node})
			} else {
				instructions = append(instructions, Instruction{Op: Add, Dst: dst, Inputs: terms, SourceNode: node})
			}
		case "activation":
			inputValue, err := singleInputValue(graph, values, node)
			if err != nil {
				return Module{}, err
			}
			dst := allocateValue()
			values[node] = dst
			activation, _ := properties["nn.activation"].(string)
			if activation == "" {
				activation = "relu"
			}
			instructions = append(instructions, Instruction{Op: Activate, Dst: dst, Input: inputValue, Activation: activation, SourceNode: node})
		case "output":
			inputValue, err := singleInputValue(graph, values, node)
			if err != nil {
				return Module{}, err
			}
			values[node] = inputValue
			outputName, _ := properties["nn.output"].(string)
			if outputName == "" {
				outputName = node
			}
			instructions = append(instructions, Instruction{Op: StoreOutput, OutputName: outputName, Input: inputValue, SourceNode: node})
		default:
			return Module{}, CompileError{Message: fmt.Sprintf("unsupported neural graph op: %s", op)}
		}
	}

	graphEdges := []GraphEdge{}
	for _, edge := range graph.Edges() {
		graphEdges = append(graphEdges, GraphEdge{ID: edge.ID, From: edge.From, To: edge.To, Weight: edge.Weight})
	}
	return Module{
		Magic:   "CANN",
		Version: 0,
		Nodes:   graph.Nodes(),
		Edges:   graphEdges,
		Functions: []Function{{
			ID:           "forward",
			Kind:         "forward",
			Instructions: instructions,
		}},
	}, nil
}

func RunNeuralBytecodeForward(module Module, inputs map[string]float64) (map[string]float64, error) {
	var forward *Function
	for index := range module.Functions {
		if module.Functions[index].Kind == "forward" {
			forward = &module.Functions[index]
			break
		}
	}
	if forward == nil {
		return nil, fmt.Errorf("neural bytecode module has no forward function")
	}

	values := map[string]float64{}
	edgeWeights := map[string]float64{}
	for _, edge := range module.Edges {
		edgeWeights[edge.ID] = edge.Weight
	}
	outputs := map[string]float64{}

	read := func(valueID string) (float64, error) {
		value, ok := values[valueID]
		if !ok {
			return 0, fmt.Errorf("missing value: %s", valueID)
		}
		return value, nil
	}

	for _, instruction := range forward.Instructions {
		switch instruction.Op {
		case LoadInput:
			value, ok := inputs[instruction.InputName]
			if !ok {
				return nil, fmt.Errorf("missing input: %s", instruction.InputName)
			}
			values[instruction.Dst] = value
		case LoadConst:
			values[instruction.Dst] = instruction.Value
		case LoadEdgeWeight:
			values[instruction.Dst] = edgeWeights[instruction.EdgeID]
		case Mul:
			left, err := read(instruction.Left)
			if err != nil {
				return nil, err
			}
			right, err := read(instruction.Right)
			if err != nil {
				return nil, err
			}
			values[instruction.Dst] = left * right
		case Add:
			total := 0.0
			for _, valueID := range instruction.Inputs {
				value, err := read(valueID)
				if err != nil {
					return nil, err
				}
				total += value
			}
			values[instruction.Dst] = total
		case Activate:
			value, err := read(instruction.Input)
			if err != nil {
				return nil, err
			}
			values[instruction.Dst] = ApplyNeuralActivation(value, instruction.Activation)
		case StoreOutput:
			value, err := read(instruction.Input)
			if err != nil {
				return nil, err
			}
			outputName := instruction.OutputName
			if outputName == "" {
				outputName = "output"
			}
			outputs[outputName] = value
		}
	}
	return outputs, nil
}

func ApplyNeuralActivation(value float64, activation string) float64 {
	switch activation {
	case "relu":
		if value > 0 {
			return value
		}
		return 0
	case "sigmoid":
		clamped := math.Max(-500, math.Min(500, value))
		return 1 / (1 + math.Exp(-clamped))
	case "tanh":
		return math.Tanh(value)
	case "none":
		return value
	default:
		return value
	}
}

func singleInputValue(graph *neuralnetwork.Graph, values map[string]string, node string) (string, error) {
	incoming := graph.IncomingEdges(node)
	sort.Slice(incoming, func(i, j int) bool { return incoming[i].ID < incoming[j].ID })
	if len(incoming) != 1 {
		return "", CompileError{Message: fmt.Sprintf("expected exactly one input edge for %s", node)}
	}
	value, ok := values[incoming[0].From]
	if !ok {
		return "", CompileError{Message: fmt.Sprintf("source node has no value: %s", incoming[0].From)}
	}
	return value, nil
}

func numberProperty(value any) (float64, bool) {
	switch typed := value.(type) {
	case float64:
		return typed, true
	case int:
		return float64(typed), true
	default:
		return 0, false
	}
}
