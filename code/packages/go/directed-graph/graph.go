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
//
// # Operations
//
// Every public method is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery. The Cage
// is injected into every callback but has no methods for this package
// (directed-graph declares zero OS capabilities).
//
// From the caller's perspective, the public API is unchanged — every
// method has the same signature as before. Operations are an internal
// implementation detail.
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
//
// # Self-loops
//
// By default, self-loops (edges from a node to itself, like A→A) are
// prohibited because they create trivial cycles, which makes topological
// sorting impossible. However, some use cases genuinely need self-loops —
// for example, modeling state machines where a state can transition to
// itself, or representing "retry" semantics in a workflow graph.
//
// Use NewAllowSelfLoops() to create a graph that permits self-loops.
// The allowSelfLoops flag is checked only in AddEdge; all other methods
// (HasCycle, TopologicalSort, etc.) work correctly regardless of the
// flag's value.
type Graph struct {
	forward        map[string]map[string]bool // node → set of successors
	reverse        map[string]map[string]bool // node → set of predecessors
	allowSelfLoops bool                       // whether A→A edges are permitted
}

// New creates an empty directed graph that prohibits self-loops.
//
// This is the default constructor. If you try to add an edge from a
// node to itself (e.g., g.AddEdge("A", "A")), it will panic.
func New() *Graph {
	return &Graph{
		forward:        make(map[string]map[string]bool),
		reverse:        make(map[string]map[string]bool),
		allowSelfLoops: false,
	}
}

// NewAllowSelfLoops creates an empty directed graph that permits self-loops.
//
// A self-loop is an edge from a node to itself, like A→A. This is useful
// for modeling state machines, retry loops, or any domain where a node
// can reference itself.
//
// Note: a graph with self-loops will have cycles (a self-loop IS a cycle
// of length 1), so TopologicalSort will return a CycleError and HasCycle
// will return true.
func NewAllowSelfLoops() *Graph {
	return &Graph{
		forward:        make(map[string]map[string]bool),
		reverse:        make(map[string]map[string]bool),
		allowSelfLoops: true,
	}
}

// AddNode adds a node to the graph. No-op if the node already exists.
func (g *Graph) AddNode(node string) {
	_, _ = StartNew[struct{}]("directed-graph.AddNode", struct{}{},
		func(_ *Cage, op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("node", node)
			if _, ok := g.forward[node]; !ok {
				g.forward[node] = make(map[string]bool)
				g.reverse[node] = make(map[string]bool)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// AddEdge adds a directed edge from 'from' to 'to'.
// Both nodes are implicitly added if they don't exist.
//
// Self-loop behavior depends on how the graph was created:
//
//   - New()              → self-loops are PROHIBITED (panics on from == to)
//   - NewAllowSelfLoops() → self-loops are ALLOWED
//
// When self-loops are allowed, AddEdge("A", "A") inserts A into both
// the forward and reverse adjacency sets for A. This means:
//   - HasEdge("A", "A") returns true
//   - Successors("A") includes "A"
//   - Predecessors("A") includes "A"
//   - HasCycle() returns true (a self-loop is a cycle of length 1)
func (g *Graph) AddEdge(from, to string) {
	// PanicOnUnexpected lets the self-loop panic propagate to the caller.
	// This preserves the documented panic behavior for programming errors.
	_, _ = StartNew[struct{}]("directed-graph.AddEdge", struct{}{},
		func(_ *Cage, op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("from", from)
			op.AddProperty("to", to)
			if from == to && !g.allowSelfLoops {
				panic(fmt.Sprintf("self-loop not allowed: %q", from))
			}
			if _, ok := g.forward[from]; !ok {
				g.forward[from] = make(map[string]bool)
				g.reverse[from] = make(map[string]bool)
			}
			if _, ok := g.forward[to]; !ok {
				g.forward[to] = make(map[string]bool)
				g.reverse[to] = make(map[string]bool)
			}
			g.forward[from][to] = true
			g.reverse[to][from] = true
			return rf.Generate(true, false, struct{}{})
		}).PanicOnUnexpected().GetResult()
}

// RemoveNode removes a node and all its incident edges.
// Returns an error if the node doesn't exist.
func (g *Graph) RemoveNode(node string) error {
	_, err := StartNew[struct{}]("directed-graph.RemoveNode", struct{}{},
		func(_ *Cage, op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("node", node)
			if _, exists := g.forward[node]; !exists {
				return rf.Fail(struct{}{}, &NodeNotFoundError{Node: node})
			}
			// Remove all edges TO this node.
			for pred := range g.reverse[node] {
				delete(g.forward[pred], node)
			}
			// Remove all edges FROM this node.
			for succ := range g.forward[node] {
				delete(g.reverse[succ], node)
			}
			delete(g.forward, node)
			delete(g.reverse, node)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// RemoveEdge removes the edge from 'from' to 'to'.
// Returns an error if the edge doesn't exist.
func (g *Graph) RemoveEdge(from, to string) error {
	_, err := StartNew[struct{}]("directed-graph.RemoveEdge", struct{}{},
		func(_ *Cage, op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("from", from)
			op.AddProperty("to", to)
			succs, ok := g.forward[from]
			if !ok || !succs[to] {
				return rf.Fail(struct{}{}, &EdgeNotFoundError{From: from, To: to})
			}
			delete(g.forward[from], to)
			delete(g.reverse[to], from)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// HasNode returns true if the node exists in the graph.
func (g *Graph) HasNode(node string) bool {
	result, _ := StartNew[bool]("directed-graph.HasNode", false,
		func(_ *Cage, op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("node", node)
			_, ok := g.forward[node]
			return rf.Generate(true, false, ok)
		}).GetResult()
	return result
}

// HasEdge returns true if there's an edge from 'from' to 'to'.
func (g *Graph) HasEdge(from, to string) bool {
	result, _ := StartNew[bool]("directed-graph.HasEdge", false,
		func(_ *Cage, op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("from", from)
			op.AddProperty("to", to)
			if succs, ok := g.forward[from]; ok {
				return rf.Generate(true, false, succs[to])
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// Nodes returns all nodes in sorted order (deterministic).
func (g *Graph) Nodes() []string {
	result, _ := StartNew[[]string]("directed-graph.Nodes", nil,
		func(_ *Cage, _ *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			nodes := make([]string, 0, len(g.forward))
			for n := range g.forward {
				nodes = append(nodes, n)
			}
			sort.Strings(nodes)
			return rf.Generate(true, false, nodes)
		}).GetResult()
	return result
}

// Edges returns all edges as [from, to] pairs, sorted deterministically.
func (g *Graph) Edges() [][2]string {
	result, _ := StartNew[[][2]string]("directed-graph.Edges", nil,
		func(_ *Cage, _ *Operation[[][2]string], rf *ResultFactory[[][2]string]) *OperationResult[[][2]string] {
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
			return rf.Generate(true, false, edges)
		}).GetResult()
	return result
}

// Predecessors returns the direct parents of a node (nodes with edges TO this node).
func (g *Graph) Predecessors(node string) ([]string, error) {
	return StartNew[[]string]("directed-graph.Predecessors", nil,
		func(_ *Cage, op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			op.AddProperty("node", node)
			preds, ok := g.reverse[node]
			if !ok {
				return rf.Fail(nil, &NodeNotFoundError{Node: node})
			}
			result := make([]string, 0, len(preds))
			for p := range preds {
				result = append(result, p)
			}
			sort.Strings(result)
			return rf.Generate(true, false, result)
		}).GetResult()
}

// Successors returns the direct children of a node (nodes this node has edges TO).
func (g *Graph) Successors(node string) ([]string, error) {
	return StartNew[[]string]("directed-graph.Successors", nil,
		func(_ *Cage, op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			op.AddProperty("node", node)
			succs, ok := g.forward[node]
			if !ok {
				return rf.Fail(nil, &NodeNotFoundError{Node: node})
			}
			result := make([]string, 0, len(succs))
			for s := range succs {
				result = append(result, s)
			}
			sort.Strings(result)
			return rf.Generate(true, false, result)
		}).GetResult()
}

// Size returns the number of nodes in the graph.
func (g *Graph) Size() int {
	result, _ := StartNew[int]("directed-graph.Size", 0,
		func(_ *Cage, _ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(g.forward))
		}).GetResult()
	return result
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
	return StartNew[[]string]("directed-graph.TopologicalSort", nil,
		func(_ *Cage, _ *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			// Compute in-degrees.
			inDegree := make(map[string]int, len(g.forward))
			for node := range g.forward {
				inDegree[node] = len(g.reverse[node])
			}

			// Collect nodes with in-degree 0.
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
				return rf.Fail(nil, &CycleError{})
			}
			return rf.Generate(true, false, result)
		}).GetResult()
}

// HasCycle returns true if the graph contains a cycle.
// Uses DFS with three-color marking (white/gray/black).
func (g *Graph) HasCycle() bool {
	result, _ := StartNew[bool]("directed-graph.HasCycle", false,
		func(_ *Cage, _ *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
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

			// Iterate over nodes in sorted order for determinism.
			nodes := make([]string, 0, len(g.forward))
			for n := range g.forward {
				nodes = append(nodes, n)
			}
			sort.Strings(nodes)

			for _, node := range nodes {
				if color[node] == white {
					if dfs(node) {
						return rf.Generate(true, false, true)
					}
				}
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// TransitiveClosure returns all nodes reachable from the given node
// by following edges forward (downstream).
func (g *Graph) TransitiveClosure(node string) (map[string]bool, error) {
	return StartNew[map[string]bool]("directed-graph.TransitiveClosure", nil,
		func(_ *Cage, op *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
			op.AddProperty("node", node)
			if _, exists := g.forward[node]; !exists {
				return rf.Fail(nil, &NodeNotFoundError{Node: node})
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
			return rf.Generate(true, false, visited)
		}).GetResult()
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
	return StartNew[[][]string]("directed-graph.IndependentGroups", nil,
		func(_ *Cage, _ *Operation[[][]string], rf *ResultFactory[[][]string]) *OperationResult[[][]string] {
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
				return rf.Fail(nil, &CycleError{})
			}
			return rf.Generate(true, false, levels)
		}).GetResult()
}

// AffectedNodes returns the set of nodes affected by changes to the
// given set of nodes. "Affected" means: the changed nodes themselves,
// plus everything that transitively depends on any of them.
//
// This is used by the build tool: if you change logic-gates, the
// affected set includes logic-gates + arithmetic + cpu-simulator + ...
func (g *Graph) AffectedNodes(changed map[string]bool) map[string]bool {
	result, _ := StartNew[map[string]bool]("directed-graph.AffectedNodes", nil,
		func(_ *Cage, _ *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
			affected := make(map[string]bool)
			for node := range changed {
				if _, exists := g.forward[node]; !exists {
					continue
				}
				affected[node] = true
				deps, _ := g.TransitiveDependents(node)
				for dep := range deps {
					affected[dep] = true
				}
			}
			return rf.Generate(true, false, affected)
		}).GetResult()
	return result
}

// AffectedNodesList is a convenience wrapper that returns a sorted slice.
func (g *Graph) AffectedNodesList(changed map[string]bool) []string {
	result, _ := StartNew[[]string]("directed-graph.AffectedNodesList", nil,
		func(_ *Cage, _ *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			affected := g.AffectedNodes(changed)
			nodes := make([]string, 0, len(affected))
			for n := range affected {
				nodes = append(nodes, n)
			}
			sort.Strings(nodes)
			return rf.Generate(true, false, nodes)
		}).GetResult()
	return result
}

// contains checks if a slice contains a value.
func contains(s []string, v string) bool {
	return slices.Contains(s, v)
}
