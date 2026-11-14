//go:build linux && amd64

package main

// TestSandboxRecordCollect: the whole recording shard, with the real strace and
// the real sandbox (KILN04 section 4.8, stage 4 PR 2b-2b-ii): `kiln record
// prepare`, every step through `kiln record step`, `kiln record collect`, and
// then the merge (recmerge) over the shard that came out.
//
// The test plays the build tool (the real one refuses a fixture tree, see
// internal/record/buildtool_test.go): it runs the steps of each action in order,
// stops an action at its first failing step, and writes the `-emit-results`
// file the build tool would. Everything else is the product.
//
// Six actions, one shard of one, each chosen for the verdict it must get:
//
//	A  x/a  a clean hit        reads its own data.txt, writes nothing
//	B  x/b  a failed step      `exit 3`; the action's status is failed
//	C  x/c  a write the trace  `chmod u+w <the checkout>`: a metadata write to the
//	        cannot place       repository root itself has no write root to coarsen
//	                           to, so the classifier says writes_complete false (W7,
//	                           section 5.5): an unknown write set, run-void (G1).
//	                           The step fails if chmod cannot run (no `|| true`)
//	D  x/d  a step run twice   the supervisor refuses the second run (125) and
//	                           leaves a .rerun marker: unknown_writes
//	E  x/e  a tracked file     clean itself, but its data.txt is rewritten after
//	        changed after      the build, by something no trace saw: the post-build
//	        the build          VerifyTree counts one unexplained tracked change
//	F  x/f  an empty recipe    a comment-only BUILD: no step, results built 0 of 0;
//	                           vacuously complete, known, and not emitted
//
// What is asserted, exactly:
//
//	collect writes the shard; its header says "tracked-changed" and counts one
//	unexplained tracked change and no unexplained path; A and E are believed and
//	complete; B is a failed record, believed; C is believed but its
//	writes_complete is false; D is an unknown write set that names the rerun;
//	the shard decodes with recfmt and the merge reads it; the merge's strict
//	verdict is run-void with the causes M1 (C, D, B) and M2b, A is recordable
//	with no reason, B is a truncated execution and E's shard is void.
//
// It is a Linux-only test in the `kiln-sandbox` CI job (assert_sandbox_tests.py
// names it). The layout and the preparation are those of TestSandboxRecordStep.

import (
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// collectKeys answers the merge's key questions from the sealed action files:
// what the merge job would recompute from its own tree.
type collectKeys struct{ pre *record.Pre }

func (k collectKeys) Weak(_ recfmt.HostFacts, action string) ([32]byte, string, error) {
	a, _ := k.pre.Action(action)
	var out [32]byte
	if a.Refusal != "" {
		return out, a.Refusal, nil
	}
	b, err := hex.DecodeString(a.Weak)
	copy(out[:], b)
	return out, "", err
}

func (k collectKeys) Strong(_ recfmt.HostFacts, _ string, _ []string) ([32]byte, error) {
	return [32]byte{7}, nil
}

func TestSandboxRecordCollect(t *testing.T) {
	newSandboxFixture(t)
	straceBin, err := exec.LookPath("strace")
	if err != nil {
		skipOrFail(t, "strace is not installed")
	}
	if straceBin, err = filepath.EvalSymlinks(straceBin); err != nil {
		t.Fatal(err)
	}
	gitBin, err := exec.LookPath("git")
	if err != nil {
		skipOrFail(t, "git is not installed")
	}
	if gitBin, err = filepath.EvalSymlinks(gitBin); err != nil {
		t.Fatal(err)
	}
	goBin, err := exec.LookPath("go")
	if err != nil {
		skipOrFail(t, "the go tool is not available to build an uninstrumented kiln")
	}
	realHome, err := os.UserHomeDir()
	if err != nil || !filepath.IsAbs(realHome) || strings.HasPrefix(realHome, "/tmp/") || realHome == "/tmp" {
		skipOrFail(t, "no home directory outside /tmp to use as the step's $HOME: %q %v", realHome, err)
	}
	base, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	home, err := os.MkdirTemp(realHome, ".kiln-collect-test-")
	if err != nil {
		skipOrFail(t, "cannot make a directory under the home directory: %v", err)
	}
	t.Cleanup(func() { os.RemoveAll(home) })
	if home, err = filepath.EvalSymlinks(home); err != nil {
		t.Fatal(err)
	}
	if err := os.Remove(legacy.RecordTmpdir); err != nil && !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("%s is in the way (not empty?): %v", legacy.RecordTmpdir, err)
	}
	t.Cleanup(func() { os.Remove(legacy.RecordTmpdir) })

	repo, recDir, bin, aux, outDir := filepath.Join(base, "repo"), filepath.Join(base, "record"), filepath.Join(base, "bin"), filepath.Join(base, "aux"), filepath.Join(base, "out")
	for _, d := range []string{repo, bin, aux, outDir} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	kiln := filepath.Join(bin, "kiln")
	if out, err := exec.Command(goBin, "build", "-o", kiln, ".").CombinedOutput(); err != nil {
		t.Fatalf("go build: %v\n%s", err, out)
	}

	// The five actions. The checkout's canonical path is known now (base is
	// canonical, and repo is a plain directory in it), which C's line needs.
	const (
		actA = "code/packages/x/a"
		actB = "code/packages/x/b"
		actC = "code/packages/x/c"
		actD = "code/packages/x/d"
		actE = "code/packages/x/e"
		actF = "code/packages/x/f"
	)
	type action struct {
		rel   string
		name  string // the build tool's package name
		steps []string
	}
	actions := []action{
		{actA, "x/a", []string{"echo hi; cat data.txt"}},
		{actB, "x/b", []string{"exit 3"}},
		// A metadata write to the repository root (W7). Quoted, absolute, and with
		// no `|| true`: if chmod cannot run the step fails and so does this test,
		// loudly, instead of the trigger silently going away.
		{actC, "x/c", []string{`chmod u+w "` + repo + `"`}},
		{actD, "x/d", []string{"echo d"}},
		{actE, "x/e", []string{"cat data.txt"}},
		{actF, "x/f", []string{}},
	}

	files := recStepPolicyFiles(imageRoots())
	files["code/programs/go/build-tool/go.mod"] = "module example.com/bt\n\ngo 1.26\n"
	files["code/programs/go/build-tool/main.go"] = "package main\n"
	for _, a := range actions {
		if len(a.steps) == 0 {
			files[a.rel+"/BUILD"] = "# nothing to run\n"
		} else {
			files[a.rel+"/BUILD"] = a.steps[0] + "\n"
		}
		files[a.rel+"/data.txt"] = "content of " + a.rel + "\n"
	}
	for p, c := range files {
		recStepWrite(t, filepath.Join(repo, filepath.FromSlash(p)), c, 0o644)
	}
	recStepGit(t, gitBin, home, repo, "init", "-q")
	recStepGit(t, gitBin, home, repo, "config", "commit.gpgsign", "false")
	recStepGit(t, gitBin, home, repo, "config", "core.autocrlf", "false")
	recStepGit(t, gitBin, home, repo, "add", "-A")
	recStepGit(t, gitBin, home, repo, "commit", "-q", "-m", "init")
	sha := recStepGit(t, gitBin, home, repo, "rev-parse", "HEAD")
	if repo, err = filepath.EvalSymlinks(repo); err != nil {
		t.Fatal(err)
	}

	// The fake build tool's exports.
	b := "BUILD"
	var pkgs []legacy.Package
	var stepRows []map[string]any
	var planPkgs, names []string
	for _, a := range actions {
		pkgs = append(pkgs, legacy.Package{
			Name: a.name, RelPath: a.rel, Language: "x",
			BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
			Deps:       map[string][]string{"linux": {}, "darwin": {}, "windows": {}},
			DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
		})
		stepRows = append(stepRows, map[string]any{"package": a.rel, "steps": a.steps})
		planPkgs = append(planPkgs, fmt.Sprintf(`{"name": %q, "rel_path": %q, "language": "x"}`, a.name, a.rel))
		names = append(names, fmt.Sprintf("%q", a.name))
	}
	graph, err := json.Marshal(legacy.Graph{Schema: legacy.GraphSchema, Packages: pkgs})
	if err != nil {
		t.Fatal(err)
	}
	stepsJSON, err := json.Marshal(map[string]any{"schema": "kiln-legacy-steps/1", "os": "linux", "packages": stepRows})
	if err != nil {
		t.Fatal(err)
	}
	recStepWrite(t, filepath.Join(aux, "graph.json"), string(graph), 0o644)
	recStepWrite(t, filepath.Join(aux, "steps.json"), string(stepsJSON), 0o644)
	recStepWrite(t, filepath.Join(aux, "plan.json"), `{"schema_version": 1, "force": true, "packages": [`+strings.Join(planPkgs, ",")+`],
  "shards": [{"index": 0, "name": "s0", "assigned_packages": [`+strings.Join(names, ",")+`], "package_names": [`+strings.Join(names, ",")+`]}]}`, 0o644)
	recStepWrite(t, filepath.Join(aux, "event.json"), `{"repository":{"default_branch":"main"}}`, 0o644)
	recStepWrite(t, filepath.Join(bin, "build-tool"), `#!/bin/sh
g=""; s=""; p=""
while [ $# -gt 0 ]; do
  case "$1" in
    -emit-kiln-graph) g="$2"; shift 2;;
    -emit-kiln-steps) s="$2"; shift 2;;
    -emit-plan) p="$2"; shift 2;;
    *) shift;;
  esac
done
if [ -n "$g" ]; then cat '`+filepath.Join(aux, "graph.json")+`' > "$g"; cat '`+filepath.Join(aux, "steps.json")+`' > "$s"; fi
if [ -n "$p" ]; then cat '`+filepath.Join(aux, "plan.json")+`' > "$p"; fi
exit 0
`, 0o755)

	// prepare, as the workflow runs it.
	prepEnv := []string{
		"PATH=/usr/bin:/bin", "HOME=" + home,
		"GITHUB_ACTIONS=true", "GITHUB_REF=refs/heads/main", "GITHUB_EVENT_NAME=schedule",
		"GITHUB_EVENT_PATH=" + filepath.Join(aux, "event.json"), "ImageOS=ubuntu24", "ImageVersion=20260101.1",
		"GITHUB_SHA=" + sha, "GITHUB_REPOSITORY=owner/repo", "GITHUB_RUN_ID=1", "GITHUB_RUN_ATTEMPT=1",
	}
	prep := exec.Command(kiln, "record", "prepare", "--root", repo, "--dir", recDir, "--build-tool", filepath.Join(bin, "build-tool"),
		"--shard-index", "0", "--shard-count", "1", "--job-start", fmt.Sprint(time.Now().Unix()),
		"--strace", straceBin, "--git", gitBin)
	prep.Dir, prep.Env = "/", prepEnv
	if out, eb, code := recStepRun(t, prep); code != 0 {
		t.Fatalf("record prepare: exit %d\nstdout: %s\nstderr: %s", code, out, eb)
	}
	pre, err := record.VerifyPre(recDir)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { // the sealed directory is read-only
		filepath.Walk(recDir, func(p string, info os.FileInfo, err error) error {
			if err == nil {
				os.Chmod(p, 0o700)
			}
			return nil
		})
	})

	// The build, serially (-jobs 1): each action's steps in order, an action
	// stopped at its first failing step, as the build tool does.
	runStep := func(a action, n int) int {
		t.Helper()
		env := append(append([]string{}, pre.Tools.BuildToolEnv...),
			"KILN_ACTION="+a.rel, fmt.Sprintf("KILN_STEP=%d", n), fmt.Sprintf("KILN_STEP_COUNT=%d", len(a.steps)))
		cmd := exec.Command(kiln, "record", "step", "--dir", recDir, "--", "sh", "-c", a.steps[n])
		cmd.Dir, cmd.Env = pre.Config.PackageDir(a.rel), env
		_, _, code := recStepRun(t, cmd)
		return code
	}
	type result struct {
		Package    string `json:"package"`
		Status     string `json:"status"`
		StepsRun   int    `json:"steps_run"`
		StepsTotal int    `json:"steps_total"`
	}
	var results []result
	for _, a := range actions {
		r := result{Package: a.rel, Status: "built", StepsTotal: len(a.steps)}
		for n := range a.steps {
			r.StepsRun++
			if code := runStep(a, n); code != 0 {
				r.Status = "failed"
				break
			}
		}
		results = append(results, r)
	}
	// D's only step is run again: a step runs once. The supervisor refuses it
	// (125) and leaves a marker.
	if code := runStep(actions[3], 0); code != record.ExitRefused {
		t.Fatalf("the second run of D's step exited %d, want %d", code, record.ExitRefused)
	}
	if _, err := os.Stat(record.StepFile(recDir, actD, 0, "rerun")); err != nil {
		t.Fatalf("no .rerun marker: %v", err)
	}
	resultsPath := filepath.Join(base, "results.json")
	data, err := json.Marshal(map[string]any{"schema": "kiln-legacy-results/1", "packages": results})
	if err != nil {
		t.Fatal(err)
	}
	recStepWrite(t, resultsPath, string(data), 0o644)

	// E's tracked file is rewritten after the build, by something no trace saw.
	if err := os.WriteFile(filepath.Join(repo, filepath.FromSlash(actE), "data.txt"), []byte("rewritten after the build\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	// collect, as the workflow runs it. A shard that is void is still collected:
	// the verdict is the merge's.
	out := filepath.Join(outDir, "records.jsonl")
	col := exec.Command(kiln, "record", "collect", "--root", repo, "--dir", recDir, "--results", resultsPath, "--out", out)
	col.Dir, col.Env = "/", []string{"PATH=/usr/bin:/bin", "HOME=" + home}
	stdout, stderr, code := recStepRun(t, col)
	if code != 0 {
		t.Fatalf("record collect: exit %d\nstdout: %s\nstderr: %s", code, stdout, stderr)
	}
	// The first CI run of this test must be diagnosable: whatever fails below,
	// show what collect itself said (its report names each failed post-build check
	// and the paths it could not explain).
	t.Cleanup(func() {
		if t.Failed() {
			t.Logf("collect's report:\n%s\nstderr: %s", stdout, stderr)
		}
	})
	for _, want := range []string{"tracked_changed_unexplained 1", "post_inventory_unexplained 0", "unknown write set: " + actD, "unknown write set: " + actC + ": writes_complete is false"} {
		if !strings.Contains(stdout, want) {
			t.Errorf("collect's report lacks %q:\n%s", want, stdout)
		}
	}

	// The shard decodes with recfmt exactly as the merge reads it.
	files4 := recfmt.ShardFiles{}
	for _, f := range []struct {
		rel string
		dst *[]byte
	}{{"binding.json", &files4.Binding}, {"host-facts.json", &files4.HostFacts}, {"inventory.json", &files4.Inventory}} {
		if *f.dst, err = os.ReadFile(filepath.Join(recDir, "pre", f.rel)); err != nil {
			t.Fatal(err)
		}
	}
	rf, err := os.Open(out)
	if err != nil {
		t.Fatal(err)
	}
	defer rf.Close()
	files4.Records = rf
	sh, err := recfmt.DecodeShard(0, files4, recfmt.DefaultLimits)
	if err != nil {
		t.Fatalf("the shard does not decode: %v", err)
	}
	h := sh.Header
	if len(h.Void) != 1 || h.Void[0] != "tracked-changed" || h.TrackedChangedUnexplained != 1 || h.PostInventoryUnexplained != 0 {
		t.Fatalf("header %+v", h)
	}
	if len(sh.Records) != 6 {
		t.Fatalf("%d records", len(sh.Records))
	}
	rec := func(a string) recfmt.ShardRecord {
		t.Helper()
		for _, r := range sh.Records {
			if r.Action == a {
				return r
			}
		}
		t.Fatalf("no record for %s", a)
		return recfmt.ShardRecord{}
	}
	complete := func(r recfmt.ShardRecord) bool { return r.Complete != nil && *r.Complete }

	// A: a clean hit.
	ra := rec(actA)
	hasPkg := false
	for _, e := range ra.Record.Entries {
		hasPkg = hasPkg || e == "pkg:"+actA
	}
	if ra.Unknown != "" || ra.Status != "built" || ra.StepsRun != 1 || !complete(ra) || !hasPkg || len(ra.Steps) != 1 || !ra.Steps[0].Clean {
		t.Errorf("A: %+v", ra)
	}
	// B: a failed step. Its record is believed (a non-zero exit is not an unknown
	// write), and its status is the build tool's.
	rb := rec(actB)
	if rb.Unknown != "" || rb.Status != "failed" || rb.StepsRun != 1 || !complete(rb) {
		t.Errorf("B: %+v", rb)
	}
	// C: believed, but the classifier could not place the write.
	rc := rec(actC)
	if rc.Unknown != "" || rc.Status != "built" || complete(rc) || rc.Complete == nil {
		t.Errorf("C: unknown %q, complete %v, reasons %q", rc.Unknown, rc.Complete, rc.Record.Reasons)
	}
	// D: the rerun.
	rd := rec(actD)
	if !strings.Contains(rd.Unknown, "run twice or out of order") || complete(rd) || len(rd.Record.Writes)+len(rd.Record.Entries) != 0 {
		t.Errorf("D: %+v", rd)
	}
	// E: itself clean.
	re := rec(actE)
	if re.Unknown != "" || re.Status != "built" || !complete(re) {
		t.Errorf("E: %+v", re)
	}

	// F: an empty recipe is vacuously complete: known, built, nothing run.
	rf2 := rec(actF)
	if rf2.Unknown != "" || rf2.Status != "built" || rf2.StepsRun != 0 || rf2.StepsTotal != 0 || !complete(rf2) {
		t.Errorf("F: %+v", rf2)
	}

	// The merge reads it: strict, the run is void, for the causes the verdicts
	// above say, and each action gets the verdict it should.
	steps := map[string]string{}
	var rels []string
	for _, a := range actions {
		pl, _ := pre.Action(a.rel)
		steps[a.rel] = recfmt.StepsDigest(pl.Steps)
		rels = append(rels, a.rel)
	}
	m := recmerge.Merge(recmerge.Input{
		Run:      recmerge.Run{Repository: "owner/repo", ID: "1", Attempt: 1, Revision: sha},
		Expected: recmerge.Expected{GraphSHA256: sh.Binding.GraphSHA256, StepsSHA256: sh.Binding.StepsSHA256, Plan: []recfmt.ShardPlan{{Index: 0, Packages: rels, Assigned: rels}}, Steps: steps},
		Shards:   []*recfmt.Shard{sh}, Keys: collectKeys{pre},
		Now: time.Now().UTC(), Kiln: "kiln/test", HonourWritesComplete: true,
	})
	if m.Emitted {
		t.Fatalf("a run with an unknown write set and an unexplained tracked change was emitted: %+v", m)
	}
	causes := strings.Join(m.RunVoid, "\n")
	if strings.Contains(causes, ": "+actF) {
		t.Errorf("the empty recipe was charged: %s", causes)
	}
	for _, want := range []string{
		"writes_complete is false: " + actC, // C
		"unknown_writes",                    // D
		"M2b shard 0: 1 changed tracked",    // E's change
		"status failed: " + actB,            // B, strict
	} {
		if !strings.Contains(causes, want) {
			t.Errorf("the merge's causes lack %q:\n%s", want, causes)
		}
	}
	for _, o := range m.Outcomes {
		switch o.Action {
		case actA:
			if !o.Recordable || len(o.Reasons) != 0 || o.Emitted {
				t.Errorf("A: %+v", o)
			}
		case actB:
			if !o.Truncated {
				t.Errorf("B is not a truncated execution: %+v", o)
			}
		case actE:
			if !o.ShardVoid || o.Emitted {
				t.Errorf("E: the shard is void, %+v", o)
			}
		case actF:
			// Not unknown (no M1 cause names it) and not emitted.
			if o.Emitted {
				t.Errorf("F: an empty recipe was emitted: %+v", o)
			}
		}
	}
}
