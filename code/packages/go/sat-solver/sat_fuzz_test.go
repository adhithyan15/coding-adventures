package satsolver

import "testing"

func FuzzParseDimacsBounded(f *testing.F) {
	for _, seed := range []string{
		"p cnf 1 1\n1 0\n",
		"c empty formula\np cnf 0 0\n",
		"p cnf 2 2\n1\n-2 0 0\n",
		"p cnf 1 1\n-0\n",
		"p cnf 2 1 extra\n1 0\n",
	} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, input string) {
		if len(input) > 64*1024 {
			t.Skip()
		}
		formula, err := ParseDimacs(input, ParseDimacsOptions{MaxVars: 32})
		if err != nil {
			return
		}
		if formula.NumVars > 32 {
			t.Fatalf("parser exceeded its declared-variable limit: %d", formula.NumVars)
		}
		for _, clause := range formula.Clauses {
			for _, lit := range clause {
				v, err := VarOf(lit)
				if err != nil || v >= formula.NumVars {
					t.Fatalf("parsed literal %d out of range for %d variables: %v", lit, formula.NumVars, err)
				}
			}
		}
		serialized, err := WriteDimacs(formula)
		if err != nil {
			t.Fatalf("WriteDimacs: %v", err)
		}
		roundTrip, err := ParseDimacs(serialized, ParseDimacsOptions{MaxVars: 32})
		if err != nil || !equalFormula(formula, roundTrip) {
			t.Fatalf("round-trip mismatch: %v", err)
		}
	})
}

func FuzzBoundedSolverAgainstOracle(f *testing.F) {
	f.Add([]byte{3, 1, 0, 1, 2, 3, 4})
	f.Add([]byte{4, 5, 2, 9, 17, 31, 0, 3, 7})
	f.Add([]byte{1, 0, 0})
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 64 {
			t.Skip()
		}
		n := 1
		if len(data) > 0 {
			n += int(data[0] % 8)
		}
		clauses := make([][]Lit, 0, len(data)/3)
		solver, err := NewSolver(SolverConfig{MaxVars: uint32(n), MaxClauses: 128, MaxClauseLiterals: 512, MaxStoredLiterals: 512})
		if err != nil {
			t.Fatalf("NewSolver: %v", err)
		}
		for v := 0; v < n; v++ {
			if _, err := solver.NewVar(); err != nil {
				t.Fatal(err)
			}
		}
		for i := 1; i+2 < len(data); i += 3 {
			length := 1 + int(data[i]%3)
			clause := make([]Lit, 0, length)
			for j := 0; j < length; j++ {
				b := data[i+j%3]
				lit, err := MakeLit(Var(int(b)%n), b&0x80 != 0)
				if err != nil {
					t.Fatal(err)
				}
				clause = append(clause, lit)
			}
			clauses = append(clauses, clause)
			if _, err := solver.AddClause(clause); err != nil {
				t.Fatal(err)
			}
		}
		for operation := 0; operation < 3; operation++ {
			assumptions := []Lit(nil)
			if len(data) > 1 {
				b := data[(operation+1)%len(data)]
				lit, err := MakeLit(Var(int(b)%n), b&0x40 != 0)
				if err != nil {
					t.Fatal(err)
				}
				assumptions = append(assumptions, lit)
				if b&0x20 != 0 {
					assumptions = append(assumptions, lit)
				}
				if b&0x10 != 0 {
					opposite, _ := NegateLit(lit)
					assumptions = append(assumptions, opposite)
				}
			}
			if operation == 0 && data[0]&0x08 != 0 {
				zero := uint64(0)
				got, err := solver.SolveWithAssumptions(assumptions, SolverOptions{MaxSteps: &zero})
				if err != nil || got != SolveUnknown {
					t.Fatalf("zero-budget stateful fuzz operation: %v (%v)", got, err)
				}
			}
			want := oracleSAT(n, clauses, assumptions)
			got, err := solver.SolveWithAssumptions(assumptions)
			if err != nil {
				t.Fatalf("SolveWithAssumptions: %v", err)
			}
			if want && got != SolveSat || !want && got != SolveUnsat {
				t.Fatalf("solver/oracle mismatch: got=%s wantSat=%t", got, want)
			}
			if got == SolveSat {
				model, valid := solver.Model()
				if !valid || !allClausesSatisfied(clauses, model) || !assumptionsSatisfied(assumptions, model) {
					t.Fatalf("invalid model %v", model)
				}
			} else if oracleSAT(n, clauses, solver.FailedCore()) {
				t.Fatalf("unsound failed core %v", solver.FailedCore())
			}
			if operation == 1 && len(data) > 2 {
				b := data[len(data)-2]
				lit, err := MakeLit(Var(int(b)%n), b&0x80 != 0)
				if err != nil {
					t.Fatal(err)
				}
				clause := []Lit{lit}
				_, err = solver.AddClause(clause)
				if err != nil {
					t.Fatal(err)
				}
				clauses = append(clauses, clause)
				if model, valid := solver.Model(); valid || model != nil || len(solver.FailedCore()) != 0 {
					t.Fatalf("fuzz add_clause did not clear saved results")
				}
			}
			if operation == 2 {
				solver.Reset()
				if model, valid := solver.Model(); valid || model != nil || len(solver.FailedCore()) != 0 {
					t.Fatalf("fuzz reset did not clear saved results")
				}
			}
		}
	})
}

func FuzzMalformedRawLiteralIsRejectedAtomically(f *testing.F) {
	f.Add(uint32(0))
	f.Add(uint32(1))
	f.Add(uint32(1 << 31))
	f.Add(^uint32(0))
	f.Fuzz(func(t *testing.T, raw uint32) {
		solver, err := NewSolver(SolverConfig{MaxVars: 8, MaxClauses: 8, MaxClauseLiterals: 16, MaxStoredLiterals: 32})
		if err != nil {
			t.Fatal(err)
		}
		a, err := MakeLit(0, false)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := solver.NewVar(); err != nil {
			t.Fatal(err)
		}
		beforeVars, beforeClauses := solver.NVars(), solver.NClauses()
		_, err = solver.AddClause([]Lit{a, Lit(raw)})
		if err != nil && (solver.NVars() != beforeVars || solver.NClauses() != beforeClauses) {
			t.Fatalf("rejected literal %#x partially changed solver", raw)
		}
		if raw >= 1<<31 && err == nil {
			t.Fatalf("accepted malformed packed literal %#x", raw)
		}
	})
}

func equalFormula(a, b CnfFormula) bool {
	if a.NumVars != b.NumVars || len(a.Clauses) != len(b.Clauses) {
		return false
	}
	for i := range a.Clauses {
		if len(a.Clauses[i]) != len(b.Clauses[i]) {
			return false
		}
		for j := range a.Clauses[i] {
			if a.Clauses[i][j] != b.Clauses[i][j] {
				return false
			}
		}
	}
	return true
}
