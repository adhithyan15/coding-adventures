package directedgraph

import (
	"errors"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Empty graph tests
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledEmptyGraphNodes(t *testing.T) {
	lg := NewLabeledGraph()
	if len(lg.Nodes()) != 0 {
		t.Errorf("expected 0 nodes, got %d", len(lg.Nodes()))
	}
}

func TestLabeledEmptyGraphEdges(t *testing.T) {
	lg := NewLabeledGraph()
	if len(lg.Edges()) != 0 {
		t.Errorf("expected 0 edges, got %d", len(lg.Edges()))
	}
}

func TestLabeledEmptyGraphSize(t *testing.T) {
	lg := NewLabeledGraph()
	if lg.Size() != 0 {
		t.Errorf("expected size 0, got %d", lg.Size())
	}
}

func TestLabeledEmptyGraphTopoSort(t *testing.T) {
	lg := NewLabeledGraph()
	result, err := lg.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	if len(result) != 0 {
		t.Errorf("expected empty topo sort, got %v", result)
	}
}

func TestLabeledEmptyGraphHasCycle(t *testing.T) {
	lg := NewLabeledGraph()
	if lg.HasCycle() {
		t.Error("empty graph should not have a cycle")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Node operations
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledAddNode(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddNode("A")
	if !lg.HasNode("A") {
		t.Error("should have node A")
	}
	if lg.Size() != 1 {
		t.Errorf("expected size 1, got %d", lg.Size())
	}
}

func TestLabeledAddNodeIdempotent(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddNode("A")
	lg.AddNode("A")
	if lg.Size() != 1 {
		t.Errorf("duplicate add should be no-op, got size %d", lg.Size())
	}
}

func TestLabeledRemoveNode(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddNode("A")
	err := lg.RemoveNode("A")
	if err != nil {
		t.Fatal(err)
	}
	if lg.HasNode("A") {
		t.Error("should not have node A after removal")
	}
}

func TestLabeledRemoveNodeNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	err := lg.RemoveNode("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

func TestLabeledRemoveNodeCleansLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("B", "C", "test")
	_ = lg.RemoveNode("B")
	if lg.HasEdge("A", "B") {
		t.Error("edge A→B should be removed with node B")
	}
	if lg.HasEdge("B", "C") {
		t.Error("edge B→C should be removed with node B")
	}
	// Labels should be cleaned up too
	labels := lg.Labels("A", "B")
	if len(labels) != 0 {
		t.Errorf("labels for A→B should be empty, got %v", labels)
	}
}

func TestLabeledNodesReturnsSorted(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddNode("C")
	lg.AddNode("A")
	lg.AddNode("B")
	nodes := lg.Nodes()
	if nodes[0] != "A" || nodes[1] != "B" || nodes[2] != "C" {
		t.Errorf("expected [A, B, C], got %v", nodes)
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Edge operations (single label)
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledAddEdge(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	if !lg.HasEdge("A", "B") {
		t.Error("should have edge A→B")
	}
	if !lg.HasEdgeWithLabel("A", "B", "compile") {
		t.Error("should have edge A→B with label 'compile'")
	}
}

func TestLabeledAddEdgeImplicitNodes(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("X", "Y", "dep")
	if !lg.HasNode("X") || !lg.HasNode("Y") {
		t.Error("AddEdge should implicitly add nodes")
	}
}

func TestLabeledAddEdgeDirected(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	if lg.HasEdge("B", "A") {
		t.Error("should not have reverse edge B→A")
	}
}

func TestLabeledSelfLoopPanics(t *testing.T) {
	lg := NewLabeledGraph()
	defer func() {
		if r := recover(); r == nil {
			t.Error("self-loop should panic in default labeled graph")
		}
	}()
	lg.AddEdge("A", "A", "loop")
}

func TestLabeledSelfLoopAllowed(t *testing.T) {
	lg := NewLabeledGraphAllowSelfLoops()
	lg.AddEdge("A", "A", "retry")
	if !lg.HasEdge("A", "A") {
		t.Error("should have self-loop A→A")
	}
	if !lg.HasEdgeWithLabel("A", "A", "retry") {
		t.Error("should have self-loop with label 'retry'")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Multiple labels on same edge
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledMultipleLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	if !lg.HasEdgeWithLabel("A", "B", "compile") {
		t.Error("should have 'compile' label")
	}
	if !lg.HasEdgeWithLabel("A", "B", "test") {
		t.Error("should have 'test' label")
	}
}

func TestLabeledMultipleLabelsEdgesOutput(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	edges := lg.Edges()
	// Each label produces a separate triple
	if len(edges) != 2 {
		t.Fatalf("expected 2 edge triples, got %d", len(edges))
	}
	// Sorted: compile before test
	if edges[0] != [3]string{"A", "B", "compile"} {
		t.Errorf("expected [A, B, compile], got %v", edges[0])
	}
	if edges[1] != [3]string{"A", "B", "test"} {
		t.Errorf("expected [A, B, test], got %v", edges[1])
	}
}

func TestLabeledLabelsMethod(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	lg.AddEdge("A", "B", "runtime")
	labels := lg.Labels("A", "B")
	if len(labels) != 3 {
		t.Errorf("expected 3 labels, got %d", len(labels))
	}
	for _, expected := range []string{"compile", "test", "runtime"} {
		if !labels[expected] {
			t.Errorf("expected label %q, not found", expected)
		}
	}
}

func TestLabeledLabelsEmptyEdge(t *testing.T) {
	lg := NewLabeledGraph()
	labels := lg.Labels("X", "Y")
	if len(labels) != 0 {
		t.Errorf("expected empty labels for non-existent edge, got %v", labels)
	}
}

func TestLabeledDuplicateLabelIsNoOp(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "compile") // duplicate
	labels := lg.Labels("A", "B")
	if len(labels) != 1 {
		t.Errorf("duplicate label should be no-op, got %d labels", len(labels))
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Remove edge (with labels)
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledRemoveEdgeSingleLabel(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	err := lg.RemoveEdge("A", "B", "compile")
	if err != nil {
		t.Fatal(err)
	}
	if lg.HasEdge("A", "B") {
		t.Error("edge should be removed when last label is removed")
	}
	if lg.HasEdgeWithLabel("A", "B", "compile") {
		t.Error("label should be removed")
	}
}

func TestLabeledRemoveEdgeOneOfMany(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	err := lg.RemoveEdge("A", "B", "compile")
	if err != nil {
		t.Fatal(err)
	}
	// Edge should still exist because "test" label remains
	if !lg.HasEdge("A", "B") {
		t.Error("edge should still exist — 'test' label remains")
	}
	if lg.HasEdgeWithLabel("A", "B", "compile") {
		t.Error("'compile' label should be removed")
	}
	if !lg.HasEdgeWithLabel("A", "B", "test") {
		t.Error("'test' label should still exist")
	}
}

func TestLabeledRemoveEdgeNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	err := lg.RemoveEdge("A", "B", "x")
	var efe *EdgeNotFoundError
	if !errors.As(err, &efe) {
		t.Errorf("expected EdgeNotFoundError, got %v", err)
	}
}

func TestLabeledRemoveEdgeLabelNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	err := lg.RemoveEdge("A", "B", "nonexistent")
	var lnfe *LabelNotFoundError
	if !errors.As(err, &lnfe) {
		t.Errorf("expected LabelNotFoundError, got %v", err)
	}
}

func TestLabeledRemoveAllLabelsRemovesEdge(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	_ = lg.RemoveEdge("A", "B", "compile")
	_ = lg.RemoveEdge("A", "B", "test")
	if lg.HasEdge("A", "B") {
		t.Error("edge should be removed after all labels are removed")
	}
	// Nodes should still exist
	if !lg.HasNode("A") || !lg.HasNode("B") {
		t.Error("nodes should still exist after edge removal")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — HasEdge / HasEdgeWithLabel
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledHasEdgeNoNode(t *testing.T) {
	lg := NewLabeledGraph()
	if lg.HasEdge("X", "Y") {
		t.Error("should not have edge in empty graph")
	}
}

func TestLabeledHasEdgeWithLabelNoNode(t *testing.T) {
	lg := NewLabeledGraph()
	if lg.HasEdgeWithLabel("X", "Y", "z") {
		t.Error("should not have labeled edge in empty graph")
	}
}

func TestLabeledHasEdgeWithWrongLabel(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	if lg.HasEdgeWithLabel("A", "B", "wrong") {
		t.Error("should not match wrong label")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Successors / Predecessors
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledSuccessors(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "C", "test")
	succs, err := lg.Successors("A")
	if err != nil {
		t.Fatal(err)
	}
	if len(succs) != 2 || succs[0] != "B" || succs[1] != "C" {
		t.Errorf("expected [B, C], got %v", succs)
	}
}

func TestLabeledSuccessorsNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	_, err := lg.Successors("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

func TestLabeledSuccessorsWithLabel(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "C", "test")
	lg.AddEdge("A", "D", "compile")

	succs, err := lg.SuccessorsWithLabel("A", "compile")
	if err != nil {
		t.Fatal(err)
	}
	if len(succs) != 2 || succs[0] != "B" || succs[1] != "D" {
		t.Errorf("expected [B, D], got %v", succs)
	}
}

func TestLabeledSuccessorsWithLabelNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	_, err := lg.SuccessorsWithLabel("X", "compile")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

func TestLabeledSuccessorsWithLabelEmpty(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	succs, err := lg.SuccessorsWithLabel("A", "nonexistent")
	if err != nil {
		t.Fatal(err)
	}
	if len(succs) != 0 {
		t.Errorf("expected empty result, got %v", succs)
	}
}

func TestLabeledPredecessors(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "C", "compile")
	lg.AddEdge("B", "C", "test")
	preds, err := lg.Predecessors("C")
	if err != nil {
		t.Fatal(err)
	}
	if len(preds) != 2 || preds[0] != "A" || preds[1] != "B" {
		t.Errorf("expected [A, B], got %v", preds)
	}
}

func TestLabeledPredecessorsNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	_, err := lg.Predecessors("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

func TestLabeledPredecessorsWithLabel(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "C", "compile")
	lg.AddEdge("B", "C", "test")
	lg.AddEdge("D", "C", "compile")

	preds, err := lg.PredecessorsWithLabel("C", "compile")
	if err != nil {
		t.Fatal(err)
	}
	if len(preds) != 2 || preds[0] != "A" || preds[1] != "D" {
		t.Errorf("expected [A, D], got %v", preds)
	}
}

func TestLabeledPredecessorsWithLabelNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	_, err := lg.PredecessorsWithLabel("X", "compile")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

func TestLabeledPredecessorsWithLabelEmpty(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	preds, err := lg.PredecessorsWithLabel("B", "nonexistent")
	if err != nil {
		t.Fatal(err)
	}
	if len(preds) != 0 {
		t.Errorf("expected empty result, got %v", preds)
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Algorithm delegation
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledTopologicalSort(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("B", "C", "compile")
	result, err := lg.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	expected := []string{"A", "B", "C"}
	for i, v := range expected {
		if result[i] != v {
			t.Errorf("position %d: expected %s, got %s", i, v, result[i])
		}
	}
}

func TestLabeledHasCycleNoCycle(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "dep")
	lg.AddEdge("B", "C", "dep")
	if lg.HasCycle() {
		t.Error("linear chain should not have a cycle")
	}
}

func TestLabeledHasCycleWithCycle(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "dep")
	lg.AddEdge("B", "C", "dep")
	lg.AddEdge("C", "A", "dep")
	if !lg.HasCycle() {
		t.Error("should detect cycle")
	}
}

func TestLabeledTransitiveClosure(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("B", "C", "compile")
	lg.AddEdge("A", "D", "test")
	closure, err := lg.TransitiveClosure("A")
	if err != nil {
		t.Fatal(err)
	}
	if len(closure) != 3 {
		t.Errorf("expected 3 reachable nodes, got %d", len(closure))
	}
	for _, node := range []string{"B", "C", "D"} {
		if !closure[node] {
			t.Errorf("expected %s in transitive closure", node)
		}
	}
}

func TestLabeledTransitiveClosureNotFound(t *testing.T) {
	lg := NewLabeledGraph()
	_, err := lg.TransitiveClosure("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Graph() accessor
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledGraphAccessor(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	g := lg.Graph()
	if g == nil {
		t.Fatal("Graph() should return non-nil")
	}
	if !g.HasEdge("A", "B") {
		t.Error("underlying graph should have edge A→B")
	}
}

func TestLabeledGraphIndependentGroups(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "C", "compile")
	lg.AddEdge("B", "D", "test")
	lg.AddEdge("C", "D", "test")
	groups, err := lg.Graph().IndependentGroups()
	if err != nil {
		t.Fatal(err)
	}
	if len(groups) != 3 {
		t.Fatalf("expected 3 levels, got %d", len(groups))
	}
}

func TestLabeledGraphAffectedNodes(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("B", "C", "compile")
	affected := lg.Graph().AffectedNodes(map[string]bool{"A": true})
	if len(affected) != 3 {
		t.Errorf("expected 3 affected nodes, got %d", len(affected))
	}
}

// ═══════════════════════════════════════════════════════════════════════
// LabeledGraph — Complex scenarios
// ═══════════════════════════════════════════════════════════════════════

func TestLabeledDiamondGraph(t *testing.T) {
	// Diamond: A→B, A→C, B→D, C→D with mixed labels
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "C", "test")
	lg.AddEdge("B", "D", "compile")
	lg.AddEdge("C", "D", "runtime")

	// Topological sort should work
	order, err := lg.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	if len(order) != 4 {
		t.Errorf("expected 4 nodes in topo sort, got %d", len(order))
	}

	// Compile-time successors of A should be just B
	compileSuccs, err := lg.SuccessorsWithLabel("A", "compile")
	if err != nil {
		t.Fatal(err)
	}
	if len(compileSuccs) != 1 || compileSuccs[0] != "B" {
		t.Errorf("expected compile successors [B], got %v", compileSuccs)
	}

	// Test-time successors of A should be just C
	testSuccs, err := lg.SuccessorsWithLabel("A", "test")
	if err != nil {
		t.Fatal(err)
	}
	if len(testSuccs) != 1 || testSuccs[0] != "C" {
		t.Errorf("expected test successors [C], got %v", testSuccs)
	}
}

func TestLabeledBuildSystemScenario(t *testing.T) {
	// Simulate a real build system with dependency types
	lg := NewLabeledGraph()

	// logic-gates has no dependencies (root)
	lg.AddNode("logic-gates")

	// arithmetic depends on logic-gates for compile
	lg.AddEdge("logic-gates", "arithmetic", "compile")

	// cpu-simulator depends on arithmetic for compile
	lg.AddEdge("arithmetic", "cpu-simulator", "compile")

	// test-harness depends on logic-gates for test only
	lg.AddEdge("logic-gates", "test-harness", "test")

	// Compile dependencies of logic-gates
	compileSuccs, err := lg.SuccessorsWithLabel("logic-gates", "compile")
	if err != nil {
		t.Fatal(err)
	}
	if len(compileSuccs) != 1 || compileSuccs[0] != "arithmetic" {
		t.Errorf("expected compile successors [arithmetic], got %v", compileSuccs)
	}

	// Test dependencies of logic-gates
	testSuccs, err := lg.SuccessorsWithLabel("logic-gates", "test")
	if err != nil {
		t.Fatal(err)
	}
	if len(testSuccs) != 1 || testSuccs[0] != "test-harness" {
		t.Errorf("expected test successors [test-harness], got %v", testSuccs)
	}

	// All successors (any label)
	allSuccs, err := lg.Successors("logic-gates")
	if err != nil {
		t.Fatal(err)
	}
	if len(allSuccs) != 2 {
		t.Errorf("expected 2 total successors, got %d", len(allSuccs))
	}
}

func TestLabeledEdgesReturnsSorted(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("C", "D", "z")
	lg.AddEdge("A", "B", "a")
	lg.AddEdge("A", "B", "b")
	edges := lg.Edges()
	if len(edges) != 3 {
		t.Fatalf("expected 3 edges, got %d", len(edges))
	}
	// Sorted by from, then to, then label
	if edges[0] != [3]string{"A", "B", "a"} {
		t.Errorf("expected [A,B,a], got %v", edges[0])
	}
	if edges[1] != [3]string{"A", "B", "b"} {
		t.Errorf("expected [A,B,b], got %v", edges[1])
	}
	if edges[2] != [3]string{"C", "D", "z"} {
		t.Errorf("expected [C,D,z], got %v", edges[2])
	}
}

func TestLabeledLabelsReturnsCopy(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	labels := lg.Labels("A", "B")
	// Mutating the returned map should not affect internal state
	labels["hacked"] = true
	internalLabels := lg.Labels("A", "B")
	if internalLabels["hacked"] {
		t.Error("Labels() should return a copy, not a reference")
	}
}

func TestLabeledRemoveNodeWithMultipleLabels(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	lg.AddEdge("B", "C", "runtime")
	_ = lg.RemoveNode("B")
	if lg.HasEdge("A", "B") {
		t.Error("edge A→B should be removed")
	}
	if lg.HasEdge("B", "C") {
		t.Error("edge B→C should be removed")
	}
	labels := lg.Labels("A", "B")
	if len(labels) != 0 {
		t.Errorf("expected no labels after node removal, got %v", labels)
	}
}

func TestLabeledSelfLoopWithMultipleLabels(t *testing.T) {
	lg := NewLabeledGraphAllowSelfLoops()
	lg.AddEdge("A", "A", "retry")
	lg.AddEdge("A", "A", "refresh")
	labels := lg.Labels("A", "A")
	if len(labels) != 2 {
		t.Errorf("expected 2 labels on self-loop, got %d", len(labels))
	}
	if !labels["retry"] || !labels["refresh"] {
		t.Errorf("expected retry and refresh labels, got %v", labels)
	}
}

func TestLabeledRemoveSelfLoop(t *testing.T) {
	lg := NewLabeledGraphAllowSelfLoops()
	lg.AddEdge("A", "A", "retry")
	lg.AddEdge("A", "A", "refresh")
	err := lg.RemoveEdge("A", "A", "retry")
	if err != nil {
		t.Fatal(err)
	}
	// Edge should still exist (refresh remains)
	if !lg.HasEdge("A", "A") {
		t.Error("self-loop should still exist — 'refresh' label remains")
	}
	// Remove last label
	err = lg.RemoveEdge("A", "A", "refresh")
	if err != nil {
		t.Fatal(err)
	}
	if lg.HasEdge("A", "A") {
		t.Error("self-loop should be fully removed")
	}
}

func TestLabeledIsolatedNodeTopoSort(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddNode("isolated")
	lg.AddEdge("A", "B", "dep")
	order, err := lg.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	if len(order) != 3 {
		t.Errorf("expected 3 nodes in topo sort, got %d", len(order))
	}
}

func TestLabeledEdgesAfterRemoval(t *testing.T) {
	lg := NewLabeledGraph()
	lg.AddEdge("A", "B", "compile")
	lg.AddEdge("A", "B", "test")
	_ = lg.RemoveEdge("A", "B", "compile")
	edges := lg.Edges()
	if len(edges) != 1 {
		t.Fatalf("expected 1 edge after removal, got %d", len(edges))
	}
	if edges[0] != [3]string{"A", "B", "test"} {
		t.Errorf("expected [A,B,test], got %v", edges[0])
	}
}
