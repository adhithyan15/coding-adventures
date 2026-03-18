package directedgraph

import "fmt"

// CycleError is returned when a cycle is detected in the graph.
// A cycle means there's a circular dependency: A depends on B depends
// on C depends on A. This makes it impossible to determine a build order.
type CycleError struct{}

func (e *CycleError) Error() string {
	return "graph contains a cycle"
}

// NodeNotFoundError is returned when operating on a node that doesn't exist.
type NodeNotFoundError struct {
	Node string
}

func (e *NodeNotFoundError) Error() string {
	return fmt.Sprintf("node not found: %q", e.Node)
}

// EdgeNotFoundError is returned when removing an edge that doesn't exist.
type EdgeNotFoundError struct {
	From string
	To   string
}

func (e *EdgeNotFoundError) Error() string {
	return fmt.Sprintf("edge not found: %q -> %q", e.From, e.To)
}
