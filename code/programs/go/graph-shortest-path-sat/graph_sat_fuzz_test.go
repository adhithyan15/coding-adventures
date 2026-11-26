package graphshortestpathsat

import (
	"testing"

	graph "github.com/adhithyan15/coding-adventures/code/packages/go/graph"
	sat "github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver"
)

// FuzzPositionalSATAgainstOracle mutates a compact graph/query description.
// The exhaustive test owns the complete finite claim; this target stress-checks
// the encoder's input mapping, cost bounds, and SAT witness replay.
func FuzzPositionalSATAgainstOracle(f *testing.F) {
	f.Add([]byte{0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 3})
	f.Add([]byte{0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 3})
	f.Add([]byte{1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 4})
	f.Add([]byte{2, 2, 0, 1, 0, 2, 0, 0, 0, 3, 4})
	f.Fuzz(func(t *testing.T, raw []byte) {
		data := make([]byte, 11)
		copy(data, raw)
		nodeCount := 2 + int(data[0]%3)
		pairs := make([][2]int, 0, nodeCount*(nodeCount-1)/2)
		for left := 0; left < nodeCount; left++ {
			for right := left + 1; right < nodeCount; right++ {
				pairs = append(pairs, [2]int{left, right})
			}
		}
		edges := make([]testEdge, 0, len(pairs))
		topology, powerOfThree := 0, 1
		for index, pair := range pairs {
			state := int(data[index+1] % 3)
			topology += state * powerOfThree
			powerOfThree *= 3
			if state > 0 {
				edges = append(edges, testEdge{pair[0], pair[1], state})
			}
		}
		representation := graph.AdjacencyList
		if data[7]&1 != 0 {
			representation = graph.AdjacencyMatrix
		}
		source := int(data[8]) % nodeCount
		destination := int(data[9]) % nodeCount
		if destination == source {
			destination = (destination + 1) % nodeCount
		}
		limit := int(data[10]%9) - 1 // -1..7; maximum simple path cost is six.
		paths := enumerateSimplePaths(nodeCount, source, destination, edges)
		description := caseDescription(representation, nodeCount, topology, edges, source, destination)
		observed, err := graph.ShortestPath(newTestGraph(nodeCount, edges, representation), vertexName(source), vertexName(destination))
		if err != nil {
			t.Fatalf("%s: ShortestPath error: %v", description, err)
		}
		if len(paths) == 0 {
			if len(observed) != 0 {
				t.Fatalf("%s: expected unreachable result, got path %v", description, observed)
			}
		} else {
			bestCost := paths[0].cost
			for _, path := range paths[1:] {
				if path.cost < bestCost {
					bestCost = path.cost
				}
			}
			vertices := make([]int, len(observed))
			for index, name := range observed {
				vertices[index], err = parseVertexName(name)
				if err != nil {
					t.Fatalf("%s: parse output %v: %v", description, observed, err)
				}
			}
			cost, replayErr := replayModel(vertices, edges, source, destination)
			if replayErr != nil || cost != bestCost {
				t.Fatalf("%s: path=%v cost=%d replay_error=%v oracle_min=%d", description, observed, cost, replayErr, bestCost)
			}
		}
		wantSAT := false
		for _, path := range paths {
			if path.cost < limit {
				wantSAT = true
				break
			}
		}
		witness, got, err := solveBound(nodeCount, edges, source, destination, &limit)
		if err != nil {
			t.Fatalf("%s limit=%d: SAT query failed: %v", description, limit, err)
		}
		if got == sat.SolveUnknown {
			t.Fatalf("%s limit=%d: bounded query returned UNKNOWN", description, limit)
		}
		if (got == sat.SolveSat) != wantSAT {
			t.Fatalf("%s limit=%d: SAT/oracle mismatch witness=%v result=%s oracle_paths=%v",
				description, limit, witness, got, paths)
		}
		if got == sat.SolveSat {
			cost, err := replayModel(witness, edges, source, destination)
			if err != nil || cost >= limit {
				t.Fatalf("%s limit=%d: invalid SAT model witness=%v cost=%d err=%v",
					description, limit, witness, cost, err)
			}
		}
	})
}
