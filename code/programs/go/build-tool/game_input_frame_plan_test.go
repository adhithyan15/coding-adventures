package main

import (
	"encoding/json"
	"flag"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
)

const planProcessArgsEnv = "CODING_ADVENTURES_BUILD_TOOL_TEST_ARGS"

// TestBuildToolPlanProcessHelper runs the production CLI in a subprocess so
// the test covers the real flag, discovery, emit, replay, and stale-plan paths.
func TestBuildToolPlanProcessHelper(t *testing.T) {
	encoded := os.Getenv(planProcessArgsEnv)
	if encoded == "" {
		return
	}
	var args []string
	if err := json.Unmarshal([]byte(encoded), &args); err != nil {
		t.Fatalf("decode subprocess arguments: %v", err)
	}
	flag.CommandLine = flag.NewFlagSet("build-tool", flag.ExitOnError)
	os.Args = append([]string{"build-tool"}, args...)
	os.Exit(runWithPackageHasher(hasher.HashPackage))
}

func runBuildToolPlanProcess(t *testing.T, args ...string) string {
	t.Helper()
	command := exec.Command(os.Args[0], "-test.run=^TestBuildToolPlanProcessHelper$")
	encoded, err := json.Marshal(args)
	if err != nil {
		t.Fatal(err)
	}
	command.Env = append(os.Environ(), planProcessArgsEnv+"="+string(encoded))
	output, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("build-tool subprocess %v failed: %v\n%s", args, err, output)
	}
	return string(output)
}

func TestTypeScriptLanguageFilteredPlanKeepsRustProvidersAndToolchain(t *testing.T) {
	root, _, _ := writeGameInputCacheFixture(t)
	for _, args := range [][]string{
		{"init", "-q"},
		{"config", "user.email", "build-tool-test@example.invalid"},
		{"config", "user.name", "Build Tool Test"},
		{"add", "."},
		{"commit", "-qm", "fixture baseline"},
	} {
		command := exec.Command("git", args...)
		command.Dir = root
		if output, err := command.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, output)
		}
	}
	baseCommand := exec.Command("git", "rev-parse", "HEAD")
	baseCommand.Dir = root
	baseBytes, err := baseCommand.Output()
	if err != nil {
		t.Fatalf("resolve fixture baseline: %v", err)
	}
	base := strings.TrimSpace(string(baseBytes))
	for _, input := range typescriptGameInputFrameInputs {
		reset := exec.Command("git", "reset", "--hard", base)
		reset.Dir = root
		if output, err := reset.CombinedOutput(); err != nil {
			t.Fatalf("reset fixture before %s probe: %v\n%s", input, err, output)
		}
		inputPath := filepath.Join(root, filepath.FromSlash(input))
		originalInput, err := os.ReadFile(inputPath)
		if err != nil {
			t.Fatalf("read plan input %s: %v", input, err)
		}
		mutatedInput := mutateTypeScriptGameInputFrameInput(t, input, originalInput)
		if len(mutatedInput) != len(originalInput) {
			t.Fatalf("plan input mutation %s changed byte length", input)
		}
		if err := os.WriteFile(inputPath, mutatedInput, 0o644); err != nil {
			t.Fatal(err)
		}
		for _, args := range [][]string{{"add", input}, {"commit", "-qm", "change " + input}} {
			command := exec.Command("git", args...)
			command.Dir = root
			if output, err := command.CombinedOutput(); err != nil {
				t.Fatalf("git %v: %v\n%s", args, err, output)
			}
		}
		planPath := filepath.Join(root, "input-plan-"+strings.ReplaceAll(strings.Trim(input, "/."), "/", "-")+".json")
		emitOutput := runBuildToolPlanProcess(t,
			"-root", root, "-language", "typescript", "-diff-base", base, "-ci-gates=", "-emit-plan", planPath,
		)
		bp, err := plan.Read(planPath)
		if err != nil {
			t.Fatalf("read emitted plan for %s: %v", input, err)
		}
		if !containsString(bp.AffectedPackages, gameInputFrameCacheConsumer) ||
			!bp.LanguagesNeeded["typescript"] || !bp.LanguagesNeeded["rust"] {
			t.Fatalf("input %s did not select TS consumer and its Rust toolchain: affected=%v languages=%v output=%s",
				input, bp.AffectedPackages, bp.LanguagesNeeded, emitOutput)
		}
		for _, provider := range []string{gameInputFrameCacheBridge, gameInputFrameCacheParser, gameInputFrameCacheWIR, gameInputFrameCacheSolver} {
			if !containsPlanPackage(bp.Packages, provider) {
				t.Fatalf("input %s emitted plan lost provider %s", input, provider)
			}
		}
		replayOutput := runBuildToolPlanProcess(t,
			"-root", root, "-language", "typescript", "-plan-file", planPath,
			"-cache-file", filepath.Join(root, "input-cache-"+strings.ReplaceAll(strings.Trim(input, "/."), "/", "-")+".json"), "-dry-run",
		)
		if !strings.Contains(replayOutput, "Loaded plan:") || !strings.Contains(replayOutput, gameInputFrameCacheConsumer) {
			t.Fatalf("input %s plan replay omitted TS consumer: %s", input, replayOutput)
		}
		t.Logf("actual external-input selector + plan replay: path=%s bytes_before=%d bytes_after=%d sha256_before=%s sha256_after=%s affected_ts=true providers=%d languages=%v replay=%q",
			input, len(originalInput), len(mutatedInput), digestHex(originalInput), digestHex(mutatedInput), len(bp.Packages), bp.LanguagesNeeded, replayOutput)
	}
	resetBaseline := exec.Command("git", "reset", "--hard", base)
	resetBaseline.Dir = root
	if output, err := resetBaseline.CombinedOutput(); err != nil {
		t.Fatalf("restore fixture baseline before proof-provider probe: %v\n%s", err, output)
	}
	bridge := filepath.Join(root, "code/packages/rust/typescript-warrant-ir/src/lib.rs")
	original, err := os.ReadFile(bridge)
	if err != nil {
		t.Fatal(err)
	}
	changed := strings.Replace(string(original), "{ 1 }", "{ 2 }", 1)
	if changed == string(original) || len(changed) != len(original) {
		t.Fatal("fixture provider mutation must be same-length")
	}
	if err := os.WriteFile(bridge, []byte(changed), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Logf("plan source mutation: path=code/packages/rust/typescript-warrant-ir/src/lib.rs bytes_before=%d bytes_after=%d sha256_before=%s sha256_after=%s diff_base=%s",
		len(original), len(changed), digestHex(original), digestHex([]byte(changed)), base)
	for _, args := range [][]string{{"add", "code/packages/rust/typescript-warrant-ir/src/lib.rs"}, {"commit", "-qm", "change Rust proof provider"}} {
		command := exec.Command("git", args...)
		command.Dir = root
		if output, err := command.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, output)
		}
	}

	planPath := filepath.Join(root, "build-plan.json")
	emitOutput := runBuildToolPlanProcess(t,
		"-root", root, "-language", "typescript", "-diff-base", base, "-ci-gates=", "-emit-plan", planPath,
	)
	if !strings.Contains(emitOutput, "Build plan written") {
		t.Fatalf("filtered emit did not report plan creation:\n%s", emitOutput)
	}
	t.Logf("actual filtered emit command: -language typescript -diff-base %s -emit-plan %s; output=%q", base, planPath, emitOutput)
	bp, err := plan.Read(planPath)
	if err != nil {
		t.Fatal(err)
	}
	entries := make(map[string]bool, len(bp.Packages))
	for _, entry := range bp.Packages {
		entries[entry.Name] = true
	}
	for _, name := range []string{gameInputFrameCacheConsumer, gameInputFrameCacheBridge, gameInputFrameCacheParser, gameInputFrameCacheWIR, gameInputFrameCacheSolver} {
		if !entries[name] {
			t.Fatalf("emitted TypeScript-filtered plan lost provider %s; package count=%d", name, len(bp.Packages))
		}
	}
	for _, edge := range [][2]string{
		{gameInputFrameCacheBridge, gameInputFrameCacheConsumer},
		{gameInputFrameCacheParser, gameInputFrameCacheBridge},
		{gameInputFrameCacheWIR, gameInputFrameCacheBridge},
		{gameInputFrameCacheSolver, gameInputFrameCacheBridge},
	} {
		if !containsPlanEdge(bp.DependencyEdges, edge) {
			t.Fatalf("emitted TypeScript-filtered plan lost provider edge %s -> %s", edge[0], edge[1])
		}
	}
	if !containsString(bp.AffectedPackages, gameInputFrameCacheConsumer) {
		t.Fatalf("Rust-only changed package did not select TypeScript consumer: affected=%v", bp.AffectedPackages)
	}
	if !bp.LanguagesNeeded["rust"] || !bp.LanguagesNeeded["typescript"] {
		t.Fatalf("emitted filtered plan omitted required toolchains: %#v", bp.LanguagesNeeded)
	}
	t.Logf("serialized plan providers=%d affected_typescript=%t languages_needed=%v", len(bp.Packages), containsString(bp.AffectedPackages, gameInputFrameCacheConsumer), bp.LanguagesNeeded)

	replayOutput := runBuildToolPlanProcess(t,
		"-root", root, "-language", "typescript", "-plan-file", planPath,
		"-cache-file", filepath.Join(root, "build-cache.json"), "-dry-run",
	)
	if !strings.Contains(replayOutput, "Loaded plan:") || !strings.Contains(replayOutput, gameInputFrameCacheConsumer) {
		t.Fatalf("actual plan replay omitted TS consumer:\n%s", replayOutput)
	}
	if strings.Contains(replayOutput, "RUN rust/") || strings.Contains(replayOutput, "Would run: rust/") {
		t.Fatalf("language filter executed a Rust provider instead of only retaining its hash inputs:\n%s", replayOutput)
	}
	t.Logf("actual plan replay command: -language typescript -plan-file %s; output=%q", planPath, replayOutput)

	stalePath := filepath.Join(root, "stale-plan.json")
	if err := os.WriteFile(stalePath, []byte("not a build plan"), 0o644); err != nil {
		t.Fatal(err)
	}
	fallbackOutput := runBuildToolPlanProcess(t,
		"-root", root, "-language", "typescript", "-plan-file", stalePath,
		"-cache-file", filepath.Join(root, "fallback-cache.json"), "-dry-run", "-diff-base", base,
	)
	if !strings.Contains(fallbackOutput, "Falling back to normal discovery flow") || !strings.Contains(fallbackOutput, gameInputFrameCacheConsumer) {
		t.Fatalf("stale-plan fallback omitted TS consumer selected by Rust provider change:\n%s", fallbackOutput)
	}
	t.Logf("actual stale-plan fallback command: -language typescript -plan-file %s -diff-base %s; output=%q", stalePath, base, fallbackOutput)

	forceTypeScriptPath := filepath.Join(root, "force-typescript-plan.json")
	runBuildToolPlanProcess(t, "-root", root, "-force", "-language", "typescript", "-ci-gates=", "-emit-plan", forceTypeScriptPath)
	forceTypeScriptPlan, err := plan.Read(forceTypeScriptPath)
	if err != nil {
		t.Fatal(err)
	}
	if !forceTypeScriptPlan.Force || !forceTypeScriptPlan.LanguagesNeeded["typescript"] || !forceTypeScriptPlan.LanguagesNeeded["rust"] || len(forceTypeScriptPlan.LanguagesNeeded) != 2 {
		t.Fatalf("filtered force plan languages = %v, want TypeScript + declared Rust only", forceTypeScriptPlan.LanguagesNeeded)
	}
	forceEntries := make(map[string]bool, len(forceTypeScriptPlan.Packages))
	for _, entry := range forceTypeScriptPlan.Packages {
		forceEntries[entry.Name] = true
	}
	if !forceEntries[gameInputFrameCacheBridge] || !forceEntries[gameInputFrameCacheConsumer] {
		t.Fatalf("filtered force plan lost provider universe: %d entries", len(forceEntries))
	}
	t.Logf("forced TypeScript plan retains %d package providers and selects toolchains %v", len(forceEntries), forceTypeScriptPlan.LanguagesNeeded)

	forceAllPath := filepath.Join(root, "force-all-plan.json")
	runBuildToolPlanProcess(t, "-root", root, "-force", "-ci-gates=", "-emit-plan", forceAllPath)
	forceAllPlan, err := plan.Read(forceAllPath)
	if err != nil {
		t.Fatal(err)
	}
	wantAllToolchains := make(map[string]bool, len(allToolchains))
	for _, toolchain := range allToolchains {
		wantAllToolchains[toolchain] = true
	}
	if !reflect.DeepEqual(forceAllPlan.LanguagesNeeded, wantAllToolchains) {
		t.Fatalf("unfiltered force plan toolchains = %v, want full set %v", forceAllPlan.LanguagesNeeded, wantAllToolchains)
	}
	t.Logf("unfiltered force plan retains full toolchain set size=%d", len(forceAllPlan.LanguagesNeeded))
}

func containsString(values []string, wanted string) bool {
	for _, value := range values {
		if value == wanted {
			return true
		}
	}
	return false
}

func containsPlanEdge(edges [][2]string, wanted [2]string) bool {
	for _, edge := range edges {
		if edge == wanted {
			return true
		}
	}
	return false
}

func containsPlanPackage(packages []plan.PackageEntry, wanted string) bool {
	for _, pkg := range packages {
		if pkg.Name == wanted {
			return true
		}
	}
	return false
}
