package main

// plan_language.go: the prerequisite-closed single-language build plan
// (-plan-closed-language L -emit-plan).
//
// # Why a new flag and not -language
//
// -language L with -emit-plan keeps its established meaning (it only steers
// the affected-root selectors and toolchain flags; the emitted plan still
// holds every package so a TypeScript plan retains the Rust providers its
// consumers hash against -- see game_input_frame_plan_test.go).  Narrowing the
// emitted plan is a different request, so it is a different, opt-in flag.
// It is valid only with -emit-plan.  Without the flag nothing here runs, and
// the plan is byte-for-byte what it was.
//
// # What the filter does with cross-language prerequisites in existing modes (unchanged)
//
//   - Normal builds (no -emit-plan): packages is narrowed to language L AFTER
//     the dependency graph is resolved over every package.  The executor only
//     schedules packages in the narrowed list, so a prerequisite of another
//     language is silently never built and never ordered against.  That is the
//     right behaviour for a local "-language go" run (the user's toolchain
//     already has what it needs) and it is NOT changed here.
//   - Plan emission: no narrowing at all (see above).  Shard closure
//     (plan.ComputeShards) pulls in every transitive prerequisite of any
//     language, so a Go package with a Python prerequisite dragged that Python
//     package into the Go shard.
//   - Plan consumption (-plan-file -language L): shard closure first, then the
//     language narrowing, i.e. the same silent drop as normal builds.
//
// # The semantics of -plan-closed-language L
//
// With -plan-closed-language L the plan contains exactly the packages of
// language L that are PREREQUISITE-CLOSED within L: a package is selected iff
// every transitive prerequisite of it is also of language L.  A package with
// any foreign (non-L) transitive prerequisite is LEFT OUT -- not selected, and
// its prerequisite is not pulled in either.  Rationale: a recording or shard
// runner that only has L's toolchain cannot build the foreign prerequisite, so
// "build the package anyway, minus its prerequisite" would measure something
// that is not the real build.  Excluding is the honest answer.
//
// The test is a taint propagation over the dependency graph, whose edges run
// prerequisite -> dependent:
//
//	taint every package whose language != L
//	taint every dependent (transitively) of a tainted package
//	selected = packages of language L that are not tainted
//
// It runs once per runner platform (linux, darwin, windows) because platform
// BUILD files can add prerequisites; a package is excluded if it is tainted on
// ANY platform, so the one plan is closed on all of them.  Graph nodes that
// are not in the package list are treated as foreign (fail closed).
//
// "all" skips this file entirely, so the plan for "all" is byte-for-byte what
// it was.  The plan JSON gains no fields: the selection is visible as the
// package list itself, and Kiln records the language it passed.

import (
	"sort"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// languagePlanSelection reports what selectPrerequisiteClosedLanguage chose.
type languagePlanSelection struct {
	// Selected are the surviving packages, in the input order.
	Selected []discovery.Package
	// Excluded are the names of packages of the language that were left out
	// because a transitive prerequisite has another language, sorted.
	Excluded []string
}

// graphForPlatform resolves the dependency graph of one runner platform.
type graphForPlatform func(goos string, packages []discovery.Package) (*directedgraph.Graph, error)

// selectPrerequisiteClosedLanguage implements the semantics above.
// graphFor resolves the full (unfiltered) package list for one platform.
func selectPrerequisiteClosedLanguage(
	packages []discovery.Package,
	language string,
	platforms []string,
	graphFor graphForPlatform,
) (languagePlanSelection, error) {
	excluded := make(map[string]bool)
	for _, goos := range platforms {
		graph, err := graphFor(goos, packages)
		if err != nil {
			return languagePlanSelection{}, err
		}
		tainted, err := foreignTaint(graph, packages, language)
		if err != nil {
			return languagePlanSelection{}, err
		}
		for name := range tainted {
			excluded[name] = true
		}
	}

	var result languagePlanSelection
	for _, pkg := range packages {
		if pkg.Language != language {
			continue
		}
		if excluded[pkg.Name] {
			result.Excluded = append(result.Excluded, pkg.Name)
			continue
		}
		result.Selected = append(result.Selected, pkg)
	}
	sort.Strings(result.Excluded)
	return result, nil
}

// successorGraph is the part of the dependency graph the taint walk needs;
// an interface so a test can inject a failing graph.
type successorGraph interface {
	Nodes() []string
	Successors(node string) ([]string, error)
}

// foreignTaint returns every graph node that is, or transitively depends on,
// a node whose language is not `language`.
func foreignTaint(graph successorGraph, packages []discovery.Package, language string) (map[string]bool, error) {
	languageOf := make(map[string]string, len(packages))
	for _, pkg := range packages {
		languageOf[pkg.Name] = pkg.Language
	}
	tainted := make(map[string]bool)
	var queue []string
	for _, node := range graph.Nodes() {
		if lang, known := languageOf[node]; !known || lang != language {
			tainted[node] = true
			queue = append(queue, node)
		}
	}
	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		dependents, err := graph.Successors(node)
		if err != nil {
			return nil, err
		}
		for _, dependent := range dependents {
			if !tainted[dependent] {
				tainted[dependent] = true
				queue = append(queue, dependent)
			}
		}
	}
	return tainted, nil
}

// platformGraphResolver is the production graphForPlatform: the same
// per-platform BUILD-file overlay and resolution emitBuildPlanAgainst uses.
func platformGraphResolver(known []discovery.Package) graphForPlatform {
	return func(goos string, packages []discovery.Package) (*directedgraph.Graph, error) {
		platformPackages := packagesForPlatform(packages, goos)
		platformKnown := known
		if platformKnown == nil {
			platformKnown = platformPackages
		}
		graph, _, err := resolver.ResolveWithTestTriggersAgainst(platformPackages, platformKnown)
		return graph, err
	}
}
