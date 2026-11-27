// The opt-in bridge keeps the Python package free of solver runtime imports.
// It reads strict DIMACS from stdin and delegates solving to the repository's
// Go CDCL solver (VER09), returning only SAT/UNSAT and a witness when SAT.
package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"

	satsolver "github.com/adhithyan15/coding-adventures/code/packages/go/sat-solver"
)

func main() {
	solver, err := satsolver.NewSolver()
	if err != nil { panic(err) }
	scanner := bufio.NewScanner(os.Stdin)
	var vars uint32
	var clause []satsolver.Lit
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || strings.HasPrefix(line, "c") { continue }
		if strings.HasPrefix(line, "p") {
			fields := strings.Fields(line); parsed, parseErr := strconv.ParseUint(fields[2], 10, 32); if parseErr != nil { panic(parseErr) }; vars = uint32(parsed)
			for i := uint32(0); i < vars; i++ { if _, err = solver.NewVar(); err != nil { panic(err) } }; continue
		}
		fields := strings.Fields(line)
		for _, field := range fields {
			n, parseErr := strconv.Atoi(field)
			if parseErr != nil { panic(parseErr) }
			if n == 0 { if _, err = solver.AddClause(clause); err != nil { panic(err) }; clause = nil; continue }
			variable := satsolver.Var(abs(n) - 1)
			literal, makeErr := satsolver.MakeLit(variable, n < 0); if makeErr != nil { panic(makeErr) }
			clause = append(clause, literal)
		}
	}
	if len(clause) > 0 { if _, err = solver.AddClause(clause); err != nil { panic(err) } }
	result, err := solver.Solve(); if err != nil { panic(err) }
	fmt.Println(strings.ToUpper(string(result)))
}

func abs(value int) int { if value < 0 { return -value }; return value }
