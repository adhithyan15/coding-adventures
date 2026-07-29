package multidirectedgraph

import (
	"fmt"
	"math"
	"sort"
)

type PropertyValue any
type PropertyBag map[string]PropertyValue

type Edge[T comparable] struct {
	ID     string
	From   T
	To     T
	Weight float64
}

type NodeNotFoundError[T comparable] struct {
	Node T
}

func (e NodeNotFoundError[T]) Error() string {
	return fmt.Sprintf("node not found: %#v", e.Node)
}

type EdgeNotFoundError struct {
	EdgeID string
}

func (e EdgeNotFoundError) Error() string {
	return fmt.Sprintf("edge not found: %s", e.EdgeID)
}

type DuplicateEdgeIDError struct {
	EdgeID string
}

func (e DuplicateEdgeIDError) Error() string {
	return fmt.Sprintf("edge ID already exists: %s", e.EdgeID)
}

type CycleError struct {
	Processed int
	Total     int
}

func (e CycleError) Error() string {
	return fmt.Sprintf("graph contains a cycle: processed %d/%d nodes", e.Processed, e.Total)
}

type MultiDirectedGraph[T comparable] struct {
	allowSelfLoops bool
	nodes          []T
	nodeSet        map[T]bool
	edges          map[string]Edge[T]
	edgeOrder      []string
	outgoing       map[T][]string
	incoming       map[T][]string

	graphProperties PropertyBag
	nodeProperties  map[T]PropertyBag
	edgeProperties  map[string]PropertyBag
	nextEdgeID      int
}

func New[T comparable]() *MultiDirectedGraph[T] {
	return NewWithSelfLoops[T](false)
}

func NewAllowSelfLoops[T comparable]() *MultiDirectedGraph[T] {
	return NewWithSelfLoops[T](true)
}

func NewWithSelfLoops[T comparable](allowSelfLoops bool) *MultiDirectedGraph[T] {
	return &MultiDirectedGraph[T]{
		allowSelfLoops:  allowSelfLoops,
		nodes:           []T{},
		nodeSet:         map[T]bool{},
		edges:           map[string]Edge[T]{},
		edgeOrder:       []string{},
		outgoing:        map[T][]string{},
		incoming:        map[T][]string{},
		graphProperties: PropertyBag{},
		nodeProperties:  map[T]PropertyBag{},
		edgeProperties:  map[string]PropertyBag{},
	}
}

func (g *MultiDirectedGraph[T]) Size() int {
	return len(g.nodes)
}

func (g *MultiDirectedGraph[T]) AddNode(node T, properties PropertyBag) {
	if !g.nodeSet[node] {
		g.nodeSet[node] = true
		g.nodes = append(g.nodes, node)
		g.outgoing[node] = []string{}
		g.incoming[node] = []string{}
		g.nodeProperties[node] = PropertyBag{}
	}
	for key, value := range properties {
		g.nodeProperties[node][key] = value
	}
}

func (g *MultiDirectedGraph[T]) RemoveNode(node T) error {
	if !g.nodeSet[node] {
		return NodeNotFoundError[T]{Node: node}
	}
	edgeIDs := append([]string{}, g.outgoing[node]...)
	edgeIDs = append(edgeIDs, g.incoming[node]...)
	sort.Strings(edgeIDs)
	edgeIDs = compactStrings(edgeIDs)
	for _, edgeID := range edgeIDs {
		if err := g.RemoveEdge(edgeID); err != nil {
			return err
		}
	}
	delete(g.nodeSet, node)
	g.nodes = removeNode(g.nodes, node)
	delete(g.outgoing, node)
	delete(g.incoming, node)
	delete(g.nodeProperties, node)
	return nil
}

func (g *MultiDirectedGraph[T]) HasNode(node T) bool {
	return g.nodeSet[node]
}

func (g *MultiDirectedGraph[T]) Nodes() []T {
	return append([]T(nil), g.nodes...)
}

func (g *MultiDirectedGraph[T]) AddEdge(from, to T, weight float64, properties PropertyBag, edgeID string) (string, error) {
	if from == to && !g.allowSelfLoops {
		return "", fmt.Errorf("self-loops are not allowed: %#v -> %#v", from, to)
	}
	if math.IsNaN(weight) {
		return "", fmt.Errorf("edge weight must not be NaN")
	}
	if edgeID == "" {
		edgeID = g.allocateEdgeID()
	}
	if _, exists := g.edges[edgeID]; exists {
		return "", DuplicateEdgeIDError{EdgeID: edgeID}
	}
	g.AddNode(from, nil)
	g.AddNode(to, nil)
	g.edges[edgeID] = Edge[T]{ID: edgeID, From: from, To: to, Weight: weight}
	g.edgeOrder = append(g.edgeOrder, edgeID)
	g.outgoing[from] = append(g.outgoing[from], edgeID)
	g.incoming[to] = append(g.incoming[to], edgeID)
	merged := cloneBag(properties)
	merged["weight"] = weight
	g.edgeProperties[edgeID] = merged
	return edgeID, nil
}

func (g *MultiDirectedGraph[T]) RemoveEdge(edgeID string) error {
	edge, err := g.Edge(edgeID)
	if err != nil {
		return err
	}
	g.outgoing[edge.From] = removeString(g.outgoing[edge.From], edgeID)
	g.incoming[edge.To] = removeString(g.incoming[edge.To], edgeID)
	delete(g.edges, edgeID)
	g.edgeOrder = removeString(g.edgeOrder, edgeID)
	delete(g.edgeProperties, edgeID)
	return nil
}

func (g *MultiDirectedGraph[T]) HasEdge(edgeID string) bool {
	_, ok := g.edges[edgeID]
	return ok
}

func (g *MultiDirectedGraph[T]) Edge(edgeID string) (Edge[T], error) {
	edge, ok := g.edges[edgeID]
	if !ok {
		return Edge[T]{}, EdgeNotFoundError{EdgeID: edgeID}
	}
	return edge, nil
}

func (g *MultiDirectedGraph[T]) Edges() []Edge[T] {
	result := []Edge[T]{}
	for _, edgeID := range g.edgeOrder {
		result = append(result, g.edges[edgeID])
	}
	return result
}

func (g *MultiDirectedGraph[T]) EdgesBetween(from, to T) ([]Edge[T], error) {
	if !g.nodeSet[from] {
		return nil, NodeNotFoundError[T]{Node: from}
	}
	if !g.nodeSet[to] {
		return nil, NodeNotFoundError[T]{Node: to}
	}
	result := []Edge[T]{}
	outgoing, err := g.OutgoingEdges(from)
	if err != nil {
		return nil, err
	}
	for _, edge := range outgoing {
		if edge.To == to {
			result = append(result, edge)
		}
	}
	return result, nil
}

func (g *MultiDirectedGraph[T]) OutgoingEdges(node T) ([]Edge[T], error) {
	if !g.nodeSet[node] {
		return nil, NodeNotFoundError[T]{Node: node}
	}
	result := []Edge[T]{}
	for _, edgeID := range g.outgoing[node] {
		result = append(result, g.edges[edgeID])
	}
	return result, nil
}

func (g *MultiDirectedGraph[T]) IncomingEdges(node T) ([]Edge[T], error) {
	if !g.nodeSet[node] {
		return nil, NodeNotFoundError[T]{Node: node}
	}
	result := []Edge[T]{}
	for _, edgeID := range g.incoming[node] {
		result = append(result, g.edges[edgeID])
	}
	return result, nil
}

func (g *MultiDirectedGraph[T]) Successors(node T) ([]T, error) {
	if !g.nodeSet[node] {
		return nil, NodeNotFoundError[T]{Node: node}
	}
	seen := map[T]bool{}
	result := []T{}
	outgoing, err := g.OutgoingEdges(node)
	if err != nil {
		return nil, err
	}
	for _, edge := range outgoing {
		if !seen[edge.To] {
			seen[edge.To] = true
			result = append(result, edge.To)
		}
	}
	return result, nil
}

func (g *MultiDirectedGraph[T]) Predecessors(node T) ([]T, error) {
	if !g.nodeSet[node] {
		return nil, NodeNotFoundError[T]{Node: node}
	}
	seen := map[T]bool{}
	result := []T{}
	incoming, err := g.IncomingEdges(node)
	if err != nil {
		return nil, err
	}
	for _, edge := range incoming {
		if !seen[edge.From] {
			seen[edge.From] = true
			result = append(result, edge.From)
		}
	}
	return result, nil
}

func (g *MultiDirectedGraph[T]) EdgeWeight(edgeID string) (float64, error) {
	edge, err := g.Edge(edgeID)
	if err != nil {
		return 0, err
	}
	return edge.Weight, nil
}

func (g *MultiDirectedGraph[T]) GraphProperties() PropertyBag {
	return cloneBag(g.graphProperties)
}

func (g *MultiDirectedGraph[T]) SetGraphProperty(key string, value PropertyValue) {
	g.graphProperties[key] = value
}

func (g *MultiDirectedGraph[T]) RemoveGraphProperty(key string) {
	delete(g.graphProperties, key)
}

func (g *MultiDirectedGraph[T]) NodeProperties(node T) (PropertyBag, error) {
	if !g.nodeSet[node] {
		return nil, NodeNotFoundError[T]{Node: node}
	}
	return cloneBag(g.nodeProperties[node]), nil
}

func (g *MultiDirectedGraph[T]) SetNodeProperty(node T, key string, value PropertyValue) error {
	if !g.nodeSet[node] {
		return NodeNotFoundError[T]{Node: node}
	}
	g.nodeProperties[node][key] = value
	return nil
}

func (g *MultiDirectedGraph[T]) RemoveNodeProperty(node T, key string) error {
	if !g.nodeSet[node] {
		return NodeNotFoundError[T]{Node: node}
	}
	delete(g.nodeProperties[node], key)
	return nil
}

func (g *MultiDirectedGraph[T]) EdgeProperties(edgeID string) (PropertyBag, error) {
	edge, err := g.Edge(edgeID)
	if err != nil {
		return nil, err
	}
	properties := cloneBag(g.edgeProperties[edgeID])
	properties["weight"] = edge.Weight
	return properties, nil
}

func (g *MultiDirectedGraph[T]) SetEdgeProperty(edgeID, key string, value PropertyValue) error {
	edge, ok := g.edges[edgeID]
	if !ok {
		return EdgeNotFoundError{EdgeID: edgeID}
	}
	if key == "weight" {
		weight, ok := numberValue(value)
		if !ok {
			return fmt.Errorf("edge property 'weight' must be numeric")
		}
		if math.IsNaN(weight) {
			return fmt.Errorf("edge weight must not be NaN")
		}
		edge.Weight = weight
		g.edges[edgeID] = edge
	}
	g.edgeProperties[edgeID][key] = value
	return nil
}

func (g *MultiDirectedGraph[T]) RemoveEdgeProperty(edgeID, key string) error {
	edge, ok := g.edges[edgeID]
	if !ok {
		return EdgeNotFoundError{EdgeID: edgeID}
	}
	if key == "weight" {
		edge.Weight = 1
		g.edges[edgeID] = edge
		g.edgeProperties[edgeID]["weight"] = 1.0
		return nil
	}
	delete(g.edgeProperties[edgeID], key)
	return nil
}

func (g *MultiDirectedGraph[T]) TopologicalSort() ([]T, error) {
	indegree := map[T]int{}
	for _, node := range g.nodes {
		indegree[node] = len(g.incoming[node])
	}
	ready := []T{}
	for _, node := range g.nodes {
		if indegree[node] == 0 {
			ready = append(ready, node)
		}
	}
	sortNodes(ready)
	order := []T{}
	for len(ready) > 0 {
		node := ready[0]
		ready = ready[1:]
		order = append(order, node)
		outgoing, err := g.OutgoingEdges(node)
		if err != nil {
			return nil, err
		}
		for _, edge := range outgoing {
			indegree[edge.To]--
			if indegree[edge.To] == 0 {
				ready = append(ready, edge.To)
				sortNodes(ready)
			}
		}
	}
	if len(order) != len(g.nodes) {
		return nil, CycleError{Processed: len(order), Total: len(g.nodes)}
	}
	return order, nil
}

func (g *MultiDirectedGraph[T]) HasCycle() bool {
	_, err := g.TopologicalSort()
	return err != nil
}

func (g *MultiDirectedGraph[T]) IndependentGroups() ([][]T, error) {
	indegree := map[T]int{}
	for _, node := range g.nodes {
		indegree[node] = len(g.incoming[node])
	}
	current := []T{}
	for _, node := range g.nodes {
		if indegree[node] == 0 {
			current = append(current, node)
		}
	}
	sortNodes(current)
	groups := [][]T{}
	processed := 0
	for len(current) > 0 {
		level := append([]T(nil), current...)
		groups = append(groups, level)
		processed += len(level)
		next := []T{}
		seenNext := map[T]bool{}
		for _, node := range current {
			outgoing, err := g.OutgoingEdges(node)
			if err != nil {
				return nil, err
			}
			for _, edge := range outgoing {
				indegree[edge.To]--
				if indegree[edge.To] == 0 && !seenNext[edge.To] {
					seenNext[edge.To] = true
					next = append(next, edge.To)
				}
			}
		}
		sortNodes(next)
		current = next
	}
	if processed != len(g.nodes) {
		return nil, CycleError{Processed: processed, Total: len(g.nodes)}
	}
	return groups, nil
}

func (g *MultiDirectedGraph[T]) allocateEdgeID() string {
	edgeID := fmt.Sprintf("e%d", g.nextEdgeID)
	for {
		if _, exists := g.edges[edgeID]; !exists {
			g.nextEdgeID++
			return edgeID
		}
		g.nextEdgeID++
		edgeID = fmt.Sprintf("e%d", g.nextEdgeID)
	}
}

func sortNodes[T comparable](nodes []T) {
	sort.Slice(nodes, func(i, j int) bool {
		return fmt.Sprintf("%T:%#v", nodes[i], nodes[i]) < fmt.Sprintf("%T:%#v", nodes[j], nodes[j])
	})
}

func cloneBag(input PropertyBag) PropertyBag {
	output := PropertyBag{}
	for key, value := range input {
		output[key] = value
	}
	return output
}

func numberValue(value PropertyValue) (float64, bool) {
	switch typed := value.(type) {
	case int:
		return float64(typed), true
	case int8:
		return float64(typed), true
	case int16:
		return float64(typed), true
	case int32:
		return float64(typed), true
	case int64:
		return float64(typed), true
	case uint:
		return float64(typed), true
	case uint8:
		return float64(typed), true
	case uint16:
		return float64(typed), true
	case uint32:
		return float64(typed), true
	case uint64:
		return float64(typed), true
	case float32:
		return float64(typed), true
	case float64:
		return typed, true
	default:
		return 0, false
	}
}

func compactStrings(values []string) []string {
	result := []string{}
	for _, value := range values {
		if len(result) == 0 || result[len(result)-1] != value {
			result = append(result, value)
		}
	}
	return result
}

func removeString(values []string, target string) []string {
	result := values[:0]
	for _, value := range values {
		if value != target {
			result = append(result, value)
		}
	}
	return result
}

func removeNode[T comparable](values []T, target T) []T {
	result := values[:0]
	for _, value := range values {
		if value != target {
			result = append(result, value)
		}
	}
	return result
}
