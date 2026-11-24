// Command covergate fails when any package in a Go cover profile is below a
// statement-coverage threshold.
//
//	go test -coverprofile=coverage.out ./prng ./pbt ./tfv
//	go run ./tools/covergate coverage.out 95
//
// Why not "go tool cover | awk"?  The BUILD file runs on Windows runners too,
// which have no awk, and a shell pipeline hides the exit status of its first
// stage.  This is the same check in the language the package is written in.
//
// A cover profile line is "path/file.go:startLine.col,endLine.col stmts count":
// a block of stmts statements that ran count times.  Coverage of a package is
// the statements in blocks with count > 0 over all statements in its blocks.
package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"path"
	"sort"
	"strconv"
	"strings"
)

type tally struct{ covered, total int }

// gate reads a cover profile and reports per-package coverage.  ok is false if
// any package is below min, or if the profile holds no packages at all (an
// empty profile must never pass).
func gate(r io.Reader, min float64) (report string, ok bool, err error) {
	pkgs := map[string]*tally{}
	sc := bufio.NewScanner(r)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	lineNo := 0
	for sc.Scan() {
		lineNo++
		line := strings.TrimSpace(sc.Text())
		if line == "" || strings.HasPrefix(line, "mode:") {
			continue
		}
		colon := strings.LastIndex(line, ":")
		fields := strings.Fields(line[colon+1:])
		if colon < 0 || len(fields) != 3 {
			return "", false, fmt.Errorf("line %d: not a cover profile block: %q", lineNo, line)
		}
		stmts, err1 := strconv.Atoi(fields[1])
		count, err2 := strconv.Atoi(fields[2])
		if err1 != nil || err2 != nil || stmts < 0 || count < 0 {
			return "", false, fmt.Errorf("line %d: bad numbers in %q", lineNo, line)
		}
		pkg := path.Dir(line[:colon])
		t := pkgs[pkg]
		if t == nil {
			t = &tally{}
			pkgs[pkg] = t
		}
		t.total += stmts
		if count > 0 {
			t.covered += stmts
		}
	}
	if err := sc.Err(); err != nil {
		return "", false, err
	}
	if len(pkgs) == 0 {
		return "", false, fmt.Errorf("the profile contains no packages")
	}
	names := make([]string, 0, len(pkgs))
	for n := range pkgs {
		names = append(names, n)
	}
	sort.Strings(names)
	ok = true
	var b strings.Builder
	for _, n := range names {
		t := pkgs[n]
		pct := 100.0
		if t.total > 0 {
			pct = 100 * float64(t.covered) / float64(t.total)
		}
		status := "ok"
		if pct < min {
			status, ok = "BELOW", false
		}
		fmt.Fprintf(&b, "%-5s %6.1f%%  %s\n", status, pct, n)
	}
	return b.String(), ok, nil
}

func run(args []string, stdout, stderr io.Writer) int {
	if len(args) != 2 {
		fmt.Fprintln(stderr, "usage: covergate <profile> <min-percent>")
		return 2
	}
	min, err := strconv.ParseFloat(args[1], 64)
	if err != nil || min < 0 || min > 100 {
		fmt.Fprintf(stderr, "covergate: bad threshold %q\n", args[1])
		return 2
	}
	f, err := os.Open(args[0])
	if err != nil {
		fmt.Fprintf(stderr, "covergate: %v\n", err)
		return 2
	}
	defer f.Close()
	report, ok, err := gate(f, min)
	if err != nil {
		fmt.Fprintf(stderr, "covergate: %v\n", err)
		return 2
	}
	fmt.Fprint(stdout, report)
	if !ok {
		fmt.Fprintf(stderr, "covergate: a package is below %.0f%% statement coverage\n", min)
		return 1
	}
	return 0
}

func main() { os.Exit(run(os.Args[1:], os.Stdout, os.Stderr)) }
