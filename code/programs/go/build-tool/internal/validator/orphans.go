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
// created by hand. A check is the only durable fix: writing the 84 missing
// BUILD files fixes today's omissions, but only a gate makes the 85th
// impossible.
//
// # How it works
//
// Every directory containing a `Cargo.toml` must have either a `BUILD` file or
// an explicit, reasoned entry in `code/BUILD-EXEMPTIONS`. A brand-new crate is
// in neither state, so it fails CI the first time it is pushed — which is the
// whole point.
//
// The exemption file distinguishes two kinds of entry, because "we deliberately
// never build this" and "we have not got to this yet" are different claims and
// should not be spelled the same way:
//
//   - EXCLUDED — genuinely never gets a BUILD. A compile-only bridge crate with
//     nothing to run, a crate that only builds under a foreign toolchain, and so
//     on. The reason must say which.
//   - PENDING — a known gap with a real crate behind it. This is a backlog, and
//     it is meant to shrink.
//
// Both suppress the failure. Keeping them distinct means the PENDING list is a
// visible, countable debt rather than a drawer things get quietly filed into.
//
// # Why stale entries are also an error
//
// An allowlist that only ever grows is a rug to sweep things under. So this also
// fails when an entry names a path that has since gained a BUILD file, or that
// no longer exists. Landing a BUILD for a PENDING crate therefore *forces* the
// same PR to delete its exemption line, and the backlog cannot silently outlive
// the problem it describes.

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// ExemptionsFile is the repo-relative path of the exemption ledger.
const ExemptionsFile = "code/BUILD-EXEMPTIONS"

// crateRoots are the repo-relative directories scanned for orphaned crates.
// Each entry's immediate subdirectories are checked; the walk does not descend
// further, because that is the layout every Rust package and program in this
// repo uses, and descending would pick up `target/` build artifacts and vendored
// sources that are not packages at all.
var crateRoots = []string{
	"code/packages/rust",
	"code/programs/rust",
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

// ValidateNoOrphanCrates returns an error naming every directory that contains a
// Cargo.toml but no BUILD file and no entry in the exemption ledger, plus every
// ledger entry that has gone stale.
//
// A missing ledger file is not an error on its own — a repo with no exemptions
// simply has none — but a missing ledger combined with an orphaned crate reports
// both, so the fix is obvious from one run.
func ValidateNoOrphanCrates(repoRoot string) error {
	exemptions, parseProblems, err := loadExemptions(repoRoot)
	if err != nil {
		return err
	}

	orphans, withBuild, err := scanCrates(repoRoot)
	if err != nil {
		return err
	}

	var problems []string
	problems = append(problems, parseProblems...)

	// 1. Orphans with no ledger entry — the check's primary job.
	byPath := make(map[string]exemption, len(exemptions))
	for _, e := range exemptions {
		byPath[e.path] = e
	}

	var unlisted []string
	for _, path := range orphans {
		if _, ok := byPath[path]; !ok {
			unlisted = append(unlisted, path)
		}
	}
	sort.Strings(unlisted)
	for _, path := range unlisted {
		problems = append(problems, fmt.Sprintf(
			"%s: has a Cargo.toml but no BUILD file, so the build tool never discovers it — "+
				"it is never built, tested or linted. Add a BUILD file (usually the one-liner "+
				"`cargo test -p <crate> -- --nocapture`), or, if it genuinely should never be "+
				"built, add a reasoned EXCLUDED entry to %s.",
			path, ExemptionsFile))
	}

	// 2. Stale entries — the part that stops the ledger becoming a dumping
	//    ground. An entry whose crate now has a BUILD, or whose directory is
	//    gone, must be deleted in the same change that resolved it.
	stale := make([]exemption, 0, len(exemptions))
	for _, e := range exemptions {
		if withBuild[e.path] {
			stale = append(stale, e)
			continue
		}
		if !dirExists(filepath.Join(repoRoot, filepath.FromSlash(e.path))) {
			stale = append(stale, e)
		}
	}
	sort.Slice(stale, func(i, j int) bool { return stale[i].line < stale[j].line })
	for _, e := range stale {
		if withBuild[e.path] {
			problems = append(problems, fmt.Sprintf(
				"%s:%d: stale %s entry for %s — that crate now HAS a BUILD file. "+
					"Delete this line; the exemption has done its job.",
				ExemptionsFile, e.line, e.kind, e.path))
			continue
		}
		problems = append(problems, fmt.Sprintf(
			"%s:%d: stale %s entry for %s — that directory does not exist. "+
				"Delete this line, or fix the path if the crate moved.",
			ExemptionsFile, e.line, e.kind, e.path))
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

		entryPath := filepath.ToSlash(filepath.Clean(fields[1]))
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

// scanCrates walks the crate roots and partitions every directory holding a
// Cargo.toml into "has a BUILD" and "does not". Paths are repo-relative and
// slash-separated so they match the ledger regardless of host OS.
func scanCrates(repoRoot string) (orphans []string, withBuild map[string]bool, err error) {
	withBuild = make(map[string]bool)

	for _, root := range crateRoots {
		absRoot := filepath.Join(repoRoot, filepath.FromSlash(root))
		entries, readErr := os.ReadDir(absRoot)
		if readErr != nil {
			if os.IsNotExist(readErr) {
				continue // A repo layout without this root is not an error.
			}
			return nil, nil, fmt.Errorf("scanning %s: %w", root, readErr)
		}

		for _, entry := range entries {
			if !entry.IsDir() {
				continue
			}
			dir := filepath.Join(absRoot, entry.Name())
			if !fileExists(filepath.Join(dir, "Cargo.toml")) {
				continue
			}
			relPath := root + "/" + entry.Name()
			if fileExists(filepath.Join(dir, "BUILD")) {
				withBuild[relPath] = true
				continue
			}
			orphans = append(orphans, relPath)
		}
	}

	sort.Strings(orphans)
	return orphans, withBuild, nil
}

func fileExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

func dirExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.IsDir()
}
