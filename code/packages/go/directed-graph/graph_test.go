package directedgraph

import (
	"errors"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════
// Empty graph tests
// ═══════════════════════════════════════════════════════════════════════

func TestEmptyGraphNodes(t *testing.T) {
	g := New()
	if len(g.Nodes()) != 0 {
		t.Errorf("expected 0 nodes, got %d", len(g.Nodes()))
	}
}

func TestEmptyGraphEdges(t *testing.T) {
	g := New()
	if len(g.Edges()) != 0 {
		t.Errorf("expected 0 edges, got %d", len(g.Edges()))
	}
}

func TestEmptyGraphTopoSort(t *testing.T) {
	g := New()
	result, err := g.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	if len(result) != 0 {
		t.Errorf("expected empty topo sort, got %v", result)
	}
}

func TestEmptyGraphSize(t *testing.T) {
	g := New()
	if g.Size() != 0 {
		t.Errorf("expected size 0, got %d", g.Size())
	}
}

func TestEmptyGraphHasCycle(t *testing.T) {
	g := New()
	if g.HasCycle() {
		t.Error("empty graph should not have a cycle")
	}
}

func TestEmptyGraphIndependentGroups(t *testing.T) {
	g := New()
	groups, err := g.IndependentGroups()
	if err != nil {
		t.Fatal(err)
	}
	if len(groups) != 0 {
		t.Errorf("expected 0 groups, got %d", len(groups))
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Single node tests
// ═══════════════════════════════════════════════════════════════════════

func TestSingleNode(t *testing.T) {
	g := New()
	g.AddNode("A")
	if !g.HasNode("A") {
		t.Error("should have node A")
	}
	if g.Size() != 1 {
		t.Errorf("expected size 1, got %d", g.Size())
	}
}

func TestAddNodeIdempotent(t *testing.T) {
	g := New()
	g.AddNode("A")
	g.AddNode("A")
	if g.Size() != 1 {
		t.Errorf("duplicate add should be no-op, got size %d", g.Size())
	}
}

func TestRemoveNode(t *testing.T) {
	g := New()
	g.AddNode("A")
	err := g.RemoveNode("A")
	if err != nil {
		t.Fatal(err)
	}
	if g.HasNode("A") {
		t.Error("should not have node A after removal")
	}
}

func TestRemoveNodeNotFound(t *testing.T) {
	g := New()
	err := g.RemoveNode("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Errorf("expected NodeNotFoundError, got %v", err)
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Edge tests
// ═══════════════════════════════════════════════════════════════════════

func TestAddEdge(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	if !g.HasEdge("A", "B") {
		t.Error("should have edge A→B")
	}
	if g.HasEdge("B", "A") {
		t.Error("should not have edge B→A (directed)")
	}
}

func TestAddEdgeImplicitNodes(t *testing.T) {
	g := New()
	g.AddEdge("X", "Y")
	if !g.HasNode("X") || !g.HasNode("Y") {
		t.Error("add_edge should implicitly add nodes")
	}
}

func TestSelfLoopPanics(t *testing.T) {
	g := New()
	defer func() {
		if r := recover(); r == nil {
			t.Error("self-loop should panic")
		}
	}()
	g.AddEdge("A", "A")
}

func TestRemoveEdge(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	err := g.RemoveEdge("A", "B")
	if err != nil {
		t.Fatal(err)
	}
	if g.HasEdge("A", "B") {
		t.Error("edge should be removed")
	}
	// Nodes should still exist
	if !g.HasNode("A") || !g.HasNode("B") {
		t.Error("nodes should still exist after edge removal")
	}
}

func TestRemoveEdgeNotFound(t *testing.T) {
	g := New()
	g.AddNode("A")
	err := g.RemoveEdge("A", "B")
	var efe *EdgeNotFoundError
	if !errors.As(err, &efe) {
		t.Errorf("expected EdgeNotFoundError, got %v", err)
	}
}

func TestRemoveNodeCleansEdges(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	_ = g.RemoveNode("B")
	if g.HasEdge("A", "B") || g.HasEdge("B", "C") {
		t.Error("edges should be cleaned up on node removal")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Predecessors and Successors
// ═══════════════════════════════════════════════════════════════════════

func TestPredecessors(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("C", "B")
	preds, err := g.Predecessors("B")
	if err != nil {
		t.Fatal(err)
	}
	if len(preds) != 2 || preds[0] != "A" || preds[1] != "C" {
		t.Errorf("expected [A, C], got %v", preds)
	}
}

func TestSuccessors(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("A", "C")
	succs, err := g.Successors("A")
	if err != nil {
		t.Fatal(err)
	}
	if len(succs) != 2 || succs[0] != "B" || succs[1] != "C" {
		t.Errorf("expected [B, C], got %v", succs)
	}
}

func TestPredecessorsNotFound(t *testing.T) {
	g := New()
	_, err := g.Predecessors("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

func TestSuccessorsNotFound(t *testing.T) {
	g := New()
	_, err := g.Successors("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Linear chain: A → B → C → D
// ═══════════════════════════════════════════════════════════════════════

func buildLinearChain() *Graph {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	g.AddEdge("C", "D")
	return g
}

func TestLinearTopoSort(t *testing.T) {
	g := buildLinearChain()
	result, err := g.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	expected := []string{"A", "B", "C", "D"}
	for i, v := range expected {
		if result[i] != v {
			t.Errorf("position %d: expected %s, got %s", i, v, result[i])
		}
	}
}

func TestLinearIndependentGroups(t *testing.T) {
	g := buildLinearChain()
	groups, err := g.IndependentGroups()
	if err != nil {
		t.Fatal(err)
	}
	if len(groups) != 4 {
		t.Fatalf("expected 4 levels, got %d", len(groups))
	}
	for i, expected := range []string{"A", "B", "C", "D"} {
		if len(groups[i]) != 1 || groups[i][0] != expected {
			t.Errorf("level %d: expected [%s], got %v", i, expected, groups[i])
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Diamond: A→B, A→C, B→D, C→D
// ═══════════════════════════════════════════════════════════════════════

func buildDiamond() *Graph {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("A", "C")
	g.AddEdge("B", "D")
	g.AddEdge("C", "D")
	return g
}

func TestDiamondIndependentGroups(t *testing.T) {
	g := buildDiamond()
	groups, err := g.IndependentGroups()
	if err != nil {
		t.Fatal(err)
	}
	if len(groups) != 3 {
		t.Fatalf("expected 3 levels, got %d", len(groups))
	}
	// Level 0: [A]
	if len(groups[0]) != 1 || groups[0][0] != "A" {
		t.Errorf("level 0: expected [A], got %v", groups[0])
	}
	// Level 1: [B, C] — parallel
	if len(groups[1]) != 2 || groups[1][0] != "B" || groups[1][1] != "C" {
		t.Errorf("level 1: expected [B, C], got %v", groups[1])
	}
	// Level 2: [D]
	if len(groups[2]) != 1 || groups[2][0] != "D" {
		t.Errorf("level 2: expected [D], got %v", groups[2])
	}
}

func TestDiamondTransitiveClosure(t *testing.T) {
	g := buildDiamond()
	closure, err := g.TransitiveClosure("A")
	if err != nil {
		t.Fatal(err)
	}
	if len(closure) != 3 { // B, C, D
		t.Errorf("expected 3 reachable nodes, got %d", len(closure))
	}
	for _, node := range []string{"B", "C", "D"} {
		if !closure[node] {
			t.Errorf("expected %s in transitive closure", node)
		}
	}
}

func TestDiamondTransitiveDependents(t *testing.T) {
	g := buildDiamond()
	// Edge direction: A→B means "B depends on A"
	// TransitiveDependents follows FORWARD edges (same as TransitiveClosure)

	// D is a leaf — nothing depends on D
	deps, err := g.TransitiveDependents("D")
	if err != nil {
		t.Fatal(err)
	}
	if len(deps) != 0 {
		t.Errorf("D has no dependents, got %v", deps)
	}

	// A is the root — everything depends on A
	deps, err = g.TransitiveDependents("A")
	if err != nil {
		t.Fatal(err)
	}
	if len(deps) != 3 {
		t.Errorf("expected 3 dependents of A (B,C,D), got %d: %v", len(deps), deps)
	}
	for _, node := range []string{"B", "C", "D"} {
		if !deps[node] {
			t.Errorf("expected %s in dependents of A", node)
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Cycle detection
// ═══════════════════════════════════════════════════════════════════════

func TestCycleDetection(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	g.AddEdge("C", "A") // cycle!
	if !g.HasCycle() {
		t.Error("should detect cycle")
	}
}

func TestCycleInTopoSort(t *testing.T) {
	g := New()
	g.AddEdge("A", "B")
	g.AddEdge("B", "C")
	g.AddEdge("C", "A")
	_, err := g.TopologicalSort()
	var ce *CycleError
	if !errors.As(err, &ce) {
		t.Error("expected CycleError from topological sort")
	}
}

func TestNoCycle(t *testing.T) {
	g := buildDiamond()
	if g.HasCycle() {
		t.Error("diamond should not have a cycle")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Affected nodes
// ═══════════════════════════════════════════════════════════════════════

func TestAffectedNodesLeaf(t *testing.T) {
	g := buildDiamond()
	// D is a leaf — nothing depends on D, so only D is affected
	affected := g.AffectedNodes(map[string]bool{"D": true})
	if len(affected) != 1 || !affected["D"] {
		t.Errorf("expected only {D}, got %v", affected)
	}
}

func TestAffectedNodesRoot(t *testing.T) {
	g := buildDiamond()
	// A is the root — everything depends on A
	// If A changes, A + B + C + D all need rebuilding
	affected := g.AffectedNodes(map[string]bool{"A": true})
	if len(affected) != 4 {
		t.Errorf("expected 4 affected nodes, got %d: %v", len(affected), affected)
	}
	for _, node := range []string{"A", "B", "C", "D"} {
		if !affected[node] {
			t.Errorf("expected %s in affected set", node)
		}
	}
}

func TestAffectedNodesMiddle(t *testing.T) {
	g := buildDiamond()
	// B is in the middle — D depends on B
	affected := g.AffectedNodes(map[string]bool{"B": true})
	if len(affected) != 2 {
		t.Errorf("expected {B, D}, got %v", affected)
	}
	if !affected["B"] || !affected["D"] {
		t.Errorf("expected B and D in affected set, got %v", affected)
	}
}

func TestAffectedNodesNonexistent(t *testing.T) {
	g := buildDiamond()
	affected := g.AffectedNodes(map[string]bool{"X": true})
	if len(affected) != 0 {
		t.Errorf("nonexistent node should produce empty affected set, got %v", affected)
	}
}

func TestTransitiveClosureNotFound(t *testing.T) {
	g := New()
	_, err := g.TransitiveClosure("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

func TestTransitiveDependentsNotFound(t *testing.T) {
	g := New()
	_, err := g.TransitiveDependents("X")
	var nfe *NodeNotFoundError
	if !errors.As(err, &nfe) {
		t.Error("expected NodeNotFoundError")
	}
}

func TestEdgesReturnsSorted(t *testing.T) {
	g := New()
	g.AddEdge("C", "D")
	g.AddEdge("A", "B")
	edges := g.Edges()
	if edges[0] != [2]string{"A", "B"} {
		t.Errorf("expected first edge [A,B], got %v", edges[0])
	}
	if edges[1] != [2]string{"C", "D"} {
		t.Errorf("expected second edge [C,D], got %v", edges[1])
	}
}

func TestHasEdgeNoNode(t *testing.T) {
	g := New()
	if g.HasEdge("X", "Y") {
		t.Error("should not have edge in empty graph")
	}
}

func TestContainsHelper(t *testing.T) {
	if !contains([]string{"a", "b", "c"}, "b") {
		t.Error("should contain b")
	}
	if contains([]string{"a", "b"}, "z") {
		t.Error("should not contain z")
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Real repo dependency graph
// ═══════════════════════════════════════════════════════════════════════

func buildRepoGraph() *Graph {
	g := New()
	// Independent roots
	for _, pkg := range []string{
		"logic-gates", "grammar-tools", "virtual-machine",
		"jvm-simulator", "clr-simulator", "wasm-simulator",
		"intel4004-simulator", "html-renderer",
	} {
		g.AddNode(pkg)
	}
	// Dependency edges (from dependency TO dependent)
	g.AddEdge("logic-gates", "arithmetic")
	g.AddEdge("arithmetic", "cpu-simulator")
	g.AddEdge("cpu-simulator", "arm-simulator")
	g.AddEdge("cpu-simulator", "riscv-simulator")
	g.AddEdge("grammar-tools", "lexer")
	g.AddEdge("lexer", "parser")
	g.AddEdge("grammar-tools", "parser")
	g.AddEdge("lexer", "bytecode-compiler")
	g.AddEdge("parser", "bytecode-compiler")
	g.AddEdge("virtual-machine", "bytecode-compiler")
	g.AddEdge("lexer", "pipeline")
	g.AddEdge("parser", "pipeline")
	g.AddEdge("bytecode-compiler", "pipeline")
	g.AddEdge("virtual-machine", "pipeline")
	g.AddEdge("arm-simulator", "assembler")
	g.AddEdge("virtual-machine", "jit-compiler")
	g.AddEdge("assembler", "jit-compiler")
	return g
}

func TestRepoGraphNoCycle(t *testing.T) {
	g := buildRepoGraph()
	if g.HasCycle() {
		t.Error("repo graph should not have a cycle")
	}
}

func TestRepoGraphTopoSort(t *testing.T) {
	g := buildRepoGraph()
	order, err := g.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	if len(order) != g.Size() {
		t.Errorf("topo sort should include all %d nodes, got %d", g.Size(), len(order))
	}
	// Verify ordering: every dependency appears before its dependent
	pos := make(map[string]int)
	for i, n := range order {
		pos[n] = i
	}
	for _, edge := range g.Edges() {
		if pos[edge[0]] >= pos[edge[1]] {
			t.Errorf("%s should come before %s in topo sort", edge[0], edge[1])
		}
	}
}

func TestRepoGraphIndependentGroups(t *testing.T) {
	g := buildRepoGraph()
	groups, err := g.IndependentGroups()
	if err != nil {
		t.Fatal(err)
	}
	// Level 0 should contain all independent roots
	level0 := make(map[string]bool)
	for _, n := range groups[0] {
		level0[n] = true
	}
	for _, root := range []string{"logic-gates", "grammar-tools", "virtual-machine", "jvm-simulator", "clr-simulator"} {
		if !level0[root] {
			t.Errorf("expected %s in level 0, got level 0 = %v", root, groups[0])
		}
	}
	t.Logf("Groups: %v", groups)
}
