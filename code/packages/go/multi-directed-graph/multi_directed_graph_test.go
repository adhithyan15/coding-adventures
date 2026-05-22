package multidirectedgraph

import "testing"

func TestKeepsParallelEdgesWithStableIDsAndProperties(t *testing.T) {
	graph := New[string]()
	first, err := graph.AddEdge("a", "b", 0.25, nil, "")
	if err != nil {
		t.Fatal(err)
	}
	second, err := graph.AddEdge("a", "b", 0.75, PropertyBag{"trainable": true}, "w1")
	if err != nil {
		t.Fatal(err)
	}
	if first != "e0" || second != "w1" {
		t.Fatalf("unexpected edge IDs: %s %s", first, second)
	}
	edges, err := graph.EdgesBetween("a", "b")
	if err != nil {
		t.Fatal(err)
	}
	if len(edges) != 2 {
		t.Fatalf("parallel edge count = %d, want 2", len(edges))
	}
	properties, ok := graph.EdgeProperties("w1")
	if !ok || properties["trainable"] != true || properties["weight"] != 0.75 {
		t.Fatalf("edge properties not preserved: %#v", properties)
	}
}

func TestRejectsDuplicateEdgeIDs(t *testing.T) {
	graph := New[string]()
	if _, err := graph.AddEdge("a", "b", 1, nil, "edge"); err != nil {
		t.Fatal(err)
	}
	if _, err := graph.AddEdge("a", "c", 1, nil, "edge"); err == nil {
		t.Fatalf("expected duplicate edge ID error")
	}
}

func TestSynchronizesWeightProperty(t *testing.T) {
	graph := New[string]()
	if _, err := graph.AddEdge("a", "b", 2, nil, "w"); err != nil {
		t.Fatal(err)
	}
	if err := graph.SetEdgeProperty("w", "weight", 3.5); err != nil {
		t.Fatal(err)
	}
	weight, ok := graph.EdgeWeight("w")
	if !ok || weight != 3.5 {
		t.Fatalf("weight = %v ok=%v, want 3.5 true", weight, ok)
	}
	if err := graph.RemoveEdgeProperty("w", "weight"); err != nil {
		t.Fatal(err)
	}
	weight, _ = graph.EdgeWeight("w")
	if weight != 1 {
		t.Fatalf("weight after removing property = %v, want 1", weight)
	}
}

func TestTopologicalSortAndIndependentGroups(t *testing.T) {
	graph := New[string]()
	graph.AddEdge("a", "c", 1, nil, "")
	graph.AddEdge("b", "c", 1, nil, "")
	graph.AddEdge("c", "d", 1, nil, "")

	order, err := graph.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	if got := join(order); got != "a,b,c,d" {
		t.Fatalf("order = %s", got)
	}
	groups, err := graph.IndependentGroups()
	if err != nil {
		t.Fatal(err)
	}
	if join(groups[0]) != "a,b" || join(groups[1]) != "c" || join(groups[2]) != "d" {
		t.Fatalf("groups = %#v", groups)
	}
}

func TestDetectsCyclesAndRemovesIncidentEdges(t *testing.T) {
	graph := New[string]()
	graph.AddEdge("a", "b", 1, nil, "")
	graph.AddEdge("b", "a", 1, nil, "")
	if !graph.HasCycle() {
		t.Fatalf("expected cycle")
	}
	if err := graph.RemoveNode("b"); err != nil {
		t.Fatal(err)
	}
	if len(graph.Edges()) != 0 {
		t.Fatalf("incident edges not removed")
	}
}

func join(values []string) string {
	result := ""
	for index, value := range values {
		if index > 0 {
			result += ","
		}
		result += value
	}
	return result
}
