package warrantcorpus

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

type captured struct{ out, err bytes.Buffer }

func (c *captured) io(env ...string) IO {
	return IO{Out: &c.out, Err: &c.err, Env: append([]string{"PATH=" + os.Getenv("PATH")}, env...)}
}

func runMain(t *testing.T, f *fixture, env []string, argv ...string) (int, string, string) {
	t.Helper()
	var c captured
	status := Main(argv, c.io(env...), refRegistry())
	return status, c.out.String(), c.err.String()
}

func TestMainRunWritesManifestAndLabelsAndCheckPasses(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	out := filepath.Join(f.pkgDir, "evidence", "test-manifest.json")
	status, stdout, stderr := runMain(t, f, nil, f.args("run", "--out", out)...)
	if status != 0 || !strings.Contains(stdout, "# wrote "+out) || stderr != "" {
		t.Fatalf("run: %d\n%s\n%s", status, stdout, stderr)
	}
	manifest := mustRead(t, out)
	if !strings.HasSuffix(string(manifest), "}\n") || strings.Contains(string(manifest), "\r") {
		t.Fatal("canonical output: LF and a final newline")
	}
	labels := mustRead(t, filepath.Join(filepath.Dir(out), LabelsFile))
	if !strings.Contains(string(labels), LabelsSchema) {
		t.Fatal("labels not written")
	}
	// check: identical.
	status, stdout, _ = runMain(t, f, nil, f.args("check", out)...)
	if status != 0 || !strings.Contains(stdout, "ok manifest test-manifest.json matches") || !strings.Contains(stdout, "ok "+LabelsFile) {
		t.Fatalf("check: %d\n%s", status, stdout)
	}
	// A different toolchain version is not drift.
	other := strings.Replace(string(manifest), `"version": "`+toolchainVersion()+`"`, `"version": "0.0.1"`, 1)
	mustWrite(t, out, []byte(other))
	if status, stdout, _ = runMain(t, f, nil, f.args("check", out)...); status != 0 {
		t.Fatalf("a toolchain version difference is diagnostic only:\n%s", stdout)
	}
	// Validate accepts it, labels included.
	if status, stdout, _ = runMain(t, f, nil, f.args("validate-manifest", out)...); status != 0 || !strings.Contains(stdout, "structurally valid") {
		t.Fatalf("validate: %d %s", status, stdout)
	}
	// WARRANT_EVIDENCE_OUT is the other way to name the output.
	viaEnv := filepath.Join(f.pkgDir, "other", "m.json")
	if status, _, _ = runMain(t, f, []string{EnvOut + "=" + viaEnv}, f.args("run")...); status != 0 {
		t.Fatal("run via env")
	}
	if string(mustRead(t, viaEnv)) != string(manifest) && !strings.Contains(string(mustRead(t, viaEnv)), `"schema"`) {
		t.Fatal("env output")
	}
}

// toolchain.version is copied from the committed manifest into the fresh one
// before the byte comparison, so a non-text version must be refused, not echoed.
func TestMainCheckRefusesANonTextToolchainVersion(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	out := filepath.Join(f.pkgDir, "evidence", "test-manifest.json")
	runMain(t, f, nil, f.args("run", "--out", out)...)
	good := string(mustRead(t, out))
	was := `"version": "` + toolchainVersion() + `"`
	for _, version := range []string{"7", "null", "{}", "[]", "true"} {
		mustWrite(t, out, []byte(strings.Replace(good, was, `"version": `+version, 1)))
		status, stdout, _ := runMain(t, f, nil, f.args("check", out)...)
		if status != 1 || !strings.Contains(stdout, "not ok manifest: the committed manifest is not a lane manifest") {
			t.Errorf("version %s: %d\n%s", version, status, stdout)
		}
	}
}

func TestMainCheckDetectsDrift(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	out := filepath.Join(f.pkgDir, "evidence", "test-manifest.json")
	runMain(t, f, nil, f.args("run", "--out", out)...)
	good := string(mustRead(t, out))
	for name, edit := range map[string]func(string) string{
		"a digest": func(s string) string {
			at := strings.Index(s, `"source_sha256": "`) + len(`"source_sha256": "`)
			flipped := "0"
			if s[at] == '0' {
				flipped = "1"
			}
			return s[:at] + flipped + s[at+1:]
		},
		"a count":    func(s string) string { return strings.Replace(s, `"cases": 256`, `"cases": 255`, 1) },
		"formatting": func(s string) string { return strings.Replace(s, "\n  ", "\n    ", 1) },
		"final line": func(s string) string { return strings.TrimSuffix(s, "\n") },
		"lane":       func(s string) string { return strings.Replace(s, `"lane": "go"`, `"lane": "python"`, 1) },
	} {
		mustWrite(t, out, []byte(edit(good)))
		status, stdout, _ := runMain(t, f, nil, f.args("check", out)...)
		if status != 1 || !strings.Contains(stdout, "not ok manifest: the committed manifest drifted") {
			t.Errorf("%s: %d\n%s", name, status, stdout)
		}
	}
	// Non-manifest content, missing files and invalid UTF-8.
	for name, content := range map[string][]byte{"not json": []byte("zzz"), "no toolchain": []byte("{}"), "array": []byte("[]"), "bad utf-8": {0xff}} {
		mustWrite(t, out, content)
		status, stdout, _ := runMain(t, f, nil, f.args("check", out)...)
		if status != 1 || !strings.Contains(stdout, "not ok manifest") {
			t.Errorf("%s: %d\n%s", name, status, stdout)
		}
	}
	status, stdout, _ := runMain(t, f, nil, f.args("check", filepath.Join(f.pkgDir, "missing.json"))...)
	if status != 1 || !strings.Contains(stdout, "cannot read the committed manifest missing.json") {
		t.Fatalf("%d %s", status, stdout)
	}
	// Labels: missing, then edited.
	mustWrite(t, out, []byte(good))
	labelsPath := filepath.Join(filepath.Dir(out), LabelsFile)
	if err := os.Remove(labelsPath); err != nil {
		t.Fatal(err)
	}
	if status, stdout, _ = runMain(t, f, nil, f.args("check", out)...); status != 1 || !strings.Contains(stdout, "not ok "+LabelsFile+": missing") {
		t.Fatalf("%d %s", status, stdout)
	}
	mustWrite(t, labelsPath, []byte("{}\n"))
	if status, stdout, _ = runMain(t, f, nil, f.args("check", out)...); status != 1 || !strings.Contains(stdout, "not ok "+LabelsFile+": differs") {
		t.Fatalf("%d %s", status, stdout)
	}
	// check under a fault cannot produce a manifest.
	status, stdout, _ = runMain(t, f, []string{EnvFault + "=invert"}, f.args("check", out)...)
	if status != 1 {
		t.Fatalf("a faulted check must fail: %d %s", status, stdout)
	}
	status, stdout, _ = runMain(t, f, []string{EnvFault + "=invert:eq_fixed", EnvFilter + "=eq_fixed_agrees_w2"}, f.args("check", out)...)
	if status != 1 {
		t.Fatalf("a faulted run fails before it can be compared: %d %s", status, stdout)
	}
	// A filtered run whose fault changes nothing passes, but still has no
	// manifest to compare.
	status, stdout, _ = runMain(t, f, []string{EnvFault + "=swap-choice", EnvFilter + "=/exhaustive/"}, f.args("check", out)...)
	if status != 1 || !strings.Contains(stdout, "cannot produce a manifest") {
		t.Fatalf("%d %s", status, stdout)
	}
}

func TestMainFaultAndFilterRules(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	for name, env := range map[string][]string{
		"filter alone":     {EnvFilter + "=eq_fixed"},
		"filter with none": {EnvFault + "=none", EnvFilter + "=eq_fixed"},
		"unknown fault":    {EnvFault + "=bogus"},
		"filter no match":  {EnvFault + "=invert", EnvFilter + "=/nothing/"},
	} {
		if status, _, stderr := runMain(t, f, env, f.args("run")...); status != 2 || !strings.HasPrefix(stderr, "error: ") {
			t.Errorf("%s: %d %q", name, status, stderr)
		}
	}
	// A faulted run fails loudly and never writes a manifest, even with --out.
	out := filepath.Join(f.pkgDir, "x", "m.json")
	status, stdout, _ := runMain(t, f, []string{EnvFault + "=invert:eq_fixed", EnvFilter + "=/exhaustive/"}, f.args("run", "--out", out)...)
	if status != 1 || !hasLine(strings.Split(stdout, "\n"), "not ok ") {
		t.Fatalf("faulted run: %d\n%s", status, stdout)
	}
	if _, err := os.Stat(out); err == nil {
		t.Fatal("a faulted run wrote a manifest")
	}
	// Every portable fault the corpus lists is caught by its own suites.
	for _, row := range listControlRows(f.corpusDir) {
		status, stdout, _ := runMain(t, f, []string{EnvFault + "=" + row, EnvFilter + "=/exhaustive/"}, f.args("run")...)
		if status != 1 || !strings.Contains(stdout, "not ok ") {
			t.Errorf("fault %s was not caught: %d", row, status)
		}
	}
	// A fault that changes nothing observable yields a clean filtered run: no
	// manifest is written, and the exit status is 0.
	status, stdout, _ = runMain(t, f, []string{EnvFault + "=swap-choice", EnvFilter + "=/exhaustive/"}, f.args("run", "--out", out)...)
	if status != 0 || !strings.Contains(stdout, "faulted or filtered run: no manifest written") {
		t.Fatalf("%d %s", status, stdout)
	}
	if fault, op := parseFaultSpec("invert:eq"); fault != "invert" || op != "eq" {
		t.Fatal("parseFaultSpec")
	}
	if fault, op := parseFaultSpec("none"); fault != "" || op != "" {
		t.Fatal("none")
	}
}

func TestMainUsageErrors(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	cases := map[string][]string{
		"no command":          {},
		"unknown command":     {"frobnicate"},
		"unknown flag":        append(f.args("run"), "--bogus"),
		"flag without value":  {"run", "--repo-root"},
		"duplicate flag":      append(f.args("run"), "--out", "a", "--out", "b"),
		"missing corpus":      {"run", "--repo-root", f.root},
		"missing binding":     {"run", "--repo-root", f.root, "--corpus", f.corpusDir},
		"stray positional":    append(f.args("run"), "extra"),
		"check no positional": f.args("check"),
		"check two":           append(f.args("check"), "a", "b"),
		"sabotage binding":    {"sabotage", "--repo-root", f.root, "--corpus", f.corpusDir},
		"source-digest none":  {"source-digest"},
	}
	for name, argv := range cases {
		if status, _, stderr := runMain(t, f, nil, argv...); status != 2 || !strings.Contains(stderr, "error: ") {
			t.Errorf("%s: %d %q", name, status, stderr)
		}
	}
}

func TestMainRefusesPathsOutsideTheRepository(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	outside := t.TempDir()
	mustWrite(t, filepath.Join(outside, "evidence.bindings"), mustRead(t, f.binding))
	for name, mutate := range map[string]func(*fixture){
		"corpus":                func(g *fixture) { g.corpusDir = realCorpus },
		"binding":               func(g *fixture) { g.binding = filepath.Join(outside, "evidence.bindings") },
		"package":               func(g *fixture) { g.pkgDir = outside },
		"root itself as corpus": func(g *fixture) { g.corpusDir = g.root },
		"missing corpus":        func(g *fixture) { g.corpusDir = filepath.Join(g.root, "nowhere") },
	} {
		g := *f
		mutate(&g)
		status, _, stderr := runMain(t, &g, nil, g.args("run")...)
		if status != 2 || !strings.Contains(stderr, "must lie inside the repository root") {
			t.Errorf("%s: %d %q", name, status, stderr)
		}
	}
	status, _, stderr := runMain(t, f, nil, f.args("validate-manifest", filepath.Join(f.pkgDir, "x.json"), "--registry", f.binding)...)
	if status != 2 {
		t.Fatalf("%d %s", status, stderr)
	}
	g := *f
	g.corpusDir = realCorpus
	if status, _, stderr = runMain(t, &g, nil, g.args("validate-manifest", f.binding)...); status != 2 || !strings.Contains(stderr, "inside the repository root") {
		t.Fatalf("validate must contain the corpus too: %d %s", status, stderr)
	}
}

func TestPrepareRefusals(t *testing.T) {
	replace := func(old, new string) string { return strings.Replace(bindingText(nil), old, new, 1) }
	for name, c := range map[string]struct{ text, code string }{
		"module":            {replace("module ct_compare", "module other"), "module"},
		"lane":              {replace("lane go", "lane python"), "lane"},
		"package prefix":    {replace("package go/ct-compare", "package python/ct-compare"), "lane"},
		"missing op":        {strings.Replace(bindingText(nil), "op eq_u64 export=RefEqU64\n", "", 1), "totality"},
		"extra op":          {bindingText(nil) + "op other export=RefEq\n", "totality"},
		"width on plain op": {replace("op eq export=RefEq", "op eq export=RefEq width=implicit"), "width"},
		"width missing":     {replace(" width=implicit", ""), "width"},
		"error row missing": {replace(" errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength", ""), "totality"},
		"error row extra":   {replace("op eq export=RefEq", "op eq export=RefEq errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"), "totality"},
		"error variant":     {replace("CT_SELECT_LENGTH_MISMATCH:", "OTHER_VARIANT:"), "totality"},
		"error duplicated":  {replace("CT_SELECT_LENGTH_MISMATCH:ErrRefLength", "CT_SELECT_LENGTH_MISMATCH:ErrRefLength,CT_SELECT_LENGTH_MISMATCH:ErrRefLength"), "totality"},
	} {
		f := newFixture(t, fixtureOptions{mini: true, binding: c.text})
		if code := laneCode(t, func() { prepare(f.config(), refRegistry()) }); code != c.code {
			t.Errorf("%s: code %q, want %q", name, code, c.code)
		}
	}
	f := newFixture(t, fixtureOptions{mini: true})
	wantCode(t, "registry", func() { prepare(f.config(), nil) })
}

func TestMainListFaultsAndSourceDigest(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	status, stdout, _ := runMain(t, f, nil, "list-faults", "--repo-root", f.root, "--corpus", f.corpusDir)
	if status != 0 || stdout != "ignore-last-byte:eq_fixed\ninvert:eq_fixed\n" {
		t.Fatalf("%d %q", status, stdout)
	}
	status, stdout, _ = runMain(t, f, nil, "source-digest", f.pkgDir)
	if status != 0 || len(strings.TrimSpace(stdout)) != 64 {
		t.Fatalf("%d %q", status, stdout)
	}
	if status, _, stderr := runMain(t, f, nil, "source-digest", filepath.Join(f.pkgDir, "nowhere")); status != 2 {
		t.Fatalf("%d %s", status, stderr)
	}
}

func TestMainSabotageRunsOneChildPerControl(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	var c captured
	status := Main(f.args("sabotage"), c.io(childPackageEnv+"="+f.pkgDir), refRegistry())
	if status != 0 || strings.Count(c.out.String(), "ok sabotage ") != 2 || strings.Contains(c.out.String(), "not ok") {
		t.Fatalf("sabotage: %d\n%s%s", status, c.out.String(), c.err.String())
	}
	// A library the faults cannot move is "not caught" is impossible to build
	// here, but a child that cannot start is: the control is reported `not ok`.
	var d captured
	io := d.io(childPackageEnv + "=" + f.pkgDir)
	io.Exe = filepath.Join(f.root, "does-not-exist")
	if status := Main(f.args("sabotage"), io, refRegistry()); status != 1 || !strings.Contains(d.out.String(), "not ok sabotage") {
		t.Fatalf("%d\n%s", status, d.out.String())
	}
	// A child that runs but exits 0 (it is not the entry program) is not caught either.
	var e captured
	io = e.io()
	io.Exe = os.Args[0]
	io.ExeArgs = []string{"-test.run=^$"}
	if status := Main(f.args("sabotage"), io, refRegistry()); status != 1 || !strings.Contains(e.out.String(), "not ok sabotage") {
		t.Fatalf("%d\n%s", status, e.out.String())
	}
}

// hangEnv makes TestHangingChild sleep: it is the "library that never returns"
// a sabotage child can turn into.
const hangEnv = "WARRANT_TEST_HANG"

func TestHangingChild(t *testing.T) {
	if os.Getenv(hangEnv) != "" {
		time.Sleep(time.Minute)
	}
}

func TestSabotageKillsAChildThatHangs(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	saved := sabotageTimeout
	sabotageTimeout = 300 * time.Millisecond
	defer func() { sabotageTimeout = saved }()
	var c captured
	io := c.io(hangEnv + "=1")
	io.Exe = os.Args[0]
	io.ExeArgs = []string{"-test.run=^TestHangingChild$"}
	start := time.Now()
	status := Main(f.args("sabotage"), io, refRegistry())
	if status != 1 || !strings.Contains(c.out.String(), "not ok sabotage") || !strings.Contains(c.out.String(), "timed out") {
		t.Fatalf("%d\n%s", status, c.out.String())
	}
	if time.Since(start) > 30*time.Second {
		t.Fatal("the hanging children were not killed at the deadline")
	}
}

func TestChildEnvAndLookup(t *testing.T) {
	env := childEnv([]string{"A=1", EnvFault + "=x", EnvFilter + "=y", EnvOut + "=z", "B=2"}, "invert:eq")
	if strings.Join(env, " ") != "A=1 B=2 "+EnvFault+"=invert:eq "+EnvFilter+"=/exhaustive/" {
		t.Fatalf("%v", env)
	}
	if lookupEnv([]string{"A=1", "A=2", "B"}, "A") != "2" || lookupEnv(nil, "A") != "" {
		t.Fatal("lookupEnv: the last assignment wins")
	}
	if baseName(`C:\x\y.json`) != "y.json" || baseName("a/b/c") != "c" || baseName("plain") != "plain" {
		t.Fatal("baseName")
	}
	l := &limitedBuffer{max: 4}
	l.Write([]byte("abcdefgh"))
	l.Write([]byte("zz"))
	if l.buffer.String() != "abcd" {
		t.Fatalf("limitedBuffer kept %q", l.buffer.String())
	}
}

func TestProcessIO(t *testing.T) {
	io := ProcessIO()
	if io.Out != os.Stdout || io.Err != os.Stderr || len(io.Env) == 0 {
		t.Fatal("ProcessIO")
	}
}
