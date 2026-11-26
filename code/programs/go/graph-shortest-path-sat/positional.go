package graphshortestpathsat

import (
	"fmt"
	sat "github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver"
)

type cnf struct {
	s   *sat.Solver
	err error
}

func (b *cnf) variable() sat.Lit {
	if b.err != nil {
		return 0
	}
	v, e := b.s.NewVar()
	b.err = e
	return sat.Lit(uint32(v) << 1)
}
func (b *cnf) clause(ls ...sat.Lit) {
	if b.err == nil {
		_, b.err = b.s.AddClause(ls)
	}
}
func exactlyOne(b *cnf, ls []sat.Lit) {
	b.clause(ls...)
	for i := range ls {
		for j := i + 1; j < len(ls); j++ {
			b.clause(ls[i]^1, ls[j]^1)
		}
	}
}

// solveLength encodes paths of exactly k edges directly as positional state.
// For each position, an at-least-one clause plus pairwise exclusions makes
// vertex and cumulative-cost state one-hot. A non-edge transition adds
// (not vertexHere OR not vertexThere). A legal weighted transition adds
// (not vertexHere OR not vertexThere OR not oldCost OR newCost), where
// newCost is exactly oldCost plus the edge weight. These implications are
// paired with one-hot cost states, so a selected path has one exact sum.
func solveLength(n int, es []testEdge, source, dest, k int, costLimit *int) ([]int, sat.SolveResult, error) {
	solver, e := sat.NewSolver()
	if e != nil {
		return nil, "", e
	}
	b := &cnf{s: solver}
	x := make([][]sat.Lit, k+1)
	costs := make([][]sat.Lit, k+1)
	for step := 0; step <= k; step++ {
		x[step] = make([]sat.Lit, n)
		for v := 0; v < n; v++ {
			x[step][v] = b.variable()
		}
		exactlyOne(b, x[step])
		if step == 0 {
			b.clause(x[step][source])
		}
		if step == k {
			b.clause(x[step][dest])
		}
		// A simple path never revisits a vertex, eliminating cyclic witnesses.
		for prior := 0; prior < step; prior++ {
			for v := 0; v < n; v++ {
				b.clause(x[prior][v]^1, x[step][v]^1)
			}
		}
		maxCost := 2 * step
		costs[step] = make([]sat.Lit, maxCost+1)
		for c := range costs[step] {
			costs[step][c] = b.variable()
		}
		exactlyOne(b, costs[step])
		if step == 0 {
			b.clause(costs[step][0])
		}
	}
	weights := make([][]int, n)
	for i := range weights {
		weights[i] = make([]int, n)
	}
	for _, e := range es {
		weights[e.left][e.right] = e.weight
		weights[e.right][e.left] = e.weight
	}
	for step := 0; step < k; step++ {
		for u := 0; u < n; u++ {
			for v := 0; v < n; v++ {
				w := weights[u][v]
				if w == 0 {
					b.clause(x[step][u]^1, x[step+1][v]^1)
					continue
				}
				for c := 0; c < len(costs[step]); c++ {
					next := c + w
					if next >= len(costs[step+1]) {
						continue
					}
					b.clause(x[step][u]^1, x[step+1][v]^1, costs[step][c]^1, costs[step+1][next])
				}
			}
		}
	}
	if costLimit != nil {
		for c, l := range costs[k] {
			if c >= *costLimit {
				b.clause(l ^ 1)
			}
		}
	}
	if b.err != nil {
		return nil, "", b.err
	}
	result, e := solver.Solve()
	if e != nil || result != sat.SolveSat {
		return nil, result, e
	}
	model, ok := solver.Model()
	if !ok {
		return nil, result, fmt.Errorf("SAT without model")
	}
	nodes := make([]int, k+1)
	for step := 0; step <= k; step++ {
		found := false
		for v, l := range x[step] {
			if model[uint32(l)>>1] {
				nodes[step] = v
				found = true
				break
			}
		}
		if !found {
			return nil, result, fmt.Errorf("position %d has no true vertex", step)
		}
	}
	return nodes, result, nil
}

// solveBound asks every possible simple-path length in turn. The disjunction
// across lengths is implemented by returning the first SAT model; only after
// every length is UNSAT may the whole reachability or cost query be UNSAT.
// The simple-path enumerator is used only as a separate test oracle.
func solveBound(n int, es []testEdge, source, dest int, costLimit *int) ([]int, sat.SolveResult, error) {
	for k := 1; k < n; k++ {
		p, r, e := solveLength(n, es, source, dest, k, costLimit)
		if e != nil {
			return nil, r, e
		}
		if r == sat.SolveSat {
			return p, r, nil
		}
		if r != sat.SolveUnsat {
			return nil, r, fmt.Errorf("solver returned %s", r)
		}
	}
	return nil, sat.SolveUnsat, nil
}
