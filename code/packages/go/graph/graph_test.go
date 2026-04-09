package graph

import (
	"sort"
	"testing"
)

// ============================================================================
// Construction Tests
// ============================================================================

func TestNewGraphIsEmpty(t *testing.T) {
	g := New()
	if g.Size() != 0 {
		t.Errorf("Expected size 0, got %d", g.Size())
	}
	if len(g.Nodes()) != 0 {
		t.Error("Expected no nodes")
	}
	if len(g.Edges()) != 0 {
		t.Error("Expected no edges")
	}
}

func TestNewWithReprAdjacencyList(t *testing.T) {
	g := NewWithRepr(AdjacencyList)
	if g.Size() != 0 {
		t.Errorf("Expected size 0, got %d", g.Size())
	}
}

func TestNewWithReprAdjacencyMatrix(t *testing.T) {
	g := NewWithRepr(AdjacencyMatrix)
	if g.Size() != 0 {
		t.Errorf("Expected size 0, got %d", g.Size())
	}
}

// ============================================================================
// Node Operations Tests
// ============================================================================

func testNodeOperations(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// Add node
	g.AddNode("A")
	if !g.HasNode("A") {
		t.Error("Expected node A to exist")
	}
	if g.Size() != 1 {
		t.Errorf("Expected size 1, got %d", g.Size())
	}

	// Add multiple nodes
	g.AddNode("B")
	g.AddNode("C")
	if g.Size() != 3 {
		t.Errorf("Expected size 3, got %d", g.Size())
	}

	nodes := g.Nodes()
	if len(nodes) != 3 {
		t.Errorf("Expected 3 nodes, got %d", len(nodes))
	}

	// Duplicate node is no-op
	g.AddNode("A")
	if g.Size() != 3 {
		t.Errorf("Expected size 3, got %d", g.Size())
	}

	// Remove node
	if err := g.RemoveNode("A"); err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}
	if g.HasNode("A") {
		t.Error("Expected node A to be removed")
	}
	if g.Size() != 2 {
		t.Errorf("Expected size 2, got %d", g.Size())
	}

	// Remove nonexistent node
	if err := g.RemoveNode("X"); err == nil {
		t.Error("Expected error for nonexistent node")
	}
}

func TestNodeOperationsAdjacencyList(t *testing.T) {
	testNodeOperations(t, AdjacencyList)
}

func TestNodeOperationsAdjacencyMatrix(t *testing.T) {
	testNodeOperations(t, AdjacencyMatrix)
}

// ============================================================================
// Edge Operations Tests
// ============================================================================

func testEdgeOperations(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// Add edge creates nodes
	g.AddEdge("A", "B")
	if !g.HasNode("A") || !g.HasNode("B") {
		t.Error("Expected nodes A and B to exist")
	}

	// Edge exists both directions
	if !g.HasEdge("A", "B") || !g.HasEdge("B", "A") {
		t.Error("Expected edge A-B to exist in both directions")
	}

	// Get edges
	edges := g.Edges()
	if len(edges) != 1 {
		t.Errorf("Expected 1 edge, got %d", len(edges))
	}

	// Add weighted edge
	g.AddEdge("B", "C", 2.5)
	w, err := g.EdgeWeight("B", "C")
	if err != nil || w != 2.5 {
		t.Errorf("Expected weight 2.5, got %v, error %v", w, err)
	}

	// Remove edge
	if err := g.RemoveEdge("A", "B"); err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}
	if g.HasEdge("A", "B") {
		t.Error("Expected edge A-B to be removed")
	}

	// Remove nonexistent edge
	if err := g.RemoveEdge("X", "Y"); err == nil {
		t.Error("Expected error for nonexistent edge")
	}
}

func TestEdgeOperationsAdjacencyList(t *testing.T) {
	testEdgeOperations(t, AdjacencyList)
}

func TestEdgeOperationsAdjacencyMatrix(t *testing.T) {
	testEdgeOperations(t, AdjacencyMatrix)
}

// ============================================================================
// Neighborhood Tests
// ============================================================================

func testNeighborhood(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	g.AddEdge("A", "B", 1.0)
	g.AddEdge("A", "C", 2.0)
	g.AddEdge("A", "D", 3.0)

	// Neighbors
	neighbors, err := g.Neighbors("A")
	if err != nil || len(neighbors) != 3 {
		t.Errorf("Expected 3 neighbors, got %d, error %v", len(neighbors), err)
	}

	// Neighbors weighted
	nw, err := g.NeighborsWeighted("A")
	if err != nil || len(nw) != 3 {
		t.Errorf("Expected 3 neighbors, got %d, error %v", len(nw), err)
	}
	if nw["B"] != 1.0 || nw["C"] != 2.0 || nw["D"] != 3.0 {
		t.Errorf("Unexpected weights: %v", nw)
	}

	// Degree
	degree, err := g.Degree("A")
	if err != nil || degree != 3 {
		t.Errorf("Expected degree 3, got %d, error %v", degree, err)
	}

	degree, err = g.Degree("B")
	if err != nil || degree != 1 {
		t.Errorf("Expected degree 1, got %d, error %v", degree, err)
	}

	// Neighbors of nonexistent node
	_, err = g.Neighbors("X")
	if err == nil {
		t.Error("Expected error for nonexistent node")
	}
}

func TestNeighborhoodAdjacencyList(t *testing.T) {
	testNeighborhood(t, AdjacencyList)
}

func TestNeighborhoodAdjacencyMatrix(t *testing.T) {
	testNeighborhood(t, AdjacencyMatrix)
}

// ============================================================================
// BFS Tests
// ============================================================================

func testBFS(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// Simple path
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	g.AddEdge("C", "D")

	result := BFS(g, "A")
	if len(result) != 4 {
		t.Errorf("Expected 4 nodes, got %d", len(result))
	}
	if result[0] != "A" {
		t.Errorf("Expected start node A, got %s", result[0])
	}

	// Tree with multiple branches
	g2 := NewWithRepr(repr)
	g2.AddEdge("A", "B")
	g2.AddEdge("A", "C")
	g2.AddEdge("B", "D")
	g2.AddEdge("B", "E")

	result2 := BFS(g2, "A")
	if len(result2) != 5 {
		t.Errorf("Expected 5 nodes, got %d", len(result2))
	}

	// Disconnected graph
	g3 := NewWithRepr(repr)
	g3.AddEdge("A", "B")
	g3.AddEdge("C", "D")

	result3 := BFS(g3, "A")
	if len(result3) != 2 {
		t.Errorf("Expected 2 nodes reachable from A, got %d", len(result3))
	}
}

func TestBFSAdjacencyList(t *testing.T) {
	testBFS(t, AdjacencyList)
}

func TestBFSAdjacencyMatrix(t *testing.T) {
	testBFS(t, AdjacencyMatrix)
}

// ============================================================================
// DFS Tests
// ============================================================================

func testDFS(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	g.AddEdge("A", "B")
	g.AddEdge("B", "D")
	g.AddEdge("A", "C")

	result := DFS(g, "A")
	if len(result) != 4 {
		t.Errorf("Expected 4 nodes, got %d", len(result))
	}
	if result[0] != "A" {
		t.Errorf("Expected start node A, got %s", result[0])
	}
}

func TestDFSAdjacencyList(t *testing.T) {
	testDFS(t, AdjacencyList)
}

func TestDFSAdjacencyMatrix(t *testing.T) {
	testDFS(t, AdjacencyMatrix)
}

// ============================================================================
// Shortest Path Tests
// ============================================================================

func testShortestPath(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// Unweighted graph
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	g.AddEdge("C", "D")

	path := ShortestPath(g, "A", "D")
	if len(path) != 4 {
		t.Errorf("Expected path of length 4, got %d: %v", len(path), path)
	}

	// Same start and end
	path2 := ShortestPath(g, "A", "A")
	if len(path2) != 1 || path2[0] != "A" {
		t.Errorf("Expected path [A], got %v", path2)
	}

	// No path
	g.AddNode("X")
	path3 := ShortestPath(g, "A", "X")
	if len(path3) != 0 {
		t.Errorf("Expected no path, got %v", path3)
	}

	// Weighted graph (Dijkstra)
	g2 := NewWithRepr(repr)
	g2.AddEdge("A", "B", 1.0)
	g2.AddEdge("B", "D", 10.0)
	g2.AddEdge("A", "C", 3.0)
	g2.AddEdge("C", "D", 3.0)

	path4 := ShortestPath(g2, "A", "D")
	if len(path4) != 3 {
		t.Errorf("Expected path of length 3, got %d: %v", len(path4), path4)
	}
}

func TestShortestPathAdjacencyList(t *testing.T) {
	testShortestPath(t, AdjacencyList)
}

func TestShortestPathAdjacencyMatrix(t *testing.T) {
	testShortestPath(t, AdjacencyMatrix)
}

// ============================================================================
// Cycle Detection Tests
// ============================================================================

func testHasCycle(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// No cycle (path)
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	if HasCycle(g) {
		t.Error("Expected no cycle in path")
	}

	// Cycle (triangle)
	g.AddEdge("C", "A")
	if !HasCycle(g) {
		t.Error("Expected cycle in triangle")
	}

	// Single node (no cycle)
	g2 := NewWithRepr(repr)
	g2.AddNode("A")
	if HasCycle(g2) {
		t.Error("Expected no cycle in single node")
	}

	// Two nodes connected (no cycle)
	g3 := NewWithRepr(repr)
	g3.AddEdge("A", "B")
	if HasCycle(g3) {
		t.Error("Expected no cycle in two-node graph")
	}
}

func TestHasCycleAdjacencyList(t *testing.T) {
	testHasCycle(t, AdjacencyList)
}

func TestHasCycleAdjacencyMatrix(t *testing.T) {
	testHasCycle(t, AdjacencyMatrix)
}

// ============================================================================
// Connected Components Tests
// ============================================================================

func testConnectedComponents(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// Three components
	g.AddEdge("A", "B")
	g.AddEdge("C", "D")
	g.AddNode("E")

	components := g.ConnectedComponents()
	if len(components) != 3 {
		t.Errorf("Expected 3 components, got %d", len(components))
	}

	// Check sizes
	sizes := make([]int, len(components))
	for i, comp := range components {
		sizes[i] = len(comp)
	}
	sort.Ints(sizes)
	expectedSizes := []int{1, 2, 2}
	for i, s := range sizes {
		if s != expectedSizes[i] {
			t.Errorf("Expected sizes %v, got %v", expectedSizes, sizes)
		}
	}
}

func TestConnectedComponentsAdjacencyList(t *testing.T) {
	testConnectedComponents(t, AdjacencyList)
}

func TestConnectedComponentsAdjacencyMatrix(t *testing.T) {
	testConnectedComponents(t, AdjacencyMatrix)
}

// ============================================================================
// IsConnected Tests
// ============================================================================

func testIsConnected(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// Empty graph (vacuously true)
	if !g.IsConnected() {
		t.Error("Expected empty graph to be connected")
	}

	// Single node
	g.AddNode("A")
	if !g.IsConnected() {
		t.Error("Expected single node to be connected")
	}

	// Two connected nodes
	g.AddEdge("A", "B")
	if !g.IsConnected() {
		t.Error("Expected two-node graph to be connected")
	}

	// Add disconnected node
	g.AddNode("C")
	if g.IsConnected() {
		t.Error("Expected disconnected graph to be not connected")
	}

	// Connect it
	g.AddEdge("B", "C")
	if !g.IsConnected() {
		t.Error("Expected connected graph to be connected")
	}
}

func TestIsConnectedAdjacencyList(t *testing.T) {
	testIsConnected(t, AdjacencyList)
}

func TestIsConnectedAdjacencyMatrix(t *testing.T) {
	testIsConnected(t, AdjacencyMatrix)
}

// ============================================================================
// Minimum Spanning Tree Tests
// ============================================================================

func testMinimumSpanningTree(t *testing.T, repr GraphRepr) {
	g := NewWithRepr(repr)

	// Simple MST
	g.AddEdge("A", "B", 1.0)
	g.AddEdge("B", "C", 2.0)
	g.AddEdge("C", "A", 3.0)

	mst := MinimumSpanningTree(g)
	if mst == nil {
		t.Error("Expected MST, got nil")
	}
	if len(mst) != 2 {
		t.Errorf("Expected 2 edges in MST, got %d", len(mst))
	}

	// Total weight should be 3 (1 + 2)
	totalWeight := 0.0
	for _, edge := range mst {
		totalWeight += edge.Weight
	}
	if totalWeight != 3.0 {
		t.Errorf("Expected total weight 3.0, got %f", totalWeight)
	}

	// Disconnected graph
	g2 := NewWithRepr(repr)
	g2.AddEdge("A", "B")
	g2.AddEdge("C", "D")
	mst2 := MinimumSpanningTree(g2)
	if mst2 != nil {
		t.Error("Expected nil MST for disconnected graph")
	}

	// Single node (should return empty slice)
	g3 := NewWithRepr(repr)
	g3.AddNode("A")
	mst3 := MinimumSpanningTree(g3)
	if mst3 == nil || len(mst3) != 0 {
		t.Errorf("Expected empty MST for single node, got %v", mst3)
	}
}

func TestMinimumSpanningTreeAdjacencyList(t *testing.T) {
	testMinimumSpanningTree(t, AdjacencyList)
}

func TestMinimumSpanningTreeAdjacencyMatrix(t *testing.T) {
	testMinimumSpanningTree(t, AdjacencyMatrix)
}

// ============================================================================
// Edge Weight Tests
// ============================================================================

func TestEdgeWeight(t *testing.T) {
	g := NewWithRepr(AdjacencyList)

	g.AddEdge("A", "B", 2.5)
	w, err := g.EdgeWeight("A", "B")
	if err != nil || w != 2.5 {
		t.Errorf("Expected weight 2.5, got %f, error %v", w, err)
	}

	// Nonexistent edge
	_, err = g.EdgeWeight("A", "C")
	if err == nil {
		t.Error("Expected error for nonexistent edge")
	}

	// Symmetric (undirected)
	w, err = g.EdgeWeight("B", "A")
	if err != nil || w != 2.5 {
		t.Errorf("Expected weight 2.5 (undirected), got %f", w)
	}
}

// ============================================================================
// Helpers
// ============================================================================

func contains(s []string, v string) bool {
	for _, x := range s {
		if x == v {
			return true
		}
	}
	return false
}
