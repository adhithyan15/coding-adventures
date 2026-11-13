package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
	starlarkeval "github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/starlark"
)

// stepsFixture builds a repository with the three kinds of package the
// recipe check has to cope with:
//
//	go/plain   a shell BUILD whose lines run as written
//	swift/sw   a shell BUILD whose `swift build` is rewritten on Linux
//	go/star    a Starlark BUILD: its lines exist only after evaluation
func stepsFixture(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	write := func(rel, content string) {
		t.Helper()
		full := filepath.Join(root, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(full, []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	write("code/packages/go/plain/BUILD", "go build ./...\ngo test ./...\n")
	write("code/packages/go/plain/go.mod", "module plain\n\ngo 1.26\n")
	write("code/packages/swift/sw/BUILD", "swift build\nswift test\n")
	write("code/packages/swift/sw/Package.swift", "// swift-tools-version:5.9\n")
	write("code/packages/go/star/BUILD", "load(\"code/packages/starlark/library-rules/go_library.star\", \"go_library\")\n\n"+
		"_targets = [\n    go_library(\n        name = \"star\",\n        srcs = [\"**/*.go\"],\n        deps = [],\n    ),\n]\n")
	write("code/packages/go/star/go.mod", "module star\n\ngo 1.26\n")
	// The Starlark rule file is the real one: the evaluator loads it from
	// the fixture root exactly as it does from the repository root.
	rule, err := os.ReadFile(filepath.Join("..", "..", "..", "packages", "starlark", "library-rules", "go_library.star"))
	if err != nil {
		t.Fatal(err)
	}
	write("code/packages/starlark/library-rules/go_library.star", string(rule))
	return root
}

// stepsExport discovers root the way main.go does (all platforms, Starlark
// evaluated) and writes the steps export to a fresh path. The full CLI cannot
// run on a fixture tree: -emit-kiln-graph insists on the real repository's
// selector relations (see TestUnknownRepositoryInputsAreFlagged).
func stepsExport(t *testing.T, root string) string {
	t.Helper()
	pkgs, err := discovery.DiscoverPackagesAllPlatforms(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	for i := range pkgs {
		pkg := &pkgs[i]
		if !starlarkeval.IsStarlarkBuild(pkg.BuildContent) {
			continue
		}
		result, err := starlarkeval.EvaluateBuildFile(filepath.Join(pkg.Path, "BUILD"), pkg.Path, root)
		if err != nil || len(result.Targets) == 0 {
			t.Fatalf("evaluating %s: %v", pkg.Name, err)
		}
		pkg.IsStarlark = true
		pkg.BuildCommands = starlarkeval.GenerateCommands(result.Targets[0])
	}
	path := filepath.Join(t.TempDir(), "s.json")
	if code := emitKilnLegacySteps(pkgs, root, path); code != 0 {
		t.Fatalf("emitKilnLegacySteps exited %d", code)
	}
	return path
}

func readSteps(t *testing.T, path string) kilnLegacySteps {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var s kilnLegacySteps
	if err := json.Unmarshal(data, &s); err != nil {
		t.Fatal(err)
	}
	return s
}

func stepsByPackage(s kilnLegacySteps) map[string][]string {
	m := map[string][]string{}
	for _, p := range s.Packages {
		m[p.Package] = p.Steps
	}
	return m
}

// The export equals EffectiveStepLines for each package of the same
// discovery, including the rewritten Swift line and the evaluated Starlark.
func TestEmitKilnStepsEqualsEffectiveStepLines(t *testing.T) {
	root := stepsFixture(t)
	got := readSteps(t, stepsExport(t, root))
	if got.Schema != "kiln-legacy-steps/1" || got.OS != runtime.GOOS {
		t.Fatalf("header = %q %q", got.Schema, got.OS)
	}
	by := stepsByPackage(got)
	if len(by) != 3 {
		t.Fatalf("packages = %v", by)
	}

	// Plain and Swift: recompute from a fresh discovery.
	pkgs, err := discovery.DiscoverPackagesAllPlatforms(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	for _, pkg := range pkgs {
		rel, _ := filepath.Rel(root, pkg.Path)
		rel = filepath.ToSlash(rel)
		if rel == "code/packages/go/star" {
			continue
		}
		if want := executor.EffectiveStepLines(pkg); !reflect.DeepEqual(by[rel], want) {
			t.Errorf("%s: steps %q, want %q", rel, by[rel], want)
		}
	}
	if want := []string{"go build ./...", "go test ./..."}; !reflect.DeepEqual(by["code/packages/go/plain"], want) {
		t.Errorf("plain = %q", by["code/packages/go/plain"])
	}
	swift := by["code/packages/swift/sw"]
	if len(swift) != 2 {
		t.Fatalf("swift = %q", swift)
	}
	if runtime.GOOS == "linux" && !strings.Contains(swift[0], "--build-system native") {
		t.Errorf("the Swift line was not rewritten on Linux: %q", swift)
	}

	// Starlark: the lines come from evaluation, not from the BUILD text.
	star := by["code/packages/go/star"]
	if len(star) == 0 {
		t.Fatal("the Starlark package has no steps")
	}
	for _, l := range star {
		if strings.Contains(l, "load(") || strings.Contains(l, "_targets") {
			t.Errorf("a raw Starlark line leaked into the steps: %q", l)
		}
	}
}

func TestEmitKilnStepsNeedsTheGraphExport(t *testing.T) {
	root := stepsFixture(t)
	steps := filepath.Join(t.TempDir(), "s.json")
	code, stderr := runBuildTool(t, "-root", root, "-validate-build-files=false", "-emit-kiln-steps", steps)
	if code != 1 || !strings.Contains(stderr, "only valid together with -emit-kiln-graph") {
		t.Fatalf("exit %d: %s", code, stderr)
	}
	if _, err := os.Stat(steps); !os.IsNotExist(err) {
		t.Fatalf("a refused run wrote %s (%v)", steps, err)
	}
	code, stderr = runBuildTool(t, "-root", root, "-emit-kiln-graph", filepath.Join(t.TempDir(), "g.json"), "-emit-kiln-steps", "")
	if code != 1 || !strings.Contains(stderr, "needs a path") {
		t.Fatalf("empty path: exit %d: %s", code, stderr)
	}
}

func TestEmitKilnStepsIsByteStable(t *testing.T) {
	root := stepsFixture(t)
	var runs [2][]byte
	for i := range runs {
		var err error
		if runs[i], err = os.ReadFile(stepsExport(t, root)); err != nil {
			t.Fatal(err)
		}
	}
	if !bytes.Equal(runs[0], runs[1]) {
		t.Fatal("two runs of the same tree wrote different bytes")
	}
	if !bytes.HasSuffix(runs[0], []byte("}\n")) {
		t.Fatal("no trailing newline")
	}
}

// A stale file is removed before any work, and a symlink is refused.
func TestEmitKilnStepsFreshnessAndSymlink(t *testing.T) {
	root := stepsFixture(t)
	out := t.TempDir()
	steps := filepath.Join(out, "s.json")
	if err := os.WriteFile(steps, []byte("stale"), 0o644); err != nil {
		t.Fatal(err)
	}
	// An early failure (no packages under this root) leaves no stale file.
	empty := t.TempDir()
	if err := os.MkdirAll(filepath.Join(empty, "code"), 0o755); err != nil {
		t.Fatal(err)
	}
	code, _ := runBuildTool(t, "-root", empty, "-emit-kiln-graph", filepath.Join(out, "g.json"), "-emit-kiln-steps", steps)
	if code == 0 {
		t.Fatal("an empty tree must fail")
	}
	if _, err := os.Stat(steps); !os.IsNotExist(err) {
		t.Fatalf("stale file survived an early failure (%v)", err)
	}

	if runtime.GOOS == "windows" {
		return
	}
	link := filepath.Join(out, "link.json")
	if err := os.Symlink(filepath.Join(out, "elsewhere.json"), link); err != nil {
		t.Fatal(err)
	}
	code, stderr := runBuildTool(t, "-root", root, "-validate-build-files=false", "-emit-kiln-graph", filepath.Join(out, "g.json"), "-emit-kiln-steps", link)
	if code != 1 || !strings.Contains(stderr, "symlink") {
		t.Fatalf("exit %d: %s", code, stderr)
	}
}

func TestBuildKilnLegacyStepsSortedAndNonNull(t *testing.T) {
	root := t.TempDir()
	pkgs := []discovery.Package{
		{Name: "go/b", Path: filepath.Join(root, "code", "b")},
		{Name: "go/a", Path: filepath.Join(root, "code", "a"), BuildCommands: []string{"x"}},
	}
	s, err := buildKilnLegacySteps(pkgs, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	if s.Packages[0].Package != "code/a" || s.Packages[1].Package != "code/b" {
		t.Fatalf("not sorted: %+v", s.Packages)
	}
	data, err := marshalKilnLegacySteps(s)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(data), `"steps": []`) {
		t.Fatalf("a package without commands must encode [], got:\n%s", data)
	}
}

func TestEmitKilnLegacyStepsFailures(t *testing.T) {
	root := t.TempDir()
	pkgs := []discovery.Package{{Name: "go/a", Path: filepath.Join(root, "code", "a")}}
	// A relative root against an absolute path cannot be made relative.
	if code := emitKilnLegacySteps(pkgs, "relative", filepath.Join(root, "s.json")); code != 1 {
		t.Fatalf("rel failure: exit %d", code)
	}
	// The output path is a directory: the atomic writer refuses it.
	if code := emitKilnLegacySteps(pkgs, root, root); code != 1 {
		t.Fatalf("directory output: exit %d", code)
	}
	if code := emitKilnLegacySteps(pkgs, root, filepath.Join(root, "s.json")); code != 0 {
		t.Fatalf("exit %d", code)
	}
}
