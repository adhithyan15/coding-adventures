package legacy

import (
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"
	"time"
)

// Regression tests for the 7c-3 security review.

func TestInsideFailsClosed(t *testing.T) {
	// A relative path cannot be related to an absolute root: refuse (true).
	root := string(filepath.Separator) + "r" + string(filepath.Separator) + "repo"
	if !inside("bin/tool", root) || !inside("../x", root) {
		t.Fatal("a relative path was judged outside the checkout")
	}
	if runtime.GOOS == "darwin" || runtime.GOOS == "windows" {
		if !inside(strings.ToUpper(root)+string(filepath.Separator)+"a", root) {
			t.Fatal("case-insensitive file systems: a case variant escaped the check")
		}
	}
}

func TestProbeNeedsARoot(t *testing.T) {
	f := newProbeFixture(t)
	if _, err := f.tc.Probe(nil, "linux", ProbeOptions{Run: func(ProbeRun) ([]byte, error) {
		t.Fatal("ran without a root")
		return nil, nil
	}}); err == nil {
		t.Fatal("an empty Root disabled the checkout check")
	}
}

func TestRelativeHomeIsDropped(t *testing.T) {
	tc := mustToolchains(t, sampleToolchains)
	tools, _ := tc.ToolsFor([]string{"rust"}, "linux", "i")
	sep := string(filepath.ListSeparator)
	for _, home := range []string{"home", "../x", ".", "/h/../x", "/h/", "/h" + sep + "/evil", "//h"} {
		if p := tc.ConstructPATH(tools, home); strings.Contains(p, ".cargo") {
			t.Errorf("home %q put %q on the PATH", home, p)
		}
	}
	if p := tc.ConstructPATH(tools, "/home/u"); !strings.Contains(p, "/home/u/.cargo/bin") {
		t.Fatalf("a good home was dropped: %q", p)
	}
}

func TestProbeExecutesTheCheckedPathUnderTheNameGiven(t *testing.T) {
	f := newProbeFixture(t)
	real := filepath.Join(f.home, "real-binary")
	if err := os.WriteFile(real, []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(real, filepath.Join(f.bin, "ltool")); err != nil {
		t.Skip("symlinks unavailable")
	}
	var got ProbeRun
	realResolved, _ := filepath.EvalSymlinks(real)
	if _, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home,
		Run: func(r ProbeRun) ([]byte, error) {
			if filepath.Base(r.Path) == "real-binary" {
				got = r
			}
			return nil, nil
		}}); err != nil {
		t.Fatal(err)
	}
	if got.Path != realResolved || got.Argv0 != "ltool" {
		t.Fatalf("ran %q as %q, want %q as ltool", got.Path, got.Argv0, realResolved)
	}
}

func TestFailedRunIsAMismatchEvenWithThePinAndKeepsOutput(t *testing.T) {
	f := newProbeFixture(t)
	f.exe("ltool")
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home,
		Run: func(ProbeRun) ([]byte, error) { return []byte("ltool 2.0.1 /secret/path"), errors.New("exit status 3") }})
	if err != nil {
		t.Fatal(err)
	}
	var r ProbeResult
	for _, x := range rs {
		if x.Tool == "ltool" {
			r = x
		}
	}
	if r.Status != ProbeMismatch || !strings.Contains(r.Detail, "exit status 3") || !strings.Contains(r.Detail, "2.0.1") || strings.Contains(r.Detail, "/secret") {
		t.Fatalf("%+v", r)
	}
}

func TestOverallProbeDeadline(t *testing.T) {
	f := newProbeFixture(t)
	f.exe("ltool")
	ran := false
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home, Total: time.Nanosecond,
		Run: func(ProbeRun) ([]byte, error) { ran = true; return nil, nil }})
	if err != nil {
		t.Fatal(err)
	}
	if ran || results(rs)["ltool"] != ProbeTimedOut {
		t.Fatalf("ran=%v %v", ran, rs)
	}
}

func TestProbeTimeoutIsASoftReason(t *testing.T) {
	f := newFixture(t)
	p, err := f.planOpts(baseGraph(), func(o *Options) {
		o.Probe = &ProbeOptions{Root: f.dir, Total: time.Nanosecond, Run: func(ProbeRun) ([]byte, error) { return nil, nil }}
	})
	if err != nil {
		t.Fatal(err)
	}
	a := p.rec(t, "x/a")
	if !contains(a.Reasons, "probe-timeout:xtool") || a.Class != f.plan(baseGraph()).rec(t, "x/a").Class {
		t.Fatalf("%v class %s", a.Reasons, a.Class)
	}
	if p.Histogram.Reasons["probe-timeout"] == 0 || reasonGroup("probe-timeout:x") != "probe-timeout" {
		t.Fatalf("%v", p.Histogram.Reasons)
	}
}

func TestProbeCleanupErrorIsReported(t *testing.T) {
	f := newProbeFixture(t)
	old := removeAll
	defer func() { removeAll = old }()
	removeAll = func(string) error { return errors.New("busy") }
	if _, err := f.tc.Probe(nil, "linux", ProbeOptions{Root: f.checkout}); err == nil || !strings.Contains(err.Error(), "busy") {
		t.Fatalf("%v", err)
	}
}

func TestProbeDefinitionsAreConstrained(t *testing.T) {
	wrap := func(probe string) string {
		return `{"a": {"pins": {"cargo": "1", "go": "1", "node": "1", "pkgutil": "image", "python3": "1", "t": "1", "uv": "1", "xcode-select": "image"}, "probes": [` + probe + `]}}`
	}
	bad := map[string]string{
		"argv0 not pinned":     `["other", "--version"]`,
		"cargo install":        `["cargo", "install", "x"]`,
		"cargo install single": `["cargo", "install"]`,
		"pip install":          `["python3", "-m", "pip", "install", "x"]`,
		"python -m":            `["python3", "-m"]`,
		"node -p":              `["node", "-p", "1"]`,
		"node -p alone":        `["node", "-p"]`,
		"uv tool run":          `["uv", "tool", "run", "x"]`,
		"go install":           `["go", "install", "x"]`,
		"go install alone":     `["go", "install"]`,
		"go env":               `["go", "env"]`,
		"version on non-go":    `["t", "version"]`,
		"-p on non-xcode":      `["t", "-p"]`,
		"pkg-info on non-pkg":  `["t", "--pkg-info=com.apple.x"]`,
		"pkg-info bad id":      `["pkgutil", "--pkg-info=Com.apple"]`,
		"pkg-info no dot":      `["pkgutil", "--pkg-info=comapple"]`,
		"pkg-info extra":       `["pkgutil", "--pkg-info=com.apple.x", "y"]`,
		"two args":             `["t", "--version", "--version"]`,
		"path argument":        `["t", "/etc/passwd"]`,
		"space":                `["t", "--version x"]`,
		"eval -c":              `["t", "-c"]`,
		"eval -e":              `["t", "-e"]`,
		"eval --eval":          `["t", "--eval"]`,
		"empty argument":       `["t", ""]`,
		"long flag guess":      `["t", "--versions"]`,
	}
	for name, p := range bad {
		if _, err := ParseToolchains([]byte(wrap(p))); err == nil {
			t.Errorf("%s accepted", name)
		}
	}
	for _, p := range []string{
		`["t"]`, `["t", "--version"]`, `["t", "-version"]`, `["t", "-V"]`, `["t", "-v"]`, `["t", "-vV"]`,
		`["t", "--numeric-version"]`, `["go", "version"]`, `["xcode-select", "-p"]`,
		`["pkgutil", "--pkg-info=com.apple.pkg.CLTools_Executables"]`,
	} {
		if _, err := ParseToolchains([]byte(wrap(p))); err != nil {
			t.Errorf("%s refused: %v", p, err)
		}
	}
}

func TestProbeDetailIsNormalised(t *testing.T) {
	f := newProbeFixture(t)
	f.exe("ltool")
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home,
		Run: func(ProbeRun) ([]byte, error) {
			return []byte("bad"), errors.New("fork/exec /Users/alice/bin/ltool: no such file")
		}})
	if err != nil {
		t.Fatal(err)
	}
	for _, r := range rs {
		if r.Tool == "ltool" && (strings.Contains(r.Detail, "alice") || strings.Contains(r.Detail, "/Users") || !strings.Contains(r.Detail, "bad")) {
			t.Fatalf("detail %q", r.Detail)
		}
	}
}

func TestListSeparatorsRefusedInPolicyPaths(t *testing.T) {
	for _, p := range []string{"/a:/b", "/a;b", "$HOME/a:b"} {
		if checkPolicyPath(p, true, false) == nil {
			t.Errorf("%q accepted", p)
		}
	}
	tc := mustToolchains(t, hrToolchains)
	for _, src := range []string{
		`{"roots": [{"path": "/a:/b", "kind": "image"}], "ephemeral": [], "absent-ok": []}`,
		`{"roots": [], "ephemeral": ["/dev/a:b"], "absent-ok": []}`,
		`{"roots": [], "ephemeral": [], "absent-ok": ["/x;y"]}`,
	} {
		if _, err := ParseHostRoots([]byte(src), tc); err == nil {
			t.Errorf("accepted %s", src)
		}
	}
	if _, err := ParseToolchains([]byte(`{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x:/evil"]}}}`)); err == nil {
		t.Error("a root with a list separator accepted")
	}
}

func TestEphemeralGlobsNeedTwoLiteralSegments(t *testing.T) {
	tc := mustToolchains(t, `{"a": {"pins": {}}}`)
	for _, g := range []string{"/*", "/**", "/proc/*", "$HOME/*", "$TMPDIR/**", "/proc*/x"} {
		src := `{"roots": [], "ephemeral": ["` + g + `"], "absent-ok": []}`
		if _, err := ParseHostRoots([]byte(src), tc); err == nil {
			t.Errorf("%s accepted", g)
		}
	}
	for _, g := range []string{"/dev/fd/*", "/sys/fs/cgroup/**", "/proc/self/exe", "$HOME/.cache/x/*"} {
		src := `{"roots": [], "ephemeral": ["` + g + `"], "absent-ok": []}`
		if _, err := ParseHostRoots([]byte(src), tc); err != nil {
			t.Errorf("%s refused: %v", g, err)
		}
	}
	for p, want := range map[string]int{"/dev/fd/*": 2, "/proc/*": 1, "$HOME": 0, "$HOME/a/*": 1, "/a/b/c": 3} {
		if got := literalSegments(p); got != want {
			t.Errorf("literalSegments(%q) = %d, want %d", p, got, want)
		}
	}
}

func TestPolicyFilesAreCaseExactAndDuplicateFree(t *testing.T) {
	tcBad := map[string]string{
		"case in entry":     `{"a": {"Pins": {}}}`,
		"upper field":       `{"a": {"PINS": {}}}`,
		"duplicate field":   `{"a": {"pins": {}, "pins": {"x": "1"}}}`,
		"duplicate pin key": `{"a": {"pins": {"x": "1", "x": "2"}}}`,
	}
	for name, src := range tcBad {
		if _, err := ParseToolchains([]byte(src)); err == nil {
			t.Errorf("toolchains %s accepted", name)
		}
	}
	for name, src := range map[string]string{
		"env case":      `{"Names": ["A"]}`,
		"env duplicate": `{"names": ["A"], "names": ["B"]}`,
	} {
		if _, err := ParseEnvPolicy([]byte(src)); err == nil {
			t.Errorf("env %s accepted", name)
		}
	}
	tc := mustToolchains(t, hrToolchains)
	for name, src := range map[string]string{
		"roots case":       `{"Roots": [], "ephemeral": [], "absent-ok": []}`,
		"root field case":  `{"roots": [{"Path": "/a", "kind": "image"}], "ephemeral": [], "absent-ok": []}`,
		"root duplicate":   `{"roots": [{"path": "/a", "path": "/b", "kind": "image"}], "ephemeral": [], "absent-ok": []}`,
		"absent-ok casing": `{"roots": [], "ephemeral": [], "Absent-OK": []}`,
	} {
		if _, err := ParseHostRoots([]byte(src), tc); err == nil {
			t.Errorf("host-roots %s accepted", name)
		}
	}
	// pinned.json and never.json use the same decoder.
	f := newFixture(t)
	f.write(PinnedPath, `{"Languages": []}`)
	if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
		t.Error("pinned.json with a case variant accepted")
	}
	f = newFixture(t)
	f.write(NeverPath, `{"packages": [], "packages": ["x/a"]}`)
	if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
		t.Error("never.json with a duplicate key accepted")
	}
	// Nulls and scalars are the type decoder's business, not this walker's.
	if err := checkExactKeys([]byte(`null`), reflect.TypeOf(&envFile{})); err != nil {
		t.Error(err)
	}
	if err := checkExactKeys([]byte(`{"names": null}`), reflect.TypeOf(envFile{})); err != nil {
		t.Error(err)
	}
}
