// Package discovery walks a monorepo directory tree following DIRS/BUILD files
// to discover packages.
//
// # How package discovery works
//
// A monorepo can contain hundreds of packages across multiple languages. Rather
// than scanning every directory (which would be slow and fragile), we use an
// explicit routing mechanism: DIRS files. Each DIRS file is a simple text file
// listing subdirectories to descend into. This is Knuth's idea of "literate
// directory structure" — the directory layout itself is a readable document.
//
// The walk is recursive. Starting from the root:
//
//  1. If the current directory has a BUILD file, it is a package. Register it
//     and stop — we don't recurse into packages.
//  2. If the current directory has a DIRS file, read it. Each non-blank,
//     non-comment line names a subdirectory to descend into.
//  3. Recurse into each listed subdirectory.
//
// # Platform-specific BUILD files
//
// On macOS, if BUILD_mac exists in a directory, we use it instead of BUILD.
// On Linux, BUILD_linux takes precedence. This allows platform-specific build
// commands (e.g., different compiler flags or test runners).
//
// # Language inference
//
// We infer a package's language from its directory path. If the path contains
// "python", "ruby", or "go" as a component under "packages" or "programs",
// that is the language. The package name is "{language}/{dirname}", e.g.,
// "python/logic-gates" or "go/directed-graph".
package discovery

import (
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
)

// Package represents a discovered package in the monorepo. Each package has
// a qualified name (like "python/logic-gates"), an absolute path on disk,
// a list of build commands from its BUILD file, and an inferred language.
type Package struct {
	Name          string   // Qualified name, e.g. "python/logic-gates"
	Path          string   // Absolute path to the package directory
	BuildCommands []string // Lines from the BUILD file (commands to execute)
	Language      string   // Inferred language: "python", "ruby", "go", or "unknown"
}

// readLines reads a file and returns non-blank, non-comment lines.
//
// Blank lines and lines starting with '#' are stripped out. Leading and
// trailing whitespace is removed from each line. If the file does not
// exist, an empty slice is returned (not an error — a missing DIRS file
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

// inferLanguage inspects the directory path to determine the programming
// language. We look for known language names ("python", "ruby", "go") as
// path components. For example, "/repo/code/packages/python/logic-gates"
// yields "python".
func inferLanguage(path string) string {
	// Split the path into its components and search for a known language.
	parts := strings.Split(filepath.ToSlash(path), "/")
	for _, lang := range []string{"python", "ruby", "go", "typescript"} {
		for _, part := range parts {
			if part == lang {
				return lang
			}
		}
	}
	return "unknown"
}

// inferPackageName builds a qualified package name like "python/logic-gates"
// from the language and the directory's basename.
func inferPackageName(path, language string) string {
	return language + "/" + filepath.Base(path)
}

// getBuildFile returns the path to the appropriate BUILD file for the current
// platform, or an empty string if none exists.
//
// Priority:
//  1. BUILD_mac on macOS (Darwin)
//  2. BUILD_linux on Linux
//  3. BUILD (cross-platform fallback)
//  4. "" if no BUILD file exists
func getBuildFile(directory string) string {
	system := runtime.GOOS

	if system == "darwin" {
		platformBuild := filepath.Join(directory, "BUILD_mac")
		if fileExists(platformBuild) {
			return platformBuild
		}
	}

	if system == "linux" {
		platformBuild := filepath.Join(directory, "BUILD_linux")
		if fileExists(platformBuild) {
			return platformBuild
		}
	}

	genericBuild := filepath.Join(directory, "BUILD")
	if fileExists(genericBuild) {
		return genericBuild
	}

	return ""
}

// GetBuildFileForPlatform is like getBuildFile but accepts an explicit OS name.
// This is useful for testing platform-specific behavior without running on
// that platform. The os parameter should be "darwin", "linux", etc.
func GetBuildFileForPlatform(directory, goos string) string {
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

// walkDirs recursively descends into subdirectories following DIRS files,
// collecting packages that have BUILD files. This is the heart of the
// discovery algorithm.
//
// The recursion stops at BUILD files: once we find a package, we don't
// look inside it for sub-packages. This keeps the model simple — a
// package is a leaf in the directory routing tree.
func walkDirs(directory string, packages *[]Package) {
	buildFile := getBuildFile(directory)

	if buildFile != "" {
		// This directory is a package. Read the BUILD commands and register it.
		commands := readLines(buildFile)
		language := inferLanguage(directory)
		name := inferPackageName(directory, language)

		*packages = append(*packages, Package{
			Name:          name,
			Path:          directory,
			BuildCommands: commands,
			Language:       language,
		})
		return
	}

	// Not a package — look for a DIRS file to find subdirectories.
	dirsFile := filepath.Join(directory, "DIRS")
	if !fileExists(dirsFile) {
		return
	}

	subdirs := readLines(dirsFile)
	for _, subdirName := range subdirs {
		subdirPath := filepath.Join(directory, subdirName)
		info, err := os.Stat(subdirPath)
		if err == nil && info.IsDir() {
			walkDirs(subdirPath, packages)
		}
	}
}

// DiscoverPackages walks DIRS files recursively starting from root,
// collecting packages with BUILD files. The returned list is sorted
// by package name for deterministic output.
//
// This is the main entry point for the discovery module. The root
// parameter should typically be the "code/" directory inside the repo.
func DiscoverPackages(root string) []Package {
	var packages []Package
	walkDirs(root, &packages)
	sort.Slice(packages, func(i, j int) bool {
		return packages[i].Name < packages[j].Name
	})
	return packages
}

// ReadLines is exported for use by the resolver (to read go.mod, etc.).
// It is identical to the internal readLines function.
func ReadLines(filepath string) []string {
	return readLines(filepath)
}
