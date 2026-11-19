//go:build linux && amd64

package main

// TestSandboxRecordSurvivor: a process of the sandbox that outlives the build
// makes the run void (KILN04 section 4.8, stage 4 PR 2b-2c-4, decision D6; merge
// rule M3).
//
// The test prepares a one-action shard, runs the action's one step through
// `kiln record step`, and then starts a sandbox of its own, directly:
//
//	kiln sandbox-exec --config <dir>/pre/sandbox.json --config-sha256 <sealed> \
//	    -- sh -c 'sleep 60'
//
// It is started WITHOUT `record step`, so it holds no flock and `collect`'s
// exclusive lock does not wait for it: it is exactly the orphan the check is
// for (a BUILD's backgrounded process, a supervisor whose build tool died).
// `collect` then has to see it: through its command line (S2, the sealed
// configuration is an argument of the sandbox's supervisor) and, for the
// `sleep` inside, through its mount namespace and mountinfo (S1).
//
// Asserted: the header void is exactly `survivor`; every record is an unknown
// write set that names it; the shard still decodes; collect's report names the
// finding. The PROBE: lines say what the kernel showed for each process of the
// tree, so a first CI run that fails says why (whether S1 or only S2 held).
//
// The sleeper is killed in cleanup (its process group, so the whole tree).
// Cost: collect waits its 10 s for the survivor to go away, and it does not.
//
// Linux-only, in the `kiln-sandbox` CI job; assert_sandbox_tests.py names it.

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

func TestSandboxRecordSurvivor(t *testing.T) {
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
	home, err := os.MkdirTemp(realHome, ".kiln-survivor-test-")
	if err != nil {
		skipOrFail(t, "cannot make a directory under the home directory: %v", err)
	}
	t.Cleanup(func() { os.RemoveAll(home) })
	if home, err = filepath.EvalSymlinks(home); err != nil {
		t.Fatal(err)
	}
	if err := os.Remove(legacy.RecordTmpdir); err != nil && !os.IsNotExist(err) {
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

	const rel, name = "code/packages/x/a", "x/a"
	files := recStepPolicyFiles(imageRoots())
	files["code/programs/go/build-tool/go.mod"] = "module example.com/bt\n\ngo 1.26\n"
	files["code/programs/go/build-tool/main.go"] = "package main\n"
	files[rel+"/BUILD"] = "cat data.txt\n"
	files[rel+"/data.txt"] = "content\n"
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
	graph, err := jsonOf(legacy.Graph{Schema: legacy.GraphSchema, Packages: []legacy.Package{{
		Name: name, RelPath: rel, Language: "sh",
		BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
		Deps:       map[string][]string{"linux": {}, "darwin": {}, "windows": {}},
		DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
	}}})
	if err != nil {
		t.Fatal(err)
	}
	recStepWrite(t, filepath.Join(aux, "graph.json"), graph, 0o644)
	recStepWrite(t, filepath.Join(aux, "steps.json"), `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[{"package":"`+rel+`","steps":["cat data.txt"]}]}`, 0o644)
	recStepWrite(t, filepath.Join(aux, "plan.json"), `{"schema_version": 1, "force": true, "packages": [{"name": "`+name+`", "rel_path": "`+rel+`", "language": "sh"}],
  "shards": [{"index": 0, "name": "s0", "assigned_packages": ["`+name+`"], "package_names": ["`+name+`"]}]}`, 0o644)
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
	// D34: when the test ends, cross-check every trace the children wrote. It is
	// registered BEFORE prepare runs (the temp dir already exists, so the check still
	// runs before it is deleted), so a probe trace that prepare refuses is checked too.
	crossCheckChildTraces(t, recDir, kiln)
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

	// The action's one step, as the build tool runs it.
	env := append(append([]string{}, pre.Tools.BuildToolEnv...), "KILN_ACTION="+rel, "KILN_STEP=0", "KILN_STEP_COUNT=1")
	step := exec.Command(kiln, "record", "step", "--dir", recDir, "--", "sh", "-c", "cat data.txt")
	step.Dir, step.Env = pre.Config.PackageDir(rel), env
	if _, eb, code := recStepRun(t, step); code != 0 {
		t.Fatalf("record step: exit %d\n%s", code, eb)
	}
	resultsPath := filepath.Join(base, "results.json")
	recStepWrite(t, resultsPath, `{"schema":"kiln-legacy-results/1","packages":[{"package":"`+rel+`","status":"built","steps_run":1,"steps_total":1}]}`, 0o644)

	// The survivor: a sandbox of the sealed configuration, started directly, so no
	// flock of the record directory is held for it. In its own process group, so
	// the cleanup can kill the whole tree.
	sleeper := exec.Command(kiln, "sandbox-exec", "--config", pre.ConfigPath, "--config-sha256", pre.Binding.SandboxConfigSHA256, "--", "sh", "-c", "sleep 60")
	sleeper.Dir = pre.Config.PackageDir(rel)
	sleeper.Env = []string{"KILN_ACTION=" + rel, "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"}
	sleeper.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	if err := sleeper.Start(); err != nil {
		t.Fatalf("starting the survivor: %v", err)
	}
	t.Cleanup(func() {
		syscall.Kill(-sleeper.Process.Pid, syscall.SIGKILL)
		sleeper.Process.Kill()
		sleeper.Wait()
		// Whatever is still named for the sealed configuration or the checkout is
		// reaped too, so a failed run leaves no sleeper behind.
		for _, p := range procsNaming(pre.ConfigPath) {
			syscall.Kill(p, syscall.SIGKILL)
		}
	})
	// Give the sandbox a moment to come up, so the PROBE lines show the whole
	// tree. (collect does not need this: the supervisor's command line names the
	// configuration from exec on, and the scan is repeated for 10 s.)
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) && !sleepSeen() {
		time.Sleep(100 * time.Millisecond)
	}
	for _, line := range survivorProbe(repo, pre.ConfigPath) {
		t.Logf("PROBE: %s", line)
	}

	out := filepath.Join(outDir, "records.jsonl")
	col := exec.Command(kiln, "record", "collect", "--root", repo, "--dir", recDir, "--results", resultsPath, "--out", out)
	col.Dir, col.Env = "/", []string{"PATH=/usr/bin:/bin", "HOME=" + home}
	start := time.Now()
	stdout, stderr, code := recStepRun(t, col)
	t.Logf("PROBE: collect took %s", time.Since(start).Round(100*time.Millisecond))
	t.Cleanup(func() {
		if t.Failed() {
			t.Logf("collect's report:\n%s\nstderr: %s", stdout, stderr)
		}
	})
	if code != 0 {
		t.Fatalf("record collect: exit %d\nstdout: %s\nstderr: %s", code, stdout, stderr)
	}
	for _, line := range strings.Split(stdout, "\n") {
		if strings.Contains(line, "survivor") {
			t.Logf("PROBE: collect said: %s", line)
		}
	}
	if !strings.Contains(stdout, "survivor") {
		t.Errorf("collect's report does not name the survivor:\n%s", stdout)
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
	if len(sh.Header.Void) != 1 || sh.Header.Void[0] != "survivor" {
		t.Fatalf("header void %v, want [survivor]", sh.Header.Void)
	}
	if len(sh.Records) != 1 {
		t.Fatalf("%d records", len(sh.Records))
	}
	for _, r := range sh.Records {
		if !strings.Contains(r.Unknown, "survivor") || r.Complete == nil || *r.Complete {
			t.Errorf("%s: unknown %q complete %v", r.Action, r.Unknown, r.Complete)
		}
	}
}

// jsonOf is json.Marshal as a string.
func jsonOf(v any) (string, error) {
	b, err := json.Marshal(v)
	return string(b), err
}

// procPIDs lists /proc's numeric entries.
func procPIDs() []int {
	entries, _ := os.ReadDir("/proc")
	var out []int
	for _, e := range entries {
		if n, err := strconv.Atoi(e.Name()); err == nil {
			out = append(out, n)
		}
	}
	return out
}

func procArgs(pid int) []string {
	data, err := os.ReadFile("/proc/" + strconv.Itoa(pid) + "/cmdline")
	if err != nil {
		return nil
	}
	return strings.Split(strings.TrimSuffix(string(data), "\x00"), "\x00")
}

// procsNaming lists the processes with arg as an element of their command line.
func procsNaming(arg string) []int {
	var out []int
	for _, p := range procPIDs() {
		for _, a := range procArgs(p) {
			if a == arg {
				out = append(out, p)
				break
			}
		}
	}
	return out
}

// sleepSeen says whether a `sleep 60` is running (in any namespace).
func sleepSeen() bool {
	for _, p := range procPIDs() {
		a := procArgs(p)
		if len(a) == 2 && filepath.Base(a[0]) == "sleep" && a[1] == "60" {
			return true
		}
	}
	return false
}

// survivorProbe says, for the processes of the sandbox's tree, what S1 and S2
// would see: the namespace, whether it is collect's, and whether its mountinfo
// lists the checkout.
func survivorProbe(repo, cfg string) []string {
	self, _ := os.Readlink("/proc/self/ns/mnt")
	out := []string{"this process's mount namespace " + self}
	for _, p := range procPIDs() {
		a := procArgs(p)
		named := false
		for _, x := range a {
			named = named || x == cfg
		}
		isSleep := len(a) == 2 && filepath.Base(a[0]) == "sleep" && a[1] == "60"
		if !named && !isSleep {
			continue
		}
		ns, nerr := os.Readlink("/proc/" + strconv.Itoa(p) + "/ns/mnt")
		mi, merr := os.ReadFile("/proc/" + strconv.Itoa(p) + "/mountinfo")
		has := false
		for _, line := range strings.Split(string(mi), "\n") {
			if f := strings.Fields(line); len(f) >= 5 && f[4] == repo {
				has = true
			}
		}
		out = append(out, fmt.Sprintf("pid %d args %q S2=%v ns=%q (err %v) other-ns=%v mountinfo-has-checkout=%v (err %v)",
			p, a, named, ns, nerr, ns != self, has, merr))
	}
	return out
}
