// Package directedgraph provides a directed graph data structure with
// algorithms for topological sorting, cycle detection, and parallel
// execution level computation.
//
// # What is a directed graph?
//
// A directed graph (or "digraph") is a set of nodes connected by edges,
// where each edge has a direction — it goes FROM one node TO another.
// Think of it like a one-way street map: you can travel from A to B,
// but that doesn't mean you can travel from B to A.
//
// In this build system, nodes are packages and edges are dependencies:
// if package A depends on package B, there's an edge from B to A
// (B must be built before A).
//
// # Why a directed graph?
//
// The dependency relationships between packages form a DAG (Directed
// Acyclic Graph). A DAG has no cycles — you can't have A depend on B
// depend on C depend on A. The key algorithms on a DAG are:
//
//   - Topological sort: order nodes so every dependency comes before
//     the things that depend on it. This gives you a valid build order.
//
//   - Independent groups: partition nodes into "levels" where everything
//     at the same level can run in parallel. Level 0 has no dependencies.
//     Level 1 depends only on level 0. And so on.
//
//   - Affected nodes: given a set of changed nodes, find everything that
//     transitively depends on them. These are the packages that need
//     rebuilding when something changes.
package directedgraph

import (
	"fmt"
	"slices"
	"sort"
)

// Graph is a directed graph with string-typed nodes.
//
// It stores both forward edges (node → its successors) and reverse
// edges (node → its predecessors) for efficient lookups in both
// directions. This doubles memory usage but makes transitive_dependents
// queries O(V+E) instead of requiring a full graph reversal.
type Graph struct {
	forward map[string]map[string]bool // node → set of successors
	reverse map[string]map[string]bool // node → set of predecessors
}

// New creates an empty directed graph.
func New() *Graph {
	return &Graph{
		forward: make(map[string]map[string]bool),
		reverse: make(map[string]map[string]bool),
	}
}

// AddNode adds a node to the graph. No-op if the node already exists.
func (g *Graph) AddNode(node string) {
	if _, ok := g.forward[node]; !ok {
		g.forward[node] = make(map[string]bool)
		g.reverse[node] = make(map[string]bool)
	}
}

// AddEdge adds a directed edge from 'from' to 'to'.
// Both nodes are implicitly added if they don't exist.
// Panics on self-loops (from == to).
func (g *Graph) AddEdge(from, to string) {
	if from == to {
		panic(fmt.Sprintf("self-loop not allowed: %q", from))
	}
	g.AddNode(from)
	g.AddNode(to)
	g.forward[from][to] = true
	g.reverse[to][from] = true
}

// RemoveNode removes a node and all its incident edges.
// Returns an error if the node doesn't exist.
func (g *Graph) RemoveNode(node string) error {
	if !g.HasNode(node) {
		return &NodeNotFoundError{Node: node}
	}
	// Remove all edges TO this node
	for pred := range g.reverse[node] {
		delete(g.forward[pred], node)
	}
	// Remove all edges FROM this node
	for succ := range g.forward[node] {
		delete(g.reverse[succ], node)
	}
	delete(g.forward, node)
	delete(g.reverse, node)
	return nil
}

// RemoveEdge removes the edge from 'from' to 'to'.
// Returns an error if the edge doesn't exist.
func (g *Graph) RemoveEdge(from, to string) error {
	if !g.HasEdge(from, to) {
		return &EdgeNotFoundError{From: from, To: to}
	}
	delete(g.forward[from], to)
	delete(g.reverse[to], from)
	return nil
}

// HasNode returns true if the node exists in the graph.
func (g *Graph) HasNode(node string) bool {
	_, ok := g.forward[node]
	return ok
}

// HasEdge returns true if there's an edge from 'from' to 'to'.
func (g *Graph) HasEdge(from, to string) bool {
	if succs, ok := g.forward[from]; ok {
		return succs[to]
	}
	return false
}

// Nodes returns all nodes in sorted order (deterministic).
func (g *Graph) Nodes() []string {
	nodes := make([]string, 0, len(g.forward))
	for n := range g.forward {
		nodes = append(nodes, n)
	}
	sort.Strings(nodes)
	return nodes
}

// Edges returns all edges as [from, to] pairs, sorted deterministically.
func (g *Graph) Edges() [][2]string {
	var edges [][2]string
	for from, succs := range g.forward {
		for to := range succs {
			edges = append(edges, [2]string{from, to})
		}
	}
	sort.Slice(edges, func(i, j int) bool {
		if edges[i][0] != edges[j][0] {
			return edges[i][0] < edges[j][0]
		}
		return edges[i][1] < edges[j][1]
	})
	return edges
}

// Predecessors returns the direct parents of a node (nodes with edges TO this node).
func (g *Graph) Predecessors(node string) ([]string, error) {
	preds, ok := g.reverse[node]
	if !ok {
		return nil, &NodeNotFoundError{Node: node}
	}
	result := make([]string, 0, len(preds))
	for p := range preds {
		result = append(result, p)
	}
	sort.Strings(result)
	return result, nil
}

// Successors returns the direct children of a node (nodes this node has edges TO).
func (g *Graph) Successors(node string) ([]string, error) {
	succs, ok := g.forward[node]
	if !ok {
		return nil, &NodeNotFoundError{Node: node}
	}
	result := make([]string, 0, len(succs))
	for s := range succs {
		result = append(result, s)
	}
	sort.Strings(result)
	return result, nil
}

// Size returns the number of nodes in the graph.
func (g *Graph) Size() int {
	return len(g.forward)
}

// TopologicalSort returns nodes in topological order using Kahn's algorithm.
//
// Kahn's algorithm works by repeatedly removing nodes with no incoming edges:
//  1. Find all nodes with in-degree 0 (no predecessors)
//  2. Remove them from the graph (conceptually), add to result
//  3. Their successors may now have in-degree 0 — repeat
//  4. If all nodes are removed, we have a valid ordering
//  5. If some nodes remain, there's a cycle
//
// Returns a CycleError if the graph contains a cycle.
func (g *Graph) TopologicalSort() ([]string, error) {
	// Compute in-degrees
	inDegree := make(map[string]int, len(g.forward))
	for node := range g.forward {
		inDegree[node] = len(g.reverse[node])
	}

	// Collect nodes with in-degree 0
	var queue []string
	for node, deg := range inDegree {
		if deg == 0 {
			queue = append(queue, node)
		}
	}
	sort.Strings(queue) // deterministic

	var result []string
	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		result = append(result, node)

		succs := make([]string, 0)
		for s := range g.forward[node] {
			succs = append(succs, s)
		}
		sort.Strings(succs)

		for _, succ := range succs {
			inDegree[succ]--
			if inDegree[succ] == 0 {
				queue = append(queue, succ)
				sort.Strings(queue)
			}
		}
	}

	if len(result) != len(g.forward) {
		return nil, &CycleError{}
	}
	return result, nil
}

// HasCycle returns true if the graph contains a cycle.
// Uses DFS with three-color marking (white/gray/black).
func (g *Graph) HasCycle() bool {
	const (
		white = 0 // unvisited
		gray  = 1 // in current DFS path
		black = 2 // fully processed
	)
	color := make(map[string]int, len(g.forward))

	var dfs func(string) bool
	dfs = func(node string) bool {
		color[node] = gray
		for succ := range g.forward[node] {
			if color[succ] == gray {
				return true // back edge = cycle
			}
			if color[succ] == white {
				if dfs(succ) {
					return true
				}
			}
		}
		color[node] = black
		return false
	}

	nodes := g.Nodes()
	for _, node := range nodes {
		if color[node] == white {
			if dfs(node) {
				return true
			}
		}
	}
	return false
}

// TransitiveClosure returns all nodes reachable from the given node
// by following edges forward (downstream).
func (g *Graph) TransitiveClosure(node string) (map[string]bool, error) {
	if !g.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
	}
	visited := make(map[string]bool)
	queue := []string{node}
	for len(queue) > 0 {
		curr := queue[0]
		queue = queue[1:]
		for succ := range g.forward[curr] {
			if !visited[succ] {
				visited[succ] = true
				queue = append(queue, succ)
			}
		}
	}
	return visited, nil
}

// TransitiveDependents returns all nodes that transitively depend on
// the given node — i.e., all nodes reachable by following FORWARD edges.
//
// Edge convention: edges go FROM dependency TO dependent.
// So logic-gates → arithmetic means "arithmetic depends on logic-gates".
//
// If "logic-gates" changes, its transitive dependents are everything
// that directly or indirectly depends on it: arithmetic, cpu-simulator,
// arm-simulator, etc. These are found by following forward edges.
//
// Note: TransitiveClosure and TransitiveDependents both follow forward
// edges. They are the same operation. TransitiveDependents exists as a
// named alias to make build-system code more readable.
func (g *Graph) TransitiveDependents(node string) (map[string]bool, error) {
	return g.TransitiveClosure(node)
}

// IndependentGroups partitions nodes into levels by topological depth.
// Nodes at the same level have no dependency on each other and can
// run in parallel.
//
// This is the key method for the build system's parallel execution.
//
// Example for a diamond graph (A→B, A→C, B→D, C→D):
//
//	Level 0: [A]      — no dependencies
//	Level 1: [B, C]   — depend only on A, can run in parallel
//	Level 2: [D]      — depends on B and C
//
// Returns a CycleError if the graph contains a cycle.
func (g *Graph) IndependentGroups() ([][]string, error) {
	inDegree := make(map[string]int, len(g.forward))
	for node := range g.forward {
		inDegree[node] = len(g.reverse[node])
	}

	var queue []string
	for node, deg := range inDegree {
		if deg == 0 {
			queue = append(queue, node)
		}
	}
	sort.Strings(queue)

	var levels [][]string
	processed := 0

	for len(queue) > 0 {
		level := make([]string, len(queue))
		copy(level, queue)
		sort.Strings(level)
		levels = append(levels, level)
		processed += len(level)

		var nextQueue []string
		for _, node := range queue {
			for succ := range g.forward[node] {
				inDegree[succ]--
				if inDegree[succ] == 0 {
					nextQueue = append(nextQueue, succ)
				}
			}
		}
		sort.Strings(nextQueue)
		queue = nextQueue
	}

	if processed != len(g.forward) {
		return nil, &CycleError{}
	}
	return levels, nil
}

// AffectedNodes returns the set of nodes affected by changes to the
// given set of nodes. "Affected" means: the changed nodes themselves,
// plus everything that transitively depends on any of them.
//
// This is used by the build tool: if you change logic-gates, the
// affected set includes logic-gates + arithmetic + cpu-simulator + ...
func (g *Graph) AffectedNodes(changed map[string]bool) map[string]bool {
	affected := make(map[string]bool)
	for node := range changed {
		if !g.HasNode(node) {
			continue
		}
		affected[node] = true
		deps, _ := g.TransitiveDependents(node)
		for dep := range deps {
			affected[dep] = true
		}
	}
	return affected
}

// AffectedNodesList is a convenience wrapper that returns a sorted slice.
func (g *Graph) AffectedNodesList(changed map[string]bool) []string {
	affected := g.AffectedNodes(changed)
	result := make([]string, 0, len(affected))
	for n := range affected {
		result = append(result, n)
	}
	sort.Strings(result)
	return result
}

// NodesList is a convenience to check if a slice contains a node.
func contains(s []string, v string) bool {
	return slices.Contains(s, v)
}
