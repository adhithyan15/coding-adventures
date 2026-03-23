// Package gitdiff provides git-based change detection for the build tool.
//
// Instead of maintaining a cache file, this module uses git diff to determine
// which files changed between the current branch and a base ref (typically
// origin/main). Changed files are mapped to packages, then the dependency
// graph's AffectedNodes() finds everything that needs rebuilding.
//
// This is the DEFAULT change detection mode. Git is the source of truth.
package gitdiff

import (
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

// GetChangedFiles runs `git diff --name-only <base>...HEAD` and returns
// the list of changed file paths relative to the repo root.
//
// Uses three-dot diff which shows changes since the merge base — exactly
// what we want for PR builds. Falls back to two-dot diff if three-dot fails.
func GetChangedFiles(repoRoot, diffBase string) []string {
	// Try three-dot diff first (merge base)
	cmd := exec.Command("git", "diff", "--name-only", diffBase+"...HEAD")
	cmd.Dir = repoRoot
	out, err := cmd.Output()
	if err != nil {
		// Fallback: two-dot diff
		cmd = exec.Command("git", "diff", "--name-only", diffBase, "HEAD")
		cmd.Dir = repoRoot
		out, err = cmd.Output()
		if err != nil {
			return nil
		}
	}

	var files []string
	for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
		line = strings.TrimSpace(line)
		if line != "" {
			files = append(files, line)
		}
	}
	return files
}

// MapFilesToPackages maps changed file paths to package names.
//
// For shell BUILD packages (or Starlark packages without declared srcs),
// a file belongs to a package if its path starts with the package's
// directory path — any file change triggers a rebuild (legacy behavior).
//
// For Starlark BUILD packages with declared srcs, we apply strict filtering:
// only trigger a rebuild if the changed file matches one of the declared
// source patterns (or is a BUILD file itself). This means editing README.md
// or CHANGELOG.md in a Starlark package does NOT trigger a rebuild.
func MapFilesToPackages(changedFiles []string, packages []discovery.Package, repoRoot string) map[string]bool {
	changed := make(map[string]bool)

	// Build relative path lookup with Starlark metadata.
	type pkgInfo struct {
		name         string
		relPath      string // repo-root-relative, platform separators
		isStarlark   bool
		declaredSrcs []string
	}
	var pkgPaths []pkgInfo

	for _, pkg := range packages {
		rel, err := filepath.Rel(repoRoot, pkg.Path)
		if err != nil {
			continue
		}
		pkgPaths = append(pkgPaths, pkgInfo{
			name:         pkg.Name,
			relPath:      filepath.ToSlash(rel),
			isStarlark:   pkg.IsStarlark,
			declaredSrcs: pkg.DeclaredSrcs,
		})
	}

	for _, f := range changedFiles {
		// Normalize to forward slashes for consistent matching.
		f = filepath.ToSlash(f)

		for _, pkg := range pkgPaths {
			if !strings.HasPrefix(f, pkg.relPath+"/") && f != pkg.relPath {
				continue
			}

			// File is under this package's directory.
			if !pkg.isStarlark || len(pkg.declaredSrcs) == 0 {
				// Shell BUILD or no declared srcs: any file triggers rebuild.
				changed[pkg.name] = true
				break
			}

			// Starlark package with declared srcs: strict filtering.
			// Get the file's path relative to the package directory.
			relToPackage := strings.TrimPrefix(f, pkg.relPath+"/")

			// BUILD file changes always trigger a rebuild — the build
			// definition itself changed.
			base := filepath.Base(relToPackage)
			if base == "BUILD" || strings.HasPrefix(base, "BUILD_") {
				changed[pkg.name] = true
				break
			}

			// Check if the file matches any declared source pattern.
			for _, pattern := range pkg.declaredSrcs {
				if globmatch.MatchPath(pattern, relToPackage) {
					changed[pkg.name] = true
					break
				}
			}
			break // file matched to this package, don't check others
		}
	}

	return changed
}
