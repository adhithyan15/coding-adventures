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
		"code/scripts/warrant_negctl.py", "code/scripts/tests/test_warrant_negctl.py",
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
		bezier2dFlatteningConsumers, satSolverSharedCorpusConsumers, {{closureProvenanceConsumer, "rust"}},
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

func TestMicrobenchmarkPairsConsumersExportCanonicalInput(t *testing.T) {
	want := []string{microbenchmarkPairsFixturePath}
	root := t.TempDir()
	packages := []discovery.Package{
		{Name: "rust/twig-guest-microbenchmark", Language: "rust", Path: filepath.Join(root, "code", "packages", "rust", "twig-guest-microbenchmark")},
		{Name: "rust/wasm-guest-microbenchmark", Language: "rust", Path: filepath.Join(root, "code", "packages", "rust", "wasm-guest-microbenchmark")},
		{Name: "go/programs/benchmark-tool", Language: "go", Path: filepath.Join(root, "code", "programs", "go", "benchmark-tool")},
		{Name: "dart/microbenchmark", Language: "dart", Path: filepath.Join(root, "code", "packages", "dart", "microbenchmark")},
		{Name: "dart/rank-statistics", Language: "dart", Path: filepath.Join(root, "code", "packages", "dart", "rank-statistics")},
		{Name: "elixir/microbenchmark", Language: "elixir", Path: filepath.Join(root, "code", "packages", "elixir", "microbenchmark")},
		{Name: "elixir/rank_statistics", Language: "elixir", Path: filepath.Join(root, "code", "packages", "elixir", "rank_statistics")},
		{Name: "lua/microbenchmark", Language: "lua", Path: filepath.Join(root, "code", "packages", "lua", "microbenchmark")},
		{Name: "lua/rank_statistics", Language: "lua", Path: filepath.Join(root, "code", "packages", "lua", "rank_statistics")},
		{Name: "perl/microbenchmark", Language: "perl", Path: filepath.Join(root, "code", "packages", "perl", "microbenchmark")},
		{Name: "perl/rank-statistics", Language: "perl", Path: filepath.Join(root, "code", "packages", "perl", "rank-statistics")},
		{Name: "haskell/microbenchmark", Language: "haskell", Path: filepath.Join(root, "code", "packages", "haskell", "microbenchmark")},
		{Name: "haskell/rank-statistics", Language: "haskell", Path: filepath.Join(root, "code", "packages", "haskell", "rank-statistics")},
		{Name: "ocaml/microbenchmark", Language: "ocaml", Path: filepath.Join(root, "code", "packages", "ocaml", "microbenchmark")},
		{Name: "ocaml/rank-statistics", Language: "ocaml", Path: filepath.Join(root, "code", "packages", "ocaml", "rank-statistics")},
		{Name: "csharp/microbenchmark", Language: "csharp", Path: filepath.Join(root, "code", "packages", "csharp", "microbenchmark")},
		{Name: "csharp/rank-statistics", Language: "csharp", Path: filepath.Join(root, "code", "packages", "csharp", "rank-statistics")},
		{Name: "fsharp/microbenchmark", Language: "fsharp", Path: filepath.Join(root, "code", "packages", "fsharp", "microbenchmark")},
		{Name: "fsharp/rank-statistics", Language: "fsharp", Path: filepath.Join(root, "code", "packages", "fsharp", "rank-statistics")},
		{Name: "java/microbenchmark", Language: "java", Path: filepath.Join(root, "code", "packages", "java", "microbenchmark")},
		{Name: "java/rank-statistics", Language: "java", Path: filepath.Join(root, "code", "packages", "java", "rank-statistics")},
		{Name: "kotlin/microbenchmark", Language: "kotlin", Path: filepath.Join(root, "code", "packages", "kotlin", "microbenchmark")},
		{Name: "kotlin/rank-statistics", Language: "kotlin", Path: filepath.Join(root, "code", "packages", "kotlin", "rank-statistics")},
		{Name: "c/microbenchmark", Language: "c", Path: filepath.Join(root, "code", "packages", "c", "microbenchmark")},
		{Name: "c/rank-statistics", Language: "c", Path: filepath.Join(root, "code", "packages", "c", "rank-statistics")},
		{Name: "swift/microbenchmark", Language: "swift", Path: filepath.Join(root, "code", "packages", "swift", "microbenchmark")},
		{Name: "swift/rank-statistics", Language: "swift", Path: filepath.Join(root, "code", "packages", "swift", "rank-statistics")},
		{Name: "cpp/microbenchmark", Language: "cpp", Path: filepath.Join(root, "code", "packages", "cpp", "microbenchmark")},
		{Name: "cpp/rank-statistics", Language: "cpp", Path: filepath.Join(root, "code", "packages", "cpp", "rank-statistics")},
		{Name: "ruby/microbenchmark", Language: "ruby", Path: filepath.Join(root, "code", "packages", "ruby", "microbenchmark")},
		{Name: "ruby/rank_statistics", Language: "ruby", Path: filepath.Join(root, "code", "packages", "ruby", "rank_statistics")},
		{Name: "rust/microbenchmark", Language: "rust", Path: filepath.Join(root, "code", "packages", "rust", "microbenchmark")},
		{Name: "rust/rank-statistics", Language: "rust", Path: filepath.Join(root, "code", "packages", "rust", "rank-statistics")},
		{Name: "typescript/microbenchmark", Language: "typescript", Path: filepath.Join(root, "code", "packages", "typescript", "microbenchmark")},
		{Name: "typescript/rank-statistics", Language: "typescript", Path: filepath.Join(root, "code", "packages", "typescript", "rank-statistics")},
		{Name: "go/microbenchmark", Language: "go", Path: filepath.Join(root, "code", "packages", "go", "microbenchmark")},
		{Name: "go/rank-statistics", Language: "go", Path: filepath.Join(root, "code", "packages", "go", "rank-statistics")},
		{Name: "python/microbenchmark", Language: "python", Path: filepath.Join(root, "code", "packages", "python", "microbenchmark")},
		{Name: "python/rank-statistics", Language: "python", Path: filepath.Join(root, "code", "packages", "python", "rank-statistics")},
	}
	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"rust/twig-guest-microbenchmark", "rust/wasm-guest-microbenchmark", "go/programs/benchmark-tool", "csharp/microbenchmark", "csharp/rank-statistics", "fsharp/microbenchmark", "fsharp/rank-statistics", "java/microbenchmark", "java/rank-statistics", "kotlin/microbenchmark", "kotlin/rank-statistics", "c/microbenchmark", "c/rank-statistics", "swift/microbenchmark", "swift/rank-statistics", "cpp/microbenchmark", "cpp/rank-statistics", "ruby/microbenchmark", "ruby/rank_statistics", "rust/microbenchmark", "rust/rank-statistics", "typescript/microbenchmark", "typescript/rank-statistics", "go/microbenchmark", "go/rank-statistics", "python/microbenchmark", "python/rank-statistics", "dart/microbenchmark", "dart/rank-statistics", "elixir/microbenchmark", "elixir/rank_statistics", "lua/microbenchmark", "lua/rank_statistics", "perl/microbenchmark", "perl/rank-statistics", "haskell/microbenchmark", "haskell/rank-statistics", "ocaml/microbenchmark", "ocaml/rank-statistics"} {
		var got []string
		for _, pkg := range graph.Packages {
			if pkg.Name == name {
				got = pkg.RepoInputs
				if len(pkg.RepoInputGlobs) != 0 || pkg.RepoInputsUnknown {
					t.Errorf("%s unexpectedly exports globs/unknown=%v", name, pkg.RepoInputsUnknown)
				}
				break
			}
		}
		if !reflect.DeepEqual(got, want) {
			t.Errorf("%s exports %v; want only %v", name, got, want)
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
