// The opt-in bridge keeps the Python package free of solver runtime imports.
// It reads strict DIMACS from stdin and delegates solving to the repository's
// Go CDCL solver (VER09), returning only its SAT/UNSAT result.
package main

import (
	"fmt"
	"io"
	"os"

	satsolver "github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver"
)

func main() {
	const maxInput = 64 * 1024
	data, err := io.ReadAll(io.LimitReader(os.Stdin, maxInput+1))
	if err != nil {
		fatal(err)
	}
	if len(data) > maxInput {
		fatal(fmt.Errorf("DIMACS input exceeds %d-byte limit", maxInput))
	}
	formula, err := satsolver.ParseDimacs(string(data))
	if err != nil {
		fatal(err)
	}
	solver, err := satsolver.NewSolver()
	if err != nil {
		fatal(err)
	}
	for i := uint32(0); i < formula.NumVars; i++ {
		if _, err = solver.NewVar(); err != nil {
			fatal(err)
		}
	}
	for _, clause := range formula.Clauses {
		if _, err = solver.AddClause(clause); err != nil {
			fatal(err)
		}
	}
	result, err := solver.Solve()
	if err != nil {
		fatal(err)
	}
	fmt.Println(result)
}

func fatal(err error) {
	fmt.Fprintln(os.Stderr, err)
	os.Exit(1)
}
