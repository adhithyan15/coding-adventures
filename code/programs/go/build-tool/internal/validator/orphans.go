package validator

// Orphan-crate detection.
//
// # The failure this exists to prevent
//
// The build tool discovers work by scanning for BUILD files. That is a good
// design — it keeps "what do I build?" declarative and language-agnostic — but
// it has one structurally silent failure mode: **a crate with no BUILD file is
// not a package the tool ever hears about.** It is not built, its test targets
// are never compiled, its assertions never run, and `cargo clippy --all-targets
// -- -D warnings` never lints it, on any platform.
//
// Nothing about that is visible. A Rust crate in the `code/packages/rust`
// workspace still *compiles* constantly, because every sibling that lists it as
// a path dependency drags it in. Compiling is not linting and it is not testing,
// so a crate can sit on main for months accumulating red tests and `-D warnings`
// errors while every CI run stays green. That is exactly how 84 crates
// accumulated before this check existed, two of them with live clippy errors.
//
// The scaffold generator (code/programs/go/scaffold-generator) does template a
// BUILD file, so this was never a tool bug — it is process drift from crates
// created by hand. A check is the only durable fix: writing the missing BUILD
// files fixes today's omissions, but only a gate makes the next one impossible.
//
// # What counts as covered
//
// Every directory containing a `Cargo.toml` must be covered, where "covered"
// means that directory **or one of its ancestors** holds a BUILD file. The
// ancestor rule is essential and not a loophole: this repo is full of native
// extension crates nested inside another language's package —
// `code/packages/python/conduit/ext/conduit_native/Cargo.toml`, for instance,
// is compiled by `code/packages/python/conduit/BUILD`. Requiring a BUILD in the
// crate's own directory would flag ~170 such crates that are already perfectly
// well built, and a gate that cries wolf gets switched off.
//
// A BUILD must also contain at least one runnable line. An empty file satisfies
// discovery, produces an empty command list, and makes the package report
// success having compiled, tested and linted nothing — a one-`touch` bypass
// that would leave no reviewable artifact behind. Blank and `#`-comment lines do
// not count.
//
// # The exemption ledger
//
// A crate that genuinely cannot be covered needs an explicit, reasoned entry in
// `code/BUILD-EXEMPTIONS`. The file distinguishes two kinds, because "we
// deliberately never build this" and "we have not got to this yet" are different
// claims and should not be spelled the same way:
//
//   - EXCLUDED — genuinely never gets a BUILD. A compile-only bridge crate with
//     nothing to run, a crate that only builds under a foreign toolchain, and so
//     on. The reason must say which, and where the crate IS covered.
//   - PENDING — a known gap with a real crate behind it. This is a backlog, and
//     it is meant to shrink.
//
// Both suppress the failure. Keeping them distinct means the PENDING list is a
// visible, countable debt rather than a drawer things get quietly filed into.
//
// # Why stale entries are also an error
//
// An allowlist that only ever grows is a rug to sweep things under. So this also
// fails when an entry names a path that has since become covered, or that no
// longer exists. Landing a BUILD for a PENDING crate therefore *forces* the same
// PR to delete its exemption line, and the backlog cannot silently outlive the
// problem it describes.
//
// # Fail closed
//
// Every ambiguous case resolves toward reporting rather than skipping. A stat
// error that is not "does not exist" is a hard failure rather than a shrug; a
// symlinked directory is followed rather than ignored; a ledger path that is not
// lexically inside the scanned tree is rejected instead of being stat-ed. A gate
// that treats "I could not tell" as "clean" is not a gate.

import (
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// ExemptionsFile is the repo-relative path of the exemption ledger.
const ExemptionsFile = "code/BUILD-EXEMPTIONS"

// scanRoot is the repo-relative directory tree searched for crates. Everything
// this repo builds lives under it.
const scanRoot = "code"

// buildFileNames are the BUILD file names discovery recognizes. A platform-
// specific BUILD is still a BUILD: a crate that ships only `BUILD_windows` is
// discovered and built on Windows, and must not be reported as an orphan just
// because it has no generic file. Keep this in sync with
// internal/discovery's resolution order.
var buildFileNames = []string{
	"BUILD",
	"BUILD_windows",
	"BUILD_mac",
	"BUILD_linux",
	"BUILD_mac_and_linux",
}

// skipDirNames are never descended into. These hold build artifacts and
// vendored third-party sources, which contain Cargo.toml files that are not this
// repo's packages and must not be reported.
var skipDirNames = map[string]bool{
	".git":          true,
	"target":        true,
	"node_modules":  true,
	"vendor":        true,
	".venv":         true,
	"_build":        true,
	"deps":          true,
	".build":        true,
	"dist-newstyle": true,
	".cargo":        true,
}

// exemptionKind distinguishes a permanent exclusion from a tracked backlog item.
type exemptionKind int

const (
	exemptionExcluded exemptionKind = iota
	exemptionPending
)

func (k exemptionKind) String() string {
	if k == exemptionPending {
		return "PENDING"
	}
	return "EXCLUDED"
}

// exemption is one parsed line of the ledger.
type exemption struct {
	kind   exemptionKind
	path   string // repo-relative, slash-separated
	reason string
	line   int
}

// ValidateNoOrphanCrates returns an error naming every crate directory that is
// not covered by a BUILD file and has no entry in the exemption ledger, plus
// every ledger entry that has gone stale.
//
// A missing ledger file is not an error on its own — a repo with no exemptions
// simply has none — but a missing ledger combined with an orphaned crate reports
// both, so the fix is obvious from one run.
func ValidateNoOrphanCrates(repoRoot string) error {
	exemptions, parseProblems, err := loadExemptions(repoRoot)
	if err != nil {
		return err
	}

	orphans, covered, err := scanCrates(repoRoot)
	if err != nil {
		return err
	}

	var problems []string
	problems = append(problems, parseProblems...)

	// 1. Orphans with no ledger entry — the check's primary job.
	listed := make(map[string]bool, len(exemptions))
	for _, e := range exemptions {
		listed[e.path] = true
	}

	var unlisted []string
	for _, o := range orphans {
		if !listed[o.path] {
			unlisted = append(unlisted, o.describe())
		}
	}
	sort.Strings(unlisted)
	problems = append(problems, unlisted...)

	// 2. Stale entries — the part that stops the ledger becoming a dumping
	//    ground. An entry whose crate is now covered, or whose directory is
	//    gone, must be deleted in the same change that resolved it.
	for _, e := range exemptions {
		switch {
		case covered[e.path]:
			problems = append(problems, fmt.Sprintf(
				"%s:%d: stale %s entry for %s — that crate is now covered by a BUILD file. "+
					"Delete this line; the exemption has done its job.",
				ExemptionsFile, e.line, e.kind, e.path))
		case !dirExists(filepath.Join(repoRoot, filepath.FromSlash(e.path))):
			problems = append(problems, fmt.Sprintf(
				"%s:%d: stale %s entry for %s — that directory does not exist. "+
					"Delete this line, or fix the path if the crate moved.",
				ExemptionsFile, e.line, e.kind, e.path))
		}
	}

	if len(problems) == 0 {
		return nil
	}
	return fmt.Errorf("orphan-crate validation failed:\n  - %s", strings.Join(problems, "\n  - "))
}

// PendingExemptionCount reports how many crates are on the backlog, so callers
// can print the number and watch it fall. Errors are reported as zero: this is a
// reporting helper, not a gate — ValidateNoOrphanCrates is the gate.
func PendingExemptionCount(repoRoot string) int {
	exemptions, _, err := loadExemptions(repoRoot)
	if err != nil {
		return 0
	}
	count := 0
	for _, e := range exemptions {
		if e.kind == exemptionPending {
			count++
		}
	}
	return count
}

// orphan is a crate directory with no covering BUILD file.
type orphan struct {
	path       string // repo-relative, slash-separated
	emptyBuild string // non-empty when a BUILD exists but has no runnable lines
}

func (o orphan) describe() string {
	if o.emptyBuild != "" {
		return fmt.Sprintf(
			"%s: has a Cargo.toml and a %s, but that BUILD contains no runnable commands — "+
				"discovery accepts it and the package then reports success having compiled, "+
				"tested and linted nothing. Add the real command (usually "+
				"`cargo test -p <crate> -- --nocapture`).",
			o.path, o.emptyBuild)
	}
	return fmt.Sprintf(
		"%s: has a Cargo.toml but no BUILD file in it or any parent directory, so the build "+
			"tool never discovers it — it is never built, tested or linted. Add a BUILD file "+
			"(usually the one-liner `cargo test -p <crate> -- --nocapture`), or, if it genuinely "+
			"should never be built, add a reasoned EXCLUDED entry to %s.",
		o.path, ExemptionsFile)
}

// loadExemptions parses the ledger. It returns the valid entries plus a list of
// human-readable problems for malformed lines, so one run reports every mistake
// rather than stopping at the first.
func loadExemptions(repoRoot string) ([]exemption, []string, error) {
	path := filepath.Join(repoRoot, filepath.FromSlash(ExemptionsFile))
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil, nil
		}
		return nil, nil, fmt.Errorf("reading %s: %w", ExemptionsFile, err)
	}

	var (
		entries  []exemption
		problems []string
		seen     = make(map[string]int)
	)

	for i, raw := range strings.Split(strings.ReplaceAll(string(data), "\r\n", "\n"), "\n") {
		lineNo := i + 1
		line := strings.TrimSpace(raw)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		// Format: <KIND> <path>  # <reason>
		body, reason, hasReason := strings.Cut(line, "#")
		reason = strings.TrimSpace(reason)
		fields := strings.Fields(body)
		if len(fields) != 2 {
			problems = append(problems, fmt.Sprintf(
				"%s:%d: expected `<EXCLUDED|PENDING> <path>  # <reason>`, got %q",
				ExemptionsFile, lineNo, line))
			continue
		}

		var kind exemptionKind
		switch fields[0] {
		case "EXCLUDED":
			kind = exemptionExcluded
		case "PENDING":
			kind = exemptionPending
		default:
			problems = append(problems, fmt.Sprintf(
				"%s:%d: unknown kind %q — must be EXCLUDED or PENDING",
				ExemptionsFile, lineNo, fields[0]))
			continue
		}

		// A reason is mandatory. An exemption without one is indistinguishable
		// from an oversight, which defeats the purpose of writing it down.
		if !hasReason || reason == "" {
			problems = append(problems, fmt.Sprintf(
				"%s:%d: %s entry for %s has no reason — every exemption must say why, "+
					"after a `#` on the same line",
				ExemptionsFile, lineNo, fields[0], fields[1]))
			continue
		}

		// Confine the path to the scanned tree BEFORE it is ever joined against
		// repoRoot and stat-ed. `filepath.Clean` does not strip leading `..`, so
		// an unchecked entry would resolve outside the repo — which both leaks a
		// directory-existence oracle through the build status and, because such
		// a path can never appear in the covered set, would make the entry
		// permanently un-stale. Both defeat the point of the file.
		entryPath, pathProblem := normalizeEntryPath(fields[1], lineNo)
		if pathProblem != "" {
			problems = append(problems, pathProblem)
			continue
		}

		if prev, dup := seen[entryPath]; dup {
			problems = append(problems, fmt.Sprintf(
				"%s:%d: duplicate entry for %s (first listed on line %d)",
				ExemptionsFile, lineNo, entryPath, prev))
			continue
		}
		seen[entryPath] = lineNo

		entries = append(entries, exemption{kind: kind, path: entryPath, reason: reason, line: lineNo})
	}

	return entries, problems, nil
}

// normalizeEntryPath validates that a ledger path is repo-relative and lexically
// inside the scanned tree, returning the cleaned path or a problem description.
func normalizeEntryPath(raw string, lineNo int) (string, string) {
	if filepath.IsAbs(raw) || strings.HasPrefix(raw, "/") || strings.HasPrefix(raw, `\`) {
		return "", fmt.Sprintf(
			"%s:%d: path %q must be repo-relative, not absolute",
			ExemptionsFile, lineNo, raw)
	}
	cleaned := filepath.ToSlash(filepath.Clean(raw))
	if cleaned == ".." || strings.HasPrefix(cleaned, "../") {
		return "", fmt.Sprintf(
			"%s:%d: path %q escapes the repository",
			ExemptionsFile, lineNo, raw)
	}
	if cleaned != scanRoot && !strings.HasPrefix(cleaned, scanRoot+"/") {
		return "", fmt.Sprintf(
			"%s:%d: path %s is not under %s/, which is the only tree scanned for crates",
			ExemptionsFile, lineNo, cleaned, scanRoot)
	}
	return cleaned, ""
}

// scanCrates walks the scan root and partitions every directory holding a
// Cargo.toml into covered and orphaned. A crate is covered when it or any
// ancestor (up to the scan root) holds a BUILD file with at least one runnable
// line. Paths are repo-relative and slash-separated so they match the ledger
// regardless of host OS.
func scanCrates(repoRoot string) (orphans []orphan, covered map[string]bool, err error) {
	covered = make(map[string]bool)
	absRoot := filepath.Join(repoRoot, filepath.FromSlash(scanRoot))

	if _, statErr := os.Stat(absRoot); statErr != nil {
		if os.IsNotExist(statErr) {
			return nil, covered, nil // A repo without this tree is not an error.
		}
		return nil, nil, fmt.Errorf("scanning %s: %w", scanRoot, statErr)
	}

	// buildCache memoizes "does this directory hold a runnable BUILD?" so the
	// ancestor walk stays linear in the number of directories rather than
	// re-stat-ing shared parents once per crate.
	buildCache := make(map[string]string)

	walkErr := filepath.WalkDir(absRoot, func(path string, d fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if !isDir(path, d) {
			return nil
		}
		if path != absRoot && skipDirNames[d.Name()] {
			return fs.SkipDir
		}

		hasCargo, statErr := regularFileExists(filepath.Join(path, "Cargo.toml"))
		if statErr != nil {
			return statErr
		}
		if !hasCargo {
			return nil
		}

		rel, relErr := relSlash(repoRoot, path)
		if relErr != nil {
			return relErr
		}

		buildName, coverErr := coveringBuild(repoRoot, absRoot, path, buildCache)
		if coverErr != nil {
			return coverErr
		}
		if buildName != "" {
			covered[rel] = true
			return nil
		}

		// Not covered. Distinguish "no BUILD anywhere" from "a BUILD exists here
		// but is empty", because the fix is different and the second is the
		// shape of a deliberate bypass.
		emptyName, emptyErr := emptyBuildIn(path)
		if emptyErr != nil {
			return emptyErr
		}
		orphans = append(orphans, orphan{path: rel, emptyBuild: emptyName})
		return nil
	})
	if walkErr != nil {
		return nil, nil, fmt.Errorf("scanning %s: %w", scanRoot, walkErr)
	}

	sort.Slice(orphans, func(i, j int) bool { return orphans[i].path < orphans[j].path })
	return orphans, covered, nil
}

// coveringBuild returns the name of the BUILD file covering dir — searching dir
// and then each ancestor up to and including root — or "" when there is none.
func coveringBuild(repoRoot, root, dir string, cache map[string]string) (string, error) {
	type pending struct{ dir string }
	var chain []pending

	current := dir
	for {
		if name, ok := cache[current]; ok {
			// Propagate the memoized answer back down the chain we walked.
			for _, p := range chain {
				cache[p.dir] = name
			}
			return name, nil
		}

		name, err := runnableBuildIn(current)
		if err != nil {
			return "", err
		}
		if name != "" {
			cache[current] = name
			for _, p := range chain {
				cache[p.dir] = name
			}
			return name, nil
		}

		chain = append(chain, pending{dir: current})
		if current == root {
			break
		}
		parent := filepath.Dir(current)
		if parent == current {
			break // Defensive: filesystem root reached.
		}
		current = parent
	}

	for _, p := range chain {
		cache[p.dir] = ""
	}
	return "", nil
}

// runnableBuildIn returns the name of the first BUILD file in dir that contains
// at least one runnable line, or "" if there is none.
func runnableBuildIn(dir string) (string, error) {
	for _, name := range buildFileNames {
		path := filepath.Join(dir, name)
		ok, err := regularFileExists(path)
		if err != nil {
			return "", err
		}
		if !ok {
			continue
		}
		runnable, err := hasRunnableLine(path)
		if err != nil {
			return "", err
		}
		if runnable {
			return name, nil
		}
	}
	return "", nil
}

// emptyBuildIn returns the name of a BUILD file present in dir that has no
// runnable lines, or "" if dir has no BUILD file at all.
func emptyBuildIn(dir string) (string, error) {
	for _, name := range buildFileNames {
		ok, err := regularFileExists(filepath.Join(dir, name))
		if err != nil {
			return "", err
		}
		if ok {
			return name, nil
		}
	}
	return "", nil
}

// hasRunnableLine reports whether a BUILD file contains at least one line that
// is neither blank nor a comment — mirroring how discovery reads it.
func hasRunnableLine(path string) (bool, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return false, fmt.Errorf("reading %s: %w", path, err)
	}
	for _, line := range strings.Split(strings.ReplaceAll(string(data), "\r\n", "\n"), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" && !strings.HasPrefix(trimmed, "#") {
			return true, nil
		}
	}
	return false, nil
}

// isDir reports whether path is a directory, following symlinks. WalkDir reports
// a symlink's own type (from lstat), so a symlinked directory would otherwise be
// skipped silently — a free bypass.
func isDir(path string, d fs.DirEntry) bool {
	if d.IsDir() {
		return true
	}
	if d.Type()&fs.ModeSymlink == 0 {
		return false
	}
	return dirExists(path)
}

// regularFileExists reports whether path is an existing non-directory. Unlike a
// bare stat-and-swallow, a stat error that is NOT "does not exist" (a permission
// problem, a path-length overflow, transient I/O) is returned rather than
// reported as absence — the difference between failing closed and failing open.
func regularFileExists(path string) (bool, error) {
	info, err := os.Stat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return false, nil
		}
		return false, fmt.Errorf("stat %s: %w", path, err)
	}
	return !info.IsDir(), nil
}

func dirExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.IsDir()
}

func relSlash(repoRoot, path string) (string, error) {
	rel, err := filepath.Rel(repoRoot, path)
	if err != nil {
		return "", fmt.Errorf("relativizing %s: %w", path, err)
	}
	return filepath.ToSlash(rel), nil
}
