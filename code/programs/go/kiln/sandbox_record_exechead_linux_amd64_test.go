//go:build linux && amd64

package main

// TestSandboxRecordExecHead: `kiln record step --exec-head` end to end, with the
// real strace and the real sandbox, and `kiln record collect` over the result
// (KILN04 D48 step 5).
//
// The harness tests of step 4 (TestSandboxExecHead*) drive the supervisor with
// the production wiring but not RunStep.  This one runs the product: `prepare`,
// the step through `kiln record step --exec-head`, the epilogue's classification
// and `collect`.
//
// The step is a Go-test-like scenario.  `go test` builds `pkg.test` under
// $TMPDIR/go-build*/b001/, runs it and deletes it, which is why every Go action
// was refused as `trace-incomplete:exec-unreadable:under-tmp`.  Here the test's
// own binary (a probe the repo owns, see the `sandbox-exechead-probe` command)
// plays the part: copy it under the step's fresh tmpfs, run it for a while (a
// real test binary runs far longer than the supervisor's confirm), delete it.
// It is deliberately NOT a renamed coreutils binary (lessons.d: Ubuntu 26.04's
// coreutils are one multi-call binary that dispatches on argv[0]).
//
// One action, run under `prepare --exec-head` (KILN04 D48 step 6: the choice is
// sealed in the binding, so a shard is wholly supervised or wholly not; the
// unsupervised control of step 5 is a second shard, prepared without the flag
// into its own record directory, at the end of the test):
//
//	H  x/h  believed: no exec-unreadable reason
//
// and a `--exec-head=false` step against the on seal, which must be refused. The
// control records the SAME action with no exec heads: refused as exec-unreadable.
//
// What is asserted, exactly:
//
//	the pass-through   the trace shows the root's argv carrying --exec-notify-fd 6
//	                   and the init's carrying 4, and strace's -y annotation of
//	                   descriptor 6 as a socket: the supervisor's socket reached the
//	                   sandbox THROUGH strace (descriptor 6, after stdout, stderr and
//	                   the trace pipe)
//	the digest fields  the step sidecar's exechead_nonce, exechead_sha256 and
//	                   exechead_bytes are the nonce in the file's header, the file's
//	                   SHA-256 and its length
//	the file           decodes strictly with the sidecar's nonce, the action's digest
//	                   and the step; has a `confirmed` line for the tmpfs binary
//	                   (leader, path as the step wrote it, head = the file's first
//	                   bytes) and an `outside` line for the host `cp`
//	collect            H's record is known and carries no exec-unreadable reason
//
// The believed-head assertion needs the series on the kernel table (7.0): on
// another kernel the same run must be refused (`exechead-kernel`), which the test
// asserts instead.

import (
	"crypto/sha256"
	"encoding/hex"
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

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

var onKernelTable = regexp.MustCompile(`^7\.0([.\-+~_]|$)`)

func TestSandboxRecordExecHead(t *testing.T) {
	fix := newSandboxFixture(t) // skips (or, with KILN_REQUIRE_SANDBOX=1, fails) without user namespaces
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
	home, err := os.MkdirTemp(realHome, ".kiln-exechead-test-")
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

	const (
		actH    = "code/packages/x/h"
		probeFn = "probe-bin"
	)
	// The Go-test-like step: build under the fresh tmpfs, run, delete.  The probe
	// is tracked in the package (it is this test binary), so the copy is a plain
	// read of a tracked file.
	line := `d="$TMPDIR/go-build/b001"; mkdir -p "$d" && cp ./` + probeFn + ` "$d/x.test" && "$d/x.test" ` + execHeadProbeCommand + ` hold 600; rm -rf "$TMPDIR/go-build"`
	type action struct{ rel, name string }
	actions := []action{{actH, "x/h"}}

	probeBytes, err := os.ReadFile(fix.exe)
	if err != nil {
		t.Fatal(err)
	}
	files := recStepPolicyFiles(imageRoots())
	files["code/programs/go/build-tool/go.mod"] = "module example.com/bt\n\ngo 1.26\n"
	files["code/programs/go/build-tool/main.go"] = "package main\n"
	for _, a := range actions {
		files[a.rel+"/BUILD"] = line + "\n"
	}
	for p, c := range files {
		recStepWrite(t, filepath.Join(repo, filepath.FromSlash(p)), c, 0o644)
	}
	for _, a := range actions {
		if err := os.WriteFile(filepath.Join(repo, filepath.FromSlash(a.rel), probeFn), probeBytes, 0o755); err != nil {
			t.Fatal(err)
		}
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

	// The fake build tool's exports (as in TestSandboxRecordCollect).
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
		stepRows = append(stepRows, map[string]any{"package": a.rel, "steps": []string{line}})
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

	prepEnv := []string{
		"PATH=/usr/bin:/bin", "HOME=" + home,
		"GITHUB_ACTIONS=true", "GITHUB_REF=refs/heads/main", "GITHUB_EVENT_NAME=schedule",
		"GITHUB_EVENT_PATH=" + filepath.Join(aux, "event.json"), "ImageOS=ubuntu24", "ImageVersion=20260101.1",
		"GITHUB_SHA=" + sha, "GITHUB_REPOSITORY=owner/repo", "GITHUB_RUN_ID=1", "GITHUB_RUN_ATTEMPT=1",
	}
	prep := exec.Command(kiln, "record", "prepare", "--root", repo, "--dir", recDir, "--build-tool", filepath.Join(bin, "build-tool"),
		"--shard-index", "0", "--shard-count", "1", "--job-start", fmt.Sprint(time.Now().Unix()),
		"--strace", straceBin, "--git", gitBin, "--exec-head")
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

	if !pre.Binding.ExecHead {
		t.Fatal("prepare --exec-head did not seal exec_head into the binding")
	}
	// The step's environment, as the build tool would give it.
	stepEnv := func(rel string) []string {
		return append(append([]string{}, pre.Tools.BuildToolEnv...), "KILN_ACTION="+rel, "KILN_STEP=0", "KILN_STEP_COUNT=1")
	}
	// A step whose own flag disagrees with the seal is refused, and runs nothing.
	{
		cmd := exec.Command(kiln, "record", "step", "--dir", recDir, "--exec-head=false", "--", "sh", "-c", line)
		cmd.Dir, cmd.Env = pre.Config.PackageDir(actH), stepEnv(actH)
		if _, eb, code := recStepRun(t, cmd); code != record.ExitRefused || !strings.Contains(eb, "disagrees with the sealed binding") {
			t.Fatalf("--exec-head=false against an on seal: exit %d, want %d (refused)\nstderr: %s", code, record.ExitRefused, eb)
		}
	}
	// The build: the wrapper's own arguments (prepare put --exec-head in
	// exec-wrapper.json), which the seal agrees with.
	runStep := func(rel string) record.StepSidecar {
		t.Helper()
		env := stepEnv(rel)
		args := []string{"record", "step", "--dir", recDir, "--exec-head"}
		cmd := exec.Command(kiln, append(args, "--", "sh", "-c", line)...)
		cmd.Dir, cmd.Env = pre.Config.PackageDir(rel), env
		stdout, stderr, code := recStepRun(t, cmd)
		if code != 0 {
			t.Fatalf("step of %s: exit %d\nstdout: %s\nstderr: %s", rel, code, stdout, stderr)
		}
		data, err := os.ReadFile(record.StepFile(recDir, rel, 0, "json"))
		if err != nil {
			t.Fatalf("no sidecar for %s: %v", rel, err)
		}
		sc, err := record.DecodeStepSidecar(data)
		if err != nil {
			t.Fatal(err)
		}
		if !sc.Clean || !sc.Started || !sc.StepNetNS || sc.StraceExit != 0 || !sc.StraceStderrEmpty || !sc.StraceStdoutEmpty {
			logStepTraces(t, recDir, rel)
			t.Fatalf("the step of %s is not clean: %+v\nstdout: %s\nstderr: %s", rel, sc, stdout, stderr)
		}
		return sc
	}
	scH := runStep(actH)

	// ---- the digest fields of the step sidecar, and the file they bind.
	ehPath := record.StepFile(recDir, actH, 0, "exechead")
	raw, err := os.ReadFile(ehPath)
	if err != nil {
		t.Fatalf("no exec-head file for the supervised step: %v", err)
	}
	sum := sha256.Sum256(raw)
	if !exechead.ValidNonce(scH.ExecHeadNonce) || scH.ExecHeadSHA256 != hex.EncodeToString(sum[:]) || scH.ExecHeadBytes != int64(len(raw)) || len(raw) == 0 {
		t.Fatalf("the step sidecar's exec-head fields do not bind the file: nonce %q sha %q bytes %d; the file is %d bytes with sha %x",
			scH.ExecHeadNonce, scH.ExecHeadSHA256, scH.ExecHeadBytes, len(raw), sum)
	}
	if scH.ExecHeadNotified < 1 || scH.ExecHeadKills != 0 {
		t.Fatalf("the supervisor's counts: notified %d, kills %d (a clean step hears at least its own execs and kills nothing)", scH.ExecHeadNotified, scH.ExecHeadKills)
	}

	// ---- the file, strictly decoded with what the recorder knows.
	act := sha256.Sum256([]byte(actH))
	file, err := exechead.Decode(raw, exechead.Limits{}, exechead.Expect{Nonce: scH.ExecHeadNonce, ActionSHA256: hex.EncodeToString(act[:]), Step: 0})
	if err != nil {
		t.Fatalf("the exec-head file does not decode: %v", err)
	}
	tmpBin := legacy.RecordTmpdir + "/go-build/b001/x.test"
	var confirmed *exechead.Line
	var sawCp bool
	for i, l := range file.Lines {
		t.Logf("exechead line %d: tid %d tgid %d %s %s %q", l.Seq, l.Tid, l.Tgid, l.Syscall, l.Status, l.Path)
		if l.Path == tmpBin && l.Status == exechead.StatusConfirmed {
			confirmed = &file.Lines[i]
		}
		if strings.HasSuffix(l.Path, "/cp") {
			sawCp = true
			if l.Status != exechead.StatusOutside {
				t.Errorf("the host cp is a placeholder, got %s", l.Status)
			}
		}
	}
	trusted := onKernelTable.MatchString(file.Header.Kernel)
	if file.Header.Kernel == "" {
		t.Fatal("the header carries no kernel")
	}
	if !sawCp {
		t.Error("no line for the host cp")
	}
	if trusted {
		if confirmed == nil {
			t.Fatalf("no confirmed line for %s (kernel %s)", tmpBin, file.Header.Kernel)
		}
		want := probeBytes[:min(len(probeBytes), exechead.HeadLimit)]
		if confirmed.Tid != confirmed.Tgid || string(confirmed.Head) != string(want) || !strings.HasPrefix(string(confirmed.Head), "\x7fELF") {
			t.Errorf("the confirmed line: tid %d tgid %d head %d bytes (the probe's first %d bytes are what the kernel loaded)",
				confirmed.Tid, confirmed.Tgid, len(confirmed.Head), len(want))
		}
	}

	// ---- the pass-through: strace passed descriptor 6 to the sandbox.
	tracePath := record.StepFile(recDir, actH, 0, "trace")
	rawTrace, err := os.ReadFile(tracePath)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(rawTrace), "6<socket:[") {
		t.Errorf("strace never printed descriptor 6 as a socket: the supervisor's socket did not reach the sandbox through strace")
	}
	tf, err := os.Open(tracePath)
	if err != nil {
		t.Fatal(err)
	}
	defer tf.Close()
	tr, err := trace.ParseOptions(tf, pre.Config.PackageDir(actH), trace.Options{HelperRoot: true, HelperExe: kiln})
	if err != nil {
		t.Fatalf("the supervised step's trace was refused: %v", err)
	}
	argAfter := func(argv []string, flag string) string {
		for i := 0; i+1 < len(argv); i++ {
			if argv[i] == flag {
				return argv[i+1]
			}
			if argv[i] == "--" {
				break
			}
		}
		return ""
	}
	if got := argAfter(tr.RootExecArgv, "--exec-notify-fd"); got != "6" {
		t.Errorf("the root's --exec-notify-fd is %q, want 6 (strace's descriptor after 3, 4 and 5): %q", got, tr.RootExecArgv)
	}
	if got := argAfter(tr.InitExecArgv, "--exec-notify-fd"); got != "4" {
		t.Errorf("the init's --exec-notify-fd is %q, want 4: %q", got, tr.InitExecArgv)
	}

	// ---- collect, as the workflow runs it.
	results := []map[string]any{}
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
	stdout, stderr, code := recStepRun(t, col)
	t.Cleanup(func() {
		if t.Failed() {
			t.Logf("collect's report:\n%s\nstderr: %s", stdout, stderr)
		}
	})
	if code != 0 {
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
	hasReason := func(r recfmt.ShardRecord, want string) bool {
		for _, x := range r.Record.Reasons {
			if x == want {
				return true
			}
		}
		return false
	}
	const execUnreadable = "trace-incomplete:exec-unreadable"
	rh := rec(actH)
	for _, r := range []recfmt.ShardRecord{rh} {
		if r.Unknown != "" {
			logStepTraces(t, recDir, r.Action)
			t.Fatalf("%s has unknown writes %q (the supervised file must not make the action unknown)", r.Action, r.Unknown)
		}
	}
	if trusted {
		// The point of the decision: the exec of the tmpfs binary is BELIEVED.
		if hasReason(rh, execUnreadable) {
			logStepTraces(t, recDir, actH)
			t.Fatalf("H (--exec-head) is still refused as exec-unreadable: %q", rh.Record.Reasons)
		}
	} else {
		// Off the table the same run must still be refused (the census says
		// exechead-kernel): never believed on a kernel nobody ran the facts on.
		if !hasReason(rh, execUnreadable) {
			t.Errorf("kernel %s is not on the table, but H was believed: %q", file.Header.Kernel, rh.Record.Reasons)
		}
	}

	// ---- the control: the SAME action, recorded by a second shard prepared WITHOUT
	// --exec-head (its own record directory), is refused as exec-unreadable.  That
	// is what proves the supervised head, and nothing else, made H pass above.
	if err := os.RemoveAll(legacy.RecordTmpdir); err != nil {
		t.Fatalf("clearing the recorder's tmpdir for the control: %v", err)
	}
	recDirC := filepath.Join(base, "record-control")
	prepC := exec.Command(kiln, "record", "prepare", "--root", repo, "--dir", recDirC, "--build-tool", filepath.Join(bin, "build-tool"),
		"--shard-index", "0", "--shard-count", "1", "--job-start", fmt.Sprint(time.Now().Unix()),
		"--strace", straceBin, "--git", gitBin)
	prepC.Dir, prepC.Env = "/", prepEnv
	if so, eb, code := recStepRun(t, prepC); code != 0 {
		t.Fatalf("record prepare (control): exit %d\nstdout: %s\nstderr: %s", code, so, eb)
	}
	t.Cleanup(func() { // the sealed directory is read-only
		filepath.Walk(recDirC, func(p string, info os.FileInfo, err error) error {
			if err == nil {
				os.Chmod(p, 0o700)
			}
			return nil
		})
	})
	preC, err := record.VerifyPre(recDirC)
	if err != nil {
		t.Fatal(err)
	}
	if preC.Binding.ExecHead {
		t.Fatal("the control's binding is sealed with exec_head")
	}
	// No --exec-head anywhere: the wrapper of an off seal has none, and a step takes
	// the off choice from the seal.
	stepC := exec.Command(kiln, "record", "step", "--dir", recDirC, "--", "sh", "-c", line)
	stepC.Dir = preC.Config.PackageDir(actH)
	stepC.Env = append(append([]string{}, preC.Tools.BuildToolEnv...), "KILN_ACTION="+actH, "KILN_STEP=0", "KILN_STEP_COUNT=1")
	if so, eb, code := recStepRun(t, stepC); code != 0 {
		t.Fatalf("the control's step: exit %d\nstdout: %s\nstderr: %s", code, so, eb)
	}
	if _, err := os.Lstat(record.StepFile(recDirC, actH, 0, "exechead")); err == nil {
		t.Fatal("the control left an exec-head file")
	}
	outC := filepath.Join(outDir, "records-control.jsonl")
	colC := exec.Command(kiln, "record", "collect", "--root", repo, "--dir", recDirC, "--results", resultsPath, "--out", outC)
	colC.Dir, colC.Env = "/", []string{"PATH=/usr/bin:/bin", "HOME=" + home}
	if so, eb, code := recStepRun(t, colC); code != 0 {
		t.Fatalf("record collect (control): exit %d\nstdout: %s\nstderr: %s", code, so, eb)
	}
	filesC := recfmt.ShardFiles{}
	for _, f := range []struct {
		rel string
		dst *[]byte
	}{{"binding.json", &filesC.Binding}, {"host-facts.json", &filesC.HostFacts}, {"inventory.json", &filesC.Inventory}} {
		if *f.dst, err = os.ReadFile(filepath.Join(recDirC, "pre", f.rel)); err != nil {
			t.Fatal(err)
		}
	}
	rfC, err := os.Open(outC)
	if err != nil {
		t.Fatal(err)
	}
	defer rfC.Close()
	filesC.Records = rfC
	shC, err := recfmt.DecodeShard(0, filesC, recfmt.DefaultLimits)
	if err != nil {
		t.Fatalf("the control's shard does not decode: %v", err)
	}
	var rc *recfmt.ShardRecord
	for i := range shC.Records {
		if shC.Records[i].Action == actH {
			rc = &shC.Records[i]
		}
	}
	if rc == nil {
		t.Fatalf("no control record for %s", actH)
	}
	if rc.Unknown != "" {
		logStepTraces(t, recDirC, actH)
		t.Fatalf("the control has unknown writes %q", rc.Unknown)
	}
	if !hasReason(*rc, execUnreadable) {
		t.Errorf("the control (no exec heads) must be refused as exec-unreadable, as today: %q", rc.Record.Reasons)
	}
}
