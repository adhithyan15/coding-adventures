// Package discovery walks a monorepo directory tree to discover packages.
//
// # How package discovery works
//
// A monorepo can contain hundreds of packages across multiple languages. The
// build system discovers them by recursively walking the directory tree and
// looking for BUILD files. Any directory containing a BUILD file is a package.
//
// The walk is recursive. Starting from the root:
//
//  1. If the current directory's name is in the skip list, ignore it entirely.
//  2. If the current directory has a BUILD file, it is a package. Register it
//     and stop — we don't recurse into packages.
//  3. Otherwise, list all subdirectories and recurse into each one.
//
// This is the same approach used by Bazel, Buck, and Pants. No configuration
// files are needed to route the walk — the presence of a BUILD file is
// sufficient to identify a package.
//
// # Skip list
//
// Certain directories are known to never contain packages: .git, .venv,
// node_modules, __pycache__, etc. The skip list prevents the walker from
// descending into these directories, keeping discovery fast even in large
// repos with deep dependency trees.
//
// # Platform-specific BUILD files
//
// Platform-specific BUILD files override the generic BUILD file. The priority
// is: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows), then
// BUILD_mac_and_linux (shared macOS/Linux), then BUILD (all platforms). This
// allows platform-specific build commands (e.g., different venv paths on
// Windows, or skipping tools that don't support certain platforms).
//
// # Language inference
//
// We infer a package's language only from the exact bucket immediately below a
// "packages" or "programs" boundary. Programs retain a "programs" identity
// segment, for example "go/programs/build-tool".
package discovery

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
)

// Package represents a discovered package in the monorepo. Each package has
// a qualified name (like "python/logic-gates"), an absolute path on disk,
// a list of build commands from its BUILD file, and an inferred language.
//
// Packages can use either shell BUILD files (traditional) or Starlark BUILD
// files (declarative). The IsStarlark flag indicates which format was found.
// For Starlark BUILD files, the build tool evaluates them to extract targets
// with explicit srcs and deps, populating DeclaredSrcs and DeclaredDeps.
type Package struct {
	Name          string   // Qualified name, e.g. "python/logic-gates"
	Path          string   // Absolute path to the package directory
	BuildCommands []string // Lines from the BUILD file (commands to execute)
	Language      string   // Canonical package/program bucket, or "unknown"
	BuildContent  string   // Raw BUILD file content (used for Starlark detection)
	IsStarlark    bool     // Whether this BUILD file is Starlark (vs shell)
	DeclaredSrcs  []string // Explicit source files from Starlark srcs field
	DeclaredDeps  []string // Explicit deps from Starlark deps field
}

// DuplicatePackageIdentityError reports two or more package directories that
// normalize to one graph identity. Paths are stable repository-relative
// identities so diagnostics never disclose the checkout root.
type DuplicatePackageIdentityError struct {
	Code    string
	Package string
	Paths   []string
}

func (e *DuplicatePackageIdentityError) Error() string {
	return fmt.Sprintf("%s: package=%s paths=%s", e.Code, e.Package, strings.Join(e.Paths, ","))
}

// knownLanguages is the canonical discovery registry. The package-parity
// denominator is defined separately; dotnet remains a shared host bucket.
var knownLanguages = map[string]bool{
	"csharp": true, "dart": true, "elixir": true, "fsharp": true,
	"go": true, "haskell": true, "java": true, "kotlin": true,
	"lua": true, "perl": true, "python": true, "ruby": true,
	"rust": true, "swift": true, "typescript": true,
	"c": true, "cpp": true, "ocaml": true,
	"wasm": true, "mosaic": true, "twig": true, "starlark": true,
	"dotnet": true,
}

// skipDirs is the set of directory names that should never be traversed
// during package discovery. These are known to contain non-source files
// (caches, dependencies, build artifacts) that would waste time to scan
// and could never contain valid packages.
var skipDirs = map[string]bool{
	".git":          true,
	".hg":           true,
	".svn":          true,
	".venv":         true,
	".tox":          true,
	".mypy_cache":   true,
	".pytest_cache": true,
	".ruff_cache":   true,
	"__pycache__":   true,
	"node_modules":  true,
	"vendor":        true,
	"dist":          true,
	"build":         true,
	"target":        true,
	".claude":       true,
	"specs":         true, // Specification and golden fixture trees are never buildable packages
	"Pods":          true,
	".dart_tool":    true,
	".build":        true, // Swift Package Manager build artefacts and dependency checkouts
	".gradle":       true, // Gradle build cache and wrapper metadata
	"gradle-build":  true, // Gradle output dir (renamed from "build" to avoid BUILD file conflict)
}

// readLines reads a file and returns non-blank, non-comment lines.
//
// Blank lines and lines starting with '#' are stripped out. Leading and
// trailing whitespace is removed from each line. If the file does not
// exist, an empty slice is returned (not an error — a missing file
// simply means "nothing to see here").
func readLines(filepath string) []string {
	data, err := os.ReadFile(filepath)
	if err != nil {
		return nil
	}

	var lines []string
	for _, line := range strings.Split(string(data), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" && !strings.HasPrefix(trimmed, "#") {
			lines = append(lines, trimmed)
		}
	}
	return lines
}

// packageBoundary returns the final packages/programs boundary in path. The
// final boundary is authoritative so nested temporary fixture roots retain
// their own identity rather than borrowing a bucket from a host checkout.
func packageBoundary(path string) (kind, bucket string, ok bool) {
	parts := strings.Split(filepath.ToSlash(path), "/")
	for index := 0; index+1 < len(parts); index++ {
		if parts[index] == "packages" || parts[index] == "programs" {
			kind = parts[index]
			bucket = parts[index+1]
			ok = true
		}
	}
	return kind, bucket, ok
}

// inferLanguage accepts only canonical exact boundary buckets. A canonical
// word elsewhere in the path cannot change an unknown bucket.
func inferLanguage(path string) string {
	_, bucket, ok := packageBoundary(path)
	if ok && knownLanguages[bucket] {
		return bucket
	}
	return "unknown"
}

// inferPackageName builds a qualified package name from the language and
// directory path.
//
// For packages (under code/packages/):
//
//	"python/logic-gates"
//
// For programs (under code/programs/), a "programs/" infix is included so
// that programs and packages with the same directory basename (e.g.,
// code/packages/go/grammar-tools and code/programs/go/grammar-tools) receive
// distinct names and do not create a self-loop in the dependency graph:
//
//	"go/programs/grammar-tools"
func inferPackageName(path, language string) string {
	kind, _, _ := packageBoundary(path)
	if kind == "programs" {
		return language + "/programs/" + filepath.Base(path)
	}
	return language + "/" + filepath.Base(path)
}

func repositoryPackagePath(root, packagePath string) string {
	parts := strings.Split(filepath.ToSlash(packagePath), "/")
	canonicalStart := -1
	for index := 0; index+1 < len(parts); index++ {
		if parts[index] == "code" && (parts[index+1] == "packages" || parts[index+1] == "programs") {
			canonicalStart = index
		}
	}
	if canonicalStart >= 0 {
		return strings.Join(parts[canonicalStart:], "/")
	}

	relative, err := filepath.Rel(root, packagePath)
	if err == nil && relative != "." && relative != "" && relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		return filepath.ToSlash(relative)
	}
	return filepath.Base(packagePath)
}

// getBuildFile returns the path to the appropriate BUILD file for the current
// platform, or an empty string if none exists.
//
// Priority (most specific wins):
//  1. Platform-specific: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows)
//  2. Shared: BUILD_mac_and_linux (macOS or Linux — for Unix-like systems)
//  3. Generic: BUILD (all platforms)
//  4. "" if no BUILD file exists
//
// This layering lets packages provide Windows-specific build commands via
// BUILD_windows while sharing a single BUILD_mac_and_linux for the common
// Unix case, falling back to BUILD when no platform differences exist.
func getBuildFile(directory string) string {
	return GetBuildFileForPlatform(directory, runtime.GOOS)
}

// GetBuildFileForPlatform is like getBuildFile but accepts an explicit OS name.
// This is useful for testing platform-specific behavior without running on
// that platform. The goos parameter should be "darwin", "linux", or "windows".
//
// Priority (most specific wins):
//  1. Platform-specific: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows)
//  2. Shared: BUILD_mac_and_linux (macOS or Linux)
//  3. Generic: BUILD (all platforms)
//  4. "" if no BUILD file exists
func GetBuildFileForPlatform(directory, goos string) string {
	// Step 1: Check for the most specific platform file.
	if goos == "darwin" {
		platformBuild := filepath.Join(directory, "BUILD_mac")
		if fileExists(platformBuild) {
			return platformBuild
		}
	}

	if goos == "linux" {
		platformBuild := filepath.Join(directory, "BUILD_linux")
		if fileExists(platformBuild) {
			return platformBuild
		}
	}

	if goos == "windows" {
		platformBuild := filepath.Join(directory, "BUILD_windows")
		if fileExists(platformBuild) {
			return platformBuild
		}
	}

	// Step 2: Check for the shared Unix file (macOS + Linux).
	if goos == "darwin" || goos == "linux" {
		sharedBuild := filepath.Join(directory, "BUILD_mac_and_linux")
		if fileExists(sharedBuild) {
			return sharedBuild
		}
	}

	// Step 3: Fall back to the generic BUILD file.
	genericBuild := filepath.Join(directory, "BUILD")
	if fileExists(genericBuild) {
		return genericBuild
	}

	return ""
}

// fileExists reports whether the named file exists and is not a directory.
func fileExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

// walkDirs recursively descends into subdirectories, collecting packages that
// have BUILD files. This is the heart of the discovery algorithm.
//
// The walk uses the skip list to avoid descending into directories that are
// known to contain non-source files (caches, dependencies, build artifacts).
//
// The recursion stops at BUILD files: once we find a package, we don't
// look inside it for sub-packages. This keeps the model simple — a
// package is a leaf in the directory tree.
func walkDirs(directory string, packages *[]Package) {
	// Check if this directory's name is in the skip list.
	dirName := filepath.Base(directory)
	if skipDirs[dirName] {
		return
	}

	buildFile := getBuildFile(directory)

	if buildFile != "" {
		// This directory is a package. Read the BUILD commands and register it.
		commands := readLines(buildFile)
		language := inferLanguage(directory)
		name := inferPackageName(directory, language)

		// Read raw BUILD content for Starlark detection.
		rawContent := ""
		if data, err := os.ReadFile(buildFile); err == nil {
			rawContent = string(data)
		}

		*packages = append(*packages, Package{
			Name:          name,
			Path:          directory,
			BuildCommands: commands,
			Language:      language,
			BuildContent:  rawContent,
		})
		return
	}

	// Not a package — list all subdirectories and recurse into each one.
	entries, err := os.ReadDir(directory)
	if err != nil {
		return
	}

	for _, entry := range entries {
		if entry.IsDir() {
			subdirPath := filepath.Join(directory, entry.Name())
			walkDirs(subdirPath, packages)
		}
	}
}

// DiscoverPackages recursively walks the directory tree starting from root,
// collecting packages with BUILD files. The returned list is sorted
// by package name for deterministic output.
//
// This is the main entry point for the discovery module. The root
// parameter should typically be the "code/" directory inside the repo.
func DiscoverPackages(root string) ([]Package, error) {
	var packages []Package
	walkDirs(root, &packages)
	sort.Slice(packages, func(i, j int) bool {
		if packages[i].Name == packages[j].Name {
			return packages[i].Path < packages[j].Path
		}
		return packages[i].Name < packages[j].Name
	})

	for index := 0; index < len(packages); {
		end := index + 1
		for end < len(packages) && packages[end].Name == packages[index].Name {
			end++
		}
		if end-index > 1 {
			paths := make([]string, 0, end-index)
			for _, pkg := range packages[index:end] {
				paths = append(paths, repositoryPackagePath(root, pkg.Path))
			}
			sort.Strings(paths)
			return nil, &DuplicatePackageIdentityError{
				Code:    "DUPLICATE_PACKAGE_IDENTITY",
				Package: packages[index].Name,
				Paths:   paths,
			}
		}
		index = end
	}

	return packages, nil
}

// ReadLines is exported for use by the resolver (to read go.mod, etc.).
// It is identical to the internal readLines function.
func ReadLines(filepath string) []string {
	return readLines(filepath)
}
