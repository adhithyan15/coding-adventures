// Build Tool — Incremental, Parallel Monorepo Build System
//
// This is the primary build tool for the coding-adventures monorepo.
// It discovers packages via recursive BUILD file walking, resolves dependencies,
// hashes source files, and only rebuilds packages whose source (or
// dependency source) has changed. Independent packages are built in
// parallel using Go goroutines.
//
// # The build flow
//
//  1. Find the repo root (walk up looking for .git)
//  2. Discover packages (walk BUILD files under code/)
//  3. Filter by language if requested
//  4. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod, Cargo.toml, package.json, mix.exs)
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
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	progress "github.com/adhithyan15/coding-adventures/code/packages/go/progress-bar"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cmdrender"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/reporter"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
	starlarkeval "github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/starlark"
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
	language := flag.String("language", "all", "Filter to language: python, ruby, go, rust, typescript, elixir, all")
	diffBase := flag.String("diff-base", "origin/main", "Git ref to diff against for change detection (default: origin/main)")
	cacheFile := flag.String("cache-file", ".build-cache.json", "Path to cache file (fallback when git diff unavailable)")
	detectLanguages := flag.Bool("detect-languages", false, "Output which language toolchains are needed based on git diff, then exit")
	emitPlan := flag.String("emit-plan", "", "Write build plan JSON to this path (used by CI detect job)")
	planFile := flag.String("plan-file", "", "Read build plan JSON, skip discovery/resolution/diff (used by CI build job)")

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

	// ── Plan-based execution path ─────────────────────────────────
	//
	// When --plan-file is set, we skip the expensive discovery/resolution/
	// git-diff steps (1-5) and reconstruct state from a pre-computed plan.
	// This is used by CI build jobs that receive a plan artifact from the
	// detect job.

	var packages []discovery.Package
	var graph *directedgraph.Graph
	var affectedSet map[string]bool
	usedPlan := false

	if *planFile != "" {
		bp, err := plan.Read(*planFile)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Warning: could not read plan file %s: %v\n", *planFile, err)
			fmt.Fprintln(os.Stderr, "Falling back to normal discovery flow")
		} else {
			// Reconstruct packages from plan entries.
			packages = make([]discovery.Package, len(bp.Packages))
			for i, pe := range bp.Packages {
				packages[i] = discovery.Package{
					Name:          pe.Name,
					Path:          filepath.Join(repoRoot, filepath.FromSlash(pe.RelPath)),
					BuildCommands: pe.BuildCommands,
					Language:      pe.Language,
					IsStarlark:    pe.IsStarlark,
					DeclaredSrcs:  pe.DeclaredSrcs,
					DeclaredDeps:  pe.DeclaredDeps,
				}

				// Re-read the platform-appropriate BUILD file for this package.
				// The plan's BuildCommands were generated on the detect job's OS
				// (typically Linux) and may contain shell syntax that doesn't
				// work on Windows (e.g., 2>/dev/null, shell quoting). By re-reading
				// the BUILD file for the current platform, we get the correct
				// commands for this runner's OS.
				platformBuild := discovery.GetBuildFileForPlatform(packages[i].Path, runtime.GOOS)
				if platformBuild != "" {
					if packages[i].IsStarlark {
						// Starlark BUILD files must be evaluated, not
						// executed line-by-line as shell commands.
						result, err := starlarkeval.EvaluateBuildFile(
							platformBuild,
							packages[i].Path,
							repoRoot,
						)
						if err != nil {
							fmt.Fprintf(os.Stderr, "Warning: Starlark re-eval for %s: %v\n", packages[i].Name, err)
						} else if len(result.Targets) > 0 {
							t := result.Targets[0]
							if len(t.Commands) > 0 {
								rendered, rerr := cmdrender.RenderCommands(t.Commands)
								if rerr != nil {
									packages[i].BuildCommands = starlarkeval.GenerateCommands(t)
								} else {
									packages[i].BuildCommands = rendered
								}
							} else {
								packages[i].BuildCommands = starlarkeval.GenerateCommands(t)
							}
						}
					} else {
						platformCmds := discovery.ReadLines(platformBuild)
						if len(platformCmds) > 0 {
							packages[i].BuildCommands = platformCmds
						}
					}
				}
			}

			// Reconstruct dependency graph from edges.
			graph = directedgraph.New()
			for _, pe := range bp.Packages {
				graph.AddNode(pe.Name)
			}
			for _, edge := range bp.DependencyEdges {
				graph.AddEdge(edge[0], edge[1])
			}

			// Reconstruct affected set.
			if bp.Force {
				*force = true
				affectedSet = nil
			} else if bp.AffectedPackages == nil {
				affectedSet = nil
			} else {
				affectedSet = make(map[string]bool)
				for _, name := range bp.AffectedPackages {
					affectedSet[name] = true
				}
			}

			// Apply language filter if requested.
			if *language != "all" {
				var filtered []discovery.Package
				for _, pkg := range packages {
					if pkg.Language == *language {
						filtered = append(filtered, pkg)
					}
				}
				packages = filtered
			}

			fmt.Printf("Loaded plan: %d packages from %s\n", len(packages), *planFile)
			usedPlan = true
		}
	}

	// ── Normal discovery flow (steps 1-5) ────────────────────────
	//
	// Runs when no plan file is provided, or when the plan file could
	// not be read.

	if !usedPlan {
		// Step 2: Discover packages.
		packages = discovery.DiscoverPackages(codeRoot)
		if len(packages) == 0 {
			fmt.Fprintln(os.Stderr, "No packages found.")
			return 0
		}

		// Step 2b: Evaluate Starlark BUILD files.
		//
		// For each discovered package, check if its BUILD file is Starlark.
		// If so, evaluate it through the Go starlark-interpreter to extract
		// declared targets (with srcs, deps, build commands). This replaces
		// the raw shell command lines with generated commands from the rule.
		starlarkCount := 0
		for i := range packages {
			pkg := &packages[i]
			if starlarkeval.IsStarlarkBuild(pkg.BuildContent) {
				pkg.IsStarlark = true
				result, err := starlarkeval.EvaluateBuildFile(
					filepath.Join(pkg.Path, "BUILD"),
					pkg.Path,
					repoRoot,
				)
				if err != nil {
					fmt.Fprintf(os.Stderr, "Warning: Starlark eval failed for %s: %v\n", pkg.Name, err)
					// Fall back to treating as shell BUILD.
					pkg.IsStarlark = false
					continue
				}

				if len(result.Targets) > 0 {
					// Use the first target's metadata (most BUILD files have one target).
					t := result.Targets[0]
					pkg.DeclaredSrcs = t.Srcs
					pkg.DeclaredDeps = t.Deps

					// If the target has structured commands (from cmd.star),
					// render them to shell strings.  Otherwise fall back to
					// GenerateCommands() which hardcodes commands by rule type.
					if len(t.Commands) > 0 {
						rendered, err := cmdrender.RenderCommands(t.Commands)
						if err != nil {
							fmt.Fprintf(os.Stderr, "Warning: cmd render failed for %s: %v\n", pkg.Name, err)
							pkg.BuildCommands = starlarkeval.GenerateCommands(t)
						} else {
							pkg.BuildCommands = rendered
						}
					} else {
						pkg.BuildCommands = starlarkeval.GenerateCommands(t)
					}
					starlarkCount++
				}
			}
		}
		if starlarkCount > 0 {
			fmt.Printf("Evaluated %d Starlark BUILD files\n", starlarkCount)
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
		graph = resolver.ResolveDependencies(packages)

		// Step 5: Git-diff change detection (default mode).
		// Git is the source of truth — no cache file needed for primary workflow.
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
	}

	// ── Emit plan and/or detect languages ─────────────────────────
	//
	// These are early-exit modes that compute metadata but don't build.

	if *emitPlan != "" {
		return emitBuildPlan(packages, graph, affectedSet, *force, *diffBase, repoRoot, *emitPlan, *detectLanguages)
	}

	if *detectLanguages {
		return detectNeededLanguages(packages, affectedSet, *force, repoRoot, *diffBase)
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

	// Steps 8-9: Execute builds with progress tracking.
	//
	// The progress tracker shows a live-updating bar on stderr while
	// builds run. It's nil in dry-run mode (no builds to track).
	var tracker *progress.Tracker
	if !*dryRun {
		tracker = progress.New(len(packages), os.Stderr, "")
		tracker.Start()
	}

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
		tracker,
	)

	if tracker != nil {
		tracker.Stop()
	}

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

// allLanguages is the canonical list of supported languages in the monorepo.
// The order is stable and matches the order used in CI toolchain setup.
var allLanguages = []string{"python", "ruby", "go", "typescript", "rust", "elixir"}

// sharedPrefixes are path prefixes that, when changed, mean ALL languages
// need rebuilding. These are cross-cutting concerns:
//   - code/grammars/ — shared grammar definitions used by all language lexers/parsers
//   - .github/ — CI configuration affects all languages
//   - code/programs/go/build-tool/ — the build tool itself
//   - code/specs/ — specifications that drive implementations across languages
var sharedPrefixes = []string{
	"code/grammars/",
	".github/",
	"code/programs/go/build-tool/",
	"code/specs/",
}

// detectNeededLanguages determines which language toolchains CI needs to
// install based on the affected packages. It outputs one line per language
// in the format "needs_<lang>=true|false" to both stdout and $GITHUB_OUTPUT
// (if the environment variable is set).
//
// Go is always needed because the build tool is written in Go.
//
// If --force is set, or if shared files (grammars, CI, build tool source)
// changed, all languages are marked as needed.
func detectNeededLanguages(
	packages []discovery.Package,
	affectedSet map[string]bool,
	force bool,
	repoRoot string,
	diffBase string,
) int {
	needed := make(map[string]bool)

	// Go is always needed — the build tool itself is Go.
	needed["go"] = true

	if force {
		// Force mode: all languages needed.
		for _, lang := range allLanguages {
			needed[lang] = true
		}
	} else if affectedSet == nil {
		// Git diff unavailable (nil means no diff data). Be safe: need everything.
		for _, lang := range allLanguages {
			needed[lang] = true
		}
	} else {
		// Check if shared files changed — if so, all languages are needed.
		changedFiles := gitdiff.GetChangedFiles(repoRoot, diffBase)
		sharedChanged := false
		for _, f := range changedFiles {
			for _, prefix := range sharedPrefixes {
				if strings.HasPrefix(f, prefix) {
					sharedChanged = true
					break
				}
			}
			if sharedChanged {
				break
			}
		}

		if sharedChanged {
			for _, lang := range allLanguages {
				needed[lang] = true
			}
		} else {
			// Only mark languages that have affected packages.
			for _, pkg := range packages {
				if affectedSet[pkg.Name] {
					needed[pkg.Language] = true
				}
			}
		}
	}

	// Output results. Each line is "needs_<lang>=true" or "needs_<lang>=false".
	// If $GITHUB_OUTPUT is set, also write there for GitHub Actions.
	ghOutput := os.Getenv("GITHUB_OUTPUT")
	var ghFile *os.File
	if ghOutput != "" {
		var err error
		ghFile, err = os.OpenFile(ghOutput, os.O_APPEND|os.O_WRONLY|os.O_CREATE, 0644)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Warning: could not open $GITHUB_OUTPUT: %v\n", err)
		} else {
			defer ghFile.Close()
		}
	}

	for _, lang := range allLanguages {
		value := needed[lang]
		line := fmt.Sprintf("needs_%s=%t", lang, value)
		fmt.Println(line)

		if ghFile != nil {
			fmt.Fprintln(ghFile, line)
		}
	}

	return 0
}

// computeLanguagesNeeded determines which language toolchains are needed
// based on affected packages, force mode, and shared file changes.
// Extracted for reuse by both detectNeededLanguages and emitBuildPlan.
func computeLanguagesNeeded(
	packages []discovery.Package,
	affectedSet map[string]bool,
	force bool,
	repoRoot string,
	diffBase string,
) map[string]bool {
	needed := make(map[string]bool)
	needed["go"] = true

	if force || affectedSet == nil {
		for _, lang := range allLanguages {
			needed[lang] = true
		}
		return needed
	}

	changedFiles := gitdiff.GetChangedFiles(repoRoot, diffBase)
	for _, f := range changedFiles {
		for _, prefix := range sharedPrefixes {
			if strings.HasPrefix(f, prefix) {
				for _, lang := range allLanguages {
					needed[lang] = true
				}
				return needed
			}
		}
	}

	for _, pkg := range packages {
		if affectedSet[pkg.Name] {
			needed[pkg.Language] = true
		}
	}
	return needed
}

// emitBuildPlan serializes the build plan to a JSON file and optionally
// outputs language detection flags. This is used by the CI detect job
// to produce a plan artifact that build jobs can consume.
func emitBuildPlan(
	packages []discovery.Package,
	graph *directedgraph.Graph,
	affectedSet map[string]bool,
	force bool,
	diffBase string,
	repoRoot string,
	outputPath string,
	alsoDetectLanguages bool,
) int {
	// Build package entries with repo-root-relative paths.
	entries := make([]plan.PackageEntry, len(packages))
	for i, pkg := range packages {
		rel, err := filepath.Rel(repoRoot, pkg.Path)
		if err != nil {
			rel = pkg.Path
		}
		entries[i] = plan.PackageEntry{
			Name:          pkg.Name,
			RelPath:       filepath.ToSlash(rel),
			Language:      pkg.Language,
			BuildCommands: pkg.BuildCommands,
			IsStarlark:    pkg.IsStarlark,
			DeclaredSrcs:  pkg.DeclaredSrcs,
			DeclaredDeps:  pkg.DeclaredDeps,
		}
	}

	// Build dependency edges from the graph.
	var edges [][2]string
	for _, node := range graph.Nodes() {
		successors, err := graph.Successors(node)
		if err != nil {
			continue
		}
		for _, succ := range successors {
			edges = append(edges, [2]string{node, succ})
		}
	}

	// Build affected packages list.
	var affectedList []string
	if affectedSet != nil {
		affectedList = make([]string, 0, len(affectedSet))
		for name := range affectedSet {
			affectedList = append(affectedList, name)
		}
	}

	// Compute languages needed.
	languagesNeeded := computeLanguagesNeeded(packages, affectedSet, force, repoRoot, diffBase)

	bp := &plan.BuildPlan{
		DiffBase:         diffBase,
		Force:            force,
		AffectedPackages: affectedList,
		Packages:         entries,
		DependencyEdges:  edges,
		LanguagesNeeded:  languagesNeeded,
	}

	if err := plan.Write(bp, outputPath); err != nil {
		fmt.Fprintf(os.Stderr, "Error writing build plan: %v\n", err)
		return 1
	}

	fmt.Printf("Build plan written to %s (%d packages)\n", outputPath, len(packages))

	// If --detect-languages was also set, output language flags.
	if alsoDetectLanguages {
		return outputLanguageFlags(languagesNeeded)
	}

	return 0
}

// outputLanguageFlags prints language detection results to stdout and
// $GITHUB_OUTPUT. Extracted from detectNeededLanguages for reuse.
func outputLanguageFlags(needed map[string]bool) int {
	ghOutput := os.Getenv("GITHUB_OUTPUT")
	var ghFile *os.File
	if ghOutput != "" {
		var err error
		ghFile, err = os.OpenFile(ghOutput, os.O_APPEND|os.O_WRONLY|os.O_CREATE, 0644)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Warning: could not open $GITHUB_OUTPUT: %v\n", err)
		} else {
			defer ghFile.Close()
		}
	}

	for _, lang := range allLanguages {
		value := needed[lang]
		line := fmt.Sprintf("needs_%s=%t", lang, value)
		fmt.Println(line)
		if ghFile != nil {
			fmt.Fprintln(ghFile, line)
		}
	}

	return 0
}
