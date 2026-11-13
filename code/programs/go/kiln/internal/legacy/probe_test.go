package legacy

import (
	"errors"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

func TestProbeMatchesWholeToken(t *testing.T) {
	yes := map[string]string{
		"1.83.0": "rustc 1.83.0 (90b35a623 2024-11-26)",
		"1.26.1": "go version go1.26.1 linux/amd64",
		"22.1.0": "v22.1.0\n",
		"3.13.7": "Python 3.13.7",
		"9.4.8":  "The Glorious Glasgow Haskell Compilation System, version 9.4.8",
		"21.0.5": `openjdk version "21.0.5" 2024-10-15`,
		"2.1.20": "kotlinc-jvm 2.1.20 (JRE 21)",
		"1.18.4": "Elixir 1.18.4 (compiled with Erlang/OTP 27)",
		"8.14":   "Gradle 8.14",
		"0.13.7": "uv 0.13.7 (abcdef 2026-11-01)",
	}
	for pin, out := range yes {
		if !probeMatches(out, pin) {
			t.Errorf("%q should carry %s", out, pin)
		}
	}
	no := [][2]string{
		{"1.83.0", "rustc 11.83.01 (x)"}, {"1.2.3", "tool 1.2.30"}, {"1.2", "tool 1.2.3"},
		{"9.0", "tool 19.0"}, {"1.2.3", "tool 1.2.3-rc1"},
	}
	for _, c := range no {
		if probeMatches(c[1], c[0]) {
			t.Errorf("%q must not carry %s", c[1], c[0])
		}
	}
	// A version that appears only inside an absolute path is not a claim about
	// the tool; nor is a timestamp.
	if probeMatches("running /opt/hostedtoolcache/go/1.26.1/x64/bin/go", "1.26.1") {
		t.Error("a path matched")
	}
	if probeMatches("built 2026-01-02T03:04:05Z", "2026-01-02") || probeMatches("at 12:34:56", "12:34:56") {
		t.Error("a timestamp matched")
	}
	if !probeMatches("go1.26.1: C:\\tools\\x", "1.26.1") {
		t.Error("a windows path took the version with it")
	}
}

func TestNormalizeProbe(t *testing.T) {
	got := normalizeProbe([]byte("v1 \x00x linux/amd64 /usr/bin/cc \"/opt/x y\" (C:\\a\\b) 2026-11-12 10:11:12"))
	for _, bad := range []string{"/usr/bin", "/opt/x", "C:\\", "2026", "10:11"} {
		if strings.Contains(got, bad) {
			t.Errorf("%q survives in %q", bad, got)
		}
	}
	if !strings.Contains(got, "linux/amd64") {
		t.Errorf("a slash inside a token was eaten: %q", got)
	}
	if strings.ContainsRune(got, 0) {
		t.Error("a control character survives")
	}
}

func TestInside(t *testing.T) {
	sep := string(filepath.Separator)
	root := sep + "r" + sep + "repo"
	for p, want := range map[string]bool{
		root: true, root + sep + "a": true, root + sep + "a" + sep + "b": true,
		sep + "r" + sep + "repo2": false, sep + "r": false, sep + "x": false,
	} {
		if inside(p, root) != want {
			t.Errorf("inside(%q) = %v", p, !want)
		}
	}
}

// probeFixture is a bin directory of fake tools and a checkout directory.
type probeFixture struct {
	t        *testing.T
	home     string
	bin      string
	checkout string
	tc       *Toolchains
}

func newProbeFixture(t *testing.T) *probeFixture {
	t.Helper()
	if runtime.GOOS == "windows" {
		t.Skip("fake executables need Unix permissions")
	}
	home := t.TempDir()
	f := &probeFixture{t: t, home: home, bin: filepath.Join(home, "bin"), checkout: filepath.Join(home, "checkout")}
	for _, d := range []string{f.bin, f.checkout} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	f.tc = mustToolchains(t, `{
	  "img":  {"always": true, "pins": {"imgtool": "image"}, "probes": [["imgtool", "--version"]], "roots": {"imgtool": ["$HOME/bin"]}},
	  "lang": {"pins": {"gone": "1.0", "ltool": "2.0.1", "nopin": "image", "other": "7.7"},
	           "probes": [["ltool", "--version"], ["ltool", "-V"], ["other", "-v"], ["gone", "-v"], ["nopin", "-x"]],
	           "roots": {"ltool": ["$HOME/bin"]}}
	}`)
	return f
}

func (f *probeFixture) exe(name string) string {
	f.t.Helper()
	p := filepath.Join(f.bin, name)
	if err := os.WriteFile(p, []byte("#!/bin/sh\necho fake\n"), 0o755); err != nil {
		f.t.Fatal(err)
	}
	return p
}

func results(rs []ProbeResult) map[string]string {
	m := map[string]string{}
	for _, r := range rs {
		m[r.Tool] = r.Status
	}
	return m
}

func TestProbeStatuses(t *testing.T) {
	f := newProbeFixture(t)
	f.exe("imgtool")
	f.exe("ltool")
	f.exe("other")
	f.exe("nopin")
	var runs []ProbeRun
	runner := func(r ProbeRun) ([]byte, error) {
		runs = append(runs, r)
		switch filepath.Base(r.Path) {
		case "ltool":
			if r.Args[0] == "-V" {
				return []byte("ltool 2.0.1"), nil
			}
			return []byte("ltool 2.0.1"), nil
		case "other":
			return []byte("other 7.7.1"), nil // not a whole token
		}
		return []byte("anything"), nil
	}
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home, Run: runner})
	if err != nil {
		t.Fatal(err)
	}
	got := results(rs)
	want := map[string]string{"imgtool": ProbeOK, "ltool": ProbeOK, "other": ProbeMismatch, "gone": ProbeAbsent, "nopin": ProbeOK}
	// `nopin` has no pin: it is only run, never compared.
	if len(got) != len(want) {
		t.Fatalf("got %v want %v", got, want)
	}
	for k, v := range want {
		if got[k] != v {
			t.Errorf("%s: got %s want %s", k, got[k], v)
		}
	}
	for i := 1; i < len(rs); i++ {
		if rs[i-1].Tool >= rs[i].Tool {
			t.Fatalf("results not sorted: %v", rs)
		}
	}
	// What every probe ran under: no shell (argv, not a command string), a
	// private working directory, a fixed PATH and nothing else of ours.
	if len(runs) == 0 {
		t.Fatal("nothing ran")
	}
	for _, r := range runs {
		if r.Timeout != ProbeTimeout || r.Limit != ProbeOutputLimit {
			t.Errorf("limits: %v %d", r.Timeout, r.Limit)
		}
		if r.Dir == "" || strings.HasPrefix(r.Dir, f.checkout) {
			t.Errorf("working directory %q", r.Dir)
		}
		var path string
		for _, e := range r.Env {
			if strings.HasPrefix(e, "PATH=") {
				path = strings.TrimPrefix(e, "PATH=")
			}
		}
		if path != f.bin {
			t.Errorf("PATH = %q, want exactly %q", path, f.bin)
		}
		if len(r.Env) != 5 {
			t.Errorf("environment is not minimal: %v", r.Env)
		}
		for _, e := range r.Env {
			if strings.HasPrefix(e, "HOME=") && strings.TrimPrefix(e, "HOME=") == f.home {
				t.Error("the probe saw the real HOME")
			}
		}
	}
	// The private directory is gone afterwards.
	if _, err := os.Stat(runs[0].Dir); err == nil {
		t.Error("probe directory not removed")
	}
}

func TestProbeRefusesBinaryInsideCheckout(t *testing.T) {
	f := newProbeFixture(t)
	real := filepath.Join(f.checkout, "evil")
	if err := os.WriteFile(real, []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(real, filepath.Join(f.bin, "ltool")); err != nil {
		t.Skip("symlinks unavailable")
	}
	ran := false
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home,
		Run: func(ProbeRun) ([]byte, error) { ran = true; return nil, nil }})
	if err != nil {
		t.Fatal(err)
	}
	if ran || results(rs)["ltool"] != ProbeRefused {
		t.Fatalf("ran=%v results=%v", ran, rs)
	}
	// A dangling link is an absent tool, not an error; a root that does not
	// exist is an error.
	if err := os.Remove(real); err != nil {
		t.Fatal(err)
	}
	rs, err = f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home,
		Run: func(ProbeRun) ([]byte, error) { ran = true; return nil, nil }})
	if err != nil || ran || results(rs)["ltool"] != ProbeAbsent {
		t.Fatalf("dangling: %v %v %v", err, ran, rs)
	}
	if _, err := f.tc.Probe(nil, "linux", ProbeOptions{Root: filepath.Join(f.home, "missing")}); err == nil {
		t.Fatal("missing root accepted")
	}
}

func TestProbeRunErrorIsMismatchAndWorstWins(t *testing.T) {
	f := newProbeFixture(t)
	f.exe("ltool")
	calls := 0
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home,
		Run: func(r ProbeRun) ([]byte, error) {
			calls++
			if r.Args[0] == "--version" {
				return []byte("ltool 2.0.1"), nil
			}
			return []byte("ltool 2.0.1"), errors.New("timed out")
		}})
	if err != nil {
		t.Fatal(err)
	}
	// One probe passed and one failed: the tool's worst result is reported.
	if results(rs)["ltool"] != ProbeMismatch || calls != 2 {
		t.Fatalf("%v calls=%d", rs, calls)
	}
}

func TestProbeAbsentToolIsNotAnError(t *testing.T) {
	f := newProbeFixture(t)
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Run: func(ProbeRun) ([]byte, error) {
		t.Fatal("ran a tool that is not there")
		return nil, nil
	}})
	if err != nil {
		t.Fatal(err)
	}
	for _, r := range rs {
		if r.Status != ProbeAbsent {
			t.Fatalf("%v", rs)
		}
	}
	// With no home, $HOME roots are not on the PATH at all.
	f.exe("ltool")
	rs, _ = f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Run: func(ProbeRun) ([]byte, error) { t.Fatal("ran"); return nil, nil }})
	if results(rs)["ltool"] != ProbeAbsent {
		t.Fatalf("%v", rs)
	}
}

func TestExecProbeReal(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("shell scripts")
	}
	dir := t.TempDir()
	script := filepath.Join(dir, "tool")
	body := "#!/bin/sh\necho \"tool 2.0.1\"\necho \"to stderr $HOME $PATH\" >&2\nif [ \"$1\" = \"fail\" ]; then exit 3; fi\nif [ \"$1\" = \"big\" ]; then head -c 200000 /dev/zero | tr '\\0' x; fi\nif [ \"$1\" = \"slow\" ]; then sleep 5; fi\n"
	if err := os.WriteFile(script, []byte(body), 0o755); err != nil {
		t.Fatal(err)
	}
	run := ProbeRun{Path: script, Env: []string{"PATH=/usr/bin:/bin", "HOME=/nonexistent-home"}, Dir: dir, Timeout: 5 * time.Second, Limit: ProbeOutputLimit}
	out, err := execProbe(run)
	if err != nil || !strings.Contains(string(out), "tool 2.0.1") || !strings.Contains(string(out), "/nonexistent-home") {
		t.Fatalf("%q %v", out, err)
	}
	run.Args = []string{"fail"}
	if out, err := execProbe(run); err == nil || !strings.Contains(string(out), "2.0.1") {
		t.Fatalf("non-zero exit must be an error and keep the output: %q %v", out, err)
	}
	run.Args = []string{"big"}
	if out, _ := execProbe(run); len(out) != ProbeOutputLimit {
		t.Fatalf("output not capped: %d", len(out))
	}
	run.Args = []string{"slow"}
	run.Timeout = 100 * time.Millisecond
	start := time.Now()
	if _, err := execProbe(run); err == nil || time.Since(start) > 4*time.Second {
		t.Fatalf("timeout not enforced: %v after %v", err, time.Since(start))
	}
	run.Path = filepath.Join(dir, "missing")
	if _, err := execProbe(run); err == nil {
		t.Fatal("missing binary ran")
	}
	// Through the real runner, end to end: a fake tool whose output carries the pin.
	f := newProbeFixture(t)
	if err := os.WriteFile(filepath.Join(f.bin, "ltool"), []byte("#!/bin/sh\necho ltool 2.0.1\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	rs, err := f.tc.Probe([]string{"lang"}, "linux", ProbeOptions{Root: f.checkout, Home: f.home})
	if err != nil || results(rs)["ltool"] != ProbeOK {
		t.Fatalf("%v %v", rs, err)
	}
}

func TestCapWriterAndLookPath(t *testing.T) {
	w := &capWriter{limit: 4}
	if n, err := w.Write([]byte("abcdefgh")); n != 8 || err != nil {
		t.Fatalf("%d %v", n, err)
	}
	if n, _ := w.Write([]byte("zz")); n != 2 || w.buf.String() != "abcd" {
		t.Fatalf("%q", w.buf.String())
	}
	if runtime.GOOS == "windows" {
		return
	}
	dir := t.TempDir()
	plain := filepath.Join(dir, "plain")
	if err := os.WriteFile(plain, []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Mkdir(filepath.Join(dir, "adir"), 0o755); err != nil {
		t.Fatal(err)
	}
	for _, n := range []string{"plain", "adir", "missing"} {
		if p, ok := lookPath(n, []string{dir}); ok {
			t.Errorf("%s resolved to %s", n, p)
		}
	}
	if err := os.Chmod(plain, 0o755); err != nil {
		t.Fatal(err)
	}
	if p, ok := lookPath("plain", []string{"/nonexistent", dir}); !ok || p != plain {
		t.Fatalf("%s %v", p, ok)
	}
}
