package graph

import (
	"errors"
	"fmt"
)

// ErrPathDistanceOverflow indicates that a topological path exists but every
// candidate path distance overflows float64 during accumulation.
var ErrPathDistanceOverflow = errors.New("shortest path distance exceeds float64 range")

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
