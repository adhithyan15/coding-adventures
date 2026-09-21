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
// A package with a closed platform-unsupported BUILD protocol is reported as
// "unsupported" without invoking a shell; its dependents are marked
// "dep-unsupported". There is no point building something whose dependency is
// broken or unavailable on this platform.
//
// # Progress tracking
//
// The executor accepts an optional progress.Tracker that receives events
// as packages are skipped, started, and finished. This powers a real-time
// progress bar in the terminal. The tracker is nil-safe — all Send calls
// are no-ops when tracker is nil, so callers don't need to guard.
package executor

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
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
	Status      string  // built, failed, skipped, dep-skipped, unsupported, dep-unsupported, or would-build
	ReasonCode  string  // Stable machine code for unsupported outcomes
	Duration    float64 // Wall-clock seconds spent building
	Stdout      string  // Combined stdout from all BUILD commands
	Stderr      string  // Combined stderr from all BUILD commands
	ReturnCode  int     // Exit code of the last failing command, or 0
}

var unsupportedBuildCommandRe = regexp.MustCompile(
	`^echo BUILD_TOOL_UNSUPPORTED:([A-Z][A-Z0-9_]{2,63}) -- skipped$`,
)

// unsupportedBuildCode recognizes the complete platform-unsupported protocol.
// It is deliberately closed: exactly one command, no shell operators, and one
// bounded uppercase diagnostic code. The caller does not execute the command;
// the echo-shaped record survives plan serialization while remaining harmless
// to older build-tool readers.
func unsupportedBuildCode(commands []string) (string, bool) {
	if len(commands) != 1 {
		return "", false
	}
	match := unsupportedBuildCommandRe.FindStringSubmatch(strings.TrimSpace(commands[0]))
	if len(match) != 2 {
		return "", false
	}
	return match[1], true
}

// runPackageBuild executes all BUILD commands for a single package.
//
// Commands are run sequentially — each must succeed before the next starts.
// This is because BUILD files are scripts: later commands may depend on
// earlier ones (e.g., "install dependencies" before "run tests").
//
// We use os/exec with shell execution so that BUILD commands can use
// shell features like pipes, redirects, and environment variables.
// On Windows, we use "cmd /C"; on Unix, "sh -c".
func runPackageBuild(pkg discovery.Package, clippy bool) BuildResult {
	start := time.Now()
	if code, ok := unsupportedBuildCode(pkg.BuildCommands); ok {
		return BuildResult{
			PackageName: pkg.Name,
			Status:      "unsupported",
			ReasonCode:  code,
		}
	}

	return runCommands(pkg, clippyGatedCommands(pkg, clippy), start)
}

// clippyGatedCommands returns the command list to run for a package. For Rust
// packages under the clippy gate it prepends a `cargo clippy --all-targets --
// -D warnings` step, run from the crate directory so warnings are promoted to
// errors *before* the BUILD commands execute. Non-Rust packages and the
// un-gated path are returned unchanged.
func clippyGatedCommands(pkg discovery.Package, clippy bool) []string {
	if !clippy || pkg.Language != "rust" {
		return pkg.BuildCommands
	}
	step, ok := clippyStepFor(pkg.BuildCommands)
	if !ok {
		return pkg.BuildCommands
	}
	return append([]string{step}, pkg.BuildCommands...)
}

// clippyStepFor derives the clippy command for a Rust package from its resolved
// (platform-selected) BUILD commands, so that clippy runs *exactly where the
// BUILD's own cargo invocation runs* — never on a platform where the crate is
// deliberately skipped.
//
// Three shapes cover every Rust BUILD in the repo:
//
//   - Unconditional cargo (e.g. `cargo test -p bitset`): lint unconditionally.
//   - Platform-guarded cargo (e.g. paint-metal's
//     `if [ "$(uname)" = "Darwin" ]; then cargo test -p paint-metal ...; else echo SKIP; fi`):
//     reuse the *same* condition, `if <cond>; then cargo clippy ...; fi`, so the
//     crate is linted on its native platform and skipped elsewhere.
//   - Pure skip (e.g. paint-vm-direct2d's `echo "SKIP: ... requires Windows"` on
//     non-Windows): no cargo runs here, so no clippy step is emitted. On the
//     crate's native platform its BUILD_windows resolves to unconditional cargo
//     and the first shape applies.
//
// # Why every command is scanned, not just the first
//
// This function used to look at `buildCommands[0]` alone. That silently
// disabled the whole clippy gate for any Rust package whose BUILD opens with a
// *preamble* rather than with cargo — and plenty do:
//
//	set -e
//	export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld
//	cargo test --package coding-adventures-sql-planner
//
// (`#!/bin/sh` and `#`-comments are already stripped by the BUILD reader, but
// `set -e`, `export …`, `cd …` and `echo "[pkg] Building…"` are not.) The first
// line is `set -e`, which contains no `cargo `, so the old code returned
// ("", false) and the package was never linted at all — on any platform, on
// any CI leg. `coding-adventures-sql-planner` had a live
// `clippy::manual_is_multiple_of` error sitting on main because of exactly
// this. Scanning the *whole* command list closes that hole: a package is
// linted iff its BUILD runs cargo anywhere.
//
// When both shapes appear, unconditional wins: if any command runs cargo
// unguarded, the crate compiles on this platform, so the lint should not be
// hidden behind some other command's platform guard.
//
// Returns (command, true) when a clippy step should run, or ("", false) when the
// resolved BUILD does not compile the crate on this platform.
func clippyStepFor(buildCommands []string) (string, bool) {
	const clippy = "cargo clippy --all-targets -- -D warnings"

	guarded := ""
	for _, raw := range buildCommands {
		command := strings.TrimSpace(raw)

		// Platform-guarded: `if <cond>; then <body>; ...`. Reuse <cond> iff the
		// guarded body actually runs cargo on the matching platform. Remember
		// the first such guard but keep scanning — an unconditional cargo later
		// in the list is the stronger signal.
		if strings.HasPrefix(command, "if ") {
			if idx := strings.Index(command, "; then "); idx != -1 && guarded == "" {
				cond := command[len("if "):idx]
				body := command[idx+len("; then "):]
				if runsCargo(body) {
					guarded = fmt.Sprintf("if %s; then %s; fi", cond, clippy)
				}
			}
			continue
		}

		// Unconditional cargo build/test: lint unconditionally.
		if runsCargo(command) {
			return clippy, true
		}
	}

	if guarded != "" {
		return guarded, true
	}

	// No cargo anywhere (e.g. a bare `echo SKIP`): nothing to lint here.
	return "", false
}

// runsCargo reports whether a shell command actually *invokes* cargo, as
// opposed to merely mentioning it.
//
// The distinction matters because several compile-only crates document their
// manual build command inside an `echo`:
//
//	echo "font-parser-node: compile-only crate (requires Node.js dev headers)"
//	echo "  To build: cargo build -p font-parser-node --release"
//
// A naive `strings.Contains(command, "cargo ")` sees the second line and
// concludes the crate is built here — so it would attach a clippy step to a
// package whose BUILD deliberately builds nothing. That is the mirror image of
// the bug this function exists to prevent, and it would turn CI red for crates
// that CI cannot even link.
//
// The rule: erase every single- and double-quoted span, then look for a bare
// `cargo` word in what remains. Quoted text is data (an echo argument, a commit
// message, a `--message-format` value); unquoted text is the command line.
// This keeps the shapes that genuinely run cargo behind a prefix:
//
//	RUSTDOCFLAGS="-D warnings" cargo doc -p x --no-deps   → env assignment
//	cd "$WORKSPACE" && cargo test --package y             → after `&&`
//
// Shell quoting is far richer than this (escapes, `$(…)`, heredocs), but BUILD
// files in this repo are deliberately simple one-liners, and the failure mode
// is symmetric and visible: a missed cargo means a missing lint step, which is
// what the surrounding tests pin.
func runsCargo(command string) bool {
	var unquoted strings.Builder
	quote := rune(0)
	for _, r := range command {
		switch {
		case quote != 0:
			if r == quote {
				quote = 0
			}
		case r == '\'' || r == '"':
			quote = r
			// Keep a separator so `echo"cargo"` cannot fuse into a word.
			unquoted.WriteByte(' ')
		default:
			unquoted.WriteRune(r)
		}
	}

	for _, field := range strings.FieldsFunc(unquoted.String(), func(r rune) bool {
		return r == ' ' || r == '\t' || r == ';' || r == '&' || r == '|' || r == '(' || r == ')'
	}) {
		if field == "cargo" {
			return true
		}
	}
	return false
}

// runCommands executes an explicit command list for a package sequentially,
// stopping at the first failure. It is shared by the plain build path and the
// clippy-gated path (which prepends a `cargo clippy` command).
func runCommands(pkg discovery.Package, commands []string, start time.Time) BuildResult {
	var allStdout, allStderr []string

	for _, command := range commands {
		cmd := shellCommand(command)
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

// shellCommand returns an exec.Cmd that runs the given command string in
// the platform-appropriate shell. On Windows this is "cmd /C command"; on
// Unix (macOS, Linux) it is "sh -c command".
//
// This is the same approach used by the Rust build tool (which uses
// cfg!(target_os = "windows") to select between cmd and sh). Python's
// subprocess.run(shell=True) and Node's child_process.exec() handle this
// automatically, but Go requires explicit selection.
func shellCommand(command string) *exec.Cmd {
	return shellCommandForOS(command, runtime.GOOS)
}

// shellCommandForOS is the testable version of shellCommand that accepts
// an explicit OS name. This allows tests to verify Windows behavior on
// non-Windows hosts.
func shellCommandForOS(command string, goos string) *exec.Cmd {
	if goos == "windows" {
		command = rewriteInlineEnvPrefixForWindows(command)
		cmd := exec.Command("cmd", "/C", command)
		// See executor_windows.go: this overrides the actual command line
		// used to launch the process so cmd.exe receives it unescaped,
		// while cmd.Args above stays intact for callers/tests.
		setRawWindowsCmdLine(cmd, command)
		return cmd
	}
	return exec.Command("sh", "-c", command)
}

// inlineEnvPrefixPattern matches a POSIX inline environment-variable prefix
// — `VAR=value command...` or `VAR="value" command...` — followed by a
// command. A double-quoted value may contain spaces (that's the point of
// quoting it, e.g. `RUSTDOCFLAGS="-D warnings"`) but not `$`, backticks, or
// another `"`, since those mean the value isn't a plain string. A bare
// (unquoted) value may contain none of those either, plus no whitespace
// (unquoted whitespace is where the value ends and the command begins).
// Either way, `&`, `|`, `;`, parens, and `<`/`>` are excluded from the
// value: those are shell control-flow/redirection characters with no
// single cmd.exe translation, and every BUILD file in this repo using one
// of those shapes already carries a hand-written BUILD_windows override
// (see readLines's own doc comment on `set -e` for the parallel case:
// don't guess at a translation cmd.exe can't express).
var inlineEnvPrefixPattern = regexp.MustCompile(
	`^([A-Za-z_][A-Za-z0-9_]*)=(?:"([^"$` + "`" + `&|;()<>]*)"|([^"$` + "`" + `&|;()<>\s]+))\s+(\S.*)$`,
)

// chainedAssignmentPattern matches a second leading `VAR=` at the start of
// what rewriteInlineEnvPrefixForWindows already isolated as "the rest of
// the command" (e.g. `DOTNET_CLI_HOME=$HOME dotnet test` inside
// `DOTNET_SKIP...=1 DOTNET_CLI_HOME=$HOME dotnet test`). Two prefixes
// stacked like this only mean "set both, then run the command" in POSIX
// shells — cmd.exe's `set` can express that too (two `set ... &&` in a
// row), but only correctly if EVERY assignment in the chain is itself
// translatable, and by the time this file is walking a second assignment
// it has already committed to treating the first `\s+` as the split point
// between name and command. Getting that ambiguous rather than silently
// mistranslating: bail out and let cmd.exe's existing (correct) rejection
// stand, matching every other multi-assignment BUILD line's reliance on a
// hand-written BUILD_windows override.
var chainedAssignmentPattern = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*=`)

// rewriteInlineEnvPrefixForWindows translates a simple POSIX inline
// environment-variable prefix into cmd.exe's equivalent, so a BUILD line
// like `RUSTDOCFLAGS="-D warnings" cargo doc -p widget --no-deps` — which
// runs fine under `sh -c` but is a syntax error to `cmd /C` (cmd has no
// notion of "set this variable for just the following command") — becomes
// `set "RUSTDOCFLAGS=-D warnings"&& cargo doc -p widget --no-deps`. `set`
// inside one `cmd /C` invocation only affects that invocation's own
// environment, which already matches the POSIX prefix's scope: each
// BuildCommands line is its own separate process (see runCommands' doc
// comment), so there is nothing for the variable to leak into either way.
// Lines that don't match inlineEnvPrefixPattern, or whose "rest" is itself
// another assignment, are returned unchanged — cmd.exe will still reject
// anything more complex, exactly as it did before this function existed,
// which is the correct outcome for a shape this translation can't safely
// express.
//
// Known limitation, not currently reached by any BUILD file in this repo:
// cmd.exe expands a literal `%NAME%` inside a command line (including
// inside `set "..."`'s own quotes) before `set` ever runs, so a value
// containing `%` would not round-trip byte-for-byte the way it does under
// `sh -c`. None of the values this rewrite has ever been exercised against
// contain `%`; if one ever does, that BUILD line needs the same treatment
// as every other unrewritable shape here — a hand-written BUILD_windows
// override, not a guess bolted onto this regex.
func rewriteInlineEnvPrefixForWindows(command string) string {
	idx := inlineEnvPrefixPattern.FindStringSubmatchIndex(command)
	if idx == nil {
		return command
	}
	name := command[idx[2]:idx[3]]
	// Exactly one of the quoted-value group (2) or bare-value group (3)
	// participated in the match — a -1 start index means that alternative
	// wasn't taken. Reading the group that lost is what would make
	// `VAR=""` (a legal, empty value) indistinguishable from "no value
	// captured"; indices avoid that ambiguity FindStringSubmatch cannot.
	var value string
	if idx[4] >= 0 {
		value = command[idx[4]:idx[5]] // quoted: `VAR="value"`
	} else {
		value = command[idx[6]:idx[7]] // bare: `VAR=value`
	}
	rest := command[idx[8]:idx[9]]
	if chainedAssignmentPattern.MatchString(rest) {
		return command
	}
	return `set "` + name + `=` + value + `"&& ` + rest
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
	clippy bool,
) map[string]BuildResult {
	// Build a lookup from name to Package for quick access.
	pkgByName := make(map[string]discovery.Package)
	pathToPkg := make(map[string]string, len(packages))
	for _, p := range packages {
		pkgByName[p.Name] = p
		pathToPkg[filepath.Clean(p.Path)] = p.Name
	}
	resourceLocker := newBuildResourceLocker()

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
	unsupportedPackages := make(map[string]bool)

	for _, level := range groups {
		// Determine what to build in this level.
		var toBuild []discovery.Package

		for _, name := range level {
			pkg, ok := pkgByName[name]
			if !ok {
				continue
			}
			unsupportedCode, selfUnsupported := unsupportedBuildCode(pkg.BuildCommands)

			// Check if any dependency of this package failed.
			// In our graph, edge A→B means B depends on A. So B's deps
			// are its predecessors. We check if any predecessor (transitively)
			// has failed.
			depFailed := false
			depUnsupported := false
			preds := collectTransitivePredecessors(name, graph)
			failedMu.Lock()
			for dep := range preds {
				if failedPackages[dep] {
					depFailed = true
					break
				}
				if unsupportedPackages[dep] {
					depUnsupported = true
				}
			}
			failedMu.Unlock()

			if depFailed && !selfUnsupported {
				resultsMu.Lock()
				results[name] = BuildResult{
					PackageName: name,
					Status:      "dep-skipped",
				}
				resultsMu.Unlock()
				tracker.Send(progress.Event{Type: progress.Skipped, Name: name})
				continue
			}
			if depUnsupported && !selfUnsupported {
				resultsMu.Lock()
				results[name] = BuildResult{
					PackageName: name,
					Status:      "dep-unsupported",
					ReasonCode:  "DEPENDENCY_UNSUPPORTED",
				}
				resultsMu.Unlock()
				unsupportedPackages[name] = true
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

			if selfUnsupported {
				resultsMu.Lock()
				results[name] = BuildResult{
					PackageName: name,
					Status:      "unsupported",
					ReasonCode:  unsupportedCode,
				}
				resultsMu.Unlock()
				unsupportedPackages[name] = true
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
				releaseResources := resourceLocker.Acquire(
					buildResourceKeys(p, pathToPkg),
					buildReadResourceKeys(p, pathToPkg),
				)
				defer releaseResources()

				tracker.Send(progress.Event{Type: progress.Started, Name: p.Name})
				result := runPackageBuild(p, clippy)
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

// maxManifestBytes bounds a package.json read. Real manifests in this repo
// are a few KB; anything past 4 MB is not a manifest we need to parse.
const maxManifestBytes = 4 << 20

var relPathRe = regexp.MustCompile(`(?:\.\.?[/\\][^ \t\r\n"'&|;()]+)+`)

// buildResourceLocker hands out one RWMutex per resource key.
//
// Two kinds of access are distinguished, and the distinction is what makes
// this affordable (see B07). A package whose BUILD text names `../X` runs an
// install *into* X, which `npm ci` begins by deleting — that is a WRITE and
// must be exclusive. A package that merely resolves `file:`-linked imports
// out of X needs X to sit still for the duration, but does not mind other
// readers — that is a READ and may be shared.
//
// Using exclusive locks for both would be a cure worse than the disease:
// `typescript/directed-graph` is read by 232 packages, so an exclusive lock
// per reader would serialise nearly the whole TypeScript tree.
type buildResourceLocker struct {
	mu    sync.Mutex
	locks map[string]*sync.RWMutex
}

func newBuildResourceLocker() *buildResourceLocker {
	return &buildResourceLocker{locks: make(map[string]*sync.RWMutex)}
}

// Acquire takes write locks on writeKeys and read locks on readKeys,
// returning a release function.
//
// Both sets are merged into ONE sorted sequence before anything is taken.
// That total order is what makes multi-key acquisition deadlock-free: two
// goroutines with overlapping key sets always contend in the same direction.
// A key present in both sets is taken for writing only — the stronger claim
// subsumes the weaker, and taking both would self-deadlock on a non-reentrant
// RWMutex.
func (l *buildResourceLocker) Acquire(writeKeys, readKeys []string) func() {
	exclusive := make(map[string]bool, len(writeKeys))
	for _, k := range writeKeys {
		exclusive[k] = true
	}

	ordered := make([]string, 0, len(writeKeys)+len(readKeys))
	seen := make(map[string]bool, len(writeKeys)+len(readKeys))
	for _, k := range append(append([]string{}, writeKeys...), readKeys...) {
		if !seen[k] {
			seen[k] = true
			ordered = append(ordered, k)
		}
	}

	if len(ordered) == 0 {
		return func() {}
	}

	sort.Strings(ordered)
	held := make([]func(), 0, len(ordered))
	for _, key := range ordered {
		lock := l.lockFor(key)
		if exclusive[key] {
			lock.Lock()
			held = append(held, lock.Unlock)
		} else {
			lock.RLock()
			held = append(held, lock.RUnlock)
		}
	}

	return func() {
		for i := len(held) - 1; i >= 0; i-- {
			held[i]()
		}
	}
}

func (l *buildResourceLocker) lockFor(key string) *sync.RWMutex {
	l.mu.Lock()
	defer l.mu.Unlock()

	lock, ok := l.locks[key]
	if !ok {
		lock = &sync.RWMutex{}
		l.locks[key] = lock
	}
	return lock
}

// buildReadResourceKeys returns the packages whose `node_modules` this
// package's build READS but never writes.
//
// The need for this is not visible in BUILD text. `code/sites/blog/BUILD` is
// `npm install --silent && npm run clean && ...` and names no relative path
// at all, yet `npm run clean` runs `forme` under tsx, and tsx resolves a
// chain of `file:`-linked imports (forme-cli -> cli-builder -> state-machine
// -> directed-graph). Node resolves a linked package's imports from its REAL
// path, so every directory along that chain must have a populated
// `node_modules` at that instant — while 113 other BUILD files are entitled
// to run `npm ci` into one of them, which starts by deleting it.
//
// The closure walked here is the same one forme-cli/bin/bootstrap.mjs
// computes for its install ordering: `file:` specifiers in dependencies,
// devDependencies, optionalDependencies and peerDependencies, followed
// transitively.
//
// Scope is deliberately npm-only. This is a `node_modules` problem; other
// ecosystems' shared state is covered by the `global:` keys above.
func buildReadResourceKeys(pkg discovery.Package, pathToPkg map[string]string) []string {
	if !usesNodeModules(pkg) {
		return nil
	}

	found := make(map[string]bool)
	visited := make(map[string]bool)
	var walk func(dir string)
	walk = func(dir string) {
		if visited[dir] {
			return
		}
		visited[dir] = true

		for _, target := range fileDependencyDirs(dir) {
			// Only ever step into a directory the discovery pass already
			// identified as a package. A `file:` specifier is just text from
			// a manifest, so "file:../../../../etc" resolves happily outside
			// the checkout; confining the walk to known packages means a
			// hostile or simply wrong specifier can neither send us
			// traversing the filesystem nor point the read below at an
			// arbitrary path. Nothing is lost: a file: dependency that is not
			// a discovered package has no BUILD, so nothing can reinstall
			// into it and there is no lock worth taking.
			name, known := pathToPkg[target]
			if !known {
				continue
			}
			if name != pkg.Name {
				found[name] = true
			}
			walk(target)
		}
	}
	walk(filepath.Clean(pkg.Path))

	keys := make([]string, 0, len(found))
	for name := range found {
		keys = append(keys, name)
	}
	sort.Strings(keys)
	return keys
}

// nodeToolRe matches a BUILD command that drives a Node package manager or
// runner, as a whole word so `npmlike-thing` does not match.
//
// The list is wider than `npm` on purpose. The directory at risk is
// `node_modules`, and anything that installs into it or resolves imports out
// of it is exposed -- a package driving `tsx` or `pnpm` directly is in the
// same position as one driving `npm`, and gating on `npm ` alone would leave
// it taking no read locks at all.
var nodeToolRe = regexp.MustCompile(`(^|[ 	;&|(])(npm|npx|pnpm|yarn|tsx|vite|vitest|node)([ 	]|$)`)

// usesNodeModules reports whether any BUILD command drives a Node tool, and
// so could install into or resolve out of a shared node_modules. Packages
// that touch none cannot participate in the race.
func usesNodeModules(pkg discovery.Package) bool {
	for _, command := range pkg.BuildCommands {
		if nodeToolRe.MatchString(command) {
			return true
		}
	}
	return false
}

// fileDependencyDirs reads one package.json and returns the absolute
// directories named by its `file:` dependency specifiers.
//
// A missing or malformed package.json yields nothing rather than an error:
// key derivation runs for every package on every build, and a package
// without a readable manifest simply has no file: links to protect.
func fileDependencyDirs(dir string) []string {
	// Reject anything that is not a plain, plausibly-sized file BEFORE
	// reading it. This runs inside the build goroutine, after a semaphore
	// slot has been taken, so a read that never returns does not just fail
	// one package -- it burns a worker slot and the level's WaitGroup never
	// completes, hanging the build until CI kills the job. A package.json
	// symlinked to /dev/zero reads without EOF, and a FIFO blocks forever;
	// Lstat rejects the symlink without following it and rejects the FIFO
	// outright.
	path := filepath.Join(dir, "package.json")
	info, err := os.Lstat(path)
	if err != nil || !info.Mode().IsRegular() || info.Size() > maxManifestBytes {
		return nil
	}

	raw, err := os.ReadFile(path)
	if err != nil {
		return nil
	}

	var manifest struct {
		Dependencies         map[string]string `json:"dependencies"`
		DevDependencies      map[string]string `json:"devDependencies"`
		OptionalDependencies map[string]string `json:"optionalDependencies"`
		PeerDependencies     map[string]string `json:"peerDependencies"`
	}
	if err := json.Unmarshal(raw, &manifest); err != nil {
		return nil
	}

	var dirs []string
	for _, section := range []map[string]string{
		manifest.Dependencies,
		manifest.DevDependencies,
		manifest.OptionalDependencies,
		manifest.PeerDependencies,
	} {
		for _, spec := range section {
			if !strings.HasPrefix(spec, "file:") {
				continue
			}
			rel := strings.TrimPrefix(spec, "file:")
			dirs = append(dirs, filepath.Clean(filepath.Join(dir, filepath.FromSlash(rel))))
		}
	}
	sort.Strings(dirs)
	return dirs
}

func buildResourceKeys(pkg discovery.Package, pathToPkg map[string]string) []string {
	return buildResourceKeysForOS(pkg, pathToPkg, runtime.GOOS)
}

// buildResourceKeysForOS is the testable version of buildResourceKeys that
// accepts an explicit OS name. This allows tests to verify Windows-specific
// lock key behaviour on non-Windows hosts.
func buildResourceKeysForOS(pkg discovery.Package, pathToPkg map[string]string, goos string) []string {
	keys := map[string]bool{
		pkg.Name: true,
	}

	for _, command := range pkg.BuildCommands {
		if goos == "windows" && isGradleCommand(command) {
			// Windows runners showed long-lived Java/Gradle processes when multiple
			// independent JVM packages were launched together. Serialize Gradle-backed
			// Java/Kotlin builds there the same way we already protect other shared
			// package-manager state such as Hex and LuaRocks.
			if pkg.Language == "java" || pkg.Language == "kotlin" {
				keys["global:gradle-windows-runner"] = true
			}
		}

		if pkg.Language == "elixir" && strings.Contains(command, "mix deps.get") {
			// Hex persists a shared cache under ~/.hex, which is not safe for
			// concurrent writes across packages in CI.
			keys["global:hex-cache"] = true
		}

		if strings.Contains(command, "rustup target add") {
			// rustup mutates the shared toolchain under RUSTUP_HOME.
			// Serialise target installs so concurrent wasm package builds do not
			// race while adding the same target.
			keys["global:rustup-targets"] = true
		}

		if pkg.Language == "lua" &&
			(strings.Contains(command, "luarocks make") || strings.Contains(command, "luarocks remove")) {
			// LuaRocks uses a shared rocks tree under HOME on every platform.
			// Concurrent writes race for that tree lock and can leave packages
			// observing partially-installed local dependencies. Serialise Lua
			// BUILDs that mutate the tree so sibling rock installs stay stable.
			keys["global:luarocks-tree"] = true
		}

		if pkg.Language == "haskell" && strings.Contains(command, "cabal ") {
			// Cabal mutates the shared store under ~/.cabal and coordinates builds
			// through package database locks. Parallel package tests can contend on
			// those global resources, so serialize Cabal-backed Haskell BUILDs.
			keys["global:cabal-store"] = true
		}

		if (pkg.Language == "csharp" || pkg.Language == "fsharp") && strings.Contains(command, "dotnet ") {
			// NuGet first-run migrations use global process mutex state outside
			// DOTNET_CLI_HOME. Parallel dotnet package builds can race there on CI.
			keys["global:dotnet-cli"] = true
		}

		for _, raw := range relPathRe.FindAllString(command, -1) {
			normalized := strings.ReplaceAll(raw, "\\", "/")
			abs := filepath.Clean(filepath.Join(pkg.Path, filepath.FromSlash(normalized)))
			if name, ok := pathToPkg[abs]; ok {
				keys[name] = true
			}
		}
	}

	result := make([]string, 0, len(keys))
	for key := range keys {
		result = append(result, key)
	}
	return result
}

func isGradleCommand(command string) bool {
	trimmed := strings.TrimSpace(command)
	if trimmed == "" {
		return false
	}

	for _, prefix := range []string{
		"gradle ",
		"gradle\t",
		"gradlew ",
		"gradlew\t",
		"gradlew.bat ",
		"gradlew.bat\t",
		"./gradlew ",
		"./gradlew\t",
		".\\gradlew ",
		".\\gradlew\t",
	} {
		if strings.HasPrefix(trimmed, prefix) {
			return true
		}
	}

	return trimmed == "gradle" ||
		trimmed == "gradlew" ||
		trimmed == "gradlew.bat" ||
		trimmed == "./gradlew" ||
		trimmed == ".\\gradlew"
}
