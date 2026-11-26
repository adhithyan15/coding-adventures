package graph

import (
	"fmt"
	"reflect"
	"sync"
)

// ConcurrentGraph is an opt-in, synchronized handle to one privately owned
// Graph. Copying a handle shares its state and mutex. The owned graph is never
// returned; use Snapshot for a detached read-only view. Construct values with
// NewConcurrentGraph; the zero value is not usable.
type ConcurrentGraph struct {
	state *concurrentGraphState
}

type concurrentGraphState struct {
	mu    sync.Mutex
	graph *Graph
}

// NewConcurrentGraph creates an empty concurrent graph using repr. An empty
// representation selects the same adjacency-list default as New.
func NewConcurrentGraph(repr GraphRepr) *ConcurrentGraph {
	return &ConcurrentGraph{state: &concurrentGraphState{graph: New(repr)}}
}

// Repr reports the graph's immutable storage representation.
func (g *ConcurrentGraph) Repr() GraphRepr {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.Repr()
}

// AddNode adds or updates a node. Property values must be portable immutable
// scalars so snapshots cannot retain caller-owned mutable references.
func (g *ConcurrentGraph) AddNode(node string, properties ...PropertyBag) error {
	if err := validateConcurrentPropertyBags(properties); err != nil {
		return err
	}
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	g.state.graph.AddNode(node, properties...)
	return nil
}

// RemoveNode removes a node and all incident edges atomically.
func (g *ConcurrentGraph) RemoveNode(node string) error {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.RemoveNode(node)
}

func (g *ConcurrentGraph) HasNode(node string) bool {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.HasNode(node)
}

func (g *ConcurrentGraph) Nodes() []string {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.Nodes()
}

// AddEdge inserts both adjacency directions and metadata in one critical
// section. Property values must be portable immutable scalars.
func (g *ConcurrentGraph) AddEdge(left, right string, weight float64, properties ...PropertyBag) error {
	if err := validateConcurrentPropertyBags(properties); err != nil {
		return err
	}
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	g.state.graph.AddEdge(left, right, weight, properties...)
	return nil
}

func (g *ConcurrentGraph) RemoveEdge(left, right string) error {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.RemoveEdge(left, right)
}

func (g *ConcurrentGraph) HasEdge(left, right string) bool {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.HasEdge(left, right)
}

func (g *ConcurrentGraph) EdgeWeight(left, right string) (float64, error) {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.EdgeWeight(left, right)
}

func (g *ConcurrentGraph) Edges() []WeightedEdge {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.Edges()
}

func (g *ConcurrentGraph) Neighbors(node string) ([]string, error) {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.Neighbors(node)
}

func (g *ConcurrentGraph) NeighborsWeighted(node string) (map[string]float64, error) {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.NeighborsWeighted(node)
}

func (g *ConcurrentGraph) Degree(node string) (int, error) {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.Degree(node)
}

func (g *ConcurrentGraph) GraphProperties() PropertyBag {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.GraphProperties()
}

func (g *ConcurrentGraph) SetGraphProperty(key string, value PropertyValue) error {
	if err := validateConcurrentPropertyValue(value); err != nil {
		return err
	}
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	g.state.graph.SetGraphProperty(key, value)
	return nil
}

func (g *ConcurrentGraph) RemoveGraphProperty(key string) {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	g.state.graph.RemoveGraphProperty(key)
}

func (g *ConcurrentGraph) NodeProperties(node string) (PropertyBag, error) {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.NodeProperties(node)
}

func (g *ConcurrentGraph) SetNodeProperty(node, key string, value PropertyValue) error {
	if err := validateConcurrentPropertyValue(value); err != nil {
		return err
	}
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.SetNodeProperty(node, key, value)
}

func (g *ConcurrentGraph) RemoveNodeProperty(node, key string) error {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.RemoveNodeProperty(node, key)
}

func (g *ConcurrentGraph) EdgeProperties(left, right string) (PropertyBag, error) {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.EdgeProperties(left, right)
}

func (g *ConcurrentGraph) SetEdgeProperty(left, right, key string, value PropertyValue) error {
	if err := validateConcurrentPropertyValue(value); err != nil {
		return err
	}
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.SetEdgeProperty(left, right, key, value)
}

func (g *ConcurrentGraph) RemoveEdgeProperty(left, right, key string) error {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.RemoveEdgeProperty(left, right, key)
}

func (g *ConcurrentGraph) Size() int {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.Size()
}

func (g *ConcurrentGraph) String() string {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return g.state.graph.String()
}

// Snapshot copies graph structure and property maps under the mutex. The
// returned query-only value is detached and remains stable after later writes.
func (g *ConcurrentGraph) Snapshot() *GraphSnapshot {
	g.state.mu.Lock()
	defer g.state.mu.Unlock()
	return &GraphSnapshot{graph: cloneConcurrentGraph(g.state.graph)}
}

// GraphSnapshot is an immutable, detached point-in-time graph view. Its query
// methods only expose copied collections and property maps. The zero value is
// invalid; copies of a valid snapshot share the same immutable snapshot.
type GraphSnapshot struct {
	graph *Graph
}

func (s *GraphSnapshot) Repr() GraphRepr                         { return s.graph.Repr() }
func (s *GraphSnapshot) HasNode(node string) bool                { return s.graph.HasNode(node) }
func (s *GraphSnapshot) Nodes() []string                         { return s.graph.Nodes() }
func (s *GraphSnapshot) HasEdge(a, b string) bool                { return s.graph.HasEdge(a, b) }
func (s *GraphSnapshot) EdgeWeight(a, b string) (float64, error) { return s.graph.EdgeWeight(a, b) }
func (s *GraphSnapshot) Edges() []WeightedEdge                   { return s.graph.Edges() }
func (s *GraphSnapshot) Neighbors(n string) ([]string, error)    { return s.graph.Neighbors(n) }
func (s *GraphSnapshot) NeighborsWeighted(n string) (map[string]float64, error) {
	return s.graph.NeighborsWeighted(n)
}
func (s *GraphSnapshot) Degree(n string) (int, error) { return s.graph.Degree(n) }
func (s *GraphSnapshot) GraphProperties() PropertyBag { return s.graph.GraphProperties() }
func (s *GraphSnapshot) NodeProperties(n string) (PropertyBag, error) {
	return s.graph.NodeProperties(n)
}
func (s *GraphSnapshot) EdgeProperties(a, b string) (PropertyBag, error) {
	return s.graph.EdgeProperties(a, b)
}
func (s *GraphSnapshot) Size() int      { return s.graph.Size() }
func (s *GraphSnapshot) String() string { return s.graph.String() }

// ShortestPath computes against this immutable snapshot.
func (s *GraphSnapshot) ShortestPath(start, end string) ([]string, error) {
	return ShortestPath(s.graph, start, end)
}

// MinimumSpanningTree computes against this immutable snapshot.
func (s *GraphSnapshot) MinimumSpanningTree() ([]WeightedEdge, error) {
	return MinimumSpanningTree(s.graph)
}

// EdgeIterator owns a stable edge slice and one cursor. It is intended for a
// single consumer; callers that need a fresh traversal create another
// iterator from the same snapshot. Copying an iterator copies its current
// cursor position, after which both cursors advance independently. Advancing
// the same iterator handle concurrently is not safe.
type EdgeIterator struct {
	edges []WeightedEdge
	index int
}

func (s *GraphSnapshot) EdgesIterator() *EdgeIterator {
	return &EdgeIterator{edges: s.graph.Edges()}
}

// Next returns the next edge and advances this iterator's private cursor.
func (i *EdgeIterator) Next() (WeightedEdge, bool) {
	if i.index >= len(i.edges) {
		return WeightedEdge{}, false
	}
	edge := i.edges[i.index]
	i.index++
	return edge, true
}

func cloneConcurrentGraph(source *Graph) *Graph {
	copy := New(source.repr)
	copy.nodeList = append([]string(nil), source.nodeList...)
	copy.nodeIndex = make(map[string]int, len(source.nodeIndex))
	for node, index := range source.nodeIndex {
		copy.nodeIndex[node] = index
	}
	copy.adj = make(map[string]map[string]float64, len(source.adj))
	for node, neighbors := range source.adj {
		neighborCopy := make(map[string]float64, len(neighbors))
		for neighbor, weight := range neighbors {
			neighborCopy[neighbor] = weight
		}
		copy.adj[node] = neighborCopy
	}
	copy.matrix = make([][]float64, len(source.matrix))
	for row, values := range source.matrix {
		copy.matrix[row] = append([]float64(nil), values...)
	}
	copy.graphProperties = copyPropertyBag(source.graphProperties)
	copy.nodeProperties = clonePropertyBags(source.nodeProperties)
	copy.edgeProperties = clonePropertyBags(source.edgeProperties)
	return copy
}

func clonePropertyBags(source map[string]PropertyBag) map[string]PropertyBag {
	copy := make(map[string]PropertyBag, len(source))
	for key, properties := range source {
		copy[key] = copyPropertyBag(properties)
	}
	return copy
}

func validateConcurrentPropertyBags(bags []PropertyBag) error {
	for _, bag := range bags {
		for key, value := range bag {
			if err := validateConcurrentPropertyValue(value); err != nil {
				return fmt.Errorf("property %q: %w", key, err)
			}
		}
	}
	return nil
}

func validateConcurrentPropertyValue(value PropertyValue) error {
	if value == nil {
		return nil
	}
	kind := reflect.TypeOf(value).Kind()
	switch kind {
	case reflect.String, reflect.Bool,
		reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64,
		reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr,
		reflect.Float32, reflect.Float64:
		return nil
	default:
		return fmt.Errorf("concurrent graph property value %T is not a portable scalar", value)
	}
}
