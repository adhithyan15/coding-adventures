package main

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

// The safety direction of predicate vs declared inputs: every file a
// predicate accepts must be covered by a declared path or glob, or Kiln would
// not key on it. The corpus is the real checkout plus synthetic names under
// each prefix a predicate accepts.
func coveredByDeclared(changed string, paths, globs []string) bool {
	for _, p := range paths {
		if changed == p || strings.HasPrefix(changed, strings.TrimSuffix(p, "/")+"/") {
			return true
		}
	}
	for _, g := range globs {
		if globmatch.MatchPath(g, changed) {
			return true
		}
	}
	return false
}

func TestRepositoryInputPredicatesAreCoveredByDeclaredInputs(t *testing.T) {
	corpus := []string{
		"code/scripts/check-perl-extension-windows.ps1",
		"code/packages/rust/perl-bridge/Cargo.toml", "code/packages/rust/perl-bridge/build.rs",
		"code/packages/rust/perl-bridge/src/lib.rs", "code/packages/rust/perl-bridge/src/a/b.rs",
		"code/specs/fixtures/ct-compare-v1/cases.json", "code/specs/fixtures/ct-compare-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-v1/new/deep.txt",
		"code/scripts/ct_compare_tfv.py", "code/scripts/ct_compare_conformance.py", "code/scripts/tfv.py",
		"code/specs/fixtures/test-bootstrap-v1/cases.json", "code/scripts/bootstrap_harness_conformance.py",
		"code/specs/fixtures/rng-v1/cases.json", "code/specs/fixtures/rng-v1/new/x.json",
		"code/grammars/warrant/a.grammar", "code/grammars/warrant/sub/b.grammar",
		"code/specs/fixtures/warrant-syntax-v1/x", "code/specs/fixtures/warrant-vectors-synthetic-v1/y/z",
		"code/scripts/warrant_syntax_corpus.py", "code/scripts/tests/test_warrant_syntax_corpus.py",
		"code/specs/VER04-warrant-language.md", "code/specs/VER04a-warrant-source-admission.md",
		"code/a.mil", "code/x/y/Z.mll", "code/x/mosaic-package.toml", "code/q/r/s/t.msl",
	}
	root := toolchainFixtureRepoRoot(t)
	skip := map[string]bool{".git": true, "node_modules": true, "target": true, ".build": true}
	_ = filepath.WalkDir(filepath.Join(root, "code"), func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return nil
		}
		if d.IsDir() {
			if skip[d.Name()] {
				return filepath.SkipDir
			}
			return nil
		}
		if rel, relErr := filepath.Rel(root, path); relErr == nil {
			corpus = append(corpus, filepath.ToSlash(rel))
		}
		return nil
	})
	accepted := 0
	for _, consumer := range repositoryInputConsumers {
		for _, changed := range corpus {
			if consumer.matches(changed) {
				accepted++
				if !coveredByDeclared(changed, consumer.paths, consumer.globs) {
					t.Errorf("%s: predicate accepts %q but no declared path or glob covers it", consumer.name, changed)
				}
			}
		}
	}
	if accepted == 0 {
		t.Fatal("corpus exercised no predicate")
	}
}

// Every flat fixture family the selector recognises by prefix is covered by
// the exported glob for that prefix.
func TestFixtureFamilyGlobsCoverSelectorHelpers(t *testing.T) {
	cases := []struct {
		prefix string
		helper func([]string) bool
	}{
		{ciGateFixturePrefix, hasCIGateFixturePath},
		{testTriggerFixturePrefix, hasTestTriggerFixturePath},
		{packageMetadataFixturePrefix, hasPackageMetadataFixturePath},
		{toolchainDetectionFixturePrefix, hasToolchainDetectionFixturePath},
		{graphFixturePrefix, hasGraphDiffFixturePath},
		{diffSelectionFixturePrefix, hasGraphDiffFixturePath},
	}
	for _, c := range cases {
		file := c.prefix + "some-case.json"
		if !c.helper([]string{file}) || !globmatch.MatchPath(flatFamilyGlob(c.prefix), file) {
			t.Errorf("prefix %s: helper and glob disagree on %s", c.prefix, file)
		}
	}
	for _, name := range []string{"x.json", "repository-x.json", "shared-input-x.json"} {
		file := sourceCollectionFixturePrefix + name
		if sourceCollectionFixtureFamily(file) == "" || !globmatch.MatchPath(flatFamilyGlob(sourceCollectionFixturePrefix), file) {
			t.Errorf("source-collection %s not covered", file)
		}
	}
	for _, fam := range []string{"repository-", "shared-input-"} {
		if !globmatch.MatchPath(flatFamilyGlob(sourceCollectionFixturePrefix+fam), sourceCollectionFixturePrefix+fam+"x.json") {
			t.Errorf("family glob for %s does not match", fam)
		}
	}
}

// Enumerates every {name, language} the selector can seed, straight from the
// raw tables (not from externalInputRelations), and requires the export to
// give it inputs or the unknown flag.
func TestEveryExternalConsumerExportsInputs(t *testing.T) {
	var all []consumerRef
	for _, list := range [][]consumerRef{
		sharedDiscoveryFixtureConsumers, duplicateDiscoveryFixtureConsumers, ciGateFixtureConsumers,
		testTriggerFixtureConsumers, packageMetadataFixtureConsumers, toolchainDetectionFixtureConsumers,
		sourceCollectionLocalFixtureConsumers, sourceCollectionRepositoryFixtureConsumers,
		sourceCollectionSharedInputFixtureConsumers, graphDiffNativeFixtureConsumers,
		starlarkOpcodeProviderConsumers, compilerIRCorpusConsumers, gfmCorpusConsumers,
		bezier2dFlatteningConsumers, {{closureProvenanceConsumer, "rust"}},
	} {
		all = append(all, list...)
	}
	// Coded readers count only when some case schedules them.
	coded := []struct {
		readers map[rune]consumerRef
		cases   map[string]string
	}{
		{resolutionNativeFixtureReaders, resolutionNativeFixtureCases},
		{hashingCacheNativeFixtureReaders, hashingCacheNativeFixtureCases},
		{validationNativeFixtureReaders, validationNativeFixtureCases},
		{planNativeFixtureReaders, planNativeFixtureCases},
	}
	for _, c := range coded {
		used := map[rune]bool{}
		for _, codes := range c.cases {
			for _, r := range codes {
				used[r] = true
			}
		}
		for r := range used {
			reader, ok := c.readers[r]
			if !ok {
				t.Fatalf("case code %q has no reader", r)
			}
			all = append(all, reader)
		}
	}
	for _, consumer := range repositoryInputConsumers {
		all = append(all, consumerRef{consumer.name, consumer.language})
	}
	inputs := repositoryInputsByPackage()
	for _, c := range all {
		in := inputs[c.name]
		if in == nil || (len(in.paths) == 0 && len(in.globs) == 0 && !in.unknown) {
			t.Errorf("%s (%s) is seeded by the selector but exports no repository inputs", c.name, c.language)
		}
	}
}

func TestKilnExportRefusesSymlinkAndEmpty(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "real.json")
	link := filepath.Join(dir, "link.json")
	if err := os.WriteFile(target, []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(target, link); err != nil {
		t.Skip("symlinks unavailable")
	}
	pkgs := []discovery.Package{{Name: "rust/x", Language: "rust", Path: filepath.Join(dir, "code", "packages", "rust", "x")}}
	if code := emitKilnLegacyGraph(pkgs, dir, link, false); code == 0 {
		t.Fatal("a symlink target must be refused")
	}
	if got, _ := os.ReadFile(target); string(got) != "x" {
		t.Fatal("the symlink target was overwritten")
	}
	if code := emitKilnLegacyGraph(nil, dir, filepath.Join(dir, "o.json"), false); code == 0 {
		t.Fatal("an empty export must fail")
	}
	// A package path that cannot be made repo-relative is an error.
	bad := []discovery.Package{{Name: "rust/x", Language: "rust", Path: "relative/x"}}
	if _, err := buildKilnLegacyGraph(bad, dir); err == nil {
		t.Fatal("an un-relativizable package path must be an error")
	}
}

func TestKilnExportRequiresDiscoveredConsumers(t *testing.T) {
	pkgs := []discovery.Package{{Name: "rust/x", Language: "rust"}}
	if len(missingExternalConsumers(pkgs)) == 0 {
		t.Fatal("the selector tables name packages that are not in this list")
	}
	dir := t.TempDir()
	if code := emitKilnLegacyGraph(pkgs, dir, filepath.Join(dir, "o.json"), true); code == 0 {
		t.Fatal("strict mode must fail on missing consumers")
	}
}

// A Starlark host BUILD beside a shell BUILD_windows: the Windows file's
// needs-toolchain directive must reach the export.
func TestKilnGraphReadsPlatformFilesOfStarlarkPackages(t *testing.T) {
	root := t.TempDir()
	dir := filepath.Join(root, "code", "packages", "rust", "starpkg")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for name, content := range map[string]string{
		"BUILD":         "load(\"//rules:rust.star\", \"rust_library\")\n",
		"BUILD_windows": "# needs-toolchain: java\ncargo test\n",
		"Cargo.toml":    "[package]\nname = \"starpkg\"\nversion = \"0.1.0\"\n",
	} {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	packages, err := discovery.DiscoverPackagesAllPlatforms(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	for i := range packages {
		packages[i].IsStarlark = true
	}
	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatal(err)
	}
	if got := graph.Packages[0].ExtraToolchains; !reflect.DeepEqual(got, []string{"java"}) {
		t.Fatalf("extra_toolchains = %v, want [java] from BUILD_windows", got)
	}
}
