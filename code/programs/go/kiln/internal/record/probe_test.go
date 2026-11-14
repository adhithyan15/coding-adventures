package record

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func probeCfg() *sandbox.Config {
	return &sandbox.Config{
		RepoRoot: "/work/repo", Home: "/home/u", Tmpdir: "/tmp/kiln", RoRoots: []string{"/bin", "/usr"},
		StateRoots: []string{}, Masks: []string{"/work/repo/.git"}, EnvByAction: map[string][]string{"p": {"PATH=/usr/bin"}}, KilnExe: "/k/kiln",
	}
}

func specNamed(t *testing.T, specs []ProbeSpec, name string) ProbeSpec {
	t.Helper()
	for _, s := range specs {
		if s.Name == name {
			return s
		}
	}
	t.Fatalf("no probe %s", name)
	return ProbeSpec{}
}

func TestProbeSpecsListsTheRunsAndEscapesPatterns(t *testing.T) {
	cfg := probeCfg()
	specs := ProbeSpecs(cfg, nil)
	var names []string
	for _, s := range specs {
		names = append(names, s.Name)
	}
	if got := strings.Join(names, ","); got != "pipes,walk,signals" {
		t.Fatalf("no shadowed mount: %s", got)
	}
	names = nil
	for _, s := range ProbeSpecs(cfg, []string{"/usr/lib/a"}) {
		names = append(names, s.Name)
	}
	if got := strings.Join(names, ","); got != "pipes,walk,shadow,signals" {
		t.Fatalf("with a shadowed mount: %s", got)
	}
	cfg.RepoRoot = `/work/re*po[1]?\x`
	walk := strings.Join(specNamed(t, ProbeSpecs(cfg, nil), ProbeWalk).Argv, " ")
	if !strings.Contains(walk, `-path /work/re\*po\[1]\?\\x`) {
		t.Fatalf("pattern characters not escaped: %s", walk)
	}
	for _, want := range []string{"-path /proc", "-path /bin", "-path /usr", "-prune", "-type s", "-type p", "-print"} {
		if !strings.Contains(walk, want) {
			t.Errorf("walk lacks %q: %s", want, walk)
		}
	}
	sh := specNamed(t, ProbeSpecs(cfg, []string{"/usr/lib/a", "/usr/lib/b"}), ProbeShadow).Argv
	if sh[0] != "find" || sh[1] != "/usr/lib/a" || sh[2] != "/usr/lib/b" || sh[3] != "-xdev" {
		t.Fatalf("shadow argv %v", sh)
	}
}

// goodTrace is a parsed-trace stand-in a probe run may legitimately have.
func goodTrace(argv []string, survivor bool) *trace.Trace {
	tr := &trace.Trace{
		RootPid: 1, InitPid: 2, InitExitLine: 10,
		Events: []trace.Event{{Pid: 2, Kind: trace.KindProcess, Syscall: "execve", Result: trace.Result{OK: true}, Argv: argv, Line: 5}},
		Procs: map[int]*trace.Process{
			1: {Pid: 1, Children: []int{2}},
			2: {Pid: 2, Parent: 1, Exit: &trace.Exit{Pid: 2}, Children: []int{3}},
		},
		Exits:     []trace.Exit{{Pid: 2}},
		ExitLines: []int{11},
	}
	if survivor {
		tr.Procs[3] = &trace.Process{Pid: 3, Parent: 2, Exit: &trace.Exit{Pid: 3, Killed: true, Signal: "SIGKILL"}}
		tr.Exits = append(tr.Exits, trace.Exit{Pid: 3, Killed: true, Signal: "SIGKILL"})
		tr.ExitLines = append(tr.ExitLines, 12)
	}
	return tr
}

func goodObs(name string) ProbeObservation {
	o := ProbeObservation{Started: true, Elapsed: time.Second}
	switch name {
	case ProbePipes:
		o.StepStdout = []byte("pipe:[12]\npipe:[13]\n")
	case ProbeSignals:
		o.StepStdout = []byte("started\n")
	}
	o.StepStdoutBytes = int64(len(o.StepStdout))
	return o
}

func TestCheckProbeRunAcceptsTheGoodRuns(t *testing.T) {
	for _, sp := range ProbeSpecs(probeCfg(), []string{"/usr/x"}) {
		if err := CheckProbeRun(sp, goodObs(sp.Name), goodTrace(sp.Argv, sp.Name == ProbeSignals)); err != nil {
			t.Errorf("%s: %v", sp.Name, err)
		}
	}
}

func TestCheckProbeRunRefuses(t *testing.T) {
	specs := ProbeSpecs(probeCfg(), []string{"/usr/x"})
	type modFn = func(o *ProbeObservation, tr *trace.Trace)
	obsOnly := func(f func(o *ProbeObservation)) modFn { return func(o *ProbeObservation, _ *trace.Trace) { f(o) } }
	trOnly := func(f func(tr *trace.Trace)) modFn { return func(_ *ProbeObservation, tr *trace.Trace) { f(tr) } }
	stdout := func(s string) modFn {
		return obsOnly(func(o *ProbeObservation) { o.StepStdout, o.StepStdoutBytes = []byte(s), int64(len(s)) })
	}
	cases := []struct {
		name, probe, want string
		mod               modFn
	}{
		{"not started", ProbeWalk, "did not start", obsOnly(func(o *ProbeObservation) { o.Started = false })},
		{"timed out", ProbeWalk, "in time", obsOnly(func(o *ProbeObservation) { o.TimedOut = true })},
		{"signalled", ProbeWalk, "killed by signal", obsOnly(func(o *ProbeObservation) { o.StraceSignal = 9 })},
		{"exit", ProbeWalk, "exited 3", obsOnly(func(o *ProbeObservation) { o.StraceExit = 3 })},
		{"strace stdout", ProbeWalk, "wrote to its standard", obsOnly(func(o *ProbeObservation) { o.StraceStdout = 1 })},
		{"strace stderr", ProbeWalk, "wrote to its standard", obsOnly(func(o *ProbeObservation) { o.StraceStderr = 1 })},
		{"truncated", ProbeWalk, "truncated", obsOnly(func(o *ProbeObservation) { o.Truncated = true })},
		{"pipes held", ProbeWalk, "pipes open", obsOnly(func(o *ProbeObservation) { o.PipesHeld = true })},
		{"step stderr", ProbeWalk, "standard error", obsOnly(func(o *ProbeObservation) { o.StepStderr = 4 })},
		{"no step exec", ProbeWalk, "execve", trOnly(func(tr *trace.Trace) { tr.Events = nil })},
		{"other argv", ProbeWalk, "not the one asked for", trOnly(func(tr *trace.Trace) { tr.Events[0].Argv = []string{"find", "/"} })},
		{"nil argv", ProbeWalk, "not the one asked for", trOnly(func(tr *trace.Trace) { tr.Events[0].Argv = nil })},
		{"step failed", ProbeWalk, "exit 0", trOnly(func(tr *trace.Trace) { tr.Procs[2].Exit.Code = 1 })},
		{"step killed", ProbeWalk, "exit 0", trOnly(func(tr *trace.Trace) { tr.Procs[2].Exit.Killed = true })},
		{"write event", ProbeWalk, "a write event", trOnly(func(tr *trace.Trace) {
			tr.Events = append(tr.Events, trace.Event{Pid: 2, Kind: trace.KindWrite, Syscall: "mkdir", Path: "/x", Line: 6})
		})},
		{"no init exit line", ProbeWalk, "init's exit call", trOnly(func(tr *trace.Trace) { tr.InitExitLine = 0 })},
		{"walk found one", ProbeWalk, "reachable", stdout("/x/sock\n")},
		{"shadow found one", ProbeShadow, "reachable", stdout("/usr/x/s\n")},
		{"pipes: a file", ProbePipes, "not two pipes", stdout("pipe:[1]\n/dev/null\n")},
		{"pipes: one line", ProbePipes, "not two pipes", stdout("pipe:[1]\n")},
		{"pipes: no newline", ProbePipes, "not two pipes", stdout("pipe:[1]\npipe:[2]")},
		{"pipes: long output", ProbePipes, "not two pipes", obsOnly(func(o *ProbeObservation) { o.StepStdoutBytes += 5000 })},
		{"signals: silent", ProbeSignals, "did not report", stdout("")},
		{"signals: no survivor seen", ProbeSignals, "survivor was not seen", trOnly(func(tr *trace.Trace) {
			tr.Exits, tr.ExitLines = tr.Exits[:1], tr.ExitLines[:1]
		})},
		{"signals: slow", ProbeSignals, "took", obsOnly(func(o *ProbeObservation) { o.Elapsed = 31 * time.Second })},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			sp := specNamed(t, specs, c.probe)
			o, tr := goodObs(sp.Name), goodTrace(sp.Argv, sp.Name == ProbeSignals)
			c.mod(&o, tr)
			err := CheckProbeRun(sp, o, tr)
			if err == nil || !strings.Contains(err.Error(), c.want) || !strings.Contains(err.Error(), "probe "+sp.Name) {
				t.Fatalf("got %v, want %q", err, c.want)
			}
		})
	}
	if err := CheckProbeRun(specNamed(t, specs, ProbeWalk), goodObs(ProbeWalk), nil); err == nil || !strings.Contains(err.Error(), "no trace") {
		t.Fatalf("a missing trace: %v", err)
	}
	fail := func(string, ...any) error { return os.ErrInvalid }
	if err := checkProbeOutput("nonesuch", ProbeObservation{}, nil, fail); err == nil {
		t.Fatal("an unknown probe passed")
	}
}

func TestProbeFailuresQuoteTheStepsOutput(t *testing.T) {
	sp := specNamed(t, ProbeSpecs(probeCfg(), nil), ProbeWalk)
	o := goodObs(ProbeWalk)
	o.StepStderr = 40
	o.StepStderrHead = []byte("find: '/x': Permission denied\n\x1b[31m" + strings.Repeat("e", 1000))
	err := CheckProbeRun(sp, o, goodTrace(sp.Argv, false))
	if err == nil || !strings.Contains(err.Error(), "Permission denied") || strings.Contains(err.Error(), "\x1b") || len(err.Error()) > 1000 {
		t.Fatalf("%v", err)
	}
}

func TestSignalsProbeGivesTheSleeperTimeToStart(t *testing.T) {
	// The sandbox's SIGKILL must not land while the sleeper is still in exec:
	// that leaves an unresumed call in the trace.
	argv := specNamed(t, ProbeSpecs(probeCfg(), nil), ProbeSignals).Argv
	if argv[2] != "sleep 300 & sleep 1; echo started" {
		t.Fatalf("%q", argv)
	}
}

func goodReport(cfg *sandbox.Config, shadowed []string) ProbeReport {
	rep := ProbeReport{Schema: ProbesSchema, Action: "p", Shadowed: shadowed}
	for _, sp := range ProbeSpecs(cfg, shadowed) {
		rep.Runs = append(rep.Runs, ProbeRunRecord{Name: sp.Name, Argv: sp.Argv, TraceBytes: 10, TraceSHA256: strings.Repeat("0", 64)})
	}
	return rep
}

func TestProbesRoundTripAndStrictDecode(t *testing.T) {
	cfg := probeCfg()
	rep := goodReport(cfg, []string{"/usr/a"})
	data, err := EncodeProbes(rep)
	if err != nil {
		t.Fatal(err)
	}
	s := string(data)
	if data[len(data)-1] == '\n' || strings.Contains(s, "null") || !strings.HasPrefix(s, `{"schema":"kiln-record-probes/1","action":"p","shadowed":["/usr/a"],"runs":[`) || !strings.HasSuffix(s, `"toolchains":[]}`) {
		t.Fatalf("shape: %s", data)
	}
	back, err := DecodeProbes(data)
	if err != nil {
		t.Fatal(err)
	}
	if err := CheckProbeReport(cfg, back, []string{"p", "q"}); err != nil {
		t.Fatal(err)
	}
	for name, bad := range map[string]string{
		"unknown key": strings.Replace(s, `"action"`, `"extra":1,"action"`, 1),
		"trailing":    s + " {}",
		"newline":     s + "\n",
		"indented":    strings.Replace(s, `{"schema"`, "{ \"schema\"", 1),
		"null list":   strings.Replace(s, `"toolchains":[]`, `"toolchains":null`, 1),
		"not json":    "x",
	} {
		if _, err := DecodeProbes([]byte(bad)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func TestCheckProbeReportRefuses(t *testing.T) {
	cfg := probeCfg()
	acts := []string{"p"}
	sh := []string{"/usr/a"}
	cases := map[string]func(r *ProbeReport){
		"schema":            func(r *ProbeReport) { r.Schema = "x" },
		"toolchains":        func(r *ProbeReport) { r.Toolchains = []json.RawMessage{json.RawMessage(`1`)} },
		"unknown action":    func(r *ProbeReport) { r.Action = "q" },
		"bad action":        func(r *ProbeReport) { r.Action = "/abs" },
		"empty action":      func(r *ProbeReport) { r.Action = "" },
		"dropped probe":     func(r *ProbeReport) { r.Runs = append(r.Runs[:1], r.Runs[2:]...) },
		"extra probe":       func(r *ProbeReport) { r.Runs = append(r.Runs, r.Runs[0]) },
		"renamed":           func(r *ProbeReport) { r.Runs[0].Name = "walk" },
		"other argv":        func(r *ProbeReport) { r.Runs[1].Argv = []string{"true"} },
		"exit":              func(r *ProbeReport) { r.Runs[0].Exit = 1 },
		"empty trace":       func(r *ProbeReport) { r.Runs[0].TraceBytes = 0 },
		"huge trace":        func(r *ProbeReport) { r.Runs[0].TraceBytes = ProbeTraceCap + 1 },
		"digest":            func(r *ProbeReport) { r.Runs[0].TraceSHA256 = "zz" },
		"shadow not sorted": func(r *ProbeReport) { r.Shadowed = []string{"/usr/b", "/usr/a"} },
		"shadow relative":   func(r *ProbeReport) { r.Shadowed = []string{"usr/a"} },
		"shadow dash":       func(r *ProbeReport) { r.Shadowed = []string{"-x"} },
		"shadow root":       func(r *ProbeReport) { r.Shadowed = []string{"/"} },
		"shadow nested":     func(r *ProbeReport) { r.Shadowed = []string{"/usr/a", "/usr/a/b"} },
		"shadow vs runs":    func(r *ProbeReport) { r.Shadowed = []string{"/usr/a", "/usr/b"} },
		"too many": func(r *ProbeReport) {
			r.Shadowed = nil
			for i := 0; i <= MaxShadowed; i++ {
				r.Shadowed = append(r.Shadowed, "/usr/m"+string(rune('a'+i%26))+string(rune('a'+i/26)))
			}
		},
	}
	for name, mod := range cases {
		t.Run(name, func(t *testing.T) {
			r := goodReport(cfg, sh)
			mod(&r)
			if err := CheckProbeReport(cfg, r, acts); err == nil {
				t.Fatal("accepted")
			}
		})
	}
	// A shard with no action: nothing ran, and nothing may be claimed.
	empty := ProbeReport{Schema: ProbesSchema}
	if err := CheckProbeReport(cfg, empty, nil); err != nil {
		t.Fatal(err)
	}
	if err := CheckProbeReport(cfg, empty, acts); err == nil {
		t.Fatal("no probes for a shard that has an action")
	}
	if err := CheckProbeReport(cfg, goodReport(cfg, nil), nil); err == nil {
		t.Fatal("probes for a shard with no action")
	}
}

func TestShadowedMounts(t *testing.T) {
	base := t.TempDir()
	ro, repo, state, link := filepath.Join(base, "ro"), filepath.Join(base, "repo"), filepath.Join(base, "state"), filepath.Join(base, "rolink")
	for _, d := range []string{ro, repo, state} {
		if err := os.Mkdir(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.Symlink(ro, link); err != nil {
		t.Skip("no symlinks")
	}
	cfg := &sandbox.Config{RepoRoot: repo, RoRoots: []string{ro, link, filepath.Join(base, "absent")}, StateRoots: []string{state}}
	defer func(old func() map[string]bool) { mountPoints = old }(mountPoints)

	mountPoints = func() map[string]bool {
		return map[string]bool{
			"/": true, ro: true, repo: true, // a root itself is not beneath it
			ro + "/b": true, ro + "/a": true, repo + "/m": true, state + "/s": true,
			link + "/x": true, base + "/other/x": true, ro + "bogus/x": true,
		}
	}
	got, err := ShadowedMounts(cfg)
	if err != nil {
		t.Fatal(err)
	}
	if want := []string{repo + "/m", ro + "/a", ro + "/b", state + "/s"}; strings.Join(got, "|") != strings.Join(want, "|") {
		t.Fatalf("got %v, want %v", got, want)
	}

	// Nested mount points: only the outermost is visible in the sandbox.
	mountPoints = func() map[string]bool {
		return map[string]bool{ro + "/a": true, ro + "/a/b": true, ro + "/a/b/c": true, ro + "/ab": true, ro + "/z": true}
	}
	got, err = ShadowedMounts(cfg)
	if err != nil {
		t.Fatal(err)
	}
	if want := []string{ro + "/a", ro + "/ab", ro + "/z"}; strings.Join(got, "|") != strings.Join(want, "|") {
		t.Fatalf("nested: got %v, want %v", got, want)
	}

	mountPoints = func() map[string]bool { return nil }
	if _, err := ShadowedMounts(cfg); err == nil {
		t.Fatal("an unreadable mountinfo gave an answer")
	}
	mountPoints = func() map[string]bool {
		m := map[string]bool{}
		for i := 0; i <= MaxShadowed; i++ {
			m[repo+"/m"+string(rune('a'+i%26))+string(rune('a'+i/26))] = true
		}
		return m
	}
	if _, err := ShadowedMounts(cfg); err == nil {
		t.Fatal("more than the maximum")
	}
	mountPoints = func() map[string]bool { return map[string]bool{"/": true} }
	os.WriteFile(filepath.Join(ro, "nope"), nil, 0o644)
	cfg.RoRoots = []string{filepath.Join(ro, "nope", "x")} // lstat of a path beneath a file: ENOTDIR
	if _, err := ShadowedMounts(cfg); err == nil {
		t.Fatal("an unreadable ro root was ignored")
	}
}

func TestProbesLeftNothing(t *testing.T) {
	base := t.TempDir()
	tmp, state := filepath.Join(base, "tmp"), filepath.Join(base, "state")
	os.Mkdir(tmp, 0o755)
	cfg := &sandbox.Config{Tmpdir: tmp, StateRoots: []string{state}}
	if err := probesLeftNothing(cfg); err != nil {
		t.Fatal(err) // an absent state root is fine
	}
	os.Mkdir(state, 0o755)
	if err := probesLeftNothing(cfg); err != nil {
		t.Fatal(err)
	}
	os.WriteFile(filepath.Join(state, "f"), nil, 0o644)
	if err := probesLeftNothing(cfg); err == nil || !strings.Contains(err.Error(), "unscrubbed:state-preloaded") {
		t.Fatalf("%v", err)
	}
	os.Remove(filepath.Join(state, "f"))
	os.WriteFile(filepath.Join(tmp, "f"), nil, 0o644)
	if err := probesLeftNothing(cfg); err == nil || !strings.Contains(err.Error(), "unscrubbed:tmpdir-written") {
		t.Fatalf("%v", err)
	}
	cfg.Tmpdir = filepath.Join(base, "gone")
	if err := probesLeftNothing(cfg); err == nil {
		t.Fatal("a missing tmpdir passed")
	}
}

func TestRunStructureProbesRefusals(t *testing.T) {
	// No action: nothing is started.
	rep, err := RunStructureProbes(ProbeInput{})
	if err != nil || rep.Action != "" || len(rep.Runs) != 0 {
		t.Fatalf("%+v %v", rep, err)
	}
	base := t.TempDir()
	repo := filepath.Join(base, "repo")
	os.MkdirAll(filepath.Join(repo, "p"), 0o755)
	os.Mkdir(filepath.Join(base, "tmp"), 0o755)
	cfg := &sandbox.Config{RepoRoot: repo, Tmpdir: filepath.Join(base, "tmp"), RoRoots: []string{}, StateRoots: []string{}}
	in := ProbeInput{Dir: base, Cfg: cfg, ConfigPath: "/c", ConfigSHA256: strings.Repeat("0", 64), Action: "p", KilnExe: "/k", Strace: filepath.Join(base, "no-strace")}
	defer func(old func() map[string]bool) { mountPoints = old }(mountPoints)

	mountPoints = func() map[string]bool { return nil }
	if _, err := RunStructureProbes(in); err == nil || !strings.Contains(err.Error(), "mount points") {
		t.Fatalf("%v", err)
	}
	mountPoints = func() map[string]bool { return map[string]bool{} }
	// strace that is not there: the first probe does not start.
	if _, err := RunStructureProbes(in); err == nil || !strings.Contains(err.Error(), "did not start") {
		t.Fatalf("%v", err)
	}
	// The probe directory is made once: a second run finds it.
	if _, err := RunStructureProbes(in); err == nil || !strings.Contains(err.Error(), "probe directory") {
		t.Fatalf("%v", err)
	}
}
