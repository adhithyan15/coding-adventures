package graphshortestpathsat

import (
	sat "github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver"
	"testing"
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
		for index, pair := range pairs {
			state := int(data[index+1] % 3)
			if state > 0 {
				edges = append(edges, testEdge{pair[0], pair[1], state})
			}
		}
		source := int(data[8]) % nodeCount
		destination := int(data[9]) % nodeCount
		if destination == source {
			destination = (destination + 1) % nodeCount
		}
		limit := int(data[10]%9) - 1 // -1..7; maximum simple path cost is six.
		paths := enumerateSimplePaths(nodeCount, source, destination, edges)
		wantSAT := false
		for _, path := range paths {
			if path.cost < limit {
				wantSAT = true
				break
			}
		}
		witness, got, err := solveBound(nodeCount, edges, source, destination, &limit)
		if err != nil {
			t.Fatalf("SAT query failed: n=%d edges=%v endpoints=%d->%d limit=%d: %v", nodeCount, edges, source, destination, limit, err)
		}
		if got == sat.SolveUnknown {
			t.Fatalf("bounded query returned UNKNOWN: n=%d edges=%v endpoints=%d->%d limit=%d", nodeCount, edges, source, destination, limit)
		}
		if (got == sat.SolveSat) != wantSAT {
			t.Fatalf("SAT/oracle mismatch: n=%d edges=%v endpoints=%d->%d limit=%d witness=%v result=%s oracle_paths=%v",
				nodeCount, edges, source, destination, limit, witness, got, paths)
		}
		if got == sat.SolveSat {
			cost, err := replayModel(witness, edges, source, destination)
			if err != nil || cost >= limit {
				t.Fatalf("invalid SAT model: n=%d edges=%v endpoints=%d->%d limit=%d witness=%v cost=%d err=%v",
					nodeCount, edges, source, destination, limit, witness, cost, err)
			}
		}
	})
}
