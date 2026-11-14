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

// The wrapper and the post-build pass are stubs until stage 4 PR 2b-2. They
// must refuse, so a workflow wired to them fails instead of recording nothing.
func TestRecordStepAndCollectAreStubs(t *testing.T) {
	for _, args := range [][]string{
		{"step", "--dir", "/d", "--", "sh", "-c", "true"}, {"step"}, {"collect", "--root", "/r", "--dir", "/d", "--results", "/x", "--out", "/o"},
	} {
		code, out, e := runRec(args...)
		if code != 1 || out != "" || !strings.Contains(e, "2b-2") || !strings.Contains(e, args[0]) {
			t.Errorf("%v: %d %q %q", args, code, out, e)
		}
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
	if got.Strace != "/usr/bin/strace" {
		t.Fatalf("%q", got.Strace)
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
