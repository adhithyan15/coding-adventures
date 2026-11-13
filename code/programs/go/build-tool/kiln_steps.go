package main

// The Kiln expected-steps export (-emit-kiln-steps)
// =================================================
//
// Kiln's recorder runs every BUILD step behind a wrapper and then asks "did
// the trace show the lines this package is SUPPOSED to run?" (KILN04 section
// 4.3, the recipe check). The lines a package runs are not the BUILD file's
// text:
//
//   - a Starlark BUILD has no text lines at all; its commands come out of
//     evaluating it (main.go does that once, during discovery),
//   - the Swift native-build-system pin rewrites some lines,
//   - on Windows the inline `VAR=value cmd` prefix is rewritten.
//
// Kiln cannot (and, being language-blind, must not) re-derive any of that. The
// executor already knows: executor.EffectiveStepLines(pkg) returns exactly the
// lines the wrapper will receive as `sh -c <line>`. This file writes them out.
//
// # Why it hangs off -emit-kiln-graph
//
// The lines must come from the SAME discovery pass as the graph, so the two
// files cannot describe different trees, and they must exist BEFORE any BUILD
// runs, because a BUILD that rewrites a later package's BUILD file must not
// be able to rewrite the expected lines too. So -emit-kiln-steps is only
// accepted together with -emit-kiln-graph, and is written in that one
// invocation, after the graph is built and before the process exits.
//
// # Shape
//
//	{
//	  "schema": "kiln-legacy-steps/1",
//	  "os": "linux",                       the OS the lines are for (runtime.GOOS)
//	  "packages": [                        sorted by package; only packages
//	                                       that have a BUILD file for that OS
//	    {"package": "code/packages/go/x",  repo-relative, slash-separated
//	     "steps": ["go build ./...", ...]} never null; [] when there are none
//	  ]
//	}
//
// Like the graph the output is byte-stable: the same tree always yields the
// same bytes.

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
)

// requireKilnConsumers makes the graph export refuse a repository whose
// selector relations name packages that were not discovered. It is true in
// every real run; only tests set it false, so the whole CLI path of the two
// exports can be exercised on a small fixture tree. It is a package variable,
// not a flag: there is no way to turn it off from the command line.
var requireKilnConsumers = true

// samePath reports whether a and b name the same file once made absolute and
// cleaned.
func samePath(a, b string) bool {
	aa, errA := filepath.Abs(a)
	bb, errB := filepath.Abs(b)
	if errA != nil || errB != nil {
		return filepath.Clean(a) == filepath.Clean(b)
	}
	return aa == bb
}

// kilnStepsSchema is the schema tag stamped on every export.
const kilnStepsSchema = "kiln-legacy-steps/1"

type kilnLegacySteps struct {
	Schema   string                 `json:"schema"`
	OS       string                 `json:"os"`
	Packages []kilnLegacyStepsEntry `json:"packages"`
}

type kilnLegacyStepsEntry struct {
	Package string   `json:"package"`
	Steps   []string `json:"steps"`
}

// buildKilnLegacySteps computes the export for packages (the full discovery
// list, Starlark already evaluated). goos is the OS the lines are for; the
// real CLI passes runtime.GOOS because that is what executor.EffectiveStepLines
// answers for.
func buildKilnLegacySteps(packages []discovery.Package, repoRoot, goos string) (*kilnLegacySteps, error) {
	out := &kilnLegacySteps{Schema: kilnStepsSchema, OS: goos, Packages: make([]kilnLegacyStepsEntry, 0, len(packages))}
	for _, pkg := range packages {
		// A package that has no BUILD file for this OS runs nothing here:
		// discovery across all platforms still lists it (its lines come
		// from another OS's file), and exporting those lines as this OS's
		// recipe would be wrong.
		if discovery.GetBuildFileForPlatform(pkg.Path, goos) == "" {
			continue
		}
		rel, err := filepath.Rel(repoRoot, pkg.Path)
		if err != nil {
			return nil, fmt.Errorf("package %s: path %s is not under %s: %w", pkg.Name, pkg.Path, repoRoot, err)
		}
		rel = filepath.ToSlash(rel)
		if rel == ".." || strings.HasPrefix(rel, "../") {
			return nil, fmt.Errorf("package %s: path %s is outside %s", pkg.Name, pkg.Path, repoRoot)
		}
		// EffectiveStepLines returns a non-nil slice (make), so the JSON
		// is [] rather than null for a package with no commands.
		lines := executor.EffectiveStepLines(pkg)
		for i, l := range lines {
			// encoding/json turns an invalid byte into U+FFFD, so two
			// different recipes would export identically; Kiln's decoder
			// refuses both U+FFFD and empty lines, so refuse them here
			// with a message that names the package.
			if l == "" || !utf8.ValidString(l) || strings.ContainsRune(l, utf8.RuneError) {
				return nil, fmt.Errorf("package %s: step %d is empty, not valid UTF-8, or contains U+FFFD and cannot be exported", pkg.Name, i)
			}
		}
		out.Packages = append(out.Packages, kilnLegacyStepsEntry{Package: rel, Steps: lines})
	}
	sort.Slice(out.Packages, func(i, j int) bool { return out.Packages[i].Package < out.Packages[j].Package })
	return out, nil
}

// marshalKilnLegacySteps renders the export: two-space indented JSON with a
// trailing newline, like the graph.
func marshalKilnLegacySteps(steps *kilnLegacySteps) ([]byte, error) {
	data, err := json.MarshalIndent(steps, "", "  ")
	if err != nil {
		return nil, err
	}
	return append(data, '\n'), nil
}

// emitKilnLegacySteps writes the export for packages to outputPath and
// returns the process exit code. It reuses the graph's atomic writer (which
// refuses a symlink or non-regular target).
func emitKilnLegacySteps(packages []discovery.Package, repoRoot, outputPath string) int {
	steps, err := buildKilnLegacySteps(packages, repoRoot, runtime.GOOS)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error building Kiln legacy steps: %v\n", err)
		return 1
	}
	data, err := marshalKilnLegacySteps(steps)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error encoding Kiln legacy steps: %v\n", err)
		return 1
	}
	if err := writeFileAtomically(outputPath, data); err != nil {
		fmt.Fprintf(os.Stderr, "Error writing Kiln legacy steps %s: %v\n", outputPath, err)
		return 1
	}
	fmt.Printf("Kiln legacy steps written to %s (%d packages)\n", outputPath, len(steps.Packages))
	return 0
}
