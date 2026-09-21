// Package cigates decides which GitHub Actions jobs a change actually needs.
//
// # The problem this solves
//
// CI used to start eleven "always-on" conformance jobs on every pull request.
// None of them declared `needs:`, so GitHub scheduled them *ahead of* the
// `detect` job — the one job whose entire purpose is to work out what needs to
// run. The planner was starved by the jobs it should have been gating.
//
// Measured over 35 successful pull-request runs, roughly 70% of a run's
// wall-clock was spent *queueing for a runner*, not computing. Each run
// demanded 16 concurrent slots from a saturated account-wide ceiling, so every
// unnecessary job lengthened the queue for every other run in flight. The
// cheapest speedup available was therefore not to make jobs faster, but to stop
// starting the ones a change does not need — running a Ruby compiled-grammar
// regeneration on a human-languages curriculum PR buys nothing and costs
// everyone.
//
// # How it works
//
// A checked-in registry (code/specs/data/ci-gates.json) names each gated job and
// declares two things about it: the packages it exercises, and the non-package
// file paths it reads. During the planning pass the build tool intersects those
// declarations with the change under test and emits one boolean per job, which
// ci.yml consumes as a job-level `if:` condition.
//
// # Why gates need BOTH packages and paths
//
// This is the part that is easy to get wrong.
//
// gitdiff.MapFilesToPackages maps a changed file to a package only when the file
// lives *under* that package's directory, and the build tool declares no shared
// prefixes. So a change under code/specs, code/fixtures, code/grammars, or
// code/scripts maps to ZERO packages and never reaches affected_packages.
//
// Nearly every gated job is a staleness check whose input lives in exactly those
// trees — the D18F manifest under code/fixtures, the .tokens/.grammar sources
// under code/grammars, the PHY00/PHY01 Dart fixtures under code/specs/fixtures
// (which the Dart tests import by relative path *out of* their own package
// directory). A package-only gate would skip the D18F job on a pull request that
// changed only the D18F manifest — which is precisely the drift that job exists
// to catch. The path clause is load-bearing.
//
// # Fail open, always
//
// Every ambiguity resolves to "run it". A false positive wastes one job; a false
// negative lets a regression through. A malformed registry is a hard error at
// plan time rather than a quiet all-false, because — as lessons.md puts it —
// "when a gate is derived by pattern-matching a build script, the failure mode
// is silence: a package that matches nothing is indistinguishable from a package
// that passed."
package cigates

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
	"golang.org/x/text/unicode/norm"
)

// CurrentSchemaVersion is the registry format this implementation understands.
// A registry stamped higher is rejected rather than half-read.
const CurrentSchemaVersion = 1

const (
	maxRegistryGates = 128
	maxGateEntries   = 4096
	maxGateIDRunes   = 80
	maxGateTextRunes = 240
	maxGlobRunes     = 512
)

// MaxMatchWork is the operation-wide Unicode-scalar path-selection budget.
const MaxMatchWork uint64 = 50_000_000

// ErrMatchWorkLimitExceeded is returned before any matcher call when the
// bounded literal-segment prefilter plus every candidate glob match exceeds
// MaxMatchWork.
var ErrMatchWorkLimitExceeded = errors.New("CI_GATE_MATCH_LIMIT_EXCEEDED")

// DefaultRegistryPath is the registry's home, relative to the repo root.
const DefaultRegistryPath = "code/specs/data/ci-gates.json"

// CIWorkflowPath and RegistryPath are self-test sentinels: when either changes,
// every gate fires. Editing the gating machinery should run everything the
// machinery gates, so a mistake in it shows up on the pull request that
// introduces it rather than three merges later. This mirrors the workflow_changed()
// escape hatch the six code/scripts/*_ci_acceptance.py scripts already use.
const (
	CIWorkflowPath = ".github/workflows/ci.yml"
	RegistryPath   = DefaultRegistryPath
)

// machineryPrefixes extends those sentinels to the code that computes the
// verdicts. CI compiles the build tool from the pull request's own source, so a
// change to this package — or to the glob matcher it delegates to, or to the
// main.go that wires them up — changes the evaluator being trusted to decide
// what runs.
//
// Without this, one pull request could make Evaluate return false for
// everything: all gated jobs would skip, ci-gate would treat "skipped" as
// passing, and the run would be green. The gate that would otherwise notice a
// build-tool change is itself computed by the modified evaluator, so that check
// is circular and cannot be relied on.
// internal/gitdiff earns its place for a stronger reason than the others:
// GetChangedFiles *produces* changedFiles, which is at once the only input to
// the path clause, the only input to touchesGatingMachinery itself, and
// upstream of the affected closure. A GetChangedFiles that returned, say,
// ["README.md"] would yield an empty-but-non-nil affected set and a non-nil
// file list, so none of the run-everything escapes trigger and every gate
// evaluates false — a complete all-false ci_jobs map that satisfies the
// workflow's guard while every gated job skips and ci-gate reports green.
//
// internal/discovery and internal/resolver are deliberately NOT here. They feed
// only the package clause, and after every job gate gained a directory-glob
// path clause, a corrupted dependency graph can no longer silence a gate whose
// files were actually touched. Keeping them out stops the sentinel from
// swallowing the whole build tool. TestUnrelatedBuildToolChangeIsNotASentinel
// pins that boundary.
var machineryPrefixes = []string{
	"code/programs/go/build-tool/internal/cigates/",
	"code/programs/go/build-tool/internal/globmatch/",
	"code/programs/go/build-tool/internal/gitdiff/",
	"code/programs/go/build-tool/main.go",
}

// Scope distinguishes a gate that controls a whole workflow job from one that
// controls a single step inside a job. Only job-scoped gates are required to
// correspond to a job key in ci.yml.
const (
	ScopeJob  = "job"
	ScopeStep = "step"
)

// Gate is one job's (or step's) declaration of what makes it necessary.
//
// Packages are qualified package names in the build tool's own convention —
// "<lang>/<name>" for code/packages, "<lang>/programs/<name>" for code/programs
// (see discovery.inferPackageName). They are matched against the *affected
// closure*, which already includes transitive dependents, so a gate only needs
// to list the packages it directly exercises; the dependency graph supplies the
// rest.
//
// Paths are repo-root-relative globs matched with internal/globmatch, which
// understands ** as "zero or more complete path segments".
type Gate struct {
	Scope       string   `json:"scope"`
	Description string   `json:"description"`
	Packages    []string `json:"packages"`
	Paths       []string `json:"paths"`
}

// EffectiveScope defaults an unset scope to "job", which is the common case.
func (g Gate) EffectiveScope() string {
	if g.Scope == "" {
		return ScopeJob
	}
	return g.Scope
}

// Registry is the deserialized ci-gates.json document.
type Registry struct {
	SchemaVersion int             `json:"schema_version"`
	Gates         map[string]Gate `json:"gates"`
}

// Load reads and validates the registry at path.
//
// Validation is strict on purpose. A gate with neither packages nor paths can
// never fire, which would make it silently dead — the exact failure mode this
// package exists to prevent — so it is rejected rather than tolerated.
func Load(path string) (*Registry, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read CI gate registry %s: %w", path, err)
	}

	var reg Registry
	if err := json.Unmarshal(data, &reg); err != nil {
		return nil, fmt.Errorf("parse CI gate registry %s: %w", path, err)
	}

	if reg.SchemaVersion < 1 {
		return nil, fmt.Errorf("%s: schema_version must be at least 1, got %d", path, reg.SchemaVersion)
	}
	if reg.SchemaVersion > CurrentSchemaVersion {
		return nil, fmt.Errorf(
			"%s: schema_version %d is newer than this build tool understands (%d)",
			path, reg.SchemaVersion, CurrentSchemaVersion,
		)
	}
	if err := validateRegistry(&reg); err != nil {
		return nil, fmt.Errorf("%s: %w", path, err)
	}

	return &reg, nil
}

// validateGateID rejects ids that cannot round-trip into a GitHub Actions output
// name. Actions output names allow letters, digits, underscores and hyphens; we
// additionally require lowercase so OutputName is unambiguous.
func validateGateID(id string) error {
	if id == "" {
		return fmt.Errorf("gate id must not be empty")
	}
	if utf8.RuneCountInString(id) > maxGateIDRunes {
		return fmt.Errorf("gate id exceeds %d Unicode scalars", maxGateIDRunes)
	}
	for _, r := range id {
		switch {
		case r >= 'a' && r <= 'z':
		case r >= '0' && r <= '9':
		case r == '-' || r == '_':
		default:
			return fmt.Errorf("gate id %q contains %q; use lowercase letters, digits, '-' and '_' only", id, r)
		}
	}
	return nil
}

func validateRegistry(reg *Registry) error {
	if reg == nil {
		return fmt.Errorf("registry is nil")
	}
	if reg.SchemaVersion < 1 || reg.SchemaVersion > CurrentSchemaVersion {
		return fmt.Errorf("unsupported schema_version %d", reg.SchemaVersion)
	}
	if len(reg.Gates) == 0 {
		return fmt.Errorf("registry declares no gates")
	}
	if len(reg.Gates) > maxRegistryGates {
		return fmt.Errorf("registry declares %d gates; maximum is %d", len(reg.Gates), maxRegistryGates)
	}

	outputOwners := make(map[string]string, len(reg.Gates))
	for id, gate := range reg.Gates {
		if err := validateGateID(id); err != nil {
			return err
		}
		outputName := OutputName(id)
		if owner, exists := outputOwners[outputName]; exists {
			return fmt.Errorf("gates %q and %q produce the same output name %q", owner, id, outputName)
		}
		outputOwners[outputName] = id
		switch gate.EffectiveScope() {
		case ScopeJob, ScopeStep:
		default:
			return fmt.Errorf("gate %q has unknown scope %q (want %q or %q)", id, gate.Scope, ScopeJob, ScopeStep)
		}
		if len(gate.Packages) == 0 && len(gate.Paths) == 0 {
			return fmt.Errorf("gate %q declares neither packages nor paths, so it could never fire", id)
		}
		if strings.TrimSpace(gate.Description) == "" {
			return fmt.Errorf("gate %q has no description", id)
		}
		if utf8.RuneCountInString(gate.Description) > maxGateTextRunes {
			return fmt.Errorf("gate %q description exceeds %d Unicode scalars", id, maxGateTextRunes)
		}
		if len(gate.Packages) > maxGateEntries || len(gate.Paths) > maxGateEntries {
			return fmt.Errorf("gate %q exceeds the %d-entry package or path limit", id, maxGateEntries)
		}
		seenPackages := make(map[string]bool, len(gate.Packages))
		for _, pkg := range gate.Packages {
			if seenPackages[pkg] {
				return fmt.Errorf("gate %q repeats package %q", id, pkg)
			}
			seenPackages[pkg] = true
			if !validPackageName(pkg) {
				return fmt.Errorf("gate %q has invalid package name %q", id, pkg)
			}
		}
		seenPaths := make(map[string]bool, len(gate.Paths))
		for _, pattern := range gate.Paths {
			if seenPaths[pattern] {
				return fmt.Errorf("gate %q repeats path glob %q", id, pattern)
			}
			seenPaths[pattern] = true
			if err := validatePortableGlob(pattern); err != nil {
				return fmt.Errorf("gate %q has invalid path glob: %w", id, err)
			}
		}
	}
	return nil
}

func validPackageName(value string) bool {
	if value == "" || utf8.RuneCountInString(value) > maxGateTextRunes {
		return false
	}
	parts := strings.Split(value, "/")
	if len(parts) < 2 {
		return false
	}
	for _, part := range parts {
		if part == "" || !asciiLowerOrDigit(part[0]) {
			return false
		}
		for index := 1; index < len(part); index++ {
			character := part[index]
			if !asciiLowerOrDigit(character) && character != '.' && character != '_' && character != '-' {
				return false
			}
		}
	}
	return true
}

func asciiLowerOrDigit(character byte) bool {
	return character >= 'a' && character <= 'z' || character >= '0' && character <= '9'
}

func validatePortableGlob(pattern string) error {
	if pattern == "" || !utf8.ValidString(pattern) || utf8.RuneCountInString(pattern) > maxGlobRunes {
		return fmt.Errorf("glob must contain 1 to %d valid Unicode scalars", maxGlobRunes)
	}
	if !norm.NFC.IsNormalString(pattern) {
		return fmt.Errorf("glob is not NFC-normalized")
	}
	if strings.HasPrefix(pattern, "/") || strings.Contains(pattern, "\\") || strings.Contains(pattern, "//") ||
		(len(pattern) >= 2 && ((pattern[0] >= 'A' && pattern[0] <= 'Z') || (pattern[0] >= 'a' && pattern[0] <= 'z')) && pattern[1] == ':') {
		return fmt.Errorf("glob is not a portable relative path")
	}
	for _, character := range pattern {
		if character < 0x20 || strings.ContainsRune("<>:\"|?", character) {
			return fmt.Errorf("glob contains an unsafe character")
		}
	}
	for _, segment := range strings.Split(pattern, "/") {
		if segment == "" || segment == "." || segment == ".." || strings.HasSuffix(segment, ".") || strings.HasSuffix(segment, " ") {
			return fmt.Errorf("glob contains an unsafe path segment")
		}
		if ambiguousCharacterClass(segment) {
			return fmt.Errorf("glob character class has an ambiguous or descending range")
		}
		if !strings.ContainsAny(segment, "*[]{}") && windowsReservedBasename(segment) {
			return fmt.Errorf("glob uses a Windows reserved basename")
		}
	}
	return nil
}

func ambiguousCharacterClass(segment string) bool {
	characters := []rune(segment)
	for index := 0; index < len(characters); {
		if characters[index] != '[' {
			index++
			continue
		}
		cursor := index + 1
		if cursor < len(characters) && characters[cursor] == '!' {
			cursor++
		}
		closing := cursor
		if closing < len(characters) && characters[closing] == ']' {
			closing++
		}
		for closing < len(characters) && characters[closing] != ']' {
			closing++
		}
		if closing == len(characters) {
			return false
		}
		body := string(characters[cursor:closing])
		if strings.Contains(body, "--") || strings.Contains(body, "&&") || strings.Contains(body, "~~") || strings.Contains(body, "||") {
			return true
		}
		member := cursor
		for member+2 < closing {
			if characters[member+1] == '-' && characters[member] > characters[member+2] {
				return true
			}
			if characters[member+1] == '-' {
				member += 3
			} else {
				member++
			}
		}
		index = closing + 1
	}
	return false
}

func windowsReservedBasename(segment string) bool {
	base := strings.ToUpper(strings.SplitN(segment, ".", 2)[0])
	if base == "CON" || base == "PRN" || base == "AUX" || base == "NUL" ||
		base == "CONIN$" || base == "CONOUT$" || base == "CLOCK$" {
		return true
	}
	if len(base) == 4 && (strings.HasPrefix(base, "COM") || strings.HasPrefix(base, "LPT")) && base[3] >= '1' && base[3] <= '9' {
		return true
	}
	return base == "COM¹" || base == "COM²" || base == "COM³" ||
		base == "LPT¹" || base == "LPT²" || base == "LPT³"
}

// OutputName converts a gate id into the GitHub Actions step-output name that
// carries its verdict, e.g. "d18f-message-conformance" → "run_d18f_message_conformance".
//
// Hyphens become underscores because Actions output names cannot contain them.
// The "run_" prefix keeps these distinct from the existing "needs_<lang>"
// toolchain flags, which validator.validateCIFullBuildToolchains asserts on
// separately — colliding with that namespace would make one gate's flag look
// like a toolchain flag to the validator.
func OutputName(gateID string) string {
	return "run_" + strings.ReplaceAll(gateID, "-", "_")
}

// SortedGateIDs returns the registry's gate ids in a stable order, so emitted
// output and log lines are deterministic across runs.
func SortedGateIDs(reg *Registry) []string {
	if reg == nil {
		return nil
	}
	ids := make([]string, 0, len(reg.Gates))
	for id := range reg.Gates {
		ids = append(ids, id)
	}
	sort.Strings(ids)
	return ids
}

// Evaluate decides, for every gate in the registry, whether its job must run.
//
// affected is the affected-package closure. A nil map means "rebuild everything"
// (force mode, or git diff unavailable) and is NOT the same as an empty map,
// which means "nothing changed" — the same nil-versus-empty convention
// plan.BuildPlan.AffectedPackages uses.
//
// changedFiles is the raw `git diff --name-only` list, repo-root-relative with
// forward slashes. A nil list means change detection could not run, and is
// treated as "run everything" for the same reason.
//
// The result maps every gate id to its verdict; ids absent from the registry are
// absent from the result.
//
// There is deliberately no separate "is main branch" parameter. Main-branch
// pushes reach this function with force=true, because ci.yml passes -force on
// main — so the main-merge safety net is the force escape, not a second code
// path that would be dead everywhere else.
func Evaluate(
	reg *Registry,
	affected map[string]bool,
	changedFiles []string,
	force bool,
) (map[string]bool, error) {
	return evaluateWithMatcher(reg, affected, changedFiles, force, globmatch.MatchPath)
}

func evaluateWithMatcher(
	reg *Registry,
	affected map[string]bool,
	changedFiles []string,
	force bool,
	matcher func(string, string) bool,
) (map[string]bool, error) {
	if err := validateRegistry(reg); err != nil {
		return nil, err
	}
	result := make(map[string]bool, len(reg.Gates))

	// Four reasons to run absolutely everything, checked before any per-gate
	// work. Each one means we cannot trust a narrower answer:
	//   force             — the caller asked for a full rebuild. On main, ci.yml
	//     always does, which is what makes gating pull requests safe: anything a
	//     PR gate gets wrong is still caught on merge.
	//   nil affected      — git diff failed; we have no idea what changed.
	//   nil changedFiles  — same, from the raw-path side.
	//   ci.yml / registry — a change to the gating machinery self-tests.
	runEverything := force ||
		affected == nil ||
		changedFiles == nil ||
		touchesGatingMachinery(changedFiles)

	if runEverything {
		for id := range reg.Gates {
			result[id] = true
		}
		return result, nil
	}

	selection, err := preflightPathSelection(reg, changedFiles)
	if err != nil {
		return nil, err
	}
	patternMatches := make(map[string]bool, len(selection.patterns))
	patternEvaluated := make(map[string]bool, len(selection.patterns))
	matchPattern := func(pattern string) bool {
		if patternEvaluated[pattern] {
			return patternMatches[pattern]
		}
		patternEvaluated[pattern] = true
		patternIndex := selection.patternIndexes[pattern]
		for fileIndex, file := range changedFiles {
			if !selection.candidate(patternIndex, fileIndex) {
				continue
			}
			if matcher(pattern, file) {
				patternMatches[pattern] = true
				return true
			}
		}
		return false
	}

	for _, id := range SortedGateIDs(reg) {
		result[id] = gateFires(reg.Gates[id], affected, matchPattern)
	}
	return result, nil
}

type literalSegmentBounds struct {
	prefix    []string
	suffix    []string
	exact     bool
	workUnits uint64
}

type pathSelectionPreflight struct {
	patterns       []string
	patternIndexes map[string]int
	fileCount      int
	candidates     []uint64
}

func (selection *pathSelectionPreflight) candidate(patternIndex, fileIndex int) bool {
	bit := patternIndex*selection.fileCount + fileIndex
	return selection.candidates[bit/64]&(uint64(1)<<uint(bit%64)) != 0
}

func preflightPathSelection(reg *Registry, changedFiles []string) (*pathSelectionPreflight, error) {
	patternSet := make(map[string]bool)
	for _, gate := range reg.Gates {
		for _, pattern := range gate.Paths {
			patternSet[pattern] = true
		}
	}
	patterns := make([]string, 0, len(patternSet))
	for pattern := range patternSet {
		patterns = append(patterns, pattern)
	}
	sort.Strings(patterns)

	// Every pair costs at least two units: either one non-empty literal segment
	// plus its separator unit, or a candidate matcher grid whose factors are at
	// least two and one. Reject before allocating the compact candidate bitset.
	if len(patterns) != 0 && uint64(len(changedFiles)) > (MaxMatchWork/2)/uint64(len(patterns)) {
		return nil, ErrMatchWorkLimitExceeded
	}
	pairCount := len(patterns) * len(changedFiles)
	selection := &pathSelectionPreflight{
		patterns:       patterns,
		patternIndexes: make(map[string]int, len(patterns)),
		fileCount:      len(changedFiles),
		candidates:     make([]uint64, (pairCount+63)/64),
	}
	for index, pattern := range patterns {
		selection.patternIndexes[pattern] = index
	}

	fileSegments := make([][]string, len(changedFiles))
	fileFactors := make([]uint64, len(changedFiles))
	for index, file := range changedFiles {
		fileSegments[index] = splitPathSegments(file)
		fileFactors[index] = uint64(utf8.RuneCountInString(file)) + 1
	}

	remaining := MaxMatchWork
	for patternIndex, pattern := range patterns {
		bounds := makeLiteralSegmentBounds(pattern)
		patternFactor := uint64(utf8.RuneCountInString(pattern)) + 1
		for fileIndex := range changedFiles {
			if bounds.workUnits > remaining {
				return nil, ErrMatchWorkLimitExceeded
			}
			remaining -= bounds.workUnits
			if !bounds.couldMatch(fileSegments[fileIndex]) {
				continue
			}
			pathFactor := fileFactors[fileIndex]
			if pathFactor > remaining/patternFactor {
				return nil, ErrMatchWorkLimitExceeded
			}
			remaining -= patternFactor * pathFactor
			bit := patternIndex*len(changedFiles) + fileIndex
			selection.candidates[bit/64] |= uint64(1) << uint(bit%64)
		}
	}
	return selection, nil
}

func makeLiteralSegmentBounds(pattern string) literalSegmentBounds {
	segments := splitPathSegments(pattern)
	firstMeta := len(segments)
	lastMeta := -1
	for index, segment := range segments {
		if strings.ContainsAny(segment, "*?[]") {
			if firstMeta == len(segments) {
				firstMeta = index
			}
			lastMeta = index
		}
	}
	bounds := literalSegmentBounds{exact: lastMeta == -1}
	if bounds.exact {
		bounds.prefix = segments
	} else {
		bounds.prefix = segments[:firstMeta]
		bounds.suffix = segments[lastMeta+1:]
	}
	for _, segment := range bounds.prefix {
		bounds.workUnits += uint64(utf8.RuneCountInString(segment)) + 1
	}
	for _, segment := range bounds.suffix {
		bounds.workUnits += uint64(utf8.RuneCountInString(segment)) + 1
	}
	return bounds
}

func (bounds literalSegmentBounds) couldMatch(path []string) bool {
	if bounds.exact {
		return len(path) == len(bounds.prefix) && equalSegments(bounds.prefix, path)
	}
	if len(path) < len(bounds.prefix)+len(bounds.suffix) ||
		!equalSegments(bounds.prefix, path[:len(bounds.prefix)]) {
		return false
	}
	return equalSegments(bounds.suffix, path[len(path)-len(bounds.suffix):])
}

func equalSegments(left, right []string) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if left[index] != right[index] {
			return false
		}
	}
	return true
}

func splitPathSegments(path string) []string {
	parts := strings.Split(strings.TrimRight(path, "/"), "/")
	segments := make([]string, 0, len(parts))
	for _, part := range parts {
		if part != "" {
			segments = append(segments, part)
		}
	}
	return segments
}

// touchesGatingMachinery reports whether the change edits ci.yml, the registry,
// or the code that evaluates it. Any of those invalidates our ability to reason
// about the rest.
func touchesGatingMachinery(changedFiles []string) bool {
	for _, file := range changedFiles {
		if file == CIWorkflowPath || file == RegistryPath {
			return true
		}
		for _, prefix := range machineryPrefixes {
			if file == prefix || strings.HasPrefix(file, prefix) {
				return true
			}
		}
	}
	return false
}

// gateFires is the per-gate decision: does this change touch anything this job
// depends on?
func gateFires(
	gate Gate,
	affected map[string]bool,
	matchPattern func(string) bool,
) bool {
	// Package clause: any declared package inside the affected closure.
	for _, pkg := range gate.Packages {
		if affected[pkg] {
			return true
		}
	}

	// Path clause: any changed file matching a declared glob. This is the half
	// that catches fixture, grammar, spec and script edits, which never appear
	// in the affected closure at all (see the package doc comment).
	for _, pattern := range gate.Paths {
		if matchPattern(pattern) {
			return true
		}
	}

	return false
}
