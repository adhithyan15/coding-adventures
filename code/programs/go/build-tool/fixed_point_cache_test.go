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

func writeFixedPointCacheFixture(t *testing.T) (string, discovery.Package, *directedgraph.Graph) {
	t.Helper()
	root := t.TempDir()
	repositoryRoot := toolchainFixtureRepoRoot(t)
	copyFixedPointReaderInputs(t, repositoryRoot, root)
	for _, input := range []string{"code/specs/fixtures/game-clock-v1/cases.json", "code/specs/game-clock.warrant"} {
		original, err := os.ReadFile(filepath.Join(repositoryRoot, filepath.FromSlash(input)))
		if err != nil {
			t.Fatal(err)
		}
		destination := filepath.Join(root, filepath.FromSlash(input))
		if err := os.MkdirAll(filepath.Dir(destination), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(destination, original, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	clockRoot := filepath.Join(root, "code/packages/rust/game-clock")
	for _, entry := range []struct{ path, contents string }{
		{"Cargo.toml", "[package]\nname = \"game-clock\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"},
		{"BUILD", "echo game-clock-cache-probe\n"},
		{"src/lib.rs", "pub fn cache_probe() -> u8 { 1 }\n"},
	} {
		path := filepath.Join(clockRoot, filepath.FromSlash(entry.path))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(entry.contents), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	exemptions := "EXCLUDED code/packages/rust  # synthetic Rust Cargo workspace root has no package target\n"
	if err := os.MkdirAll(filepath.Join(root, "code"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "code/BUILD-EXEMPTIONS"), []byte(exemptions), 0o644); err != nil {
		t.Fatal(err)
	}
	ensureFixedPointReaderPackages(t, root)
	packageRoot := filepath.Join(root, "code/packages/rust/fixed-point")
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
	build := "echo fixed-point-cache-probe"
	pkg := discovery.Package{
		Name: "rust/fixed-point", Language: "rust", Path: packageRoot,
		BuildContent: build, BuildCommands: []string{build},
	}
	graph, _, err := resolver.ResolveWithTestTriggers([]discovery.Package{pkg})
	if err != nil {
		t.Fatalf("create fixed-point cache graph: %v", err)
	}
	return root, pkg, graph
}

func copyFixedPointReaderInputs(t *testing.T, repositoryRoot, root string) {
	t.Helper()
	inputs := append(append(append(append([]string{}, fixedPointInputPaths...), fixedPointReporterInputs...), fixedPointWarrantIRInputs...), warrantAssurancePaths...)
	seen := map[string]bool{}
	for _, input := range inputs {
		if seen[input] {
			continue
		}
		seen[input] = true
		copyFixedPointFixturePath(t, repositoryRoot, root, input)
	}
}

func ensureFixedPointReaderPackages(t *testing.T, root string) {
	t.Helper()
	for _, packageName := range []string{"fixed-point", "ca-assurance-report", "warrant-ir"} {
		packageRoot := filepath.Join(root, "code/packages/rust", packageName)
		if err := os.MkdirAll(packageRoot, 0o755); err != nil {
			t.Fatal(err)
		}
		if packageName == "fixed-point" || !fixedPointFixtureFileExists(filepath.Join(packageRoot, "Cargo.toml")) {
			contents := "[package]\nname = \"" + packageName + "\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
			if err := os.WriteFile(filepath.Join(packageRoot, "Cargo.toml"), []byte(contents), 0o644); err != nil {
				t.Fatal(err)
			}
		}
		if !fixedPointFixtureFileExists(filepath.Join(packageRoot, "BUILD")) {
			if err := os.WriteFile(filepath.Join(packageRoot, "BUILD"), []byte("echo "+packageName+"-cache-probe\n"), 0o644); err != nil {
				t.Fatal(err)
			}
		}
		if !fixedPointFixtureFileExists(filepath.Join(packageRoot, "src/lib.rs")) {
			if err := os.MkdirAll(filepath.Join(packageRoot, "src"), 0o755); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(filepath.Join(packageRoot, "src/lib.rs"), []byte("pub fn cache_probe() -> u8 { 1 }\n"), 0o644); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func fixedPointFixtureFileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

func copyFixedPointFixturePath(t *testing.T, fromRoot, toRoot, relative string) {
	t.Helper()
	from := filepath.Join(fromRoot, filepath.FromSlash(relative))
	to := filepath.Join(toRoot, filepath.FromSlash(relative))
	info, err := os.Stat(from)
	if err != nil {
		t.Fatalf("stat real fixed-point reader input %s: %v", relative, err)
	}
	if info.IsDir() {
		entries, err := os.ReadDir(from)
		if err != nil {
			t.Fatal(err)
		}
		if err := os.MkdirAll(to, 0o755); err != nil {
			t.Fatal(err)
		}
		for _, entry := range entries {
			copyFixedPointFixturePath(t, fromRoot, toRoot, filepath.ToSlash(filepath.Join(filepath.FromSlash(relative), entry.Name())))
		}
		return
	}
	contents, err := os.ReadFile(from)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Dir(to), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(to, contents, 0o644); err != nil {
		t.Fatal(err)
	}
}

func runFixedPointCacheProbe(t *testing.T, root string, pkg discovery.Package, graph *directedgraph.Graph) (string, string) {
	t.Helper()
	packageHash, err := hashPackageWithRepositoryInputs(pkg, root, hasher.HashPackage)
	if err != nil {
		t.Fatalf("hash fixed-point package and repository inputs: %v", err)
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
		t.Fatalf("executor omitted fixed-point result: %#v", results)
	}
	if err := buildCache.Save(cachePath); err != nil {
		t.Fatal(err)
	}
	return result.Status, packageHash
}

func mutateFixedPointInput(t *testing.T, path string, original []byte) []byte {
	t.Helper()
	var before, after string
	switch path {
	case ".cargo/config.toml", "code/packages/rust/.cargo/config.toml":
		before, after = `linker = "rust-lld"`, `linker = "lld-link"`
	case "code/packages/rust/Cargo.lock":
		before, after = "version = 4\n", "version = 3\n"
	case "code/packages/rust/Cargo.toml":
		before, after = `resolver = "2"`, `resolver = "1"`
	case "code/specs/fixtures/fixed-point-q16-v1/cases.json":
		before, after = `"id": "to-small-positive",
      "operation": "to_integer",
      "input": {
        "raw": 1`, `"id": "to-small-positive",
      "operation": "to_integer",
      "input": {
        "raw": 2`
	case "code/specs/fixed-point.warrant":
		before, after = "b.raw == 0i32", "b.raw != 0i32"
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

func TestFixedPointCacheHashesEveryExternalInput(t *testing.T) {
	root, pkg, graph := writeFixedPointCacheFixture(t)
	status, originalHash := runFixedPointCacheProbe(t, root, pkg, graph)
	if status != "built" {
		t.Fatalf("initial cache status = %q, want built", status)
	}
	if status, _ := runFixedPointCacheProbe(t, root, pkg, graph); status != "skipped" {
		t.Fatalf("unchanged cache status = %q, want skipped", status)
	}
	for _, control := range []string{
		".cargo/config.toml.bak",
		"code/packages/rust/.cargo/credentials.toml",
		"code/packages/rust/Cargo.lock.bak",
		"code/packages/rust/Cargo.toml.bak",
		"code/specs/fixed-point.warrant.bak",
		"code/specs/fixtures/fixed-point-q16-v10/cases.json",
	} {
		path := filepath.Join(root, filepath.FromSlash(control))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte("nearby untracked control\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		status, hash := runFixedPointCacheProbe(t, root, pkg, graph)
		if status != "skipped" || hash != originalHash {
			t.Fatalf("negative control %s changed cache: status=%q hash_changed=%t", control, status, hash != originalHash)
		}
	}
	for _, input := range fixedPointInputPaths {
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		original := mustReadFile(t, inputPath)
		mutated := mutateFixedPointInput(t, input, original)
		if err := os.WriteFile(inputPath, mutated, 0o644); err != nil {
			t.Fatal(err)
		}
		status, changedHash := runFixedPointCacheProbe(t, root, pkg, graph)
		if status != "built" || changedHash == originalHash {
			t.Fatalf("external input %s cache status=%q hash_changed=%t", input, status, changedHash != originalHash)
		}
		if err := os.WriteFile(inputPath, original, 0o644); err != nil {
			t.Fatal(err)
		}
		status, restoredHash := runFixedPointCacheProbe(t, root, pkg, graph)
		if status != "built" || restoredHash != originalHash {
			t.Fatalf("restored input %s status=%q hash_restored=%t", input, status, restoredHash == originalHash)
		}
		if status, _ := runFixedPointCacheProbe(t, root, pkg, graph); status != "skipped" {
			t.Fatalf("restored input %s unchanged cache status = %q, want skipped", input, status)
		}
	}
}

func TestFixedPointRustFilteredPlanEmitAndReplaySelectsAllInputs(t *testing.T) {
	root, _, _ := writeFixedPointCacheFixture(t)
	for _, args := range [][]string{
		{"init", "-q"},
		{"config", "user.email", "build-tool-test@example.invalid"},
		{"config", "user.name", "Build Tool Test"},
		{"add", "."},
		{"commit", "-qm", "fixed-point fixture baseline"},
	} {
		fixedPointGit(t, root, args...)
	}
	base := strings.TrimSpace(fixedPointGitOutput(t, root, "rev-parse", "HEAD"))

	for _, input := range fixedPointInputPaths {
		fixedPointGit(t, root, "reset", "--hard", base)
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		original := mustReadFile(t, inputPath)
		mutated := mutateFixedPointInput(t, input, original)
		if err := os.WriteFile(inputPath, mutated, 0o644); err != nil {
			t.Fatal(err)
		}
		fixedPointGit(t, root, "add", input)
		fixedPointGit(t, root, "commit", "-qm", "change "+input)

		planPath := filepath.Join(root, "fixed-point-plan-"+strings.NewReplacer("/", "-", ".", "_").Replace(input)+".json")
		emitOutput := runBuildToolPlanProcess(t,
			"-root", root, "-language", "rust", "-diff-base", base,
			"-ci-gates=", "-emit-plan", planPath,
		)
		buildPlan, err := plan.Read(planPath)
		if err != nil {
			t.Fatalf("read Rust-filtered plan for %s: %v", input, err)
		}
		wantAffected := []string{"rust/fixed-point"}
		switch input {
		case ".cargo/config.toml", "code/packages/rust/.cargo/config.toml", "code/packages/rust/Cargo.lock", "code/packages/rust/Cargo.toml":
			wantAffected = []string{"rust/ca-assurance-report", "rust/fixed-point", "rust/game-clock", "rust/warrant-ir"}
		case "code/specs/fixed-point.warrant":
			wantAffected = []string{"rust/ca-assurance-report", "rust/fixed-point", "rust/warrant-grammar", "rust/warrant-ir"}
		}
		if !reflect.DeepEqual(buildPlan.AffectedPackages, wantAffected) || !buildPlan.LanguagesNeeded["rust"] {
			t.Fatalf("input %s affected=%v, want exact consumers %v; languages=%v output=%q",
				input, buildPlan.AffectedPackages, wantAffected, buildPlan.LanguagesNeeded, emitOutput)
		}
		replayOutput := runBuildToolPlanProcess(t,
			"-root", root, "-language", "rust", "-plan-file", planPath,
			"-cache-file", filepath.Join(root, "fixed-point-plan-cache.json"), "-dry-run",
		)
		if !strings.Contains(replayOutput, "Loaded plan:") {
			t.Fatalf("Rust-filtered plan replay failed for %s: %s", input, replayOutput)
		}
		for _, selected := range wantAffected {
			if !strings.Contains(replayOutput, selected) {
				t.Fatalf("Rust-filtered plan replay omitted %s for %s: %s", selected, input, replayOutput)
			}
		}
		t.Logf("Rust-filtered plan input=%s affected=%v replay=%q", input, buildPlan.AffectedPackages, replayOutput)
	}
}

func TestFixedPointVerifierInputsSelectExactConsumersInRustPlans(t *testing.T) {
	root, _, _ := writeFixedPointCacheFixture(t)
	for _, args := range [][]string{
		{"init", "-q"},
		{"config", "user.email", "build-tool-test@example.invalid"},
		{"config", "user.name", "Build Tool Test"},
		{"add", "."},
		{"commit", "-qm", "fixed-point verifier fixture baseline"},
	} {
		fixedPointGit(t, root, args...)
	}
	base := strings.TrimSpace(fixedPointGitOutput(t, root, "rev-parse", "HEAD"))
	probes := []struct {
		path, old, changed string
		want               []string
	}{
		{"code/packages/rust/fixed-point/src/lib.rs", "const SCALE: i128 = 65_536;", "const SCALE: i128 = 65_535;", []string{"rust/ca-assurance-report", "rust/fixed-point", "rust/warrant-ir"}},
		{"code/packages/rust/fixed-point/evidence/proof-manifest.json", "\"max_len\": 0", "\"max_len\": 1", []string{"rust/ca-assurance-report", "rust/fixed-point"}},
		{"code/packages/rust/fixed-point/evidence.bindings", "fuzz_iterations 4096", "fuzz_iterations 4097", []string{"rust/ca-assurance-report", "rust/fixed-point", "rust/warrant-ir"}},
	}
	for _, probe := range probes {
		fixedPointGit(t, root, "reset", "--hard", base)
		path := filepath.Join(root, filepath.FromSlash(probe.path))
		original := mustReadFile(t, path)
		changed := bytes.Replace(original, []byte(probe.old), []byte(probe.changed), 1)
		if bytes.Equal(changed, original) || len(changed) != len(original) {
			t.Fatalf("selection probe %s was not a same-length mutation", probe.path)
		}
		if err := os.WriteFile(path, changed, 0o644); err != nil {
			t.Fatal(err)
		}
		fixedPointGit(t, root, "add", probe.path)
		fixedPointGit(t, root, "commit", "-qm", "change "+probe.path)
		planPath := filepath.Join(root, "reader-plan-"+strings.ReplaceAll(strings.ReplaceAll(probe.path, "/", "-"), ".", "_")+".json")
		output := runBuildToolPlanProcess(t, "-root", root, "-language", "rust", "-diff-base", base, "-ci-gates=", "-emit-plan", planPath)
		buildPlan, err := plan.Read(planPath)
		if err != nil {
			t.Fatalf("read reader plan for %s: %v", probe.path, err)
		}
		if !reflect.DeepEqual(buildPlan.AffectedPackages, probe.want) {
			t.Fatalf("path %s selected %v, want %v; output=%s", probe.path, buildPlan.AffectedPackages, probe.want, output)
		}
		replay := runBuildToolPlanProcess(t, "-root", root, "-language", "rust", "-plan-file", planPath,
			"-cache-file", filepath.Join(root, "reader-cache.json"), "-dry-run")
		if !strings.Contains(replay, "Loaded plan:") {
			t.Fatalf("reader plan did not replay for %s: %s", probe.path, replay)
		}
		for _, selected := range probe.want {
			if !strings.Contains(replay, selected) {
				t.Fatalf("replayed plan omitted %s for %s: %s", selected, probe.path, replay)
			}
		}
		t.Logf("fixed-point reader selection path=%s affected=%v", probe.path, buildPlan.AffectedPackages)
	}
}

func TestFixedPointCLIFallbackCacheUsesEveryExternalInput(t *testing.T) {
	root, _, _ := writeFixedPointCacheFixture(t)
	for _, args := range [][]string{
		{"init", "-q"},
		{"config", "user.email", "build-tool-test@example.invalid"},
		{"config", "user.name", "Build Tool Test"},
		{"add", "."},
		{"commit", "-qm", "fixed-point CLI cache baseline"},
	} {
		fixedPointGit(t, root, args...)
	}
	cachePath := filepath.Join(root, "persisted-fixed-point-cache.json")
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
	statusOfFixedPoint := func(output string) string {
		t.Helper()
		for _, line := range strings.Split(output, "\n") {
			fields := strings.Fields(line)
			if len(fields) >= 2 && fields[0] == "rust/fixed-point" {
				return strings.ToLower(fields[1])
			}
		}
		t.Fatalf("CLI report omitted rust/fixed-point: %s", output)
		return ""
	}
	if status := statusOfFixedPoint(run()); status != "built" {
		t.Fatalf("initial CLI fallback status = %q, want built", status)
	}
	if status := statusOfFixedPoint(run()); status != "skipped" {
		t.Fatalf("unchanged CLI fallback status = %q, want skipped", status)
	}
	for _, input := range fixedPointInputPaths {
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		original := mustReadFile(t, inputPath)
		mutated := mutateFixedPointInput(t, input, original)
		if err := os.WriteFile(inputPath, mutated, 0o644); err != nil {
			t.Fatal(err)
		}
		if status := statusOfFixedPoint(run()); status != "built" {
			t.Fatalf("same-length mutation of %s CLI fallback status = %q, want built", input, status)
		}
		if err := os.WriteFile(inputPath, original, 0o644); err != nil {
			t.Fatal(err)
		}
		if status := statusOfFixedPoint(run()); status != "built" {
			t.Fatalf("restoring %s CLI fallback status = %q, want built", input, status)
		}
		if status := statusOfFixedPoint(run()); status != "skipped" {
			t.Fatalf("restored %s CLI fallback warm status = %q, want skipped", input, status)
		}
	}
}

func fixedPointGit(t *testing.T, root string, args ...string) {
	t.Helper()
	command := exec.Command("git", args...)
	command.Dir = root
	if output, err := command.CombinedOutput(); err != nil {
		t.Fatalf("git %v: %v\n%s", args, err, output)
	}
}

func fixedPointGitOutput(t *testing.T, root string, args ...string) string {
	t.Helper()
	command := exec.Command("git", args...)
	command.Dir = root
	output, err := command.Output()
	if err != nil {
		t.Fatalf("git %v: %v", args, err)
	}
	return string(output)
}

func TestFixedPointSelectorAndCanonicalBoundaryShareExactInputs(t *testing.T) {
	if !sort.StringsAreSorted(fixedPointInputPaths) {
		t.Fatalf("fixed-point inputs are not sorted: %v", fixedPointInputPaths)
	}
	selectorInputs := repositoryInputsByPackage()["rust/fixed-point"]
	if selectorInputs == nil || !reflect.DeepEqual(selectorInputs.paths, fixedPointInputPaths) {
		t.Fatalf("selector inputs = %#v, want %v", selectorInputs, fixedPointInputPaths)
	}
	data, err := os.ReadFile(filepath.Join(toolchainFixtureRepoRoot(t), "code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json"))
	if err != nil {
		t.Fatal(err)
	}
	var boundary graphdiff.RepositoryBoundary
	if err := json.Unmarshal(data, &boundary); err != nil {
		t.Fatal(err)
	}
	const packageRoot = "code/packages/rust/fixed-point"
	paths := map[string]bool{}
	for _, rule := range boundary.Boundaries {
		applies := false
		for _, root := range rule.AppliesTo.ExactRoots {
			applies = applies || root == packageRoot
		}
		for _, root := range rule.AppliesTo.DescendantRoots {
			applies = applies || strings.HasPrefix(packageRoot, root+"/")
		}
		for _, root := range rule.AppliesTo.ExcludedRoots {
			if packageRoot == root || strings.HasPrefix(packageRoot, root+"/") {
				applies = false
			}
		}
		if applies {
			for _, input := range rule.Inputs {
				paths[input.Path] = true
			}
		}
	}
	var got []string
	for path := range paths {
		got = append(got, path)
	}
	sort.Strings(got)
	if !reflect.DeepEqual(got, fixedPointInputPaths) {
		t.Fatalf("canonical boundary paths=%v, want %v", got, fixedPointInputPaths)
	}
}
