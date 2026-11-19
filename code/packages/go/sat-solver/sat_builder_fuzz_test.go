package satsolver

import "testing"

// FuzzBuilderBitvectorsAgainstScalarOracle checks wider ripple arithmetic and
// equality with a scalar oracle, then asks the solver to reject a mutant output.
func FuzzBuilderBitvectorsAgainstScalarOracle(f *testing.F) {
	f.Add(uint8(0), uint64(0), uint64(0))
	f.Add(uint8(1), uint64(1), uint64(1))
	f.Add(uint8(4), uint64(9), uint64(7))
	f.Add(uint8(8), uint64(173), uint64(91))
	f.Fuzz(func(t *testing.T, widthByte uint8, aValue, bValue uint64) {
		width := int(widthByte % 9)
		mask := uint64(0)
		if width > 0 {
			mask = (uint64(1) << width) - 1
		}
		aValue &= mask
		bValue &= mask
		solver, err := NewSolver(SolverConfig{MaxVars: 128, MaxClauses: 4096, MaxClauseLiterals: 16384, MaxStoredLiterals: 16384})
		if err != nil {
			t.Fatal(err)
		}
		builder, err := NewCnfBuilder(solver)
		if err != nil {
			t.Fatal(err)
		}
		a, b := make([]Lit, width), make([]Lit, width)
		for i := 0; i < width; i++ {
			a[i], err = builder.NewLit()
			if err != nil {
				t.Fatal(err)
			}
			b[i], err = builder.NewLit()
			if err != nil {
				t.Fatal(err)
			}
		}
		sum, err := builder.BvAdd(a, b)
		if err != nil {
			t.Fatal(err)
		}
		equal, err := builder.BvEq(a, b)
		if err != nil {
			t.Fatal(err)
		}
		assumptions := make([]Lit, 0, 3*width+1)
		for i := 0; i < width; i++ {
			assumptions = append(assumptions, truthLit(t, a[i], aValue&(uint64(1)<<i) != 0))
			assumptions = append(assumptions, truthLit(t, b[i], bValue&(uint64(1)<<i) != 0))
		}
		wantSum := (aValue + bValue) & mask
		for i := 0; i < width; i++ {
			assumptions = append(assumptions, truthLit(t, sum[i], wantSum&(uint64(1)<<i) != 0))
		}
		wantEqual := aValue == bValue
		assumptions = append(assumptions, truthLit(t, equal, wantEqual))
		if got, err := solver.SolveWithAssumptions(assumptions); err != nil || got != SolveSat {
			t.Fatalf("width=%d a=%d b=%d expected outputs: %v %v", width, aValue, bValue, got, err)
		}
		model, valid := solver.Model()
		if !valid || len(model) != int(solver.NVars()) {
			t.Fatalf("missing output model: valid=%t len=%d vars=%d", valid, len(model), solver.NVars())
		}
		for i := 0; i < width; i++ {
			if model[uint32(sum[i])>>1] != (wantSum&(uint64(1)<<i) != 0) {
				t.Fatalf("sum witness bit %d disagrees with scalar oracle", i)
			}
		}
		if model[uint32(equal)>>1] != wantEqual {
			t.Fatal("equality witness disagrees with scalar oracle")
		}
		if width > 0 {
			bad := append([]Lit(nil), assumptions...)
			bad[2*width] = bad[2*width] ^ 1
			if got, err := solver.SolveWithAssumptions(bad); err != nil || got != SolveUnsat {
				t.Fatalf("accepted wrong sum bit width=%d a=%d b=%d: %v %v", width, aValue, bValue, got, err)
			}
		}
		badEq := append([]Lit(nil), assumptions...)
		badEq[len(badEq)-1] = badEq[len(badEq)-1] ^ 1
		if got, err := solver.SolveWithAssumptions(badEq); err != nil || got != SolveUnsat {
			t.Fatalf("accepted wrong equality width=%d a=%d b=%d: %v %v", width, aValue, bValue, got, err)
		}
	})
}
