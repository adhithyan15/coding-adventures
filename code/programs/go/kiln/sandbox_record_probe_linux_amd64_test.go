//go:build linux && amd64

package main

// TestSandboxRecordProbe: `kiln record prepare` with the real in-sandbox
// structure probe (KILN04 section 4.8, decision P14).
//
// internal/record's unit tests replace the probe with a fake. This one runs
// the real thing: the real sandbox under the real strace, four times, and pins
// what no fake can:
//
//	success     every probe passes on a plain runner; pre/probes.json is sealed,
//	            names the runs, and carries the digest of each trace in
//	            <dir>/probe/; the signals self-check really sees the survivor
//	state root  a FIFO in a state root fails prepare, by the config's own
//	            state-preloaded check (it comes before any probe)
//	walk        the walk probe itself finds a FIFO in a state root: the
//	            exported runner is given a hand-built configuration whose state
//	            root holds one (sandbox-exec does not require empty state roots)
//	shadow      a FIFO hidden beneath a tmpfs mounted over a directory of a
//	            read-only root: the host scan skips what a mount hides, and
//	            either the shadow probe finds it or the kernel refuses to bind
//	            the root at all (locked child mounts); both fail closed, and the
//	            log says which (needs `sudo -n mount`, as the CI job has)
//	signals     a strace wrapper that adds `-e signal=none` makes prepare fail ON
//	            THE signals PROBE: a tracer that cannot show a signal death is
//	            not one the daemon-left rule can rest on
//
// Facts worth reading in the CI log are logged with the marker "PROBE:" (the
// assert script prints those lines for a passing test).
//
// The world (siblings under one temporary directory, as the sandbox requires):
//
//	base/repo/            a committed fixture repository: one package, x/a
//	base/record/          --dir, absent until prepare creates it
//	base/bin/             kiln (UNINSTRUMENTED: under -cover the root writes
//	                      coverage files, which would be traced), the fake build
//	                      tool and, for the third case, the strace wrapper
//	$HOME/.kiln-probe-*   the step's $HOME on the host, outside /tmp (M2)

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// probeWorld is one prepared-to-run fixture.
type probeWorld struct {
	base, repo, recDir, bin, home, kiln, strace, git, sha, event string
	state                                                        string // a state root's path, or ""
	shadowRoot                                                   string // an image root with a mount over its m/, or ""
}

// probeOpts shapes a world.
type probeOpts struct {
	state  bool // declare a state root (an empty directory)
	shadow bool // an image root whose m/ is a tmpfs mount hiding m/evil.fifo
}

// sudoMount mounts a tmpfs over dir and unmounts it in cleanup. Without a
// usable `sudo -n` it skips (or, with KILN_REQUIRE_SANDBOX=1, fails).
func sudoMount(t *testing.T, dir string) {
	t.Helper()
	if out, err := exec.Command("sudo", "-n", "mount", "-t", "tmpfs", "none", dir).CombinedOutput(); err != nil {
		skipOrFail(t, "cannot mount a tmpfs with sudo (%v): %s", err, strings.TrimSpace(string(out)))
	}
	t.Cleanup(func() {
		if out, err := exec.Command("sudo", "-n", "umount", dir).CombinedOutput(); err != nil {
			t.Errorf("umount %s: %v: %s", dir, err, out)
		}
	})
}

// newProbeKiln builds the uninstrumented kiln once per test.
func newProbeKiln(t *testing.T, dir string) string {
	t.Helper()
	goBin, err := exec.LookPath("go")
	if err != nil {
		skipOrFail(t, "the go tool is not available to build an uninstrumented kiln")
	}
	kiln := filepath.Join(dir, "kiln")
	if out, err := exec.Command(goBin, "build", "-o", kiln, ".").CombinedOutput(); err != nil {
		t.Fatalf("go build: %v\n%s", err, out)
	}
	return kiln
}

// newProbeWorld makes a world. opts.state declares a state root (an empty
// directory) in host-roots.json; its path is world.state.
func newProbeWorld(t *testing.T, kiln string, opts probeOpts) *probeWorld {
	t.Helper()
	w := &probeWorld{kiln: kiln}
	var err error
	if w.strace, err = exec.LookPath("strace"); err != nil {
		skipOrFail(t, "strace is not installed")
	}
	if w.strace, err = filepath.EvalSymlinks(w.strace); err != nil {
		t.Fatal(err)
	}
	if w.git, err = exec.LookPath("git"); err != nil {
		skipOrFail(t, "git is not installed")
	}
	if w.git, err = filepath.EvalSymlinks(w.git); err != nil {
		t.Fatal(err)
	}
	realHome, err := os.UserHomeDir()
	if err != nil || !filepath.IsAbs(realHome) || strings.HasPrefix(realHome, "/tmp/") || realHome == "/tmp" {
		skipOrFail(t, "no home directory outside /tmp to use as the step's $HOME: %q %v", realHome, err)
	}
	if w.base, err = filepath.EvalSymlinks(t.TempDir()); err != nil {
		t.Fatal(err)
	}
	if w.home, err = os.MkdirTemp(realHome, ".kiln-probe-test-"); err != nil {
		skipOrFail(t, "cannot make a directory under the home directory: %v", err)
	}
	t.Cleanup(func() { os.RemoveAll(w.home) })
	if w.home, err = filepath.EvalSymlinks(w.home); err != nil {
		t.Fatal(err)
	}
	// The step's TMPDIR is a fixed path that must be absent or empty. Remove only
	// an empty one; never delete somebody's data.
	if err := os.Remove(legacy.RecordTmpdir); err != nil && !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("%s is in the way (not empty?): %v", legacy.RecordTmpdir, err)
	}
	t.Cleanup(func() { os.Remove(legacy.RecordTmpdir) })

	w.repo, w.recDir, w.bin = filepath.Join(w.base, "repo"), filepath.Join(w.base, "record"), filepath.Join(w.base, "bin")
	aux := filepath.Join(w.base, "aux")
	for _, d := range []string{w.repo, w.bin, aux} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	if kiln == "" {
		t.Fatal("no kiln")
	}
	// The policy files: the fixture's only toolchain key is "sh", so the package's
	// language is "sh" (the policy decoders refuse a language they do not know).
	files := recStepPolicyFiles(imageRoots())
	var extra []string
	if opts.state {
		w.state = filepath.Join(w.base, "state")
		if err := os.Mkdir(w.state, 0o755); err != nil {
			t.Fatal(err)
		}
		extra = append(extra, fmt.Sprintf(`{"path": %q, "kind": "state", "ecosystem": "fixture"}`, w.state))
	}
	if opts.shadow {
		// The root is a sibling of the step's $HOME under the runner's real home:
		// Config.Validate refuses a read-only root inside /tmp or /var/tmp (and
		// w.base is a t.TempDir()). Its cleanup is registered BEFORE the mount's,
		// so the unmount runs first.
		var err error
		if w.shadowRoot, err = os.MkdirTemp(realHome, ".kiln-probe-shadow-"); err != nil {
			skipOrFail(t, "cannot make a directory under the home directory: %v", err)
		}
		t.Cleanup(func() { os.RemoveAll(w.shadowRoot) })
		if w.shadowRoot, err = filepath.EvalSymlinks(w.shadowRoot); err != nil {
			t.Fatal(err)
		}
		m := filepath.Join(w.shadowRoot, "m")
		if err := os.MkdirAll(m, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := syscall.Mkfifo(filepath.Join(m, "evil.fifo"), 0o600); err != nil {
			t.Fatal(err)
		}
		sudoMount(t, m) // the FIFO is now hidden on the host, and visible to the sandbox
		extra = append(extra, fmt.Sprintf(`{"path": %q, "kind": "image"}`, w.shadowRoot))
	}
	if len(extra) > 0 {
		hr := files["code/kiln/legacy/host-roots.json"]
		// Roots must be strictly sorted by path: put the state root where it sorts.
		var tc struct {
			Roots []json.RawMessage `json:"roots"`
		}
		if err := json.Unmarshal([]byte(hr), &tc); err != nil {
			t.Fatal(err)
		}
		var raw []string
		for _, r := range tc.Roots {
			raw = append(raw, string(r))
		}
		raw = append(raw, extra...)
		sortByPath(t, raw)
		files["code/kiln/legacy/host-roots.json"] = `{"roots": [` + strings.Join(raw, ",") + `], "ephemeral": ["/dev/null"], "absent-ok": []}`
	}
	const action = "code/packages/x/a"
	for p, c := range map[string]string{
		"code/programs/go/build-tool/go.mod":  "module example.com/bt\n\ngo 1.26\n",
		"code/programs/go/build-tool/main.go": "package main\n",
		action + "/BUILD":                     "echo hi\n",
	} {
		files[p] = c
	}
	for p, c := range files {
		recStepWrite(t, filepath.Join(w.repo, filepath.FromSlash(p)), c, 0o644)
	}
	recStepGit(t, w.git, w.home, w.repo, "init", "-q")
	recStepGit(t, w.git, w.home, w.repo, "config", "commit.gpgsign", "false")
	recStepGit(t, w.git, w.home, w.repo, "config", "core.autocrlf", "false")
	recStepGit(t, w.git, w.home, w.repo, "add", "-A")
	recStepGit(t, w.git, w.home, w.repo, "commit", "-q", "-m", "init")
	w.sha = recStepGit(t, w.git, w.home, w.repo, "rev-parse", "HEAD")
	if w.repo, err = filepath.EvalSymlinks(w.repo); err != nil {
		t.Fatal(err)
	}

	b := "BUILD"
	graph, err := json.Marshal(legacy.Graph{Schema: legacy.GraphSchema, Packages: []legacy.Package{{
		Name: "x/a", RelPath: action, Language: "sh",
		BuildFiles: map[string]*string{"linux": &b, "darwin": &b, "windows": &b},
		Deps:       map[string][]string{"linux": {}, "darwin": {}, "windows": {}},
		DevDeps:    []string{}, RepoInputs: []string{}, RepoInputGlobs: []string{}, ExtraToolchains: []string{},
	}}})
	if err != nil {
		t.Fatal(err)
	}
	recStepWrite(t, filepath.Join(aux, "graph.json"), string(graph), 0o644)
	recStepWrite(t, filepath.Join(aux, "steps.json"), `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[{"package":"`+action+`","steps":["echo hi"]}]}`, 0o644)
	recStepWrite(t, filepath.Join(aux, "plan.json"), `{"schema_version": 1, "force": true,
  "packages": [{"name": "x/a", "rel_path": "`+action+`", "language": "sh"}],
  "shards": [{"index": 0, "name": "s0", "assigned_packages": ["x/a"], "package_names": ["x/a"]}]}`, 0o644)
	w.event = filepath.Join(aux, "event.json")
	recStepWrite(t, w.event, `{"repository":{"default_branch":"main"}}`, 0o644)
	recStepWrite(t, filepath.Join(w.bin, "build-tool"), `#!/bin/sh
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
	t.Cleanup(func() { // a sealed directory is read-only
		filepath.Walk(w.recDir, func(p string, info os.FileInfo, err error) error {
			if err == nil {
				os.Chmod(p, 0o700)
			}
			return nil
		})
	})
	return w
}

// sortByPath orders host-roots entries (JSON objects) by their "path".
func sortByPath(t *testing.T, raw []string) {
	t.Helper()
	path := func(s string) string {
		var r struct {
			Path string `json:"path"`
		}
		if err := json.Unmarshal([]byte(s), &r); err != nil {
			t.Fatal(err)
		}
		return r.Path
	}
	for i := 1; i < len(raw); i++ {
		for j := i; j > 0 && path(raw[j-1]) > path(raw[j]); j-- {
			raw[j-1], raw[j] = raw[j], raw[j-1]
		}
	}
}

// prepare runs `kiln record prepare` with the given strace and returns its
// standard error and exit status.
func (w *probeWorld) prepare(t *testing.T, strace string) (string, int) {
	t.Helper()
	cmd := exec.Command(w.kiln, "record", "prepare", "--root", w.repo, "--dir", w.recDir, "--build-tool", filepath.Join(w.bin, "build-tool"),
		"--shard-index", "0", "--shard-count", "1", "--job-start", fmt.Sprint(time.Now().Unix()),
		"--strace", strace, "--git", w.git)
	cmd.Dir = "/"
	cmd.Env = []string{
		"PATH=/usr/bin:/bin", "HOME=" + w.home,
		"GITHUB_ACTIONS=true", "GITHUB_REF=refs/heads/main", "GITHUB_EVENT_NAME=schedule",
		"GITHUB_EVENT_PATH=" + w.event, "ImageOS=ubuntu24", "ImageVersion=20260101.1",
		"GITHUB_SHA=" + w.sha, "GITHUB_REPOSITORY=owner/repo", "GITHUB_RUN_ID=1", "GITHUB_RUN_ATTEMPT=1",
	}
	_, stderr, code := recStepRun(t, cmd)
	return stderr, code
}

// probeTails is the last lines of every probe trace, for a failure's log.
func (w *probeWorld) probeTails(n int) string {
	var b strings.Builder
	files, _ := filepath.Glob(filepath.Join(w.recDir, record.ProbeDir, "*.trace"))
	for _, f := range files {
		raw, _ := os.ReadFile(f)
		lines := strings.Split(strings.TrimRight(string(raw), "\n"), "\n")
		if len(lines) > n {
			lines = lines[len(lines)-n:]
		}
		fmt.Fprintf(&b, "--- %s (%d bytes)\n", filepath.Base(f), len(raw))
		for _, l := range lines {
			if len(l) > 300 {
				l = l[:300] + "..."
			}
			b.WriteString(l + "\n")
		}
	}
	return b.String()
}

func TestSandboxRecordProbe(t *testing.T) {
	// Skips (or, with KILN_REQUIRE_SANDBOX=1, fails) when the kernel will not
	// give us a user namespace.
	newSandboxFixture(t)
	kiln := newProbeKiln(t, t.TempDir())

	t.Run("success", func(t *testing.T) {
		w := newProbeWorld(t, kiln, probeOpts{})
		stderr, code := w.prepare(t, w.strace)
		if code != 0 {
			t.Fatalf("record prepare: exit %d\nstderr: %s\nprobe traces:\n%s", code, stderr, w.probeTails(25))
		}
		pre, err := record.VerifyPre(w.recDir)
		if err != nil {
			t.Fatal(err)
		}
		rep := pre.Probes
		var names []string
		for _, r := range rep.Runs {
			names = append(names, r.Name)
			raw, err := os.ReadFile(filepath.Join(w.recDir, record.ProbeDir, r.Name+".trace"))
			if err != nil {
				t.Fatal(err)
			}
			sum := sha256.Sum256(raw)
			if hex.EncodeToString(sum[:]) != r.TraceSHA256 || int64(len(raw)) != r.TraceBytes {
				t.Errorf("%s: the sealed digest or size is not the trace file's", r.Name)
			}
			t.Logf("PROBE: run %s argv=%d trace_bytes=%d", r.Name, len(r.Argv), r.TraceBytes)
		}
		t.Logf("PROBE: runs=%v shadowed=%v action=%s", names, rep.Shadowed, rep.Action)
		if rep.Action != "code/packages/x/a" || len(names) < 3 || names[0] != "pipes" || names[1] != "walk" || names[len(names)-1] != "signals" {
			t.Fatalf("runs %v", names)
		}
		if st, err := os.Lstat(filepath.Join(w.recDir, record.ProbeDir)); err != nil || st.Mode().Perm() != 0o700 {
			t.Errorf("probe directory: %v %v", st, err)
		}
		// Nothing of the probe is left where a step could see it.
		if entries, err := os.ReadDir(legacy.RecordTmpdir); err != nil || len(entries) != 0 {
			t.Errorf("tmpdir after the probes: %v %v", entries, err)
		}
	})

	t.Run("fifo in a state root", func(t *testing.T) {
		w := newProbeWorld(t, kiln, probeOpts{state: true})
		if err := syscall.Mkfifo(filepath.Join(w.state, "evil.fifo"), 0o600); err != nil {
			t.Fatal(err)
		}
		stderr, code := w.prepare(t, w.strace)
		if code == 0 {
			t.Fatalf("record prepare accepted a FIFO in a state root\nprobe traces:\n%s", w.probeTails(25))
		}
		// Which check stops it: the state root's own emptiness check comes first
		// (unscrubbed:state-preloaded); the probe's `walk` is the backstop for a
		// FIFO that appears after it.
		t.Logf("PROBE: fifo in a state root: exit %d: %s", code, firstLineOf(stderr))
		if !strings.Contains(stderr, "state-preloaded") && !strings.Contains(stderr, "probe walk") {
			t.Fatalf("it failed for another reason:\n%s", stderr)
		}
		if _, err := os.Lstat(filepath.Join(w.recDir, "pre", "SEALED")); err == nil {
			t.Fatal("a failed prepare left a SEALED record")
		}
	})

	t.Run("walk finds a FIFO in a state root", func(t *testing.T) {
		w := newProbeWorld(t, kiln, probeOpts{})
		if stderr, code := w.prepare(t, w.strace); code != 0 {
			t.Fatalf("record prepare: exit %d\nstderr: %s", code, stderr)
		}
		pre, err := record.VerifyPre(w.recDir)
		if err != nil {
			t.Fatal(err)
		}
		// A configuration like the sealed one, but with a state root that holds a
		// FIFO: prepare's own checks would refuse it, the exported runner does not.
		state := filepath.Join(w.base, "walk-state")
		if err := os.Mkdir(state, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := syscall.Mkfifo(filepath.Join(state, "evil.fifo"), 0o600); err != nil {
			t.Fatal(err)
		}
		cfg := *pre.Config
		cfg.StateRoots = []string{state}
		data, err := record.MarshalConfig(&cfg)
		if err != nil {
			t.Fatal(err)
		}
		cfgDir, runDir := filepath.Join(w.base, "walk-cfg"), filepath.Join(w.base, "walk-run")
		for _, d := range []string{cfgDir, runDir} {
			if err := os.Mkdir(d, 0o700); err != nil {
				t.Fatal(err)
			}
		}
		cfgPath := filepath.Join(cfgDir, "sandbox.json")
		if err := os.WriteFile(cfgPath, data, 0o600); err != nil {
			t.Fatal(err)
		}
		sum := sha256.Sum256(data)
		action := pre.Probes.Action
		_, err = record.RunStructureProbes(record.ProbeInput{
			Dir: runDir, Cfg: &cfg, ConfigPath: cfgPath, ConfigSHA256: hex.EncodeToString(sum[:]), Action: action,
			KilnExe: pre.Config.KilnExe, Strace: w.strace,
			Env: []string{"KILN_ACTION=" + action, "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=" + pre.Tools.Path},
		})
		if err == nil || !strings.Contains(err.Error(), "probe walk") || !strings.Contains(err.Error(), "reachable") {
			t.Fatalf("the walk probe did not find the FIFO: %v", err)
		}
		t.Logf("PROBE: walk found a FIFO in a state root: %s", firstLineOf(err.Error()))
		if entries, _ := os.ReadDir(legacy.RecordTmpdir); len(entries) != 0 {
			t.Errorf("tmpdir after the probes: %v", entries)
		}
	})

	t.Run("shadow finds a FIFO a mount hides", func(t *testing.T) {
		w := newProbeWorld(t, kiln, probeOpts{shadow: true})
		stderr, code := w.prepare(t, w.strace)
		if code == 0 {
			t.Fatalf("record prepare accepted a FIFO hidden beneath a mount point\nprobe traces:\n%s", w.probeTails(25))
		}
		t.Logf("PROBE: shadow: exit %d: %s", code, firstLineOf(stderr))
		// Two outcomes are correct, and both fail closed:
		//
		//  (i)  the sandbox starts, and the shadow probe finds the FIFO the host
		//       scan could not see beneath the mount: "probe shadow ... reachable".
		//  (ii) the kernel refuses to start the sandbox at all. In a new user and
		//       mount namespace the inherited mounts are LOCKED, and a
		//       non-recursive bind (MsBind) of a directory that has a locked
		//       child mount fails (EINVAL, has_locked_children in fs/namespace.c).
		//       The helper then exits 125 and prepare fails on the FIRST probe,
		//       with the helper's own message in the quoted step output. In that
		//       case the kernel closes the shadow risk outright, and the probe is
		//       defence in depth.
		//
		// The match for (ii) is specific: the first probe, strace's exit 125, and
		// the sandbox helper's own error prefix. Anything else is a failure.
		switch {
		case strings.Contains(stderr, "probe shadow") && strings.Contains(stderr, "reachable"):
			t.Logf("PROBE: shadow-outcome=probe-found-the-fifo")
		case strings.Contains(stderr, "probe pipes") && strings.Contains(stderr, "strace exited 125") && strings.Contains(stderr, "kiln sandbox-"):
			t.Logf("PROBE: shadow-outcome=kernel-refused-the-bind (locked child mount)")
		default:
			t.Fatalf("it failed, but neither on the shadow probe nor on the sandbox refusing to bind the root\nstderr: %s\nprobe traces:\n%s", stderr, w.probeTails(25))
		}
		if _, err := os.Lstat(filepath.Join(w.recDir, "pre", "SEALED")); err == nil {
			t.Fatal("a failed prepare left a SEALED record")
		}
	})

	t.Run("a strace that hides signals", func(t *testing.T) {
		w := newProbeWorld(t, kiln, probeOpts{})
		// The wrapper inserts `-e signal=none` right before the first `--` (the
		// end of strace's own options: StraceArgs ends with `-o <file> --`), i.e.
		// AFTER the recorder's own `-e signal=all`, where it should win. If strace
		// keeps the first, the test says so: the prepare then succeeds.
		wrapper := filepath.Join(w.bin, "strace-nosignals")
		recStepWrite(t, wrapper, `#!/bin/sh
n=$#; i=0; inserted=""
while [ $i -lt $n ]; do
  a="$1"; shift
  if [ -z "$inserted" ] && [ "$a" = "--" ]; then
    set -- "$@" -e signal=none "$a"; inserted=1
  else
    set -- "$@" "$a"
  fi
  i=$((i+1))
done
exec '`+w.strace+`' "$@"
`, 0o755)
		stderr, code := w.prepare(t, wrapper)
		if code == 0 {
			t.Fatalf("record prepare accepted a strace run with -e signal=none appended: either strace keeps the FIRST -e signal= (then the wrapper proves nothing) or the signals probe is blind\nprobe traces:\n%s", w.probeTails(25))
		}
		t.Logf("PROBE: strace -e signal=none: exit %d: %s", code, firstLineOf(stderr))
		if !strings.Contains(stderr, "probe signals") {
			t.Fatalf("it did not fail on the signals probe\nstderr: %s\nprobe traces:\n%s", stderr, w.probeTails(25))
		}
		if _, err := os.Lstat(filepath.Join(w.recDir, "pre", "SEALED")); err == nil {
			t.Fatal("a failed prepare left a SEALED record")
		}
	})
}

func firstLineOf(s string) string {
	s, _, _ = strings.Cut(strings.TrimSpace(s), "\n")
	if len(s) > 250 {
		s = s[:250] + "..."
	}
	return s
}
