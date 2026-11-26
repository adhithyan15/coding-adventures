package graph

import (
	"math"
	"reflect"
	"testing"
)

func representations() []GraphRepr {
	return []GraphRepr{AdjacencyList, AdjacencyMatrix}
}

func makeGraph(repr GraphRepr) *Graph {
	g := New(repr)
	g.AddEdge("London", "Paris", 300)
	g.AddEdge("London", "Amsterdam", 520)
	g.AddEdge("Paris", "Berlin", 878)
	g.AddEdge("Amsterdam", "Berlin", 655)
	g.AddEdge("Amsterdam", "Brussels", 180)
	return g
}

func makeTriangle(repr GraphRepr) *Graph {
	g := New(repr)
	g.AddEdge("A", "B", 1)
	g.AddEdge("B", "C", 1)
	g.AddEdge("C", "A", 1)
	return g
}

func makePath(repr GraphRepr) *Graph {
	g := New(repr)
	g.AddEdge("A", "B", 1)
	g.AddEdge("B", "C", 1)
	return g
}

func TestConstructionAndNodes(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)
		if g.Size() != 0 {
			t.Fatalf("Size() = %d, want 0", g.Size())
		}
		g.AddNode("A")
		g.AddNode("B")
		if !g.HasNode("A") || !g.HasNode("B") {
			t.Fatalf("HasNode failed for repr=%s", repr)
		}
		if err := g.RemoveNode("A"); err != nil {
			t.Fatalf("RemoveNode failed: %v", err)
		}
		if g.HasNode("A") {
			t.Fatalf("node A should be removed for repr=%s", repr)
		}
	}
}

func TestEdgesAndNeighbors(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)
		g.AddEdge("A", "B", 2.5)
		if !g.HasEdge("A", "B") || !g.HasEdge("B", "A") {
			t.Fatalf("undirected edge missing for repr=%s", repr)
		}
		weight, err := g.EdgeWeight("A", "B")
		if err != nil || weight != 2.5 {
			t.Fatalf("EdgeWeight = %v, %v", weight, err)
		}
		neighbors, _ := g.Neighbors("A")
		if !reflect.DeepEqual(neighbors, []string{"B"}) {
			t.Fatalf("Neighbors(A) = %#v", neighbors)
		}
	}
}

func TestPropertyBags(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)

		g.SetGraphProperty("name", "city-map")
		g.SetGraphProperty("version", 1)
		graphProps := g.GraphProperties()
		if graphProps["name"] != "city-map" || graphProps["version"] != 1 {
			t.Fatalf("GraphProperties() = %#v", graphProps)
		}
		graphProps["name"] = "mutated"
		if g.GraphProperties()["name"] != "city-map" {
			t.Fatalf("GraphProperties should return a copy for repr=%s", repr)
		}
		g.RemoveGraphProperty("version")
		if _, ok := g.GraphProperties()["version"]; ok {
			t.Fatalf("RemoveGraphProperty failed for repr=%s", repr)
		}

		g.AddNode("A", PropertyBag{"kind": "input"})
		g.AddNode("A", PropertyBag{"trainable": false})
		if err := g.SetNodeProperty("A", "slot", 0); err != nil {
			t.Fatalf("SetNodeProperty failed: %v", err)
		}
		nodeProps, err := g.NodeProperties("A")
		if err != nil {
			t.Fatalf("NodeProperties failed: %v", err)
		}
		wantNodeProps := PropertyBag{"kind": "input", "trainable": false, "slot": 0}
		if !reflect.DeepEqual(nodeProps, wantNodeProps) {
			t.Fatalf("NodeProperties = %#v, want %#v", nodeProps, wantNodeProps)
		}
		nodeProps["kind"] = "mutated"
		nodeProps, _ = g.NodeProperties("A")
		if nodeProps["kind"] != "input" {
			t.Fatalf("NodeProperties should return a copy for repr=%s", repr)
		}
		if err := g.RemoveNodeProperty("A", "slot"); err != nil {
			t.Fatalf("RemoveNodeProperty failed: %v", err)
		}

		g.AddEdge("A", "B", 2.5, PropertyBag{"role": "distance"})
		edgeProps, err := g.EdgeProperties("B", "A")
		if err != nil {
			t.Fatalf("EdgeProperties failed: %v", err)
		}
		wantEdgeProps := PropertyBag{"role": "distance", "weight": 2.5}
		if !reflect.DeepEqual(edgeProps, wantEdgeProps) {
			t.Fatalf("EdgeProperties = %#v, want %#v", edgeProps, wantEdgeProps)
		}
		if err := g.SetEdgeProperty("B", "A", "weight", 7); err != nil {
			t.Fatalf("SetEdgeProperty weight failed: %v", err)
		}
		weight, _ := g.EdgeWeight("A", "B")
		if weight != 7 {
			t.Fatalf("EdgeWeight after weight property = %v, want 7", weight)
		}
		if err := g.SetEdgeProperty("A", "B", "trainable", true); err != nil {
			t.Fatalf("SetEdgeProperty failed: %v", err)
		}
		if err := g.RemoveEdgeProperty("A", "B", "role"); err != nil {
			t.Fatalf("RemoveEdgeProperty failed: %v", err)
		}
		edgeProps, _ = g.EdgeProperties("A", "B")
		wantEdgeProps = PropertyBag{"trainable": true, "weight": float64(7)}
		if !reflect.DeepEqual(edgeProps, wantEdgeProps) {
			t.Fatalf("EdgeProperties after updates = %#v, want %#v", edgeProps, wantEdgeProps)
		}

		if err := g.RemoveEdge("A", "B"); err != nil {
			t.Fatalf("RemoveEdge failed: %v", err)
		}
		if _, err := g.EdgeProperties("A", "B"); err == nil {
			t.Fatalf("EdgeProperties should fail after RemoveEdge for repr=%s", repr)
		}
	}
}

func TestTraversalsAndConnectivity(t *testing.T) {
	for _, repr := range representations() {
		bfsResult, _ := BFS(makePath(repr), "A")
		if !reflect.DeepEqual(bfsResult, []string{"A", "B", "C"}) {
			t.Fatalf("BFS = %#v", bfsResult)
		}
		dfsResult, _ := DFS(makePath(repr), "A")
		if !reflect.DeepEqual(dfsResult, []string{"A", "B", "C"}) {
			t.Fatalf("DFS = %#v", dfsResult)
		}
		if !IsConnected(makeGraph(repr)) {
			t.Fatalf("expected connected graph for repr=%s", repr)
		}
	}
}

func TestShortestPathRejectsInvalidWeightsBeforeEndpointShortcut(t *testing.T) {
	invalidWeights := []struct {
		name   string
		weight float64
	}{
		{name: "negative", weight: -1},
		{name: "NaN", weight: math.NaN()},
		{name: "positive infinity", weight: math.Inf(1)},
		{name: "negative infinity", weight: math.Inf(-1)},
	}

	for _, repr := range representations() {
		for _, test := range invalidWeights {
			t.Run(string(repr)+"/"+test.name, func(t *testing.T) {
				g := New(repr)
				g.AddNode("A")
				g.AddEdge("X", "Y", test.weight) // unrelated to the query

				path, err := ShortestPath(g, "A", "A")
				if err == nil {
					t.Fatalf("ShortestPath(A, A) = %#v, nil error; want invalid-weight error", path)
				}
				if path != nil {
					t.Fatalf("ShortestPath(A, A) path = %#v, want nil on invalid graph", path)
				}
			})
		}
	}
}

func TestShortestPathRejectsInvalidWeightPropertyUpdates(t *testing.T) {
	for _, repr := range representations() {
		for _, weight := range []float64{-2, math.NaN(), math.Inf(1)} {
			g := New(repr)
			g.AddEdge("A", "B", 1)
			if err := g.SetEdgeProperty("A", "B", "weight", weight); err != nil {
				t.Fatalf("SetEdgeProperty(%v) failed for repr=%s: %v", weight, repr, err)
			}
			if _, err := ShortestPath(g, "A", "B"); err == nil {
				t.Fatalf("ShortestPath accepted weight %v for repr=%s", weight, repr)
			}
		}
	}
}

func TestShortestPathWeightValidationPreservesMutationAndMSTSemantics(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)
		g.AddEdge("A", "B", -2)
		g.AddEdge("B", "C", 1)

		weight, err := g.EdgeWeight("A", "B")
		if err != nil || weight != -2 {
			t.Fatalf("EdgeWeight(A, B) = %v, %v; want retained -2 for repr=%s", weight, err, repr)
		}
		if !g.HasEdge("A", "B") {
			t.Fatalf("AddEdge should retain a finite negative edge for repr=%s", repr)
		}
		if _, err := ShortestPath(g, "A", "C"); err == nil {
			t.Fatalf("ShortestPath accepted finite negative edge for repr=%s", repr)
		}

		mst, err := MinimumSpanningTree(g)
		if err != nil {
			t.Fatalf("MinimumSpanningTree rejected finite negative edge for repr=%s: %v", repr, err)
		}
		if len(mst) != 2 || mst[0].Weight != -2 {
			t.Fatalf("MinimumSpanningTree = %#v; want both edges including weight -2 for repr=%s", mst, repr)
		}
	}
}

func TestShortestPathSupportsFiniteDistancesAboveFormerSentinel(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)
		g.AddEdge("A", "B", 1.2e308)

		path, err := ShortestPath(g, "A", "B")
		if err != nil {
			t.Fatalf("ShortestPath with a large finite distance failed for repr=%s: %v", repr, err)
		}
		if !reflect.DeepEqual(path, []string{"A", "B"}) {
			t.Fatalf("ShortestPath with a large finite distance = %#v, want [A B] for repr=%s", path, repr)
		}
	}
}

func TestComponentsAndCycleDetection(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)
		g.AddEdge("A", "B", 1)
		g.AddEdge("B", "C", 1)
		g.AddEdge("D", "E", 1)
		g.AddNode("F")
		components := ConnectedComponents(g)
		if len(components) != 3 {
			t.Fatalf("ConnectedComponents count = %d, want 3", len(components))
		}
		if !HasCycle(makeTriangle(repr)) {
			t.Fatalf("triangle should have a cycle for repr=%s", repr)
		}
		if HasCycle(makePath(repr)) {
			t.Fatalf("path should not have a cycle for repr=%s", repr)
		}
	}
}

func TestShortestPathAndMST(t *testing.T) {
	for _, repr := range representations() {
		path, err := ShortestPath(makeGraph(repr), "London", "Berlin")
		if err != nil {
			t.Fatalf("ShortestPath failed: %v", err)
		}
		wantPath := []string{"London", "Amsterdam", "Berlin"}
		if !reflect.DeepEqual(path, wantPath) {
			t.Fatalf("ShortestPath = %#v, want %#v", path, wantPath)
		}

		mst, err := MinimumSpanningTree(makeGraph(repr))
		if err != nil {
			t.Fatalf("MinimumSpanningTree failed: %v", err)
		}
		if len(mst) != 4 {
			t.Fatalf("MST edge count = %d, want 4", len(mst))
		}
	}
}

func TestDisconnectedMSTFails(t *testing.T) {
	for _, repr := range representations() {
		g := New(repr)
		g.AddEdge("A", "B", 1)
		g.AddNode("C")
		if _, err := MinimumSpanningTree(g); err == nil {
			t.Fatalf("MinimumSpanningTree should fail on disconnected graph for repr=%s", repr)
		}
	}
}
