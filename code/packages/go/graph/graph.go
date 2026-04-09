// Package graph provides an undirected graph data structure with weighted edges,
// supporting both adjacency list and adjacency matrix representations, plus
// comprehensive graph algorithms.
//
// # What is an undirected graph?
//
// An undirected graph G = (V, E) is a pair of sets:
//   - V (vertices/nodes): any hashable type
//   - E (edges): unordered pairs {u, v} with optional weights (default 1.0)
//
// Since edges are unordered, {u,v} == {v,u}. Think of it like a two-way street
// map: if you can travel from A to B, you can also travel from B to A.
//
// Real-world uses:
//   - Social networks (friendships are mutual)
//   - Road networks (two-way streets)
//   - Computer networks (bidirectional connections)
//   - Game maps (movement between adjacent cells both ways)
//
// # Two Representations
//
// ADJACENCY_LIST (default):
//   - Space: O(V + E) — only stores existing edges
//   - Edge lookup: O(degree(u)) — scan neighbor map
//   - Best for SPARSE graphs (most real-world graphs)
//   - map[string]map[string]float64 stores weights
//
// ADJACENCY_MATRIX:
//   - Space: O(V²) — allocates a slot for every possible edge
//   - Edge lookup: O(1) — single array read
//   - Best for DENSE graphs or O(1) edge lookup needed
//   - [][]float64 where 0.0 means no edge
//
// All algorithms work unchanged on either representation because they only
// call the Graph's public API.
//
// # Features
//
//   - Two representations: adjacency list (default) and adjacency matrix
//   - Weighted edges: optional weights on edges (default 1.0)
//   - Core operations: add/remove nodes/edges, neighbor queries, degree
//   - Algorithms: BFS, DFS, shortest path, cycle detection, connected components,
//     minimum spanning tree, graph connectivity
//
package graph

import (
	"container/heap"
	"fmt"
	"math"
	"sort"
)

// GraphRepr specifies the internal storage representation.
type GraphRepr int

const (
	AdjacencyList GraphRepr = iota
	AdjacencyMatrix
)

// Graph represents an undirected weighted graph.
// Choose the representation at construction time. Both forms expose the
// identical public API.
type Graph struct {
	repr     GraphRepr
	// For adjacency list:
	adjacency map[string]map[string]float64
	// For adjacency matrix:
	nodeList []string
	nodeIdx  map[string]int
	matrix   [][]float64
}

// New creates and returns a new empty graph with adjacency list representation.
func New() *Graph {
	return NewWithRepr(AdjacencyList)
}

// NewWithRepr creates a new empty graph with the specified representation.
func NewWithRepr(repr GraphRepr) *Graph {
	if repr == AdjacencyMatrix {
		return &Graph{
			repr:     repr,
			nodeList: []string{},
			nodeIdx:  make(map[string]int),
			matrix:   [][]float64{},
		}
	}
	return &Graph{
		repr:      AdjacencyList,
		adjacency: make(map[string]map[string]float64),
	}
}

// AddNode adds a node to the graph. If the node already exists, this is a no-op.
func (g *Graph) AddNode(node string) {
	if g.repr == AdjacencyMatrix {
		if _, exists := g.nodeIdx[node]; !exists {
			idx := len(g.nodeList)
			g.nodeList = append(g.nodeList, node)
			g.nodeIdx[node] = idx
			// Add new row and column of zeros.
			for i := range g.matrix {
				g.matrix[i] = append(g.matrix[i], 0.0)
			}
			g.matrix = append(g.matrix, make([]float64, idx+1))
		}
	} else {
		if _, exists := g.adjacency[node]; !exists {
			g.adjacency[node] = make(map[string]float64)
		}
	}
}

// RemoveNode removes a node and all its edges from the graph.
// Returns an error if the node doesn't exist.
func (g *Graph) RemoveNode(node string) error {
	if !g.HasNode(node) {
		return fmt.Errorf("node not found: %s", node)
	}

	if g.repr == AdjacencyMatrix {
		idx := g.nodeIdx[node]
		delete(g.nodeIdx, node)
		g.nodeList = append(g.nodeList[:idx], g.nodeList[idx+1:]...)
		// Update indices for nodes that shifted down.
		for i := idx; i < len(g.nodeList); i++ {
			g.nodeIdx[g.nodeList[i]] = i
		}
		// Remove the row.
		g.matrix = append(g.matrix[:idx], g.matrix[idx+1:]...)
		// Remove the column from every remaining row.
		for i := range g.matrix {
			g.matrix[i] = append(g.matrix[i][:idx], g.matrix[i][idx+1:]...)
		}
	} else {
		// Remove all edges connected to this node
		for neighbor := range g.adjacency[node] {
			delete(g.adjacency[neighbor], node)
		}
		// Remove the node itself
		delete(g.adjacency, node)
	}
	return nil
}

// HasNode returns true if the node exists in the graph.
func (g *Graph) HasNode(node string) bool {
	if g.repr == AdjacencyMatrix {
		_, exists := g.nodeIdx[node]
		return exists
	}
	_, exists := g.adjacency[node]
	return exists
}

// Nodes returns all nodes in the graph, sorted alphabetically.
func (g *Graph) Nodes() []string {
	if g.repr == AdjacencyMatrix {
		nodes := make([]string, len(g.nodeList))
		copy(nodes, g.nodeList)
		sort.Strings(nodes)
		return nodes
	}
	nodes := make([]string, 0, len(g.adjacency))
	for node := range g.adjacency {
		nodes = append(nodes, node)
	}
	sort.Strings(nodes)
	return nodes
}

// Size returns the number of nodes in the graph.
func (g *Graph) Size() int {
	if g.repr == AdjacencyMatrix {
		return len(g.nodeList)
	}
	return len(g.adjacency)
}

// AddEdge adds an undirected edge between u and v with the given weight.
// Both nodes are added automatically if they do not already exist.
// If the edge already exists its weight is updated.
func (g *Graph) AddEdge(u, v string, weight ...float64) error {
	w := 1.0
	if len(weight) > 0 {
		w = weight[0]
	}

	g.AddNode(u)
	g.AddNode(v)

	if g.repr == AdjacencyMatrix {
		i, j := g.nodeIdx[u], g.nodeIdx[v]
		g.matrix[i][j] = w
		g.matrix[j][i] = w
	} else {
		g.adjacency[u][v] = w
		g.adjacency[v][u] = w
	}
	return nil
}

// RemoveEdge removes the edge between u and v.
// Returns an error if either node or the edge does not exist.
func (g *Graph) RemoveEdge(u, v string) error {
	if !g.HasEdge(u, v) {
		return fmt.Errorf("edge not found: %s -- %s", u, v)
	}

	if g.repr == AdjacencyMatrix {
		i, j := g.nodeIdx[u], g.nodeIdx[v]
		g.matrix[i][j] = 0.0
		g.matrix[j][i] = 0.0
	} else {
		delete(g.adjacency[u], v)
		delete(g.adjacency[v], u)
	}
	return nil
}

// HasEdge returns true if an edge exists between u and v.
func (g *Graph) HasEdge(u, v string) bool {
	if g.repr == AdjacencyMatrix {
		if ui, exists := g.nodeIdx[u]; exists {
			if vi, exists := g.nodeIdx[v]; exists {
				return g.matrix[ui][vi] != 0.0
			}
		}
		return false
	}
	if neighbors, exists := g.adjacency[u]; exists {
		_, hasEdge := neighbors[v]
		return hasEdge
	}
	return false
}

// Edge represents an undirected edge with endpoints and weight.
type Edge struct {
	U, V   string
	Weight float64
}

// Edges returns all edges as a slice of Edge structs.
// Each undirected edge appears exactly once.
func (g *Graph) Edges() []Edge {
	var edges []Edge

	if g.repr == AdjacencyMatrix {
		n := len(g.nodeList)
		for i := 0; i < n; i++ {
			for j := i + 1; j < n; j++ {
				w := g.matrix[i][j]
				if w != 0.0 {
					edges = append(edges, Edge{g.nodeList[i], g.nodeList[j], w})
				}
			}
		}
	} else {
		seen := make(map[[2]string]bool)
		for u, neighbors := range g.adjacency {
			for v, w := range neighbors {
				// Canonical ordering
				var a, b string
				if u <= v {
					a, b = u, v
				} else {
					a, b = v, u
				}
				if !seen[[2]string{a, b}] {
					edges = append(edges, Edge{a, b, w})
					seen[[2]string{a, b}] = true
				}
			}
		}
	}

	// Sort edges for consistent output
	sort.Slice(edges, func(i, j int) bool {
		if edges[i].U != edges[j].U {
			return edges[i].U < edges[j].U
		}
		return edges[i].V < edges[j].V
	})

	return edges
}

// EdgeWeight returns the weight of edge (u, v).
// Returns an error if the edge does not exist.
func (g *Graph) EdgeWeight(u, v string) (float64, error) {
	if !g.HasEdge(u, v) {
		return 0, fmt.Errorf("edge not found: %s -- %s", u, v)
	}

	if g.repr == AdjacencyMatrix {
		i, j := g.nodeIdx[u], g.nodeIdx[v]
		return g.matrix[i][j], nil
	}
	return g.adjacency[u][v], nil
}

// Neighbors returns all neighbours of node as a sorted slice.
// Returns an error if the node does not exist.
func (g *Graph) Neighbors(node string) ([]string, error) {
	if !g.HasNode(node) {
		return nil, fmt.Errorf("node not found: %s", node)
	}

	var neighbors []string

	if g.repr == AdjacencyMatrix {
		idx := g.nodeIdx[node]
		for j, w := range g.matrix[idx] {
			if w != 0.0 {
				neighbors = append(neighbors, g.nodeList[j])
			}
		}
	} else {
		for neighbor := range g.adjacency[node] {
			neighbors = append(neighbors, neighbor)
		}
	}

	sort.Strings(neighbors)
	return neighbors, nil
}

// NeighborsWeighted returns {neighbour: weight} for all neighbours of node.
// Returns an error if the node does not exist.
func (g *Graph) NeighborsWeighted(node string) (map[string]float64, error) {
	if !g.HasNode(node) {
		return nil, fmt.Errorf("node not found: %s", node)
	}

	result := make(map[string]float64)

	if g.repr == AdjacencyMatrix {
		idx := g.nodeIdx[node]
		for j, w := range g.matrix[idx] {
			if w != 0.0 {
				result[g.nodeList[j]] = w
			}
		}
	} else {
		for neighbor, weight := range g.adjacency[node] {
			result[neighbor] = weight
		}
	}

	return result, nil
}

// Degree returns the degree of node (number of incident edges).
// Returns an error if the node does not exist.
func (g *Graph) Degree(node string) (int, error) {
	neighbors, err := g.Neighbors(node)
	if err != nil {
		return 0, err
	}
	return len(neighbors), nil
}

// IsConnected returns true if every node can reach every other node.
// An empty graph is vacuously connected (true).
func (g *Graph) IsConnected() bool {
	if g.Size() == 0 {
		return true
	}
	start := g.Nodes()[0]
	reachable := BFS(g, start)
	return len(reachable) == g.Size()
}

// ConnectedComponents returns a list of connected components, each as a slice of nodes.
func (g *Graph) ConnectedComponents() [][]string {
	var components [][]string
	unvisited := make(map[string]bool)
	for _, node := range g.Nodes() {
		unvisited[node] = true
	}

	for len(unvisited) > 0 {
		var start string
		for node := range unvisited {
			start = node
			break
		}
		component := BFS(g, start)
		components = append(components, component)
		for _, node := range component {
			delete(unvisited, node)
		}
	}

	return components
}

// ============================================================================
// Algorithms (pure functions)
// ============================================================================
// All functions here are pure — they take a Graph as input and return a result.
// They never mutate the graph. They work identically on both ADJACENCY_LIST and
// ADJACENCY_MATRIX graphs because they only call the Graph's public API.

// BFS returns nodes reachable from start in breadth-first order.
// Nodes not reachable from start (in a disconnected graph) are excluded.
// Time: O(V + E). Space: O(V) for the visited set and queue.
func BFS(g *Graph, start string) []string {
	visited := make(map[string]bool)
	queue := []string{start}
	var result []string

	visited[start] = true

	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		result = append(result, node)

		neighbors, _ := g.Neighbors(node)
		for _, neighbor := range neighbors {
			if !visited[neighbor] {
				visited[neighbor] = true
				queue = append(queue, neighbor)
			}
		}
	}

	return result
}

// DFS returns nodes reachable from start in depth-first order.
// Nodes not reachable from start (in a disconnected graph) are excluded.
// Time: O(V + E). Space: O(V) for the visited set and stack.
func DFS(g *Graph, start string) []string {
	visited := make(map[string]bool)
	stack := []string{start}
	var result []string

	for len(stack) > 0 {
		node := stack[len(stack)-1]
		stack = stack[:len(stack)-1]

		if visited[node] {
			continue
		}

		visited[node] = true
		result = append(result, node)

		neighbors, _ := g.Neighbors(node)
		// Reverse sort for consistent ordering
		for i := len(neighbors) - 1; i >= 0; i-- {
			if !visited[neighbors[i]] {
				stack = append(stack, neighbors[i])
			}
		}
	}

	return result
}

// ShortestPath returns the shortest (lowest-weight) path from start to end.
// Returns nil if no path exists.
// For unweighted graphs (all weights 1.0) uses BFS — O(V + E).
// For weighted graphs uses Dijkstra's algorithm — O((V + E) log V).
func ShortestPath(g *Graph, start, end string) []string {
	if start == end {
		if g.HasNode(start) {
			return []string{start}
		}
		return nil
	}

	// Check if all weights are 1.0
	allUnit := true
	for _, edge := range g.Edges() {
		if edge.Weight != 1.0 {
			allUnit = false
			break
		}
	}

	if allUnit {
		return shortestPathBFS(g, start, end)
	}
	return shortestPathDijkstra(g, start, end)
}

func shortestPathBFS(g *Graph, start, end string) []string {
	parent := make(map[string]*string)
	parentStart := (*string)(nil)
	parent[start] = parentStart
	queue := []string{start}

	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]

		if node == end {
			break
		}

		neighbors, _ := g.Neighbors(node)
		for _, neighbor := range neighbors {
			if _, exists := parent[neighbor]; !exists {
				p := node
				parent[neighbor] = &p
				queue = append(queue, neighbor)
			}
		}
	}

	if _, exists := parent[end]; !exists {
		return nil
	}

	var path []string
	cur := end
	for {
		path = append(path, cur)
		p := parent[cur]
		if p == nil {
			break
		}
		cur = *p
	}

	// Reverse path
	for i, j := 0, len(path)-1; i < j; i, j = i+1, j-1 {
		path[i], path[j] = path[j], path[i]
	}

	return path
}

func shortestPathDijkstra(g *Graph, start, end string) []string {
	dist := make(map[string]float64)
	parent := make(map[string]*string)

	for _, node := range g.Nodes() {
		dist[node] = math.Inf(1)
	}
	dist[start] = 0

	pq := &priorityQueue{}
	heap.Init(pq)
	heap.Push(pq, &pqItem{node: start, priority: 0})

	for pq.Len() > 0 {
		item := heap.Pop(pq).(*pqItem)
		d, node := item.priority, item.node

		if d > dist[node] {
			continue
		}

		if node == end {
			break
		}

		neighbors, _ := g.NeighborsWeighted(node)
		for neighbor, weight := range neighbors {
			newDist := dist[node] + weight
			if newDist < dist[neighbor] {
				dist[neighbor] = newDist
				p := node
				parent[neighbor] = &p
				heap.Push(pq, &pqItem{node: neighbor, priority: newDist})
			}
		}
	}

	if math.IsInf(dist[end], 1) {
		return nil
	}

	var path []string
	cur := end
	for {
		path = append(path, cur)
		p, exists := parent[cur]
		if !exists {
			break
		}
		cur = *p
	}

	// Reverse path
	for i, j := 0, len(path)-1; i < j; i, j = i+1, j-1 {
		path[i], path[j] = path[j], path[i]
	}

	return path
}

// HasCycle returns true if the graph contains any cycle.
// Uses iterative DFS.
// Time: O(V + E).
func HasCycle(g *Graph) bool {
	visited := make(map[string]bool)

	for _, start := range g.Nodes() {
		if visited[start] {
			continue
		}

		stack := []struct {
			node   string
			parent *string
		}{{node: start, parent: nil}}

		for len(stack) > 0 {
			item := stack[len(stack)-1]
			stack = stack[:len(stack)-1]

			node, par := item.node, item.parent

			if visited[node] {
				continue
			}

			visited[node] = true
			neighbors, _ := g.Neighbors(node)

			for _, neighbor := range neighbors {
				if !visited[neighbor] {
					stack = append(stack, struct {
						node   string
						parent *string
					}{node: neighbor, parent: &node})
				} else if par == nil || neighbor != *par {
					// Back edge: visited neighbor that isn't our parent → cycle
					return true
				}
			}
		}
	}

	return false
}

// MinimumSpanningTree returns the MST as a slice of edges.
// Returns nil if the graph is not connected (has more than one node).
// Uses Kruskal's algorithm with Union-Find.
// Time: O(E log E) for sorting + O(E · α(V)) for Union-Find.
func MinimumSpanningTree(g *Graph) []Edge {
	nodes := g.Nodes()
	if len(nodes) == 0 {
		return []Edge{} // Empty graph
	}

	if len(nodes) == 1 {
		return []Edge{} // Single node
	}

	// Sort edges by weight
	edges := g.Edges()
	sort.Slice(edges, func(i, j int) bool {
		return edges[i].Weight < edges[j].Weight
	})

	uf := newUnionFind(nodes)
	var mst []Edge

	for _, e := range edges {
		if uf.find(e.U) != uf.find(e.V) {
			uf.union(e.U, e.V)
			mst = append(mst, e)
			if len(mst) == len(nodes)-1 {
				break
			}
		}
	}

	if len(mst) < len(nodes)-1 {
		return nil // Not connected
	}

	return mst
}

// ============================================================================
// Union-Find (helper for Kruskal's algorithm)
// ============================================================================

type unionFind struct {
	parent map[string]string
	rank   map[string]int
}

func newUnionFind(nodes []string) *unionFind {
	uf := &unionFind{
		parent: make(map[string]string),
		rank:   make(map[string]int),
	}
	for _, n := range nodes {
		uf.parent[n] = n
		uf.rank[n] = 0
	}
	return uf
}

func (uf *unionFind) find(x string) string {
	if uf.parent[x] != x {
		uf.parent[x] = uf.find(uf.parent[x]) // path compression
	}
	return uf.parent[x]
}

func (uf *unionFind) union(a, b string) {
	ra, rb := uf.find(a), uf.find(b)
	if ra == rb {
		return
	}
	// Attach the shorter tree under the taller tree
	if uf.rank[ra] < uf.rank[rb] {
		ra, rb = rb, ra
	}
	uf.parent[rb] = ra
	if uf.rank[ra] == uf.rank[rb] {
		uf.rank[ra]++
	}
}

// ============================================================================
// Priority queue for Dijkstra's algorithm
// ============================================================================

type pqItem struct {
	node     string
	priority float64
	index    int
}

type priorityQueue []*pqItem

func (pq priorityQueue) Len() int           { return len(pq) }
func (pq priorityQueue) Less(i, j int) bool { return pq[i].priority < pq[j].priority }
func (pq priorityQueue) Swap(i, j int) {
	pq[i], pq[j] = pq[j], pq[i]
	pq[i].index = i
	pq[j].index = j
}

func (pq *priorityQueue) Push(x any) {
	item := x.(*pqItem)
	item.index = len(*pq)
	*pq = append(*pq, item)
}

func (pq *priorityQueue) Pop() any {
	old := *pq
	n := len(old)
	item := old[n-1]
	old[n-1] = nil
	item.index = -1
	*pq = old[0 : n-1]
	return item
}
