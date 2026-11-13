package main

// Every package -> external input relation the selector knows
// ===========================================================
//
// selectPackageRoots (main.go) decides which packages a changed file
// schedules. Besides ordinary "this file is inside that package" matching it
// knows many *external* relations: a conformance fixture under code/specs/
// that a native test in some build-tool port reads, an external corpus whose
// reader is a library, a spec that gates a compiler's acceptance step, and the
// repository-wide guards in repository_inputs.go.
//
// Kiln keys a legacy package on its declared inputs, so every one of those
// relations must reach the Kiln graph export (-emit-kiln-graph) as
// `repo_inputs` / `repo_input_globs`; a relation missing from the export
// would let Kiln replay a stale cache entry after a fixture changed.
//
// # One source of truth
//
// The selector reads the consumer lists, the fixture path prefixes, the
// corpus path constants and the case maps declared in main.go. This file does
// NOT copy any of them: externalInputRelations() is built from those very
// variables and constants, so adding a consumer or a case to the selector
// adds it to the export in the same edit. TestEveryExternalConsumerExportsInputs
// enumerates every {name, language} the selector can seed and requires the
// export to give it non-empty inputs (or repo_inputs_unknown).
//
// # Shape of each relation
//
//   - exact corpus files and per-case fixture files are `paths`;
//   - a family selected by "prefix + any flat .json" is the glob
//     prefix+"*.json". `*` also matches an empty stem and a backslash, which
//     the selector rejects, so the glob is deliberately a superset of what the
//     selector reads: over-keying is safe, under-keying is not;
//   - a relation that names consumers but no path data at all would be marked
//     repo_inputs_unknown by the export (none exist today).

import (
	"sort"
)

type consumerRef = struct{ name, language string }

// externalInputRelation is one selector relation: every consumer in
// consumers reads every path and glob.
type externalInputRelation struct {
	what      string
	consumers []consumerRef
	paths     []string
	globs     []string
}

// flatFamilyGlob is the glob covering the flat "<prefix><stem>.json" files the
// selector's has*FixturePath / *FixtureCase helpers recognise.
func flatFamilyGlob(prefix string) string { return prefix + "*.json" }

// casePaths expands a case map into the exact fixture paths whose reader-code
// string contains code.
func casePaths(prefix string, cases map[string]string, code rune) []string {
	var out []string
	for name, codes := range cases {
		for _, c := range codes {
			if c == code {
				out = append(out, prefix+name+".json")
				break
			}
		}
	}
	sort.Strings(out)
	return out
}

// codedRelations turns a (reader table, case map) pair into one relation per
// reader, carrying the exact case files that reader is scheduled for.
func codedRelations(what, prefix string, readers map[rune]consumerRef, cases map[string]string) []externalInputRelation {
	codes := make([]rune, 0, len(readers))
	for code := range readers {
		codes = append(codes, code)
	}
	sort.Slice(codes, func(i, j int) bool { return codes[i] < codes[j] })
	var out []externalInputRelation
	for _, code := range codes {
		paths := casePaths(prefix, cases, code)
		if len(paths) == 0 {
			continue // a reader no case schedules reads nothing external
		}
		out = append(out, externalInputRelation{
			what:      what,
			consumers: []consumerRef{readers[code]},
			paths:     paths,
		})
	}
	return out
}

// externalInputRelations lists every relation of selectPackageRoots except
// repositoryInputConsumers (which carries its own paths and globs).
func externalInputRelations() []externalInputRelation {
	relations := []externalInputRelation{
		{what: "shared discovery fixture", consumers: sharedDiscoveryFixtureConsumers, paths: []string{sharedDiscoveryFixturePath}},
		{what: "duplicate discovery fixture", consumers: duplicateDiscoveryFixtureConsumers, paths: []string{duplicateDiscoveryFixturePath}},
		{what: "CI gate fixtures", consumers: ciGateFixtureConsumers, globs: []string{flatFamilyGlob(ciGateFixturePrefix)}},
		{what: "test trigger fixtures", consumers: testTriggerFixtureConsumers, globs: []string{flatFamilyGlob(testTriggerFixturePrefix)}},
		{what: "package metadata fixtures", consumers: packageMetadataFixtureConsumers, globs: []string{flatFamilyGlob(packageMetadataFixturePrefix)}},
		{what: "toolchain detection fixtures", consumers: toolchainDetectionFixtureConsumers, globs: []string{flatFamilyGlob(toolchainDetectionFixturePrefix)}},
		// The "local" family is every source-collection file that is not
		// repository-/shared-input-; the glob over the whole prefix covers it
		// (and more, which is safe).
		{what: "source collection local fixtures", consumers: sourceCollectionLocalFixtureConsumers, globs: []string{flatFamilyGlob(sourceCollectionFixturePrefix)}},
		{what: "source collection repository fixtures", consumers: sourceCollectionRepositoryFixtureConsumers, globs: []string{flatFamilyGlob(sourceCollectionFixturePrefix + "repository-")}},
		{what: "source collection shared-input fixtures", consumers: sourceCollectionSharedInputFixtureConsumers, globs: []string{flatFamilyGlob(sourceCollectionFixturePrefix + "shared-input-")}},
		{what: "graph and diff-selection fixtures", consumers: graphDiffNativeFixtureConsumers, globs: []string{flatFamilyGlob(graphFixturePrefix), flatFamilyGlob(diffSelectionFixturePrefix)}},

		{what: "CV02 closure provenance specification", consumers: []consumerRef{{closureProvenanceConsumer, "rust"}}, paths: []string{closureProvenanceSpecPath}},
		{what: "STP01 opcode provider corpus", consumers: starlarkOpcodeProviderConsumers, paths: []string{starlarkOpcodeProviderCorpusPath}},
		{what: "BF03 compiler IR corpus", consumers: compilerIRCorpusConsumers, paths: []string{compilerIRCorpusPath}},
		{what: "TE04 GFM corpora", consumers: gfmCorpusConsumers, paths: []string{gfmFencedBlockCorpusPath, gfmExtensionCorpusPath}},
		{what: "G2D02 bezier flattening corpus", consumers: bezier2dFlatteningConsumers, paths: []string{bezier2dFlatteningCorpusPath}},
	}
	relations = append(relations, codedRelations("resolution fixtures", resolutionFixtureCasePrefix, resolutionNativeFixtureReaders, resolutionNativeFixtureCases)...)
	relations = append(relations, codedRelations("hashing-cache fixtures", hashingCacheFixtureCasePrefix, hashingCacheNativeFixtureReaders, hashingCacheNativeFixtureCases)...)
	relations = append(relations, codedRelations("validation fixtures", validationFixtureCasePrefix, validationNativeFixtureReaders, validationNativeFixtureCases)...)
	relations = append(relations, codedRelations("plan fixtures", planFixtureCasePrefix, planNativeFixtureReaders, planNativeFixtureCases)...)
	return relations
}

// consumerInputs is what one package reads outside its directory.
type consumerInputs struct {
	paths, globs []string
	unknown      bool
}

// repositoryInputsByPackage merges every relation (and the
// repositoryInputConsumers table) into per-package-name inputs (the selector matches consumers by name; language only filters -language).
func repositoryInputsByPackage() map[string]*consumerInputs {
	out := make(map[string]*consumerInputs)
	add := func(c consumerRef, paths, globs []string) {
		in := out[c.name]
		if in == nil {
			in = &consumerInputs{}
			out[c.name] = in
		}
		in.paths = append(in.paths, paths...)
		in.globs = append(in.globs, globs...)
		if len(paths) == 0 && len(globs) == 0 {
			in.unknown = true // fail closed: a relation with no path data
		}
	}
	for _, relation := range externalInputRelations() {
		for _, c := range relation.consumers {
			add(c, relation.paths, relation.globs)
		}
	}
	for _, consumer := range repositoryInputConsumers {
		add(consumerRef{consumer.name, consumer.language}, consumer.paths, consumer.globs)
	}
	return out
}
