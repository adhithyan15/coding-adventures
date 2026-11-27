package graph

import (
	"errors"
	"math"
	"testing"
)

func tinyFloatPathOracle(model tinyGraphModel, start, end int) (topologyReachable, finitePath bool, best float64) {
	best = math.Inf(1)
	visited := [tinyGraphNodeCount]bool{}
	visited[start] = true

	var visit func(node int, distance float64)
	visit = func(node int, distance float64) {
		if node == end {
			topologyReachable = true
			if !math.IsInf(distance, 1) && (!finitePath || distance < best) {
				finitePath = true
				best = distance
			}
			return
		}
		for neighbor := 0; neighbor < model.nodes; neighbor++ {
			if !model.present[node][neighbor] || visited[neighbor] {
				continue
			}
			visited[neighbor] = true
			visit(neighbor, distance+model.weight[node][neighbor])
			visited[neighbor] = false
		}
	}
	visit(start, 0)
	return topologyReachable, finitePath, best
}

func assertShortestPathMatchesFloatOracle(t *testing.T, model tinyGraphModel, start, end int, repr GraphRepr) {
	t.Helper()
	g := buildTinyGraph(model, repr)
	path, err := ShortestPath(g, tinyGraphNode(start), tinyGraphNode(end))
	topologyReachable, finitePath, best := tinyFloatPathOracle(model, start, end)
	if !topologyReachable {
		if err != nil || len(path) != 0 {
			t.Fatalf("unreachable query got path=%v err=%v", path, err)
		}
		return
	}
	if !finitePath {
		if !errors.Is(err, ErrPathDistanceOverflow) || path != nil {
			t.Fatalf("overflow-only query got path=%v err=%v", path, err)
		}
		return
	}
	if err != nil {
		t.Fatalf("finite query returned error: %v", err)
	}
	if len(path) == 0 || path[0] != tinyGraphNode(start) || path[len(path)-1] != tinyGraphNode(end) || len(path) > model.nodes {
		t.Fatalf("finite query returned invalid path %v", path)
	}
	distance := 0.0
	for index := 1; index < len(path); index++ {
		left := -1
		right := -1
		for node := 0; node < model.nodes; node++ {
			if path[index-1] == tinyGraphNode(node) {
				left = node
			}
			if path[index] == tinyGraphNode(node) {
				right = node
			}
		}
		if left < 0 || right < 0 || !model.present[left][right] {
			t.Fatalf("finite query returned non-edge in path %v", path)
		}
		distance += model.weight[left][right]
	}
	if math.IsInf(distance, 1) || distance != best {
		t.Fatalf("path %v has computed distance %g, want finite optimum %g", path, distance, best)
	}
}

func TestShortestPathDistanceOverflowModelProperty(t *testing.T) {
	// Exhaust every three-node undirected graph where each edge is absent,
	// one, MaxFloat64, or the smallest positive subnormal float64.
	for encoding := 0; encoding < 64; encoding++ {
		model := tinyGraphModel{nodes: 3}
		remaining := encoding
		for left := 0; left < 3; left++ {
			for right := left + 1; right < 3; right++ {
				code := remaining % 4
				remaining /= 4
				var weight float64
				switch code {
				case 1:
					weight = 1
				case 2:
					weight = math.MaxFloat64
				case 3:
					weight = math.SmallestNonzeroFloat64
				default:
					continue
				}
				model.addEdge(left, right, weight)
			}
		}
		for _, repr := range representations() {
			for start := 0; start < model.nodes; start++ {
				for end := 0; end < model.nodes; end++ {
					assertShortestPathMatchesFloatOracle(t, model, start, end, repr)
				}
			}
		}
	}
}

// FuzzShortestPathDistanceOverflow explores at most four nodes and six edges.
// Edge codes select absent, unit, MaxFloat64, or the smallest positive float64.
func FuzzShortestPathDistanceOverflow(f *testing.F) {
	f.Add([]byte{2, 0, 2, 2, 0, 0, 0, 0, 0})
	f.Add([]byte{2, 0, 2, 2, 0, 0, 1, 0, 0})
	f.Add([]byte{2, 0, 3, 2, 0, 0, 0, 0, 0})
	f.Add([]byte{2, 2, 3, 2, 0, 0, 0, 0, 0})

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
				code := encoded[3+edgeIndex] % 4
				edgeIndex++
				var weight float64
				switch code {
				case 1:
					weight = 1
				case 2:
					weight = math.MaxFloat64
				case 3:
					weight = math.SmallestNonzeroFloat64
				default:
					continue
				}
				model.addEdge(left, right, weight)
			}
		}
		for _, repr := range representations() {
			assertShortestPathMatchesFloatOracle(t, model, start, end, repr)
		}
	})
}
