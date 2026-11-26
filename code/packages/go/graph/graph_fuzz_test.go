package graph

import (
	"math"
	"testing"
)

// FuzzShortestPathWeightPreconditions maps each bounded input to at most four
// nodes and six edges. The byte encoding is part of the replay contract:
// bytes 0..2 select node count and endpoints, and bytes 3..8 select each edge
// as absent, weight 1/2, negative, NaN, +Inf, or -Inf. Truncation and empty
// inputs are valid seeds, and work is bounded by the four-node domain.
func FuzzShortestPathWeightPreconditions(f *testing.F) {
	f.Add([]byte{0, 0, 1, 1, 0, 0, 0, 0, 0})
	f.Add([]byte{2, 0, 3, 1, 0, 0, 0, 0, 1})
	f.Add([]byte{2, 0, 0, 3, 0, 0, 0, 0, 0})
	f.Add([]byte{2, 2, 3, 4, 0, 0, 0, 0, 0})
	f.Add([]byte{2, 0, 3, 5, 0, 0, 0, 0, 0})
	f.Add([]byte{2, 0, 3, 6, 0, 0, 0, 0, 0})

	f.Fuzz(func(t *testing.T, input []byte) {
		var encoded [9]byte
		copy(encoded[:], input)
		nodeCount := 2 + int(encoded[0]%3)
		start := int(encoded[1] % byte(nodeCount))
		end := int(encoded[2] % byte(nodeCount))
		model := tinyGraphModel{nodes: nodeCount}
		edgeIndex := 0
		for left := 0; left < nodeCount; left++ {
			for right := left + 1; right < nodeCount; right++ {
				code := encoded[3+edgeIndex] % 7
				edgeIndex++
				switch code {
				case 1:
					model.addEdge(left, right, 1)
				case 2:
					model.addEdge(left, right, 2)
				case 3:
					model.addEdge(left, right, -1)
				case 4:
					model.addEdge(left, right, math.NaN())
				case 5:
					model.addEdge(left, right, math.Inf(1))
				case 6:
					model.addEdge(left, right, math.Inf(-1))
				}
			}
		}

		for _, repr := range representations() {
			g := buildTinyGraph(model, repr)
			path, err := ShortestPath(g, tinyGraphNode(start), tinyGraphNode(end))
			invalidWeight := false
			for left := 0; left < nodeCount; left++ {
				for right := left + 1; right < nodeCount; right++ {
					weight := model.weight[left][right]
					if model.present[left][right] && (weight < 0 || math.IsNaN(weight) || math.IsInf(weight, 0)) {
						invalidWeight = true
					}
				}
			}
			if invalidWeight {
				if err == nil || path != nil {
					t.Fatalf("repr=%s input=%x: invalid graph returned path=%v, err=%v", repr, input, path, err)
				}
				continue
			}
			if err != nil {
				t.Fatalf("repr=%s input=%x: valid graph returned error: %v", repr, input, err)
			}
			if !tinyGraphPathMatches(model, start, end, path) {
				t.Fatalf("repr=%s input=%x: path %v disagrees with exhaustive oracle", repr, input, path)
			}
		}
	})
}
