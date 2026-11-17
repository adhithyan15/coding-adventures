package main

import (
	"os"
	"os/exec"
	"strings"
	"testing"
)

// KILN00 section 17 (amended by D24): Kiln may depend on in-repo Go modules,
// wired with `replace`, and NEVER on a third-party module.  Kiln is the thing
// that decides what a build may trust; a dependency pulled from outside the
// repository would be code it cannot vouch for.  This test is the CI check of
// that rule, and it runs wherever `go test ./...` does (the build tool's Go
// BUILD, and the Windows one).

const repoModulePrefix = "github.com/adhithyan15/coding-adventures/"

// TestNoThirdPartyDependencies has three parts: every `require` in go.mod names
// a repo module, every `replace` points at a relative in-repo path, and the
// import graph the compiler actually builds (go list -deps, every package of
// this module and its tests) holds only the standard library and repo packages.
func TestNoThirdPartyDependencies(t *testing.T) {
	mod, err := os.ReadFile("go.mod")
	if err != nil {
		t.Fatal(err)
	}
	inBlock := ""
	requires, replaces := map[string]bool{}, map[string]bool{}
	for n, raw := range strings.Split(string(mod), "\n") {
		line := strings.TrimSpace(raw)
		if i := strings.Index(line, "//"); i >= 0 {
			line = strings.TrimSpace(line[:i])
		}
		switch {
		case line == "" || line == ")":
			inBlock = ""
			continue
		case strings.HasSuffix(line, "("):
			inBlock = strings.TrimSpace(strings.TrimSuffix(line, "("))
			continue
		}
		directive, rest := inBlock, line
		if directive == "" {
			f := strings.Fields(line)
			directive, rest = f[0], strings.TrimSpace(strings.TrimPrefix(line, f[0]))
		}
		switch directive {
		case "module", "go", "toolchain":
		case "require":
			if !strings.HasPrefix(rest, repoModulePrefix) {
				t.Errorf("go.mod line %d: require of a third-party module: %q", n+1, rest)
			}
			requires[strings.Fields(rest)[0]] = true
		case "replace":
			from, to, ok := strings.Cut(rest, "=>")
			from, to = strings.TrimSpace(from), strings.TrimSpace(to)
			if !ok || !strings.HasPrefix(from, repoModulePrefix) {
				t.Errorf("go.mod line %d: replace of a non-repo module: %q", n+1, rest)
			}
			if !strings.HasPrefix(to, "../") || strings.Contains(to, " ") {
				t.Errorf("go.mod line %d: replace target is not a relative in-repo path: %q", n+1, to)
			}
			// kiln is code/programs/go/kiln, so this prefix reaches code/packages/go.
			const wantPrefix = "../../../packages/go/"
			if !strings.HasPrefix(to, wantPrefix) {
				t.Errorf("go.mod line %d: replace target must start with %s: %q", n+1, wantPrefix, to)
			} else if strings.Contains(to[len(wantPrefix):], "..") || strings.Contains(to[len(wantPrefix):], "/") {
				t.Errorf("go.mod line %d: replace target must be one package directory: %q", n+1, to)
			}
			replaces[from] = true
		default:
			t.Errorf("go.mod line %d: unexpected directive %q (exclude, retract, tool and the like are not allowed)", n+1, directive)
		}
	}

	// Every required repo module must be wired to a local directory: a require
	// with no replace would be fetched from the network.
	for m := range requires {
		if !replaces[m] {
			t.Errorf("go.mod requires %s but has no replace to a local path", m)
		}
	}

	out, err := exec.Command("go", "list", "-deps", "-test", "-f",
		"{{if not .Standard}}{{.ImportPath}}{{end}}", "./...").Output()
	if err != nil {
		t.Fatalf("go list -deps: %v", err)
	}
	seen := 0
	for _, p := range strings.Split(string(out), "\n") {
		// Test variants print as "pkg [pkg.test]"; the synthetic test mains as "pkg.test".
		p, _, _ = strings.Cut(strings.TrimSpace(p), " [")
		if p == "" {
			continue
		}
		seen++
		if !strings.HasPrefix(p, repoModulePrefix) {
			t.Errorf("third-party import path in the build: %s", p)
		}
	}
	if seen == 0 {
		t.Fatal("go list returned no non-standard packages; the check is not looking at anything")
	}
}
