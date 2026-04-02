// Package tree provides a rooted tree data structure backed by a directed graph.
//
// # What is a Tree?
//
// A tree is one of the most fundamental data structures in computer science.
// You encounter trees everywhere:
//
//   - File systems: directories contain files and subdirectories
//   - HTML/XML: elements contain child elements
//   - Programming languages: Abstract Syntax Trees (ASTs) represent code structure
//   - Organization charts: managers have direct reports
//
// Formally, a tree is a connected, acyclic graph where:
//
//  1. There is exactly one root node (a node with no parent).
//  2. Every other node has exactly one parent.
//  3. There are no cycles.
//
// These constraints mean a tree with N nodes always has exactly N-1 edges.
//
// # Tree vs. Graph
//
// A tree IS a graph (specifically, a directed acyclic graph with the
// single-parent constraint). We leverage this by building our Tree on top
// of the Graph type from the directed-graph package. The Graph handles
// all the low-level node/edge storage, while this Tree type enforces the
// tree invariants and provides tree-specific operations like traversals,
// depth calculation, and lowest common ancestor.
//
// Edges point from parent to child:
//
//	Program
//	├── Assignment    (edge: Program → Assignment)
//	│   ├── Name      (edge: Assignment → Name)
//	│   └── BinaryOp  (edge: Assignment → BinaryOp)
//	└── Print         (edge: Program → Print)
//
// # Implementation Strategy
//
// We store the tree as a Graph with edges pointing parent → child.
// This means:
//
//   - graph.Successors(node) returns the children
//   - graph.Predecessors(node) returns a slice with 0 or 1 element
//     (the parent, or empty for the root)
//
// We maintain the tree invariants by checking them in AddChild:
//
//   - The parent must already exist in the tree
//   - The child must NOT already exist (no duplicate nodes)
//   - Since we only add one parent edge per child, cycles are impossible
package tree

import (
	"fmt"
	"sort"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
)

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------
// Trees impose strict structural constraints. When those constraints are
// violated, we return specific error types so callers can handle them.

// NodeNotFoundError is returned when an operation references a node
// that doesn't exist in the tree.
type NodeNotFoundError struct {
	Node string
}

func (e *NodeNotFoundError) Error() string {
	return fmt.Sprintf("node not found in tree: %q", e.Node)
}

// DuplicateNodeError is returned when trying to add a node that already
// exists. In a tree, every node has exactly one parent, so duplicates
// would violate the tree invariant.
type DuplicateNodeError struct {
	Node string
}

func (e *DuplicateNodeError) Error() string {
	return fmt.Sprintf("node already exists in tree: %q", e.Node)
}

// RootRemovalError is returned when trying to remove the root node.
// The root is the anchor of the entire tree; removing it would leave
// a disconnected collection of subtrees.
type RootRemovalError struct{}

func (e *RootRemovalError) Error() string {
	return "cannot remove the root node"
}

// ---------------------------------------------------------------------------
// The Tree struct
// ---------------------------------------------------------------------------

// Tree is a rooted tree backed by a directed graph.
//
// A tree is a directed graph with three constraints:
//  1. Exactly one root (no predecessors)
//  2. Every non-root node has exactly one parent
//  3. No cycles
//
// Edges point parent → child. Build the tree by specifying a root node,
// then adding children one at a time with AddChild.
//
// Example:
//
//	t := tree.New("Program")
//	t.AddChild("Program", "Assignment")
//	t.AddChild("Program", "Print")
//	t.AddChild("Assignment", "Name")
//	t.AddChild("Assignment", "BinaryOp")
//	fmt.Println(t.ToAscii())
type Tree struct {
	graph *directedgraph.Graph
	root  string
}

// New creates a new tree with the given root node.
//
// The root will be the ancestor of every other node in the tree.
// A tree always starts with a root — you can't have an empty tree.
func New(root string) *Tree {
	result, _ := StartNew[*Tree]("tree.New", nil,
		func(op *Operation[*Tree], rf *ResultFactory[*Tree]) *OperationResult[*Tree] {
			op.AddProperty("root", root)
			g := directedgraph.New()
			g.AddNode(root)
			return rf.Generate(true, false, &Tree{graph: g, root: root})
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Mutation
// ---------------------------------------------------------------------------

// AddChild adds a child node under the given parent.
//
// This is the primary way to build up a tree. Each call adds one new
// node and one edge (parent → child).
//
// Returns NodeNotFoundError if parent is not in the tree.
// Returns DuplicateNodeError if child already exists.
func (t *Tree) AddChild(parent, child string) error {
	_, err := StartNew[struct{}]("tree.AddChild", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("parent", parent)
			op.AddProperty("child", child)
			if !t.graph.HasNode(parent) {
				return rf.Fail(struct{}{}, &NodeNotFoundError{Node: parent})
			}
			if t.graph.HasNode(child) {
				return rf.Fail(struct{}{}, &DuplicateNodeError{Node: child})
			}
			t.graph.AddEdge(parent, child)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// RemoveSubtree removes a node and all its descendants from the tree.
//
// This is a "prune" operation — it cuts off an entire branch. The parent
// of the removed node is unaffected.
//
// Returns NodeNotFoundError if node is not in the tree.
// Returns RootRemovalError if node is the root.
func (t *Tree) RemoveSubtree(node string) error {
	_, err := StartNew[struct{}]("tree.RemoveSubtree", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(struct{}{}, &NodeNotFoundError{Node: node})
			}
			if node == t.root {
				return rf.Fail(struct{}{}, &RootRemovalError{})
			}

			toRemove := t.collectSubtreeNodes(node)
			for i := len(toRemove) - 1; i >= 0; i-- {
				_ = t.graph.RemoveNode(toRemove[i])
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// collectSubtreeNodes collects all nodes in the subtree rooted at node
// using BFS. Returns a slice starting with node, then all descendants.
func (t *Tree) collectSubtreeNodes(node string) []string {
	var result []string
	queue := []string{node}

	for len(queue) > 0 {
		current := queue[0]
		queue = queue[1:]
		result = append(result, current)

		children, _ := t.graph.Successors(current)
		sort.Strings(children)
		queue = append(queue, children...)
	}

	return result
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

// Root returns the root node of the tree.
func (t *Tree) Root() string {
	result, _ := StartNew[string]("tree.Root", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, t.root)
		}).GetResult()
	return result
}

// Parent returns the parent of a node, or "" if the node is the root.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Parent(node string) (string, error) {
	return StartNew[string]("tree.Parent", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail("", &NodeNotFoundError{Node: node})
			}

			preds, _ := t.graph.Predecessors(node)
			if len(preds) == 0 {
				return rf.Generate(true, false, "")
			}
			return rf.Generate(true, false, preds[0])
		}).GetResult()
}

// Children returns the children of a node (sorted alphabetically).
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Children(node string) ([]string, error) {
	return StartNew[[]string]("tree.Children", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(nil, &NodeNotFoundError{Node: node})
			}

			children, _ := t.graph.Successors(node)
			sort.Strings(children)
			return rf.Generate(true, false, children)
		}).GetResult()
}

// Siblings returns the siblings of a node (other children of the same parent).
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Siblings(node string) ([]string, error) {
	return StartNew[[]string]("tree.Siblings", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(nil, &NodeNotFoundError{Node: node})
			}

			parentNode, err := t.Parent(node)
			if err != nil {
				return rf.Fail(nil, err)
			}
			if parentNode == "" {
				return rf.Generate(true, false, []string{})
			}

			allChildren, _ := t.Children(parentNode)
			var siblings []string
			for _, c := range allChildren {
				if c != node {
					siblings = append(siblings, c)
				}
			}
			if siblings == nil {
				siblings = []string{}
			}
			return rf.Generate(true, false, siblings)
		}).GetResult()
}

// IsLeaf returns true if the node has no children.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) IsLeaf(node string) (bool, error) {
	return StartNew[bool]("tree.IsLeaf", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(false, &NodeNotFoundError{Node: node})
			}
			children, _ := t.graph.Successors(node)
			return rf.Generate(true, false, len(children) == 0)
		}).GetResult()
}

// IsRoot returns true if the node is the root of the tree.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) IsRoot(node string) (bool, error) {
	return StartNew[bool]("tree.IsRoot", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(false, &NodeNotFoundError{Node: node})
			}
			return rf.Generate(true, false, node == t.root)
		}).GetResult()
}

// Depth returns the depth of a node (distance from root).
//
// Root = 0, its children = 1, grandchildren = 2, etc.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Depth(node string) (int, error) {
	return StartNew[int]("tree.Depth", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(0, &NodeNotFoundError{Node: node})
			}

			d := 0
			current := node
			for current != t.root {
				preds, _ := t.graph.Predecessors(current)
				current = preds[0]
				d++
			}

			return rf.Generate(true, false, d)
		}).GetResult()
}

// Height returns the height of the tree (maximum depth of any node).
//
// A single-node tree has height 0.
func (t *Tree) Height() int {
	result, _ := StartNew[int]("tree.Height", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			maxDepth := 0
			type item struct {
				node  string
				depth int
			}
			queue := []item{{t.root, 0}}

			for len(queue) > 0 {
				current := queue[0]
				queue = queue[1:]

				if current.depth > maxDepth {
					maxDepth = current.depth
				}

				children, _ := t.graph.Successors(current.node)
				for _, child := range children {
					queue = append(queue, item{child, current.depth + 1})
				}
			}

			return rf.Generate(true, false, maxDepth)
		}).GetResult()
	return result
}

// Size returns the total number of nodes in the tree.
func (t *Tree) Size() int {
	result, _ := StartNew[int]("tree.Size", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, t.graph.Size())
		}).GetResult()
	return result
}

// Nodes returns all nodes in the tree (sorted alphabetically).
func (t *Tree) Nodes() []string {
	result, _ := StartNew[[]string]("tree.Nodes", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			nodes := t.graph.Nodes()
			sort.Strings(nodes)
			return rf.Generate(true, false, nodes)
		}).GetResult()
	return result
}

// Leaves returns all leaf nodes (sorted alphabetically).
func (t *Tree) Leaves() []string {
	result, _ := StartNew[[]string]("tree.Leaves", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			var leaves []string
			for _, n := range t.graph.Nodes() {
				children, _ := t.graph.Successors(n)
				if len(children) == 0 {
					leaves = append(leaves, n)
				}
			}
			sort.Strings(leaves)
			return rf.Generate(true, false, leaves)
		}).GetResult()
	return result
}

// HasNode returns true if the node exists in the tree.
func (t *Tree) HasNode(node string) bool {
	result, _ := StartNew[bool]("tree.HasNode", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("node", node)
			return rf.Generate(true, false, t.graph.HasNode(node))
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Traversals
// ---------------------------------------------------------------------------
//
// Tree traversals visit every node exactly once, in different orders.
//
// 1. Preorder (root first): Visit a node, then visit all its children.
//    Top-down. Good for: copying a tree, prefix notation.
//
// 2. Postorder (root last): Visit all children, then the node.
//    Bottom-up. Good for: computing sizes, deleting trees.
//
// 3. LevelOrder (BFS): Visit all nodes at depth 0, then 1, then 2, etc.
//
// For a tree:
//
//	    A
//	   / \
//	  B   C
//	 / \
//	D   E
//
// Preorder:    A, B, D, E, C
// Postorder:   D, E, B, C, A
// LevelOrder:  A, B, C, D, E

// Preorder returns nodes in preorder (parent before children).
//
// Uses an explicit stack. Children are pushed in reverse sorted order
// so the smallest pops first.
func (t *Tree) Preorder() []string {
	result, _ := StartNew[[]string]("tree.Preorder", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			var nodes []string
			stack := []string{t.root}

			for len(stack) > 0 {
				node := stack[len(stack)-1]
				stack = stack[:len(stack)-1]
				nodes = append(nodes, node)

				children, _ := t.graph.Successors(node)
				sort.Sort(sort.Reverse(sort.StringSlice(children)))
				stack = append(stack, children...)
			}

			return rf.Generate(true, false, nodes)
		}).GetResult()
	return result
}

// Postorder returns nodes in postorder (children before parent).
//
// Uses a recursive helper. Children visited in sorted order.
func (t *Tree) Postorder() []string {
	result, _ := StartNew[[]string]("tree.Postorder", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			var nodes []string
			t.postorderRecursive(t.root, &nodes)
			return rf.Generate(true, false, nodes)
		}).GetResult()
	return result
}

func (t *Tree) postorderRecursive(node string, result *[]string) {
	children, _ := t.graph.Successors(node)
	sort.Strings(children)
	for _, child := range children {
		t.postorderRecursive(child, result)
	}
	*result = append(*result, node)
}

// LevelOrder returns nodes in level-order (breadth-first).
//
// Classic BFS using a queue. Children visited in sorted order.
func (t *Tree) LevelOrder() []string {
	result, _ := StartNew[[]string]("tree.LevelOrder", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			var nodes []string
			queue := []string{t.root}

			for len(queue) > 0 {
				node := queue[0]
				queue = queue[1:]
				nodes = append(nodes, node)

				children, _ := t.graph.Successors(node)
				sort.Strings(children)
				queue = append(queue, children...)
			}

			return rf.Generate(true, false, nodes)
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

// PathTo returns the path from the root to the given node.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) PathTo(node string) ([]string, error) {
	return StartNew[[]string]("tree.PathTo", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(nil, &NodeNotFoundError{Node: node})
			}

			var path []string
			current := node

			for current != "" {
				path = append(path, current)
				parent, _ := t.Parent(current)
				current = parent
			}

			// Reverse
			for i, j := 0, len(path)-1; i < j; i, j = i+1, j-1 {
				path[i], path[j] = path[j], path[i]
			}

			return rf.Generate(true, false, path)
		}).GetResult()
}

// LCA returns the lowest common ancestor of nodes a and b.
//
// The LCA is the deepest node that is an ancestor of both a and b.
//
// Returns NodeNotFoundError if a or b doesn't exist.
func (t *Tree) LCA(a, b string) (string, error) {
	return StartNew[string]("tree.LCA", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("a", a)
			op.AddProperty("b", b)
			if !t.graph.HasNode(a) {
				return rf.Fail("", &NodeNotFoundError{Node: a})
			}
			if !t.graph.HasNode(b) {
				return rf.Fail("", &NodeNotFoundError{Node: b})
			}

			pathA, _ := t.PathTo(a)
			pathB, _ := t.PathTo(b)

			lcaNode := t.root
			minLen := len(pathA)
			if len(pathB) < minLen {
				minLen = len(pathB)
			}

			for i := 0; i < minLen; i++ {
				if pathA[i] == pathB[i] {
					lcaNode = pathA[i]
				} else {
					break
				}
			}

			return rf.Generate(true, false, lcaNode)
		}).GetResult()
}

// Subtree extracts the subtree rooted at the given node.
//
// Returns a NEW Tree object. The original tree is not modified.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Subtree(node string) (*Tree, error) {
	return StartNew[*Tree]("tree.Subtree", nil,
		func(op *Operation[*Tree], rf *ResultFactory[*Tree]) *OperationResult[*Tree] {
			op.AddProperty("node", node)
			if !t.graph.HasNode(node) {
				return rf.Fail(nil, &NodeNotFoundError{Node: node})
			}

			newTree := New(node)
			queue := []string{node}

			for len(queue) > 0 {
				current := queue[0]
				queue = queue[1:]

				children, _ := t.graph.Successors(current)
				sort.Strings(children)
				for _, child := range children {
					_ = newTree.AddChild(current, child)
					queue = append(queue, child)
				}
			}

			return rf.Generate(true, false, newTree)
		}).GetResult()
}

// ---------------------------------------------------------------------------
// Visualization
// ---------------------------------------------------------------------------

// ToAscii renders the tree as an ASCII art diagram.
//
// Produces output like:
//
//	Program
//	├── Assignment
//	│   ├── BinaryOp
//	│   └── Name
//	└── Print
func (t *Tree) ToAscii() string {
	result, _ := StartNew[string]("tree.ToAscii", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			var lines []string
			t.asciiRecursive(t.root, "", "", &lines)
			return rf.Generate(true, false, strings.Join(lines, "\n"))
		}).GetResult()
	return result
}

func (t *Tree) asciiRecursive(node, prefix, childPrefix string, lines *[]string) {
	*lines = append(*lines, prefix+node)
	children, _ := t.graph.Successors(node)
	sort.Strings(children)

	for i, child := range children {
		if i < len(children)-1 {
			t.asciiRecursive(child, childPrefix+"├── ", childPrefix+"│   ", lines)
		} else {
			t.asciiRecursive(child, childPrefix+"└── ", childPrefix+"    ", lines)
		}
	}
}

// ---------------------------------------------------------------------------
// Graph access
// ---------------------------------------------------------------------------

// Graph returns the underlying directed graph.
func (t *Tree) Graph() *directedgraph.Graph {
	result, _ := StartNew[*directedgraph.Graph]("tree.Graph", nil,
		func(op *Operation[*directedgraph.Graph], rf *ResultFactory[*directedgraph.Graph]) *OperationResult[*directedgraph.Graph] {
			return rf.Generate(true, false, t.graph)
		}).GetResult()
	return result
}

// String returns a string representation showing root and size.
func (t *Tree) String() string {
	result, _ := StartNew[string]("tree.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf("Tree(root=%q, size=%d)", t.root, t.Size()))
		}).GetResult()
	return result
}
