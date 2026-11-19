package validator

// BUILD fronts must name whose BUILD they run
// ===========================================
//
// An in-place edit of a Rust package's own BUILD front is package-local: the
// package and its prerequisites rebuild, its dependents do not (see
// internal/pkgmeta, "BUILD fronts"). That is only sound if selection can see
// every package whose BUILD another package runs, so those packages stay
// ordinary. Selection finds them by scanning BUILD lines for a BUILD-front
// invocation that names the other package's directory
// (`(cd ../paint_vm_metal_native && bash BUILD)`).
//
// This tripwire keeps the scan honest. With -validate-build-files, i.e. in
// the detect job of every pull request, it fails when:
//
//  1. a BUILD line invokes a BUILD front but names no other package's
//     directory, so selection cannot see whose BUILD it runs; or
//  2. a tracked *.sh, *.bash, *.ps1, *.cmd or *.bat under code/ runs a BUILD
//     front (scripts are not scanned for selection).
//
// Like metadata_exec.go this is a best-effort tripwire, not a security
// boundary: a variable or glob in the directory name gets past the scan.

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/pkgmeta"
)

// ValidateBuildFrontInvocations returns an error naming each BUILD line or
// script line that runs a BUILD front without naming the package it runs.
func ValidateBuildFrontInvocations(repoRoot string, packages []discovery.Package) error {
	fronts := make([]pkgmeta.PackageFronts, 0, len(packages))
	for _, pkg := range packages {
		entry := pkgmeta.PackageFronts{Name: pkg.Name, Dir: filepath.Base(pkg.Path), Fronts: map[string]string{}}
		for _, name := range pkgmeta.BuildFrontNames {
			if data, err := os.ReadFile(filepath.Join(pkg.Path, name)); err == nil {
				entry.Fronts[name] = string(data)
			}
		}
		fronts = append(fronts, entry)
	}
	_, unresolved := pkgmeta.ScanBuildFrontReferences(fronts)
	var findings []string
	for _, item := range unresolved {
		findings = append(findings, fmt.Sprintf("%s/%s:%d: %s", item.Package, item.Front, item.Line, item.Text))
	}

	cmd := exec.Command("git", "-C", repoRoot, "ls-files", "-z", "--", "code")
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	output, err := cmd.Output()
	if err != nil {
		return fmt.Errorf("listing tracked files for BUILD-front validation: %w: %s", err, strings.TrimSpace(stderr.String()))
	}
	for _, record := range bytes.Split(output, []byte{0}) {
		name := string(record)
		if name == "" || !buildFrontScriptCandidate(name) {
			continue
		}
		data, readErr := os.ReadFile(filepath.Join(repoRoot, filepath.FromSlash(name)))
		if readErr != nil {
			continue
		}
		findings = append(findings, scriptFrontInvocations(name, string(data))...)
	}
	if len(findings) == 0 {
		return nil
	}
	sort.Strings(findings)
	return fmt.Errorf(
		"BUILD-front invocation validation failed: these lines run a BUILD front without naming the package directory, or run one from a script:\n  - %s\n"+
			"Edits to a Rust package's own BUILD front do not rebuild its dependents, so selection must see who runs it "+
			"(code/specs/12-build-system.md, \"BUILD fronts\"); run the other package's BUILD from a BUILD line that names its "+
			"directory, e.g. `(cd ../<pkg> && sh BUILD)`.",
		strings.Join(findings, "\n  - "),
	)
}

// buildFrontScriptCandidate reports whether a tracked file is a script kind
// the validation scans.
func buildFrontScriptCandidate(name string) bool {
	base := path.Base(name)
	for _, suffix := range []string{".sh", ".bash", ".ps1", ".cmd", ".bat"} {
		if strings.HasSuffix(base, suffix) {
			return true
		}
	}
	return false
}

// scriptFrontInvocations returns a finding for every non-comment line of a
// script that runs a BUILD front.
func scriptFrontInvocations(name, text string) []string {
	var found []string
	for index, line := range strings.Split(text, "\n") {
		if pkgmeta.IsCommentLine(line) || !pkgmeta.IsFrontInvocation(line) {
			continue
		}
		found = append(found, fmt.Sprintf("%s:%d: %s", name, index+1, strings.TrimSpace(line)))
	}
	return found
}
