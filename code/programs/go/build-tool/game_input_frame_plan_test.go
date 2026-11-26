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

// assertTypeScriptPlanLeavesOutRustDependents pins the prerequisite-closed
// language plan: the TypeScript consumer depends on Rust providers, so a
// TypeScript plan holds neither it nor any non-TypeScript package.  Before
// plan_language.go, -language did not narrow an emitted plan and this test
// expected the providers and the Rust toolchain to be retained.
func assertTypeScriptPlanLeavesOutRustDependents(t *testing.T, bp *plan.BuildPlan) {
	t.Helper()
	for _, entry := range bp.Packages {
		if entry.Language != "typescript" || entry.Name == gameInputFrameCacheConsumer {
			t.Fatalf("TypeScript plan kept %s (%s) despite Rust prerequisites", entry.Name, entry.Language)
		}
	}
	if containsString(bp.AffectedPackages, gameInputFrameCacheConsumer) || bp.LanguagesNeeded["rust"] {
		t.Fatalf("TypeScript plan schedules the excluded consumer or Rust: affected=%v languages=%v",
			bp.AffectedPackages, bp.LanguagesNeeded)
	}
}

func TestTypeScriptLanguageFilteredPlanLeavesOutRustDependentConsumer(t *testing.T) {
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
		// Selection still works for a TypeScript run: a normal (non-plan)
		// -language typescript run selects the consumer from the changed input.
		dryRun := runBuildToolPlanProcess(t,
			"-root", root, "-language", "typescript", "-diff-base", base, "-ci-gates=",
			"-cache-file", filepath.Join(root, "input-cache-"+strings.ReplaceAll(strings.Trim(input, "/."), "/", "-")+".json"), "-dry-run",
		)
		if !strings.Contains(dryRun, gameInputFrameCacheConsumer) {
			t.Fatalf("input %s did not select the TS consumer: %s", input, dryRun)
		}
		// An emitted TypeScript plan is prerequisite-closed (plan_language.go):
		// the consumer has Rust prerequisites, so it is left out, and nothing
		// of another language is pulled in.
		planPath := filepath.Join(root, "input-plan-"+strings.ReplaceAll(strings.Trim(input, "/."), "/", "-")+".json")
		runBuildToolPlanProcess(t,
			"-root", root, "-language", "typescript", "-diff-base", base, "-ci-gates=", "-emit-plan", planPath,
		)
		bp, err := plan.Read(planPath)
		if err != nil {
			t.Fatalf("read emitted plan for %s: %v", input, err)
		}
		assertTypeScriptPlanLeavesOutRustDependents(t, bp)
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
	bp, err := plan.Read(planPath)
	if err != nil {
		t.Fatal(err)
	}
	assertTypeScriptPlanLeavesOutRustDependents(t, bp)

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
	if strings.Contains(fallbackOutput, "RUN rust/") || strings.Contains(fallbackOutput, "Would run: rust/") {
		t.Fatalf("language filter executed a Rust provider:\n%s", fallbackOutput)
	}

	forceTypeScriptPath := filepath.Join(root, "force-typescript-plan.json")
	runBuildToolPlanProcess(t, "-root", root, "-force", "-language", "typescript", "-ci-gates=", "-emit-plan", forceTypeScriptPath)
	forceTypeScriptPlan, err := plan.Read(forceTypeScriptPath)
	if err != nil {
		t.Fatal(err)
	}
	assertTypeScriptPlanLeavesOutRustDependents(t, forceTypeScriptPlan)

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
