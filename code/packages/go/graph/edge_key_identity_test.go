package graph

import (
	"fmt"
	"testing"
)

var identityTestNames = []string{"a", "b\x00c", "a\x00b", "c", "left", "right"}

type identityModelPair struct {
	left  string
	right string
}

type identityModelEdge struct {
	weight   float64
	label    string
	hasLabel bool
}

type edgeIdentityModel struct {
	nodes map[string]struct{}
	edges map[identityModelPair]identityModelEdge
}

func newEdgeIdentityModel() *edgeIdentityModel {
	return &edgeIdentityModel{
		nodes: make(map[string]struct{}),
		edges: make(map[identityModelPair]identityModelEdge),
	}
}

// modelPairKey uses a comparable pair in the test oracle. It deliberately
// does not call the production edgeKey helper.
func modelPairKey(left, right string) identityModelPair {
	if right < left {
		left, right = right, left
	}
	return identityModelPair{left: left, right: right}
}

func modelAddEdge(model *edgeIdentityModel, left, right string, weight float64, label string) {
	model.nodes[left] = struct{}{}
	model.nodes[right] = struct{}{}
	model.edges[modelPairKey(left, right)] = identityModelEdge{
		weight:   weight,
		label:    label,
		hasLabel: true,
	}
}

func assertGraphMatchesEdgeIdentityModel(t *testing.T, g *Graph, model *edgeIdentityModel) {
	t.Helper()
	if g.Size() != len(model.nodes) {
		t.Fatalf("Size() = %d, model has %d nodes", g.Size(), len(model.nodes))
	}
	actualNodes := make(map[string]struct{})
	for _, node := range g.Nodes() {
		actualNodes[node] = struct{}{}
	}
	if len(actualNodes) != len(model.nodes) {
		t.Fatalf("Nodes() = %#v, model has %#v", actualNodes, model.nodes)
	}
	for node := range model.nodes {
		if _, ok := actualNodes[node]; !ok {
			t.Fatalf("Nodes() is missing %q", node)
		}
	}

	actualEdges := g.Edges()
	if len(actualEdges) != len(model.edges) {
		t.Fatalf("Edges() length = %d, model has %d edges: %#v", len(actualEdges), len(model.edges), actualEdges)
	}
	seen := make(map[identityModelPair]struct{}, len(actualEdges))
	for _, edge := range actualEdges {
		key := modelPairKey(edge.Left, edge.Right)
		want, ok := model.edges[key]
		if !ok {
			t.Fatalf("Edges() contains unexpected edge %q-%q", edge.Left, edge.Right)
		}
		if _, duplicate := seen[key]; duplicate {
			t.Fatalf("Edges() contains duplicate pair %#v", key)
		}
		seen[key] = struct{}{}
		if edge.Weight != want.weight {
			t.Fatalf("edge %q-%q weight = %g, model wants %g", edge.Left, edge.Right, edge.Weight, want.weight)
		}
	}

	for _, left := range identityTestNames {
		for _, right := range identityTestNames {
			if left == right {
				continue
			}
			key := modelPairKey(left, right)
			want, exists := model.edges[key]
			if g.HasEdge(left, right) != exists {
				t.Fatalf("HasEdge(%q,%q) = %t, model wants %t", left, right, g.HasEdge(left, right), exists)
			}
			if !exists {
				continue
			}
			weight, err := g.EdgeWeight(left, right)
			if err != nil || weight != want.weight {
				t.Fatalf("EdgeWeight(%q,%q) = %g, %v; model wants %g", left, right, weight, err, want.weight)
			}
			properties, err := g.EdgeProperties(left, right)
			if err != nil || properties["weight"] != want.weight {
				t.Fatalf("EdgeProperties(%q,%q) = %#v, %v; bad weight", left, right, properties, err)
			}
			label, hasLabel := properties["label"]
			if hasLabel != want.hasLabel || (hasLabel && label != want.label) {
				t.Fatalf("EdgeProperties(%q,%q).label = %#v/%t, model wants %#v/%t", left, right, label, hasLabel, want.label, want.hasLabel)
			}
		}
	}
}

func addDelimiterCollisionEdges(g *Graph) {
	g.AddEdge("a", "b\x00c", 1, PropertyBag{"owner": "first", "note": "one"})
	g.AddEdge("c", "a\x00b", 2, PropertyBag{"owner": "second", "note": "two"})
}

func TestEdgePropertiesUseDistinctUnorderedEndpointPairs(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := New(repr)
			addDelimiterCollisionEdges(g)
			if !g.HasEdge("a", "b\x00c") || !g.HasEdge("a\x00b", "c") {
				t.Fatal("distinct NUL-containing endpoint pairs must both exist")
			}
			if len(g.Edges()) != 2 {
				t.Fatalf("Edges() = %#v, want both distinct pairs", g.Edges())
			}
			for _, endpoints := range [][2]string{
				{"a", "b\x00c"}, {"b\x00c", "a"},
				{"a\x00b", "c"}, {"c", "a\x00b"},
			} {
				properties, err := g.EdgeProperties(endpoints[0], endpoints[1])
				if err != nil {
					t.Fatalf("EdgeProperties(%q,%q): %v", endpoints[0], endpoints[1], err)
				}
				wantOwner := "first"
				if endpoints[0] == "a\x00b" || endpoints[1] == "a\x00b" {
					wantOwner = "second"
				}
				if properties["owner"] != wantOwner {
					t.Fatalf("EdgeProperties(%q,%q).owner = %#v, want %q", endpoints[0], endpoints[1], properties["owner"], wantOwner)
				}
			}

			if err := g.SetEdgeProperty("a", "b\x00c", "owner", "updated-first"); err != nil {
				t.Fatal(err)
			}
			first, _ := g.EdgeProperties("b\x00c", "a")
			second, _ := g.EdgeProperties("c", "a\x00b")
			if first["owner"] != "updated-first" || second["owner"] != "second" {
				t.Fatalf("property update crossed edge identities: first=%#v second=%#v", first, second)
			}
			if err := g.RemoveEdgeProperty("b\x00c", "a", "note"); err != nil {
				t.Fatal(err)
			}
			first, _ = g.EdgeProperties("a", "b\x00c")
			second, _ = g.EdgeProperties("a\x00b", "c")
			if _, ok := first["note"]; ok || second["note"] != "two" {
				t.Fatalf("property delete crossed edge identities: first=%#v second=%#v", first, second)
			}
			if err := g.SetEdgeProperty("a\x00b", "c", "survivor", "yes"); err != nil {
				t.Fatal(err)
			}
			if err := g.RemoveEdge("b\x00c", "a"); err != nil {
				t.Fatal(err)
			}
			second, err := g.EdgeProperties("a\x00b", "c")
			if err != nil || !g.HasEdge("a\x00b", "c") || second["owner"] != "second" || second["survivor"] != "yes" {
				t.Fatalf("removing first edge damaged second: edge=%t properties=%#v err=%v", g.HasEdge("a\x00b", "c"), second, err)
			}
			if len(g.Edges()) != 1 {
				t.Fatalf("remaining Edges() = %#v, want one", g.Edges())
			}
		})
	}
}

func TestRemovingNodePreservesPropertiesOfDistinctNULPair(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := New(repr)
			addDelimiterCollisionEdges(g)
			if err := g.RemoveNode("a"); err != nil {
				t.Fatal(err)
			}
			properties, err := g.EdgeProperties("a\x00b", "c")
			if err != nil || !g.HasEdge("a\x00b", "c") || properties["owner"] != "second" {
				t.Fatalf("RemoveNode(%q) damaged distinct edge: exists=%t properties=%#v err=%v", "a", g.HasEdge("a\x00b", "c"), properties, err)
			}
		})
	}
}

func TestConcurrentSnapshotPreservesDistinctEdgeIdentityAndDetachment(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			for _, edge := range []struct {
				left, right string
				weight      float64
				owner       string
			}{
				{"a", "b\x00c", 1, "first"},
				{"c", "a\x00b", 2, "second"},
			} {
				if err := g.AddEdge(edge.left, edge.right, edge.weight, PropertyBag{"owner": edge.owner}); err != nil {
					t.Fatal(err)
				}
			}
			snapshot := g.Snapshot()
			if len(snapshot.Edges()) != 2 {
				t.Fatalf("snapshot Edges() = %#v, want both edge pairs", snapshot.Edges())
			}
			iterator := snapshot.EdgesIterator()
			count := 0
			for {
				_, ok := iterator.Next()
				if !ok {
					break
				}
				count++
			}
			if count != 2 {
				t.Fatalf("snapshot iterator produced %d edges, want 2", count)
			}
			if err := g.SetEdgeProperty("a", "b\x00c", "owner", "updated-first"); err != nil {
				t.Fatal(err)
			}
			if err := g.RemoveEdge("a", "b\x00c"); err != nil {
				t.Fatal(err)
			}
			current, err := g.EdgeProperties("a\x00b", "c")
			old, oldErr := snapshot.EdgeProperties("a\x00b", "c")
			if err != nil || oldErr != nil || current["owner"] != "second" || old["owner"] != "second" {
				t.Fatalf("current/snapshot second-edge properties = %#v/%v, %#v/%v", current, err, old, oldErr)
			}
			oldFirst, oldErr := snapshot.EdgeProperties("b\x00c", "a")
			if oldErr != nil || oldFirst["owner"] != "first" || len(snapshot.Edges()) != 2 {
				t.Fatalf("detached snapshot changed: first=%#v err=%v edges=%#v", oldFirst, oldErr, snapshot.Edges())
			}
		})
	}
}

func TestEdgeIdentityIndependentBoundedModel(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := New(repr)
			model := newEdgeIdentityModel()
			for index, left := range identityTestNames {
				for _, right := range identityTestNames[index+1:] {
					weight := float64(index + 1)
					label := fmt.Sprintf("edge-%d-%d", index, len(model.edges))
					g.AddEdge(left, right, weight, PropertyBag{"label": label})
					modelAddEdge(model, left, right, weight, label)
				}
			}
			assertGraphMatchesEdgeIdentityModel(t, g, model)
		})
	}
}

func FuzzEdgeIdentityModel(f *testing.F) {
	f.Add([]byte{0, 0, 0, 1, 0, 0, 2, 3, 1})
	f.Add([]byte{1, 0, 0, 1, 0, 0, 2, 3, 1})
	f.Add([]byte{0, 5, 4, 2, 7, 2, 3, 1, 4, 1, 0, 3})

	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 257 {
			data = data[:257]
		}
		if len(data) == 0 {
			return
		}
		repr := representations()[int(data[0]%2)]
		g := New(repr)
		model := newEdgeIdentityModel()
		trace := data[1:]
		for offset := 0; offset+3 < len(trace); offset += 4 {
			encoded := trace[offset : offset+4]
			operation := encoded[0] % 6
			left := identityTestNames[int(encoded[1])%len(identityTestNames)]
			right := identityTestNames[int(encoded[2])%len(identityTestNames)]
			if left == right {
				right = identityTestNames[(int(encoded[2])+1)%len(identityTestNames)]
			}
			pair := modelPairKey(left, right)
			edge, exists := model.edges[pair]
			label := fmt.Sprintf("v%d", encoded[3]%8)
			switch operation {
			case 0: // add or replace one undirected edge
				weight := 1 + float64(encoded[3]%11)
				g.AddEdge(left, right, weight, PropertyBag{"label": label})
				modelAddEdge(model, left, right, weight, label)
			case 1: // update edge metadata
				err := g.SetEdgeProperty(left, right, "label", label)
				if exists {
					if err != nil {
						t.Fatalf("SetEdgeProperty existing edge: %v", err)
					}
					edge.label, edge.hasLabel = label, true
					model.edges[pair] = edge
				} else if err == nil {
					t.Fatal("SetEdgeProperty missing edge unexpectedly succeeded")
				}
			case 2: // delete one metadata key
				err := g.RemoveEdgeProperty(left, right, "label")
				if exists {
					if err != nil {
						t.Fatalf("RemoveEdgeProperty existing edge: %v", err)
					}
					edge.label, edge.hasLabel = "", false
					model.edges[pair] = edge
				} else if err == nil {
					t.Fatal("RemoveEdgeProperty missing edge unexpectedly succeeded")
				}
			case 3: // delete the edge
				err := g.RemoveEdge(left, right)
				if exists {
					if err != nil {
						t.Fatalf("RemoveEdge existing edge: %v", err)
					}
					delete(model.edges, pair)
				} else if err == nil {
					t.Fatal("RemoveEdge missing edge unexpectedly succeeded")
				}
			case 4: // delete a node and every incident edge
				nodeExists := false
				if _, nodeExists = model.nodes[left]; nodeExists {
					if err := g.RemoveNode(left); err != nil {
						t.Fatalf("RemoveNode existing node: %v", err)
					}
					delete(model.nodes, left)
					for key := range model.edges {
						if key.left == left || key.right == left {
							delete(model.edges, key)
						}
					}
				} else if err := g.RemoveNode(left); err == nil {
					t.Fatal("RemoveNode missing node unexpectedly succeeded")
				}
			case 5: // create or retain an isolated node
				g.AddNode(left)
				model.nodes[left] = struct{}{}
			}
			assertGraphMatchesEdgeIdentityModel(t, g, model)
		}
	})
}
