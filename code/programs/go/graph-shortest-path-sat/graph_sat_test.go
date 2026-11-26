package graphshortestpathsat

import (
	"fmt"
	"strconv"
	"strings"
	"testing"

	graph "github.com/adhithyan15/coding-adventures/code/packages/go/graph"
	sat "github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver"
)

type testEdge struct{ left, right, weight int }
type testPath struct {
	vertices []int
	cost     int
}

// enumerateSimplePaths is a deliberately small DFS oracle. Its search shape
// differs from both Dijkstra in the graph package and positional SAT below.
func enumerateSimplePaths(nodeCount, source, destination int, edges []testEdge) []testPath {
	adjacency := make([][]testEdge, nodeCount)
	for _, edge := range edges {
		adjacency[edge.left] = append(adjacency[edge.left], edge)
		adjacency[edge.right] = append(adjacency[edge.right], testEdge{edge.right, edge.left, edge.weight})
	}
	paths := []testPath{}
	visited := make([]bool, nodeCount)
	vertices := []int{source}
	visited[source] = true
	var visit func(int, int)
	visit = func(current, cost int) {
		if current == destination {
			paths = append(paths, testPath{append([]int(nil), vertices...), cost})
			return
		}
		for _, edge := range adjacency[current] {
			if !visited[edge.right] {
				visited[edge.right] = true
				vertices = append(vertices, edge.right)
				visit(edge.right, cost+edge.weight)
				vertices = vertices[:len(vertices)-1]
				visited[edge.right] = false
			}
		}
	}
	visit(source, 0)
	return paths
}

func newTestGraph(nodeCount int, edges []testEdge, representation graph.GraphRepr) *graph.Graph {
	result := graph.New(representation)
	for vertex := 0; vertex < nodeCount; vertex++ {
		result.AddNode(vertexName(vertex))
	}
	for _, edge := range edges {
		result.AddEdge(vertexName(edge.left), vertexName(edge.right), float64(edge.weight))
	}
	return result
}

func vertexName(vertex int) string { return fmt.Sprintf("v%d", vertex) }

func parseVertexName(name string) (int, error) {
	if !strings.HasPrefix(name, "v") {
		return 0, fmt.Errorf("unexpected node label %q", name)
	}
	return strconv.Atoi(strings.TrimPrefix(name, "v"))
}

// replayModel checks a SAT witness without relying on any encoder-internal
// values. It validates endpoints, no repeated vertices, edge membership,
// and exact integer cost against the topology supplied to the test.
func replayModel(vertices []int, edges []testEdge, source, destination int) (int, error) {
	if len(vertices) < 2 || vertices[0] != source || vertices[len(vertices)-1] != destination {
		return 0, fmt.Errorf("witness endpoints %v do not match %d->%d", vertices, source, destination)
	}
	seen := make(map[int]bool, len(vertices))
	for _, vertex := range vertices {
		if seen[vertex] {
			return 0, fmt.Errorf("witness repeats vertex %d: %v", vertex, vertices)
		}
		seen[vertex] = true
	}
	cost := 0
	for step := 1; step < len(vertices); step++ {
		found := false
		for _, edge := range edges {
			if edge.left == vertices[step-1] && edge.right == vertices[step] ||
				edge.right == vertices[step-1] && edge.left == vertices[step] {
				cost += edge.weight
				found = true
				break
			}
		}
		if !found {
			return 0, fmt.Errorf("witness uses missing edge %d-%d: %v", vertices[step-1], vertices[step], vertices)
		}
	}
	return cost, nil
}

func caseDescription(representation graph.GraphRepr, nodeCount, topology int, edges []testEdge, source, destination int) string {
	return fmt.Sprintf("representation=%s vertices=%d topology=%d edges=%v endpoints=%d->%d",
		representation, nodeCount, topology, edges, source, destination)
}

func TestShortestPathAgainstPositionalSAT(t *testing.T) {
	checked := 0
	for nodeCount := 2; nodeCount <= 4; nodeCount++ {
		pairs := make([][2]int, 0, nodeCount*(nodeCount-1)/2)
		for left := 0; left < nodeCount; left++ {
			for right := left + 1; right < nodeCount; right++ {
				pairs = append(pairs, [2]int{left, right})
			}
		}
		topologyCount := 1
		for range pairs {
			topologyCount *= 3
		}
		for topology := 0; topology < topologyCount; topology++ {
			edges := topologyEdges(topology, pairs)
			for _, representation := range []graph.GraphRepr{graph.AdjacencyList, graph.AdjacencyMatrix} {
				candidateGraph := newTestGraph(nodeCount, edges, representation)
				for source := 0; source < nodeCount; source++ {
					for destination := 0; destination < nodeCount; destination++ {
						if source == destination {
							continue
						}
						description := caseDescription(representation, nodeCount, topology, edges, source, destination)
						oraclePaths := enumerateSimplePaths(nodeCount, source, destination, edges)
						observed, err := graph.ShortestPath(candidateGraph, vertexName(source), vertexName(destination))
						if err != nil {
							t.Fatalf("%s: ShortestPath error: %v", description, err)
						}

						if len(oraclePaths) == 0 {
							if len(observed) != 0 {
								t.Fatalf("%s: expected unreachable result, got path %v", description, observed)
							}
							witness, result, err := solveBound(nodeCount, edges, source, destination, nil)
							if err != nil || result != sat.SolveUnsat {
								t.Fatalf("%s: reachability query witness=%v result=%s err=%v; want UNSAT", description, witness, result, err)
							}
							checked++
							continue
						}

						bestCost := oraclePaths[0].cost
						for _, path := range oraclePaths[1:] {
							if path.cost < bestCost {
								bestCost = path.cost
							}
						}
						observedVertices := make([]int, len(observed))
						for index, name := range observed {
							observedVertices[index], err = parseVertexName(name)
							if err != nil {
								t.Fatalf("%s: parse ShortestPath output %v: %v", description, observed, err)
							}
						}
						observedCost, err := replayModel(observedVertices, edges, source, destination)
						if err != nil || observedCost != bestCost {
							t.Fatalf("%s: ShortestPath path=%v cost=%d replay_error=%v oracle_min=%d oracle_paths=%v",
								description, observed, observedCost, err, bestCost, oraclePaths)
						}

						cheaperThan := observedCost
						witness, result, err := solveBound(nodeCount, edges, source, destination, &cheaperThan)
						if err != nil || result != sat.SolveUnsat {
							t.Fatalf("%s: cheaper-than-observed query witness=%v result=%s err=%v; observed=%v cost=%d",
								description, witness, result, err, observed, observedCost)
						}

						// False claim control: claiming cost+1 cannot be optimal,
						// because the actual returned path is a cheaper witness.
						falseClaimLimit := observedCost + 1
						witness, result, err = solveBound(nodeCount, edges, source, destination, &falseClaimLimit)
						if err != nil || result != sat.SolveSat {
							t.Fatalf("%s: false-claim control witness=%v result=%s err=%v; want SAT below %d",
								description, witness, result, err, falseClaimLimit)
						}
						witnessCost, err := replayModel(witness, edges, source, destination)
						if err != nil || witnessCost >= falseClaimLimit {
							t.Fatalf("%s: SAT model failed replay: witness=%v cost=%d bound=%d err=%v",
								description, witness, witnessCost, falseClaimLimit, err)
						}
						checked++
					}
				}
			}
		}
	}
	if checked != 17832 {
		t.Fatalf("checked %d finite cases; want 17832", checked)
	}
	t.Logf("positional SAT checks: %d ordered graph/layout/endpoint cases", checked)
}

// topologyEdges maps trits 0/1/2 to absent/weight-1/weight-2 on canonical
// undirected pairs. Distinct trit assignments yield distinct topologies.
func topologyEdges(topology int, pairs [][2]int) []testEdge {
	edges := []testEdge{}
	for _, pair := range pairs {
		state := topology % 3
		topology /= 3
		if state != 0 {
			edges = append(edges, testEdge{pair[0], pair[1], state})
		}
	}
	return edges
}
