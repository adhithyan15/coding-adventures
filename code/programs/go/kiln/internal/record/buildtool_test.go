package record

import (
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// repoRoot finds the checkout this test file lives in, or "".
func repoRoot() string {
	d, err := os.Getwd()
	if err != nil {
		return ""
	}
	for {
		if _, err := os.Stat(filepath.Join(d, "code", "programs", "go", "build-tool", "go.mod")); err == nil {
			return d
		}
		parent := filepath.Dir(d)
		if parent == d {
			return ""
		}
		d = parent
	}
}

// TestRealBuildToolExportsAreWhatPrepareReads builds the REAL build tool and
// runs it, with the very arguments `kiln record prepare` passes (GraphArgs and
// PlanArgs), on THIS repository. Then it reads the three exports with the
// readers `prepare` uses.
//
// Why on this repository and not on a small fixture. The graph export refuses a
// tree whose repository-input relations name packages discovery did not find
// (build-tool's requireKilnConsumers), and a variable of another module's main
// package cannot be set from here, so a fixture tree cannot pass the real
// command line. The unit tests of Prepare use a fake build tool that writes the
// same three formats; this test is what ties the fake to the real thing, so a
// format change in the build tool fails here, in Kiln's own tests.
//
// It takes about a minute (two full discovery passes over the repository) and
// is skipped with -short, when Go or the build tool's sources are missing, and
// in a sparse checkout (where the export itself refuses the tree).
func TestRealBuildToolExportsAreWhatPrepareReads(t *testing.T) {
	if testing.Short() {
		t.Skip("-short: runs the real build tool over the whole repository")
	}
	root := repoRoot()
	if root == "" {
		t.Skip("the repository root was not found")
	}
	goTool, err := exec.LookPath("go")
	if err != nil {
		t.Skip("go is not on PATH")
	}
	out := t.TempDir()
	exe := filepath.Join(out, "build-tool")
	if runtime.GOOS == "windows" {
		exe += ".exe"
	}
	build := exec.Command(goTool, "build", "-o", exe, ".")
	build.Dir = filepath.Join(root, "code", "programs", "go", "build-tool")
	if msg, err := build.CombinedOutput(); err != nil {
		t.Skipf("cannot build the build tool here: %v\n%s", err, msg)
	}

	// The MINIMAL environment `prepare` gives the build tool (buildToolEnv): the
	// exports must not need anything else of the job's environment.
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("no home directory")
	}
	gitPath, err := exec.LookPath("git")
	if err != nil {
		t.Skip("the version-control tool is not on PATH")
	}
	env := buildToolEnv(home, filepath.Dir(gitPath)+":/usr/bin:/bin", nil)
	graphPath, stepsPath, planPath := filepath.Join(out, "graph.json"), filepath.Join(out, "steps.json"), filepath.Join(out, "plan.json")
	err = execTool(ToolRun{Path: exe, Args: GraphArgs(root, graphPath, stepsPath), Dir: out, Env: env, Timeout: 10 * time.Minute})
	if err != nil && strings.Contains(err.Error(), "were not discovered") {
		t.Skipf("this checkout is not the whole repository: %v", err)
	}
	if err != nil {
		t.Fatalf("graph and steps export: %v", err)
	}
	if err := execTool(ToolRun{Path: exe, Args: PlanArgs(root, 3, planPath), Dir: out, Env: env, Timeout: 10 * time.Minute}); err != nil {
		t.Fatalf("plan export: %v", err)
	}

	g, err := legacy.ReadGraphFile(graphPath)
	if err != nil {
		t.Fatal(err)
	}
	steps, err := legacy.ReadStepsFile(stepsPath)
	if err != nil {
		t.Fatal(err)
	}
	if steps.OS != runtime.GOOS {
		t.Fatalf("steps are for %q, this is %q", steps.OS, runtime.GOOS)
	}
	part, err := ReadPartition(planPath, 3)
	if err != nil {
		t.Fatal(err)
	}
	// Every package a shard runs is a package of the graph: the property
	// Prepare checks for the shard it records.
	inGraph := map[string]bool{}
	for _, p := range g.Packages {
		inGraph[p.RelPath] = true
	}
	assigned := 0
	for i := 0; i < 3; i++ {
		runs, err := part.Runs(i)
		if err != nil || len(runs) == 0 {
			t.Fatalf("shard %d: %v (%d packages)", i, err, len(runs))
		}
		for _, rel := range runs {
			if !inGraph[rel] {
				t.Fatalf("shard %d runs %q, which the graph export does not list", i, rel)
			}
		}
		assigned += len(part.Shards[i].Assigned)
	}
	if assigned < len(g.Packages)/2 {
		t.Fatalf("only %d of %d packages are assigned", assigned, len(g.Packages))
	}
	if d, err := part.Digest(); err != nil || len(d) != 64 {
		t.Fatalf("%q %v", d, err)
	}
	// A package of this module has a recipe on Linux and Darwin.
	if runtime.GOOS != "windows" {
		if lines, ok := steps.Lines("code/programs/go/kiln"); !ok || len(lines) == 0 {
			t.Fatalf("no steps for the kiln package: %v %v", lines, ok)
		}
	}
}
