package multidirectedgraph

import (
	"errors"
	"math"
	"reflect"
	"testing"
)

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
	successors, err := graph.Successors("a")
	if err != nil {
		t.Fatal(err)
	}
	predecessors, err := graph.Predecessors("b")
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(successors, []string{"b"}) || !reflect.DeepEqual(predecessors, []string{"a"}) {
		t.Fatalf("parallel edges were not deduplicated: successors=%v predecessors=%v", successors, predecessors)
	}
	properties, err := graph.EdgeProperties("w1")
	if err != nil || properties["trainable"] != true || properties["weight"] != 0.75 {
		t.Fatalf("edge properties not preserved: properties=%#v err=%v", properties, err)
	}
}

func TestAutoAllocatedEdgeIDsAvoidExplicitIDs(t *testing.T) {
	graph := New[string]()
	if _, err := graph.AddEdge("a", "b", 1, nil, "e0"); err != nil {
		t.Fatal(err)
	}
	edgeID, err := graph.AddEdge("b", "c", 1, nil, "")
	if err != nil {
		t.Fatal(err)
	}
	if edgeID != "e1" {
		t.Fatalf("edge ID = %q, want e1", edgeID)
	}
}

func TestRejectsDuplicateEdgeIDs(t *testing.T) {
	graph := New[string]()
	if _, err := graph.AddEdge("a", "b", 1, nil, "edge"); err != nil {
		t.Fatal(err)
	}
	_, err := graph.AddEdge("a", "c", 1, nil, "edge")
	var duplicate DuplicateEdgeIDError
	if !errors.As(err, &duplicate) || duplicate.EdgeID != "edge" {
		t.Fatalf("expected DuplicateEdgeIDError for edge, got %v", err)
	}
}

func TestCopiesAndMutatesPropertyBags(t *testing.T) {
	graph := New[string]()
	nodeInput := PropertyBag{"role": "input"}
	graph.AddNode("a", nodeInput)
	graph.AddNode("a", PropertyBag{"shape": "[1]"})
	nodeInput["role"] = "mutated"

	graph.SetGraphProperty("name", "generic")
	graph.SetGraphProperty("version", 1)
	graph.RemoveGraphProperty("version")
	graphProperties := graph.GraphProperties()
	graphProperties["name"] = "mutated"
	if graph.GraphProperties()["name"] != "generic" {
		t.Fatal("graph properties leaked a mutable internal map")
	}

	nodeProperties, err := graph.NodeProperties("a")
	if err != nil {
		t.Fatal(err)
	}
	if nodeProperties["role"] != "input" || nodeProperties["shape"] != "[1]" {
		t.Fatalf("node properties = %#v", nodeProperties)
	}
	nodeProperties["role"] = "mutated"
	if err := graph.SetNodeProperty("a", "kind", "feature"); err != nil {
		t.Fatal(err)
	}
	if err := graph.RemoveNodeProperty("a", "shape"); err != nil {
		t.Fatal(err)
	}
	nodeProperties, err = graph.NodeProperties("a")
	if err != nil {
		t.Fatal(err)
	}
	if nodeProperties["role"] != "input" || nodeProperties["kind"] != "feature" {
		t.Fatalf("node property updates = %#v", nodeProperties)
	}
	if _, exists := nodeProperties["shape"]; exists {
		t.Fatalf("removed node property still present: %#v", nodeProperties)
	}
}

func TestSynchronizesWeightProperty(t *testing.T) {
	graph := New[string]()
	input := PropertyBag{"channel": "left"}
	if _, err := graph.AddEdge("a", "b", 2, input, "w"); err != nil {
		t.Fatal(err)
	}
	input["channel"] = "mutated"
	properties, err := graph.EdgeProperties("w")
	if err != nil || properties["channel"] != "left" {
		t.Fatalf("edge property input was not copied: properties=%#v err=%v", properties, err)
	}
	properties["channel"] = "mutated"
	properties, _ = graph.EdgeProperties("w")
	if properties["channel"] != "left" {
		t.Fatal("edge properties leaked a mutable internal map")
	}

	if err := graph.SetEdgeProperty("w", "weight", int32(3)); err != nil {
		t.Fatal(err)
	}
	weight, err := graph.EdgeWeight("w")
	if err != nil || weight != 3 {
		t.Fatalf("weight = %v err=%v, want 3", weight, err)
	}
	if err := graph.SetEdgeProperty("w", "weight", "heavy"); err == nil {
		t.Fatal("expected a non-numeric weight error")
	}
	if err := graph.SetEdgeProperty("w", "weight", math.NaN()); err == nil {
		t.Fatal("expected a NaN weight error")
	}
	if err := graph.RemoveEdgeProperty("w", "weight"); err != nil {
		t.Fatal(err)
	}
	weight, err = graph.EdgeWeight("w")
	if err != nil || weight != 1 {
		t.Fatalf("weight after removing property = %v err=%v, want 1", weight, err)
	}
}

func TestControlsSelfLoops(t *testing.T) {
	graph := New[string]()
	if _, err := graph.AddEdge("a", "a", 1, nil, ""); err == nil {
		t.Fatal("expected self-loop rejection")
	}

	graph = NewAllowSelfLoops[string]()
	edgeID, err := graph.AddEdge("a", "a", 1, nil, "")
	if err != nil || !graph.HasEdge(edgeID) || !graph.HasCycle() {
		t.Fatalf("allowed self-loop was not retained: edge=%q err=%v", edgeID, err)
	}
}

func TestMissingNodesAndEdgesReturnTypedErrors(t *testing.T) {
	graph := New[string]()

	nodeChecks := []func() error{
		func() error { _, err := graph.NodeProperties("missing"); return err },
		func() error { _, err := graph.OutgoingEdges("missing"); return err },
		func() error { _, err := graph.IncomingEdges("missing"); return err },
		func() error { _, err := graph.Successors("missing"); return err },
		func() error { _, err := graph.Predecessors("missing"); return err },
		func() error { return graph.SetNodeProperty("missing", "key", "value") },
		func() error { return graph.RemoveNodeProperty("missing", "key") },
		func() error { return graph.RemoveNode("missing") },
	}
	for index, check := range nodeChecks {
		var target NodeNotFoundError[string]
		if err := check(); !errors.As(err, &target) {
			t.Fatalf("node check %d returned %v, want NodeNotFoundError", index, err)
		}
	}

	edgeChecks := []func() error{
		func() error { _, err := graph.Edge("missing"); return err },
		func() error { _, err := graph.EdgeWeight("missing"); return err },
		func() error { _, err := graph.EdgeProperties("missing"); return err },
		func() error { return graph.SetEdgeProperty("missing", "key", "value") },
		func() error { return graph.RemoveEdgeProperty("missing", "key") },
		func() error { return graph.RemoveEdge("missing") },
	}
	for index, check := range edgeChecks {
		var target EdgeNotFoundError
		if err := check(); !errors.As(err, &target) {
			t.Fatalf("edge check %d returned %v, want EdgeNotFoundError", index, err)
		}
	}
}

func TestRemovesEdgesAndIncidentEdges(t *testing.T) {
	graph := New[string]()
	incoming, _ := graph.AddEdge("a", "b", 1, nil, "")
	outgoing, _ := graph.AddEdge("b", "c", 1, nil, "")
	kept, _ := graph.AddEdge("a", "c", 1, nil, "")

	if err := graph.RemoveEdge(kept); err != nil {
		t.Fatal(err)
	}
	if graph.HasEdge(kept) {
		t.Fatal("removed edge still exists")
	}
	if err := graph.RemoveNode("b"); err != nil {
		t.Fatal(err)
	}
	if graph.HasNode("b") || graph.HasEdge(incoming) || graph.HasEdge(outgoing) {
		t.Fatalf("node or incident edges remain: nodes=%v edges=%v", graph.Nodes(), graph.Edges())
	}
}

func TestTopologicalSortAndIndependentGroupsAccountForParallelEdges(t *testing.T) {
	graph := New[string]()
	graph.AddEdge("a", "c", 1, nil, "")
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

func TestCycleErrorsIncludeProgress(t *testing.T) {
	graph := New[string]()
	graph.AddNode("ready", nil)
	graph.AddEdge("a", "b", 1, nil, "")
	graph.AddEdge("b", "a", 1, nil, "")

	_, err := graph.TopologicalSort()
	var cycle CycleError
	if !errors.As(err, &cycle) || cycle.Processed != 1 || cycle.Total != 3 {
		t.Fatalf("topological cycle error = %#v", err)
	}
	_, err = graph.IndependentGroups()
	if !errors.As(err, &cycle) || !graph.HasCycle() {
		t.Fatalf("independent groups cycle error = %#v", err)
	}
}

func TestSupportsGenericNodeValues(t *testing.T) {
	graph := New[int]()
	if graph.Size() != 0 {
		t.Fatalf("new graph size = %d", graph.Size())
	}
	graph.AddEdge(2, 3, 1, nil, "")
	graph.AddNode(1, nil)

	order, err := graph.TopologicalSort()
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(order, []int{1, 2, 3}) {
		t.Fatalf("generic node order = %v", order)
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
