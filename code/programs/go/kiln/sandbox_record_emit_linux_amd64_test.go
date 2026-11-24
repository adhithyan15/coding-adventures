//go:build linux && amd64

package main

// TestSandboxRecordEmit: the POSITIVE half of the merge job, end to end
// (KILN04 section 7.1, stage 4 PR 3b).
//
// TestSandboxRecordCollect proves the void half: a shard with an unknown write
// set, an unexplained tracked change or a failed action makes the run void and
// nothing reaches the ledger. That is the fail-closed direction and it is the
// more important one. This test proves the other direction, which nothing
// covered: a shard whose actions are all clean is emitted, its manifests and
// passes are written, and they come back off disk through the real stores.
//
// A run is void if ANY action is void, so a positive emit needs its own shard:
// two clean actions, each reading its own data.txt and writing nothing.
//
// It is a Linux-only test in the `kiln-sandbox` CI job (assert_sandbox_tests.py
// names it), like its void counterpart.

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/host"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

func TestSandboxRecordEmit(t *testing.T) {
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
		skipOrFail(t, "go is not installed")
	}
	if goBin, err = filepath.EvalSymlinks(goBin); err != nil {
		t.Fatal(err)
	}
	realHome, err := os.UserHomeDir()
	if err != nil {
		skipOrFail(t, "no user home directory: %v", err)
	}
	if realHome, err = filepath.EvalSymlinks(realHome); err != nil {
		t.Fatal(err)
	}

	base, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	home, err := os.MkdirTemp(realHome, ".kiln-emit-test-")
	if err != nil {
		skipOrFail(t, "cannot create a home directory outside the fixture: %v", err)
	}
	t.Cleanup(func() { os.RemoveAll(home) })
	if home, err = filepath.EvalSymlinks(home); err != nil {
		t.Fatal(err)
	}
	if err := os.Remove(legacy.RecordTmpdir); err != nil && !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("%s is in the way: %v", legacy.RecordTmpdir, err)
	}
	t.Cleanup(func() { os.Remove(legacy.RecordTmpdir) })

	repo := filepath.Join(base, "repo")
	recDir := filepath.Join(base, "record")
	bin := filepath.Join(base, "bin")
	aux := filepath.Join(base, "aux")
	outDir := filepath.Join(base, "out")
	for _, d := range []string{repo, bin, aux, outDir} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	kiln := filepath.Join(bin, "kiln")
	if b, err := exec.Command(goBin, "build", "-o", kiln, ".").CombinedOutput(); err != nil {
		t.Fatalf("go build: %v\n%s", err, b)
	}

	// Two clean actions. Each reads its own data.txt and writes nothing, so the
	// classifier reports a complete, known write set and the merge may emit.
	const (
		actA = "code/packages/x/a"
		actB = "code/packages/x/b"
	)
	type action struct {
		rel   string
		name  string
		steps []string
	}
	actions := []action{
		{actA, "x/a", []string{"cat data.txt"}},
		{actB, "x/b", []string{"cat data.txt"}},
	}

	files := recStepPolicyFiles(imageRoots())
	files["code/programs/go/build-tool/go.mod"] = "module example.com/bt\n\ngo 1.26\n"
	files["code/programs/go/build-tool/main.go"] = "package main\n"
	for _, a := range actions {
		files[a.rel+"/BUILD"] = a.steps[0] + "\n"
		files[a.rel+"/data.txt"] = "content of " + a.rel + "\n"
	}
	for p, content := range files {
		recStepWrite(t, filepath.Join(repo, filepath.FromSlash(p)), content, 0o644)
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

	// The fake build tool's exports: a graph, a steps list and a plan that puts
	// both actions in shard 0 of 1.
	b := "BUILD"
	var pkgs []legacy.Package
	var stepRows []map[string]any
	var planPkgs, names []string
	for _, a := range actions {
		pkgs = append(pkgs, legacy.Package{
			Name: a.name, RelPath: a.rel, Language: "sh",
			BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
			Deps:       map[string][]string{"linux": {}, "darwin": {}, "windows": {}},
			DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
		})
		stepRows = append(stepRows, map[string]any{"package": a.rel, "steps": a.steps})
		planPkgs = append(planPkgs, fmt.Sprintf(`{"name": %q, "rel_path": %q, "language": "sh"}`, a.name, a.rel))
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
	logProcReadsOnFailure(t, recDir)
	crossCheckChildTraces(t, recDir, kiln)
	prep := exec.Command(kiln, "record", "prepare", "--root", repo, "--dir", recDir, "--build-tool", filepath.Join(bin, "build-tool"),
		"--shard-index", "0", "--shard-count", "1", "--job-start", fmt.Sprint(time.Now().Unix()),
		"--strace", straceBin, "--git", gitBin)
	prep.Dir, prep.Env = "/", prepEnv
	if stdout, stderr, code := recStepRun(t, prep); code != 0 {
		t.Fatalf("record prepare: exit %d\nstdout: %s\nstderr: %s", code, stdout, stderr)
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

	// The build, serially (-jobs 1): each action's one step, through the
	// recorder's own step supervisor so the trace is the product's.
	for _, a := range actions {
		env := append(append([]string{}, pre.Tools.BuildToolEnv...),
			"KILN_ACTION="+a.rel, "KILN_STEP=0", "KILN_STEP_COUNT=1")
		cmd := exec.Command(kiln, "record", "step", "--dir", recDir, "--", "sh", "-c", a.steps[0])
		cmd.Dir, cmd.Env = pre.Config.PackageDir(a.rel), env
		if stdout, stderr, code := recStepRun(t, cmd); code != 0 {
			t.Fatalf("record step %s: exit %d\nstdout: %s\nstderr: %s", a.rel, code, stdout, stderr)
		}
	}
	type result struct {
		Package    string `json:"package"`
		Status     string `json:"status"`
		StepsRun   int    `json:"steps_run"`
		StepsTotal int    `json:"steps_total"`
	}
	var results []result
	for _, a := range actions {
		results = append(results, result{Package: a.rel, Status: "built", StepsRun: 1, StepsTotal: 1})
	}
	resultsPath := filepath.Join(base, "results.json")
	resultsJSON, err := json.Marshal(map[string]any{"schema": "kiln-legacy-results/1", "packages": results})
	if err != nil {
		t.Fatal(err)
	}
	recStepWrite(t, resultsPath, string(resultsJSON), 0o644)

	// collect, as the workflow runs it.
	out := filepath.Join(outDir, "records.jsonl")
	col := exec.Command(kiln, "record", "collect", "--root", repo, "--dir", recDir, "--results", resultsPath, "--out", out)
	col.Dir, col.Env = "/", []string{"PATH=/usr/bin:/bin", "HOME=" + home}
	stdout, stderr, code := recStepRun(t, col)
	if code != 0 {
		t.Fatalf("record collect: exit %d\nstdout: %s\nstderr: %s", code, stdout, stderr)
	}
	t.Cleanup(func() {
		if t.Failed() {
			t.Logf("collect's report:\n%s\nstderr:\n%s", stdout, stderr)
		}
	})

	// The shard artifact the CI job uploads: the three sealed pre/ files plus the
	// records collect wrote. The merge job reads exactly this.
	artDir := filepath.Join(base, "shard0")
	if err := os.Mkdir(artDir, 0o755); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{record.ShardBindingFile, record.ShardHostFactsFile, record.ShardInventoryFile} {
		data, err := os.ReadFile(filepath.Join(recDir, "pre", name))
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(artDir, name), data, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	recordsData, err := os.ReadFile(out)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(artDir, record.ShardRecordsFile), recordsData, 0o644); err != nil {
		t.Fatal(err)
	}

	// The merge job: the same code path `kiln record-merge` runs.
	ledgerDir := filepath.Join(base, "ledger")
	rep, err := record.Merge(record.MergeOptions{
		Root:       repo,
		Source:     source.NewGitAt(repo, gitBin, source.PinnedEnv()),
		GraphPath:  filepath.Join(recDir, "pre", "graph.json"),
		StepsPath:  filepath.Join(recDir, "pre", "steps.json"),
		PlanPath:   filepath.Join(recDir, "pre", "build-plan.json"),
		ShardCount: 1,
		Shards:     []record.ShardArtifact{{Index: 0, Dir: artDir}},
		Run:        recmerge.Run{Repository: "owner/repo", ID: "1", Attempt: 1, Revision: sha},
		Kiln:       "kiln/test traced/2 recorder/13",
		Now:        time.Now().UTC(),
		LedgerDir:  ledgerDir,
		Scope:      ledger.Local,
		Authority:  host.NewLocal(io.Discard),
	})
	if err != nil {
		t.Fatalf("record.Merge: %v", err)
	}
	if !rep.Verdict.Emitted {
		t.Fatalf("a clean run was not emitted: %v", rep.Verdict.RunVoid)
	}
	if !rep.Written {
		t.Fatal("an emitted run was not written to the ledger")
	}
	if rep.Manifests != len(actions) || rep.Passes != len(actions) {
		t.Fatalf("%d manifests and %d passes for %d actions", rep.Manifests, rep.Passes, len(actions))
	}
	if rep.Recordable != len(actions) || rep.Actions != len(actions) {
		t.Errorf("%d recordable of %d actions, want %d of %d", rep.Recordable, rep.Actions, len(actions), len(actions))
	}

	// The passes read back off disk, by key. This is the join the ledger exists
	// for: a key written by the merge must be found by a later lookup.
	keys := make([][32]byte, 0, len(rep.Verdict.Passes))
	want := map[[32]byte]string{}
	for _, p := range rep.Verdict.Passes {
		keys = append(keys, p.Key)
		want[p.Key] = p.Target
	}
	got, err := ledger.Open(ledgerDir, ledger.Local).Lookup(keys)
	if err != nil {
		t.Fatalf("Lookup: %v", err)
	}
	if len(got) != len(keys) {
		t.Fatalf("Lookup returned %d of %d passes", len(got), len(keys))
	}
	for key, target := range want {
		r, ok := got[key]
		if !ok {
			t.Errorf("the pass for %s is not in the ledger", target)
			continue
		}
		if r.Kind != ledger.KindPass {
			t.Errorf("the record for %s is kind %q, want %q", target, r.Kind, ledger.KindPass)
		}
		if r.Target != target {
			t.Errorf("the record for %s names target %q", target, r.Target)
		}
		if r.Revision != sha {
			t.Errorf("the record for %s names revision %q, want %q", target, r.Revision, sha)
		}
	}

	// The manifests read back off disk too, keyed by the weak key. A pass without
	// its manifest is inert (stage 5 needs the manifest's entries to decide what
	// to re-read), so a written pass with no manifest would be a silent hole.
	wanted := make(map[[32]byte]struct{}, len(rep.Verdict.Manifests))
	for _, m := range rep.Verdict.Manifests {
		wanted[m.Weak] = struct{}{}
	}
	set, _, err := ledger.OpenManifests(ledgerDir, ledger.Local).Load(time.Now().UTC(), wanted)
	if err != nil {
		t.Fatalf("Load manifests: %v", err)
	}
	for _, m := range rep.Verdict.Manifests {
		list := set.Manifests(m.Weak)
		if len(list) == 0 {
			t.Errorf("the manifest for %s is not in the ledger", m.Target)
			continue
		}
		found := false
		for _, got := range list {
			if got.Target != m.Target || got.ID() != m.ID() {
				continue
			}
			found = true
			pkg := "pkg:" + m.Target
			named := false
			for _, e := range got.Entries {
				if e == pkg {
					named = true
				}
			}
			if !named {
				t.Errorf("the manifest for %s does not name %q: %v", m.Target, pkg, got.Entries)
			}
		}
		if !found {
			t.Errorf("the manifest for %s came back changed", m.Target)
		}
	}

	// The two stores keep separate files: passes as segments in the ledger
	// directory, manifests in its manifests/ subdirectory.
	if _, err := os.Stat(filepath.Join(ledgerDir, "manifests")); err != nil {
		t.Errorf("the ledger has no manifests/ subdirectory: %v", err)
	}
	segments := 0
	ents, err := os.ReadDir(ledgerDir)
	if err != nil {
		t.Fatal(err)
	}
	for _, e := range ents {
		if strings.HasSuffix(e.Name(), ".jsonl") {
			segments++
		}
	}
	if segments == 0 {
		t.Error("the ledger has no pass segment")
	}
}
