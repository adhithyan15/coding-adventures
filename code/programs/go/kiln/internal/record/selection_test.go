package record

// The merge job's reading of the recording selection (KILN04 7.3)
// ===============================================================
//
// `expectedFor` is the part of the merge that turns the build tool's exports and
// the shards' bindings into what recmerge expects. The selection is read from
// the bindings only; the unselected set is OUR graph minus OUR plan; and the
// steps export, which stays whole, may name packages the plan does not run
// only under a selection. The rules those feed are tested in recmerge.

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// A two-language repository: two Go packages and one Rust package. The plan of
// a `go` recording holds only the Go ones; the graph and steps stay whole.
const (
	selGraph = `{"schema":"kiln-legacy-graph/1","packages":[` +
		`{"name":"go/a","rel_path":"code/packages/go/a","language":"go","build_files":{"linux":"BUILD","darwin":"BUILD","windows":"BUILD"},"deps":{"linux":[],"darwin":[],"windows":[]},"dev_deps":[],"repo_inputs":[],"repo_input_globs":[],"extra_toolchains":[]},` +
		`{"name":"go/b","rel_path":"code/packages/go/b","language":"go","build_files":{"linux":"BUILD","darwin":"BUILD","windows":"BUILD"},"deps":{"linux":[],"darwin":[],"windows":[]},"dev_deps":[],"repo_inputs":[],"repo_input_globs":[],"extra_toolchains":[]},` +
		`{"name":"rust/c","rel_path":"code/packages/rust/c","language":"rust","build_files":{"linux":"BUILD","darwin":"BUILD","windows":"BUILD"},"deps":{"linux":[],"darwin":[],"windows":[]},"dev_deps":[],"repo_inputs":[],"repo_input_globs":[],"extra_toolchains":[]}]}`
	selSteps = `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
		`{"package":"code/packages/go/a","steps":["go test"]},{"package":"code/packages/go/b","steps":["go vet"]},` +
		`{"package":"code/packages/rust/c","steps":["cargo test"]}]}`
	selGoPlan = `{"packages":[{"name":"go/a","rel_path":"code/packages/go/a"},{"name":"go/b","rel_path":"code/packages/go/b"}],` +
		`"shards":[{"index":0,"assigned_packages":["go/a","go/b"],"package_names":["go/a","go/b"]}]}`
	selAllPlan = `{"packages":[{"name":"go/a","rel_path":"code/packages/go/a"},{"name":"go/b","rel_path":"code/packages/go/b"},{"name":"rust/c","rel_path":"code/packages/rust/c"}],` +
		`"shards":[{"index":0,"assigned_packages":["go/a","go/b","rust/c"],"package_names":["go/a","go/b","rust/c"]}]}`
)

func selInputs(t *testing.T, steps, plan string) (*legacy.Graph, *legacy.Steps, *Partition) {
	t.Helper()
	g, err := legacy.ParseGraph([]byte(selGraph))
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

// shardWith is a decoded shard whose binding seals lang and whose records name
// the given actions.
func shardWith(index int, lang string, actions ...string) *recfmt.Shard {
	sh := &recfmt.Shard{ArtifactIndex: index, BindingID: strings.Repeat("a", 63) + string(rune('0'+index))}
	sh.Binding.Language = lang
	for _, a := range actions {
		sh.Records = append(sh.Records, recfmt.ShardRecord{Action: a})
	}
	return sh
}

// A go recording: the whole steps export is accepted (the rust entry is
// ignored, not refused), only planned packages get a recipe digest, and the
// unselected set is the graph minus the plan.
func TestExpectedForASelectionIgnoresUnplannedStepsAndFindsTheForeignSet(t *testing.T) {
	g, s, p := selInputs(t, selSteps, selGoPlan)
	exp, err := expectedFor(g, s, p, []*recfmt.Shard{shardWith(0, "go")}, "g", "s")
	if err != nil {
		t.Fatal(err)
	}
	if exp.Language != "go" {
		t.Fatalf("language %q", exp.Language)
	}
	if len(exp.Steps) != 2 || exp.Steps["code/packages/rust/c"] != "" {
		t.Fatalf("steps: %v", exp.Steps)
	}
	if len(exp.Foreign) != 1 || exp.Foreign[0] != "code/packages/rust/c" || exp.PlanOutsideSelection != 0 {
		t.Fatalf("foreign %v, outside %d", exp.Foreign, exp.PlanOutsideSelection)
	}
}

// The default is unchanged: the same whole steps export is a broken input,
// because every package of the plan is run and the rust one is not in it.
func TestExpectedForAllStillRefusesAnUnplannedStepsEntry(t *testing.T) {
	g, s, p := selInputs(t, selSteps, selGoPlan)
	for _, lang := range []string{""} {
		_, err := expectedFor(g, s, p, []*recfmt.Shard{shardWith(0, lang)}, "g", "s")
		if err == nil || !strings.Contains(err.Error(), "which no shard of the plan runs") {
			t.Fatalf("language %q: %v", lang, err)
		}
	}
	// And with the plan really holding everything, there is nothing foreign.
	g, s, p = selInputs(t, selSteps, selAllPlan)
	exp, err := expectedFor(g, s, p, []*recfmt.Shard{shardWith(0, "")}, "g", "s")
	if err != nil || exp.Language != "" || exp.Foreign != nil || len(exp.Steps) != 3 {
		t.Fatalf("%+v %v", exp, err)
	}
}

// A shard that seals `go` over an all-language plan: the plan holds a rust
// package the selection excludes, which recmerge turns into a void run.
func TestExpectedForCountsPlanPackagesOutsideTheSelection(t *testing.T) {
	g, s, p := selInputs(t, selSteps, selAllPlan)
	exp, err := expectedFor(g, s, p, []*recfmt.Shard{shardWith(0, "go")}, "g", "s")
	if err != nil {
		t.Fatal(err)
	}
	if exp.PlanOutsideSelection != 1 || len(exp.Foreign) != 0 {
		t.Fatalf("outside %d, foreign %v", exp.PlanOutsideSelection, exp.Foreign)
	}
}

// MUTANT KILLER. The selection and the foreign set come from the bindings and
// the regenerated plan, never from what a shard's records say it ran: a shard
// listing an extra (or an unselected) package changes neither.
func TestExpectedForNeverTrustsAShardsPackageList(t *testing.T) {
	g, s, p := selInputs(t, selSteps, selGoPlan)
	honest, err := expectedFor(g, s, p, []*recfmt.Shard{shardWith(0, "go", "code/packages/go/a", "code/packages/go/b")}, "g", "s")
	if err != nil {
		t.Fatal(err)
	}
	lying := shardWith(0, "go", "code/packages/go/a", "code/packages/go/b", "code/packages/rust/c", "code/packages/zz/extra")
	got, err := expectedFor(g, s, p, []*recfmt.Shard{lying}, "g", "s")
	if err != nil {
		t.Fatal(err)
	}
	if strings.Join(got.Foreign, ",") != strings.Join(honest.Foreign, ",") || len(got.Steps) != len(honest.Steps) ||
		got.Language != honest.Language || got.PlanOutsideSelection != honest.PlanOutsideSelection {
		t.Fatalf("a shard's package list moved the expectations:\n honest %+v\n lying  %+v", honest, got)
	}
	// And a shard's records carry no language: only the binding does.
	if got.Language != "go" {
		t.Fatalf("language %q", got.Language)
	}
}

// The binding of the lowest shard index names the selection (ties by id), in
// whatever order the shards arrive; recmerge voids any shard that disagrees.
func TestSelectionOfIsTheLowestShardsBinding(t *testing.T) {
	a, b, c := shardWith(0, "go"), shardWith(1, ""), shardWith(2, "rust")
	for _, order := range [][]*recfmt.Shard{{a, b, c}, {c, b, a}, {b, c, a}} {
		if got := selectionOf(order); got != "go" {
			t.Fatalf("got %q", got)
		}
	}
	if selectionOf(nil) != "" {
		t.Fatal("no shards, no selection")
	}
	if selectionOf([]*recfmt.Shard{shardWith(0, ""), shardWith(1, "go")}) != "" {
		t.Fatal("the default shard 0 must win")
	}
}
