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
	"bytes"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/pkgmeta"
)

func isBuildFront(base string) bool {
	switch base {
	case "BUILD", "BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux":
		return true
	default:
		return false
	}
}

// GetChangedFiles runs `git diff --no-renames --name-only -z <base>...HEAD` and returns
// the list of changed file paths relative to the repo root.
//
// Uses three-dot diff which shows changes since the merge base — exactly
// what we want for PR builds. Disabling rename detection includes both the
// deleted source and added destination, so consumers of either path rebuild.
// Falls back to two-dot diff if three-dot fails.
func GetChangedFiles(repoRoot, diffBase string) []string {
	// Try three-dot diff first (merge base)
	cmd := exec.Command("git", "diff", "--no-renames", "--name-only", "-z", diffBase+"...HEAD")
	cmd.Dir = repoRoot
	out, err := cmd.Output()
	if err != nil {
		// Fallback: two-dot diff
		cmd = exec.Command("git", "diff", "--no-renames", "--name-only", "-z", diffBase, "HEAD")
		cmd.Dir = repoRoot
		out, err = cmd.Output()
		if err != nil {
			return nil
		}
	}

	var files []string
	for _, path := range bytes.Split(out, []byte{0}) {
		if len(path) != 0 {
			files = append(files, string(path))
		}
	}
	return files
}

// MapFilesToPackages maps changed file paths to package names: every package
// whose own BUILD must re-run. It is the union of the two sets
// ClassifyChangedFiles returns; callers that propagate changes to dependents
// must use ClassifyChangedFiles instead.
func MapFilesToPackages(changedFiles []string, packages []discovery.Package, repoRoot string) map[string]bool {
	propagating, local := ClassifyChangedFiles(changedFiles, packages, repoRoot)
	for name := range local {
		propagating[name] = true
	}
	return propagating
}

// ClassifyChangedFiles maps changed file paths to packages and splits them by
// whether the change can reach the package's dependents.
//
// For shell BUILD packages (or Starlark packages without declared srcs),
// a file belongs to a package if its path starts with the package's
// directory path, and any file change triggers a rebuild of that package.
// Whether the change also rebuilds the package's dependents depends on the
// file:
//
//	Package type      | File changed               | Package | Dependents
//	------------------|----------------------------|---------|-----------
//	Shell BUILD       | src/main.py                | YES     | YES
//	Shell BUILD       | README.md                  | YES     | NO  (local)
//	Shell BUILD       | CHANGELOG.d/x.md           | YES     | NO  (local)
//	Shell BUILD       | required_capabilities.json | YES     | NO  (local)
//	Shell BUILD       | docs/README.md             | YES     | YES (nested)
//	Starlark, srcs    | README.md (not in srcs)    | NO      | NO
//	Starlark, srcs    | README.md (in srcs)        | YES     | YES
//	Starlark, srcs    | src/main.py                | YES     | YES
//	Starlark, srcs    | BUILD                      | YES     | YES
//
// The "local" rows are package-local metadata (internal/pkgmeta): files no
// dependent's build or tests can read. Such a package is returned in local,
// and only there; a package that also has an ordinary change is propagating.
//
// For Starlark BUILD packages with declared srcs, we apply strict filtering:
// only trigger a rebuild if the changed file matches one of the declared
// source patterns (or is a BUILD file itself). This means editing README.md
// or CHANGELOG.md in a Starlark package does NOT trigger a rebuild. A
// declared source is an input the package author named, so it always
// propagates, even when its name looks like metadata.
func ClassifyChangedFiles(changedFiles []string, packages []discovery.Package, repoRoot string) (propagating, local map[string]bool) {
	propagating = make(map[string]bool)
	local = make(map[string]bool)

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
		// Normalize to forward slashes for consistent matching. On Linux and
		// macOS a backslash is an ordinary filename byte and survives, which
		// keeps such a path out of the metadata list (conservative).
		f = filepath.ToSlash(f)

		for _, pkg := range pkgPaths {
			if !strings.HasPrefix(f, pkg.relPath+"/") && f != pkg.relPath {
				continue
			}

			// File is under this package's directory.
			relToPackage := strings.TrimPrefix(f, pkg.relPath+"/")
			if f == pkg.relPath {
				relToPackage = ""
			}

			if !pkg.isStarlark || len(pkg.declaredSrcs) == 0 {
				// Shell BUILD or no declared srcs: any file triggers a
				// rebuild; only package-local metadata stops there.
				if pkgmeta.IsPackageLocal(relToPackage) {
					local[pkg.name] = true
				} else {
					propagating[pkg.name] = true
				}
				break
			}

			// Starlark package with declared srcs: strict filtering.
			// BUILD file changes always trigger a rebuild — the build
			// definition itself changed.
			base := filepath.Base(relToPackage)
			if isBuildFront(base) {
				propagating[pkg.name] = true
				break
			}

			// Check if the file matches any declared source pattern.
			for _, pattern := range pkg.declaredSrcs {
				if globmatch.MatchPath(pattern, relToPackage) {
					propagating[pkg.name] = true
					break
				}
			}
			break // file matched to this package, don't check others
		}
	}

	// A package with any propagating change is not local-only.
	for name := range propagating {
		delete(local, name)
	}
	return propagating, local
}
