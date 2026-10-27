package resolver

// Cargo dev-dependencies as test triggers
// =======================================
//
// The dependency graph built by ResolveDependencies answers one question:
// "what must be built before what?" For Rust that comes from the
// `[dependencies]` table of each Cargo.toml. Cargo has a second table that
// the graph deliberately leaves out:
//
//	[dev-dependencies]
//	mosmodel-compiler  = { path = "../../rust/mosmodel-compiler" }
//	moslayout-compiler = { path = "../../rust/moslayout-compiler" }
//
// A dev-dependency is linked into a crate's tests, examples and benches,
// never into its library. That makes it invisible to build ordering — and,
// before this file existed, invisible to change detection too, which is the
// expensive half. Every Mosaic package and app (code/packages/mosaic/*,
// code/programs/mosaic/*) is tested by a small Rust smoke harness whose ONLY
// in-repository dependencies are dev-dependencies on the Mosaic compilers and
// the artifact builder, and through those on every `mosaic-emit-*` backend.
// So a pull request that changed mosaic-emit-qt re-ran the emitter's own
// tests and none of the thirty Mosaic package and app suites that compile
// real `.mll` through it. CI was green on exactly the regressions those
// suites exist to catch.
//
// # Why a separate relation, not more graph edges
//
// The obvious fix is to add a dev-dependency edge D → P to the graph. That is
// wrong for three reasons, the last of which is fatal:
//
//  1. Over-selection. A graph edge makes P a *dependent* of D, so every
//     dependent of P is selected too. But nothing that links P's library has
//     changed — only P's tests have something new to run against. The
//     repository-input consumers in main's repository_inputs.go make the same
//     argument for the same reason: select a test the way a test is selected,
//     not the way a library is.
//  2. Contract. The cross-implementation conformance fixture
//     resolution/rust-field-aware pins the build graph's edges and says
//     `[dev-dependencies]` contribute none. The graph stays what that
//     contract describes.
//  3. Cycles. Cargo allows a dev-dependency cycle: B depends on A for real,
//     and A dev-depends on B to test the two together. As graph edges that is
//     A → B → A, IndependentGroups() reports a cycle, and the executor marks
//     EVERY package in the repository failed. One innocent test harness
//     would take down the whole build.
//
// So dev-dependencies form their own relation, TestTriggers:
//
//	TestTriggers[D] = the packages whose Cargo.toml dev-depends on D
//
// and change detection applies it exactly ONE hop, after the normal closure:
//
//	affected = AffectedNodes(changed)         — build closure over the graph
//	         ∪ TestTriggers[a] for every a in that closure
//	         + prerequisites of all of the above
//
// One hop is not a shortcut, it is the whole semantics. P is re-tested when
// something P's tests link has changed — D itself, or anything D depends on,
// which is precisely "D is in the build closure". P's own dependents are not
// added, because P's library is unchanged; and the triggers are never chased
// transitively, so a cycle through a dev-dependency simply cannot loop.
// Prerequisites added afterwards (expandAffectedSetWithPrereqs) are scheduled
// only so a fresh runner can build, not because they changed, so they are
// added AFTER the hop and never trigger anyone.
//
// # Parsing — the same approach as [dependencies], but failing closed
//
// Entries resolve exactly the way normal Rust dependencies do: the key (or a
// `package = "..."` rename) is looked up in the same per-language name table,
// and only entries with a quoted `path` are candidates — a crates.io
// dependency such as `tempfile = "3"` is never an in-repository edge.
//
// Because a missed trigger silently skips a dependent's tests, the reader
// FAILS CLOSED inside a dev-dependency table, the same way mosaic.go does for
// Mosaic manifests. Every entry must be one line of the form
//
//	name = "1.0"                            # a version string
//	name = { path = "...", features = [] }  # a one-line inline table
//
// and anything else stops the plan with a *CargoManifestError:
//
//	[dev-dependencies.name]                 # a sub-table header
//	name.path = "../name"                   # a dotted key
//	"name" = { path = "../name" }           # a quoted key
//	name = { path = "../name",              # an inline table (or array)
//	         features = ["x"] }             #   spanning lines
//	name = { workspace = true }             # path inherited from a workspace
//	name = true                             # any other value shape
//	[dev_dependencies]                      # the legacy underscore spelling
//	dev-dependencies = { ... }              # a root-level or dotted key that
//	package.dev-dependencies.name = "..."   #   spells the table another way
//
// None of these appears in the repository today (every one of the ~900
// dev-dependency lines is the plain form), so the check costs nothing now and
// turns a future silent miss into a loud, fixable error.
//
// Target-specific tables (`[target.'cfg(unix)'.dev-dependencies]`) are left
// out, matching parseRustDeps, which also reads only the top-level
// `[dependencies]` table.

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// TestTriggers maps a package to the packages that dev-depend on it, sorted
// and unique. A change that puts the key in the build closure must re-run the
// tests of every package listed — and nothing further (see the file comment).
type TestTriggers map[string][]string

// Dependents returns the packages that dev-depend on name, or nil.
func (triggers TestTriggers) Dependents(name string) []string {
	if triggers == nil {
		return nil
	}
	return triggers[name]
}

// CargoManifestError reports a Cargo.toml whose dev-dependency tables the
// resolver refuses to guess about. Returning it (rather than resolving no
// triggers) keeps change detection from failing open.
type CargoManifestError struct {
	Path   string
	Reason string
}

func (err *CargoManifestError) Error() string {
	return fmt.Sprintf(
		"%s: %s; write each dev-dependency as one line of a plain [dev-dependencies] table, "+
			"e.g. name = { path = \"../name\" }",
		err.Path, err.Reason,
	)
}

// devDependenciesTable is the one dev-dependency header the reader models.
const devDependenciesTable = "[dev-dependencies]"

// usesCargoDevDependencies reports which discovered languages carry a Cargo
// smoke harness or crate whose dev-dependencies are read. Rust and WASM are
// Cargo crates; a Mosaic package is tested by a Cargo harness beside its
// manifest (see mosaic.go).
func usesCargoDevDependencies(language string) bool {
	switch language {
	case "rust", "wasm", "mosaic":
		return true
	default:
		return false
	}
}

// cargoNameTableLanguage returns the language whose name table resolves a
// package's Cargo dependencies. Rust and WASM keep the table their normal
// [dependencies] already use. A Mosaic harness's Cargo dependencies are
// ordinary Rust crates (the compilers, the artifact builder), so it resolves
// through the Rust table; the Mosaic table holds only Mosaic packages.
func cargoNameTableLanguage(language string) string {
	if language == "mosaic" {
		return "rust"
	}
	return language
}

// parseRustDevDeps returns the repository packages that pkg's Cargo.toml
// lists in its top-level [dev-dependencies] table, in file order (duplicates
// allowed; the caller de-duplicates). A package without a Cargo.toml has no
// dev-dependencies. Any shape listed in the file comment is an error.
func parseRustDevDeps(pkg discovery.Package, knownNames map[string]string) ([]string, error) {
	cargoToml := filepath.Join(pkg.Path, "Cargo.toml")
	data, err := os.ReadFile(cargoToml)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, &CargoManifestError{Path: cargoToml, Reason: err.Error()}
	}
	refuse := func(reason string) ([]string, error) {
		return nil, &CargoManifestError{Path: cargoToml, Reason: reason}
	}

	var deps []string
	inDevDeps := false
	depth := 0 // value brackets left open by earlier lines (outside the table)
	for _, rawLine := range strings.Split(string(data), "\n") {
		line := strings.TrimSpace(stripTomlComment(rawLine))
		if line == "" {
			continue
		}

		if depth > 0 {
			// Still inside a multi-line array or inline table that opened on
			// an earlier line of some other table: a `[` here is an element,
			// not a header.
			depth += tomlBracketDelta(line)
			if depth < 0 {
				depth = 0
			}
			continue
		}

		if strings.HasPrefix(line, "[") {
			header := strings.Join(strings.Fields(line), "")
			inDevDeps = header == devDependenciesTable
			switch {
			case strings.HasPrefix(header, "[dev-dependencies.") ||
				strings.HasPrefix(header, "[dev_dependencies"):
				return refuse(fmt.Sprintf("uses the table header %q", line))
			}
			continue
		}

		eq := tomlKeyValueSplit(line)
		if eq < 0 {
			if inDevDeps {
				return refuse(fmt.Sprintf("has the line %q in [dev-dependencies], which is not a key = value entry", line))
			}
			continue
		}
		key := strings.TrimSpace(line[:eq])
		value := strings.TrimSpace(line[eq+1:])

		if !inDevDeps {
			// Outside the table, only track multi-line values and refuse the
			// spellings that would assign dev-dependencies from elsewhere.
			if spellsDevDependencies(key) {
				return refuse(fmt.Sprintf("assigns dev-dependencies through the key %q", key))
			}
			depth = tomlBracketDelta(value)
			if depth < 0 {
				depth = 0
			}
			continue
		}

		if !isBareTomlKey(key) {
			return refuse(fmt.Sprintf("uses the key %q in [dev-dependencies]; write it as a bare key", key))
		}
		switch {
		case tomlBasicStringValue(value) != "" || value == `""` || value == "''":
			// A registry version requirement: not a repository package.
			continue
		case strings.HasPrefix(value, "{"):
			if tomlBracketDelta(value) != 0 || !strings.HasSuffix(value, "}") {
				return refuse(fmt.Sprintf("gives [dev-dependencies] %s an inline table that is not on one line", key))
			}
		default:
			return refuse(fmt.Sprintf("gives [dev-dependencies] %s a value that is neither a version string nor an inline table", key))
		}

		if cargoInlineHasField(value, "workspace") {
			return refuse(fmt.Sprintf("inherits [dev-dependencies] %s from a workspace, whose path this reader does not follow", key))
		}
		if crateName, ok := cargoPathDependencyName(key, value); ok {
			if pkgName, found := knownNames[crateName]; found {
				deps = append(deps, pkgName)
			}
		}
	}
	return deps, nil
}

// spellsDevDependencies reports whether a key outside any dev-dependency
// table nonetheless assigns one: `dev-dependencies = { ... }` at the root,
// or a dotted key whose path passes through `dev-dependencies`.
func spellsDevDependencies(key string) bool {
	for _, part := range splitTomlKeyPath(key) {
		if part == "dev-dependencies" || part == "dev_dependencies" {
			return true
		}
	}
	return false
}

// cargoInlineHasField reports whether a one-line inline table assigns the
// field name at its top level (`{ workspace = true }`).
func cargoInlineHasField(value, name string) bool {
	for _, field := range splitCargoInlineFields(value) {
		field = strings.TrimLeft(strings.TrimSpace(field), "{")
		parts := strings.SplitN(field, "=", 2)
		if len(parts) == 2 && strings.TrimSpace(strings.ToLower(parts[0])) == name {
			return true
		}
	}
	return false
}

// resolveTestTriggers reads every Cargo-backed package's dev-dependencies and
// inverts them into TestTriggers. nameTable returns the (cached) name table
// for a language, the same tables the dependency graph resolves through.
// Self-references are dropped. A malformed dev-dependency table anywhere is
// an error for the whole resolution.
func resolveTestTriggers(
	packages []discovery.Package,
	nameTable func(language string) map[string]string,
) (TestTriggers, error) {
	sets := make(map[string]map[string]bool)
	for _, pkg := range packages {
		if !usesCargoDevDependencies(pkg.Language) {
			continue
		}
		devDeps, err := parseRustDevDeps(pkg, nameTable(cargoNameTableLanguage(pkg.Language)))
		if err != nil {
			return nil, err
		}
		for _, dep := range devDeps {
			if dep == pkg.Name {
				continue
			}
			if sets[dep] == nil {
				sets[dep] = make(map[string]bool)
			}
			sets[dep][pkg.Name] = true
		}
	}

	triggers := make(TestTriggers, len(sets))
	for dep, dependents := range sets {
		names := make([]string, 0, len(dependents))
		for name := range dependents {
			names = append(names, name)
		}
		sort.Strings(names)
		triggers[dep] = names
	}
	return triggers, nil
}
