// Package hasher computes SHA256 hashes for package source files.
//
// # Why hashing?
//
// The core of incremental builds is change detection. If nothing changed
// in a package's source files, there is no reason to rebuild it. We detect
// changes by computing a SHA256 hash of all relevant source files and
// comparing it against the cached hash from the last build.
//
// # How hashing works
//
// The hashing algorithm is deterministic — given the same files with the
// same contents, it always produces the same hash. Here is the procedure:
//
//  1. Collect all source files in the package directory, filtered by the
//     language's relevant extensions. Always include BUILD files.
//  2. Sort the file list lexicographically by relative path. This ensures
//     that file ordering does not affect the hash.
//  3. SHA256-hash each file's contents individually.
//  4. Concatenate all individual hashes into one string.
//  5. SHA256-hash that concatenated string to produce the final hash.
//
// This two-level hashing means:
//   - Reordering files doesn't change the hash (we sort first).
//   - Adding or removing a file changes the hash.
//   - Modifying any file's contents changes the hash.
//
// # Dependency hashing
//
// A package should be rebuilt if any of its transitive dependencies changed.
// HashDeps takes a package's dependency information and produces a single
// hash representing the state of all its dependencies.
package hasher

import (
	"crypto/sha256"
	"encoding/hex"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

// sourceExtensions maps languages to the file extensions that matter for
// change detection. If any file with these extensions changes, the package
// needs rebuilding.
var sourceExtensions = map[string]map[string]bool{
	"python":     {".py": true, ".toml": true, ".cfg": true},
	"ruby":       {".rb": true, ".gemspec": true},
	"go":         {".go": true},
	"typescript": {".ts": true, ".tsx": true, ".json": true},
	"rust":       {".rs": true, ".toml": true},
	"elixir":     {".ex": true, ".exs": true},
	"dart":       {".dart": true, ".yaml": true},
	"starlark":   {".star": true},
	"perl":       {".pl": true, ".pm": true, ".t": true, ".xs": true},
	"haskell":    {".hs": true, ".cabal": true},
	// .cs and .fs are C# and F# source files. .csproj and .fsproj are the
	// project manifests — equivalent to Cargo.toml or go.mod. Changes to
	// any of these should invalidate the build cache and trigger a rebuild.
	"dotnet": {".cs": true, ".fs": true, ".csproj": true, ".fsproj": true},
}

// specialFilenames maps languages to filenames that should always be
// included regardless of their extension.
var specialFilenames = map[string]map[string]bool{
	"python":     {},
	"ruby":       {"Gemfile": true, "Rakefile": true},
	"go":         {"go.mod": true, "go.sum": true},
	"typescript": {"package.json": true, "tsconfig.json": true, "vitest.config.ts": true},
	"rust":       {"Cargo.toml": true, "Cargo.lock": true},
	"elixir":     {"mix.exs": true, "mix.lock": true},
	"dart":       {"pubspec.yaml": true, "pubspec.lock": true, "analysis_options.yaml": true},
	"starlark":   {},
	"perl":       {"Makefile.PL": true, "Build.PL": true, "cpanfile": true, "MANIFEST": true, "META.json": true, "META.yml": true},
	"haskell":    {},
	// global.json pins the .NET SDK version — a change here should trigger
	// a rebuild even if no source files changed. NuGet.Config controls the
	// package feed sources (case-insensitive filename on Windows, so both
	// variants are tracked).
	"dotnet": {"global.json": true, "NuGet.Config": true, "nuget.config": true},
}

// collectSourceFiles walks the package directory and returns all source
// files relevant to the package's language. Files are sorted by their
// relative path for deterministic hashing.
//
// The collection rules:
//   - BUILD, BUILD_mac, BUILD_linux, and BUILD_windows are always included.
//   - Files matching the language's extensions are included.
//   - Special filenames (go.mod, Gemfile, etc.) are included.
//   - Everything else is ignored.
func collectSourceFiles(pkg discovery.Package) []string {
	extensions := sourceExtensions[pkg.Language]
	specials := specialFilenames[pkg.Language]
	if extensions == nil {
		extensions = make(map[string]bool)
	}
	if specials == nil {
		specials = make(map[string]bool)
	}

	var files []string

	// Walk the package directory recursively.
	filepath.Walk(pkg.Path, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}

		name := info.Name()

		// Always include BUILD files — they define how the package is built.
		if name == "BUILD" || name == "BUILD_mac" || name == "BUILD_linux" || name == "BUILD_windows" {
			files = append(files, path)
			return nil
		}

		// Check if the file extension matches.
		ext := filepath.Ext(name)
		if extensions[ext] {
			files = append(files, path)
			return nil
		}

		// Check special filenames.
		if specials[name] {
			files = append(files, path)
			return nil
		}

		return nil
	})

	// Sort by relative path for determinism. Two developers with different
	// absolute paths to the repo should get the same hash.
	sort.Slice(files, func(i, j int) bool {
		relI, _ := filepath.Rel(pkg.Path, files[i])
		relJ, _ := filepath.Rel(pkg.Path, files[j])
		return relI < relJ
	})

	return files
}

// resolveDeclaredSrcs converts the declared source patterns from a Starlark
// BUILD file into actual file paths. Each pattern is resolved relative to
// the package directory. Glob patterns (like "src/**/*.py") are expanded.
// The BUILD file itself is always included.
//
// Files are sorted by relative path for deterministic hashing.
func resolveDeclaredSrcs(pkg discovery.Package) []string {
	var files []string

	// Always include the BUILD file itself.
	for _, name := range []string{"BUILD", "BUILD_mac", "BUILD_linux", "BUILD_windows"} {
		buildPath := filepath.Join(pkg.Path, name)
		if fileExists(buildPath) {
			files = append(files, buildPath)
		}
	}

	// Resolve each declared pattern by walking the package directory
	// and matching each file against the pattern using globmatch.MatchPath.
	//
	// We use WalkDir + globmatch instead of filepath.Glob because
	// filepath.Glob does NOT support ** (recursive globbing). The
	// pattern "src/**/*.py" would silently match nothing with Glob.
	if len(pkg.DeclaredSrcs) > 0 {
		filepath.WalkDir(pkg.Path, func(path string, d os.DirEntry, err error) error {
			if err != nil || d.IsDir() {
				return nil
			}

			// Get path relative to the package directory, using forward slashes.
			rel, err := filepath.Rel(pkg.Path, path)
			if err != nil {
				return nil
			}
			rel = filepath.ToSlash(rel)

			// Check against each declared source pattern.
			for _, pattern := range pkg.DeclaredSrcs {
				if globmatch.MatchPath(pattern, rel) {
					files = append(files, path)
					break
				}
			}
			return nil
		})
	}

	// Sort by relative path for determinism.
	sort.Slice(files, func(i, j int) bool {
		relI, _ := filepath.Rel(pkg.Path, files[i])
		relJ, _ := filepath.Rel(pkg.Path, files[j])
		return relI < relJ
	})

	// Deduplicate (BUILD file may also match a pattern).
	deduped := make([]string, 0, len(files))
	seen := make(map[string]bool)
	for _, f := range files {
		if !seen[f] {
			seen[f] = true
			deduped = append(deduped, f)
		}
	}

	return deduped
}

// fileExists reports whether a file exists and is not a directory.
func fileExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

// hashFile computes the SHA256 hex digest of a single file's contents.
// We read in 8KB chunks to handle large files without loading them
// entirely into memory.
func hashFile(path string) (string, error) {
	f, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer f.Close()

	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return "", err
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}

// HashPackage computes a SHA256 hash representing all source files in
// the package. The hash changes if any source file is added, removed,
// or modified.
//
// When a package has DeclaredSrcs (from a Starlark BUILD file), we hash
// ONLY those declared files — this is strict mode. When DeclaredSrcs is
// empty (shell BUILD files), we fall back to extension-based collection.
//
// If the package has no source files, we hash the empty string for
// consistency — every package gets a hash, even empty ones.
func HashPackage(pkg discovery.Package) string {
	var files []string
	if len(pkg.DeclaredSrcs) > 0 {
		// Strict mode: hash only declared sources.
		files = resolveDeclaredSrcs(pkg)
	} else {
		// Legacy mode: extension-based collection.
		files = collectSourceFiles(pkg)
	}

	if len(files) == 0 {
		// No source files — hash the empty string.
		h := sha256.Sum256([]byte(""))
		return hex.EncodeToString(h[:])
	}

	// Hash each file individually, concatenate all hashes, hash again.
	// This two-level scheme means the final hash changes if any file
	// changes, is added, or is removed.
	var fileHashes []string
	for _, f := range files {
		fh, err := hashFile(f)
		if err != nil {
			// If we can't read a file, use a sentinel to ensure the hash
			// differs from the cached version, triggering a rebuild.
			fh = "error-reading-file"
		}
		fileHashes = append(fileHashes, fh)
	}

	combined := strings.Join(fileHashes, "")
	h := sha256.Sum256([]byte(combined))
	return hex.EncodeToString(h[:])
}

// HashDeps computes a SHA256 hash of all transitive dependency hashes.
//
// If any transitive dependency's source files changed, this hash will
// change too, triggering a rebuild of the dependent package. This is
// how we propagate changes through the dependency tree.
//
// In our graph, edges go dep → pkg (dependency points to dependent).
// So a package's dependencies are found by following reverse edges
// (predecessors). We use TransitiveDependents which follows forward
// edges from a given node — but here we need the reverse: the packages
// that this package depends ON.
//
// Wait — actually, in our graph convention:
//   - Edge A → B means "A must be built before B" (B depends on A)
//   - So B's dependencies are its predecessors
//   - TransitiveDependents(B) gives everything that depends on B (forward)
//   - We need "transitive dependencies of B" = everything B depends on
//
// We collect predecessors transitively by walking reverse edges.
func HashDeps(
	packageName string,
	graph *directedgraph.Graph,
	packageHashes map[string]string,
) string {
	if !graph.HasNode(packageName) {
		h := sha256.Sum256([]byte(""))
		return hex.EncodeToString(h[:])
	}

	// Collect all transitive dependencies (packages this one depends on).
	// In our graph, edge A→B means B depends on A. So B's deps are its
	// predecessors. We walk backwards (reverse edges) from packageName.
	transitiveDeps := collectTransitivePredecessors(packageName, graph)

	if len(transitiveDeps) == 0 {
		h := sha256.Sum256([]byte(""))
		return hex.EncodeToString(h[:])
	}

	// Sort for determinism, concatenate hashes, hash again.
	sorted := make([]string, 0, len(transitiveDeps))
	for dep := range transitiveDeps {
		sorted = append(sorted, dep)
	}
	sort.Strings(sorted)

	var combined strings.Builder
	for _, dep := range sorted {
		combined.WriteString(packageHashes[dep])
	}

	h := sha256.Sum256([]byte(combined.String()))
	return hex.EncodeToString(h[:])
}

// collectTransitivePredecessors walks backwards through the graph from
// the given node, collecting all nodes it transitively depends on.
//
// In our graph, edge A→B means "B depends on A". So to find everything
// that packageName depends on, we follow predecessors (reverse edges).
func collectTransitivePredecessors(node string, graph *directedgraph.Graph) map[string]bool {
	visited := make(map[string]bool)

	// Get direct predecessors to start.
	preds, err := graph.Predecessors(node)
	if err != nil {
		return visited
	}

	// BFS through predecessors.
	queue := make([]string, len(preds))
	copy(queue, preds)
	for _, p := range preds {
		visited[p] = true
	}

	for len(queue) > 0 {
		current := queue[0]
		queue = queue[1:]

		morePreds, err := graph.Predecessors(current)
		if err != nil {
			continue
		}
		for _, pred := range morePreds {
			if !visited[pred] {
				visited[pred] = true
				queue = append(queue, pred)
			}
		}
	}

	return visited
}
