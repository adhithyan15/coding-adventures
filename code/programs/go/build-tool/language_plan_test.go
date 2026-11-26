package main

import (
	"bytes"
	"errors"
	"flag"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// languagePlanFixture lays out a small repository:
//
//	go/base        (no prerequisites)
//	go/mid         -> go/base                      Go-only closure: selected
//	go/leaf        (no prerequisites)              selected
//	go/needs-py    -> python/lib                   foreign prerequisite: excluded
//	go/transitive  -> go/needs-py                  transitive foreign: excluded
//	python/lib     (no prerequisites)
//	python/app     -> go/base                      Go is a prerequisite OF python: fine
//
// Cross-language edges use the documented "# build-tool: deps=" comment.
func languagePlanFixture(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	write := func(lang, name, build string) {
		dir := filepath.Join(root, "code", "packages", lang, name)
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "BUILD"), []byte(build), 0o644); err != nil {
			t.Fatal(err)
		}
		if lang == "go" {
			mod := "module example.com/" + name + "\n\ngo 1.26\n"
			if err := os.WriteFile(filepath.Join(dir, "go.mod"), []byte(mod), 0o644); err != nil {
				t.Fatal(err)
			}
		}
	}
	write("go", "base", "go test ./...\n")
	write("go", "mid", "# build-tool: deps=go/base\ngo test ./...\n")
	write("go", "leaf", "go test ./...\n")
	write("go", "needs-py", "# build-tool: deps=python/lib\ngo test ./...\n")
	write("go", "transitive", "# build-tool: deps=go/needs-py\ngo test ./...\n")
	write("python", "lib", "python -m pytest\n")
	write("python", "app", "# build-tool: deps=go/base\npython -m pytest\n")
	return root
}

// emitLanguagePlan runs the real CLI path and returns the plan file bytes.
func emitLanguagePlan(t *testing.T, root string, extra ...string) []byte {
	t.Helper()
	planPath := filepath.Join(t.TempDir(), "plan.json")
	originalArgs, originalFlags, originalStderr, originalStdout := os.Args, flag.CommandLine, os.Stderr, os.Stdout
	devnull, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err != nil {
		t.Fatal(err)
	}
	flag.CommandLine = flag.NewFlagSet("build-tool-test", flag.ContinueOnError)
	os.Args = append([]string{
		"build-tool", "-root", root, "-force", "-validate-build-files=false", "-ci-gates", "",
		"-emit-plan", planPath,
	}, extra...)
	os.Stderr, os.Stdout = devnull, devnull
	code := run()
	os.Args, flag.CommandLine, os.Stderr, os.Stdout = originalArgs, originalFlags, originalStderr, originalStdout
	devnull.Close()
	if code != 0 {
		t.Fatalf("run() exit %d", code)
	}
	data, err := os.ReadFile(planPath)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func readPlanBytes(t *testing.T, data []byte) *plan.BuildPlan {
	t.Helper()
	path := filepath.Join(t.TempDir(), "p.json")
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}
	bp, err := plan.Read(path)
	if err != nil {
		t.Fatal(err)
	}
	return bp
}

func planNames(bp *plan.BuildPlan) []string {
	var names []string
	for _, p := range bp.Packages {
		names = append(names, p.Name)
	}
	sort.Strings(names)
	return names
}

func containsName(list []string, s string) bool {
	for _, item := range list {
		if item == s {
			return true
		}
	}
	return false
}

func TestLanguagePlanSelectsOnlyPrerequisiteClosedGoPackages(t *testing.T) {
	root := languagePlanFixture(t)
	bp := readPlanBytes(t, emitLanguagePlan(t, root, "-plan-closed-language", "go", "-shard-count", "2"))

	want := []string{"go/base", "go/leaf", "go/mid"}
	if got := planNames(bp); !reflect.DeepEqual(got, want) {
		t.Fatalf("selected packages = %v, want %v", got, want)
	}
	for _, edge := range bp.DependencyEdges {
		for _, end := range edge {
			if !containsName(want, end) {
				t.Fatalf("edge %v leaves the selection", edge)
			}
		}
	}
	for _, name := range bp.AffectedPackages {
		if !containsName(want, name) {
			t.Fatalf("affected list names an unselected package: %v", bp.AffectedPackages)
		}
	}
	for goos, state := range bp.PlatformOverrides {
		for _, name := range state.AffectedPackages {
			if !containsName(want, name) {
				t.Fatalf("%s affected list names an unselected package: %v", goos, state.AffectedPackages)
			}
		}
	}
	// Shards cover the selected set only.
	seen := map[string]bool{}
	for _, shard := range bp.Shards {
		for _, name := range shard.PackageNames {
			if !containsName(want, name) {
				t.Fatalf("shard %s contains %s outside the selection", shard.Name, name)
			}
		}
		for _, name := range shard.AssignedPackages {
			seen[name] = true
		}
	}
	if len(seen) != len(want) {
		t.Fatalf("shards assign %v, want exactly %v", seen, want)
	}
}

func TestLanguagePlanIsDeterministic(t *testing.T) {
	root := languagePlanFixture(t)
	first := emitLanguagePlan(t, root, "-plan-closed-language", "go", "-shard-count", "3")
	for i := 0; i < 3; i++ {
		again := emitLanguagePlan(t, root, "-plan-closed-language", "go", "-shard-count", "3")
		if !bytes.Equal(first, again) {
			t.Fatal("same inputs produced different plan bytes")
		}
	}
}

func TestLanguageAllPlanIsUnchangedAndKeepsEveryPackage(t *testing.T) {
	root := languagePlanFixture(t)
	implicit := emitLanguagePlan(t, root, "-shard-count", "3")
	explicit := emitLanguagePlan(t, root, "-language", "all", "-shard-count", "3")
	// -language go alone must still emit every package (origin/main behaviour).
	if filtered := planNames(readPlanBytes(t, emitLanguagePlan(t, root, "-language", "go", "-shard-count", "3"))); len(filtered) != 7 {
		t.Fatalf("-language go with -emit-plan must not narrow the plan, got %v", filtered)
	}
	if !bytes.Equal(implicit, explicit) {
		t.Fatal("-language all differs from the default plan")
	}
	got := planNames(readPlanBytes(t, explicit))
	want := []string{"go/base", "go/leaf", "go/mid", "go/needs-py", "go/transitive", "python/app", "python/lib"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("all plan packages = %v, want %v", got, want)
	}
}

func TestLanguagePlanWeightsBalanceOverSelectedPackagesOnly(t *testing.T) {
	root := languagePlanFixture(t)
	weights := filepath.Join(root, "code", "durations.json")
	body := `{"schema":"ci-package-durations-v1","unit":"seconds","durations":` +
		`{"go/base":10,"go/mid":20,"go/leaf":30,"python/lib":5000,"go/needs-py":5000}}`
	if err := os.WriteFile(weights, []byte(body), 0o644); err != nil {
		t.Fatal(err)
	}
	bp := readPlanBytes(t, emitLanguagePlan(t, root, "-plan-closed-language", "go", "-shard-count", "2", "-shard-weights", weights))
	total := 0
	for _, shard := range bp.Shards {
		total += shard.EstimatedCost
		if shard.EstimatedCost >= 5000 {
			t.Fatalf("shard %s is costed with an excluded package: %d", shard.Name, shard.EstimatedCost)
		}
	}
	if total == 0 {
		t.Fatal("weighted shards carry no cost")
	}
}

// fakeGraphFor builds per-platform graphs from edge lists (prereq, dependent).
func fakeGraphFor(edges map[string][][2]string, nodes []string) graphForPlatform {
	return func(goos string, _ []discovery.Package) (platformView, error) {
		g := directedgraph.New()
		for _, n := range nodes {
			g.AddNode(n)
		}
		for _, e := range edges[goos] {
			g.AddEdge(e[0], e[1])
		}
		return platformView{Graph: g}, nil
	}
}

func TestSelectPrerequisiteClosedLanguageUnionOverPlatforms(t *testing.T) {
	pkgs := []discovery.Package{
		{Name: "go/a", Language: "go"}, {Name: "go/b", Language: "go"},
		{Name: "go/c", Language: "go"}, {Name: "py/x", Language: "python"},
	}
	nodes := []string{"go/a", "go/b", "go/c", "py/x"}
	// go/b needs a python package only on windows; go/c needs go/b.
	edges := map[string][][2]string{
		"linux":   {{"go/a", "go/b"}, {"go/b", "go/c"}},
		"windows": {{"go/a", "go/b"}, {"go/b", "go/c"}, {"py/x", "go/b"}},
	}
	got, err := selectPrerequisiteClosedLanguage(pkgs, "go", []string{"linux", "darwin", "windows"}, fakeGraphFor(edges, nodes))
	if err != nil {
		t.Fatal(err)
	}
	if len(got.Selected) != 1 || got.Selected[0].Name != "go/a" {
		t.Fatalf("selected %v", got.Selected)
	}
	if !reflect.DeepEqual(got.Excluded, []string{"go/b", "go/c"}) {
		t.Fatalf("excluded %v", got.Excluded)
	}
	// On linux alone nothing is foreign, so all three Go packages survive.
	got, _ = selectPrerequisiteClosedLanguage(pkgs, "go", []string{"linux"}, fakeGraphFor(edges, nodes))
	if len(got.Selected) != 3 || len(got.Excluded) != 0 {
		t.Fatalf("linux-only selection %v / %v", got.Selected, got.Excluded)
	}
}

func TestSelectPrerequisiteClosedLanguageFailsClosedOnUnknownNode(t *testing.T) {
	pkgs := []discovery.Package{{Name: "go/a", Language: "go"}}
	edges := map[string][][2]string{"linux": {{"ghost", "go/a"}}}
	got, _ := selectPrerequisiteClosedLanguage(pkgs, "go", []string{"linux"}, fakeGraphFor(edges, []string{"go/a"}))
	if len(got.Selected) != 0 || !reflect.DeepEqual(got.Excluded, []string{"go/a"}) {
		t.Fatalf("unknown prerequisite must exclude: %v / %v", got.Selected, got.Excluded)
	}
}

// failingSuccessors is a graph whose Successors lookup fails for one node.
type failingSuccessors struct{ nodes []string }

func (g failingSuccessors) Nodes() []string { return g.nodes }
func (g failingSuccessors) Successors(string) ([]string, error) {
	return nil, errors.New("injected successor failure")
}

func TestForeignTaintReturnsSuccessorErrors(t *testing.T) {
	pkgs := []discovery.Package{{Name: "py/x", Language: "python"}, {Name: "go/a", Language: "go"}}
	if _, err := foreignTaint(platformView{Graph: failingSuccessors{nodes: []string{"py/x", "go/a"}}}, pkgs, "go"); err == nil {
		t.Fatal("a Successors error must fail the plan, not be skipped")
	}
}

func runExpectingFailure(t *testing.T, args ...string) int {
	t.Helper()
	originalArgs, originalFlags, originalStderr, originalStdout := os.Args, flag.CommandLine, os.Stderr, os.Stdout
	devnull, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err != nil {
		t.Fatal(err)
	}
	flag.CommandLine = flag.NewFlagSet("build-tool-test", flag.ContinueOnError)
	os.Args = append([]string{"build-tool"}, args...)
	os.Stderr, os.Stdout = devnull, devnull
	code := run()
	os.Args, flag.CommandLine, os.Stderr, os.Stdout = originalArgs, originalFlags, originalStderr, originalStdout
	devnull.Close()
	return code
}

func TestPlanClosedLanguageIsRejectedOutsideEmitPlan(t *testing.T) {
	root := languagePlanFixture(t)
	if code := runExpectingFailure(t, "-root", root, "-force", "-dry-run", "-plan-closed-language", "go"); code != 1 {
		t.Fatalf("without -emit-plan: exit %d, want 1", code)
	}
	planPath := filepath.Join(t.TempDir(), "p.json")
	if code := runExpectingFailure(t, "-root", root, "-force", "-emit-plan", planPath, "-plan-closed-language", "all"); code != 1 {
		t.Fatalf("with all: exit %d, want 1", code)
	}
}

func TestForeignTestTriggerProviderTaintsTheConsumer(t *testing.T) {
	pkgs := []discovery.Package{
		{Name: "go/a", Language: "go"}, {Name: "go/b", Language: "go"},
		{Name: "go/c", Language: "go"}, {Name: "rust/x", Language: "rust"},
	}
	g := directedgraph.New()
	for _, p := range pkgs {
		g.AddNode(p.Name)
	}
	g.AddEdge("go/b", "go/c") // go/c builds on go/b
	// go/b's tests hash a Rust provider; no build edge exists.
	view := platformView{Graph: g, Triggers: resolver.TestTriggers{"rust/x": {"go/b"}}}
	tainted, err := foreignTaint(view, pkgs, "go")
	if err != nil {
		t.Fatal(err)
	}
	if !tainted["go/b"] || !tainted["go/c"] || tainted["go/a"] {
		t.Fatalf("trigger taint wrong: %v", tainted)
	}
}

func TestUnresolvedBuildToolDepsTaintsThePackage(t *testing.T) {
	pkgs := []discovery.Package{
		{Name: "go/a", Language: "go"},
		{Name: "go/typo", Language: "go", BuildContent: "# build-tool: deps=go/a, go/nope\n"},
		{Name: "go/user", Language: "go"},
	}
	g := directedgraph.New()
	for _, p := range pkgs {
		g.AddNode(p.Name)
	}
	g.AddEdge("go/typo", "go/user")
	got, err := selectPrerequisiteClosedLanguage(pkgs, "go", []string{"linux"},
		func(string, []discovery.Package) (platformView, error) {
			return platformView{Graph: g, Unresolved: []string{"go/typo"}}, nil
		})
	if err != nil {
		t.Fatal(err)
	}
	if len(got.Selected) != 1 || got.Selected[0].Name != "go/a" || len(got.Excluded) != 2 {
		t.Fatalf("selected %v excluded %v", got.Selected, got.Excluded)
	}
	names := map[string]bool{"go/a": true, "go/typo": true}
	if u := resolver.UnresolvedBuildToolDeps(pkgs[1], names); len(u) != 1 || u[0] != "go/nope" {
		t.Fatalf("UnresolvedBuildToolDeps = %v", u)
	}
}

func TestClosedPlanFailsOnAnUnreadablePlatformBuildFile(t *testing.T) {
	root := languagePlanFixture(t)
	// A BUILD_windows that exists but has no read permission.
	file := filepath.Join(root, "code", "packages", "go", "leaf", "BUILD_windows")
	if err := os.WriteFile(file, []byte("go test ./...\n"), 0o000); err != nil {
		t.Fatal(err)
	}
	if _, err := os.ReadFile(file); err == nil {
		t.Skip("file permissions are not enforced here (root or Windows)")
	}
	planPath := filepath.Join(t.TempDir(), "p.json")
	if code := runExpectingFailure(t, "-root", root, "-force", "-validate-build-files=false", "-ci-gates", "",
		"-emit-plan", planPath, "-plan-closed-language", "go"); code != 1 {
		t.Fatalf("closed plan with an unreadable platform BUILD: exit %d, want 1", code)
	}
}

func TestClosedPlanWithNoSelectedPackagesFails(t *testing.T) {
	root := languagePlanFixture(t)
	planPath := filepath.Join(t.TempDir(), "p.json")
	if code := runExpectingFailure(t, "-root", root, "-force", "-validate-build-files=false", "-ci-gates", "",
		"-emit-plan", planPath, "-plan-closed-language", "gooo"); code != 1 {
		t.Fatalf("misspelled language: exit %d, want 1", code)
	}
	if _, err := os.Stat(planPath); err == nil {
		t.Fatal("no plan may be written for an empty selection")
	}
}
