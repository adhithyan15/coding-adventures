// labeled_graph.go -- Labeled Directed Graph
// ============================================
//
// A LabeledGraph extends the basic directed graph with edge labels.
// Each edge can have one or more string labels, turning the graph into
// a "multigraph-like" structure where the same pair of nodes can be
// connected by edges with different semantic meanings.
//
// # Why labeled edges?
//
// In a build system, you might want to distinguish between different
// kinds of dependencies:
//
//   - "compile" dependency: package A needs package B at compile time
//   - "test"    dependency: package A needs package B only for testing
//   - "runtime" dependency: package A needs package B at runtime
//
// With labeled edges, you can query "what are A's compile-time
// dependencies?" without conflating them with test-only dependencies.
//
// # Architecture: composition over inheritance
//
// Rather than duplicating the adjacency-map logic from Graph, LabeledGraph
// wraps a Graph and adds a label map on top:
//
//     ┌─────────────────────────────────────────────────┐
//     │ LabeledGraph                                    │
//     │                                                 │
//     │  ┌───────────────────────┐                      │
//     │  │ graph *Graph          │  ← handles nodes,    │
//     │  │   forward adjacency   │    edges, algorithms  │
//     │  │   reverse adjacency   │                      │
//     │  └───────────────────────┘                      │
//     │                                                 │
//     │  labels map[[2]string]map[string]bool            │
//     │    (from, to) → set of label strings            │
//     │                                                 │
//     └─────────────────────────────────────────────────┘
//
// The underlying Graph stores a single edge between any two nodes,
// regardless of how many labels that edge carries. When all labels
// for an edge are removed, the underlying edge is also removed.
//
// All algorithm methods (TopologicalSort, HasCycle, TransitiveClosure,
// etc.) delegate to the underlying Graph, so they work identically.

package directedgraph

import "sort"

// LabeledGraph is a directed graph where each edge carries one or more
// string labels. Internally, it wraps a Graph (for nodes, edges, and
// algorithms) and adds a label map keyed by (from, to) pairs.
type LabeledGraph struct {
	graph  *Graph                          // underlying unlabeled graph
	labels map[[2]string]map[string]bool   // (from, to) → set of labels
}

// NewLabeledGraph creates an empty labeled directed graph.
// Self-loops are prohibited (inherits from New()).
func NewLabeledGraph() *LabeledGraph {
	return &LabeledGraph{
		graph:  New(),
		labels: make(map[[2]string]map[string]bool),
	}
}

// NewLabeledGraphAllowSelfLoops creates an empty labeled directed graph
// that permits self-loops.
func NewLabeledGraphAllowSelfLoops() *LabeledGraph {
	return &LabeledGraph{
		graph:  NewAllowSelfLoops(),
		labels: make(map[[2]string]map[string]bool),
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Node operations
// ═══════════════════════════════════════════════════════════════════════
// These delegate directly to the underlying Graph.

// AddNode adds a node to the graph. No-op if the node already exists.
func (lg *LabeledGraph) AddNode(node string) {
	lg.graph.AddNode(node)
}

// RemoveNode removes a node and all its incident edges (including labels).
// Returns a NodeNotFoundError if the node doesn't exist.
//
// When a node is removed, we must also clean up any label entries for
// edges that touched that node. We iterate over all label keys and
// remove any that reference the deleted node.
func (lg *LabeledGraph) RemoveNode(node string) error {
	if !lg.graph.HasNode(node) {
		return &NodeNotFoundError{Node: node}
	}

	// Clean up labels for all edges involving this node.
	// We collect keys first to avoid mutating the map during iteration.
	var keysToDelete [][2]string
	for key := range lg.labels {
		if key[0] == node || key[1] == node {
			keysToDelete = append(keysToDelete, key)
		}
	}
	for _, key := range keysToDelete {
		delete(lg.labels, key)
	}

	return lg.graph.RemoveNode(node)
}

// HasNode returns true if the node exists in the graph.
func (lg *LabeledGraph) HasNode(node string) bool {
	return lg.graph.HasNode(node)
}

// Nodes returns all nodes in sorted order (deterministic).
func (lg *LabeledGraph) Nodes() []string {
	return lg.graph.Nodes()
}

// Size returns the number of nodes in the graph.
func (lg *LabeledGraph) Size() int {
	return lg.graph.Size()
}

// ═══════════════════════════════════════════════════════════════════════
// Labeled edge operations
// ═══════════════════════════════════════════════════════════════════════
//
// Each edge in a LabeledGraph carries one or more labels. AddEdge
// requires a label; if you want multiple labels on the same edge,
// call AddEdge multiple times with different labels.
//
// The underlying Graph tracks whether an edge exists at all (for
// algorithm purposes). The label map tracks which labels are on
// each edge.

// AddEdge adds a directed edge from 'from' to 'to' with the given label.
//
// If the edge already exists (possibly with different labels), the new
// label is added to the existing set — the edge is not duplicated in
// the underlying graph.
//
// If the edge does not yet exist, it is created in the underlying graph
// and the label is recorded.
//
// Panics on self-loops if the underlying graph prohibits them.
func (lg *LabeledGraph) AddEdge(from, to, label string) {
	// This may panic if self-loops are not allowed — that's intentional.
	// The underlying graph handles self-loop validation.
	lg.graph.AddEdge(from, to)

	key := [2]string{from, to}
	if lg.labels[key] == nil {
		lg.labels[key] = make(map[string]bool)
	}
	lg.labels[key][label] = true
}

// RemoveEdge removes a specific label from the edge from→to.
//
// If this was the last label on the edge, the underlying edge is also
// removed from the graph. If other labels remain, only the specified
// label is removed and the edge persists.
//
// Returns an error if:
//   - The edge does not exist (EdgeNotFoundError)
//   - The label does not exist on the edge (LabelNotFoundError)
func (lg *LabeledGraph) RemoveEdge(from, to, label string) error {
	key := [2]string{from, to}
	labelSet, exists := lg.labels[key]
	if !exists || !lg.graph.HasEdge(from, to) {
		return &EdgeNotFoundError{From: from, To: to}
	}
	if !labelSet[label] {
		return &LabelNotFoundError{From: from, To: to, Label: label}
	}

	delete(labelSet, label)

	// If no labels remain, remove the underlying edge entirely.
	if len(labelSet) == 0 {
		delete(lg.labels, key)
		return lg.graph.RemoveEdge(from, to)
	}

	return nil
}

// HasEdge returns true if there's any edge from 'from' to 'to'
// (regardless of label).
func (lg *LabeledGraph) HasEdge(from, to string) bool {
	return lg.graph.HasEdge(from, to)
}

// HasEdgeWithLabel returns true if there's an edge from 'from' to 'to'
// with the specific label.
func (lg *LabeledGraph) HasEdgeWithLabel(from, to, label string) bool {
	key := [2]string{from, to}
	if labelSet, ok := lg.labels[key]; ok {
		return labelSet[label]
	}
	return false
}

// Edges returns all edges as [from, to, label] triples, sorted
// deterministically. If an edge has multiple labels, it appears
// once per label.
//
// Example: if edge A→B has labels "compile" and "test", the output
// includes both ["A", "B", "compile"] and ["A", "B", "test"].
func (lg *LabeledGraph) Edges() [][3]string {
	var edges [][3]string
	for key, labelSet := range lg.labels {
		for label := range labelSet {
			edges = append(edges, [3]string{key[0], key[1], label})
		}
	}
	sort.Slice(edges, func(i, j int) bool {
		if edges[i][0] != edges[j][0] {
			return edges[i][0] < edges[j][0]
		}
		if edges[i][1] != edges[j][1] {
			return edges[i][1] < edges[j][1]
		}
		return edges[i][2] < edges[j][2]
	})
	return edges
}

// Labels returns the set of labels on the edge from→to.
// Returns an empty map if the edge doesn't exist.
func (lg *LabeledGraph) Labels(from, to string) map[string]bool {
	key := [2]string{from, to}
	if labelSet, ok := lg.labels[key]; ok {
		// Return a copy to prevent callers from mutating internal state.
		result := make(map[string]bool, len(labelSet))
		for l := range labelSet {
			result[l] = true
		}
		return result
	}
	return make(map[string]bool)
}

// ═══════════════════════════════════════════════════════════════════════
// Neighbor queries
// ═══════════════════════════════════════════════════════════════════════
//
// These methods let you ask "who are my neighbors?" with optional
// label filtering. The unfiltered versions delegate to the underlying
// Graph. The label-filtered versions scan the label map.

// Successors returns the direct successors of a node (any label).
func (lg *LabeledGraph) Successors(node string) ([]string, error) {
	return lg.graph.Successors(node)
}

// SuccessorsWithLabel returns successors connected by edges with the
// given label.
//
// For example, if A→B has label "compile" and A→C has label "test",
// then SuccessorsWithLabel("A", "compile") returns ["B"].
func (lg *LabeledGraph) SuccessorsWithLabel(node, label string) ([]string, error) {
	if !lg.graph.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
	}

	var result []string
	succs, err := lg.graph.Successors(node)
	if err != nil {
		return nil, err
	}
	for _, succ := range succs {
		key := [2]string{node, succ}
		if labelSet, ok := lg.labels[key]; ok && labelSet[label] {
			result = append(result, succ)
		}
	}
	sort.Strings(result)
	return result, nil
}

// Predecessors returns the direct predecessors of a node (any label).
func (lg *LabeledGraph) Predecessors(node string) ([]string, error) {
	return lg.graph.Predecessors(node)
}

// PredecessorsWithLabel returns predecessors connected by edges with
// the given label.
func (lg *LabeledGraph) PredecessorsWithLabel(node, label string) ([]string, error) {
	if !lg.graph.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
	}

	var result []string
	preds, err := lg.graph.Predecessors(node)
	if err != nil {
		return nil, err
	}
	for _, pred := range preds {
		key := [2]string{pred, node}
		if labelSet, ok := lg.labels[key]; ok && labelSet[label] {
			result = append(result, pred)
		}
	}
	sort.Strings(result)
	return result, nil
}

// ═══════════════════════════════════════════════════════════════════════
// Algorithm delegation
// ═══════════════════════════════════════════════════════════════════════
//
// All graph algorithms delegate to the underlying Graph. Labels don't
// affect the structural algorithms — topological sort, cycle detection,
// and transitive closure only care about whether edges exist, not what
// they're labeled.

// TopologicalSort returns nodes in topological order.
// Returns a CycleError if the graph contains a cycle.
func (lg *LabeledGraph) TopologicalSort() ([]string, error) {
	return lg.graph.TopologicalSort()
}

// HasCycle returns true if the graph contains a cycle.
func (lg *LabeledGraph) HasCycle() bool {
	return lg.graph.HasCycle()
}

// TransitiveClosure returns all nodes reachable from the given node
// by following edges forward.
func (lg *LabeledGraph) TransitiveClosure(node string) (map[string]bool, error) {
	return lg.graph.TransitiveClosure(node)
}

// Graph returns the underlying unlabeled Graph, giving access to all
// the base graph methods (IndependentGroups, AffectedNodes, etc.).
func (lg *LabeledGraph) Graph() *Graph {
	return lg.graph
}
