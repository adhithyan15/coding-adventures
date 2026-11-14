//go:build linux && amd64

package main

// TestSandboxRecordStep: `kiln record prepare` and then `kiln record step`, with
// the real strace and the real sandbox (KILN04 section 4.8, decision P12).
//
// The unit tests of the supervisor (internal/record/step_unix_test.go) put a
// fake strace behind tools.json. This one is the real thing, and pins what a
// fake cannot: that the supervisor's argument list is accepted by the real
// strace, that the trace of the real sandbox parses in helper mode, that
// strace's own standard error really is empty on a healthy step (so "clean"
// is attainable), that the step's descriptors 1 and 2 really are pipes, that
// the relayed output reaches the caller, and that a BUILD cannot read the
// record directory or its own trace.
//
// Layout (siblings under one temporary directory, as the sandbox requires; the
// record directory, the checkout and the programs are all separate):
//
//	base/repo/            a committed fixture repository: one package, x/a
//	base/record/          --dir, absent until prepare creates it
//	base/bin/kiln         an UNINSTRUMENTED build of this program (under -cover
//	                      the root would write coverage files, which are traced)
//	base/bin/build-tool   a shell script that writes the three exports from
//	                      files in base/aux (the real build tool refuses a
//	                      fixture tree, see internal/record/buildtool_test.go)
//	$HOME/.kiln-step-*    the step's $HOME on the host; outside /tmp, because
//	                      the sandbox refuses a home inside it when a root lies
//	                      beneath home (M2), and a runner's home never is
//
// prepare needs a GitHub-shaped environment: the GITHUB_* variables and an
// event file naming the default branch.

import (
	"bytes"
	"crypto/sha256"
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
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

const recStepAction = "code/packages/x/a"

func recStepWrite(t *testing.T, p, content string, mode os.FileMode) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte(content), mode); err != nil {
		t.Fatal(err)
	}
}

// recStepGit runs git with a throw-away identity and no user configuration.
func recStepGit(t *testing.T, gitBin, home, dir string, args ...string) string {
	t.Helper()
	cmd := exec.Command(gitBin, args...)
	cmd.Dir = dir
	cmd.Env = []string{
		"HOME=" + home, "PATH=/usr/bin:/bin", "GIT_CONFIG_NOSYSTEM=1",
		"GIT_AUTHOR_NAME=t", "GIT_AUTHOR_EMAIL=t@example.com", "GIT_COMMITTER_NAME=t", "GIT_COMMITTER_EMAIL=t@example.com",
	}
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("git %s: %v\n%s", strings.Join(args, " "), err, out)
	}
	return strings.TrimSpace(string(out))
}

// recStepRun runs a command and returns its output and exit status.
func recStepRun(t *testing.T, cmd *exec.Cmd) (string, string, int) {
	t.Helper()
	var ob, eb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &ob, &eb
	err := cmd.Run()
	var ee *exec.ExitError
	switch {
	case err == nil:
		return ob.String(), eb.String(), 0
	case errors.As(err, &ee):
		return ob.String(), eb.String(), ee.ExitCode()
	}
	t.Fatalf("%v", err)
	return "", "", -1
}

func TestSandboxRecordStep(t *testing.T) {
	// Skips (or, with KILN_REQUIRE_SANDBOX=1, fails) when the kernel will not
	// give us a user namespace.
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
	home, err := os.MkdirTemp(realHome, ".kiln-step-test-")
	if err != nil {
		skipOrFail(t, "cannot make a directory under the home directory: %v", err)
	}
	t.Cleanup(func() { os.RemoveAll(home) })
	if home, err = filepath.EvalSymlinks(home); err != nil {
		t.Fatal(err)
	}
	// The step's TMPDIR is a fixed path (legacy.RecordTmpdir) that must be
	// absent or empty. Remove only an empty one; never delete somebody's data.
	if err := os.Remove(legacy.RecordTmpdir); err != nil && !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("%s is in the way (not empty?): %v", legacy.RecordTmpdir, err)
	}
	t.Cleanup(func() { os.Remove(legacy.RecordTmpdir) })

	repo, recDir, bin, aux := filepath.Join(base, "repo"), filepath.Join(base, "record"), filepath.Join(base, "bin"), filepath.Join(base, "aux")
	for _, d := range []string{repo, bin, aux} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}

	// kiln: uninstrumented, outside the checkout.
	kiln := filepath.Join(bin, "kiln")
	build := exec.Command(goBin, "build", "-o", kiln, ".")
	if out, err := build.CombinedOutput(); err != nil {
		t.Fatalf("go build: %v\n%s", err, out)
	}

	// The three step lines. The third one is the test of the isolation: from
	// inside the sandbox, the record directory, its sealed binding and this
	// step's own trace are not there.
	trace2 := record.StepFile(recDir, recStepAction, 2, "trace")
	lines := []string{
		"echo hi; cat data.txt",
		"readlink /proc/self/fd/1; readlink /proc/self/fd/2",
		fmt.Sprintf(`for p in %s %s %s; do if test -e "$p"; then echo "visible:$p"; fi; done; cat %s 2>/dev/null; exit 0`,
			recDir, filepath.Join(recDir, "pre", "binding.json"), trace2, trace2),
		// The step holds no descriptor but its own: 0, 1, 2 and the one ls opens
		// to read the directory. Fd 5 is strace's trace pipe in the supervisor's
		// world and must not exist here.
		"ls /proc/self/fd",
		// And it cannot write to that pipe: the redirection fails. The payload
		// is spelled with an empty string in the middle so the line itself, which
		// strace records, does not contain it.
		`if (echo for""ged >&5) 2>/dev/null; then echo wrote; else echo refused; fi`,
	}

	// The repository: the files `prepare` reads (the same set the unit tests
	// of prepare use) and one package.
	// The policy files come from recStepPolicyFiles (sorted and de-duplicated as
	// the strict decoder requires; record_step_policy_test.go decodes them on
	// every OS).
	files := recStepPolicyFiles(imageRoots())
	for p, c := range map[string]string{
		"code/programs/go/build-tool/go.mod":  "module example.com/bt\n\ngo 1.26\n",
		"code/programs/go/build-tool/main.go": "package main\n",
		recStepAction + "/BUILD":              lines[0] + "\n",
		recStepAction + "/data.txt":           "somefile content\n",
	} {
		files[p] = c
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
	pkgDir := filepath.Join(repo, filepath.FromSlash(recStepAction))

	// The fake build tool and its exports.
	b := "BUILD"
	graph, err := json.Marshal(legacy.Graph{Schema: legacy.GraphSchema, Packages: []legacy.Package{{
		Name: "x/a", RelPath: recStepAction, Language: "x",
		BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
		Deps:       map[string][]string{"linux": {}, "darwin": {}, "windows": {}},
		DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
	}}})
	if err != nil {
		t.Fatal(err)
	}
	stepsJSON, err := json.Marshal(map[string]any{"schema": "kiln-legacy-steps/1", "os": "linux",
		"packages": []map[string]any{{"package": recStepAction, "steps": lines}}})
	if err != nil {
		t.Fatal(err)
	}
	recStepWrite(t, filepath.Join(aux, "graph.json"), string(graph), 0o644)
	recStepWrite(t, filepath.Join(aux, "steps.json"), string(stepsJSON), 0o644)
	recStepWrite(t, filepath.Join(aux, "plan.json"), `{"schema_version": 1, "force": true,
  "packages": [{"name": "x/a", "rel_path": "`+recStepAction+`", "language": "x"}],
  "shards": [{"index": 0, "name": "s0", "assigned_packages": ["x/a"], "package_names": ["x/a"]}]}`, 0o644)
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

	// step: as the build tool's -exec-wrapper runs it.
	step := func(n int) (stdout, stderr string, code int, sc record.StepSidecar) {
		t.Helper()
		env := append(append([]string{}, pre.Tools.BuildToolEnv...),
			"KILN_ACTION="+recStepAction, fmt.Sprintf("KILN_STEP=%d", n), fmt.Sprintf("KILN_STEP_COUNT=%d", len(lines)))
		cmd := exec.Command(kiln, "record", "step", "--dir", recDir, "--", "sh", "-c", lines[n])
		cmd.Dir, cmd.Env = pkgDir, env
		stdout, stderr, code = recStepRun(t, cmd)
		data, err := os.ReadFile(record.StepFile(recDir, recStepAction, n, "json"))
		if err != nil {
			t.Fatalf("step %d: exit %d, no sidecar: %v\nstdout: %s\nstderr: %s", n, code, err, stdout, stderr)
		}
		if sc, err = record.DecodeStepSidecar(data); err != nil {
			t.Fatal(err)
		}
		return
	}
	mustClean := func(n int, sc record.StepSidecar, code int, stderr string) {
		t.Helper()
		if code != 0 || !sc.Clean || !sc.StepNetNS || !sc.Started || sc.StraceExit != 0 || sc.RootStatus != 0 || sc.Truncated ||
			!sc.StraceStderrEmpty || !sc.StraceStdoutEmpty || !sc.LineInPlan {
			t.Fatalf("step %d: exit %d, sidecar %+v\nstderr: %s", n, code, sc, stderr)
		}
	}

	// Step 0: a command that reads a file. Its output reaches the caller, and
	// the trace parses in helper mode.
	out0, err0, code0, sc0 := step(0)
	mustClean(0, sc0, code0, err0)
	if out0 != "hi\nsomefile content\n" || err0 != "" {
		t.Fatalf("relayed output %q, stderr %q", out0, err0)
	}
	tracePath := record.StepFile(recDir, recStepAction, 0, "trace")
	fh, err := os.Open(tracePath)
	if err != nil {
		t.Fatal(err)
	}
	defer fh.Close()
	tr, err := trace.ParseOptions(fh, pkgDir, trace.Options{HelperRoot: true, HelperExe: kiln})
	if err != nil {
		raw, _ := os.ReadFile(tracePath)
		if len(raw) > 6000 {
			raw = raw[len(raw)-6000:]
		}
		t.Fatalf("the trace was refused: %v\n...%s", err, raw)
	}
	if ex := tr.StepExit(); ex == nil || ex.Killed || ex.Code != 0 {
		t.Errorf("step exit %+v", ex)
	}
	// What `cat data.txt` leaves in the trace: an execve of cat by some process,
	// then an openat by THAT process of "data.txt". Event.Path is "as printed
	// (unresolved)", so the open is relative to the process's working directory
	// (Event.Cwd, or the directory descriptor's -y path when there is one); the
	// assertion resolves it and requires the package's data.txt, opened for
	// reading successfully, by the process that exec'd cat.
	dataTxt := filepath.Join(pkgDir, "data.txt")
	resolve := func(e trace.Event) string {
		if filepath.IsAbs(e.Path) {
			return filepath.Clean(e.Path)
		}
		base := e.Cwd
		if e.DirfdPath != "" {
			base = e.DirfdPath
		}
		return filepath.Join(base, e.Path)
	}
	catPid := -1
	for _, e := range tr.Events {
		if e.Syscall == "execve" && e.Result.OK && strings.HasSuffix(e.Path, "/cat") {
			catPid = e.Pid
		}
	}
	sawRead := false
	for _, e := range tr.Events {
		if e.Pid == catPid && (e.Syscall == "openat" || e.Syscall == "open") && e.Result.OK && resolve(e) == dataTxt {
			sawRead = true
		}
	}
	if catPid < 0 || !sawRead {
		// Show what the trace holds, so a failure in CI can be diagnosed from
		// the log: the events of every process that exec'd cat, and the raw lines
		// that mention data.txt.
		var ev strings.Builder
		for _, e := range tr.Events {
			if e.Pid == catPid || strings.Contains(e.Path, "data.txt") {
				fmt.Fprintf(&ev, "  line %d pid %d %s kind %v ok %v path %q cwd %q dirfd %q\n", e.Line, e.Pid, e.Syscall, e.Kind, e.Result.OK, e.Path, e.Cwd, e.DirfdPath)
			}
		}
		raw, _ := os.ReadFile(tracePath)
		var lines []string
		for _, l := range strings.Split(string(raw), "\n") {
			if strings.Contains(l, "data.txt") || strings.Contains(l, "/cat") {
				if len(l) > 300 {
					l = l[:300] + "..."
				}
				lines = append(lines, l)
			}
		}
		t.Errorf("cat pid %d, read of %s by it: %v\nparsed events:\n%sraw lines:\n%s", catPid, dataTxt, sawRead, ev.String(), strings.Join(lines, "\n"))
	}
	if sc0.TraceBytes == 0 {
		t.Error("an empty trace")
	}
	// The sidecar's digest is of the bytes the supervisor wrote from strace's
	// pipe (-o /proc/self/fd/5), and the trace ends in a newline.
	raw0, err := os.ReadFile(tracePath)
	if err != nil {
		t.Fatal(err)
	}
	if sum := sha256.Sum256(raw0); sc0.TraceSHA256 != hex.EncodeToString(sum[:]) || sc0.TraceBytes != int64(len(raw0)) || raw0[len(raw0)-1] != '\n' {
		t.Errorf("trace digest or size mismatch, or no final newline: %+v (%d bytes)", sc0, len(raw0))
	}

	// Step 1: the step's descriptors 1 and 2 are pipes.
	out1, err1, code1, sc1 := step(1)
	mustClean(1, sc1, code1, err1)
	got := strings.Fields(out1)
	if len(got) != 2 || !strings.HasPrefix(got[0], "pipe:[") || !strings.HasPrefix(got[1], "pipe:[") {
		t.Fatalf("the step's descriptors 1 and 2 are %q, want two pipes", out1)
	}

	// Step 2: the record directory, the sealed binding and the step's own
	// trace do not exist in the step's world.
	out2, err2, code2, sc2 := step(2)
	mustClean(2, sc2, code2, err2)
	if strings.TrimSpace(out2) != "" {
		t.Fatalf("the step could see what it must not:\n%s", out2)
	}

	// Step 3: only descriptors 0 to 2 (plus ls's own, 3) exist in the step.
	out3, err3, code3, sc3 := step(3)
	mustClean(3, sc3, code3, err3)
	for _, fd := range strings.Fields(out3) {
		if fd != "0" && fd != "1" && fd != "2" && fd != "3" {
			t.Fatalf("the step holds descriptor %s: %q", fd, out3)
		}
	}
	// Nothing is classified before the action's last step ends it (P13).
	if _, err := os.Stat(record.RecordFile(recDir, recStepAction)); err == nil {
		t.Fatal("record.json exists before the last step ran")
	}
	// Step 4: it cannot write to the trace pipe, and what it tried to write is
	// not in any trace. It is the last step, so it also classifies the action.
	out4, err4, code4, sc4 := step(4)
	mustClean(4, sc4, code4, err4)
	if strings.TrimSpace(out4) != "refused" {
		t.Fatalf("the step's write to descriptor 5: %q", out4)
	}
	for n := 0; n < len(lines); n++ {
		raw, err := os.ReadFile(record.StepFile(recDir, recStepAction, n, "trace"))
		if err != nil {
			t.Fatal(err)
		}
		if bytes.Contains(raw, []byte("forged")) {
			t.Fatalf("the trace of step %d holds what the step wrote to descriptor 5", n)
		}
	}

	// The serial order: every step has the next number, from 0.
	for n, sc := range []record.StepSidecar{sc0, sc1, sc2, sc3, sc4} {
		if sc.Seq != n {
			t.Errorf("step %d has seq %d", n, sc.Seq)
		}
	}
	// The last step classified the action at its end, with the real classifier
	// over the real traces (P13): record.json exists and is a record, not a
	// failure marker, and the parse half of G7 held for all five traces (the
	// root's exit lines, and the sealed configuration's digest in the root's and
	// the init's traced execve).
	if _, err := os.Stat(record.RecordFailedFile(recDir, recStepAction)); err == nil {
		why, _ := os.ReadFile(record.RecordFailedFile(recDir, recStepAction))
		t.Fatalf("record.failed: %s", why)
	}
	rawRec, err := os.ReadFile(record.RecordFile(recDir, recStepAction))
	if err != nil {
		t.Fatal(err)
	}
	cr, err := record.DecodeClassified(rawRec)
	if err != nil {
		t.Fatal(err)
	}
	if !cr.ParseOK || cr.Record == nil {
		t.Fatalf("the epilogue's parse half of G7 failed on real traces: %q", cr.ParseReasons)
	}
	if cr.StepsRun != len(lines) || !cr.StepNetNS || cr.BindingID != pre.BindingID {
		t.Errorf("steps_run %d, step_netns %v, binding %s", cr.StepsRun, cr.StepNetNS, cr.BindingID)
	}
	// The read of data.txt (a file of the action's own package) is the package
	// entry of the manifest.
	hasPkg := false
	for _, e := range cr.Record.Entries {
		hasPkg = hasPkg || e == "pkg:"+recStepAction
	}
	if !hasPkg {
		t.Errorf("the read of data.txt is not in the record's entries %q (reasons %q)", cr.Record.Entries, cr.Record.Reasons)
	}

	// A step is run once.
	cmd := exec.Command(kiln, "record", "step", "--dir", recDir, "--", "sh", "-c", lines[0])
	cmd.Dir = pkgDir
	cmd.Env = append(append([]string{}, pre.Tools.BuildToolEnv...), "KILN_ACTION="+recStepAction, "KILN_STEP=0", fmt.Sprintf("KILN_STEP_COUNT=%d", len(lines)))
	if _, eb, code := recStepRun(t, cmd); code != record.ExitRefused || !strings.Contains(eb, "already run") {
		t.Fatalf("a second run of step 0: exit %d, %s", code, eb)
	}

	// The exit status of the step is the supervisor's: a failing step fails.
	// (The plan has three steps and all have run; a fourth action would be
	// needed for another one, so this is covered by the unit tests.)
}
