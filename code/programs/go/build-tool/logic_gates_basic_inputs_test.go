package main

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// LG02 4.3 (LGE-S1b): an edit to the basic-gates Warrant document, to any file of its fixture,
// to the drift script or to the script's tests must select rust/logic-gates, whose BUILD runs
// the guard; a lookalike path must select nothing.
func TestLogicGatesBasicInputsSelectTheCrate(t *testing.T) {
	// Every consumer a selection can name must be discoverable (the selector fails closed), so
	// the list carries the other consumers the shared spec path also selects.
	packages := []discovery.Package{
		{Name: "rust/logic-gates", Language: "rust"},
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/warrant-corpus-gen", Language: "rust"},
		{Name: "python/warrant-oracle", Language: "python"},
	}
	for _, changed := range []string{
		"code/specs/logic-gates.warrant",
		"code/specs/logic-gates-basic.warrant",
		"code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.json",
		"code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.profile",
		"code/specs/fixtures/logic-gates-basic-warrant-v2/chunks/000.txt",
		"code/scripts/logic_gates_basic_drift.py",
		"code/scripts/tests/test_logic_gates_basic_drift.py",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !got["rust/logic-gates"] {
			t.Errorf("%s selected %v, err %v; want rust/logic-gates", changed, got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{changed}, packages, "python"); err != nil || got["rust/logic-gates"] {
			t.Errorf("%s python-only selected %v, err %v", changed, got, err)
		}
	}
	for _, changed := range []string{
		"code/specs/logic-gates-basic.warrant.draft",
		"code/specs/logic-gates-basic2.warrant",
		"code/specs/logic-gates-basic.warrant/x",
		"code/specs/LOGIC-gates-basic.warrant",
		"docs/code/specs/logic-gates-basic.warrant",
		`code\specs\logic-gates-basic.warrant`,
		"code/specs/fixtures/logic-gates-basic-warrant-v2",
		"code/specs/fixtures/logic-gates-basic-warrant-v2/",
		"code/specs/fixtures/logic-gates-basic-warrant-v2x/corpus.json",
		"code/specs/fixtures/logic-gates-basic-warrant-v3/corpus.json",
		"code/specs/fixtures/logic-gates-basic-warrant-v2/../x.json",
		"code/specs/fixtures/logic-gates-basic-warrant-v2//corpus.json",
		`code/specs/fixtures/logic-gates-basic-warrant-v2/a\b.json`,
		"code/scripts/logic_gates_basic_drift.py.bak",
		"code/scripts/logic_gates_basic_drift2.py",
		"code/scripts/tests/test_logic_gates_basic_drift.pyc",
		"code/scripts/other/logic_gates_basic_drift.py",
	} {
		if isLogicGatesBasicInput(changed) {
			t.Errorf("predicate accepted lookalike %q", changed)
		}
	}
}

// The new row's declared paths are exactly its predicate (the Kiln export keys on them), and it
// is a separate row from the warrantEvidenceRow of the same crate, which the coverage test
// recognises by its inputs prefix.
func TestLogicGatesBasicRowPathsMatchItsPredicate(t *testing.T) {
	row := logicGatesBasicRow()
	if row.name != "rust/logic-gates" || strings.HasPrefix(row.inputs, warrantEvidenceInputsPrefix) {
		t.Fatalf("unexpected row %q / %q", row.name, row.inputs)
	}
	for _, p := range row.paths {
		probe := p
		if p == logicGatesBasicFixtureDir {
			probe = p + "/corpus.json"
		}
		if !row.matches(probe) {
			t.Errorf("declared path %q is not accepted by the predicate", probe)
		}
	}
	if len(row.treePaths) != 1 || row.treePaths[0] != logicGatesBasicFixtureDir {
		t.Errorf("treePaths %v", row.treePaths)
	}
	// The generic evidence row for the crate is still present (CV2-F13 coverage test).
	found := false
	for _, crate := range warrantEvidenceRowCrates() {
		found = found || crate == "logic-gates"
	}
	if !found {
		t.Error("warrantEvidenceRow(\"logic-gates\") is gone")
	}
}

// The row exists to guarantee the guard runs, so every BUILD file that the package uses on some
// platform must run the script and its tests, and every path the row declares must exist or be
// the not-yet-created fixture directory (LGE-S1c).
func TestLogicGatesBasicRowIsPinnedToTheBuildLine(t *testing.T) {
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate test source")
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", ".."))
	pkg := filepath.Join(repoRoot, "code", "packages", "rust", "logic-gates")
	if _, err := os.Stat(pkg); err != nil {
		t.Skip("rust/logic-gates is not in this checkout")
	}
	for _, name := range []string{"BUILD", "BUILD_linux", "BUILD_windows"} {
		data, err := os.ReadFile(filepath.Join(pkg, name))
		if err != nil {
			t.Fatalf("%s: %v", name, err)
		}
		text := string(data)
		for _, want := range []string{
			"../../../scripts/logic_gates_basic_drift.py",
			"../../../scripts/tests/test_logic_gates_basic_drift.py",
		} {
			ran := false
			for _, line := range strings.Split(text, "\n") {
				if !strings.HasPrefix(strings.TrimSpace(line), "#") && strings.Contains(line, want) {
					ran = true
				}
			}
			if !ran {
				t.Errorf("%s has no non-comment line running %q", name, want)
			}
		}
	}
	for _, p := range logicGatesBasicPaths {
		if p == logicGatesBasicFixtureDir {
			continue // created by LGE-S1c; the drift guard treats its absence as "skipped"
		}
		if _, err := os.Stat(filepath.Join(repoRoot, filepath.FromSlash(p))); err != nil {
			t.Errorf("declared path %s: %v", p, err)
		}
	}
}
