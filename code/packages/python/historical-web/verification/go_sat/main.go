// The opt-in bridge keeps the Python package free of solver runtime imports.
// It reads strict DIMACS from stdin and delegates solving to the repository's
// Go CDCL solver (VER09), returning only SAT/UNSAT and a witness when SAT.
package main

import (
	"fmt"
	"io"
	"os"

	satsolver "github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver"
)

func main() {
	data, err := io.ReadAll(os.Stdin)
	if err != nil {
		fatal(err)
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
