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
// A file belongs to a package if its path starts with the package's
// directory path relative to the repo root.
func MapFilesToPackages(changedFiles []string, packages []discovery.Package, repoRoot string) map[string]bool {
	changed := make(map[string]bool)

	// Build relative path lookup
	type pkgInfo struct {
		name    string
		relPath string
	}
	var pkgPaths []pkgInfo

	for _, pkg := range packages {
		rel, err := filepath.Rel(repoRoot, pkg.Path)
		if err != nil {
			continue
		}
		pkgPaths = append(pkgPaths, pkgInfo{name: pkg.Name, relPath: rel})
	}

	for _, f := range changedFiles {
		for _, pkg := range pkgPaths {
			if strings.HasPrefix(f, pkg.relPath+"/") || f == pkg.relPath {
				changed[pkg.name] = true
				break
			}
		}
	}

	return changed
}
