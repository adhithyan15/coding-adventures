package neuralnetwork

import (
	"fmt"
	"math"
	"sort"
)

type PropertyValue any
type PropertyBag map[string]PropertyValue

type ActivationKind string

const (
	Relu    ActivationKind = "relu"
	Sigmoid ActivationKind = "sigmoid"
	Tanh    ActivationKind = "tanh"
	None    ActivationKind = "none"
)

type Edge struct {
	ID         string
	From       string
	To         string
	Weight     float64
	Properties PropertyBag
}

type WeightedInput struct {
	From       string
	Weight     float64
	EdgeID     string
	Properties PropertyBag
}

type Graph struct {
	graphProperties PropertyBag
	nodes           []string
	nodeProperties  map[string]PropertyBag
	edges           []Edge
	nextEdgeID      int
}

func NewGraph(name string) *Graph {
	graph := &Graph{
		graphProperties: PropertyBag{"nn.version": "0"},
		nodes:           []string{},
		nodeProperties:  map[string]PropertyBag{},
		edges:           []Edge{},
	}
	if name != "" {
		graph.graphProperties["nn.name"] = name
	}
	return graph
}

func (g *Graph) GraphProperties() PropertyBag {
	return cloneBag(g.graphProperties)
}

func (g *Graph) AddNode(node string, properties PropertyBag) {
	if _, ok := g.nodeProperties[node]; !ok {
		g.nodes = append(g.nodes, node)
		g.nodeProperties[node] = PropertyBag{}
	}
	for key, value := range properties {
		g.nodeProperties[node][key] = value
	}
}

func (g *Graph) Nodes() []string {
	return append([]string(nil), g.nodes...)
}

func (g *Graph) NodeProperties(node string) PropertyBag {
	return cloneBag(g.nodeProperties[node])
}

func (g *Graph) AddEdge(from, to string, weight float64, properties PropertyBag, edgeID string) string {
	g.AddNode(from, nil)
	g.AddNode(to, nil)
	if edgeID == "" {
		edgeID = fmt.Sprintf("e%d", g.nextEdgeID)
		g.nextEdgeID++
	}
	merged := cloneBag(properties)
	merged["weight"] = weight
	g.edges = append(g.edges, Edge{
		ID:         edgeID,
		From:       from,
		To:         to,
		Weight:     weight,
		Properties: merged,
	})
	return edgeID
}

func (g *Graph) Edges() []Edge {
	return append([]Edge(nil), g.edges...)
}

func (g *Graph) IncomingEdges(node string) []Edge {
	incoming := []Edge{}
	for _, edge := range g.edges {
		if edge.To == node {
			incoming = append(incoming, edge)
		}
	}
	return incoming
}

func (g *Graph) EdgeProperties(edgeID string) (PropertyBag, bool) {
	for _, edge := range g.edges {
		if edge.ID == edgeID {
			return cloneBag(edge.Properties), true
		}
	}
	return nil, false
}

func (g *Graph) TopologicalSort() ([]string, error) {
	indegree := map[string]int{}
	outgoing := map[string][]string{}
	for _, node := range g.nodes {
		indegree[node] = 0
		outgoing[node] = []string{}
	}
	for _, edge := range g.edges {
		indegree[edge.To]++
		outgoing[edge.From] = append(outgoing[edge.From], edge.To)
	}

	ready := []string{}
	for _, node := range g.nodes {
		if indegree[node] == 0 {
			ready = append(ready, node)
		}
	}
	sort.Strings(ready)
	order := []string{}
	for len(ready) > 0 {
		node := ready[0]
		ready = ready[1:]
		order = append(order, node)
		for _, successor := range outgoing[node] {
			indegree[successor]--
			if indegree[successor] == 0 {
				ready = append(ready, successor)
				sort.Strings(ready)
			}
		}
	}
	if len(order) != len(g.nodes) {
		return nil, fmt.Errorf("neural graph contains a cycle")
	}
	return order, nil
}

type NeuralNetwork struct {
	Graph *Graph
}

func NewNetwork(name string) *NeuralNetwork {
	return &NeuralNetwork{Graph: NewGraph(name)}
}

func (n *NeuralNetwork) Input(node string) *NeuralNetwork {
	AddInput(n.Graph, node, node, nil)
	return n
}

func (n *NeuralNetwork) Constant(node string, value float64, properties PropertyBag) *NeuralNetwork {
	AddConstant(n.Graph, node, value, properties)
	return n
}

func (n *NeuralNetwork) WeightedSum(node string, inputs []WeightedInput, properties PropertyBag) *NeuralNetwork {
	AddWeightedSum(n.Graph, node, inputs, properties)
	return n
}

func (n *NeuralNetwork) Activation(node, input string, activation ActivationKind, properties PropertyBag, edgeID string) *NeuralNetwork {
	AddActivation(n.Graph, node, input, activation, properties, edgeID)
	return n
}

func (n *NeuralNetwork) Output(node, input, outputName string, properties PropertyBag, edgeID string) *NeuralNetwork {
	AddOutput(n.Graph, node, input, outputName, properties, edgeID)
	return n
}

func CreateNeuralGraph(name string) *Graph {
	return NewGraph(name)
}

func CreateNeuralNetwork(name string) *NeuralNetwork {
	return NewNetwork(name)
}

func AddInput(graph *Graph, node, inputName string, properties PropertyBag) {
	if inputName == "" {
		inputName = node
	}
	graph.AddNode(node, mergeBag(properties, PropertyBag{"nn.op": "input", "nn.input": inputName}))
}

func AddConstant(graph *Graph, node string, value float64, properties PropertyBag) error {
	if math.IsNaN(value) || math.IsInf(value, 0) {
		return fmt.Errorf("constant value must be finite")
	}
	graph.AddNode(node, mergeBag(properties, PropertyBag{"nn.op": "constant", "nn.value": value}))
	return nil
}

func AddWeightedSum(graph *Graph, node string, inputs []WeightedInput, properties PropertyBag) {
	graph.AddNode(node, mergeBag(properties, PropertyBag{"nn.op": "weighted_sum"}))
	for _, input := range inputs {
		graph.AddEdge(input.From, node, input.Weight, input.Properties, input.EdgeID)
	}
}

func AddActivation(graph *Graph, node, input string, activation ActivationKind, properties PropertyBag, edgeID string) string {
	graph.AddNode(node, mergeBag(properties, PropertyBag{"nn.op": "activation", "nn.activation": string(activation)}))
	return graph.AddEdge(input, node, 1, nil, edgeID)
}

func AddOutput(graph *Graph, node, input, outputName string, properties PropertyBag, edgeID string) string {
	if outputName == "" {
		outputName = node
	}
	graph.AddNode(node, mergeBag(properties, PropertyBag{"nn.op": "output", "nn.output": outputName}))
	return graph.AddEdge(input, node, 1, nil, edgeID)
}

func CreateXorNetwork(name string) *NeuralNetwork {
	if name == "" {
		name = "xor"
	}
	return CreateNeuralNetwork(name).
		Input("x0").
		Input("x1").
		Constant("bias", 1, PropertyBag{"nn.role": "bias"}).
		WeightedSum("h_or_sum", []WeightedInput{
			{From: "x0", Weight: 20, EdgeID: "x0_to_h_or"},
			{From: "x1", Weight: 20, EdgeID: "x1_to_h_or"},
			{From: "bias", Weight: -10, EdgeID: "bias_to_h_or"},
		}, PropertyBag{"nn.layer": "hidden", "nn.role": "weighted_sum"}).
		Activation("h_or", "h_or_sum", Sigmoid, PropertyBag{"nn.layer": "hidden", "nn.role": "activation"}, "h_or_sum_to_h_or").
		WeightedSum("h_nand_sum", []WeightedInput{
			{From: "x0", Weight: -20, EdgeID: "x0_to_h_nand"},
			{From: "x1", Weight: -20, EdgeID: "x1_to_h_nand"},
			{From: "bias", Weight: 30, EdgeID: "bias_to_h_nand"},
		}, PropertyBag{"nn.layer": "hidden", "nn.role": "weighted_sum"}).
		Activation("h_nand", "h_nand_sum", Sigmoid, PropertyBag{"nn.layer": "hidden", "nn.role": "activation"}, "h_nand_sum_to_h_nand").
		WeightedSum("out_sum", []WeightedInput{
			{From: "h_or", Weight: 20, EdgeID: "h_or_to_out"},
			{From: "h_nand", Weight: 20, EdgeID: "h_nand_to_out"},
			{From: "bias", Weight: -30, EdgeID: "bias_to_out"},
		}, PropertyBag{"nn.layer": "output", "nn.role": "weighted_sum"}).
		Activation("out_activation", "out_sum", Sigmoid, PropertyBag{"nn.layer": "output", "nn.role": "activation"}, "out_sum_to_activation").
		Output("out", "out_activation", "prediction", PropertyBag{"nn.layer": "output"}, "activation_to_out")
}

func cloneBag(input PropertyBag) PropertyBag {
	output := PropertyBag{}
	for key, value := range input {
		output[key] = value
	}
	return output
}

func mergeBag(left PropertyBag, right PropertyBag) PropertyBag {
	output := cloneBag(left)
	for key, value := range right {
		output[key] = value
	}
	return output
}
