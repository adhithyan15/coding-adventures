package graph

import (
	"math"
	"strings"
	"testing"
)

func TestConcurrentSnapshotShortestPathInheritsWeightPreconditions(t *testing.T) {
	weights := []struct {
		name  string
		value float64
		check func(float64) bool
	}{
		{name: "negative", value: -2, check: func(got float64) bool { return got == -2 }},
		{name: "nan", value: math.NaN(), check: math.IsNaN},
		{name: "positive_infinity", value: math.Inf(1), check: func(got float64) bool { return math.IsInf(got, 1) }},
		{name: "negative_infinity", value: math.Inf(-1), check: func(got float64) bool { return math.IsInf(got, -1) }},
	}
	for _, repr := range representations() {
		for _, weight := range weights {
			for _, mutation := range []string{"add_edge", "set_edge_property"} {
				t.Run(string(repr)+"/"+weight.name+"/"+mutation, func(t *testing.T) {
					g := NewConcurrentGraph(repr)
					var err error
					if mutation == "add_edge" {
						err = g.AddEdge("invalid-left", "invalid-right", weight.value)
					} else {
						err = g.AddEdge("invalid-left", "invalid-right", 1)
						if err == nil {
							err = g.SetEdgeProperty("invalid-left", "invalid-right", "weight", weight.value)
						}
					}
					if err != nil {
						t.Fatalf("legacy mutation rejected weight %v: %v", weight.value, err)
					}
					if err := g.AddNode("start"); err != nil {
						t.Fatal(err)
					}
					if err := g.AddNode("disconnected-end"); err != nil {
						t.Fatal(err)
					}
					stored, err := g.EdgeWeight("invalid-left", "invalid-right")
					if err != nil || !weight.check(stored) {
						t.Fatalf("legacy edge weight = %v, %v; want %v", stored, err, weight.value)
					}

					snapshot := g.Snapshot()
					for _, endpoints := range [][2]string{{"start", "start"}, {"start", "disconnected-end"}} {
						path, err := snapshot.ShortestPath(endpoints[0], endpoints[1])
						if err == nil || !strings.Contains(err.Error(), "finite, non-negative edge weights") {
							t.Fatalf("ShortestPath(%q,%q) error = %v, want invalid-weight error", endpoints[0], endpoints[1], err)
						}
						if path != nil {
							t.Fatalf("ShortestPath(%q,%q) returned path %v with error", endpoints[0], endpoints[1], path)
						}
					}
				})
			}
		}
	}
}

func TestConcurrentSnapshotMinimumSpanningTreePreservesNegativeWeights(t *testing.T) {
	for _, repr := range representations() {
		t.Run(string(repr), func(t *testing.T) {
			g := NewConcurrentGraph(repr)
			if err := g.AddEdge("A", "B", 1); err != nil {
				t.Fatal(err)
			}
			if err := g.AddEdge("B", "C", 2); err != nil {
				t.Fatal(err)
			}
			if err := g.AddEdge("A", "C", 4); err != nil {
				t.Fatal(err)
			}
			if err := g.SetEdgeProperty("A", "B", "weight", -5); err != nil {
				t.Fatalf("SetEdgeProperty rejected a finite negative weight: %v", err)
			}

			tree, err := g.Snapshot().MinimumSpanningTree()
			if err != nil || len(tree) != 2 {
				t.Fatalf("MinimumSpanningTree() = %v, %v", tree, err)
			}
			total := 0.0
			negative := false
			for _, edge := range tree {
				total += edge.Weight
				negative = negative || edge.Weight < 0
			}
			if !negative || total != -3 {
				t.Fatalf("minimum spanning tree = %v, total weight %v; want a negative edge and total -3", tree, total)
			}
		})
	}
}
