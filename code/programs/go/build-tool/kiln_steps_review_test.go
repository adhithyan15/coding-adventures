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
)

// Regression tests for the review of the steps export.

// buildDirs makes root/code/<name>/BUILD for each name and returns packages
// pointing at them.
func buildDirs(t *testing.T, root string, names ...string) []discovery.Package {
	t.Helper()
	var pkgs []discovery.Package
	for _, n := range names {
		dir := filepath.Join(root, "code", n)
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "BUILD"), []byte("x\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		pkgs = append(pkgs, discovery.Package{Name: "go/" + n, Path: dir})
	}
	return pkgs
}

func TestBuildKilnLegacyStepsSortedAndNonNull(t *testing.T) {
	root := t.TempDir()
	pkgs := buildDirs(t, root, "b", "a")
	pkgs[1].BuildCommands = []string{"x"}
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
	pkgs := buildDirs(t, root, "a")
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

// Two different recipes must not export to the same bytes: encoding/json would
// turn each invalid byte into U+FFFD. The export refuses the line instead.
func TestPoCStepsCollide(t *testing.T) {
	root := t.TempDir()
	pkgs := buildDirs(t, root, "x")
	for name, line := range map[string]string{
		"invalid byte A": "printf '\xff' > out",
		"invalid byte B": "printf '\xfe' > out",
		"U+FFFD":         "echo �",
		"empty line":     "",
	} {
		pkgs[0].BuildCommands = []string{"ok", line}
		if _, err := buildKilnLegacySteps(pkgs, root, "linux"); err == nil || !strings.Contains(err.Error(), "go/x") {
			t.Errorf("%s: accepted (%v)", name, err)
		}
	}
	pkgs[0].BuildCommands = []string{"echo é ✓"}
	if _, err := buildKilnLegacySteps(pkgs, root, "linux"); err != nil {
		t.Fatalf("valid UTF-8 refused: %v", err)
	}
}

// A package directory outside the repository root has a "../" relative path.
func TestStepsRefusePackageOutsideRoot(t *testing.T) {
	root := t.TempDir()
	outside := buildDirs(t, t.TempDir(), "elsewhere")
	if _, err := buildKilnLegacySteps(outside, root, "linux"); err == nil || !strings.Contains(err.Error(), "outside") {
		t.Fatalf("got %v", err)
	}
	// A name that merely starts with two dots is a legal directory.
	dots := buildDirs(t, root, "..dots")
	if _, err := buildKilnLegacySteps(dots, root, "linux"); err != nil {
		t.Fatalf("%v", err)
	}
}

// A package whose only BUILD file is BUILD_windows has no recipe on Linux or
// macOS: discovery across all platforms lists it, the export must not.
func TestStepsOmitPackagesWithoutABuildFileForThisOS(t *testing.T) {
	root := t.TempDir()
	dir := filepath.Join(root, "code", "winonly")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "BUILD_windows"), []byte("go build\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	pkgs := []discovery.Package{{Name: "go/winonly", Path: dir, BuildCommands: []string{"go build"}}}
	for _, goos := range []string{"linux", "darwin"} {
		s, err := buildKilnLegacySteps(pkgs, root, goos)
		if err != nil || len(s.Packages) != 0 {
			t.Errorf("%s: %+v %v", goos, s, err)
		}
	}
	s, err := buildKilnLegacySteps(pkgs, root, "windows")
	if err != nil || len(s.Packages) != 1 {
		t.Fatalf("windows: %+v %v", s, err)
	}
}

func TestSamePath(t *testing.T) {
	dir := t.TempDir()
	if !samePath(filepath.Join(dir, "a.json"), filepath.Join(dir, ".", "x", "..", "a.json")) {
		t.Error("equal after Clean")
	}
	if samePath(filepath.Join(dir, "a.json"), filepath.Join(dir, "b.json")) {
		t.Error("different files")
	}
	if !samePath("rel.json", "./rel.json") {
		t.Error("relative spellings of one file")
	}
}

// The whole CLI path of the two exports on a fixture tree. The test switch
// turns off only the check that the real repository's selector tables name
// discovered packages; it is a package variable, not reachable from the
// command line.
func TestEmitKilnGraphAndStepsThroughTheCLI(t *testing.T) {
	requireKilnConsumers = false
	defer func() { requireKilnConsumers = true }()

	root := stepsFixture(t)
	out := t.TempDir()
	runOnce := func(name string) ([]byte, []byte) {
		t.Helper()
		g, s := filepath.Join(out, name+"-g.json"), filepath.Join(out, name+"-s.json")
		code, stderr := runBuildTool(t, "-root", root, "-validate-build-files=false", "-emit-kiln-graph", g, "-emit-kiln-steps", s)
		if code != 0 {
			t.Fatalf("exit %d: %s", code, stderr)
		}
		gb, err := os.ReadFile(g)
		if err != nil {
			t.Fatal(err)
		}
		sb, err := os.ReadFile(s)
		if err != nil {
			t.Fatal(err)
		}
		return gb, sb
	}
	g1, s1 := runOnce("one")
	g2, s2 := runOnce("two")
	if !bytes.Equal(g1, g2) || !bytes.Equal(s1, s2) {
		t.Fatal("two CLI runs differ")
	}
	// Same discovery pass: three packages in both, and the same lines as the
	// direct export.
	var steps kilnLegacySteps
	if err := json.Unmarshal(s1, &steps); err != nil {
		t.Fatal(err)
	}
	if len(steps.Packages) != 3 || steps.OS != runtime.GOOS {
		t.Fatalf("%+v", steps)
	}
	direct := readSteps(t, stepsExport(t, root))
	if !reflect.DeepEqual(stepsByPackage(direct), stepsByPackage(steps)) {
		t.Fatalf("CLI steps %v != direct %v", stepsByPackage(steps), stepsByPackage(direct))
	}
	var graph kilnLegacyGraph
	if err := json.Unmarshal(g1, &graph); err != nil || len(graph.Packages) != 3 {
		t.Fatalf("graph: %v %d", err, len(graph.Packages))
	}
}

// -emit-kiln-graph and -emit-kiln-steps may not name the same file, however
// the path is spelled.
func TestEmitKilnStepsRefusesTheGraphPath(t *testing.T) {
	root := stepsFixture(t)
	dir := t.TempDir()
	p := filepath.Join(dir, "x.json")
	for _, other := range []string{p, filepath.Join(dir, ".", "sub", "..", "x.json")} {
		code, stderr := runBuildTool(t, "-root", root, "-emit-kiln-graph", p, "-emit-kiln-steps", other)
		if code != 1 || !strings.Contains(stderr, "same file") {
			t.Fatalf("exit %d: %s", code, stderr)
		}
	}
	if _, err := os.Stat(p); !os.IsNotExist(err) {
		t.Fatalf("a refused run wrote the file (%v)", err)
	}
}
