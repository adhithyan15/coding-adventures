package graph

import (
	"fmt"
	"math/rand"
	"reflect"
	"slices"
	"sort"
	"sync"
	"sync/atomic"
	"testing"
)

func TestConcurrentGraphSnapshotIsDetachedAndHasIndependentIterators(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			input := PropertyBag{"kind": "road", "open": true, "lanes": 2}
			if err := g.AddEdge("A", "B", 3, input); err != nil {
				t.Fatal(err)
			}
			input["kind"] = "changed by caller"
			if err := g.SetGraphProperty("name", "snapshot source"); err != nil {
				t.Fatal(err)
			}

			snapshot := g.Snapshot()
			props, err := snapshot.EdgeProperties("A", "B")
			if err != nil || props["kind"] != "road" {
				t.Fatalf("snapshot edge properties = %#v, %v", props, err)
			}
			props["kind"] = "changed through query result"
			if got, _ := snapshot.EdgeProperties("B", "A"); got["kind"] != "road" {
				t.Fatalf("snapshot returned aliased edge properties: %#v", got)
			}
			graphProps := snapshot.GraphProperties()
			graphProps["name"] = "changed through query result"
			if got := snapshot.GraphProperties()["name"]; got != "snapshot source" {
				t.Fatalf("snapshot returned aliased graph properties: %#v", got)
			}

			first := snapshot.EdgesIterator()
			second := snapshot.EdgesIterator()
			if edge, ok := first.Next(); !ok || edge.Left != "A" || edge.Right != "B" {
				t.Fatalf("first iterator edge = %#v, %v", edge, ok)
			}
			if edge, ok := second.Next(); !ok || edge.Left != "A" || edge.Right != "B" {
				t.Fatalf("second iterator did not own an independent cursor: %#v, %v", edge, ok)
			}
			if _, ok := first.Next(); ok {
				t.Fatal("iterator should be exhausted")
			}
			if _, ok := first.Next(); ok {
				t.Fatal("exhausted iterator should remain exhausted")
			}

			if err := g.RemoveEdge("A", "B"); err != nil {
				t.Fatal(err)
			}
			if !snapshot.HasEdge("A", "B") || g.HasEdge("A", "B") {
				t.Fatal("snapshot changed after source mutation or source did not change")
			}
		})
	}
}

func TestConcurrentGraphRejectsReferencePropertiesWithoutMutation(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			if err := g.AddNode("A"); err != nil {
				t.Fatal(err)
			}
			if err := g.AddNode("B"); err != nil {
				t.Fatal(err)
			}
			before := g.Snapshot()
			if err := g.AddNode("rejected", PropertyBag{"bad": []int{1}}); err == nil {
				t.Fatal("AddNode accepted a slice-valued property")
			}
			shared := map[string]string{"mutable": "value"}
			if err := g.AddEdge("A", "B", 2, PropertyBag{"bad": shared}); err == nil {
				t.Fatal("AddEdge accepted a map-valued property")
			}
			if err := g.SetGraphProperty("bad", []int{1}); err == nil {
				t.Fatal("SetGraphProperty accepted a slice-valued property")
			}
			after := g.Snapshot()
			if !reflect.DeepEqual(before.Nodes(), after.Nodes()) || !reflect.DeepEqual(before.Edges(), after.Edges()) {
				t.Fatalf("rejected mutation changed graph: before=%v/%v after=%v/%v", before.Nodes(), before.Edges(), after.Nodes(), after.Edges())
			}
			if g.HasEdge("A", "B") {
				t.Fatal("rejected edge insertion left an edge behind")
			}
		})
	}
}

func TestConcurrentGraphPropertyAndSnapshotQueries(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			if g.Repr() != repr {
				t.Fatalf("Repr() = %s, want %s", g.Repr(), repr)
			}
			if err := g.AddNode("A", PropertyBag{"kind": "town", "open": true}); err != nil {
				t.Fatal(err)
			}
			if err := g.AddNode("B"); err != nil {
				t.Fatal(err)
			}
			if err := g.AddNode("C"); err != nil {
				t.Fatal(err)
			}
			if err := g.SetNodeProperty("A", "population", 12); err != nil {
				t.Fatal(err)
			}
			if err := g.SetNodeProperty("A", "invalid", map[string]int{"x": 1}); err == nil {
				t.Fatal("SetNodeProperty accepted a map-valued property")
			}
			nodeProperties, err := g.NodeProperties("A")
			if err != nil || nodeProperties["population"] != 12 || nodeProperties["kind"] != "town" {
				t.Fatalf("NodeProperties(A) = %#v, %v", nodeProperties, err)
			}
			nodeProperties["kind"] = "external mutation"
			if got, _ := g.NodeProperties("A"); got["kind"] != "town" {
				t.Fatalf("node property read aliases owned state: %#v", got)
			}
			if err := g.RemoveNodeProperty("A", "population"); err != nil {
				t.Fatal(err)
			}

			if err := g.AddEdge("A", "B", 2, PropertyBag{"route": "local"}); err != nil {
				t.Fatal(err)
			}
			if err := g.AddEdge("B", "C", 3); err != nil {
				t.Fatal(err)
			}
			if err := g.AddEdge("A", "C", 10); err != nil {
				t.Fatal(err)
			}
			if !g.HasNode("A") || !reflect.DeepEqual(g.Nodes(), []string{"A", "B", "C"}) {
				t.Fatalf("concurrent node queries: has A=%v, nodes=%v", g.HasNode("A"), g.Nodes())
			}
			if weight, err := g.EdgeWeight("A", "B"); err != nil || weight != 2 {
				t.Fatalf("EdgeWeight(A,B) = %v, %v", weight, err)
			}
			if edges := g.Edges(); len(edges) != 3 {
				t.Fatalf("Edges() returned %d entries, want 3", len(edges))
			}
			edgeProperties, err := g.EdgeProperties("A", "B")
			if err != nil || edgeProperties["route"] != "local" {
				t.Fatalf("EdgeProperties(A,B) = %#v, %v", edgeProperties, err)
			}
			edgeProperties["route"] = "external mutation"
			if got, _ := g.EdgeProperties("A", "B"); got["route"] != "local" {
				t.Fatalf("edge property read aliases owned state: %#v", got)
			}
			if err := g.SetEdgeProperty("A", "B", "lanes", uint8(2)); err != nil {
				t.Fatal(err)
			}
			if err := g.SetEdgeProperty("A", "B", "invalid", []string{"x"}); err == nil {
				t.Fatal("SetEdgeProperty accepted a slice-valued property")
			}
			if err := g.SetEdgeProperty("B", "C", "weight", float64(4)); err != nil {
				t.Fatal(err)
			}
			if err := g.RemoveEdgeProperty("A", "B", "lanes"); err != nil {
				t.Fatal(err)
			}
			if err := g.RemoveEdgeProperty("B", "C", "weight"); err != nil {
				t.Fatal(err)
			}
			if degree, err := g.Degree("B"); err != nil || degree != 2 {
				t.Fatalf("Degree(B) = %d, %v", degree, err)
			}
			if neighbors, err := g.Neighbors("B"); err != nil || !reflect.DeepEqual(neighbors, []string{"A", "C"}) {
				t.Fatalf("Neighbors(B) = %v, %v", neighbors, err)
			}
			if neighbors, err := g.NeighborsWeighted("B"); err != nil || neighbors["A"] != 2 || neighbors["C"] != 1 {
				t.Fatalf("NeighborsWeighted(B) = %v, %v", neighbors, err)
			}
			if err := g.SetGraphProperty("name", "test-map"); err != nil {
				t.Fatal(err)
			}
			if err := g.SetGraphProperty("optional", nil); err != nil {
				t.Fatal(err)
			}
			if got := g.GraphProperties(); got["name"] != "test-map" {
				t.Fatalf("GraphProperties() = %#v", got)
			}
			if got := g.GraphProperties(); got["optional"] != nil {
				t.Fatalf("GraphProperties() nil scalar = %#v", got)
			}
			if err := g.SetGraphProperty("invalid", []int{1}); err == nil {
				t.Fatal("SetGraphProperty accepted a reference value")
			}
			g.RemoveGraphProperty("name")

			snapshot := g.Snapshot()
			if snapshot.Repr() != repr || snapshot.Size() != 3 || snapshot.String() == "" {
				t.Fatalf("snapshot metadata: repr=%s size=%d string=%q", snapshot.Repr(), snapshot.Size(), snapshot.String())
			}
			if !snapshot.HasNode("A") || !snapshot.HasEdge("A", "B") || len(snapshot.Edges()) != 3 {
				t.Fatal("snapshot query methods lost graph state")
			}
			if neighbors, err := snapshot.Neighbors("B"); err != nil || !reflect.DeepEqual(neighbors, []string{"A", "C"}) {
				t.Fatalf("snapshot Neighbors(B) = %v, %v", neighbors, err)
			}
			if degree, err := snapshot.Degree("B"); err != nil || degree != 2 {
				t.Fatalf("snapshot Degree(B) = %d, %v", degree, err)
			}
			if got, err := snapshot.EdgeProperties("A", "B"); err != nil || got["route"] != "local" || got["weight"] != float64(2) {
				t.Fatalf("snapshot EdgeProperties(A,B) = %#v, %v", got, err)
			}
			if got, err := snapshot.NodeProperties("A"); err != nil || got["kind"] != "town" {
				t.Fatalf("snapshot NodeProperties(A) = %#v, %v", got, err)
			}
			if path, err := snapshot.ShortestPath("A", "C"); err != nil || !reflect.DeepEqual(path, []string{"A", "B", "C"}) {
				t.Fatalf("snapshot ShortestPath(A,C) = %v, %v", path, err)
			}
			if tree, err := snapshot.MinimumSpanningTree(); err != nil || len(tree) != 2 {
				t.Fatalf("snapshot MinimumSpanningTree() = %v, %v", tree, err)
			}
			iterator := snapshot.EdgesIterator()
			if _, ok := iterator.Next(); !ok {
				t.Fatal("snapshot iterator unexpectedly empty")
			}
			iteratorCopy := *iterator
			second, secondOK := iterator.Next()
			copiedSecond, copiedSecondOK := iteratorCopy.Next()
			if !secondOK || !copiedSecondOK || second != copiedSecond {
				t.Fatalf("iterator copy did not fork its cursor: original=%#v/%v copy=%#v/%v", second, secondOK, copiedSecond, copiedSecondOK)
			}
			seen := 2
			for {
				if _, ok := iterator.Next(); !ok {
					break
				}
				seen++
			}
			copiedCount := 2
			for {
				if _, ok := iteratorCopy.Next(); !ok {
					break
				}
				copiedCount++
			}
			if seen != 3 {
				t.Fatalf("snapshot iterator returned %d edges, want 3", seen)
			}
			if copiedCount != 3 {
				t.Fatalf("copied snapshot iterator returned %d edges, want 3", copiedCount)
			}
			if size, value := g.Size(), g.String(); size != 3 || value == "" {
				t.Fatalf("concurrent graph queries: size=%d string=%q", size, value)
			}
			if err := g.RemoveNode("A"); err != nil || snapshot.Size() != 3 {
				t.Fatalf("source mutation affected snapshot: remove err=%v size=%d", err, snapshot.Size())
			}
		})
	}
}

func TestConcurrentGraphSnapshotsNeverObserveHalfAnUndirectedEdge(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			const iterations = 400
			var readers sync.WaitGroup
			start := make(chan struct{})
			readers.Add(4)
			for r := 0; r < 4; r++ {
				go func() {
					defer readers.Done()
					<-start
					for n := 0; n < iterations; n++ {
						checkSnapshotSymmetry(t, g.Snapshot())
					}
				}()
			}
			close(start)
			for n := 0; n < iterations; n++ {
				props := PropertyBag{"generation": n}
				if err := g.AddEdge("left", "right", float64(n+1), props); err != nil {
					t.Fatal(err)
				}
				if err := g.RemoveEdge("left", "right"); err != nil {
					t.Fatal(err)
				}
			}
			readers.Wait()
		})
	}
}

func checkSnapshotSymmetry(t *testing.T, snapshot *GraphSnapshot) {
	t.Helper()
	for _, edge := range snapshot.Edges() {
		if !snapshot.HasEdge(edge.Left, edge.Right) || !snapshot.HasEdge(edge.Right, edge.Left) {
			t.Fatalf("snapshot contains one-sided edge: %#v", edge)
		}
		forward, err := snapshot.EdgeWeight(edge.Left, edge.Right)
		if err != nil {
			t.Fatalf("forward weight for %#v: %v", edge, err)
		}
		reverse, err := snapshot.EdgeWeight(edge.Right, edge.Left)
		if err != nil || forward != reverse || forward != edge.Weight {
			t.Fatalf("inconsistent edge snapshot %#v: forward=%v reverse=%v err=%v", edge, forward, reverse, err)
		}
		props, err := snapshot.EdgeProperties(edge.Left, edge.Right)
		if err != nil {
			t.Fatal(err)
		}
		if props["weight"] != edge.Weight {
			t.Fatalf("edge metadata disagrees with topology: %#v", props)
		}
		if generation, ok := props["generation"].(int); ok && float64(generation+1) != edge.Weight {
			t.Fatalf("snapshot mixed generations: weight=%v properties=%#v", edge.Weight, props)
		}
		if weightCopy, ok := props["weightCopy"].(float64); ok && weightCopy != edge.Weight {
			t.Fatalf("snapshot mixed edge weight and property: weight=%v properties=%#v", edge.Weight, props)
		}
	}
}

func TestConcurrentGraphSerializesSameEdgeMutations(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			var workers sync.WaitGroup
			start := make(chan struct{})
			for worker := 0; worker < 8; worker++ {
				workers.Add(1)
				go func(worker int) {
					defer workers.Done()
					<-start
					for iteration := 0; iteration < 75; iteration++ {
						weight := float64(worker*100 + iteration + 1)
						if err := g.AddEdge("same-left", "same-right", weight, PropertyBag{"worker": worker, "weightCopy": weight}); err != nil {
							t.Errorf("AddEdge: %v", err)
							return
						}
						snapshot := g.Snapshot()
						checkSnapshotSymmetry(t, snapshot)
						if got, err := snapshot.EdgeWeight("same-left", "same-right"); err != nil || got < 1 {
							t.Errorf("snapshot weight = %v, %v", got, err)
							return
						}
					}
				}(worker)
			}
			close(start)
			workers.Wait()
			if edges := g.Snapshot().Edges(); len(edges) != 1 {
				t.Fatalf("final edges = %s", fmt.Sprint(edges))
			}
		})
	}
}

func TestConcurrentGraphValueCopiesShareTheOwnerAndMutex(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			original := NewConcurrentGraph(repr)
			copied := *original
			const workers = 8
			const operations = 100
			start := make(chan struct{})
			var group sync.WaitGroup
			for worker := 0; worker < workers; worker++ {
				group.Add(1)
				go func(worker int) {
					defer group.Done()
					<-start
					owner := original
					if worker%2 != 0 {
						owner = &copied
					}
					for operation := 0; operation < operations; operation++ {
						left := fmt.Sprintf("worker-%d-node-%d", worker, operation)
						weight := float64(worker*operations + operation + 1)
						if err := owner.AddEdge(left, "center", weight, PropertyBag{"weightCopy": weight}); err != nil {
							t.Errorf("AddEdge through copied handle: %v", err)
							return
						}
					}
				}(worker)
			}
			group.Add(2)
			for reader := 0; reader < 2; reader++ {
				go func(reader int) {
					defer group.Done()
					<-start
					owner := original
					if reader == 1 {
						owner = &copied
					}
					for operation := 0; operation < operations; operation++ {
						checkSnapshotSymmetry(t, owner.Snapshot())
					}
				}(reader)
			}
			close(start)
			group.Wait()

			wantEdges := workers * operations
			if got := len(original.Snapshot().Edges()); got != wantEdges {
				t.Fatalf("original sees %d edges, want %d", got, wantEdges)
			}
			if got := len(copied.Snapshot().Edges()); got != wantEdges {
				t.Fatalf("copied handle sees %d edges, want %d", got, wantEdges)
			}
			if original.Size() != wantEdges+1 || copied.Size() != wantEdges+1 {
				t.Fatalf("handles do not share final graph state: original=%d copied=%d", original.Size(), copied.Size())
			}
		})
	}
}

func TestConcurrentGraphPreservesSequentialOrderForCompletedCalls(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			if err := g.AddEdge("A", "B", 1); err != nil {
				t.Fatal(err)
			}
			// This read begins only after AddEdge returned, so its result must
			// reflect that completed mutation in real-time order.
			if !g.HasEdge("B", "A") {
				t.Fatal("read following completed AddEdge lost its real-time order")
			}
			if err := g.RemoveEdge("A", "B"); err != nil {
				t.Fatal(err)
			}
			if g.HasEdge("B", "A") {
				t.Fatal("read following completed RemoveEdge lost its real-time order")
			}
		})
	}
}

func TestConcurrentGraphTwoOperationHistoriesAreLinearizable(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			for round := 0; round < 250; round++ {
				initiallyPresent := round%2 == 1
				g := NewConcurrentGraph(repr)
				if initiallyPresent {
					if err := g.AddEdge("A", "B", 7); err != nil {
						t.Fatal(err)
					}
				}
				var clock atomic.Int64
				start := make(chan struct{})
				var workers sync.WaitGroup
				history := [2]operationRecord{{kind: operationAdd}, {kind: operationRemove}}
				workers.Add(2)
				go func() {
					defer workers.Done()
					<-start
					history[0].invoked = clock.Add(1)
					history[0].err = g.AddEdge("A", "B", 11)
					history[0].returned = clock.Add(1)
				}()
				go func() {
					defer workers.Done()
					<-start
					history[1].invoked = clock.Add(1)
					history[1].err = g.RemoveEdge("A", "B")
					history[1].returned = clock.Add(1)
				}()
				close(start)
				workers.Wait()
				finalPresent := g.HasEdge("A", "B")
				if !linearizableTwoOperationHistory(initiallyPresent, finalPresent, history) {
					t.Fatalf("non-linearizable history: initial=%v final=%v history=%+v", initiallyPresent, finalPresent, history)
				}
			}
		})
	}
}

func TestTwoOperationHistoryOracleRejectsImpossibleNegativeControl(t *testing.T) {
	addThenRemove := [2]operationRecord{
		{kind: operationAdd, invoked: 1, returned: 2},
		{kind: operationRemove, invoked: 3, returned: 4},
	}
	if !linearizableTwoOperationHistory(false, false, addThenRemove) {
		t.Fatal("valid add-then-remove control rejected")
	}
	if linearizableTwoOperationHistory(false, true, addThenRemove) {
		t.Fatal("oracle accepted impossible final-present state after a completed remove")
	}
}

type operationKind uint8

const (
	operationAdd operationKind = iota
	operationRemove
)

type operationRecord struct {
	kind     operationKind
	invoked  int64
	returned int64
	err      error
}

func linearizableTwoOperationHistory(initialPresent, finalPresent bool, history [2]operationRecord) bool {
	orders := [][2]int{{0, 1}, {1, 0}}
	for _, order := range orders {
		first, second := history[order[0]], history[order[1]]
		if first.returned < second.invoked {
			// This order is consistent with real-time precedence.
		} else if second.returned < first.invoked {
			continue
		}
		present := initialPresent
		matches := true
		for _, index := range order {
			operation := history[index]
			switch operation.kind {
			case operationAdd:
				if operation.err != nil {
					matches = false
				}
				present = true
			case operationRemove:
				if present {
					if operation.err != nil {
						matches = false
					}
				} else if operation.err == nil {
					matches = false
				}
				present = false
			}
		}
		if matches && present == finalPresent {
			return true
		}
	}
	return false
}

func TestConcurrentGraphSeededModelHistories(t *testing.T) {
	const seeds = 24
	const operationsPerSeed = 160
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			for seed := int64(0); seed < seeds; seed++ {
				g := NewConcurrentGraph(repr)
				model := newGraphModel()
				rng := rand.New(rand.NewSource(seed))
				for step := 0; step < operationsPerSeed; step++ {
					operation := byte(rng.Intn(4))
					left := fmt.Sprintf("n%d", rng.Intn(4))
					right := fmt.Sprintf("n%d", rng.Intn(4))
					applyGraphModelOperation(t, g, model, operation, left, right, float64(rng.Intn(5)))
					assertGraphMatchesModel(t, g.Snapshot(), model)
				}
			}
		})
	}
}

func FuzzConcurrentGraphModelHistories(f *testing.F) {
	f.Add([]byte{0, 2, 3, 1, 2, 2, 3, 0, 1})
	f.Add([]byte{2, 2, 2, 1, 3, 3, 0, 0})
	f.Add([]byte{})
	f.Fuzz(func(t *testing.T, trace []byte) {
		if len(trace) > 128 {
			trace = trace[:128]
		}
		for _, repr := range representations() {
			g := NewConcurrentGraph(repr)
			model := newGraphModel()
			for index, encoded := range trace {
				operation := encoded % 4
				left := fmt.Sprintf("n%d", (encoded>>2)&3)
				right := fmt.Sprintf("n%d", (encoded>>4)&3)
				weight := float64((int(encoded) + index) % 5)
				applyGraphModelOperation(t, g, model, operation, left, right, weight)
				assertGraphMatchesModel(t, g.Snapshot(), model)
			}
		}
	})
}

type graphModel struct {
	nodeOrder []string
	nodes     map[string]struct{}
	edges     map[modelEdgeKey]modelEdge
}

type modelEdgeKey struct {
	left  string
	right string
}

func modelEdgeIdentity(left, right string) modelEdgeKey {
	if right < left {
		left, right = right, left
	}
	return modelEdgeKey{left: left, right: right}
}

type modelEdge struct {
	left, right string
	weight      float64
}

func newGraphModel() *graphModel {
	return &graphModel{nodes: make(map[string]struct{}), edges: make(map[modelEdgeKey]modelEdge)}
}

func (m *graphModel) addNode(node string) {
	if _, exists := m.nodes[node]; exists {
		return
	}
	m.nodes[node] = struct{}{}
	m.nodeOrder = append(m.nodeOrder, node)
}

func (m *graphModel) removeNode(node string) bool {
	if _, exists := m.nodes[node]; !exists {
		return false
	}
	delete(m.nodes, node)
	for index, existing := range m.nodeOrder {
		if existing == node {
			m.nodeOrder = append(m.nodeOrder[:index], m.nodeOrder[index+1:]...)
			break
		}
	}
	for key, edge := range m.edges {
		if edge.left == node || edge.right == node {
			delete(m.edges, key)
		}
	}
	return true
}

func (m *graphModel) addEdge(left, right string, weight float64) {
	m.addNode(left)
	m.addNode(right)
	if weight == 0 {
		weight = 1
	}
	key := modelEdgeIdentity(left, right)
	m.edges[key] = modelEdge{left: left, right: right, weight: weight}
}

func (m *graphModel) removeEdge(left, right string) bool {
	key := modelEdgeIdentity(left, right)
	if _, exists := m.edges[key]; !exists {
		return false
	}
	delete(m.edges, key)
	return true
}

func applyGraphModelOperation(t *testing.T, graph *ConcurrentGraph, model *graphModel, operation byte, left, right string, weight float64) {
	t.Helper()
	switch operation % 4 {
	case 0:
		if err := graph.AddNode(left); err != nil {
			t.Fatal(err)
		}
		model.addNode(left)
	case 1:
		removed := model.removeNode(left)
		err := graph.RemoveNode(left)
		if (err == nil) != removed {
			t.Fatalf("RemoveNode(%q) error = %v, model found node=%v", left, err, removed)
		}
	case 2:
		if err := graph.AddEdge(left, right, weight); err != nil {
			t.Fatal(err)
		}
		model.addEdge(left, right, weight)
	case 3:
		removed := model.removeEdge(left, right)
		err := graph.RemoveEdge(left, right)
		if (err == nil) != removed {
			t.Fatalf("RemoveEdge(%q,%q) error = %v, model found edge=%v", left, right, err, removed)
		}
	}
}

func assertGraphMatchesModel(t *testing.T, snapshot *GraphSnapshot, model *graphModel) {
	t.Helper()
	wantNodes := make([]string, len(model.nodeOrder))
	copy(wantNodes, model.nodeOrder)
	if snapshot.Repr() == AdjacencyList {
		sort.Strings(wantNodes)
	}
	if got := snapshot.Nodes(); !slices.Equal(got, wantNodes) {
		t.Fatalf("Nodes() = %v, model wants %v", got, wantNodes)
	}
	if snapshot.Size() != len(model.nodes) {
		t.Fatalf("Size() = %d, model wants %d", snapshot.Size(), len(model.nodes))
	}
	gotEdges := snapshot.Edges()
	if len(gotEdges) != len(model.edges) {
		t.Fatalf("Edges() has %d edges, model wants %d: %v", len(gotEdges), len(model.edges), gotEdges)
	}
	for key, want := range model.edges {
		weight, err := snapshot.EdgeWeight(want.left, want.right)
		if err != nil || weight != want.weight {
			t.Fatalf("EdgeWeight(%q,%q) = %v, %v; model wants %v", want.left, want.right, weight, err, want.weight)
		}
		if !snapshot.HasEdge(want.left, want.right) || !snapshot.HasEdge(want.right, want.left) {
			t.Fatalf("model edge %#v is not symmetric", key)
		}
		neighbors, err := snapshot.NeighborsWeighted(want.left)
		if err != nil || neighbors[want.right] != want.weight {
			t.Fatalf("NeighborsWeighted(%q) = %v, %v; model wants %q=%v", want.left, neighbors, err, want.right, want.weight)
		}
	}
}
