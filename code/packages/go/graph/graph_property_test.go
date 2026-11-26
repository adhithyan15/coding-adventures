package graph

import (
	"math"
	"strconv"
	"testing"
)

const tinyGraphNodeCount = 4

// tinyGraphModel is the oracle's representation. It is populated independently
// from the Graph under test, so the reference does not ask Graph.Neighbors or
// Graph.EdgeWeight to tell it what the input graph contains.
type tinyGraphModel struct {
	nodes   int
	present [tinyGraphNodeCount][tinyGraphNodeCount]bool
	weight  [tinyGraphNodeCount][tinyGraphNodeCount]float64
}

func (model *tinyGraphModel) addEdge(left, right int, weight float64) {
	model.present[left][right] = true
	model.present[right][left] = true
	model.weight[left][right] = weight
	model.weight[right][left] = weight
}

func tinyGraphNode(index int) string {
	return "node-" + strconv.Itoa(index)
}

func buildTinyGraph(model tinyGraphModel, repr GraphRepr) *Graph {
	g := New(repr)
	for node := 0; node < model.nodes; node++ {
		g.AddNode(tinyGraphNode(node))
	}
	for left := 0; left < model.nodes; left++ {
		for right := left + 1; right < model.nodes; right++ {
			if model.present[left][right] {
				g.AddEdge(tinyGraphNode(left), tinyGraphNode(right), model.weight[left][right])
			}
		}
	}
	return g
}

func tinyGraphOracle(model tinyGraphModel, start, end int) (float64, bool) {
	best := math.Inf(1)
	found := false
	visited := [tinyGraphNodeCount]bool{}
	visited[start] = true

	var visit func(node int, distance float64)
	visit = func(node int, distance float64) {
		if node == end {
			if !found || distance < best {
				best = distance
				found = true
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
	return best, found
}

func tinyGraphPathMatches(model tinyGraphModel, start, end int, path []string) bool {
	expectedDistance, reachable := tinyGraphOracle(model, start, end)
	if !reachable {
		return len(path) == 0
	}
	if len(path) == 0 || path[0] != tinyGraphNode(start) || path[len(path)-1] != tinyGraphNode(end) || len(path) > model.nodes {
		return false
	}
	seen := make(map[string]bool, len(path))
	distance := 0.0
	for i, nodeName := range path {
		if seen[nodeName] {
			return false
		}
		seen[nodeName] = true
		if i == 0 {
			continue
		}
		left, right := -1, -1
		for node := 0; node < model.nodes; node++ {
			if tinyGraphNode(node) == path[i-1] {
				left = node
			}
			if tinyGraphNode(node) == nodeName {
				right = node
			}
		}
		if left < 0 || right < 0 || !model.present[left][right] {
			return false
		}
		distance += model.weight[left][right]
	}
	return distance == expectedDistance
}

func TestShortestPathMatchesExhaustiveFourNodeModel(t *testing.T) {
	// Each of the six undirected edge slots has three choices: absent, weight 1,
	// or weight 2. Every generated graph is queried for every ordered endpoint
	// pair in both storage representations; failures print the exact graph code.
	edgeSlots := [][2]int{{0, 1}, {0, 2}, {0, 3}, {1, 2}, {1, 3}, {2, 3}}
	const graphCount = 729 // 3^6

	for graphCode := 0; graphCode < graphCount; graphCode++ {
		model := tinyGraphModel{nodes: tinyGraphNodeCount}
		choices := graphCode
		for _, edge := range edgeSlots {
			switch choices % 3 {
			case 1:
				model.addEdge(edge[0], edge[1], 1)
			case 2:
				model.addEdge(edge[0], edge[1], 2)
			}
			choices /= 3
		}

		for _, repr := range representations() {
			g := buildTinyGraph(model, repr)
			for start := 0; start < model.nodes; start++ {
				for end := 0; end < model.nodes; end++ {
					path, err := ShortestPath(g, tinyGraphNode(start), tinyGraphNode(end))
					if err != nil {
						t.Fatalf("graph=%d repr=%s start=%d end=%d: unexpected error: %v", graphCode, repr, start, end, err)
					}
					if !tinyGraphPathMatches(model, start, end, path) {
						distance, reachable := tinyGraphOracle(model, start, end)
						t.Fatalf("graph=%d repr=%s start=%d end=%d path=%v: oracle reachable=%t distance=%v", graphCode, repr, start, end, path, reachable, distance)
					}
				}
			}
		}
	}
}

func TestShortestPathModelRejectsKnownWrongRoute(t *testing.T) {
	model := tinyGraphModel{nodes: tinyGraphNodeCount}
	model.addEdge(0, 1, 1)
	model.addEdge(1, 3, 1)
	model.addEdge(0, 2, 1)
	model.addEdge(2, 3, 2)
	wrongRoute := []string{tinyGraphNode(0), tinyGraphNode(2), tinyGraphNode(3)}
	if tinyGraphPathMatches(model, 0, 3, wrongRoute) {
		t.Fatalf("oracle accepted known route with cost 3 although a cost-2 route exists: %v", wrongRoute)
	}
}
