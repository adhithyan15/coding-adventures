package satsolver

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"reflect"
	"strconv"
	"strings"
	"testing"
)

const sharedCorpusPath = "../../../specs/fixtures/sat-cnf-v1/cases.json"
const sharedCorpusSHA256 = "ba66f3a23690ced4d01621e2e26341c1c2e5bee7bce5a455e7a82a8435b32c63"

type corpus struct {
	Version int          `json:"version"`
	Cases   []corpusCase `json:"cases"`
}

type corpusCase struct {
	ID     string       `json:"id"`
	DIMACS string       `json:"dimacs"`
	Steps  []corpusStep `json:"steps"`
}

type corpusStep struct {
	Op          string        `json:"op"`
	Assumptions []int         `json:"assumptions"`
	Clause      []int         `json:"clause"`
	Budget      *corpusBudget `json:"budget"`
	Expect      string        `json:"expect"`
	ExpectCore  []int         `json:"expect_core"`
}

type corpusBudget struct {
	MaxConflicts *uint64 `json:"max_conflicts"`
	MaxSteps     *uint64 `json:"max_steps"`
}

func loadSharedCorpus(t *testing.T) corpus {
	t.Helper()
	data, err := os.ReadFile(sharedCorpusPath)
	if err != nil {
		t.Fatalf("read shared SAT corpus: %v", err)
	}
	digest := sha256.Sum256(data)
	if got := hex.EncodeToString(digest[:]); got != sharedCorpusSHA256 {
		t.Fatalf("shared corpus digest changed: got %s, want %s", got, sharedCorpusSHA256)
	}
	var got corpus
	if err := json.Unmarshal(data, &got); err != nil {
		t.Fatalf("decode shared SAT corpus: %v", err)
	}
	if got.Version != 1 || len(got.Cases) != 11 {
		t.Fatalf("shared corpus shape: version=%d cases=%d", got.Version, len(got.Cases))
	}
	wantIDs := []string{"empty_formula_sat", "assumption_satisfies_clause", "assumption_conflict_with_irrelevant_and_duplicate", "nonminimal_but_sound_failed_core", "empty_clause_unsat", "transport_crlf_comments_spanning_and_multiple_clauses", "satlib_terminator_ignores_following_text", "negative_zero_terminates_empty_clause", "contradictory_assumptions_and_recovery", "zero_budget_unknown_then_unbounded_sat", "clause_addition_invalidates_model_and_preserves_constraint"}
	seen := make(map[string]bool, len(got.Cases))
	for _, c := range got.Cases {
		if c.ID == "" || seen[c.ID] {
			t.Fatalf("empty or duplicate shared corpus ID %q", c.ID)
		}
		seen[c.ID] = true
	}
	for i, id := range wantIDs {
		if got.Cases[i].ID != id {
			t.Fatalf("shared corpus ID[%d]=%q, want %q", i, got.Cases[i].ID, id)
		}
	}
	return got
}

// parseDIMACSOracle decodes the signed DIMACS source directly, independently
// of ParseDimacs and the production literal helpers used by the solver.
func parseDIMACSOracle(t *testing.T, text string) (uint32, [][]Lit) {
	t.Helper()
	var n uint32
	var declared int
	header := false
	var clauses [][]Lit
	var current []Lit
	for _, line := range strings.Split(strings.ReplaceAll(strings.ReplaceAll(text, "\r\n", "\n"), "\r", "\n"), "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "c") {
			continue
		}
		if line == "%" {
			break
		}
		if !header {
			fields := strings.Fields(line)
			if len(fields) != 4 || fields[0] != "p" || fields[1] != "cnf" {
				t.Fatalf("oracle rejected fixture header %q", line)
			}
			v, e1 := strconv.ParseUint(fields[2], 10, 32)
			c, e2 := strconv.Atoi(fields[3])
			if e1 != nil || e2 != nil || c < 0 {
				t.Fatalf("bad oracle header %q", line)
			}
			n, declared, header = uint32(v), c, true
			continue
		}
		for _, field := range strings.Fields(line) {
			v, err := strconv.ParseInt(field, 10, 64)
			if err != nil {
				t.Fatalf("bad fixture token %q", field)
			}
			if v == 0 {
				clauses = append(clauses, append([]Lit(nil), current...))
				current = nil
				continue
			}
			if v < -int64(n) || v > int64(n) {
				t.Fatalf("fixture literal %d outside declared variables", v)
			}
			neg := v < 0
			idx := uint64(v)
			if neg {
				idx = -idx
			}
			idx--
			// Encode here arithmetically; don't call MakeLit/VarOf/SignOf.
			packed := uint32(idx << 1)
			if neg {
				packed++
			}
			current = append(current, Lit(packed))
		}
	}
	if !header || len(current) != 0 || len(clauses) != declared {
		t.Fatalf("oracle DIMACS framing mismatch: header=%t clauses=%d want=%d pending=%v", header, len(clauses), declared, current)
	}
	return n, clauses
}

func literalFromDIMACS(t *testing.T, n int) Lit {
	t.Helper()
	if n == 0 {
		t.Fatal("zero is a clause terminator, not an assumption or clause literal")
	}
	lit, err := MakeLit(Var(absInt(n)-1), n < 0)
	if err != nil {
		t.Fatalf("MakeLit(%d): %v", n, err)
	}
	return lit
}

func literalsFromDIMACS(t *testing.T, values []int) []Lit {
	t.Helper()
	out := make([]Lit, len(values))
	for i, value := range values {
		out[i] = literalFromDIMACS(t, value)
	}
	return out
}

func absInt(n int) int {
	if n < 0 {
		return -n
	}
	return n
}

func budgetOptions(b *corpusBudget) []SolverOptions {
	if b == nil {
		return nil
	}
	return []SolverOptions{{MaxConflicts: b.MaxConflicts, MaxSteps: b.MaxSteps}}
}

func allClausesSatisfied(clauses [][]Lit, model []bool) bool {
	for _, clause := range clauses {
		clauseSatisfied := false
		for _, lit := range clause {
			v := uint32(lit) >> 1
			if int(v) >= len(model) || uint32(lit) >= 1<<31 {
				return false
			}
			negated := uint32(lit)&1 != 0
			value := model[v]
			if negated {
				value = !value
			}
			if value {
				clauseSatisfied = true
				break
			}
		}
		if !clauseSatisfied {
			return false
		}
	}
	return true
}

func oracleSAT(n int, clauses [][]Lit, assumptions []Lit) bool {
	if n < 0 || n > 20 {
		panic("test oracle domain must be bounded")
	}
	for mask := uint64(0); mask < uint64(1)<<n; mask++ {
		model := make([]bool, n)
		for v := range model {
			model[v] = mask&(uint64(1)<<v) != 0
		}
		if !allClausesSatisfied(clauses, model) {
			continue
		}
		ok := true
		for _, lit := range assumptions {
			v := uint32(lit) >> 1
			negated := uint32(lit)&1 != 0
			value := model[v]
			if negated {
				value = !value
			}
			if !value {
				ok = false
				break
			}
		}
		if ok {
			return true
		}
	}
	return false
}

func TestSharedStatefulCorpus(t *testing.T) {
	data := loadSharedCorpus(t)
	for _, tc := range data.Cases {
		t.Run(tc.ID, func(t *testing.T) {
			oracleVars, oracleClauses := parseDIMACSOracle(t, tc.DIMACS)
			formula, err := ParseDimacs(tc.DIMACS)
			if err != nil {
				t.Fatalf("ParseDimacs: %v", err)
			}
			if formula.NumVars != oracleVars || !sameClauses(formula.Clauses, oracleClauses) {
				t.Fatalf("production parser disagrees with independent signed-DIMACS decoder")
			}
			solver, err := NewSolver()
			if err != nil {
				t.Fatalf("NewSolver: %v", err)
			}
			for want := Var(0); uint32(want) < formula.NumVars; want++ {
				got, err := solver.NewVar()
				if err != nil || got != want {
					t.Fatalf("NewVar: got (%d,%v), want %d", got, err, want)
				}
			}
			clauses := cloneClauses(oracleClauses)
			for _, clause := range formula.Clauses {
				if _, err := solver.AddClause(clause); err != nil {
					t.Fatalf("AddClause: %v", err)
				}
			}
			var lastAssumptions []Lit
			for stepIndex, step := range tc.Steps {
				switch step.Op {
				case "solve":
					lastAssumptions = literalsFromDIMACS(t, step.Assumptions)
					result, err := solver.SolveWithAssumptions(lastAssumptions, budgetOptions(step.Budget)...)
					if err != nil {
						t.Fatalf("step %d solve: %v", stepIndex, err)
					}
					if string(result) != step.Expect {
						t.Fatalf("step %d result: got %s, want %s", stepIndex, result, step.Expect)
					}
					switch result {
					case SolveSat:
						model, valid := solver.Model()
						if !valid || len(model) != int(formula.NumVars) {
							t.Fatalf("SAT model validity/width: valid=%t len=%d vars=%d", valid, len(model), formula.NumVars)
						}
						if !allClausesSatisfied(clauses, model) || !assumptionsSatisfied(lastAssumptions, model) {
							t.Fatalf("SAT model does not satisfy the original formula and assumptions: %v", model)
						}
						if len(solver.FailedCore()) != 0 {
							t.Fatalf("SAT left a failed core: %v", solver.FailedCore())
						}
					case SolveUnsat:
						model, valid := solver.Model()
						if valid || model != nil {
							t.Fatalf("UNSAT exposed a valid model: %v", model)
						}
						core := solver.FailedCore()
						if hasDuplicateLits(core) || !isSubset(core, lastAssumptions) {
							t.Fatalf("failed core is not a duplicate-free assumption subset: core=%v assumptions=%v", core, lastAssumptions)
						}
						if oracleSAT(int(formula.NumVars), clauses, core) {
							t.Fatalf("failed core does not make original formula UNSAT: %v", core)
						}
						if step.ExpectCore != nil && !sameLitSet(core, literalsFromDIMACS(t, step.ExpectCore)) {
							t.Fatalf("failed core %v, want exact sound core %v", core, step.ExpectCore)
						}
					case SolveUnknown:
						model, valid := solver.Model()
						if valid || model != nil || len(solver.FailedCore()) != 0 {
							t.Fatalf("UNKNOWN leaked model/core: model=%v valid=%t core=%v", model, valid, solver.FailedCore())
						}
					}
				case "add_clause":
					clause := literalsFromDIMACS(t, step.Clause)
					_, err := solver.AddClause(clause)
					if err != nil {
						t.Fatalf("step %d AddClause: %v", stepIndex, err)
					}
					clauses = append(clauses, append([]Lit(nil), clause...))
					model, valid := solver.Model()
					if valid || model != nil || len(solver.FailedCore()) != 0 {
						t.Fatalf("AddClause did not invalidate saved result: model=%v valid=%t core=%v", model, valid, solver.FailedCore())
					}
				case "reset":
					solver.Reset()
					model, valid := solver.Model()
					if valid || model != nil || len(solver.FailedCore()) != 0 {
						t.Fatalf("Reset did not invalidate saved result: model=%v valid=%t core=%v", model, valid, solver.FailedCore())
					}
				default:
					t.Fatalf("unknown corpus operation %q", step.Op)
				}
			}
		})
	}
}

func sameClauses(a, b [][]Lit) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if len(a[i]) != len(b[i]) {
			return false
		}
		for j := range a[i] {
			if a[i][j] != b[i][j] {
				return false
			}
		}
	}
	return true
}

func cloneClauses(clauses [][]Lit) [][]Lit {
	out := make([][]Lit, len(clauses))
	for i, clause := range clauses {
		out[i] = append([]Lit(nil), clause...)
	}
	return out
}

func assumptionsSatisfied(assumptions []Lit, model []bool) bool {
	for _, lit := range assumptions {
		v := uint32(lit) >> 1
		if uint32(lit) >= 1<<31 || int(v) >= len(model) {
			return false
		}
		value := model[v]
		negated := uint32(lit)&1 != 0
		if negated {
			value = !value
		}
		if !value {
			return false
		}
	}
	return true
}

func hasDuplicateLits(lits []Lit) bool {
	seen := make(map[Lit]struct{}, len(lits))
	for _, lit := range lits {
		if _, ok := seen[lit]; ok {
			return true
		}
		seen[lit] = struct{}{}
	}
	return false
}

func isSubset(a, b []Lit) bool {
	set := make(map[Lit]struct{}, len(b))
	for _, lit := range b {
		set[lit] = struct{}{}
	}
	for _, lit := range a {
		if _, ok := set[lit]; !ok {
			return false
		}
	}
	return true
}

func sameLitSet(a, b []Lit) bool {
	return len(a) == len(b) && isSubset(a, b) && isSubset(b, a)
}

func TestLiteralEncodingAndCheckedBoundaries(t *testing.T) {
	for v := Var(0); v < 32; v++ {
		for _, negated := range []bool{false, true} {
			lit, err := MakeLit(v, negated)
			if err != nil {
				t.Fatal(err)
			}
			gotVar, err := VarOf(lit)
			if err != nil || gotVar != v {
				t.Fatalf("VarOf(%d)=(%d,%v), want %d", lit, gotVar, err, v)
			}
			gotSign, err := SignOf(lit)
			if err != nil || gotSign != negated {
				t.Fatalf("SignOf(%d)=(%t,%v), want %t", lit, gotSign, err, negated)
			}
			inverse, err := NegateLit(lit)
			if err != nil || inverse != lit^1 {
				t.Fatalf("NegateLit(%d)=(%d,%v)", lit, inverse, err)
			}
		}
	}
	if _, err := MakeLit(Var(1<<30), false); err == nil {
		t.Fatal("MakeLit accepted an unrepresentable variable")
	}
	if _, err := VarOf(Lit(1 << 31)); err == nil {
		t.Fatal("VarOf accepted a literal outside the compatible range")
	}
	if got, err := LitToString(Lit(0)); err != nil || got != "1" {
		t.Fatalf("LitToString(0)=(%q,%v)", got, err)
	}
	if got, err := LitToString(Lit(3)); err != nil || got != "-2" {
		t.Fatalf("LitToString(3)=(%q,%v)", got, err)
	}
}

func TestUndeclaredLiteralGrowthAndPublicValueStates(t *testing.T) {
	solver, _ := NewSolver()
	lit, _ := MakeLit(2, false)
	if got, err := solver.Value(lit); err != nil || got != Undef {
		t.Fatalf("unallocated Value = %v, %v", got, err)
	}
	if ok, err := solver.AddClause([]Lit{lit}); err != nil || !ok {
		t.Fatalf("AddClause growth: %t, %v", ok, err)
	}
	if solver.NVars() != 3 {
		t.Fatalf("AddClause allocated %d variables, want 3", solver.NVars())
	}
	if got, err := solver.Solve(); err != nil || got != SolveSat {
		t.Fatalf("solve = %v, %v", got, err)
	}
	model, valid := solver.Model()
	if !valid || len(model) != 3 || !model[2] {
		t.Fatalf("grown-variable model = %v, valid=%t", model, valid)
	}
	limited, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 8, MaxClauseLiterals: 8, MaxStoredLiterals: 8})
	before := limited.NVars()
	if _, err := limited.AddClause([]Lit{lit}); err == nil || limited.NVars() != before {
		t.Fatal("resource refusal was not atomic")
	}
	if _, err := limited.SolveWithAssumptions([]Lit{lit}); err == nil || limited.NVars() != before {
		t.Fatal("assumption resource refusal was not atomic")
	}
	if Undef != 0 || True != 1 || False != -1 {
		t.Fatalf("public LBool codes: undef=%d true=%d false=%d", Undef, True, False)
	}
}

func TestInspectionAPIsAndNotGate(t *testing.T) {
	s, _ := NewSolver()
	v, _ := s.NewVar()
	a, _ := MakeLit(v, false)
	na := a ^ 1
	if got, err := s.Value(a); err != nil || got != Undef {
		t.Fatalf("unassigned Value: %v, %v", got, err)
	}
	if got := s.VarValue(v); got != Undef {
		t.Fatalf("unassigned VarValue: %v", got)
	}
	if got := s.VarValue(Var(9)); got != Undef {
		t.Fatalf("unallocated VarValue: %v", got)
	}
	if _, err := s.AddClause([]Lit{a}); err != nil {
		t.Fatal(err)
	}
	if got := s.VarValue(v); got != True {
		t.Fatalf("root VarValue: %v", got)
	}
	if got, err := s.Value(na); err != nil || got != False {
		t.Fatalf("negated Value: %v, %v", got, err)
	}
	if got, err := s.Value(Lit(1 << 31)); err == nil || got != Undef {
		t.Fatal("Value accepted malformed packed literal")
	}
	b, _ := NewCnfBuilder(nil)
	if v, err := b.NewVar(); err != nil || v != 0 {
		t.Fatalf("builder NewVar: %d %v", v, err)
	}
	x, _ := b.NewLit()
	nx, err := b.Not(x)
	if err != nil || nx != x^1 {
		t.Fatalf("Not: %v, %v", nx, err)
	}
}

func TestValueQueriesRemainAlignedWithSATModel(t *testing.T) {
	s, _ := NewSolver()
	aVar, _ := s.NewVar()
	bVar, _ := s.NewVar()
	a, _ := MakeLit(aVar, false)
	if _, err := s.AddClause([]Lit{a}); err != nil {
		t.Fatal(err)
	}
	if got, err := s.Solve(); err != nil || got != SolveSat {
		t.Fatalf("solve=%v %v", got, err)
	}
	model, valid := s.Model()
	if !valid || len(model) != 2 {
		t.Fatalf("model=%v valid=%t", model, valid)
	}
	for v, want := range model {
		if got := s.VarValue(Var(v)); (got == True) != want || got == Undef {
			t.Fatalf("VarValue(%d)=%v for model=%v", v, got, model)
		}
		pos, _ := MakeLit(Var(v), false)
		neg := pos ^ 1
		positive, err := s.Value(pos)
		if err != nil || (positive == True) != want {
			t.Fatalf("Value(positive %d)=%v want=%t err=%v", v, positive, want, err)
		}
		negative, err := s.Value(neg)
		if err != nil || (negative == True) == want {
			t.Fatalf("Value(negative %d)=%v want negation of %t err=%v", v, negative, want, err)
		}
	}
	if _, err := s.NewVar(); err != nil {
		t.Fatal(err)
	}
	if _, valid := s.Model(); valid {
		t.Fatal("NewVar retained old model")
	}
	if got := s.VarValue(aVar); got != True {
		t.Fatalf("NewVar discarded permanent root assignment: %v", got)
	}
	s.Reset()
	if _, valid := s.Model(); valid {
		t.Fatal("Reset retained old model")
	}
	if got := s.VarValue(bVar); got != Undef {
		t.Fatalf("Reset retained a decision assignment: %v", got)
	}
}

func TestParserResourceCeilings(t *testing.T) {
	s, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 2, MaxClauseLiterals: 2, MaxStoredLiterals: 2})
	a, _ := MakeLit(0, false)
	b, _ := MakeLit(1, false)
	if _, err := s.AddClause([]Lit{a, b}); err != nil {
		t.Fatal(err)
	}
	beforeVars, beforeClauses := s.NVars(), s.NClauses()
	tooWide := Lit(4)
	if _, err := s.AddClause([]Lit{a, tooWide}); err == nil {
		t.Fatal("AddClause exceeded variable cap")
	}
	if s.NVars() != beforeVars || s.NClauses() != beforeClauses {
		t.Fatal("variable-cap refusal mutated solver")
	}
	if _, err := NewSolver(SolverConfig{MaxVars: (1 << 30) + 1}); err == nil {
		t.Fatal("NewSolver allowed ceiling beyond encoding")
	}
	if _, err := ParseDimacs("p cnf 1073741825 0\n", ParseDimacsOptions{MaxVars: 1073741825}); err == nil {
		t.Fatal("DIMACS accepted variables outside literal representation")
	}
	limitedLits, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 8, MaxClauseLiterals: 2, MaxStoredLiterals: 1})
	u0, _ := MakeLit(0, false)
	u1, _ := MakeLit(1, false)
	_, _ = limitedLits.NewVar()
	_, _ = limitedLits.NewVar()
	if ok, err := limitedLits.AddClause([]Lit{u0}); err != nil || !ok {
		t.Fatalf("first root literal: %t %v", ok, err)
	}
	if _, err := limitedLits.AddClause([]Lit{u1}); err == nil {
		t.Fatal("root unit bypassed stored literal ceiling")
	}
	if limitedLits.NVars() != 2 || limitedLits.NClauses() != 0 || limitedLits.VarValue(0) != True || limitedLits.VarValue(1) != Undef {
		t.Fatal("root-literal quota refusal was not atomic")
	}
	totalLimit, _ := NewSolver(SolverConfig{MaxVars: 3, MaxClauses: 8, MaxClauseLiterals: 2, MaxStoredLiterals: 4})
	t0, _ := MakeLit(0, false)
	t1, _ := MakeLit(1, false)
	t2, _ := MakeLit(2, false)
	for i := 0; i < 3; i++ {
		_, _ = totalLimit.NewVar()
	}
	for _, l := range []Lit{t0, t1} {
		if _, err := totalLimit.AddClause([]Lit{l}); err != nil {
			t.Fatal(err)
		}
	}
	beforeVars, beforeCount := totalLimit.NVars(), totalLimit.inputClauseCount
	if _, err := totalLimit.AddClause([]Lit{t2}); err == nil {
		t.Fatal("total original literal ceiling not enforced")
	}
	if totalLimit.NVars() != beforeVars || totalLimit.inputClauseCount != beforeCount || totalLimit.VarValue(2) != Undef {
		t.Fatal("total-original-literal refusal mutated solver")
	}
}

func TestDIMACSOptionAndWriterRefusals(t *testing.T) {
	if _, err := ParseDimacs("p cnf 0 0", ParseDimacsOptions{}, ParseDimacsOptions{}); err == nil {
		t.Fatal("parser accepted multiple options")
	}
	if _, err := ParseDimacs("p cnf 1 0\n", ParseDimacsOptions{MaxVars: 0}); err != nil {
		t.Fatal(err)
	}
	if _, err := WriteDimacs(CnfFormula{NumVars: 1, Clauses: [][]Lit{{2}}}); err == nil {
		t.Fatal("writer accepted undeclared literal")
	}
	if _, err := WriteDimacs(CnfFormula{NumVars: maxVar + 2}); err == nil {
		t.Fatal("writer accepted unrepresentable variable count")
	}
}

func TestConflictBudgetAndRootConflictPrecedence(t *testing.T) {
	build := func() *Solver {
		s, _ := NewSolver()
		x, _ := s.NewVar()
		y, _ := s.NewVar()
		a, _ := MakeLit(x, false)
		b, _ := MakeLit(y, false)
		for _, cl := range [][]Lit{{a, b}, {a, b ^ 1}, {a ^ 1, b}, {a ^ 1, b ^ 1}} {
			_, _ = s.AddClause(cl)
		}
		return s
	}
	s := build()
	zero := uint64(0)
	if got, err := s.Solve(SolverOptions{MaxConflicts: &zero}); err != nil || got != SolveUnknown {
		t.Fatalf("zero conflict budget=%v %v", got, err)
	}
	if got, err := s.Solve(); err != nil || got != SolveUnsat {
		t.Fatalf("unbounded recovery=%v %v", got, err)
	}
	root := build()
	one := uint64(1)
	if got, err := root.Solve(SolverOptions{MaxConflicts: &one}); err != nil || got != SolveUnsat {
		t.Fatalf("observed root conflict did not outrank limit=%v %v", got, err)
	}
	if root.Okay() {
		t.Fatal("root conflict was not sticky")
	}
}

func TestPublicRefusalPathsAndBuilderPreflight(t *testing.T) {
	bad := Lit(1 << 31)
	if _, err := NegateLit(bad); err == nil {
		t.Fatal("NegateLit accepted malformed value")
	}
	if _, err := SignOf(bad); err == nil {
		t.Fatal("SignOf accepted malformed value")
	}
	if _, err := NewSolver(SolverConfig{MaxVars: (1 << 30) + 1}); err == nil {
		t.Fatal("invalid representation config accepted")
	}
	for _, cfg := range []SolverConfig{{MaxVars: 1 << 21}, {MaxClauses: 1 << 21}, {MaxClauseLiterals: 1 << 25}, {MaxStoredLiterals: 1 << 25}} {
		if _, err := NewSolver(cfg); err == nil {
			t.Fatalf("raised hard resource ceiling accepted: %+v", cfg)
		}
	}
	if _, err := NewSolver(SolverConfig{}, SolverConfig{}); err == nil {
		t.Fatal("multiple configs accepted")
	}
	limited, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 2, MaxClauseLiterals: 4, MaxStoredLiterals: 4})
	builder, _ := NewCnfBuilder(limited)
	a, _ := builder.NewLit()
	b, _ := builder.NewLit()
	if _, err := builder.Or(a, b); err == nil {
		t.Fatal("Or ignored clause ceiling")
	}
	if _, err := builder.Xor(a, b); err == nil {
		t.Fatal("Xor ignored clause ceiling")
	}
	if _, err := builder.Equiv(a, b); err == nil {
		t.Fatal("Equiv ignored clause ceiling")
	}
	if _, err := builder.Ite(a, b, a); err == nil {
		t.Fatal("Ite ignored clause ceiling")
	}
	if _, err := builder.FullAdder(a, b, a); err == nil {
		t.Fatal("FullAdder ignored clause ceiling")
	}
	if _, err := builder.BvAdd([]Lit{a}, []Lit{b}); err == nil {
		t.Fatal("BvAdd ignored clause ceiling")
	}
	if _, err := builder.BvEq([]Lit{a}, []Lit{b}); err == nil {
		t.Fatal("BvEq ignored clause ceiling")
	}
	if _, err := builder.Not(bad); err == nil {
		t.Fatal("Not accepted malformed literal")
	}
	if solverVars := limited.NVars(); solverVars != 2 {
		t.Fatalf("failed builder operations changed variable count: %d", solverVars)
	}
	if limited.inputLiteralCount != 0 || limited.inputClauseCount != 0 || limited.NClauses() != 0 {
		t.Fatal("gate preflight left partial formula state")
	}
	maxed, _ := NewSolver(SolverConfig{MaxVars: 1, MaxClauses: 8, MaxClauseLiterals: 8, MaxStoredLiterals: 8})
	bb, _ := NewCnfBuilder(maxed)
	if _, err := bb.NewLit(); err != nil {
		t.Fatal(err)
	}
	if _, err := bb.NewLit(); err == nil {
		t.Fatal("NewLit ignored variable ceiling")
	}
	if _, err := maxed.Solve(SolverOptions{}, SolverOptions{}); err == nil {
		t.Fatal("Solve accepted multiple options")
	}
	if _, err := maxed.SolveWithAssumptions([]Lit{bad}); err == nil {
		t.Fatal("Solve accepted malformed assumption")
	}
}

func TestLearnedArenaHasHardPhysicalEntryLimit(t *testing.T) {
	s, _ := NewSolver(SolverConfig{MaxVars: 1, MaxClauses: 1, MaxClauseLiterals: 8, MaxStoredLiterals: 2})
	_, _ = s.NewVar()
	literal, _ := MakeLit(0, false)
	for range 3 {
		id, ok := s.addLearned([]Lit{literal})
		if !ok {
			t.Fatal("learned entry refused before physical arena limit")
		}
		s.cs[id].deleted = true
		s.cs[id].lits = nil
	}
	if _, ok := s.addLearned([]Lit{literal}); ok {
		t.Fatal("learned tombstones bypassed physical clause arena ceiling")
	}
	if len(s.cs) != 3 {
		t.Fatalf("physical arena length=%d want=3", len(s.cs))
	}
}

func TestLearnedReductionKeepsReasonLockedAndBinaryClauses(t *testing.T) {
	s, _ := NewSolver()
	for i := 0; i < 4; i++ {
		_, _ = s.NewVar()
	}
	l0, _ := MakeLit(0, false)
	l1, _ := MakeLit(1, false)
	l2, _ := MakeLit(2, false)
	l3, _ := MakeLit(3, false)
	locked, _ := s.addLearned([]Lit{l0, l1, l2})
	binary, _ := s.addLearned([]Lit{l0, l1})
	candidate, _ := s.addLearned([]Lit{l1, l2, l3})
	candidate2, _ := s.addLearned([]Lit{l0, l2, l3})
	s.reason[0] = locked
	s.learnedLimit = 1
	s.reduceLearned()
	if s.cs[locked].deleted {
		t.Fatal("reduction deleted a clause locked as a live reason")
	}
	if s.cs[binary].deleted {
		t.Fatal("reduction deleted a binary learned clause")
	}
	if s.cs[candidate].deleted == s.cs[candidate2].deleted || (s.cs[candidate].deleted && len(s.cs[candidate].lits) != 0) || (s.cs[candidate2].deleted && len(s.cs[candidate2].lits) != 0) {
		t.Fatal("reduction did not reclaim an unlocked long learned clause")
	}
	if s.NClauses() != 3 {
		t.Fatalf("NClauses after reduction=%d want3", s.NClauses())
	}
	for _, bucket := range s.watches {
		if cap(bucket) != len(bucket) {
			t.Fatalf("reduction retained historical watch capacity: len=%d cap=%d", len(bucket), cap(bucket))
		}
	}
}

func TestUnknownPreservesPendingRootPropagation(t *testing.T) {
	solver, _ := NewSolver()
	x, _ := solver.NewVar()
	y, _ := solver.NewVar()
	z, _ := solver.NewVar()
	xl, _ := MakeLit(x, false)
	yl, _ := MakeLit(y, false)
	zl, _ := MakeLit(z, false)
	for _, cl := range [][]Lit{{xl, yl}, {xl, yl ^ 1}, {xl ^ 1, zl}} {
		if _, err := solver.AddClause(cl); err != nil {
			t.Fatal(err)
		}
	}
	one := uint64(1)
	if got, err := solver.Solve(SolverOptions{MaxSteps: &one}); err != nil || got != SolveUnknown {
		t.Fatalf("bounded solve = %v, %v", got, err)
	}
	if got := solver.Propagate(^uint64(0)); got != -1 {
		t.Fatalf("retry Propagate conflict = %d", got)
	}
	if got := solver.VarValue(z); got != True {
		t.Fatalf("pending root propagation lost z=true: %v", got)
	}
}

func TestAssumptionBackjumpLeavesPendingLearnedRootFacts(t *testing.T) {
	for _, mirrored := range []bool{false, true} {
		t.Run(fmt.Sprint("mirrored_", mirrored), func(t *testing.T) {
			build := func() *Solver {
				solver, err := NewSolver()
				if err != nil {
					t.Fatal(err)
				}
				for i := 0; i < 4; i++ {
					if _, err := solver.NewVar(); err != nil {
						t.Fatal(err)
					}
				}
				x, _ := MakeLit(0, mirrored)
				y, _ := MakeLit(1, mirrored)
				z, _ := MakeLit(2, mirrored)
				for _, clause := range [][]Lit{{x, y}, {x, y ^ 1}, {x ^ 1, z}} {
					if _, err := solver.AddClause(clause); err != nil {
						t.Fatal(err)
					}
				}
				if mirrored {
					solver.phase[0] = 1
				} else {
					solver.phase[0] = -1
				}
				return solver
			}
			xExpected := True
			if mirrored {
				xExpected = False
			}
			a, _ := MakeLit(3, false)
			budget := uint64(2)
			solver := build()
			if got, err := solver.SolveWithAssumptions([]Lit{a}, SolverOptions{MaxSteps: &budget}); err != nil || got != SolveUnknown {
				t.Fatalf("bounded solve=%v %v", got, err)
			}
			if got := solver.Propagate(^uint64(0)); got != -1 {
				t.Fatalf("pending root propagation conflict=%d", got)
			}
			if solver.VarValue(0) != xExpected || solver.VarValue(2) != xExpected {
				t.Fatalf("lost root implications: x=%v z=%v want=%v", solver.VarValue(0), solver.VarValue(2), xExpected)
			}
			if solver.VarValue(3) != Undef {
				t.Fatal("assumption leaked into root state")
			}
			if got, err := solver.Solve(); err != nil || got != SolveSat {
				t.Fatalf("unrestricted recovery=%v %v", got, err)
			}
			model, valid := solver.Model()
			clauses := [][]Lit{{mustLit(t, 0, mirrored), mustLit(t, 1, mirrored)}, {mustLit(t, 0, mirrored), mustLit(t, 1, mirrored) ^ 1}, {mustLit(t, 0, mirrored) ^ 1, mustLit(t, 2, mirrored)}}
			if !valid || !allClausesSatisfied(clauses, model) {
				t.Fatalf("recovered model invalid: valid=%v model=%v", valid, model)
			}
			resetCase := build()
			if got, err := resetCase.SolveWithAssumptions([]Lit{a}); err != nil || got != SolveSat {
				t.Fatalf("SAT assumption control=%v %v", got, err)
			}
			resetCase.Reset()
			if resetCase.VarValue(0) != xExpected || resetCase.VarValue(2) != xExpected || resetCase.VarValue(3) != Undef {
				t.Fatalf("Reset lost learned root facts or retained assumption: x=%v z=%v a=%v", resetCase.VarValue(0), resetCase.VarValue(2), resetCase.VarValue(3))
			}
		})
	}
}

func mustLit(t *testing.T, v Var, neg bool) Lit {
	t.Helper()
	l, err := MakeLit(v, neg)
	if err != nil {
		t.Fatal(err)
	}
	return l
}

func TestKnownRootUnsatSurvivesAssumptionsAndBudget(t *testing.T) {
	s, _ := NewSolver()
	x, _ := s.NewVar()
	y, _ := s.NewVar()
	a, _ := MakeLit(x, false)
	b, _ := MakeLit(y, false)
	for _, cl := range [][]Lit{{a, b}, {a, b ^ 1}, {a ^ 1, b}, {a ^ 1, b ^ 1}} {
		if _, err := s.AddClause(cl); err != nil {
			t.Fatal(err)
		}
	}
	if got, err := s.Solve(); err != nil || got != SolveUnsat {
		t.Fatalf("base solve=%v %v", got, err)
	}
	if s.Okay() {
		t.Fatal("root UNSAT was not recorded")
	}
	if got, err := s.SolveWithAssumptions([]Lit{a}); err != nil || got != SolveUnsat {
		t.Fatalf("assumption after root UNSAT=%v %v", got, err)
	}
	if s.Okay() {
		t.Fatal("assumption solve cleared known root UNSAT")
	}
	zero := uint64(0)
	if got, err := s.Solve(SolverOptions{MaxSteps: &zero}); err != nil || got != SolveUnsat {
		t.Fatalf("zero-budget known root UNSAT=%v %v", got, err)
	}
}

func TestPublicPropagatePersistsRootConflictBeforeBudgetedRetry(t *testing.T) {
	s, _ := NewSolver()
	x, _ := s.NewVar()
	y, _ := s.NewVar()
	a, _ := MakeLit(x, false)
	b, _ := MakeLit(y, false)
	for _, cl := range [][]Lit{{a, b}, {a, b ^ 1}, {a ^ 1, b}, {a ^ 1, b ^ 1}} {
		if _, err := s.AddClause(cl); err != nil {
			t.Fatal(err)
		}
	}
	one := uint64(1)
	if got, err := s.Solve(SolverOptions{MaxSteps: &one}); err != nil || got != SolveUnknown {
		t.Fatalf("bounded base=%v %v", got, err)
	}
	conflict := s.Propagate(^uint64(0))
	if conflict < 0 {
		t.Fatalf("public Propagate missed permanent root conflict: %d", conflict)
	}
	if s.Okay() {
		t.Fatal("public Propagate did not persist root contradiction")
	}
	budget := uint64(1)
	if got, err := s.Solve(SolverOptions{MaxConflicts: &budget}); err != nil || got != SolveUnsat {
		t.Fatalf("known root conflict did not outrank budget: %v %v", got, err)
	}
}

func TestAssumptionBackjumpDoesNotMakeBaseFormulaUnsat(t *testing.T) {
	for _, thirdNegated := range []bool{false, true} {
		t.Run(fmt.Sprint("thirdNegated_", thirdNegated), func(t *testing.T) {
			solver, _ := NewSolver()
			for i := 0; i < 3; i++ {
				_, _ = solver.NewVar()
			}
			a, _ := MakeLit(0, false)
			b, _ := MakeLit(1, false)
			z, _ := MakeLit(2, thirdNegated)
			if _, err := solver.AddClause([]Lit{z ^ 1, b}); err != nil {
				t.Fatal(err)
			}
			if _, err := solver.AddClause([]Lit{z ^ 1, b ^ 1}); err != nil {
				t.Fatal(err)
			}
			// Force the heuristic onto the wrong phase first so this exercises a
			// learned consequence whose natural backjump crosses the assumption.
			solver.activity[2] = 100
			if thirdNegated {
				solver.phase[2] = -1
			} else {
				solver.phase[2] = 1
			}
			if got, err := solver.SolveWithAssumptions([]Lit{a}); err != nil || got != SolveSat {
				t.Fatalf("assumption solve = %v, %v", got, err)
			}
			if !solver.Okay() {
				t.Fatal("assumption conflict poisoned permanent formula")
			}
			if got, err := solver.Solve(); err != nil || got != SolveSat {
				t.Fatalf("unrestricted solve = %v, %v", got, err)
			}
		})
	}
}

func TestSolverIncrementalBudgetsAndSnapshotOwnership(t *testing.T) {
	solver, err := NewSolver()
	if err != nil {
		t.Fatal(err)
	}
	a, _ := MakeLit(0, false)
	b, _ := MakeLit(1, false)
	if _, err := solver.NewVar(); err != nil {
		t.Fatal(err)
	}
	if _, err := solver.NewVar(); err != nil {
		t.Fatal(err)
	}
	if ok, err := solver.AddClause([]Lit{a, b}); err != nil || !ok {
		t.Fatalf("AddClause: ok=%t err=%v", ok, err)
	}
	zero := uint64(0)
	if got, err := solver.Solve(SolverOptions{MaxSteps: &zero}); err != nil || got != SolveUnknown {
		t.Fatalf("zero-step solve: %v, %v", got, err)
	}
	if got, err := solver.Solve(); err != nil || got != SolveSat {
		t.Fatalf("unbounded recovery: %v, %v", got, err)
	}
	model, valid := solver.Model()
	if !valid || len(model) != 2 || !allClausesSatisfied([][]Lit{{a, b}}, model) {
		t.Fatalf("bad model snapshot: %v valid=%t", model, valid)
	}
	model[0] = !model[0]
	second, secondValid := solver.Model()
	if !secondValid || !allClausesSatisfied([][]Lit{{a, b}}, second) {
		t.Fatalf("mutating returned model changed solver state: %v", second)
	}
	if ok, err := solver.AddClause([]Lit{a}); err != nil || !ok {
		t.Fatalf("AddClause unit: ok=%t err=%v", ok, err)
	}
	if model, valid := solver.Model(); valid || model != nil {
		t.Fatalf("AddClause retained a stale model: %v valid=%t", model, valid)
	}
	if got, err := solver.SolveWithAssumptions([]Lit{negateForTest(t, a)}); err != nil || got != SolveUnsat {
		t.Fatalf("assumption conflict: %v, %v", got, err)
	}
	core := solver.FailedCore()
	if !reflect.DeepEqual(core, []Lit{negateForTest(t, a)}) {
		t.Fatalf("failed core %v", core)
	}
	core[0] = a
	if !reflect.DeepEqual(solver.FailedCore(), []Lit{negateForTest(t, a)}) {
		t.Fatal("mutating returned core changed solver state")
	}
	if got, err := solver.SolveWithAssumptions(nil); err != nil || got != SolveSat {
		t.Fatalf("assumption cleanup: %v, %v", got, err)
	}
	if len(solver.FailedCore()) != 0 {
		t.Fatalf("SAT retained a stale failed core: %v", solver.FailedCore())
	}
}

func negateForTest(t *testing.T, lit Lit) Lit {
	t.Helper()
	got, err := NegateLit(lit)
	if err != nil {
		t.Fatal(err)
	}
	return got
}

func TestKnownRootContradictionClearsPriorFailedCore(t *testing.T) {
	solver, err := NewSolver()
	if err != nil {
		t.Fatal(err)
	}
	a, _ := MakeLit(0, false)
	b, _ := MakeLit(1, false)
	_, _ = solver.NewVar()
	_, _ = solver.NewVar()
	if ok, err := solver.AddClause([]Lit{negateForTest(t, a), b}); err != nil || !ok {
		t.Fatalf("initial clause: ok=%t err=%v", ok, err)
	}
	if got, err := solver.SolveWithAssumptions([]Lit{a, negateForTest(t, b)}); err != nil || got != SolveUnsat {
		t.Fatalf("assumption UNSAT: %v, %v", got, err)
	}
	if len(solver.FailedCore()) == 0 {
		t.Fatal("expected a nonempty first failed core")
	}
	if ok, err := solver.AddClause([]Lit{a}); err != nil || !ok {
		t.Fatalf("root unit add: ok=%t err=%v", ok, err)
	}
	if ok, err := solver.AddClause([]Lit{negateForTest(t, a)}); err != nil || ok {
		t.Fatalf("contradictory root unit add: ok=%t err=%v", ok, err)
	}
	if got, err := solver.SolveWithAssumptions([]Lit{b}); err != nil || got != SolveUnsat {
		t.Fatalf("known permanent contradiction: %v, %v", got, err)
	}
	if core := solver.FailedCore(); len(core) != 0 {
		t.Fatalf("known root contradiction returned stale core %v", core)
	}
}

func TestFailedCoreIsSoundButNeedNotBeMinimal(t *testing.T) {
	formula, err := ParseDimacs("p cnf 4 5\n-1 -2 0\n-1 3 4 0\n-1 3 -4 0\n-1 -3 4 0\n-1 -3 -4 0\n")
	if err != nil {
		t.Fatal(err)
	}
	solver, _ := NewSolver()
	for i := Var(0); uint32(i) < formula.NumVars; i++ {
		if _, err := solver.NewVar(); err != nil {
			t.Fatal(err)
		}
	}
	for _, clause := range formula.Clauses {
		if _, err := solver.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	a, _ := MakeLit(0, false)
	b, _ := MakeLit(1, false)
	assumptions := []Lit{a, b}
	if result, err := solver.SolveWithAssumptions(assumptions); err != nil || result != SolveUnsat {
		t.Fatalf("SolveWithAssumptions: %v, %v", result, err)
	}
	core := solver.FailedCore()
	if len(core) == 0 || hasDuplicateLits(core) || !isSubset(core, assumptions) {
		t.Fatalf("invalid core %v", core)
	}
	if oracleSAT(int(formula.NumVars), formula.Clauses, core) {
		t.Fatalf("core is not sound: %v", core)
	}
}

func TestDIMACSStrictnessRoundTripAndLimits(t *testing.T) {
	input := "c test\r\np cnf 3 3\r\n1\r\n-2 0 3 0\r\n0\r\n"
	formula, err := ParseDimacs(input)
	if err != nil {
		t.Fatal(err)
	}
	if formula.NumVars != 3 || len(formula.Clauses) != 3 || len(formula.Clauses[2]) != 0 {
		t.Fatalf("unexpected parsed formula: %+v", formula)
	}
	serialized, err := WriteDimacs(formula)
	if err != nil {
		t.Fatal(err)
	}
	roundTrip, err := ParseDimacs(serialized)
	if err != nil || !reflect.DeepEqual(formula, roundTrip) {
		t.Fatalf("round trip mismatch: %+v -> %q -> %+v (%v)", formula, serialized, roundTrip, err)
	}
	if serialized[len(serialized)-2:] != " 0" {
		t.Fatalf("empty clause writer spelling = %q", serialized[len(serialized)-2:])
	}
	malformed := []string{
		"p cnfX 1 0\n", "p cnf 1 0 extra\n", "p cnf +1 0\n", "p cnf 1 1\n+1 0\n", "p cnf 1 1\n1\n",
		"p cnf 1 1\n2 0\n", "p cnf 1 0\n1 0\n", "p cnf -1 0\n",
		"p cnf 1 1\n1 bad 0\n", "p cnf 1 1\n0 0\n",
	}
	for _, text := range malformed {
		if _, err := ParseDimacs(text); err == nil {
			t.Errorf("ParseDimacs accepted malformed input %q", text)
		}
	}
	if _, err := ParseDimacs("p cnf 100 0\n", ParseDimacsOptions{MaxVars: 99}); err == nil {
		t.Fatal("ParseDimacs ignored MaxVars")
	}
	if got, err := ParseDimacs("p cnf 1 1\n1 0\n%\nmalformed trailing\n"); err != nil || len(got.Clauses) != 1 {
		t.Fatalf("SATLIB terminator: %+v, %v", got, err)
	}
}

func TestAllTseitinGateTruthTablesAndBitVectors(t *testing.T) {
	for _, gate := range []string{"and", "or", "xor", "equiv", "ite"} {
		t.Run(gate, func(t *testing.T) {
			solver, _ := NewSolver()
			builder, err := NewCnfBuilder(solver)
			if err != nil {
				t.Fatal(err)
			}
			a, _ := builder.NewLit()
			b, _ := builder.NewLit()
			c, _ := builder.NewLit()
			var out Lit
			switch gate {
			case "and":
				out, err = builder.And(a, b)
			case "or":
				out, err = builder.Or(a, b)
			case "xor":
				out, err = builder.Xor(a, b)
			case "equiv":
				out, err = builder.Equiv(a, b)
			case "ite":
				out, err = builder.Ite(a, b, c)
			}
			if err != nil {
				t.Fatal(err)
			}
			for av := 0; av < 2; av++ {
				for bv := 0; bv < 2; bv++ {
					for cv := 0; cv < 2; cv++ {
						inputs := []int{av, bv, cv}
						assumptions := make([]Lit, 0, 4)
						for i, lit := range []Lit{a, b, c} {
							assumptions = append(assumptions, truthLit(t, lit, inputs[i] == 1))
						}
						want := gateValue(gate, av == 1, bv == 1, cv == 1)
						assumptions = append(assumptions, truthLit(t, out, want))
						if got, err := solver.SolveWithAssumptions(assumptions); err != nil || got != SolveSat {
							t.Fatalf("inputs %v expected %t: %v (%v)", inputs, want, got, err)
						}
						assumptions[len(assumptions)-1] = truthLit(t, out, !want)
						if got, err := solver.SolveWithAssumptions(assumptions); err != nil || got != SolveUnsat {
							t.Fatalf("wrong output for inputs %v: %v (%v)", inputs, got, err)
						}
					}
				}
			}
		})
	}

	builder, _ := NewCnfBuilder(nil)
	a, _ := builder.NewLit()
	b, _ := builder.NewLit()
	cin, _ := builder.NewLit()
	fa, err := builder.FullAdder(a, b, cin)
	if err != nil {
		t.Fatal(err)
	}
	for av := 0; av < 2; av++ {
		for bv := 0; bv < 2; bv++ {
			for cv := 0; cv < 2; cv++ {
				total := av + bv + cv
				assumptions := []Lit{truthLit(t, a, av == 1), truthLit(t, b, bv == 1), truthLit(t, cin, cv == 1), truthLit(t, fa.Sum, total%2 == 1), truthLit(t, fa.Carry, total >= 2)}
				if got, err := builder.GetSolver().SolveWithAssumptions(assumptions); err != nil || got != SolveSat {
					t.Fatalf("full adder %d%d%d: %v (%v)", av, bv, cv, got, err)
				}
				bad := append([]Lit(nil), assumptions...)
				bad[len(bad)-2] = truthLit(t, fa.Sum, total%2 == 0)
				if got, err := builder.GetSolver().SolveWithAssumptions(bad); err != nil || got != SolveUnsat {
					t.Fatalf("full adder accepted wrong sum %d%d%d: %v (%v)", av, bv, cv, got, err)
				}
			}
		}
	}

	if _, err := builder.BvEq([]Lit{a}, []Lit{a, b}); err == nil {
		t.Fatal("BvEq accepted width mismatch")
	}
	if _, err := builder.BvAdd([]Lit{a}, []Lit{a, b}); err == nil {
		t.Fatal("BvAdd accepted width mismatch")
	}
	emptyEq, err := builder.BvEq(nil, nil)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := builder.GetSolver().SolveWithAssumptions([]Lit{emptyEq}); err != nil || got != SolveSat {
		t.Fatalf("empty bvEq: %v (%v)", got, err)
	}
}

func truthLit(t *testing.T, lit Lit, truth bool) Lit {
	t.Helper()
	if truth {
		return lit
	}
	return negateForTest(t, lit)
}

func gateValue(gate string, a, b, c bool) bool {
	switch gate {
	case "and":
		return a && b
	case "or":
		return a || b
	case "xor":
		return a != b
	case "equiv":
		return a == b
	case "ite":
		if a {
			return b
		}
		return c
	default:
		panic("unknown test gate")
	}
}

func TestFullAdderRippleCarryAndMutantCounterexample(t *testing.T) {
	builder, _ := NewCnfBuilder(nil)
	a, _ := builder.NewLit()
	b, _ := builder.NewLit()
	cin, _ := builder.NewLit()
	fa, err := builder.FullAdder(a, b, cin)
	if err != nil {
		t.Fatal(err)
	}
	for av := 0; av < 2; av++ {
		for bv := 0; bv < 2; bv++ {
			for cv := 0; cv < 2; cv++ {
				sum := av + bv + cv
				assumptions := []Lit{truthLit(t, a, av == 1), truthLit(t, b, bv == 1), truthLit(t, cin, cv == 1), truthLit(t, fa.Sum, sum%2 == 1), truthLit(t, fa.Carry, sum >= 2)}
				if got, err := builder.GetSolver().SolveWithAssumptions(assumptions); err != nil || got != SolveSat {
					t.Fatalf("full adder %d%d%d: %v (%v)", av, bv, cv, got, err)
				}
			}
		}
	}

	ripple, _ := NewCnfBuilder(nil)
	x := make([]Lit, 4)
	y := make([]Lit, 4)
	for i := range x {
		x[i], _ = ripple.NewLit()
		y[i], _ = ripple.NewLit()
	}
	sum, err := ripple.BvAdd(x, y)
	if err != nil {
		t.Fatal(err)
	}
	for xv := 0; xv < 16; xv++ {
		for yv := 0; yv < 16; yv++ {
			assumptions := make([]Lit, 0, 12)
			for bit := 0; bit < 4; bit++ {
				assumptions = append(assumptions, truthLit(t, x[bit], xv&(1<<bit) != 0), truthLit(t, y[bit], yv&(1<<bit) != 0), truthLit(t, sum[bit], (xv+yv)&(1<<bit) != 0))
			}
			if got, err := ripple.GetSolver().SolveWithAssumptions(assumptions); err != nil || got != SolveSat {
				t.Fatalf("bvAdd %d + %d: %v (%v)", xv, yv, got, err)
			}
			bad := append([]Lit(nil), assumptions...)
			bad[2] = truthLit(t, sum[0], (xv+yv)&1 == 0)
			if got, err := ripple.GetSolver().SolveWithAssumptions(bad); err != nil || got != SolveUnsat {
				t.Fatalf("bvAdd accepted wrong low bit for %d + %d: %v (%v)", xv, yv, got, err)
			}
		}
	}
	eqBuilder, _ := NewCnfBuilder(nil)
	ea, _ := eqBuilder.NewLit()
	eb, _ := eqBuilder.NewLit()
	eq, err := eqBuilder.BvEq([]Lit{ea}, []Lit{eb})
	if err != nil {
		t.Fatal(err)
	}
	for av := 0; av < 2; av++ {
		for bv := 0; bv < 2; bv++ {
			want := av == bv
			assumptions := []Lit{truthLit(t, ea, av == 1), truthLit(t, eb, bv == 1), truthLit(t, eq, want)}
			if got, err := eqBuilder.GetSolver().SolveWithAssumptions(assumptions); err != nil || got != SolveSat {
				t.Fatalf("bvEq %d=%d: %v (%v)", av, bv, got, err)
			}
			assumptions[2] = truthLit(t, eq, !want)
			if got, err := eqBuilder.GetSolver().SolveWithAssumptions(assumptions); err != nil || got != SolveUnsat {
				t.Fatalf("bvEq accepted wrong result %d=%d: %v (%v)", av, bv, got, err)
			}
		}
	}

	// Mutant control: omit AND's reverse implication clauses. The broken gate
	// admits a=1,b=1,out=0, which the independent truth table rejects; replay
	// the same assignment through the real builder and require UNSAT.
	mutant, _ := NewCnfBuilder(nil)
	ma, _ := mutant.NewLit()
	mb, _ := mutant.NewLit()
	mout, _ := mutant.NewLit()
	for _, clause := range [][]Lit{{negateForTest(t, mout), ma}, {negateForTest(t, mout), mb}} {
		if _, err := mutant.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	wrong := []Lit{ma, mb, negateForTest(t, mout)}
	if got, err := mutant.GetSolver().SolveWithAssumptions(wrong); err != nil || got != SolveSat {
		t.Fatalf("mutant did not expose counterexample: %v (%v)", got, err)
	}
	witness, valid := mutant.GetSolver().Model()
	if !valid {
		t.Fatal("mutant did not return its SAT witness")
	}
	witnessOut := witness[uint32(mout)>>1]
	witnessExpected := witness[uint32(ma)>>1] && witness[uint32(mb)>>1]
	if witnessOut == witnessExpected {
		t.Fatalf("mutant witness does not violate the independent AND truth table: %v", witness)
	}
	healthy, _ := NewCnfBuilder(nil)
	ha, _ := healthy.NewLit()
	hb, _ := healthy.NewLit()
	hout, err := healthy.And(ha, hb)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := healthy.GetSolver().SolveWithAssumptions([]Lit{ha, hb, negateForTest(t, hout)}); err != nil || got != SolveUnsat {
		t.Fatalf("healthy gate accepted mutant witness: %v (%v)", got, err)
	}
}

func TestPigeonholeAndQueensAgainstIndependentChecks(t *testing.T) {
	// PHP(4,3): every pigeon chooses a hole and no hole has two pigeons.
	solver, _ := NewSolver()
	p := make([][]Lit, 4)
	for i := range p {
		p[i] = make([]Lit, 3)
		for j := range p[i] {
			v, err := solver.NewVar()
			if err != nil {
				t.Fatal(err)
			}
			p[i][j], _ = MakeLit(v, false)
		}
	}
	for i := range p {
		if _, err := solver.AddClause(p[i]); err != nil {
			t.Fatal(err)
		}
	}
	for h := 0; h < 3; h++ {
		for i := 0; i < 4; i++ {
			for j := i + 1; j < 4; j++ {
				if _, err := solver.AddClause([]Lit{negateForTest(t, p[i][h]), negateForTest(t, p[j][h])}); err != nil {
					t.Fatal(err)
				}
			}
		}
	}
	solver.restartBase = 1
	solver.learnedLimit = 1
	if got, err := solver.Solve(); err != nil || got != SolveUnsat {
		t.Fatalf("PHP(4,3): %v (%v)", got, err)
	}
	if solver.Restarts == 0 {
		t.Fatal("forced low Luby threshold did not exercise restart path")
	}
	phpClauses := make([][]Lit, 0, 4+3*6)
	for i := range p {
		phpClauses = append(phpClauses, append([]Lit(nil), p[i]...))
	}
	for h := 0; h < 3; h++ {
		for i := 0; i < 4; i++ {
			for j := i + 1; j < 4; j++ {
				phpClauses = append(phpClauses, []Lit{negateForTest(t, p[i][h]), negateForTest(t, p[j][h])})
			}
		}
	}
	if oracleSAT(12, phpClauses, nil) {
		t.Fatal("independent enumeration found a model for PHP(4,3)")
	}
	phpSat, _ := NewSolver()
	phpSatVars := make([][]Lit, 3)
	for i := range phpSatVars {
		phpSatVars[i] = make([]Lit, 3)
		for j := range phpSatVars[i] {
			v, err := phpSat.NewVar()
			if err != nil {
				t.Fatal(err)
			}
			phpSatVars[i][j], _ = MakeLit(v, false)
		}
	}
	phpSatClauses := make([][]Lit, 0, 3+3*3)
	for i := range phpSatVars {
		phpSatClauses = append(phpSatClauses, append([]Lit(nil), phpSatVars[i]...))
		if _, err := phpSat.AddClause(phpSatVars[i]); err != nil {
			t.Fatal(err)
		}
	}
	for h := 0; h < 3; h++ {
		for i := 0; i < 3; i++ {
			for j := i + 1; j < 3; j++ {
				clause := []Lit{negateForTest(t, phpSatVars[i][h]), negateForTest(t, phpSatVars[j][h])}
				phpSatClauses = append(phpSatClauses, clause)
				if _, err := phpSat.AddClause(clause); err != nil {
					t.Fatal(err)
				}
			}
		}
	}
	if got, err := phpSat.Solve(); err != nil || got != SolveSat {
		t.Fatalf("PHP(3,3) SAT control: %v (%v)", got, err)
	}
	phpModel, phpValid := phpSat.Model()
	if !phpValid || !allClausesSatisfied(phpSatClauses, phpModel) || !oracleSAT(9, phpSatClauses, nil) {
		t.Fatal("PHP(3,3) failed independent SAT control")
	}

	// N-Queens(4): validate the returned board independently.
	qsolver, _ := NewSolver()
	q := make([][]Lit, 4)
	for r := range q {
		q[r] = make([]Lit, 4)
		for c := range q[r] {
			v, err := qsolver.NewVar()
			if err != nil {
				t.Fatal(err)
			}
			q[r][c], _ = MakeLit(v, false)
		}
	}
	for r := 0; r < 4; r++ {
		if _, err := qsolver.AddClause(q[r]); err != nil {
			t.Fatal(err)
		}
		for c1 := 0; c1 < 4; c1++ {
			for c2 := c1 + 1; c2 < 4; c2++ {
				_, err := qsolver.AddClause([]Lit{negateForTest(t, q[r][c1]), negateForTest(t, q[r][c2])})
				if err != nil {
					t.Fatal(err)
				}
			}
		}
	}
	for c := 0; c < 4; c++ {
		for r1 := 0; r1 < 4; r1++ {
			for r2 := r1 + 1; r2 < 4; r2++ {
				if _, err := qsolver.AddClause([]Lit{negateForTest(t, q[r1][c]), negateForTest(t, q[r2][c])}); err != nil {
					t.Fatal(err)
				}
			}
		}
	}
	for r1 := 0; r1 < 4; r1++ {
		for c1 := 0; c1 < 4; c1++ {
			for r2 := r1 + 1; r2 < 4; r2++ {
				for c2 := 0; c2 < 4; c2++ {
					if absInt(r1-r2) == absInt(c1-c2) {
						if _, err := qsolver.AddClause([]Lit{negateForTest(t, q[r1][c1]), negateForTest(t, q[r2][c2])}); err != nil {
							t.Fatal(err)
						}
					}
				}
			}
		}
	}
	if got, err := qsolver.Solve(); err != nil || got != SolveSat {
		t.Fatalf("4-queens: %v (%v)", got, err)
	}
	model, valid := qsolver.Model()
	if !valid {
		t.Fatal("4-queens returned invalid model")
	}
	count := 0
	for r := range q {
		row := 0
		for c := range q[r] {
			if model[q[r][c]>>1] {
				count++
				row++
			}
		}
		if row != 1 {
			t.Fatalf("row %d has %d queens", r, row)
		}
	}
	if count != 4 {
		t.Fatalf("board has %d queens", count)
	}
	for c := 0; c < 4; c++ {
		col := 0
		for r := 0; r < 4; r++ {
			if model[uint32(q[r][c])>>1] {
				col++
			}
		}
		if col != 1 {
			t.Fatalf("column %d has %d queens", c, col)
		}
	}
	for r1 := 0; r1 < 4; r1++ {
		for c1 := 0; c1 < 4; c1++ {
			if !model[uint32(q[r1][c1])>>1] {
				continue
			}
			for r2 := r1 + 1; r2 < 4; r2++ {
				for c2 := 0; c2 < 4; c2++ {
					if model[uint32(q[r2][c2])>>1] && absInt(r1-r2) == absInt(c1-c2) {
						t.Fatalf("attacking queens at (%d,%d) and (%d,%d)", r1, c1, r2, c2)
					}
				}
			}
		}
	}
}

func TestDeterministicDifferentialSATProperties(t *testing.T) {
	rng := newSplitMix64(0x9e3779b97f4a7c15)
	satCount, unsatCount := 0, 0
	for round := 0; round < 120; round++ {
		n := 4 + int(rng.bounded(5))
		m := int(float64(n) * 4.26)
		clauses := make([][]Lit, 0, m)
		solver, _ := NewSolver()
		for i := 0; i < n; i++ {
			if _, err := solver.NewVar(); err != nil {
				t.Fatal(err)
			}
		}
		for c := 0; c < m; c++ {
			clause := make([]Lit, 0, 3)
			for len(clause) < 3 {
				v := Var(rng.bounded(uint64(n)))
				negated := rng.bounded(2) == 1
				lit, _ := MakeLit(v, negated)
				if !containsLit(clause, lit) && !containsLit(clause, negateForTest(t, lit)) {
					clause = append(clause, lit)
				}
			}
			clauses = append(clauses, clause)
			if _, err := solver.AddClause(clause); err != nil {
				t.Fatal(err)
			}
		}
		expected := oracleSAT(n, clauses, nil)
		got, err := solver.Solve()
		if err != nil {
			t.Fatalf("round %d: %v", round, err)
		}
		if expected {
			satCount++
			if got != SolveSat {
				t.Fatalf("round %d expected SAT, got %s", round, got)
			}
			model, valid := solver.Model()
			if !valid || !allClausesSatisfied(clauses, model) {
				t.Fatalf("round %d invalid SAT model %v", round, model)
			}
		} else {
			unsatCount++
			if got != SolveUnsat {
				t.Fatalf("round %d expected UNSAT, got %s", round, got)
			}
			if oracleSAT(n, clauses, solver.FailedCore()) {
				t.Fatalf("round %d returned unsound core", round)
			}
		}
		assumptions := make([]Lit, 0, 4)
		for count := int(rng.bounded(5)); count > 0; count-- {
			lit, _ := MakeLit(Var(rng.bounded(uint64(n))), rng.bounded(2) == 1)
			assumptions = append(assumptions, lit)
			if rng.bounded(3) == 0 { // Exercise repeated assumptions.
				assumptions = append(assumptions, lit)
			}
			if rng.bounded(7) == 0 { // Exercise contradictory assumption pairs.
				opposite, _ := NegateLit(lit)
				assumptions = append(assumptions, opposite)
			}
		}
		assumptionExpected := oracleSAT(n, clauses, assumptions)
		assumptionResult, err := solver.SolveWithAssumptions(assumptions)
		if err != nil {
			t.Fatalf("round %d assumption solve: %v", round, err)
		}
		if assumptionExpected && assumptionResult != SolveSat || !assumptionExpected && assumptionResult != SolveUnsat {
			t.Fatalf("round %d assumption oracle mismatch: result=%s expectedSat=%t assumptions=%v", round, assumptionResult, assumptionExpected, assumptions)
		}
		if assumptionResult == SolveSat {
			model, valid := solver.Model()
			if !valid || !allClausesSatisfied(clauses, model) || !assumptionsSatisfied(assumptions, model) {
				t.Fatalf("round %d invalid assumption model", round)
			}
		} else {
			core := solver.FailedCore()
			if hasDuplicateLits(core) || !isSubset(core, assumptions) || oracleSAT(n, clauses, core) {
				t.Fatalf("round %d unsound assumption core %v", round, core)
			}
		}
		newClause := make([]Lit, 0, 2)
		for count := 1 + int(rng.bounded(2)); count > 0; count-- {
			lit, _ := MakeLit(Var(rng.bounded(uint64(n))), rng.bounded(2) == 1)
			newClause = append(newClause, lit)
		}
		if _, err := solver.AddClause(newClause); err != nil {
			t.Fatalf("round %d add clause: %v", round, err)
		}
		clauses = append(clauses, newClause)
		if model, valid := solver.Model(); valid || model != nil || len(solver.FailedCore()) != 0 {
			t.Fatalf("round %d AddClause did not clear previous result", round)
		}
		postAddExpected := oracleSAT(n, clauses, nil)
		postAddResult, err := solver.Solve()
		if err != nil || (postAddExpected && postAddResult != SolveSat) || (!postAddExpected && postAddResult != SolveUnsat) {
			fresh, _ := NewSolver()
			for i := 0; i < n; i++ {
				_, _ = fresh.NewVar()
			}
			for _, cl := range clauses {
				_, _ = fresh.AddClause(cl)
			}
			freshResult, _ := fresh.Solve()
			var learned [][]Lit
			for _, c := range solver.cs {
				if c.learnt {
					learned = append(learned, c.lits)
				}
			}
			t.Fatalf("round %d post-add mismatch: %s (%v), expectedSat=%t fresh=%s model=%v learned=%v clauses=%v", round, postAddResult, err, postAddExpected, freshResult, solver.model, learned, clauses)
		}
		solver.Reset()
		resetResult, err := solver.Solve()
		if err != nil || resetResult != postAddResult {
			t.Fatalf("round %d reset changed formula result: %s vs %s (%v)", round, resetResult, postAddResult, err)
		}
	}
	if satCount == 0 || unsatCount == 0 {
		t.Fatalf("differential rounds lacked SAT/UNSAT: %d/%d", satCount, unsatCount)
	}
}

func containsLit(lits []Lit, want Lit) bool {
	for _, lit := range lits {
		if lit == want {
			return true
		}
	}
	return false
}

type splitMix64 struct{ state uint64 }

func newSplitMix64(seed uint64) *splitMix64 { return &splitMix64{state: seed} }
func (r *splitMix64) next() uint64 {
	r.state += 0x9e3779b97f4a7c15
	z := r.state
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}
func (r *splitMix64) bounded(n uint64) uint64 {
	if n == 0 {
		panic("zero bound")
	}
	threshold := -n % n
	for {
		x := r.next()
		if x >= threshold {
			return x % n
		}
	}
}

func TestBudgetsPerCallAndInvalidInputIsAtomic(t *testing.T) {
	solver, _ := NewSolver()
	a, _ := MakeLit(0, false)
	if _, err := solver.NewVar(); err != nil {
		t.Fatal(err)
	}
	if _, err := solver.AddClause([]Lit{a}); err != nil {
		t.Fatal(err)
	}
	bad := Lit(1 << 31)
	if _, err := solver.AddClause([]Lit{a, bad}); err == nil {
		t.Fatal("AddClause accepted malformed input")
	}
	if solver.NClauses() != 0 {
		t.Fatalf("invalid clause partially mutated solver, clauses=%d", solver.NClauses())
	}
	if _, err := solver.SolveWithAssumptions([]Lit{a, bad}); err == nil {
		t.Fatal("Solve accepted malformed assumptions")
	}
	if got, err := solver.Solve(); err != nil || got != SolveSat {
		t.Fatalf("formula after rejected input: %v (%v)", got, err)
	}
	zero := uint64(0)
	if got, err := solver.Solve(SolverOptions{MaxConflicts: &zero}); err != nil || got != SolveSat {
		t.Fatalf("conflict-only budget unexpectedly limited steps: %v (%v)", got, err)
	}
	if !solver.Okay() {
		t.Fatal("solver marked not okay after recoverable budget path")
	}
}

func TestBuilderFailureAtomicityAndConstLiterals(t *testing.T) {
	solver, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 4, MaxClauseLiterals: 8, MaxStoredLiterals: 8})
	builder, err := NewCnfBuilder(solver)
	if err != nil {
		t.Fatal(err)
	}
	a, err := builder.NewLit()
	if err != nil {
		t.Fatal(err)
	}
	b, err := builder.NewLit()
	if err != nil {
		t.Fatal(err)
	}
	beforeVars, beforeClauses := solver.NVars(), solver.NClauses()
	if _, err := builder.And(a, b); err == nil {
		t.Fatal("builder exceeded variable ceiling")
	}
	if solver.NVars() != beforeVars || solver.NClauses() != beforeClauses {
		t.Fatal("failed gate operation was not atomic")
	}
	if _, err := builder.And(a, Lit(1<<31)); err == nil {
		t.Fatal("builder accepted malformed literal")
	}
	if solver.NVars() != beforeVars || solver.NClauses() != beforeClauses {
		t.Fatal("invalid gate input mutated solver")
	}
}

func TestCnfFormulaWriteReadAndEmptyConstants(t *testing.T) {
	builder, _ := NewCnfBuilder(nil)
	trueLit, err := builder.TrueLit()
	if err != nil {
		t.Fatal(err)
	}
	falseLit, err := builder.FalseLit()
	if err != nil {
		t.Fatal(err)
	}
	if got, err := builder.GetSolver().SolveWithAssumptions([]Lit{trueLit, negateForTest(t, falseLit)}); err != nil || got != SolveSat {
		t.Fatalf("constant literals: %v (%v)", got, err)
	}
	formula := CnfFormula{NumVars: 2, Clauses: [][]Lit{{0, 2}, {}, {3}}}
	text, err := WriteDimacs(formula)
	if err != nil {
		t.Fatal(err)
	}
	parsed, err := ParseDimacs(text)
	if err != nil || !reflect.DeepEqual(parsed, formula) {
		t.Fatalf("write/read: %+v (%v), text=%q", parsed, err, text)
	}
}

func TestRepeatedFormulaAndSolveStatsRemainDefined(t *testing.T) {
	solver, _ := NewSolver()
	a, _ := MakeLit(0, false)
	_, _ = solver.NewVar()
	if _, err := solver.AddClause([]Lit{a}); err != nil {
		t.Fatal(err)
	}
	if solver.NVars() != 1 || solver.NClauses() != 0 {
		t.Fatalf("stats vars=%d clauses=%d", solver.NVars(), solver.NClauses())
	}
	if got, err := solver.Solve(); err != nil || got != SolveSat {
		t.Fatalf("solve=%v %v", got, err)
	}
	if solver.Conflicts > ^uint64(0)-solver.Decisions {
		t.Fatal(fmt.Sprintf("unexpectedly overflowing statistics: %+v", solver))
	}
}

func TestRefusalsPreserveSavedModelAndCore(t *testing.T) {
	s, _ := NewSolver()
	x, _ := s.NewVar()
	lit, _ := MakeLit(x, false)
	if _, err := s.AddClause([]Lit{lit}); err != nil {
		t.Fatal(err)
	}
	if got, err := s.Solve(); err != nil || got != SolveSat {
		t.Fatalf("SAT setup=%v %v", got, err)
	}
	beforeModel, beforeValid := s.Model()
	if _, err := s.AddClause([]Lit{Lit(1 << 31)}); err == nil {
		t.Fatal("malformed clause accepted")
	}
	afterModel, afterValid := s.Model()
	if !beforeValid || !afterValid || !reflect.DeepEqual(beforeModel, afterModel) {
		t.Fatalf("invalid-input refusal changed saved model: before=%v/%v after=%v/%v", beforeModel, beforeValid, afterModel, afterValid)
	}

	coreSolver, _ := NewSolver()
	aVar, _ := coreSolver.NewVar()
	bVar, _ := coreSolver.NewVar()
	a, _ := MakeLit(aVar, false)
	b, _ := MakeLit(bVar, false)
	if _, err := coreSolver.AddClause([]Lit{a ^ 1, b ^ 1}); err != nil {
		t.Fatal(err)
	}
	if got, err := coreSolver.SolveWithAssumptions([]Lit{a, b}); err != nil || got != SolveUnsat {
		t.Fatalf("UNSAT setup=%v %v", got, err)
	}
	beforeCore := coreSolver.FailedCore()
	if _, err := coreSolver.AddClause([]Lit{Lit(1 << 31)}); err == nil {
		t.Fatal("malformed clause accepted")
	}
	if afterCore := coreSolver.FailedCore(); !reflect.DeepEqual(beforeCore, afterCore) {
		t.Fatalf("invalid-input refusal changed saved core: before=%v after=%v", beforeCore, afterCore)
	}

	quotaModel, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 2, MaxClauseLiterals: 2, MaxStoredLiterals: 1})
	q0, _ := quotaModel.NewVar()
	q1, _ := quotaModel.NewVar()
	l0, _ := MakeLit(q0, false)
	l1, _ := MakeLit(q1, false)
	if _, err := quotaModel.AddClause([]Lit{l0}); err != nil {
		t.Fatal(err)
	}
	if got, err := quotaModel.Solve(); err != nil || got != SolveSat {
		t.Fatalf("quota model setup=%v %v", got, err)
	}
	quotaBefore, _ := quotaModel.Model()
	if _, err := quotaModel.AddClause([]Lit{l1}); err == nil {
		t.Fatal("stored-literal quota refusal did not occur")
	}
	quotaAfter, quotaValid := quotaModel.Model()
	if !quotaValid || !reflect.DeepEqual(quotaBefore, quotaAfter) {
		t.Fatalf("quota refusal changed saved model: %v -> %v valid=%t", quotaBefore, quotaAfter, quotaValid)
	}

	quotaCore, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 1, MaxClauseLiterals: 2, MaxStoredLiterals: 2})
	qaVar, _ := quotaCore.NewVar()
	qbVar, _ := quotaCore.NewVar()
	qa, _ := MakeLit(qaVar, false)
	qb, _ := MakeLit(qbVar, false)
	if _, err := quotaCore.AddClause([]Lit{qa ^ 1, qb ^ 1}); err != nil {
		t.Fatal(err)
	}
	if got, err := quotaCore.SolveWithAssumptions([]Lit{qa, qb}); err != nil || got != SolveUnsat {
		t.Fatalf("quota core setup=%v %v", got, err)
	}
	coreBeforeQuota := quotaCore.FailedCore()
	if _, err := quotaCore.AddClause([]Lit{qa, qb}); err == nil {
		t.Fatal("clause-count quota refusal did not occur")
	}
	if coreAfterQuota := quotaCore.FailedCore(); !reflect.DeepEqual(coreBeforeQuota, coreAfterQuota) {
		t.Fatalf("quota refusal changed saved core: %v -> %v", coreBeforeQuota, coreAfterQuota)
	}
}

func TestWatchBucketCapacityIsBoundedByLiteralOccurrences(t *testing.T) {
	s, _ := NewSolver(SolverConfig{MaxVars: 16, MaxClauses: 32, MaxClauseLiterals: 64, MaxStoredLiterals: 64})
	for i := 0; i < 16; i++ {
		if _, err := s.NewVar(); err != nil {
			t.Fatal(err)
		}
	}
	for i := 0; i < 12; i++ {
		a, _ := MakeLit(Var(i), false)
		b, _ := MakeLit(Var((i+1)%16), false)
		c, _ := MakeLit(Var((i+2)%16), false)
		if _, err := s.AddClause([]Lit{a, b, c}); err != nil {
			t.Fatal(err)
		}
	}
	for i := 0; i < 100; i++ {
		a, _ := MakeLit(Var(i%12), i%2 == 0)
		if got, err := s.SolveWithAssumptions([]Lit{a}); err != nil || (got != SolveSat && got != SolveUnknown) {
			t.Fatalf("assumption solve=%v %v", got, err)
		}
		var capacity int
		for _, bucket := range s.watches {
			capacity += cap(bucket)
		}
		if bound := 4 * s.storedLiteralCount(); uint64(capacity) > bound {
			t.Fatalf("watch capacity=%d exceeds literal-derived bound=%d", capacity, bound)
		}
	}
}

func TestBuilderPublicValidationAndAtomicPreflight(t *testing.T) {
	bad := Lit(1 << 31)
	valid := []Lit{mustLit(t, 0, false), mustLit(t, 1, false), mustLit(t, 2, false)}
	invalidOps := []struct {
		name string
		call func(*CnfBuilder) error
	}{
		{"And", func(b *CnfBuilder) error { _, e := b.And(bad, valid[0]); return e }},
		{"Or", func(b *CnfBuilder) error { _, e := b.Or(valid[0], bad); return e }},
		{"Xor", func(b *CnfBuilder) error { _, e := b.Xor(bad, valid[1]); return e }},
		{"Equiv", func(b *CnfBuilder) error { _, e := b.Equiv(valid[0], bad); return e }},
		{"Not", func(b *CnfBuilder) error { _, e := b.Not(bad); return e }},
		{"Ite", func(b *CnfBuilder) error { _, e := b.Ite(valid[0], bad, valid[2]); return e }},
		{"FullAdder", func(b *CnfBuilder) error { _, e := b.FullAdder(valid[0], bad, valid[2]); return e }},
		{"BvAdd", func(b *CnfBuilder) error { _, e := b.BvAdd([]Lit{valid[0], bad}, []Lit{valid[1], valid[2]}); return e }},
		{"BvEq", func(b *CnfBuilder) error { _, e := b.BvEq([]Lit{bad}, []Lit{valid[0]}); return e }},
	}
	for _, tc := range invalidOps {
		t.Run("invalid_"+tc.name, func(t *testing.T) {
			s, _ := NewSolver()
			for i := 0; i < 3; i++ {
				_, _ = s.NewVar()
			}
			b, _ := NewCnfBuilder(s)
			if err := tc.call(b); err == nil {
				t.Fatal("invalid literal accepted")
			}
			if s.NVars() != 3 || s.inputClauseCount != 0 {
				t.Fatal("invalid operation mutated solver")
			}
		})
	}
	preflightConfigs := []struct {
		name string
		cfg  SolverConfig
	}{
		{"variables", SolverConfig{MaxVars: 2, MaxClauses: 8, MaxClauseLiterals: 16, MaxStoredLiterals: 16}},
		{"clauses", SolverConfig{MaxVars: 8, MaxClauses: 2, MaxClauseLiterals: 16, MaxStoredLiterals: 16}},
		{"original_literals", SolverConfig{MaxVars: 8, MaxClauses: 8, MaxClauseLiterals: 6, MaxStoredLiterals: 16}},
		{"stored_literals", SolverConfig{MaxVars: 8, MaxClauses: 8, MaxClauseLiterals: 16, MaxStoredLiterals: 6}},
	}
	for _, tc := range preflightConfigs {
		t.Run("preflight_"+tc.name, func(t *testing.T) {
			s, err := NewSolver(tc.cfg)
			if err != nil {
				t.Fatal(err)
			}
			for i := 0; i < 2; i++ {
				_, _ = s.NewVar()
			}
			b, _ := NewCnfBuilder(s)
			beforeVars, beforeClauses, beforeLiterals := s.NVars(), s.inputClauseCount, s.inputLiteralCount
			if _, err := b.And(valid[0], valid[1]); err == nil {
				t.Fatal("preflight limit not enforced")
			}
			if s.NVars() != beforeVars || s.inputClauseCount != beforeClauses || s.inputLiteralCount != beforeLiterals {
				t.Fatal("failed preflight mutated solver")
			}
		})
	}
	for _, makeLiteral := range []func(*CnfBuilder) (Lit, error){(*CnfBuilder).TrueLit, (*CnfBuilder).FalseLit} {
		s, _ := NewSolver(SolverConfig{MaxVars: 1, MaxClauses: 1, MaxClauseLiterals: 1, MaxStoredLiterals: 1})
		_, _ = s.NewVar()
		b, _ := NewCnfBuilder(s)
		if _, err := makeLiteral(b); err == nil {
			t.Fatal("constant literal ignored variable ceiling")
		}
	}
}

func TestLowThresholdReductionOnOracleBoundedPigeonhole(t *testing.T) {
	const pigeons, holes = 5, 4
	s, _ := NewSolver()
	vars := make([][]Lit, pigeons)
	clauses := make([][]Lit, 0, pigeons+holes*pigeons*(pigeons-1)/2)
	for p := 0; p < pigeons; p++ {
		vars[p] = make([]Lit, holes)
		for h := 0; h < holes; h++ {
			v, err := s.NewVar()
			if err != nil {
				t.Fatal(err)
			}
			vars[p][h], _ = MakeLit(v, false)
		}
		clauses = append(clauses, append([]Lit(nil), vars[p]...))
	}
	for h := 0; h < holes; h++ {
		for p := 0; p < pigeons; p++ {
			for q := p + 1; q < pigeons; q++ {
				clause := []Lit{vars[p][h] ^ 1, vars[q][h] ^ 1}
				clauses = append(clauses, clause)
			}
		}
	}
	for _, clause := range clauses {
		if _, err := s.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	s.restartBase, s.learnedLimit = 1, 1
	if got, err := s.Solve(); err != nil || got != SolveUnsat {
		t.Fatalf("PHP(5,4) with maintenance thresholds: %v %v", got, err)
	}
	if oracleSAT(pigeons*holes, clauses, nil) {
		t.Fatal("independent assignment enumeration found PHP(5,4) model")
	}
	deleted := false
	for _, clause := range s.cs {
		if clause.learnt && clause.deleted {
			deleted = true
			break
		}
	}
	if !deleted {
		t.Fatal("low threshold did not exercise learned reduction")
	}
	var watchCapacity int
	for _, bucket := range s.watches {
		watchCapacity += cap(bucket)
	}
	if bound := 4 * s.storedLiteralCount(); uint64(watchCapacity) > bound {
		t.Fatalf("repeated watcher movement exceeded literal-derived bound: capacity=%d bound=%d", watchCapacity, bound)
	}
}

func TestEveryBuilderPreflightRejectsBeforeMutation(t *testing.T) {
	calls := []struct {
		name string
		call func(*CnfBuilder, []Lit) error
	}{
		{"And", func(b *CnfBuilder, x []Lit) error { _, e := b.And(x[0], x[1]); return e }},
		{"Or", func(b *CnfBuilder, x []Lit) error { _, e := b.Or(x[0], x[1]); return e }},
		{"Xor", func(b *CnfBuilder, x []Lit) error { _, e := b.Xor(x[0], x[1]); return e }},
		{"Equiv", func(b *CnfBuilder, x []Lit) error { _, e := b.Equiv(x[0], x[1]); return e }},
		{"Ite", func(b *CnfBuilder, x []Lit) error { _, e := b.Ite(x[0], x[1], x[2]); return e }},
		{"FullAdder", func(b *CnfBuilder, x []Lit) error { _, e := b.FullAdder(x[0], x[1], x[2]); return e }},
		{"BvAdd", func(b *CnfBuilder, x []Lit) error { _, e := b.BvAdd([]Lit{x[0]}, []Lit{x[1]}); return e }},
		{"BvEq", func(b *CnfBuilder, x []Lit) error { _, e := b.BvEq([]Lit{x[0]}, []Lit{x[1]}); return e }},
	}
	for _, tc := range calls {
		t.Run(tc.name, func(t *testing.T) {
			s, _ := NewSolver(SolverConfig{MaxVars: 8, MaxClauses: 2, MaxClauseLiterals: 64, MaxStoredLiterals: 64})
			inputs := []Lit{mustLit(t, 0, false), mustLit(t, 1, false), mustLit(t, 2, false)}
			for range inputs {
				if _, err := s.NewVar(); err != nil {
					t.Fatal(err)
				}
			}
			b, _ := NewCnfBuilder(s)
			if err := tc.call(b, inputs); err == nil {
				t.Fatal("gate exceeded clause ceiling")
			}
			if s.NVars() != 3 || s.inputClauseCount != 0 || s.inputLiteralCount != 0 {
				t.Fatal("gate preflight refusal changed formula")
			}
		})
	}
}

func TestAssumptionGrowthAndZeroBudgetRecovery(t *testing.T) {
	s, _ := NewSolver()
	a, _ := MakeLit(Var(3), false)
	zero := uint64(0)
	if got, err := s.SolveWithAssumptions([]Lit{a}, SolverOptions{MaxSteps: &zero}); err != nil || got != SolveUnknown {
		t.Fatalf("zero-budget fresh assumption=%v %v", got, err)
	}
	if s.NVars() != 4 {
		t.Fatalf("assumption did not auto-grow variables: %d", s.NVars())
	}
	if s.VarValue(3) != Undef {
		t.Fatal("zero-budget assumption leaked after UNKNOWN")
	}
	if got, err := s.SolveWithAssumptions([]Lit{a}); err != nil || got != SolveSat {
		t.Fatalf("unbounded assumption recovery=%v %v", got, err)
	}
	model, valid := s.Model()
	if !valid || len(model) != 4 || !model[3] {
		t.Fatalf("fresh-assumption model=%v valid=%t", model, valid)
	}
}

func TestConflictBudgetAssumptionRecoveryAndSoundCore(t *testing.T) {
	s, _ := NewSolver()
	a, _ := s.NewVar()
	p, _ := s.NewVar()
	la, _ := MakeLit(a, false)
	lp, _ := MakeLit(p, false)
	for _, clause := range [][]Lit{{la ^ 1, lp}, {la ^ 1, lp ^ 1}} {
		if _, err := s.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	zero := uint64(0)
	if got, err := s.SolveWithAssumptions([]Lit{la}, SolverOptions{MaxConflicts: &zero}); err != nil || got != SolveUnknown {
		t.Fatalf("zero-conflict assumption=%v %v", got, err)
	}
	if got, err := s.SolveWithAssumptions([]Lit{la}); err != nil || got != SolveUnsat {
		t.Fatalf("assumption retry=%v %v", got, err)
	}
	core := s.FailedCore()
	if len(core) == 0 || oracleSAT(2, [][]Lit{{la ^ 1, lp}, {la ^ 1, lp ^ 1}}, core) {
		t.Fatalf("returned core is not independently UNSAT: %v", core)
	}
	if got, err := s.Solve(); err != nil || got != SolveSat {
		t.Fatalf("unrestricted base recovery=%v %v", got, err)
	}
}

func TestLearnedClauseStorageBudgetReturnsUnknownAtomically(t *testing.T) {
	s, _ := NewSolver(SolverConfig{MaxVars: 2, MaxClauses: 2, MaxClauseLiterals: 4, MaxStoredLiterals: 4})
	for i := 0; i < 2; i++ {
		_, _ = s.NewVar()
	}
	x, _ := MakeLit(0, false)
	y, _ := MakeLit(1, false)
	clauses := [][]Lit{{x, y}, {x, y ^ 1}}
	for _, clause := range clauses {
		if _, err := s.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	s.phase[0] = -1
	if got, err := s.Solve(); err != nil || got != SolveUnknown {
		t.Fatalf("learned storage refusal=%v %v", got, err)
	}
	if s.inputClauseCount != 2 || s.inputLiteralCount != 4 || s.NClauses() != 2 {
		t.Fatal("learned storage refusal changed original formula")
	}
	if got, err := s.SolveWithAssumptions([]Lit{x}); err != nil || got != SolveSat {
		t.Fatalf("satisfying-assumption recovery=%v %v", got, err)
	}
	model, valid := s.Model()
	if !valid || !allClausesSatisfied(clauses, model) {
		t.Fatalf("recovered model invalid: %v/%t", model, valid)
	}
}

func TestAssumptionReplayConflictBudgetPreservesBaseFormula(t *testing.T) {
	s, _ := NewSolver()
	for i := 0; i < 4; i++ {
		_, _ = s.NewVar()
	}
	x, _ := MakeLit(0, false)
	y, _ := MakeLit(1, false)
	a, _ := MakeLit(2, false)
	b, _ := MakeLit(3, false)
	clauses := [][]Lit{{x, y}, {x, y ^ 1}, {x ^ 1, a ^ 1, b}, {x ^ 1, a ^ 1, b ^ 1}}
	for _, clause := range clauses {
		if _, err := s.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	s.phase[0] = -1
	one := uint64(1)
	if got, err := s.SolveWithAssumptions([]Lit{a}, SolverOptions{MaxConflicts: &one}); err != nil || got != SolveUnknown {
		t.Fatalf("replay conflict budget=%v %v", got, err)
	}
	if got, err := s.SolveWithAssumptions([]Lit{a}); err != nil || got != SolveUnsat {
		t.Fatalf("replay recovery=%v %v", got, err)
	}
	core := s.FailedCore()
	if len(core) == 0 || oracleSAT(4, clauses, core) {
		t.Fatalf("replayed assumption core is not independently UNSAT: %v", core)
	}
	if got, err := s.Solve(); err != nil || got != SolveSat {
		t.Fatalf("unrestricted follow-up=%v %v", got, err)
	}
	model, valid := s.Model()
	if !valid || !allClausesSatisfied(clauses, model) {
		t.Fatalf("unrestricted base model invalid: %v/%t", model, valid)
	}
}

func TestAssumptionReplayAlreadyTrueAndContradictory(t *testing.T) {
	for _, forceAssumption := range []bool{true, false} {
		t.Run(fmt.Sprintf("force_%t", forceAssumption), func(t *testing.T) {
			s, _ := NewSolver()
			for i := 0; i < 5; i++ {
				_, _ = s.NewVar()
			}
			x, _ := MakeLit(0, false)
			y, _ := MakeLit(1, false)
			p, _ := MakeLit(2, false)
			q, _ := MakeLit(3, false)
			a, _ := MakeLit(4, false)
			last := a
			if !forceAssumption {
				last ^= 1
			}
			clauses := [][]Lit{{x, y}, {x, y ^ 1}, {x ^ 1, p}, {x ^ 1, q}, {p ^ 1, q ^ 1, last}}
			for _, clause := range clauses {
				if _, err := s.AddClause(clause); err != nil {
					t.Fatal(err)
				}
			}
			s.phase[0] = -1
			got, err := s.SolveWithAssumptions([]Lit{a})
			if forceAssumption {
				if err != nil || got != SolveSat {
					t.Fatalf("already-true assumption replay=%v %v", got, err)
				}
				model, valid := s.Model()
				if !valid || !model[4] || !allClausesSatisfied(clauses, model) {
					t.Fatalf("replayed SAT model invalid: %v/%t", model, valid)
				}
			} else {
				if err != nil || got != SolveUnsat {
					t.Fatalf("contradictory assumption replay=%v %v", got, err)
				}
				core := s.FailedCore()
				if len(core) == 0 || oracleSAT(5, clauses, core) {
					t.Fatalf("replayed core not independently UNSAT: %v", core)
				}
				if got, err := s.Solve(); err != nil || got != SolveSat {
					t.Fatalf("base formula after core=%v %v", got, err)
				}
				model, valid := s.Model()
				if !valid || !allClausesSatisfied(clauses, model) {
					t.Fatalf("base model invalid: %v/%t", model, valid)
				}
			}
		})
	}
}

func TestPendingConflictSurvivesLearnedReduction(t *testing.T) {
	s, _ := NewSolver()
	const variables = 7
	for i := 0; i < variables; i++ {
		if _, err := s.NewVar(); err != nil {
			t.Fatal(err)
		}
	}
	clauses := [][]int{{-2, -7, 6}, {7, 3, 4}, {1, -6, -2}, {3, 5, -4}, {-3, 1, 4}, {5, 2, 1}, {-2, -3, -4}, {7, 2, 5}, {-5, 7, 1}, {-7, 5, 4}, {-4, -7, -1}, {4, -1, -2}, {2, 1, -5}}
	original := make([][]Lit, len(clauses))
	for i, clause := range clauses {
		original[i] = literalsFromDIMACS(t, clause)
		if _, err := s.AddClause(original[i]); err != nil {
			t.Fatal(err)
		}
	}
	// These test-only thresholds force a pending long conflict to overlap
	// learned database reduction; production defaults remain unchanged.
	s.restartBase, s.learnedLimit = 1, 0
	if !oracleSAT(variables, original, nil) {
		t.Fatal("independent enumeration says fixed formula is UNSAT")
	}
	got, err := s.Solve()
	if err != nil || got != SolveSat {
		t.Fatalf("pending-conflict reduction solve=%v %v", got, err)
	}
	model, valid := s.Model()
	if !valid || !allClausesSatisfied(original, model) {
		t.Fatalf("pending-conflict model violates original CNF: %v/%t", model, valid)
	}
	if s.Restarts == 0 {
		t.Fatal("low restart threshold did not run")
	}
	deleted := false
	for _, clause := range s.cs {
		if clause.learnt && clause.deleted {
			deleted = true
			break
		}
	}
	if !deleted {
		t.Fatal("low learned limit did not reduce learned clauses")
	}
}

func TestAddClauseFinishesPendingRootContradiction(t *testing.T) {
	s, _ := NewSolver()
	x, _ := s.NewVar()
	y, _ := s.NewVar()
	a, _ := MakeLit(x, false)
	b, _ := MakeLit(y, false)
	for _, clause := range [][]Lit{{a, b}, {a, b ^ 1}, {a ^ 1, b}, {a ^ 1, b ^ 1}} {
		if _, err := s.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	one := uint64(1)
	if got, err := s.Solve(SolverOptions{MaxSteps: &one}); err != nil || got != SolveUnknown {
		t.Fatalf("budgeted root proof=%v %v", got, err)
	}
	u, _ := MakeLit(Var(3), false)
	v, _ := MakeLit(Var(4), false)
	if added, err := s.AddClause([]Lit{u, v}); err != nil || added {
		t.Fatalf("AddClause did not report propagated root conflict: added=%t err=%v", added, err)
	}
	if s.Okay() {
		t.Fatal("AddClause failed to persist root contradiction")
	}
	if got, err := s.Solve(); err != nil || got != SolveUnsat {
		t.Fatalf("follow-up known root result=%v %v", got, err)
	}
}

func TestDIMACSNumericSpellingAndLiteralStringBoundaries(t *testing.T) {
	for _, input := range []string{"", "p cnf +1 0\n", "p cnf 1 18446744073709551616\n", "p cnf 1 1\n18446744073709551616 0\n", "p cnf 1 1\n- 0\n"} {
		if _, err := ParseDimacs(input); err == nil {
			t.Fatalf("accepted malformed DIMACS %q", input)
		}
	}
	if _, err := LitToString(Lit(1 << 31)); err == nil {
		t.Fatal("LitToString accepted malformed packed literal")
	}
	if got, err := LitToString(Lit((1 << 31) - 1)); err != nil || got != "-1073741824" {
		t.Fatalf("maximum representable literal=%q %v", got, err)
	}
}

func TestEmptyBvAddAndMultiBitBvEq(t *testing.T) {
	builder, _ := NewCnfBuilder(nil)
	empty, err := builder.BvAdd(nil, nil)
	if err != nil || len(empty) != 0 {
		t.Fatalf("empty BvAdd=%v %v", empty, err)
	}
	a0, _ := builder.NewLit()
	a1, _ := builder.NewLit()
	b0, _ := builder.NewLit()
	b1, _ := builder.NewLit()
	eq, err := builder.BvEq([]Lit{a0, a1}, []Lit{b0, b1})
	if err != nil {
		t.Fatal(err)
	}
	for _, values := range [][4]bool{{false, false, false, false}, {false, true, false, true}, {true, false, true, false}, {true, true, true, true}, {false, false, true, false}, {true, false, true, true}} {
		assumptions := []Lit{truthLit(t, a0, values[0]), truthLit(t, a1, values[1]), truthLit(t, b0, values[2]), truthLit(t, b1, values[3])}
		want := values[0] == values[2] && values[1] == values[3]
		assumptions = append(assumptions, truthLit(t, eq, want))
		if got, err := builder.GetSolver().SolveWithAssumptions(assumptions); err != nil || got != SolveSat {
			t.Fatalf("BvEq expected %t on %v: %v %v", want, values, got, err)
		}
		assumptions[4] = truthLit(t, eq, !want)
		if got, err := builder.GetSolver().SolveWithAssumptions(assumptions); err != nil || got != SolveUnsat {
			t.Fatalf("BvEq wrong result accepted on %v: %v %v", values, got, err)
		}
	}
	if _, err := builder.BvAdd([]Lit{a0}, nil); err == nil {
		t.Fatal("BvAdd accepted mismatched widths")
	}
}

func TestKnownRootConflictPrecedesZeroBudgetAfterPendingWork(t *testing.T) {
	s, _ := NewSolver()
	x, _ := s.NewVar()
	y, _ := s.NewVar()
	a, _ := MakeLit(x, false)
	b, _ := MakeLit(y, false)
	clauses := [][]Lit{{a, b}, {a, b ^ 1}, {a ^ 1, b}, {a ^ 1, b ^ 1}}
	for _, clause := range clauses {
		if _, err := s.AddClause(clause); err != nil {
			t.Fatal(err)
		}
	}
	one, zero := uint64(1), uint64(0)
	if got, err := s.Solve(SolverOptions{MaxSteps: &one}); err != nil || got != SolveUnknown {
		t.Fatalf("first bounded result=%v %v", got, err)
	}
	if got, err := s.Solve(SolverOptions{MaxSteps: &zero}); err != nil || got != SolveUnknown {
		t.Fatalf("zero-budget retry=%v %v", got, err)
	}
	if got, err := s.Solve(); err != nil || got != SolveUnsat {
		t.Fatalf("unbounded root-conflict retry=%v %v", got, err)
	}
	if oracleSAT(2, clauses, nil) {
		t.Fatal("independent oracle found a model for root-conflict formula")
	}
}

func TestEnqueueAlreadyAssignedIsIdempotentAndRejectsOpposite(t *testing.T) {
	s, _ := NewSolver()
	v, err := s.NewVar()
	if err != nil {
		t.Fatal(err)
	}
	positive, _ := MakeLit(v, false)
	if !s.enqueue(positive, -1) {
		t.Fatal("first root assignment failed")
	}
	trailLength := len(s.trail)
	if !s.enqueue(positive, -1) {
		t.Fatal("same root assignment was not idempotent")
	}
	if len(s.trail) != trailLength {
		t.Fatal("idempotent assignment duplicated trail entry")
	}
	if s.enqueue(positive^1, -1) {
		t.Fatal("conflicting assignment was accepted")
	}
	if got := s.VarValue(v); got != True {
		t.Fatalf("conflict changed existing assignment: %v", got)
	}
}
