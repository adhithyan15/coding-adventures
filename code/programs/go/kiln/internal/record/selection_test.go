package record

// The merge job's handling of the recording selection (KILN04 7.3)
// ================================================================
//
// `expectedFor` turns the build tool's exports and the merge job's OWN
// declaration of the selection into what recmerge expects. The selection is
// never read from a shard; the unselected set is OUR graph minus OUR plan; the
// steps export, which stays whole, may name packages the plan does not run only
// under a selection; and a selection's plan is checked for language and
// closure against OUR graph. The rules those feed are tested in recmerge.

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// A two-language repository: two Go packages and one Rust package. The plan of
// a `go` recording holds only the Go ones; the graph and steps stay whole.
const (
	selOS = `"build_files":{"linux":"BUILD","darwin":"BUILD","windows":"BUILD"}`

	selSteps = `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
		`{"package":"code/packages/go/a","steps":["go test"]},{"package":"code/packages/go/b","steps":["go vet"]},` +
		`{"package":"code/packages/rust/c","steps":["cargo test"]}]}`
	selGoPlan = `{"packages":[{"name":"go/a","rel_path":"code/packages/go/a"},{"name":"go/b","rel_path":"code/packages/go/b"}],` +
		`"shards":[{"index":0,"assigned_packages":["go/a","go/b"],"package_names":["go/a","go/b"]}]}`
	selAllPlan = `{"packages":[{"name":"go/a","rel_path":"code/packages/go/a"},{"name":"go/b","rel_path":"code/packages/go/b"},{"name":"rust/c","rel_path":"code/packages/rust/c"}],` +
		`"shards":[{"index":0,"assigned_packages":["go/a","go/b","rust/c"],"package_names":["go/a","go/b","rust/c"]}]}`
)

// selGraph builds the graph; bDeps are go/b's prerequisites on every platform
// and bDev its dev deps (test-trigger providers), as JSON array bodies.
func selGraph(bDeps, bDev string) string {
	pkg := func(name, rel, lang, deps, dev string) string {
		return `{"name":"` + name + `","rel_path":"` + rel + `","language":"` + lang + `",` + selOS +
			`,"deps":{"linux":[` + deps + `],"darwin":[` + deps + `],"windows":[` + deps + `]},"dev_deps":[` + dev +
			`],"repo_inputs":[],"repo_input_globs":[],"extra_toolchains":[]}`
	}
	return `{"schema":"kiln-legacy-graph/1","packages":[` +
		pkg("go/a", "code/packages/go/a", "go", "", "") + "," +
		pkg("go/b", "code/packages/go/b", "go", bDeps, bDev) + "," +
		pkg("rust/c", "code/packages/rust/c", "rust", "", "") + `]}`
}

func selInputs(t *testing.T, graph, steps, plan string) (*legacy.Graph, *legacy.Steps, *Partition) {
	t.Helper()
	g, err := legacy.ParseGraph([]byte(graph))
	if err != nil {
		t.Fatal(err)
	}
	s, err := legacy.DecodeSteps([]byte(steps))
	if err != nil {
		t.Fatal(err)
	}
	p, err := ParsePartition([]byte(plan), 1)
	if err != nil {
		t.Fatal(err)
	}
	return g, s, p
}

// A go recording: the whole steps export is accepted (the rust entry is
// skipped, not refused), only planned packages get a recipe digest, and the
// unselected set is the graph minus the plan.
func TestExpectedForASelectionSkipsUnplannedStepsAndFindsTheForeignSet(t *testing.T) {
	g, s, p := selInputs(t, selGraph("", ""), selSteps, selGoPlan)
	exp, err := expectedFor(g, s, p, "go", "g", "s")
	if err != nil {
		t.Fatal(err)
	}
	if exp.Language != "go" {
		t.Fatalf("language %q", exp.Language)
	}
	if len(exp.Steps) != 2 || exp.Steps["code/packages/rust/c"] != "" {
		t.Fatalf("steps: %v", exp.Steps)
	}
	if len(exp.Foreign) != 1 || exp.Foreign[0] != "code/packages/rust/c" || exp.PlanOutsideSelection != 0 || exp.PlanNotClosed != 0 {
		t.Fatalf("%+v", exp)
	}
}

// The all-language refusal, tested where it can be reached (the older
// TestMergeRefusesAStepsExportThePlanDoesNotRun failed at B1 before it ever got
// here, so it was deleted in favour of this): with no selection a steps entry
// for a package the plan does not run is a broken input.
func TestExpectedForAllRefusesAStepsEntryThePlanDoesNotRun(t *testing.T) {
	g, s, p := selInputs(t, selGraph("", ""), selSteps, selGoPlan)
	for _, lang := range []string{""} {
		_, err := expectedFor(g, s, p, lang, "g", "s")
		if err == nil || !strings.Contains(err.Error(), "which no shard of the plan runs") {
			t.Fatalf("language %q: %v", lang, err)
		}
	}
	// The same inputs under a selection are fine: the entry is skipped.
	if _, err := expectedFor(g, s, p, "go", "g", "s"); err != nil {
		t.Fatalf("a selection refused the whole steps export: %v", err)
	}
	// With the plan really holding everything there is nothing foreign.
	g, s, p = selInputs(t, selGraph("", ""), selSteps, selAllPlan)
	exp, err := expectedFor(g, s, p, "", "g", "s")
	if err != nil || exp.Language != "" || exp.Foreign != nil || len(exp.Steps) != 3 {
		t.Fatalf("%+v %v", exp, err)
	}
}

// A language-restricted plan under the default selection: expectedFor reports
// the packages the plan leaves out, and recmerge voids the run on them (a
// restricted plan is never read as `all`).
func TestExpectedForAllReportsWhatARestrictedPlanLeavesOut(t *testing.T) {
	steps := `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[{"package":"code/packages/go/a","steps":["x"]},{"package":"code/packages/go/b","steps":["y"]}]}`
	g, s, p := selInputs(t, selGraph("", ""), steps, selGoPlan)
	exp, err := expectedFor(g, s, p, "", "g", "s")
	if err != nil {
		t.Fatal(err)
	}
	if exp.Language != "" || len(exp.Foreign) != 1 {
		t.Fatalf("%+v", exp)
	}
}

// A plan holding a package of another language was not built with the selection
// the merge declares.
func TestExpectedForCountsPlanPackagesOutsideTheSelection(t *testing.T) {
	g, s, p := selInputs(t, selGraph("", ""), selSteps, selAllPlan)
	exp, err := expectedFor(g, s, p, "go", "g", "s")
	if err != nil {
		t.Fatal(err)
	}
	if exp.PlanOutsideSelection != 1 || len(exp.Foreign) != 0 {
		t.Fatalf("outside %d, foreign %v", exp.PlanOutsideSelection, exp.Foreign)
	}
}

// Closure, from the merge's own graph: a plan package with a prerequisite (a
// dependency or a dev dependency, on any platform) the plan does not run makes
// the plan not closed, whatever the build tool said when it chose it.
func TestExpectedForChecksTheClosureOfThePlanOnItsOwnGraph(t *testing.T) {
	cases := []struct {
		name, deps, dev string
		want            int
	}{
		{"closed", `"go/a"`, "", 0},
		{"a dependency outside the plan", `"rust/c"`, "", 1},
		{"a dev dependency outside the plan", "", `"rust/c"`, 1},
		{"one open package counts once", `"rust/c"`, `"rust/c"`, 1},
	}
	for _, c := range cases {
		g, s, p := selInputs(t, selGraph(c.deps, c.dev), selSteps, selGoPlan)
		exp, err := expectedFor(g, s, p, "go", "g", "s")
		if err != nil {
			t.Fatalf("%s: %v", c.name, err)
		}
		if exp.PlanNotClosed != c.want {
			t.Errorf("%s: PlanNotClosed = %d, want %d", c.name, exp.PlanNotClosed, c.want)
		}
	}
	// Closure is judged only under a selection: with every language recorded the
	// plan is every package, so it is closed by construction.
	g, s, p := selInputs(t, selGraph(`"rust/c"`, ""), selSteps, selAllPlan)
	if exp, err := expectedFor(g, s, p, "", "g", "s"); err != nil || exp.PlanNotClosed != 0 {
		t.Fatalf("%+v %v", exp, err)
	}
}
