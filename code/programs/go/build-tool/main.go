// Build Tool — Incremental, Parallel Monorepo Build System
//
// This is the primary build tool for the coding-adventures monorepo.
// It discovers packages via DIRS/BUILD files, resolves dependencies,
// hashes source files, and only rebuilds packages whose source (or
// dependency source) has changed. Independent packages are built in
// parallel using Go goroutines.
//
// # The build flow
//
//  1. Find the repo root (walk up looking for .git)
//  2. Discover packages (walk DIRS/BUILD files under code/)
//  3. Filter by language if requested
//  4. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod)
//  5. Hash all packages and their dependencies
//  6. Load cache, determine what needs building
//  7. If --dry-run, report what would build and exit
//  8. Execute builds in parallel by dependency level
//  9. Update and save cache
//  10. Print report
//  11. Exit with code 1 if any builds failed
//
// # Why Go?
//
// The Go implementation is the primary build tool because:
//   - It compiles to a single static binary — no runtime dependencies.
//   - Goroutines make parallel execution natural and lightweight.
//   - Fast startup time compared to Python/Ruby interpreters.
//   - The binary can be committed to the repo for zero-install CI.
package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/reporter"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// findRepoRoot walks up from the given directory (or cwd) looking for
// a .git directory. This is how we auto-detect the repo root without
// requiring the user to pass --root every time.
func findRepoRoot(start string) string {
	if start == "" {
		var err error
		start, err = os.Getwd()
		if err != nil {
			return ""
		}
	}

	current, err := filepath.Abs(start)
	if err != nil {
		return ""
	}

	for {
		gitDir := filepath.Join(current, ".git")
		if info, err := os.Stat(gitDir); err == nil && info.IsDir() {
			return current
		}

		parent := filepath.Dir(current)
		if parent == current {
			// Reached filesystem root without finding .git.
			return ""
		}
		current = parent
	}
}

func main() {
	os.Exit(run())
}

// run contains the actual logic, separated from main() so we can
// return an exit code cleanly.
func run() int {
	// CLI flags — using Go's standard flag package.
	root := flag.String("root", "", "Repo root directory (auto-detect from .git)")
	force := flag.Bool("force", false, "Rebuild everything regardless of cache")
	dryRun := flag.Bool("dry-run", false, "Show what would build without executing")
	jobs := flag.Int("jobs", runtime.NumCPU(), "Max parallel jobs")
	language := flag.String("language", "all", "Filter to language: python, ruby, go, all")
	diffBase := flag.String("diff-base", "origin/main", "Git ref to diff against for change detection (default: origin/main)")
	cacheFile := flag.String("cache-file", ".build-cache.json", "Path to cache file (fallback when git diff unavailable)")

	flag.Parse()

	// Step 1: Find the repo root.
	repoRoot := *root
	if repoRoot == "" {
		repoRoot = findRepoRoot("")
		if repoRoot == "" {
			fmt.Fprintln(os.Stderr, "Error: Could not find repo root (.git directory).")
			fmt.Fprintln(os.Stderr, "Use -root to specify the repo root.")
			return 1
		}
	}

	repoRoot, err := filepath.Abs(repoRoot)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		return 1
	}

	// The build starts from the code/ directory inside the repo root.
	codeRoot := filepath.Join(repoRoot, "code")
	if info, err := os.Stat(codeRoot); err != nil || !info.IsDir() {
		fmt.Fprintf(os.Stderr, "Error: %s does not exist or is not a directory.\n", codeRoot)
		return 1
	}

	// Step 2: Discover packages.
	packages := discovery.DiscoverPackages(codeRoot)
	if len(packages) == 0 {
		fmt.Fprintln(os.Stderr, "No packages found.")
		return 0
	}

	// Step 3: Filter by language if requested.
	if *language != "all" {
		var filtered []discovery.Package
		for _, pkg := range packages {
			if pkg.Language == *language {
				filtered = append(filtered, pkg)
			}
		}
		packages = filtered
		if len(packages) == 0 {
			fmt.Fprintf(os.Stderr, "No %s packages found.\n", *language)
			return 0
		}
	}

	fmt.Printf("Discovered %d packages\n", len(packages))

	// Step 4: Resolve dependencies.
	graph := resolver.ResolveDependencies(packages)

	// Step 5: Git-diff change detection (default mode).
	// Git is the source of truth — no cache file needed for primary workflow.
	var affectedSet map[string]bool

	if !*force {
		changedFiles := gitdiff.GetChangedFiles(repoRoot, *diffBase)
		if len(changedFiles) > 0 {
			changedPkgs := gitdiff.MapFilesToPackages(changedFiles, packages, repoRoot)
			if len(changedPkgs) > 0 {
				affectedSet = graph.AffectedNodes(changedPkgs)
				fmt.Printf("Git diff: %d packages changed, %d affected (including dependents)\n",
					len(changedPkgs), len(affectedSet))
			} else {
				fmt.Println("Git diff: no package files changed — nothing to build")
				affectedSet = make(map[string]bool) // empty = build nothing
			}
		} else {
			fmt.Println("Git diff unavailable — falling back to hash-based cache")
		}
	}

	// Step 6: Hash all packages (needed for cache fallback).
	packageHashes := make(map[string]string)
	depsHashes := make(map[string]string)

	for _, pkg := range packages {
		packageHashes[pkg.Name] = hasher.HashPackage(pkg)
		depsHashes[pkg.Name] = hasher.HashDeps(pkg.Name, graph, packageHashes)
	}

	// Step 7: Load cache (fallback if git diff didn't work).
	cachePath := *cacheFile
	if !filepath.IsAbs(cachePath) {
		cachePath = filepath.Join(repoRoot, cachePath)
	}

	buildCache := cache.New()
	buildCache.Load(cachePath)

	// Steps 8-9: Execute builds.
	results := executor.ExecuteBuilds(
		packages,
		graph,
		buildCache,
		packageHashes,
		depsHashes,
		*force,
		*dryRun,
		*jobs,
		affectedSet,
	)

	// Step 10: Save cache (secondary record, not primary mechanism).
	if !*dryRun {
		if err := buildCache.Save(cachePath); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: could not save cache: %v\n", err)
		}
	}

	// Step 10: Print report.
	reporter.PrintReport(results, nil)

	// Step 11: Exit with code 1 if any builds failed.
	for _, r := range results {
		if r.Status == "failed" {
			return 1
		}
	}
	return 0
}
