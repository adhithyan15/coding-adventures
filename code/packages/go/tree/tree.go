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
	g := directedgraph.New()
	g.AddNode(root)
	return &Tree{graph: g, root: root}
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
	if !t.graph.HasNode(parent) {
		return &NodeNotFoundError{Node: parent}
	}
	if t.graph.HasNode(child) {
		return &DuplicateNodeError{Node: child}
	}

	t.graph.AddEdge(parent, child)
	return nil
}

// RemoveSubtree removes a node and all its descendants from the tree.
//
// This is a "prune" operation — it cuts off an entire branch. The parent
// of the removed node is unaffected.
//
// Returns NodeNotFoundError if node is not in the tree.
// Returns RootRemovalError if node is the root.
func (t *Tree) RemoveSubtree(node string) error {
	if !t.graph.HasNode(node) {
		return &NodeNotFoundError{Node: node}
	}
	if node == t.root {
		return &RootRemovalError{}
	}

	// Collect subtree via BFS, then remove in reverse (children first)
	toRemove := t.collectSubtreeNodes(node)

	for i := len(toRemove) - 1; i >= 0; i-- {
		_ = t.graph.RemoveNode(toRemove[i])
	}
	return nil
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
	return t.root
}

// Parent returns the parent of a node, or "" if the node is the root.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Parent(node string) (string, error) {
	if !t.graph.HasNode(node) {
		return "", &NodeNotFoundError{Node: node}
	}

	preds, _ := t.graph.Predecessors(node)
	if len(preds) == 0 {
		return "", nil
	}
	return preds[0], nil
}

// Children returns the children of a node (sorted alphabetically).
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Children(node string) ([]string, error) {
	if !t.graph.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
	}

	children, _ := t.graph.Successors(node)
	sort.Strings(children)
	return children, nil
}

// Siblings returns the siblings of a node (other children of the same parent).
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Siblings(node string) ([]string, error) {
	if !t.graph.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
	}

	parentNode, err := t.Parent(node)
	if err != nil {
		return nil, err
	}
	if parentNode == "" {
		return []string{}, nil
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
	return siblings, nil
}

// IsLeaf returns true if the node has no children.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) IsLeaf(node string) (bool, error) {
	if !t.graph.HasNode(node) {
		return false, &NodeNotFoundError{Node: node}
	}

	children, _ := t.graph.Successors(node)
	return len(children) == 0, nil
}

// IsRoot returns true if the node is the root of the tree.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) IsRoot(node string) (bool, error) {
	if !t.graph.HasNode(node) {
		return false, &NodeNotFoundError{Node: node}
	}

	return node == t.root, nil
}

// Depth returns the depth of a node (distance from root).
//
// Root = 0, its children = 1, grandchildren = 2, etc.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Depth(node string) (int, error) {
	if !t.graph.HasNode(node) {
		return 0, &NodeNotFoundError{Node: node}
	}

	d := 0
	current := node
	for current != t.root {
		preds, _ := t.graph.Predecessors(current)
		current = preds[0]
		d++
	}

	return d, nil
}

// Height returns the height of the tree (maximum depth of any node).
//
// A single-node tree has height 0.
func (t *Tree) Height() int {
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

	return maxDepth
}

// Size returns the total number of nodes in the tree.
func (t *Tree) Size() int {
	return t.graph.Size()
}

// Nodes returns all nodes in the tree (sorted alphabetically).
func (t *Tree) Nodes() []string {
	nodes := t.graph.Nodes()
	sort.Strings(nodes)
	return nodes
}

// Leaves returns all leaf nodes (sorted alphabetically).
func (t *Tree) Leaves() []string {
	var leaves []string
	for _, n := range t.graph.Nodes() {
		children, _ := t.graph.Successors(n)
		if len(children) == 0 {
			leaves = append(leaves, n)
		}
	}
	sort.Strings(leaves)
	return leaves
}

// HasNode returns true if the node exists in the tree.
func (t *Tree) HasNode(node string) bool {
	return t.graph.HasNode(node)
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
	var result []string
	stack := []string{t.root}

	for len(stack) > 0 {
		node := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		result = append(result, node)

		children, _ := t.graph.Successors(node)
		sort.Sort(sort.Reverse(sort.StringSlice(children)))
		stack = append(stack, children...)
	}

	return result
}

// Postorder returns nodes in postorder (children before parent).
//
// Uses a recursive helper. Children visited in sorted order.
func (t *Tree) Postorder() []string {
	var result []string
	t.postorderRecursive(t.root, &result)
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
	var result []string
	queue := []string{t.root}

	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		result = append(result, node)

		children, _ := t.graph.Successors(node)
		sort.Strings(children)
		queue = append(queue, children...)
	}

	return result
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

// PathTo returns the path from the root to the given node.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) PathTo(node string) ([]string, error) {
	if !t.graph.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
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

	return path, nil
}

// LCA returns the lowest common ancestor of nodes a and b.
//
// The LCA is the deepest node that is an ancestor of both a and b.
//
// Returns NodeNotFoundError if a or b doesn't exist.
func (t *Tree) LCA(a, b string) (string, error) {
	if !t.graph.HasNode(a) {
		return "", &NodeNotFoundError{Node: a}
	}
	if !t.graph.HasNode(b) {
		return "", &NodeNotFoundError{Node: b}
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

	return lcaNode, nil
}

// Subtree extracts the subtree rooted at the given node.
//
// Returns a NEW Tree object. The original tree is not modified.
//
// Returns NodeNotFoundError if the node doesn't exist.
func (t *Tree) Subtree(node string) (*Tree, error) {
	if !t.graph.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
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

	return newTree, nil
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
	var lines []string
	t.asciiRecursive(t.root, "", "", &lines)
	return strings.Join(lines, "\n")
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
	return t.graph
}

// String returns a string representation showing root and size.
func (t *Tree) String() string {
	return fmt.Sprintf("Tree(root=%q, size=%d)", t.root, t.Size())
}
