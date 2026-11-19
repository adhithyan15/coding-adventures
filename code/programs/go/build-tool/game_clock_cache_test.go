package main

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/graphdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

func writeGameClockCacheFixture(t *testing.T) (string, discovery.Package, *directedgraph.Graph) {
	t.Helper()
	root := t.TempDir()
	repositoryRoot := toolchainFixtureRepoRoot(t)
	for _, input := range gameClockInputPaths {
		original, err := os.ReadFile(filepath.Join(repositoryRoot, filepath.FromSlash(input)))
		if err != nil {
			t.Fatalf("read game-clock external input %s: %v", input, err)
		}
		destination := filepath.Join(root, filepath.FromSlash(input))
		if err := os.MkdirAll(filepath.Dir(destination), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(destination, original, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	validatorSource, err := os.ReadFile(filepath.Join(repositoryRoot, "code/specs/fixtures/game-clock-v1/validate.py"))
	if err != nil {
		t.Fatalf("read game-clock corpus oracle: %v", err)
	}
	validatorPath := filepath.Join(root, "code/specs/fixtures/game-clock-v1/validate.py")
	if err := os.WriteFile(validatorPath, validatorSource, 0o644); err != nil {
		t.Fatal(err)
	}
	packageRoot := filepath.Join(root, "code/packages/rust/game-clock")
	exemptions := "EXCLUDED code/packages/rust  # synthetic Rust Cargo workspace root has no package target\n"
	if err := os.MkdirAll(filepath.Join(root, "code"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "code/BUILD-EXEMPTIONS"), []byte(exemptions), 0o644); err != nil {
		t.Fatal(err)
	}
	for _, entry := range []struct{ path, contents string }{
		{"Cargo.toml", "[package]\nname = \"game-clock\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"},
		{"BUILD", "echo game-clock-cache-probe\n"},
		{"src/lib.rs", "pub fn cache_probe() -> u8 { 1 }\n"},
	} {
		path := filepath.Join(packageRoot, filepath.FromSlash(entry.path))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(entry.contents), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	warrantRoot := filepath.Join(root, "code/packages/rust/warrant-grammar")
	for _, entry := range []struct{ path, contents string }{
		{"Cargo.toml", "[package]\nname = \"warrant-grammar\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"},
		{"BUILD", "echo warrant-grammar-cache-probe\n"},
		{"src/lib.rs", "pub fn cache_probe() -> u8 { 1 }\n"},
	} {
		path := filepath.Join(warrantRoot, filepath.FromSlash(entry.path))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(entry.contents), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	build := "echo game-clock-cache-probe"
	pkg := discovery.Package{
		Name: "rust/game-clock", Language: "rust", Path: packageRoot,
		BuildContent: build, BuildCommands: []string{build},
	}
	graph, _, err := resolver.ResolveWithTestTriggers([]discovery.Package{pkg})
	if err != nil {
		t.Fatalf("create game-clock cache graph: %v", err)
	}
	return root, pkg, graph
}

func runGameClockCacheProbe(t *testing.T, root string, pkg discovery.Package, graph *directedgraph.Graph) (string, string) {
	t.Helper()
	packageHash, err := hashPackageWithRepositoryInputs(pkg, root, hasher.HashPackage)
	if err != nil {
		t.Fatalf("hash game-clock package and repository inputs: %v", err)
	}
	packageHashes, depsHashes, failed, err := hashPackagesAndDependencies([]discovery.Package{pkg}, graph, func(candidate discovery.Package) (string, error) {
		return hashPackageWithRepositoryInputs(candidate, root, hasher.HashPackage)
	})
	if err != nil {
		t.Fatalf("hash cache inputs for %s: %v", failed, err)
	}
	cachePath := filepath.Join(root, ".build-cache.json")
	buildCache := cache.New()
	buildCache.Load(cachePath)
	results := executor.ExecuteBuilds([]discovery.Package{pkg}, graph, buildCache, packageHashes, depsHashes,
		false, false, 1, nil, nil, false)
	result, ok := results[pkg.Name]
	if !ok {
		t.Fatalf("executor omitted game-clock result: %#v", results)
	}
	if err := buildCache.Save(cachePath); err != nil {
		t.Fatal(err)
	}
	return result.Status, packageHash
}

func mutateGameClockInput(t *testing.T, path string, original []byte) []byte {
	t.Helper()
	var before, after string
	switch path {
	case ".cargo/config.toml", "code/packages/rust/.cargo/config.toml":
		before, after = `linker = "rust-lld"`, `linker = "lld-link"`
	case "code/packages/rust/Cargo.lock":
		before, after = "version = 4\n", "version = 3\n"
	case "code/packages/rust/Cargo.toml":
		before, after = `resolver = "2"`, `resolver = "1"`
	case "code/specs/fixtures/game-clock-v1/cases.json":
		before, after = `"id": "from_hz_60_rounding"`, `"id": "from_hz_61_rounding"`
	case "code/specs/game-clock.warrant":
		before, after = "s.fixed_dt_ns == 0u64", "s.fixed_dt_ns != 0u64"
	default:
		t.Fatalf("no same-length semantic mutation for %s", path)
	}
	mutated := bytes.Replace(original, []byte(before), []byte(after), 1)
	if bytes.Equal(mutated, original) {
		t.Fatalf("mutation text %q was not found in %s", before, path)
	}
	if len(mutated) != len(original) {
		t.Fatalf("mutation of %s changed length from %d to %d", path, len(original), len(mutated))
	}
	if strings.HasSuffix(path, ".json") && !json.Valid(mutated) {
		t.Fatalf("mutation of %s produced invalid JSON", path)
	}
	return mutated
}

func TestGameClockCacheHashesEveryExternalInput(t *testing.T) {
	root, pkg, graph := writeGameClockCacheFixture(t)
	status, originalHash := runGameClockCacheProbe(t, root, pkg, graph)
	if status != "built" {
		t.Fatalf("initial cache status = %q, want built", status)
	}
	if status, _ := runGameClockCacheProbe(t, root, pkg, graph); status != "skipped" {
		t.Fatalf("unchanged cache status = %q, want skipped", status)
	}
	for _, input := range gameClockInputPaths {
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		original := mustReadFile(t, inputPath)
		mutated := mutateGameClockInput(t, input, original)
		if err := os.WriteFile(inputPath, mutated, 0o644); err != nil {
			t.Fatal(err)
		}
		if input == "code/specs/fixtures/game-clock-v1/cases.json" {
			validateGameClockCorpus(t, root)
		}
		status, changedHash := runGameClockCacheProbe(t, root, pkg, graph)
		if status != "built" || changedHash == originalHash {
			t.Fatalf("external input %s cache status=%q hash_changed=%t", input, status, changedHash != originalHash)
		}
		if err := os.WriteFile(inputPath, original, 0o644); err != nil {
			t.Fatal(err)
		}
		status, restoredHash := runGameClockCacheProbe(t, root, pkg, graph)
		if status != "built" || restoredHash != originalHash {
			t.Fatalf("restored input %s status=%q hash_restored=%t", input, status, restoredHash == originalHash)
		}
		if status, _ := runGameClockCacheProbe(t, root, pkg, graph); status != "skipped" {
			t.Fatalf("restored input %s unchanged cache status = %q, want skipped", input, status)
		}
	}
}

func TestGameClockRustFilteredPlanEmitAndReplaySelectsAllInputs(t *testing.T) {
	root, _, _ := writeGameClockCacheFixture(t)
	for _, args := range [][]string{
		{"init", "-q"},
		{"config", "user.email", "build-tool-test@example.invalid"},
		{"config", "user.name", "Build Tool Test"},
		{"add", "."},
		{"commit", "-qm", "game-clock fixture baseline"},
	} {
		gameClockGit(t, root, args...)
	}
	base := strings.TrimSpace(gameClockGitOutput(t, root, "rev-parse", "HEAD"))

	for _, input := range gameClockInputPaths {
		gameClockGit(t, root, "reset", "--hard", base)
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		original := mustReadFile(t, inputPath)
		mutated := mutateGameClockInput(t, input, original)
		if err := os.WriteFile(inputPath, mutated, 0o644); err != nil {
			t.Fatal(err)
		}
		if input == "code/specs/fixtures/game-clock-v1/cases.json" {
			validateGameClockCorpus(t, root)
		}
		gameClockGit(t, root, "add", input)
		gameClockGit(t, root, "commit", "-qm", "change "+input)

		planPath := filepath.Join(root, "game-clock-plan-"+strings.NewReplacer("/", "-", ".", "_").Replace(input)+".json")
		emitOutput := runBuildToolPlanProcess(t,
			"-root", root, "-language", "rust", "-diff-base", base,
			"-ci-gates=", "-emit-plan", planPath,
		)
		buildPlan, err := plan.Read(planPath)
		if err != nil {
			t.Fatalf("read Rust-filtered plan for %s: %v", input, err)
		}
		if !containsString(buildPlan.AffectedPackages, "rust/game-clock") || !buildPlan.LanguagesNeeded["rust"] {
			t.Fatalf("input %s did not select Rust game-clock: affected=%v languages=%v output=%q",
				input, buildPlan.AffectedPackages, buildPlan.LanguagesNeeded, emitOutput)
		}
		replayOutput := runBuildToolPlanProcess(t,
			"-root", root, "-language", "rust", "-plan-file", planPath,
			"-cache-file", filepath.Join(root, "game-clock-plan-cache.json"), "-dry-run",
		)
		if !strings.Contains(replayOutput, "Loaded plan:") || !strings.Contains(replayOutput, "rust/game-clock") {
			t.Fatalf("Rust-filtered plan replay omitted rust/game-clock for %s: %s", input, replayOutput)
		}
		t.Logf("Rust-filtered plan input=%s affected=%v replay=%q", input, buildPlan.AffectedPackages, replayOutput)
	}
}

func TestGameClockCLIFallbackCacheUsesEveryExternalInput(t *testing.T) {
	root, _, _ := writeGameClockCacheFixture(t)
	for _, args := range [][]string{
		{"init", "-q"},
		{"config", "user.email", "build-tool-test@example.invalid"},
		{"config", "user.name", "Build Tool Test"},
		{"add", "."},
		{"commit", "-qm", "game-clock CLI cache baseline"},
	} {
		gameClockGit(t, root, args...)
	}
	cachePath := filepath.Join(root, "persisted-game-clock-cache.json")
	run := func() string {
		t.Helper()
		output := runBuildToolPlanProcess(t,
			"-root", root, "-language", "rust",
			"-plan-file", filepath.Join(root, "missing-plan.json"),
			"-diff-base", "missing-diff-base", "-ci-gates=",
			"-cache-file", cachePath,
		)
		if !strings.Contains(output, "Falling back to normal discovery flow") {
			t.Fatalf("CLI did not enter normal cache fallback flow: %s", output)
		}
		return output
	}
	statusOfClock := func(output string) string {
		t.Helper()
		for _, line := range strings.Split(output, "\n") {
			fields := strings.Fields(line)
			if len(fields) >= 2 && fields[0] == "rust/game-clock" {
				return strings.ToLower(fields[1])
			}
		}
		t.Fatalf("CLI report omitted rust/game-clock: %s", output)
		return ""
	}
	if status := statusOfClock(run()); status != "built" {
		t.Fatalf("initial CLI fallback status = %q, want built", status)
	}
	if status := statusOfClock(run()); status != "skipped" {
		t.Fatalf("unchanged CLI fallback status = %q, want skipped", status)
	}
	for _, input := range gameClockInputPaths {
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		original := mustReadFile(t, inputPath)
		mutated := mutateGameClockInput(t, input, original)
		if err := os.WriteFile(inputPath, mutated, 0o644); err != nil {
			t.Fatal(err)
		}
		if input == "code/specs/fixtures/game-clock-v1/cases.json" {
			validateGameClockCorpus(t, root)
		}
		if status := statusOfClock(run()); status != "built" {
			t.Fatalf("same-length mutation of %s CLI fallback status = %q, want built", input, status)
		}
		if err := os.WriteFile(inputPath, original, 0o644); err != nil {
			t.Fatal(err)
		}
		if status := statusOfClock(run()); status != "built" {
			t.Fatalf("restoring %s CLI fallback status = %q, want built", input, status)
		}
		if status := statusOfClock(run()); status != "skipped" {
			t.Fatalf("restored %s CLI fallback warm status = %q, want skipped", input, status)
		}
	}
}

func gameClockGit(t *testing.T, root string, args ...string) {
	t.Helper()
	command := exec.Command("git", args...)
	command.Dir = root
	if output, err := command.CombinedOutput(); err != nil {
		t.Fatalf("git %v: %v\n%s", args, err, output)
	}
}

func gameClockGitOutput(t *testing.T, root string, args ...string) string {
	t.Helper()
	command := exec.Command("git", args...)
	command.Dir = root
	output, err := command.Output()
	if err != nil {
		t.Fatalf("git %v: %v", args, err)
	}
	return string(output)
}

func validateGameClockCorpus(t *testing.T, root string) {
	t.Helper()
	validator := filepath.Join(root, "code/specs/fixtures/game-clock-v1/validate.py")
	command := exec.Command("python3", validator)
	output, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("validate temporary game-clock corpus: %v\n%s", err, output)
	}
}

func TestGameClockSelectorAndCanonicalBoundaryShareExactInputs(t *testing.T) {
	if !sort.StringsAreSorted(gameClockInputPaths) {
		t.Fatalf("Rust game-clock inputs are not sorted: %v", gameClockInputPaths)
	}
	selectorInputs := repositoryInputsByPackage()["rust/game-clock"]
	if selectorInputs == nil || !reflect.DeepEqual(selectorInputs.paths, gameClockInputPaths) {
		t.Fatalf("Rust game-clock selector inputs = %#v, want exact hash set %v", selectorInputs, gameClockInputPaths)
	}
	for _, input := range gameClockInputPaths {
		if !isGameClockInput(input) {
			t.Fatalf("Rust game-clock selector rejected exact input %s", input)
		}
	}
	for _, input := range []string{
		".cargo/config.toml.bak", "code/packages/rust/Cargo.lock.bak",
		"code/packages/rust/game-clock/../../Cargo.toml",
		"code/specs/fixtures/game-clock-v10/cases.json", "code/specs/game-clock.warrant.bak",
	} {
		if isGameClockInput(input) {
			t.Errorf("Rust game-clock selector accepted lookalike %s", input)
		}
	}

	repositoryRoot := toolchainFixtureRepoRoot(t)
	data, err := os.ReadFile(filepath.Join(repositoryRoot, "code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json"))
	if err != nil {
		t.Fatal(err)
	}
	var boundary graphdiff.RepositoryBoundary
	if err := json.Unmarshal(data, &boundary); err != nil {
		t.Fatal(err)
	}
	const packageRoot = "code/packages/rust/game-clock"
	boundaryPaths := make(map[string]bool)
	for _, rule := range boundary.Boundaries {
		applies := false
		for _, root := range rule.AppliesTo.ExactRoots {
			applies = applies || root == packageRoot
		}
		for _, root := range rule.AppliesTo.DescendantRoots {
			applies = applies || strings.HasPrefix(packageRoot, root+"/")
		}
		for _, excluded := range rule.AppliesTo.ExcludedRoots {
			if packageRoot == excluded || strings.HasPrefix(packageRoot, excluded+"/") {
				applies = false
			}
		}
		if applies {
			for _, input := range rule.Inputs {
				boundaryPaths[input.Path] = true
			}
		}
	}
	var got []string
	for path := range boundaryPaths {
		got = append(got, path)
	}
	sort.Strings(got)
	if !reflect.DeepEqual(got, gameClockInputPaths) {
		t.Fatalf("canonical boundary paths for %s = %v, want exact six-input set %v", packageRoot, got, gameClockInputPaths)
	}
}
