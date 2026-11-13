package main

// The Kiln legacy results export (-emit-results) and the -exec-wrapper
// argument parser (KILN04 section 4.3, "Wiring").
//
// Kiln traces legacy BUILD steps by running the old build tool with a wrapper
// in front of every `sh -c <line>` (-exec-wrapper). Afterwards it must know,
// per package, how the run ended and how many steps ran, because a trace is
// only trusted when the number of traced steps matches the number of steps the
// executor actually ran. -emit-results writes that record.
//
// Status mapping. The "status" field is the executor's own BuildResult.Status,
// passed through unchanged, so Kiln sees exactly what the report prints:
//
//	built            every step ran and exited 0
//	failed           a step exited non-zero; later steps did not run
//	skipped          not in the affected set / cache says up to date
//	dep-skipped      a dependency failed, so this package did not run
//	unsupported      the BUILD declares the platform unsupported (not run)
//	dep-unsupported  a dependency is unsupported (not run)
//	would-build      -dry-run only; nothing ran
//
// Step counts. steps_total is the length of the command list the executor
// ran (with -clippy off, the BUILD's step count). steps_run counts steps that
// were started: for a failed package that is the failing step's index plus
// one (so run < total when it failed mid-package); for packages that never
// ran it is 0 and steps_total is the BUILD's own step count.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
)

const kilnResultsSchema = "kiln-legacy-results/1"

type kilnResultPackage struct {
	Package    string `json:"package"`
	Status     string `json:"status"`
	StepsRun   int    `json:"steps_run"`
	StepsTotal int    `json:"steps_total"`
}

type kilnLegacyResults struct {
	Schema   string              `json:"schema"`
	Packages []kilnResultPackage `json:"packages"`
}

// parseExecWrapper parses the -exec-wrapper value: a JSON array of non-empty
// strings without NUL. JSON is used because a shell-style split of a string
// is ambiguous.
func parseExecWrapper(value string) ([]string, error) {
	var argv []string
	dec := json.NewDecoder(strings.NewReader(value))
	if err := dec.Decode(&argv); err != nil {
		return nil, fmt.Errorf("-exec-wrapper must be a JSON array of strings: %w", err)
	}
	if dec.More() {
		return nil, fmt.Errorf("-exec-wrapper has trailing data after the JSON array")
	}
	if len(argv) == 0 {
		return nil, fmt.Errorf("-exec-wrapper must not be an empty array")
	}
	for i, a := range argv {
		if a == "" {
			return nil, fmt.Errorf("-exec-wrapper element %d is empty", i)
		}
		if strings.ContainsRune(a, 0) {
			return nil, fmt.Errorf("-exec-wrapper element %d contains NUL", i)
		}
	}
	return argv, nil
}

// resolveExecWrapper makes wrapper[0] an absolute path, so a step's working
// directory (the package, which a BUILD can write to) can never supply the
// wrapper. A path with a separator must already be absolute; a bare name is
// resolved with exec.LookPath, and the result must be absolute.
func resolveExecWrapper(argv []string) ([]string, error) {
	first := argv[0]
	if strings.ContainsAny(first, `/\`) && !filepath.IsAbs(first) {
		return nil, fmt.Errorf("-exec-wrapper program %q is a relative path; use an absolute path or a bare name found on PATH", first)
	}
	resolved, err := exec.LookPath(first)
	if err != nil {
		return nil, fmt.Errorf("-exec-wrapper program %q: %w", first, err)
	}
	if !filepath.IsAbs(resolved) {
		return nil, fmt.Errorf("-exec-wrapper program %q resolved to a relative path %q", first, resolved)
	}
	return append([]string{resolved}, argv[1:]...), nil
}

// buildKilnResults assembles the export, sorted by rel_path. Only packages
// present in results are listed.
func buildKilnResults(packages []discovery.Package, repoRoot string, results map[string]executor.BuildResult) (*kilnLegacyResults, error) {
	out := &kilnLegacyResults{Schema: kilnResultsSchema, Packages: []kilnResultPackage{}}
	for _, pkg := range packages {
		r, ok := results[pkg.Name]
		if !ok {
			continue
		}
		rel, err := filepath.Rel(repoRoot, pkg.Path)
		if err != nil {
			return nil, fmt.Errorf("package %s: path %s is not under %s: %w", pkg.Name, pkg.Path, repoRoot, err)
		}
		// A run that recorded a total reports it; every other outcome
		// (skipped, unsupported, the cycle-detected failure, ...) reports the
		// BUILD's own step count.
		total := r.StepsTotal
		if total == 0 {
			total = len(pkg.BuildCommands)
		}
		out.Packages = append(out.Packages, kilnResultPackage{
			Package:    filepath.ToSlash(rel),
			Status:     r.Status,
			StepsRun:   r.StepsRun,
			StepsTotal: total,
		})
	}
	sort.Slice(out.Packages, func(i, j int) bool { return out.Packages[i].Package < out.Packages[j].Package })
	return out, nil
}

// writeKilnResults writes the export atomically (temp file, fsync, rename);
// a symlink at path is refused.
func writeKilnResults(packages []discovery.Package, repoRoot string, results map[string]executor.BuildResult, path string) error {
	doc, err := buildKilnResults(packages, repoRoot, results)
	if err != nil {
		return err
	}
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", "  ")
	if err := enc.Encode(doc); err != nil {
		return err
	}
	return writeFileAtomically(path, buf.Bytes())
}

// checkResultsPath refuses a symlink or non-regular output path before any
// BUILD runs, so a refusal is not discovered after a long build.
func checkResultsPath(path string) error {
	info, err := os.Lstat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	if info.Mode()&os.ModeSymlink != 0 {
		return fmt.Errorf("refusing to write through symlink %s", path)
	}
	if !info.Mode().IsRegular() {
		return fmt.Errorf("%s exists and is not a regular file", path)
	}
	return nil
}
