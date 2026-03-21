// Package executor runs BUILD commands for packages that need rebuilding.
//
// # Parallel execution by levels
//
// The key insight of the build system is that not all packages depend on
// each other. The dependency graph can be partitioned into "levels" where
// packages within the same level have no dependencies on each other. These
// can safely run in parallel.
//
// For example, in a diamond dependency graph A→B, A→C, B→D, C→D:
//
//	Level 0: [A]     — no dependencies, build first
//	Level 1: [B, C]  — depend only on A, can run in parallel
//	Level 2: [D]     — depends on B and C, build last
//
// # Go's concurrency advantage
//
// This is where Go shines. Unlike the Python implementation (which uses
// ThreadPoolExecutor), Go uses goroutines — lightweight user-space threads
// managed by the Go runtime. Goroutines are ~2KB each (vs ~8MB for OS
// threads), so we can spawn thousands without worry.
//
// The pattern: for each level, launch one goroutine per package. A
// semaphore (buffered channel) limits concurrency to maxJobs. A WaitGroup
// ensures we wait for all goroutines in a level before proceeding to the
// next level.
//
// # Failure propagation
//
// If a package fails, all its transitive dependents are marked "dep-skipped".
// There is no point building something whose dependency is broken.
//
// # Progress tracking
//
// The executor accepts an optional progress.Tracker that receives events
// as packages are skipped, started, and finished. This powers a real-time
// progress bar in the terminal. The tracker is nil-safe — all Send calls
// are no-ops when tracker is nil, so callers don't need to guard.
package executor

import (
	"fmt"
	"os/exec"
	"strings"
	"sync"
	"time"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	progress "github.com/adhithyan15/coding-adventures/code/packages/go/progress-bar"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// BuildResult holds the outcome of building a single package.
type BuildResult struct {
	PackageName string  // Qualified name, e.g. "python/logic-gates"
	Status      string  // "built", "failed", "skipped", "dep-skipped", "would-build"
	Duration    float64 // Wall-clock seconds spent building
	Stdout      string  // Combined stdout from all BUILD commands
	Stderr      string  // Combined stderr from all BUILD commands
	ReturnCode  int     // Exit code of the last failing command, or 0
}

// runPackageBuild executes all BUILD commands for a single package.
//
// Commands are run sequentially — each must succeed before the next starts.
// This is because BUILD files are scripts: later commands may depend on
// earlier ones (e.g., "install dependencies" before "run tests").
//
// We use os/exec with shell execution (sh -c) so that BUILD commands can
// use shell features like pipes, redirects, and environment variables.
func runPackageBuild(pkg discovery.Package) BuildResult {
	start := time.Now()
	var allStdout, allStderr []string

	for _, command := range pkg.BuildCommands {
		cmd := exec.Command("sh", "-c", command)
		cmd.Dir = pkg.Path

		var stdout, stderr strings.Builder
		cmd.Stdout = &stdout
		cmd.Stderr = &stderr

		err := cmd.Run()
		allStdout = append(allStdout, stdout.String())
		allStderr = append(allStderr, stderr.String())

		if err != nil {
			elapsed := time.Since(start).Seconds()
			exitCode := 1
			if exitErr, ok := err.(*exec.ExitError); ok {
				exitCode = exitErr.ExitCode()
			}
			return BuildResult{
				PackageName: pkg.Name,
				Status:      "failed",
				Duration:    elapsed,
				Stdout:      strings.Join(allStdout, ""),
				Stderr:      strings.Join(allStderr, ""),
				ReturnCode:  exitCode,
			}
		}
	}

	elapsed := time.Since(start).Seconds()
	return BuildResult{
		PackageName: pkg.Name,
		Status:      "built",
		Duration:    elapsed,
		Stdout:      strings.Join(allStdout, ""),
		Stderr:      strings.Join(allStderr, ""),
		ReturnCode:  0,
	}
}

// ExecuteBuilds runs BUILD commands for packages respecting dependency order.
//
// This is the main orchestrator. It:
//  1. Gets independent_groups from the dependency graph
//  2. For each level, determines which packages need building
//  3. Skips packages whose deps failed ("dep-skipped")
//  4. Skips packages whose hashes haven't changed ("skipped")
//  5. In dry-run mode, marks packages as "would-build"
//  6. Otherwise, launches goroutines with semaphore-limited concurrency
//  7. Updates the cache after each build
//  8. Sends progress events to the tracker (if non-nil)
//
// The function returns a map from package name to BuildResult.
func ExecuteBuilds(
	packages []discovery.Package,
	graph *directedgraph.Graph,
	buildCache *cache.BuildCache,
	packageHashes map[string]string,
	depsHashes map[string]string,
	force bool,
	dryRun bool,
	maxJobs int,
	affectedSet map[string]bool,
	tracker *progress.Tracker,
) map[string]BuildResult {
	// Build a lookup from name to Package for quick access.
	pkgByName := make(map[string]discovery.Package)
	for _, p := range packages {
		pkgByName[p.Name] = p
	}

	// Get the parallel execution levels from the dependency graph.
	groups, err := graph.IndependentGroups()
	if err != nil {
		// Cycle detected — return an error result for all packages.
		results := make(map[string]BuildResult)
		for _, pkg := range packages {
			results[pkg.Name] = BuildResult{
				PackageName: pkg.Name,
				Status:      "failed",
				Stderr:      fmt.Sprintf("cycle detected in dependency graph: %v", err),
				ReturnCode:  1,
			}
		}
		return results
	}

	results := make(map[string]BuildResult)
	var resultsMu sync.Mutex // protects results map

	failedPackages := make(map[string]bool)
	var failedMu sync.Mutex // protects failedPackages

	for _, level := range groups {
		// Determine what to build in this level.
		var toBuild []discovery.Package

		for _, name := range level {
			pkg, ok := pkgByName[name]
			if !ok {
				continue
			}

			// Check if any dependency of this package failed.
			// In our graph, edge A→B means B depends on A. So B's deps
			// are its predecessors. We check if any predecessor (transitively)
			// has failed.
			depFailed := false
			preds := collectTransitivePredecessors(name, graph)
			failedMu.Lock()
			for dep := range preds {
				if failedPackages[dep] {
					depFailed = true
					break
				}
			}
			failedMu.Unlock()

			if depFailed {
				resultsMu.Lock()
				results[name] = BuildResult{
					PackageName: name,
					Status:      "dep-skipped",
				}
				resultsMu.Unlock()
				tracker.Send(progress.Event{Type: progress.Skipped, Name: name})
				continue
			}

			// Check if the package is in the affected set (git-diff mode).
			// If affectedSet is non-nil, it takes priority over cache.
			if affectedSet != nil && !affectedSet[name] {
				resultsMu.Lock()
				results[name] = BuildResult{
					PackageName: name,
					Status:      "skipped",
				}
				resultsMu.Unlock()
				tracker.Send(progress.Event{Type: progress.Skipped, Name: name})
				continue
			}

			// Check if the package needs building (cache fallback).
			pkgHash := packageHashes[name]
			depHash := depsHashes[name]

			if affectedSet == nil && !force && !buildCache.NeedsBuild(name, pkgHash, depHash) {
				resultsMu.Lock()
				results[name] = BuildResult{
					PackageName: name,
					Status:      "skipped",
				}
				resultsMu.Unlock()
				tracker.Send(progress.Event{Type: progress.Skipped, Name: name})
				continue
			}

			if dryRun {
				resultsMu.Lock()
				results[name] = BuildResult{
					PackageName: name,
					Status:      "would-build",
				}
				resultsMu.Unlock()
				tracker.Send(progress.Event{Type: progress.Skipped, Name: name})
				continue
			}

			toBuild = append(toBuild, pkg)
		}

		if len(toBuild) == 0 || dryRun {
			continue
		}

		// Execute this level in parallel using goroutines + semaphore.
		//
		// The semaphore pattern: a buffered channel acts as a counting
		// semaphore. Each goroutine sends to the channel before starting
		// work (acquiring the semaphore) and receives after finishing
		// (releasing it). If the channel is full, the goroutine blocks
		// until another finishes.
		workers := maxJobs
		if workers <= 0 {
			workers = len(toBuild)
			if workers > 8 {
				workers = 8
			}
		}

		semaphore := make(chan struct{}, workers)
		var wg sync.WaitGroup

		for _, pkg := range toBuild {
			wg.Add(1)
			go func(p discovery.Package) {
				defer wg.Done()
				semaphore <- struct{}{}        // acquire
				defer func() { <-semaphore }() // release

				tracker.Send(progress.Event{Type: progress.Started, Name: p.Name})
				result := runPackageBuild(p)
				tracker.Send(progress.Event{Type: progress.Finished, Name: p.Name, Status: result.Status})

				resultsMu.Lock()
				results[p.Name] = result
				resultsMu.Unlock()

				// Update the cache based on the result.
				if result.Status == "built" {
					buildCache.Record(
						p.Name,
						packageHashes[p.Name],
						depsHashes[p.Name],
						"success",
					)
				} else if result.Status == "failed" {
					failedMu.Lock()
					failedPackages[p.Name] = true
					failedMu.Unlock()
					buildCache.Record(
						p.Name,
						packageHashes[p.Name],
						depsHashes[p.Name],
						"failed",
					)
				}
			}(pkg)
		}

		wg.Wait()
	}

	return results
}

// collectTransitivePredecessors walks backwards through the graph from
// the given node, collecting all nodes it transitively depends on.
func collectTransitivePredecessors(node string, graph *directedgraph.Graph) map[string]bool {
	visited := make(map[string]bool)

	preds, err := graph.Predecessors(node)
	if err != nil {
		return visited
	}

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
