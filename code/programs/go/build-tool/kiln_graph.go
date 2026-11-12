package main

// The Kiln legacy graph export (-emit-kiln-graph)
// ===============================================
//
// Kiln is the content-addressed build system that is meant to replace this
// tool. Its core knows no language: it hashes inputs, runs commands and
// stores outputs, and nothing in it understands Cargo.toml, package.json or a
// BUILD file's dialect. But the repository's real structure is language
// knowledge, and this tool is where it already lives:
//
//   - which BUILD file each operating system actually runs,
//   - the dependency edges the resolver reads out of each language's
//     manifests (which differ per OS when a platform BUILD file declares
//     different prerequisites),
//   - the Cargo dev-dependencies that make a crate's tests link another crate,
//   - the repository-wide inputs a test reads outside its own directory
//     (repository_inputs.go), and
//   - the toolchains a package needs beyond its own language.
//
// -emit-kiln-graph is the seam. It writes that knowledge once, as plain data,
// in a schema (kiln-legacy-graph/1, KILN03 section 2) that Kiln reads without
// importing a line of this tool. When a package has a real Kiln recipe the
// export is simply not consulted for it; until then it lets Kiln key and
// schedule the legacy shell BUILDs.
//
// # What the export is NOT
//
// It is not a build plan. -emit-plan answers "what should CI run for this
// diff on the detect runner"; this answers "what is the whole repository, on
// every runner, regardless of any diff". So it:
//
//   - lists EVERY discovered package: it forces -language all, because a
//     language slice of the graph would silently drop cross-language edges;
//   - implies -force: no Git diff is read, and none is needed;
//   - describes all three operating systems at once, resolving the graph
//     once per OS the way emitBuildPlanAgainst computes platform_overrides.
//
// It does honour -validate-build-files, exactly as -emit-plan does, so an
// inconsistent tree fails before a graph is written from it.
//
// # Field by field
//
//	name, rel_path, language  identity: the qualified name ("rust/parser"),
//	                          the slash-separated repo-relative directory,
//	                          and the discovery bucket.
//	build_files               per OS, the one file name (relative to rel_path)
//	                          GetBuildFileForPlatform selects, or null when
//	                          the package has no file for that OS. The
//	                          candidate order is not duplicated here: asking
//	                          the discovery function is what keeps the two
//	                          from drifting.
//	deps                      per OS, the DIRECT dependencies in the resolved
//	                          graph. The graph stores edges dep -> dependent
//	                          ("dep must build first"), so a package's deps
//	                          are its graph PREDECESSORS. Transitive closure
//	                          is the consumer's job.
//	dev_deps                  the test-trigger relation inverted. The resolver
//	                          stores TestTriggers[D] = packages whose tests
//	                          link D; Kiln wants, for a package P, the D it
//	                          links, so dev_deps(P) = { D : P in TestTriggers[D] }.
//	                          It is OS-independent in intent; we take the union
//	                          over the three OS resolutions so a [target.cfg]
//	                          dev-dependency is never lost. These are NOT
//	                          edges: a dev-dep cycle (A deps B, B dev-deps A)
//	                          is legal and must stay out of `deps`.
//	repo_inputs               the repo-relative paths from the
//	                          repositoryInputConsumers table whose entry names
//	                          this package. A consumer whose predicate cannot
//	                          be written as a path list lists none (see the
//	                          `paths` field comment there).
//	repo_input_globs          the same, for inputs that are a pattern ("**"
//	                          any depth, "*" within a segment) rather than a
//	                          finite list, e.g. code/**/*.mll for Mosaic.
//	repo_inputs_unknown       true (otherwise omitted) when a consumer
//	                          declares neither paths nor globs.
//	extra_toolchains          toolchains beyond the package's own language,
//	                          the union of discovery's value and each OS's
//	                          re-derivation from that OS's BUILD file.
//
// Every list is sorted and de-duplicated and every collection is emitted
// non-null, so the same tree always yields byte-identical output.

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// kilnGraphSchema is the schema tag stamped on every export.
const kilnGraphSchema = "kiln-legacy-graph/1"

// kilnGraphPlatforms are the Go GOOS names the export describes, in the order
// the plan uses. JSON object keys are sorted on output regardless.
var kilnGraphPlatforms = []string{"linux", "darwin", "windows"}

type kilnLegacyGraph struct {
	Schema   string              `json:"schema"`
	Packages []kilnLegacyPackage `json:"packages"`
}

type kilnLegacyPackage struct {
	Name           string              `json:"name"`
	RelPath        string              `json:"rel_path"`
	Language       string              `json:"language"`
	BuildFiles     map[string]*string  `json:"build_files"`
	Deps           map[string][]string `json:"deps"`
	DevDeps        []string            `json:"dev_deps"`
	RepoInputs     []string            `json:"repo_inputs"`
	RepoInputGlobs []string            `json:"repo_input_globs"`
	// RepoInputsUnknown is set (and otherwise omitted) when a consumer
	// names this package but declares neither paths nor globs: the export
	// cannot say what it reads, and Kiln must not treat [] as "nothing".
	RepoInputsUnknown bool     `json:"repo_inputs_unknown,omitempty"`
	ExtraToolchains   []string `json:"extra_toolchains"`
}

// sortedUnique returns the set's members sorted, never nil.
func sortedUnique(set map[string]bool) []string {
	out := make([]string, 0, len(set))
	for v := range set {
		out = append(out, v)
	}
	sort.Strings(out)
	return out
}

// buildKilnLegacyGraph computes the export for every package in packages
// (which must be the full discovery list, Starlark already evaluated).
func buildKilnLegacyGraph(packages []discovery.Package, repoRoot string) (*kilnLegacyGraph, error) {
	// Per-OS views: the package list re-read for that OS's BUILD file, then
	// resolved against itself (nothing is filtered, so known == packages).
	deps := make(map[string]map[string]map[string]bool) // goos -> pkg -> deps
	devDeps := make(map[string]map[string]bool)         // pkg -> dev deps
	toolchains := make(map[string]map[string]bool)      // pkg -> toolchains
	for _, pkg := range packages {
		toolchains[pkg.Name] = make(map[string]bool)
		for _, tc := range pkg.ExtraToolchains {
			toolchains[pkg.Name][tc] = true
		}
	}

	for _, goos := range kilnGraphPlatforms {
		platformPackages := packagesForPlatform(packages, goos)
		graph, triggers, err := resolver.ResolveWithTestTriggersAgainst(platformPackages, platformPackages)
		if err != nil {
			return nil, fmt.Errorf("resolving %s graph: %w", goos, err)
		}
		deps[goos] = make(map[string]map[string]bool)
		for _, pkg := range platformPackages {
			set := make(map[string]bool)
			// Edges run dep -> dependent, so a package's deps are its
			// predecessors.
			if graph != nil && graph.HasNode(pkg.Name) {
				preds, err := graph.Predecessors(pkg.Name)
				if err != nil {
					return nil, fmt.Errorf("reading %s dependencies of %s: %w", goos, pkg.Name, err)
				}
				for _, d := range preds {
					set[d] = true
				}
			}
			deps[goos][pkg.Name] = set
			for _, tc := range pkg.ExtraToolchains {
				toolchains[pkg.Name][tc] = true
			}
		}
		// Invert TestTriggers[D] = {P...} into dev_deps(P) = {D...}.
		for dep, dependents := range triggers {
			for _, p := range dependents {
				if devDeps[p] == nil {
					devDeps[p] = make(map[string]bool)
				}
				devDeps[p][dep] = true
			}
		}
	}

	out := &kilnLegacyGraph{Schema: kilnGraphSchema, Packages: make([]kilnLegacyPackage, 0, len(packages))}
	for _, pkg := range packages {
		rel, err := filepath.Rel(repoRoot, pkg.Path)
		if err != nil {
			rel = pkg.Path
		}
		entry := kilnLegacyPackage{
			Name:            pkg.Name,
			RelPath:         filepath.ToSlash(rel),
			Language:        pkg.Language,
			BuildFiles:      make(map[string]*string, len(kilnGraphPlatforms)),
			Deps:            make(map[string][]string, len(kilnGraphPlatforms)),
			DevDeps:         sortedUnique(devDeps[pkg.Name]),
			RepoInputs:      []string{},
			RepoInputGlobs:  []string{},
			ExtraToolchains: sortedUnique(toolchains[pkg.Name]),
		}
		for _, goos := range kilnGraphPlatforms {
			if file := discovery.GetBuildFileForPlatform(pkg.Path, goos); file != "" {
				name := filepath.Base(file)
				entry.BuildFiles[goos] = &name
			} else {
				entry.BuildFiles[goos] = nil
			}
			entry.Deps[goos] = sortedUnique(deps[goos][pkg.Name])
		}
		// A repo-input entry belongs to the package it names (and, as in
		// repositoryInputConsumersForChange, only when the language matches).
		for _, consumer := range repositoryInputConsumers {
			if consumer.name == pkg.Name && consumer.language == pkg.Language {
				entry.RepoInputs = append(entry.RepoInputs, consumer.paths...)
				entry.RepoInputGlobs = append(entry.RepoInputGlobs, consumer.globs...)
				if len(consumer.paths) == 0 && len(consumer.globs) == 0 {
					entry.RepoInputsUnknown = true
				}
			}
		}
		entry.RepoInputs = sortedUnique(stringSet(entry.RepoInputs))
		entry.RepoInputGlobs = sortedUnique(stringSet(entry.RepoInputGlobs))
		out.Packages = append(out.Packages, entry)
	}
	sort.Slice(out.Packages, func(i, j int) bool { return out.Packages[i].Name < out.Packages[j].Name })
	return out, nil
}

func stringSet(values []string) map[string]bool {
	set := make(map[string]bool, len(values))
	for _, v := range values {
		set[v] = true
	}
	return set
}

// marshalKilnLegacyGraph renders the export: two-space indented JSON with a
// trailing newline. encoding/json sorts map keys, so output is deterministic.
func marshalKilnLegacyGraph(graph *kilnLegacyGraph) ([]byte, error) {
	data, err := json.MarshalIndent(graph, "", "  ")
	if err != nil {
		return nil, err
	}
	return append(data, '\n'), nil
}

// emitKilnLegacyGraph writes the export for packages to outputPath.
func emitKilnLegacyGraph(packages []discovery.Package, repoRoot, outputPath string) int {
	graph, err := buildKilnLegacyGraph(packages, repoRoot)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error building Kiln legacy graph: %v\n", err)
		return 1
	}
	data, err := marshalKilnLegacyGraph(graph)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error encoding Kiln legacy graph: %v\n", err)
		return 1
	}
	if err := os.WriteFile(outputPath, data, 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "Error writing Kiln legacy graph %s: %v\n", outputPath, err)
		return 1
	}
	fmt.Printf("Kiln legacy graph written to %s (%d packages)\n", outputPath, len(graph.Packages))
	return 0
}
