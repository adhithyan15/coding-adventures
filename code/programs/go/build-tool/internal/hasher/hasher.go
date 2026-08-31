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
//  3. Frame each repository-relative UTF-8 path and exact raw content with
//     unsigned 64-bit big-endian byte lengths.
//  4. SHA256-hash that single unambiguous stream.
//
// This framed hashing means:
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
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"unicode/utf8"

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
	"ocaml":      {".ml": true, ".mli": true, ".opam": true},
	"java":       {".java": true},
	"kotlin":     {".kt": true, ".kts": true},
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
	"ocaml":      {"dune": true, "dune-project": true, ".ocamlformat": true},
	"java":       {"settings.gradle.kts": true, "build.gradle.kts": true},
	"kotlin":     {"settings.gradle.kts": true, "build.gradle.kts": true},
	// global.json pins the .NET SDK version — a change here should trigger
	// a rebuild even if no source files changed. NuGet.Config controls the
	// package feed sources (case-insensitive filename on Windows, so both
	// variants are tracked).
	"dotnet": {"global.json": true, "NuGet.Config": true, "nuget.config": true},
}

var buildFilenames = map[string]bool{
	"BUILD":               true,
	"BUILD_mac":           true,
	"BUILD_linux":         true,
	"BUILD_windows":       true,
	"BUILD_mac_and_linux": true,
}

// generatedDirectoryComponents is the shared, case-sensitive v1 registry.
// Matching is by an exact normalized path component, so authored directories
// such as _Build and _build-example remain source candidates.
var generatedDirectoryComponents = map[string]bool{
	".build":        true,
	".cargo":        true,
	".claude":       true,
	".dart_tool":    true,
	".git":          true,
	".gradle":       true,
	".hg":           true,
	".mypy_cache":   true,
	".pytest_cache": true,
	".ruff_cache":   true,
	".stack-work":   true,
	".svn":          true,
	".tox":          true,
	".venv":         true,
	"Pods":          true,
	"__pycache__":   true,
	"_build":        true,
	"build":         true,
	"cover":         true,
	"deps":          true,
	"dist":          true,
	"dist-newstyle": true,
	"gradle-build":  true,
	"node_modules":  true,
	"target":        true,
	"vendor":        true,
}

var declaredManifestExtensions = map[string]map[string]bool{
	"ocaml": {".opam": true},
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
	files, _ := collectSourceFilesChecked(pkg)
	return files
}

func collectSourceFilesChecked(pkg discovery.Package) ([]string, error) {
	extensions := sourceExtensions[pkg.Language]
	specials := specialFilenames[pkg.Language]
	files := make([]string, 0)

	err := walkSourceFiles(pkg.Path, func(path string, entry os.DirEntry) error {
		name := entry.Name()
		if buildFilenames[name] || extensions[filepath.Ext(name)] || specials[name] {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	if err := sortPortablePaths(files, pkg.Path); err != nil {
		return nil, err
	}
	return files, nil
}

// resolveDeclaredSrcs converts the declared source patterns from a Starlark
// BUILD file into actual file paths. Each pattern is resolved relative to
// the package directory. Glob patterns (like "src/**/*.py") are expanded.
// The BUILD file itself is always included.
//
// Files are sorted by relative path for deterministic hashing.
func resolveDeclaredSrcs(pkg discovery.Package) []string {
	files, _ := resolveDeclaredSrcsChecked(pkg)
	return files
}

func resolveDeclaredSrcsChecked(pkg discovery.Package) ([]string, error) {
	files := make([]string, 0)
	specials := specialFilenames[pkg.Language]
	manifestExtensions := declaredManifestExtensions[pkg.Language]

	err := walkSourceFiles(pkg.Path, func(path string, entry os.DirEntry) error {
		name := entry.Name()
		rel, err := portableRelativePath(pkg.Path, path)
		if err != nil {
			return err
		}
		include := buildFilenames[name] || specials[name]
		if !include && filepath.Dir(path) == filepath.Clean(pkg.Path) && manifestExtensions[filepath.Ext(name)] {
			include = true
		}
		if !include {
			for _, pattern := range pkg.DeclaredSrcs {
				if globmatch.MatchPath(pattern, rel) {
					include = true
					break
				}
			}
		}
		if include {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	if err := sortPortablePaths(files, pkg.Path); err != nil {
		return nil, err
	}
	return files, nil
}

// walkSourceFiles enumerates only regular lexical descendants of root. It
// prunes the exact shared generated-directory registry before matching either
// extension or declared-source rules and never traverses link/reparse entries.
func walkSourceFiles(root string, visit func(string, os.DirEntry) error) error {
	cleanRoot := filepath.Clean(root)
	rootInfo, err := os.Lstat(cleanRoot)
	if err != nil || !rootInfo.IsDir() || rootInfo.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(rootInfo) {
		return fmt.Errorf("package root is not a stable directory")
	}
	return filepath.WalkDir(cleanRoot, func(path string, entry os.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if path == cleanRoot {
			return nil
		}
		info, err := entry.Info()
		if err != nil {
			return err
		}
		linked := info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info)
		if entry.IsDir() {
			if linked || generatedDirectoryComponents[entry.Name()] {
				return filepath.SkipDir
			}
			return nil
		}
		if linked || !info.Mode().IsRegular() {
			return nil
		}
		return visit(path, entry)
	})
}

// hasWindowsReparsePoint avoids a platform-specific source file while still
// rejecting NTFS junctions and other reparse entries. On Windows the Sys value
// exposes a FileAttributes field; FILE_ATTRIBUTE_REPARSE_POINT is 0x400.
func hasWindowsReparsePoint(info os.FileInfo) bool {
	system := reflect.ValueOf(info.Sys())
	if !system.IsValid() {
		return false
	}
	if system.Kind() == reflect.Pointer {
		if system.IsNil() {
			return false
		}
		system = system.Elem()
	}
	if system.Kind() != reflect.Struct {
		return false
	}
	attributes := system.FieldByName("FileAttributes")
	return attributes.IsValid() && attributes.CanUint() && attributes.Uint()&0x400 != 0
}

func portableRelativePath(root, path string) (string, error) {
	relative, err := filepath.Rel(filepath.Clean(root), filepath.Clean(path))
	if err != nil {
		return "", fmt.Errorf("source path is not relative to package root")
	}
	portable := filepath.ToSlash(relative)
	if err := validateRepositoryPath(portable); err != nil {
		return "", fmt.Errorf("source path is not portable UTF-8")
	}
	return portable, nil
}

func sortPortablePaths(files []string, root string) error {
	relatives := make(map[string]string, len(files))
	for _, file := range files {
		relative, err := portableRelativePath(root, file)
		if err != nil {
			return err
		}
		relatives[file] = relative
	}
	sort.Slice(files, func(i, j int) bool {
		return relatives[files[i]] < relatives[files[j]]
	})
	return nil
}

func repositoryRelativePackagePath(pkg discovery.Package) (string, error) {
	normalized := filepath.ToSlash(filepath.Clean(pkg.Path))
	parts := strings.Split(normalized, "/")
	canonicalStart := -1
	for index := 0; index+1 < len(parts); index++ {
		if parts[index] == "code" && (parts[index+1] == "packages" || parts[index+1] == "programs") {
			canonicalStart = index
		}
	}
	if canonicalStart >= 0 {
		candidate := strings.Join(parts[canonicalStart:], "/")
		if err := validateRepositoryPath(candidate); err != nil {
			return "", err
		}
		return candidate, nil
	}

	identity := strings.Split(pkg.Name, "/")
	var candidate string
	if len(identity) == 3 && identity[1] == "programs" {
		candidate = "code/programs/" + identity[0] + "/" + identity[2]
	} else if len(identity) == 2 {
		candidate = "code/packages/" + identity[0] + "/" + identity[1]
	} else {
		return "", fmt.Errorf("cannot derive repository-relative package path")
	}
	if err := validateRepositoryPath(candidate); err != nil {
		return "", err
	}
	return candidate, nil
}

func validateRepositoryPath(path string) error {
	if path == "" || !utf8.ValidString(path) || strings.ContainsRune(path, '\x00') || strings.Contains(path, "\\") || strings.HasPrefix(path, "/") {
		return fmt.Errorf("path is not portable UTF-8")
	}
	for _, component := range strings.Split(path, "/") {
		if component == "" || component == "." || component == ".." {
			return fmt.Errorf("path is not portable UTF-8")
		}
	}
	return nil
}

func ensureUnlinkedComponents(root, path string) error {
	relative, err := filepath.Rel(filepath.Clean(root), filepath.Clean(path))
	if err != nil || relative == ".." || strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		return fmt.Errorf("source path escapes package root")
	}
	current := filepath.Clean(root)
	components := []string{"."}
	if relative != "." {
		components = append(components, strings.Split(relative, string(filepath.Separator))...)
	}
	for _, component := range components {
		if component != "." {
			current = filepath.Join(current, component)
		}
		info, err := os.Lstat(current)
		if err != nil {
			return fmt.Errorf("source path is unavailable")
		}
		if info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info) {
			return fmt.Errorf("source link component is not hashable")
		}
	}
	return nil
}

func writeFileFrame(hash io.Writer, packageRoot, root, path string) error {
	relative, err := portableRelativePath(root, path)
	if err != nil {
		return err
	}
	repositoryPath := packageRoot + "/" + relative
	if err := validateRepositoryPath(repositoryPath); err != nil {
		return err
	}
	pathBytes := []byte(repositoryPath)
	if err := binary.Write(hash, binary.BigEndian, uint64(len(pathBytes))); err != nil {
		return err
	}
	if _, err := hash.Write(pathBytes); err != nil {
		return err
	}

	if err := ensureUnlinkedComponents(root, path); err != nil {
		return err
	}
	before, err := os.Lstat(path)
	if err != nil || !before.Mode().IsRegular() || before.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(before) {
		return fmt.Errorf("source is not a stable regular file")
	}
	source, err := os.Open(path)
	if err != nil {
		return fmt.Errorf("source is not readable")
	}
	defer source.Close()
	opened, err := source.Stat()
	if err != nil || !opened.Mode().IsRegular() || !os.SameFile(before, opened) {
		return fmt.Errorf("source changed before hashing")
	}
	contentLength := opened.Size()
	if contentLength < 0 {
		return fmt.Errorf("source length is invalid")
	}
	if err := binary.Write(hash, binary.BigEndian, uint64(contentLength)); err != nil {
		return err
	}
	written, err := io.CopyBuffer(hash, source, make([]byte, 8192))
	if err != nil {
		return fmt.Errorf("source read failed")
	}
	after, err := source.Stat()
	if err != nil || written != contentLength || after.Size() != opened.Size() || !after.ModTime().Equal(opened.ModTime()) {
		return fmt.Errorf("source changed while hashing")
	}
	if err := ensureUnlinkedComponents(root, path); err != nil {
		return err
	}
	return nil
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
func HashPackage(pkg discovery.Package) (string, error) {
	var (
		files []string
		err   error
	)
	if len(pkg.DeclaredSrcs) > 0 {
		// Strict mode: hash only declared sources.
		files, err = resolveDeclaredSrcsChecked(pkg)
	} else {
		// Legacy mode: extension-based collection.
		files, err = collectSourceFilesChecked(pkg)
	}
	if err != nil {
		return "", fmt.Errorf("source collection failed")
	}
	packageRoot, err := repositoryRelativePackagePath(pkg)
	if err != nil {
		return "", fmt.Errorf("repository identity is invalid")
	}
	hash := sha256.New()
	for _, file := range files {
		if err := writeFileFrame(hash, packageRoot, pkg.Path, file); err != nil {
			return "", fmt.Errorf("source input is not stable")
		}
	}
	return hex.EncodeToString(hash.Sum(nil)), nil
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
