package main

// Package-local BUILD fronts: the filesystem half
// ===============================================
//
// internal/gitdiff.ClassifyChangedFiles decides from a path, a Git status and
// two package facts (language, Starlark) whether an in-place edit of a root
// BUILD front is package-local. Three vetoes need the filesystem and are
// computed here, then handed to the classifier as gitdiff.StatusOrdinary:
//
//	V1  the package carries the platform-unsupported marker at HEAD in any of
//	    its fronts (its dependents are reported "dep-unsupported"; editing the
//	    BUILD may change that);
//	V2  the changed front cannot be read, or is Starlark content (the
//	    evaluator can fall back to a shell package when evaluation fails);
//	V3  another package's BUILD runs this package's BUILD front
//	    (pkgmeta.ScanBuildFrontReferences).
//
// Nothing is read unless the change list holds a candidate, so ordinary pull
// requests pay nothing. The selection rule itself is documented in
// internal/pkgmeta ("BUILD fronts") and code/specs/12-build-system.md.

import (
	"os"
	"path"
	"path/filepath"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/pkgmeta"
	starlarkeval "github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/starlark"
)

// ordinaryBuildFronts returns the changed BUILD-front paths that must stay
// ordinary although Git says M: vetoes V1-V3. A nil result means none.
func ordinaryBuildFronts(changed []string, statuses gitdiff.ChangeStatuses, packages []discovery.Package, repoRoot string) map[string]bool {
	packageByDir := make(map[string]discovery.Package, len(packages))
	for _, pkg := range packages {
		if rel, err := filepath.Rel(repoRoot, pkg.Path); err == nil {
			packageByDir[filepath.ToSlash(filepath.Clean(rel))] = pkg
		}
	}

	type candidate struct {
		path string
		pkg  discovery.Package
	}
	var candidates []candidate
	for _, changedPath := range changed {
		if !statuses.Modified(changedPath) {
			continue
		}
		slashed := filepath.ToSlash(changedPath)
		if !pkgmeta.IsBuildFront(path.Base(slashed)) {
			continue
		}
		pkg, ok := packageByDir[path.Dir(slashed)]
		if !ok || pkg.IsStarlark || !pkgmeta.BuildFrontLocalFor(pkg.Language) {
			continue
		}
		candidates = append(candidates, candidate{changedPath, pkg})
	}
	if len(candidates) == 0 {
		return nil
	}

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
	executed, _ := pkgmeta.ScanBuildFrontReferences(fronts)
	frontsByName := make(map[string]pkgmeta.PackageFronts, len(fronts))
	for _, entry := range fronts {
		frontsByName[entry.Name] = entry
	}

	ordinary := map[string]bool{}
	for _, c := range candidates {
		entry := frontsByName[c.pkg.Name]
		// V3: another package runs this package's BUILD.
		if executed[c.pkg.Name] {
			ordinary[c.path] = true
			continue
		}
		// V1: unsupported-protocol marker in any front at HEAD.
		marked := false
		for _, content := range entry.Fronts {
			if strings.Contains(content, pkgmeta.UnsupportedMarker) {
				marked = true
				break
			}
		}
		if marked {
			ordinary[c.path] = true
			continue
		}
		// V2: unreadable or Starlark content.
		content, ok := entry.Fronts[path.Base(filepath.ToSlash(c.path))]
		if !ok || starlarkeval.IsStarlarkBuild(content) {
			ordinary[c.path] = true
		}
	}
	return ordinary
}
