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

// ChangeStatuses maps a raw changed Git path to the first letter of its
// `git diff --name-status` status: A (added), M (modified), D (deleted),
// T (type changed, e.g. a file became a symlink), R/C (renamed/copied, never
// produced with --no-renames, but parsed defensively), U, X or B.
//
// Only an addition or an in-place modification can leave a package-local
// metadata change local (see ClassifyChangedFiles). A nil map means "status
// unknown", which makes every metadata path propagate: callers that cannot
// say how a file changed get the conservative behaviour.
type ChangeStatuses map[string]byte

// Additive reports whether path was added or modified in place. Anything
// else, including a path with no recorded status, is not additive.
func (statuses ChangeStatuses) Additive(path string) bool {
	status, ok := statuses[path]
	return ok && (status == 'A' || status == 'M')
}

// GetChangedFiles runs `git diff --no-renames --name-status -z <base>...HEAD` and returns
// the list of changed file paths relative to the repo root.
//
// Uses three-dot diff which shows changes since the merge base — exactly
// what we want for PR builds. Disabling rename detection includes both the
// deleted source and added destination, so consumers of either path rebuild.
// Falls back to two-dot diff if three-dot fails.
func GetChangedFiles(repoRoot, diffBase string) []string {
	files, _ := GetChangedFilesWithStatus(repoRoot, diffBase)
	return files
}

// GetChangedFilesWithStatus is GetChangedFiles plus each path's status.
//
// With --no-renames a rename is reported as the deletion of its old path (D)
// and the addition of its new path (A), so a rename out of, into or within a
// metadata location always carries a D and therefore propagates. If Git
// gives no answer, both results are nil (the caller falls back to hashing).
func GetChangedFilesWithStatus(repoRoot, diffBase string) ([]string, ChangeStatuses) {
	// Try three-dot diff first (merge base)
	cmd := exec.Command("git", "diff", "--no-renames", "--name-status", "-z", diffBase+"...HEAD")
	cmd.Dir = repoRoot
	out, err := cmd.Output()
	if err != nil {
		// Fallback: two-dot diff
		cmd = exec.Command("git", "diff", "--no-renames", "--name-status", "-z", diffBase, "HEAD")
		cmd.Dir = repoRoot
		out, err = cmd.Output()
		if err != nil {
			return nil, nil
		}
	}
	return parseNameStatus(out)
}

// parseNameStatus decodes `--name-status -z` output: a status token, then one
// path (two for R and C, which are both recorded as non-additive).
//
//	"M\x00a/x.go\x00D\x00b/CHANGELOG.md\x00" → [a/x.go b/CHANGELOG.md], {M, D}
//
// Two inputs give nil, nil, the same answer as "Git gave no answer", so every
// caller takes its existing fallback (hashing in run(), a full plan in
// -emit-plan) exactly as it did before statuses existed:
//
//	empty output   An empty diff. Callers test for nil, not for length, so a
//	               non-nil empty slice here would make -emit-plan's per-platform
//	               overrides say "build nothing" while the top-level set says
//	               "build everything".
//	undecodable    A truncated record or an empty path. Returning the paths
//	               read so far would silently drop the rest of the diff.
func parseNameStatus(out []byte) ([]string, ChangeStatuses) {
	var files []string
	statuses := ChangeStatuses{}
	fields := bytes.Split(out, []byte{0})
	for index := 0; index < len(fields); {
		token := fields[index]
		index++
		if len(token) == 0 {
			continue
		}
		status := token[0]
		paths := 1
		if status == 'R' || status == 'C' {
			paths = 2
			status = 'R'
		}
		if index+paths > len(fields) {
			return nil, nil
		}
		for _, path := range fields[index : index+paths] {
			if len(path) == 0 {
				return nil, nil
			}
			files = append(files, string(path))
			// A path listed twice keeps its least additive status.
			if prior, seen := statuses[string(path)]; !seen || prior == 'A' || prior == 'M' {
				statuses[string(path)] = status
			}
		}
		index += paths
	}
	if files == nil {
		return nil, nil
	}
	return files, statuses
}

// MapFilesToPackages maps changed file paths to package names: every package
// whose own BUILD must re-run. It is the union of the two sets
// ClassifyChangedFiles returns; callers that propagate changes to dependents
// must use ClassifyChangedFiles instead.
func MapFilesToPackages(changedFiles []string, packages []discovery.Package, repoRoot string) map[string]bool {
	propagating, local := ClassifyChangedFiles(changedFiles, packages, repoRoot, nil)
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
//	Package type      | File changed (status)          | Package | Dependents
//	------------------|--------------------------------|---------|-----------
//	Shell BUILD       | src/main.py                    | YES     | YES
//	Shell BUILD       | README.md                      | YES     | YES (read by packaging)
//	Shell BUILD       | CHANGELOG.md (A or M)          | YES     | NO  (local)
//	Shell BUILD       | CHANGELOG.d/x.md (A or M)      | YES     | NO  (local)
//	Shell BUILD       | required_capabilities.json (M) | YES     | NO  (local)
//	Shell BUILD       | CHANGELOG.md (D, T, R)         | YES     | YES (may be required)
//	Shell BUILD       | evidence/run.sh                | YES     | YES (not a data type)
//	Shell BUILD       | sub/required_capabilities.json | YES     | YES (nested)
//	Starlark, srcs    | CHANGELOG.md (not in srcs)     | NO      | NO
//	Starlark, srcs    | CHANGELOG.md (in srcs)         | YES     | YES
//	Starlark, srcs    | src/main.py                    | YES     | YES
//	Starlark, srcs    | BUILD                          | YES     | YES
//
// The "local" rows are package-local metadata (internal/pkgmeta): files no
// dependent's build or tests can read, added or modified in place according
// to statuses. A nil statuses map classifies nothing as local. Such a
// package is returned in local, and only there; a package that also has an
// ordinary change is propagating.
//
// For Starlark BUILD packages with declared srcs, we apply strict filtering:
// only trigger a rebuild if the changed file matches one of the declared
// source patterns (or is a BUILD file itself). This means editing README.md
// or CHANGELOG.md in a Starlark package does NOT trigger a rebuild. A
// declared source is an input the package author named, so it always
// propagates, even when its name looks like metadata.
func ClassifyChangedFiles(changedFiles []string, packages []discovery.Package, repoRoot string, statuses ChangeStatuses) (propagating, local map[string]bool) {
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

	for _, raw := range changedFiles {
		// Normalize to forward slashes for consistent matching. On Linux and
		// macOS a backslash is an ordinary filename byte and survives, which
		// keeps such a path out of the metadata list (conservative).
		f := filepath.ToSlash(raw)

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
				// Only an addition or in-place edit can be local. A deleted,
				// renamed-away or retyped metadata file can break a
				// dependent that needs it to exist (packaging file lists).
				if pkgmeta.IsPackageLocal(relToPackage) && statuses.Additive(raw) {
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
