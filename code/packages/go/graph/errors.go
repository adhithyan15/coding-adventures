package graph

import "fmt"

type NodeNotFoundError struct {
	Node string
}

func (e *NodeNotFoundError) Error() string {
	return fmt.Sprintf("node not found: %q", e.Node)
}

type EdgeNotFoundError struct {
	Left  string
	Right string
}

func (e *EdgeNotFoundError) Error() string {
	return fmt.Sprintf("edge not found: %q -- %q", e.Left, e.Right)
}
