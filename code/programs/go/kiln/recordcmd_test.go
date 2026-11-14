package main

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/host"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// recordHooks restores the command's hooks after a test.
func recordHooks(t *testing.T) {
	t.Helper()
	p, h, s, r := recordPlatformOK, recordHost, recordSelf, recordPrepare
	t.Cleanup(func() { recordPlatformOK, recordHost, recordSelf, recordPrepare = p, h, s, r })
}

// githubHost is a host.GitHub whose environment is env, with an event file that
// names main as the default branch.
func githubHost(t *testing.T, env map[string]string) host.Host {
	t.Helper()
	ev := filepath.Join(t.TempDir(), "event.json")
	if err := os.WriteFile(ev, []byte(`{"repository":{"default_branch":"main"}}`), 0o600); err != nil {
		t.Fatal(err)
	}
	all := map[string]string{
		"GITHUB_ACTIONS": "true", "GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "schedule",
		"GITHUB_EVENT_PATH": ev, "ImageOS": "ubuntu24", "ImageVersion": "20260101.1",
	}
	for k, v := range env {
		all[k] = v
	}
	return host.NewGitHub(func(k string) string { return all[k] }, &bytes.Buffer{})
}

var goodArgs = []string{"prepare", "--root", "/r", "--dir", "/d", "--build-tool", "/b", "--shard-index", "3", "--shard-count", "12", "--job-start", "1794000000"}

func runRec(args ...string) (int, string, string) {
	var out, errb bytes.Buffer
	code := runRecord(args, &out, &errb)
	return code, out.String(), errb.String()
}

func TestRecordUsage(t *testing.T) {
	if code, _, e := runRec(); code != 2 || !strings.Contains(e, "usage") {
		t.Fatalf("%d %s", code, e)
	}
	if code, _, e := runRec("nope"); code != 2 || !strings.Contains(e, "unknown record subcommand") {
		t.Fatalf("%d %s", code, e)
	}
	// Through the real dispatcher too.
	var out, errb bytes.Buffer
	if code := run([]string{"record"}, &out, &errb); code != 2 {
		t.Fatal(code)
	}
	if code := run([]string{}, &out, &errb); code != 2 || !strings.Contains(errb.String(), "record") {
		t.Fatalf("the top-level usage does not mention record: %s", errb.String())
	}
}

// The post-build pass is a stub until stage 4 PR 2b-2b. It must refuse, so a
// workflow wired to it fails instead of recording nothing.
func TestRecordCollectIsAStub(t *testing.T) {
	code, out, e := runRec("collect", "--root", "/r", "--dir", "/d", "--results", "/x", "--out", "/o")
	if code != 1 || out != "" || !strings.Contains(e, "2b-2b") || !strings.Contains(e, "collect") {
		t.Errorf("%d %q %q", code, out, e)
	}
}

// recordStepHooks also restores the step hook.
func recordStepHooks(t *testing.T) {
	t.Helper()
	recordHooks(t)
	st := recordStep
	t.Cleanup(func() { recordStep = st })
}

func TestRecordStepUsageAndPlatform(t *testing.T) {
	recordStepHooks(t)
	recordStep = func(record.StepOptions) (*record.StepResult, error) {
		t.Fatal("RunStep was reached on a refusal")
		return nil, nil
	}
	recordPlatformOK = func() bool { return true }
	for name, args := range map[string][]string{
		"nothing":         {"step"},
		"no dir":          {"step", "--", "sh", "-c", "true"},
		"a relative dir":  {"step", "--dir", "rec", "--", "sh", "-c", "true"},
		"no command":      {"step", "--dir", "/d"},
		"an unknown flag": {"step", "--nope", "--dir", "/d", "--", "sh", "-c", "true"},
	} {
		if code, out, e := runRec(args...); code != 2 || out != "" || e == "" {
			t.Errorf("%s: %d %q %q", name, code, out, e)
		}
	}
	// Off linux/amd64 the step is refused with 3, before anything is looked at.
	recordPlatformOK = func() bool { return false }
	code, out, e := runRec("step", "--dir", "/d", "--", "sh", "-c", "true")
	if code != 3 || out != "" || !strings.Contains(e, "linux/amd64") || !strings.Contains(e, "not run") {
		t.Fatalf("%d %q %q", code, out, e)
	}
}

func TestRecordStepPassesWhatItWasGivenAndMirrorsTheStatus(t *testing.T) {
	recordStepHooks(t)
	recordPlatformOK = func() bool { return true }
	recordSelf = func() (string, error) { return "/canonical/kiln", nil }
	wd, _ := os.Getwd()
	var got record.StepOptions
	recordStep = func(o record.StepOptions) (*record.StepResult, error) {
		got = o
		if o.Stdout != nil {
			o.Stdout.Write([]byte("relayed\n"))
		}
		return &record.StepResult{ExitCode: 7}, nil
	}
	code, out, e := runRec("step", "--dir", "/d", "--", "sh", "-c", "make -j4")
	if code != 7 || out != "relayed\n" || e != "" {
		t.Fatalf("%d %q %q", code, out, e)
	}
	if got.Dir != "/d" || got.KilnExe != "/canonical/kiln" || got.Cwd != wd || strings.Join(got.Argv, "|") != "sh|-c|make -j4" || len(got.Environ) == 0 {
		t.Fatalf("%+v", got)
	}
}

func TestRecordStepRefusalsExit125WithoutEchoingTheEnvironment(t *testing.T) {
	recordStepHooks(t)
	recordPlatformOK = func() bool { return true }
	recordSelf = func() (string, error) { return "/canonical/kiln", nil }
	recordStep = func(record.StepOptions) (*record.StepResult, error) {
		return nil, errors.New("the environment is wrong")
	}
	code, out, e := runRec("step", "--dir", "/d", "--", "sh", "-c", "true")
	if code != record.ExitRefused || out != "" || !strings.Contains(e, "the environment is wrong") {
		t.Fatalf("%d %q %q", code, out, e)
	}
	recordSelf = func() (string, error) { return "", errors.New("no exe") }
	if code, _, e := runRec("step", "--dir", "/d", "--", "sh", "-c", "true"); code != record.ExitRefused || !strings.Contains(e, "no exe") {
		t.Fatalf("%d %q", code, e)
	}
}

func TestRecordPrepareUsageErrors(t *testing.T) {
	recordHooks(t)
	recordPrepare = func(record.Options) (*record.PrepareResult, error) {
		t.Fatal("Prepare was reached on a usage error")
		return nil, nil
	}
	without := func(flag string) []string {
		var out []string
		for i := 0; i < len(goodArgs); i++ {
			if goodArgs[i] == flag {
				i++
				continue
			}
			out = append(out, goodArgs[i])
		}
		return out
	}
	for name, args := range map[string][]string{
		"no root":            without("--root"),
		"no dir":             without("--dir"),
		"no build tool":      without("--build-tool"),
		"no shard index":     without("--shard-index"),
		"no shard count":     without("--shard-count"),
		"no job start":       without("--job-start"),
		"a stray argument":   append(append([]string{}, goodArgs...), "extra"),
		"an unknown flag":    append(append([]string{}, goodArgs...), "--nope"),
		"a relative root":    replace(goodArgs, "/r", "repo"),
		"a relative dir":     replace(goodArgs, "/d", "rec"),
		"a relative tool":    replace(goodArgs, "/b", "bt"),
		"a relative strace":  append(append([]string{}, goodArgs...), "--strace", "strace"),
		"a relative git":     append(append([]string{}, goodArgs...), "--git", "git"),
		"a negative index":   replace(goodArgs, "3", "-3"),
		"a zero shard count": replace(goodArgs, "12", "0"),
	} {
		if code, _, _ := runRec(args...); code != 2 {
			t.Errorf("%s: exit %d, want 2", name, code)
		}
	}
}

func replace(args []string, old, new string) []string {
	out := append([]string{}, args...)
	for i, a := range out {
		if a == old {
			out[i] = new
		}
	}
	return out
}

// Platform and trust come before any work, and nothing is written.
func TestRecordPrepareRefusesWhereItMustNot(t *testing.T) {
	recordHooks(t)
	reached := false
	recordPrepare = func(record.Options) (*record.PrepareResult, error) {
		reached = true
		return nil, errors.New("unreachable")
	}

	recordPlatformOK = func() bool { return false }
	recordHost = func() host.Host { return githubHost(t, nil) }
	if code, _, e := runRec(goodArgs...); code != 3 || !strings.Contains(e, "linux/amd64") || !strings.Contains(e, "nothing was written") {
		t.Fatalf("platform: %d %s", code, e)
	}

	recordPlatformOK = func() bool { return true }
	for name, h := range map[string]host.Host{
		"not GitHub":     host.NewLocal(&bytes.Buffer{}),
		"untrusted CI":   host.NewUntrusted(&bytes.Buffer{}),
		"a pull request": githubHost(t, map[string]string{"GITHUB_EVENT_NAME": "pull_request"}),
		"another ref":    githubHost(t, map[string]string{"GITHUB_REF": "refs/heads/feature"}),
		"no event file":  githubHost(t, map[string]string{"GITHUB_EVENT_PATH": ""}),
	} {
		recordHost = func() host.Host { return h }
		if code, _, e := runRec(goodArgs...); code != 3 || !strings.Contains(e, "trusted GitHub run") {
			t.Errorf("%s: %d %s", name, code, e)
		}
	}

	// A runner with no image id (self-hosted): trusted, but its keys could not
	// be told apart.
	recordHost = func() host.Host { return githubHost(t, map[string]string{"ImageOS": "", "ImageVersion": ""}) }
	if code, _, e := runRec(goodArgs...); code != 3 || !strings.Contains(e, "image id") {
		t.Fatalf("image: %d %s", code, e)
	}
	if reached {
		t.Fatal("Prepare was reached although the run was refused")
	}
}

func TestRecordPrepareRunsPrepareOnATrustedRun(t *testing.T) {
	recordHooks(t)
	recordPlatformOK = func() bool { return true }
	recordHost = func() host.Host { return githubHost(t, nil) }
	recordSelf = func() (string, error) { return "/opt/kiln", nil }
	var got record.Options
	recordPrepare = func(o record.Options) (*record.PrepareResult, error) {
		got = o
		return &record.PrepareResult{
			Binding: recfmt.Binding{ShardIndex: 3, ShardCount: 12}, BindingID: strings.Repeat("ab", 32),
			PreDir: "/d/pre", Actions: 40, Refused: 2,
		}, nil
	}
	code, out, e := runRec(append(append([]string{}, goodArgs...), "--strace", "/opt/strace")...)
	if code != 0 || e != "" {
		t.Fatalf("%d %s", code, e)
	}
	if got.Root != "/r" || got.Dir != "/d" || got.BuildTool != "/b" || got.KilnExe != "/opt/kiln" || got.Strace != "/opt/strace" ||
		got.ShardIndex != 3 || got.ShardCount != 12 || got.JobStart != 1794000000 ||
		got.Image != "github/"+got.Image[len("github/"):] || !strings.Contains(got.Image, "ubuntu24") || got.Getenv == nil || got.Environ == nil {
		t.Fatalf("%+v", got)
	}
	for _, want := range []string{"ok: shard 3 of 12 prepared", strings.Repeat("ab", 32), "40 actions (2 without a weak key)", "/d/pre/exec-wrapper.json"} {
		if !strings.Contains(out, want) {
			t.Errorf("%q missing from %q", want, out)
		}
	}
	// The default strace.
	runRec(goodArgs...)
	if got.Strace != "/usr/bin/strace" || got.Git != "/usr/bin/git" {
		t.Fatalf("%q %q", got.Strace, got.Git)
	}
	runRec(append(append([]string{}, goodArgs...), "--git", "/opt/git")...)
	if got.Git != "/opt/git" {
		t.Fatalf("%q", got.Git)
	}
}

func TestRecordPrepareFailures(t *testing.T) {
	recordHooks(t)
	recordPlatformOK = func() bool { return true }
	recordHost = func() host.Host { return githubHost(t, nil) }
	recordSelf = func() (string, error) { return "", errors.New("no /proc") }
	if code, _, e := runRec(goodArgs...); code != 1 || !strings.Contains(e, "cannot locate this binary") {
		t.Fatalf("%d %s", code, e)
	}
	recordSelf = func() (string, error) { return "/opt/kiln", nil }
	recordPrepare = func(record.Options) (*record.PrepareResult, error) { return nil, errors.New("the tree is dirty") }
	if code, out, e := runRec(goodArgs...); code != 1 || out != "" || !strings.Contains(e, "record prepare: the tree is dirty") {
		t.Fatalf("%d %q %s", code, out, e)
	}
}

func TestRecordSelfIsCanonical(t *testing.T) {
	p, err := recordSelf()
	if err != nil {
		t.Fatal(err)
	}
	real, err := filepath.EvalSymlinks(p)
	if err != nil || real != p {
		t.Fatalf("%q resolves to %q (%v)", p, real, err)
	}
}

func TestRecordDefaultHooks(t *testing.T) {
	// The defaults are the real ones: the platform test is the GOOS/GOARCH of
	// the build, and the host is detected from the environment.
	if recordPlatformOK() != (osAndArch() == "linux/amd64") {
		t.Fatal(osAndArch())
	}
	if recordHost() == nil {
		t.Fatal("no host")
	}
}

func osAndArch() string { return runtime.GOOS + "/" + runtime.GOARCH }
