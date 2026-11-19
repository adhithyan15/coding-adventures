//go:build linux && amd64

package main

// TestSandboxRecordGates: gates and sandboxed toolchain probes, end to end
// (KILN04 section 4.8, decision P7, stage 4 PR 2b-2c-3).
//
// `kiln record prepare` runs the toolchain probes of every language in the graph
// IN THE SANDBOX (never on the host), on a configuration of their own with an
// empty checkout, and gives each action the gate the plan would. The fixture:
//
//	toolchains.json   fake  pins sed to 999.0.0, probed with `sed --version`
//	                  sh    always included; bash and sh pinned to the image
//	never.json        x/n
//
//	action  language  gate it must get
//	x/a     sh        ""                             recordable
//	x/g     fake      probe-mismatch:sed             the real sed is not 999.0.0
//	x/n     sh        never:x/n
//	x/u     nolang    unknown-toolchain:nolang       (also a refusal: no weak key)
//	x/v     greplang  ""                             grep pinned to the host's own
//	                  version: ok only if the sandboxed probe printed it
//
// Asserted: the action files and pre/probes.json (sed mismatches and its detail
// is REAL tool output, not a refusal or a timeout; bash and grep are ok),
// the probes' own sandbox configuration (an empty checkout, no state, no masks,
// HOME replaced), that every step still runs, the gates after `step` and
// `collect`, and that after the merge only x/a and x/v are recordable.
//
// The gates and the probe statuses are logged with the marker "PROBE:" for the
// CI log. It is a Linux-only test in the `kiln-sandbox` CI job.

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// gatesToolchains is the fixture's toolchains.json. `greplang` pins grep to the
// EXACT version this host's grep prints, so its probe is a positive versioned
// case: it can only be `ok` if the sandboxed probe really ran grep and printed
// that version (an image pin says `ok` whatever the run returned, and any
// failure of a versioned probe is a mismatch).
func gatesToolchains(grepVersion string) string {
	return `{"fake":{"pins":{"sed":"999.0.0"},"probes":[["sed","--version"]],"roots":{"sed":["/usr/bin","/bin"]}},
 "greplang":{"pins":{"grep":"` + grepVersion + `"},"probes":[["grep","--version"]],"roots":{"grep":["/usr/bin","/bin"]}},
 "sh":{"always":true,"pins":{"bash":"image","sh":"image"},"probes":[["bash","--version"]],"roots":{"bash":["/usr/bin","/bin"],"sh":["/usr/bin","/bin"]}}}
`
}

// hostGrepVersion runs `grep --version` on the host and parses the version out
// of its first line (`grep (GNU grep) 3.11`).
func hostGrepVersion(t *testing.T) string {
	t.Helper()
	out, err := exec.Command("grep", "--version").Output()
	if err != nil {
		skipOrFail(t, "grep --version: %v", err)
	}
	first, _, _ := strings.Cut(string(out), "\n")
	m := regexp.MustCompile(`\d+(\.\d+)+`).FindString(first)
	if m == "" {
		skipOrFail(t, "cannot read a version from %q", first)
	}
	return m
}

func TestSandboxRecordGates(t *testing.T) {
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
	realHome, err := os.UserHomeDir()
	if err != nil || !filepath.IsAbs(realHome) || strings.HasPrefix(realHome, "/tmp/") || realHome == "/tmp" {
		skipOrFail(t, "no home directory outside /tmp to use as the step's $HOME: %q %v", realHome, err)
	}
	base, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	home, err := os.MkdirTemp(realHome, ".kiln-gates-test-")
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
	kiln := newProbeKiln(t, bin)

	type action struct{ rel, name, lang, gate string }
	actions := []action{
		{"code/packages/x/a", "x/a", "sh", ""},
		{"code/packages/x/g", "x/g", "fake", "probe-mismatch:sed"},
		{"code/packages/x/n", "x/n", "sh", "never:x/n"},
		{"code/packages/x/u", "x/u", "nolang", "unknown-toolchain:nolang"},
		{"code/packages/x/v", "x/v", "greplang", ""},
	}
	files := recStepPolicyFiles(imageRoots())
	files["code/kiln/legacy/toolchains.json"] = gatesToolchains(hostGrepVersion(t))
	files["code/kiln/legacy/never.json"] = `{"packages": ["x/n"]}`
	files["code/programs/go/build-tool/go.mod"] = "module example.com/bt\n\ngo 1.26\n"
	files["code/programs/go/build-tool/main.go"] = "package main\n"
	for _, a := range actions {
		files[a.rel+"/BUILD"] = "echo hi; cat data.txt\n"
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

	b := "BUILD"
	var pkgs []legacy.Package
	var stepRows []map[string]any
	var planPkgs, names []string
	for _, a := range actions {
		pkgs = append(pkgs, legacy.Package{
			Name: a.name, RelPath: a.rel, Language: a.lang,
			BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
			Deps:       map[string][]string{"linux": {}, "darwin": {}, "windows": {}},
			DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
		})
		stepRows = append(stepRows, map[string]any{"package": a.rel, "steps": []string{"echo hi; cat data.txt"}})
		planPkgs = append(planPkgs, fmt.Sprintf(`{"name": %q, "rel_path": %q, "language": %q}`, a.name, a.rel, a.lang))
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

	logProcReadsOnFailure(t, recDir)
	prep := exec.Command(kiln, "record", "prepare", "--root", repo, "--dir", recDir, "--build-tool", filepath.Join(bin, "build-tool"),
		"--shard-index", "0", "--shard-count", "1", "--job-start", fmt.Sprint(time.Now().Unix()),
		"--strace", straceBin, "--git", gitBin)
	prep.Dir = "/"
	prep.Env = []string{
		"PATH=/usr/bin:/bin", "HOME=" + home,
		"GITHUB_ACTIONS=true", "GITHUB_REF=refs/heads/main", "GITHUB_EVENT_NAME=schedule",
		"GITHUB_EVENT_PATH=" + filepath.Join(aux, "event.json"), "ImageOS=ubuntu24", "ImageVersion=20260101.1",
		"GITHUB_SHA=" + sha, "GITHUB_REPOSITORY=owner/repo", "GITHUB_RUN_ID=1", "GITHUB_RUN_ATTEMPT=1",
	}
	t.Cleanup(func() { // a sealed directory is read-only
		filepath.Walk(recDir, func(p string, info os.FileInfo, err error) error {
			if err == nil {
				os.Chmod(p, 0o700)
			}
			return nil
		})
	})
	// D34: when the test ends, cross-check every trace the children wrote (registered
	// after the temp dir, so it runs before the directory is deleted).
	crossCheckChildTraces(t, recDir)
	if out, eb, code := recStepRun(t, prep); code != 0 {
		t.Fatalf("record prepare: exit %d\nstdout: %s\nstderr: %s", code, out, eb)
	}
	pre, err := record.VerifyPre(recDir)
	if err != nil {
		t.Fatal(err)
	}

	// The action files: the gate the plan would give, sealed.
	for _, a := range actions {
		pl, ok := pre.Action(a.rel)
		if !ok {
			t.Fatalf("no action file for %s", a.rel)
		}
		t.Logf("PROBE: gate %s (%s) = %q refusal=%q", a.name, a.lang, pl.Gate, pl.Refusal)
		if pl.Gate != a.gate || pl.Schema != "kiln-record-action/3" {
			t.Errorf("%s: %+v, want gate %q", a.name, pl, a.gate)
		}
		if (a.lang == "nolang") != (pl.Refusal != "") {
			t.Errorf("%s: refusal %q", a.name, pl.Refusal)
		}
	}

	// probes.json: the version of sed on the image is not 999.0.0; bash is only
	// pinned to the image, so it runs and is ok.
	status := map[string]string{}
	for _, tp := range pre.Probes.Toolchains {
		status[tp.Tool] = tp.Status
		t.Logf("PROBE: toolchain %s %s %q", tp.Tool, tp.Status, tp.Detail)
	}
	if status["sed"] != legacy.ProbeMismatch || status["bash"] != legacy.ProbeOK || status["grep"] != legacy.ProbeOK || len(status) != 3 {
		t.Errorf("toolchain probes %+v", pre.Probes.Toolchains)
	}
	// A mismatch is also what a BROKEN probe sandbox looks like, so its detail
	// must be the tool's own output: this separates "the sandbox ran sed and it
	// is not 999.0.0" from "the sandbox never ran it".
	for _, tp := range pre.Probes.Toolchains {
		if tp.Tool != "sed" {
			continue
		}
		if !strings.Contains(tp.Detail, "GNU sed") || strings.Contains(tp.Detail, "refused") || strings.Contains(tp.Detail, "did not finish") {
			t.Errorf("sed's detail is not real tool output: %q", tp.Detail)
		}
	}

	// The probes' own configuration: an empty checkout, nothing writable.
	cfgPath := filepath.Join(recDir, record.ProbeDir, record.ProbeConfigFile)
	raw, err := os.ReadFile(cfgPath)
	if err != nil {
		t.Fatal(err)
	}
	pc, err := sandbox.ParseConfig(raw)
	if err != nil {
		t.Fatal(err)
	}
	if pc.RepoRoot != filepath.Join(recDir, record.ProbeDir, record.ProbeRootDir) || len(pc.StateRoots) != 0 || len(pc.Masks) != 0 ||
		pc.Home != pre.Config.Home || pc.Tmpdir != filepath.Join(recDir, record.ProbeDir, record.ProbeTmpDir) || pc.Tmpdir == pre.Config.Tmpdir || strings.Join(pc.RoRoots, ",") != strings.Join(pre.Config.RoRoots, ",") {
		t.Errorf("probe configuration %+v", pc)
	}
	t.Logf("PROBE: probe tmpdir %s (steps tmpdir %s)", pc.Tmpdir, pre.Config.Tmpdir)
	if env := pc.EnvByAction["p"]; len(pc.EnvByAction) != 1 || !containsString(env, "HOME="+pc.Home) || !containsString(env, "LC_ALL=C") {
		t.Errorf("probe environment %v", pc.EnvByAction)
	}
	if ents, err := os.ReadDir(filepath.Join(pc.RepoRoot, record.ProbeActionDir)); err != nil || len(ents) != 0 {
		t.Errorf("the probes' checkout is not empty: %v %v", ents, err)
	}

	// The build: every action still runs, gated or not.
	for _, a := range actions {
		env := append(append([]string{}, pre.Tools.BuildToolEnv...), "KILN_ACTION="+a.rel, "KILN_STEP=0", "KILN_STEP_COUNT=1")
		cmd := exec.Command(kiln, "record", "step", "--dir", recDir, "--", "sh", "-c", "echo hi; cat data.txt")
		cmd.Dir, cmd.Env = pre.Config.PackageDir(a.rel), env
		if _, eb, code := recStepRun(t, cmd); code != 0 {
			t.Fatalf("step of %s: exit %d: %s", a.name, code, eb)
		}
	}
	var results []map[string]any
	for _, a := range actions {
		results = append(results, map[string]any{"package": a.rel, "status": "built", "steps_run": 1, "steps_total": 1})
	}
	data, err := json.Marshal(map[string]any{"schema": "kiln-legacy-results/1", "packages": results})
	if err != nil {
		t.Fatal(err)
	}
	resultsPath := filepath.Join(base, "results.json")
	recStepWrite(t, resultsPath, string(data), 0o644)
	out := filepath.Join(outDir, "records.jsonl")
	col := exec.Command(kiln, "record", "collect", "--root", repo, "--dir", recDir, "--results", resultsPath, "--out", out)
	col.Dir, col.Env = "/", []string{"PATH=/usr/bin:/bin", "HOME=" + home}
	if stdout, stderr, code := recStepRun(t, col); code != 0 {
		t.Fatalf("record collect: exit %d\nstdout: %s\nstderr: %s", code, stdout, stderr)
	}

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
	for _, a := range actions {
		var rec *recfmt.ShardRecord
		for i := range sh.Records {
			if sh.Records[i].Action == a.rel {
				rec = &sh.Records[i]
			}
		}
		if rec == nil {
			t.Fatalf("no record for %s", a.name)
		}
		t.Logf("PROBE: record %s gate=%q status=%s", a.name, rec.Gate, rec.Status)
		if rec.Gate != a.gate || rec.Status != "built" {
			t.Errorf("%s: gate %q status %q, want gate %q built", a.name, rec.Gate, rec.Status, a.gate)
		}
	}

	// The merge: a gate is a precondition, so only x/a is recordable.
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
	for _, o := range m.Outcomes {
		t.Logf("PROBE: outcome %s recordable=%v reasons=%v", o.Action, o.Recordable, o.Reasons)
		reasons := strings.Join(o.Reasons, "\n")
		switch o.Action {
		case "code/packages/x/a", "code/packages/x/v":
			if !o.Recordable || len(o.Reasons) != 0 {
				logStepTraces(t, recDir, o.Action) // D32: a trace reason would be here
				t.Errorf("%s: %+v", o.Action, o)
			}
		case "code/packages/x/g":
			if o.Recordable || !strings.Contains(reasons, "precondition:probe-mismatch:sed") {
				t.Errorf("x/g: %+v", o)
			}
		case "code/packages/x/n":
			if o.Recordable || !strings.Contains(reasons, "precondition:never:x/n") {
				t.Errorf("x/n: %+v", o)
			}
		case "code/packages/x/u":
			if o.Recordable || !strings.Contains(reasons, "precondition:") {
				t.Errorf("x/u: %+v", o)
			}
		}
	}
}

func containsString(l []string, s string) bool {
	for _, e := range l {
		if e == s {
			return true
		}
	}
	return false
}
