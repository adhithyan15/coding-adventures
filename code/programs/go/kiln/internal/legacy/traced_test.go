package legacy

import (
	"crypto/sha256"
	"errors"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

const (
	pkgA = "code/packages/x/a"
	pkgC = "code/packages/x/c"
	pkgZ = "code/packages/x/z"
)

// tracedOpts is the fixture's options with a HOME and a TMPDIR added.
// newTracedFixture is newFixture minus a BUILD file in a directory that the
// base graph does not list (x/q): a traced key refuses a graph that disagrees
// with the tree.
func newTracedFixture(t *testing.T) *fixture {
	f := newFixture(t)
	f.remove("code/packages/x/q/BUILD")
	return f
}

func (f *fixture) tracedOpts() Options {
	o := f.opts("linux")
	o.Environ = append(o.Environ, "HOME=/home/runner", "TMPDIR=/tmp/kiln", "PATH=/evil/bin", "UNLISTED=1")
	return o
}

func (f *fixture) root() string {
	r, err := filepath.Abs(f.dir)
	if err != nil {
		f.t.Fatal(err)
	}
	return r
}

func (f *fixture) traced(g *Graph) *Traced {
	f.t.Helper()
	tr, err := NewTraced(g, f.tracedOpts(), f.root(), []string{"-exec-wrapper=/k", "-emit-results=/r"})
	if err != nil {
		f.t.Fatalf("NewTraced: %v", err)
	}
	return tr
}

func mustWeak(t *testing.T, tr *Traced, rel string) *WeakKey {
	t.Helper()
	w, err := tr.WeakKey(rel)
	if err != nil {
		t.Fatalf("WeakKey(%s): %v", rel, err)
	}
	return w
}

func envOf(vars []key.EnvVar, name string) (string, bool) {
	for _, v := range vars {
		if v.Name == name {
			return v.Value, true
		}
	}
	return "", false
}

func TestWeakKeySections(t *testing.T) {
	f := newTracedFixture(t)
	w := mustWeak(t, f.traced(baseGraph()), pkgA)
	a := w.Action
	if a.SemanticsVersion != 2 {
		t.Errorf("semantics %d", a.SemanticsVersion)
	}
	if a.OS != "linux" || a.Arch != "x64" || a.Target != pkgA || a.Phase != key.PhaseTest {
		t.Errorf("platform/action: %+v", a)
	}
	if a.HostImageID == nil || *a.HostImageID != "test/image" {
		t.Error("host image missing")
	}
	if len(a.Inputs) != 0 || len(a.Reads) != 0 {
		t.Error("a weak key has inputs or reads")
	}
	var targets []string
	for _, c := range a.Consumed {
		targets = append(targets, c.Target)
		if c.Phase != key.PhaseBuild {
			t.Errorf("%s phase %s", c.Target, c.Phase)
		}
	}
	if got := strings.Join(targets, ","); got != "policy:env.json,policy:git,policy:host-roots.json,tool:build-tool" {
		t.Errorf("consumed %s (policy:toolchains.json must stay out, and weak: must not appear)", got)
	}
	wantRecipe, _ := key.LegacyRecipeFiles([]key.NamedFile{{Name: "BUILD", Contents: []byte("echo a\n")}})
	if string(a.Recipe) != string(wantRecipe) {
		t.Error("recipe is not the selected BUILD file")
	}
	// Tools: x's pin plus the always set (cc, sh), sorted by name.
	var tools []string
	for _, tl := range a.Tools {
		tools = append(tools, tl.Name+"@"+tl.Version)
	}
	if got := strings.Join(tools, ","); got != "cc@image,sh@image,xtool@1.2.3" {
		t.Errorf("tools %s", got)
	}
	k, err := key.Key(a)
	if err != nil || k != w.Key {
		t.Errorf("Key field disagrees with the action: %v", err)
	}
}

func TestWeakKeyEnvironment(t *testing.T) {
	f := newTracedFixture(t)
	w := mustWeak(t, f.traced(baseGraph()), pkgA)
	// The scrubbed environment: allowlisted, HOME, TMPDIR, constructed PATH,
	// LANG and TZ.  OTHER, UNLISTED and the inherited PATH are gone.
	want := map[string]string{
		"FIXTURE_A": "1", "HOME": "/home/runner", "TMPDIR": "/tmp/kiln", "LANG": "C.UTF-8", "TZ": "UTC",
		"PATH": "/nonexistent/kiln-test-bin:/opt/x/bin",
	}
	if len(w.Scrubbed) != len(want) {
		t.Errorf("scrubbed env %v", w.Scrubbed)
	}
	for n, v := range want {
		if got, ok := envOf(w.Scrubbed, n); !ok || got != v {
			t.Errorf("%s = %q (set %v), want %q", n, got, ok, v)
		}
	}
	// What the recorder runs with is exactly what is keyed, plus kiln:* names.
	environ := w.Environ()
	if len(environ) != len(w.Scrubbed) {
		t.Fatal("Environ differs in size from Scrubbed")
	}
	for _, e := range environ {
		if strings.HasPrefix(e, "kiln:") {
			t.Errorf("kiln-owned name %q leaks into the process environment", e)
		}
	}
	// The key's env section is the scrubbed environment plus the kiln entries.
	keyed := map[string]string{}
	for _, v := range w.Action.Env {
		keyed[v.Name] = v.Value
	}
	for _, v := range w.Scrubbed {
		if keyed[v.Name] != v.Value {
			t.Errorf("keyed %s = %q, run with %q", v.Name, keyed[v.Name], v.Value)
		}
	}
	if len(keyed) != len(w.Scrubbed)+3 {
		t.Errorf("keyed env has %d names for %d scrubbed", len(keyed), len(w.Scrubbed))
	}
	if keyed[EnvRoot] != f.root() || keyed[EnvRecorder] != "1" {
		t.Errorf("kiln entries: %v", keyed)
	}
	if keyed[EnvExecutorArgs] != `"-emit-results=/r" "-exec-wrapper=/k"` {
		t.Errorf("executor args %q", keyed[EnvExecutorArgs])
	}
}

func TestScrubbedEnvOverrideOrder(t *testing.T) {
	tc, err := ParseToolchains([]byte(baseToolchains))
	if err != nil {
		t.Fatal(err)
	}
	pol, err := ParseEnvPolicy([]byte(`{"names": ["HOME", "LANG", "PATH", "TMPDIR", "TZ"]}`))
	if err != nil {
		t.Fatal(err)
	}
	tools, _ := tc.ToolsFor([]string{"x"}, "linux", "img")
	environ := []string{"LANG=en_US", "TZ=Europe/Paris", "PATH=/evil", "HOME=/h", "TMPDIR=/t", "HOME=/h2"}
	vars, err := ScrubbedEnv(pol, tc, tools, environ)
	if err != nil {
		t.Fatal(err)
	}
	for n, want := range map[string]string{
		"LANG": "C.UTF-8", "TZ": "UTC", "PATH": "/nonexistent/kiln-test-bin:/opt/x/bin", "HOME": "/h2", "TMPDIR": "/t",
	} {
		if got, _ := envOf(vars, n); got != want {
			t.Errorf("%s = %q, want %q (env.json must not win over Kiln's own values)", n, got, want)
		}
	}
	for i := 1; i < len(vars); i++ {
		if vars[i-1].Name >= vars[i].Name {
			t.Error("not sorted")
		}
		if vars[i].Origin != key.OriginEnv {
			t.Error("origin is not env")
		}
	}
	// With no HOME or TMPDIR set, neither appears, even though env.json lists
	// them; PATH, LANG and TZ always do.
	vars, err = ScrubbedEnv(pol, tc, tools, []string{"PATH=/evil"})
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := envOf(vars, "HOME"); ok {
		t.Error("HOME invented")
	}
	if _, ok := envOf(vars, "TMPDIR"); ok {
		t.Error("TMPDIR invented")
	}
	if len(vars) != 3 {
		t.Errorf("vars %v", vars)
	}
	// $HOME roots follow the home that is passed.
	tools, _ = tc.ToolsFor(nil, "linux", "img")
	if p, _ := envOf(mustScrub(t, pol, tc, tools, []string{"HOME=/h"}), "PATH"); p != "/nonexistent/kiln-test-bin" {
		t.Errorf("PATH %q", p)
	}
}

func mustScrub(t *testing.T, pol *EnvPolicy, tc *Toolchains, tools []key.Tool, environ []string) []key.EnvVar {
	t.Helper()
	v, err := ScrubbedEnv(pol, tc, tools, environ)
	if err != nil {
		t.Fatal(err)
	}
	return v
}

func TestScrubbedEnvRefusals(t *testing.T) {
	tc, _ := ParseToolchains([]byte(baseToolchains))
	pol, _ := ParseEnvPolicy([]byte(`{"names": ["FIXTURE_*"]}`))
	if _, err := ScrubbedEnv(pol, tc, nil, []string{"FIXTURE_TOKEN=abc"}); err == nil {
		t.Error("a secret-like name was kept")
	}
	if _, err := ScrubbedEnv(pol, tc, nil, []string{"FIXTURE_A=\xff"}); err == nil {
		t.Error("an invalid UTF-8 value was kept")
	}
	// A kiln-reserved name (':') can only arrive through a prefix pattern.
	colon, _ := ParseEnvPolicy([]byte(`{"names": ["kiln*"]}`))
	if _, err := ScrubbedEnv(colon, tc, nil, []string{"kiln:root=/x"}); err == nil || !strings.Contains(err.Error(), "reserved") {
		t.Errorf("reserved name: %v", err)
	}
	// A HOME that is not clean is dropped from PATH construction but still passed.
	v, err := ScrubbedEnv(pol, tc, nil, []string{"HOME=relative/home"})
	if err != nil {
		t.Fatal(err)
	}
	if h, _ := envOf(v, "HOME"); h != "relative/home" {
		t.Error("HOME not passed through")
	}
}

func TestWeakKeyDeterministic(t *testing.T) {
	f := newTracedFixture(t)
	a := mustWeak(t, f.traced(baseGraph()), pkgA)
	b := mustWeak(t, f.traced(baseGraph()), pkgA)
	if a.Key != b.Key {
		t.Fatal("two runs differ")
	}
	tr := f.traced(baseGraph())
	if mustWeak(t, tr, pkgA).Key != mustWeak(t, tr, pkgA).Key {
		t.Error("WeakKey is not memoised")
	}
	if a.Key == mustWeak(t, f.traced(baseGraph()), "code/packages/x/z").Key {
		t.Error("two packages share a weak key")
	}
}

func TestWeakKeyChanges(t *testing.T) {
	flags := func() []string { return []string{"-exec-wrapper=/k", "-emit-results=/r"} }
	cases := map[string]func(f *fixture, o *Options, root *string, flags *[]string){
		"BUILD": func(f *fixture, _ *Options, _ *string, _ *[]string) { f.write(pkgA+"/BUILD", "echo changed\n") },
		"env.json": func(f *fixture, _ *Options, _ *string, _ *[]string) {
			f.write(EnvPath, `{"names": ["FIXTURE_*", "LANG", "OTHER"]}`)
		},
		"host-roots": func(f *fixture, _ *Options, _ *string, _ *[]string) {
			f.write(HostRootsPath, `{"roots": [{"path": "/lib", "kind": "image"}, {"path": "/usr", "kind": "image"}], "ephemeral": ["/dev/null"], "absent-ok": []}`)
		},
		"build tool": func(f *fixture, _ *Options, _ *string, _ *[]string) {
			f.write(BuildToolPath+"/main.go", "package main // x\n")
		},
		".gitattributes": func(f *fixture, _ *Options, _ *string, _ *[]string) { f.write(".gitattributes", "* text=auto\n") },
		".gitmodules": func(f *fixture, _ *Options, _ *string, _ *[]string) {
			f.write("vendor/sub/.gitmodules", "[submodule]\n")
		},
		"allowed var":   func(_ *fixture, o *Options, _ *string, _ *[]string) { o.Environ = append(o.Environ, "FIXTURE_B=2") },
		"HOME":          func(_ *fixture, o *Options, _ *string, _ *[]string) { o.Environ = append(o.Environ, "HOME=/other") },
		"TMPDIR":        func(_ *fixture, o *Options, _ *string, _ *[]string) { o.Environ = append(o.Environ, "TMPDIR=/other") },
		"image":         func(_ *fixture, o *Options, _ *string, _ *[]string) { o.Host = imager("other/image") },
		"arch":          func(_ *fixture, o *Options, _ *string, _ *[]string) { o.Arch = "arm64" },
		"root":          func(_ *fixture, _ *Options, r *string, _ *[]string) { *r = filepath.Join(*r, "elsewhere") },
		"executor args": func(_ *fixture, _ *Options, _ *string, fl *[]string) { *fl = []string{"-exec-wrapper=/k"} },
	}
	for name, mutate := range cases {
		// Each case gets its own checkout, and its baseline is taken in that
		// checkout before the change, so kiln:root is the same on both sides.
		f := newTracedFixture(t)
		ref, err := NewTraced(baseGraph(), f.tracedOpts(), f.root(), flags())
		if err != nil {
			t.Fatal(err)
		}
		base := mustWeak(t, ref, pkgA).Key
		o, root, fl := f.tracedOpts(), f.root(), flags()
		mutate(f, &o, &root, &fl)
		tr, err := NewTraced(baseGraph(), o, root, fl)
		if err != nil {
			t.Fatalf("%s: %v", name, err)
		}
		if mustWeak(t, tr, pkgA).Key == base {
			t.Errorf("%s did not change the weak key", name)
		}
	}
}

func TestWeakKeyIgnores(t *testing.T) {
	f := newTracedFixture(t)
	base := mustWeak(t, f.traced(baseGraph()), pkgA).Key
	// An unlisted variable, another package's files, and toolchains.json for a
	// language this package does not use all leave the weak key alone.
	o := f.tracedOpts()
	o.Environ = append(o.Environ, "UNLISTED_AGAIN=1")
	f.write(pkgZ+"/src.txt", "different")
	f.write("code/packages/x/q/src.txt", "different")
	f.write(ToolchainsPath, strings.Replace(baseToolchains, `"otool": "image"`, `"otool": "9.9"`, 1))
	tr, err := NewTraced(baseGraph(), o, f.root(), []string{"-exec-wrapper=/k", "-emit-results=/r"})
	if err != nil {
		t.Fatal(err)
	}
	if got := mustWeak(t, tr, pkgA).Key; got != base {
		t.Error("an unrelated change moved the weak key")
	}
	// The same flags in another order key the same.
	tr2, _ := NewTraced(baseGraph(), o, f.root(), []string{"-emit-results=/r", "-exec-wrapper=/k"})
	if got := mustWeak(t, tr2, pkgA).Key; got != base {
		t.Error("flag order moved the weak key")
	}
}

func TestWeakKeyToolsFollowTheStaticClosure(t *testing.T) {
	f := newTracedFixture(t)
	g := baseGraph()
	// x/z uses a second language; x/a's closure does not hold it.
	for i := range g.Packages {
		if g.Packages[i].Name == "x/z" {
			g.Packages[i].Language = "other"
		}
	}
	tr := f.traced(g)
	z := mustWeak(t, tr, pkgZ)
	var names []string
	for _, tl := range z.Action.Tools {
		names = append(names, tl.Name)
	}
	if got := strings.Join(names, ","); got != "cc,otool,sh" {
		t.Errorf("x/z tools %s", got)
	}
	if p, _ := envOf(z.Scrubbed, "PATH"); p != "/nonexistent/kiln-test-bin" {
		t.Errorf("x/z PATH %q: a tool without roots adds nothing, and x's roots must not appear", p)
	}
}

func TestWeakKeyRefusals(t *testing.T) {
	f := newTracedFixture(t)
	refusal := func(tr *Traced, rel, want string) {
		t.Helper()
		_, err := tr.WeakKey(rel)
		r, ok := IsRefusal(err)
		if !ok || r.Reason != want || !strings.Contains(err.Error(), want) {
			t.Errorf("%s: got %v, want refusal %q", rel, err, want)
		}
	}
	// A self-hosted runner's image id.
	o := f.tracedOpts()
	o.Host = imager("self-hosted/unknown-7")
	tr, err := NewTraced(baseGraph(), o, f.root(), nil)
	if err != nil {
		t.Fatal(err)
	}
	refusal(tr, pkgA, "host-unknown")
	// An unknown toolchain.
	g := baseGraph()
	g.Packages[len(g.Packages)-1].Language = "mystery"
	refusal(f.traced(g), pkgZ, "unknown-toolchain:mystery")
	// No BUILD file for this OS.
	g = baseGraph()
	g.Packages[0].BuildFiles["linux"] = nil
	refusal(f.traced(g), pkgA, "no-build-file:linux")
	// A BUILD that is a symlink.
	if runtime.GOOS != "windows" {
		ln := newTracedFixture(t)
		if err := os.Remove(filepath.Join(ln.dir, pkgA, "BUILD")); err != nil {
			t.Fatal(err)
		}
		if err := os.Symlink("../b/BUILD", filepath.Join(ln.dir, pkgA, "BUILD")); err != nil {
			t.Skip("symlinks unavailable")
		}
		refusal(ln.traced(baseGraph()), pkgA, "symlink:BUILD")
	}
	// A missing BUILD file is an error, not a refusal.
	g = baseGraph()
	nb := "BUILD_nope"
	g.Packages[0].BuildFiles["linux"] = &nb
	if _, err := NewTraced(g, f.tracedOpts(), f.root(), nil); err == nil {
		t.Error("a BUILD named by the graph but absent was accepted")
	}
	// A package that is not in the graph.
	if _, err := f.traced(baseGraph()).WeakKey("code/packages/x/nowhere"); err == nil {
		t.Error("unknown package accepted")
	}
	if _, ok := IsRefusal(errors.New("x")); ok {
		t.Error("a plain error is a refusal")
	}
}

func TestNewTracedRefusals(t *testing.T) {
	f := newTracedFixture(t)
	good := f.tracedOpts()
	root := f.root()
	for name, c := range map[string]struct {
		mutate func(o *Options)
		root   string
		flags  []string
	}{
		"no source":     {func(o *Options) { o.Source = nil }, root, nil},
		"no host":       {func(o *Options) { o.Host = nil }, root, nil},
		"bad os":        {func(o *Options) { o.OS = "plan9" }, root, nil},
		"bad arch":      {func(o *Options) { o.Arch = "riscv" }, root, nil},
		"relative root": {func(*Options) {}, "relative/dir", nil},
		"empty root":    {func(*Options) {}, "", nil},
		"flag no dash":  {func(*Options) {}, root, []string{"flag"}},
		"flag twice":    {func(*Options) {}, root, []string{"-a=1", "-a=2"}},
		"flag empty":    {func(*Options) {}, root, []string{"-="}},
		"flag utf8":     {func(*Options) {}, root, []string{"-a=\xff"}},
	} {
		o := good
		c.mutate(&o)
		if _, err := NewTraced(baseGraph(), o, c.root, c.flags); err == nil {
			t.Errorf("%s accepted", name)
		}
	}
	// A missing policy file fails closed.
	f.remove(HostRootsPath)
	if _, err := NewTraced(baseGraph(), f.tracedOpts(), root, nil); err == nil {
		t.Error("a missing host-roots.json was accepted")
	}
	// A Source that cannot list.
	o := good
	o.Source = source.NewFolder(filepath.Join(f.dir, "does-not-exist"))
	if _, err := NewTraced(baseGraph(), o, root, nil); err == nil {
		t.Error("an unreadable tree was accepted")
	}
}

func TestCanonicalExecutorArgs(t *testing.T) {
	a, err := CanonicalExecutorArgs([]string{"-b=2", "--a=1 2", "-c"})
	if err != nil {
		t.Fatal(err)
	}
	if a != `"--a=1 2" "-b=2" "-c"` {
		t.Errorf("got %s", a)
	}
	if e, err := CanonicalExecutorArgs(nil); err != nil || e != "" {
		t.Errorf("no flags: %q %v", e, err)
	}
	// A value with a space or a quote cannot be confused with two flags.
	x, _ := CanonicalExecutorArgs([]string{`-a=b" "-c`})
	y, _ := CanonicalExecutorArgs([]string{"-a=b", "-c"})
	if x == y {
		t.Error("quoting is ambiguous")
	}
}

// ---------------------------------------------------------------------------
// Entry digests
// ---------------------------------------------------------------------------

func entryDigest(t *testing.T, tr *Traced, name string) [32]byte {
	t.Helper()
	d, err := tr.EntryDigest(name)
	if err != nil {
		t.Fatalf("%s: %v", name, err)
	}
	return d
}

func TestPathDigests(t *testing.T) {
	f := newTracedFixture(t)
	f.write("code/scripts/run.sh", "echo hi\n")
	tr := f.traced(baseGraph())
	content := h32sum("echo hi\n")
	if entryDigest(t, tr, "path:code/scripts/run.sh") != key.FileDigest(false, content) {
		t.Error("file state wrong")
	}
	if entryDigest(t, tr, "path:code/scripts") != key.DirectoryDigest() {
		t.Error("directory state wrong")
	}
	if entryDigest(t, tr, "path:code") != key.DirectoryDigest() {
		t.Error("ancestor directory state wrong")
	}
	if entryDigest(t, tr, "path:code/scripts/nope.sh") != key.AbsentDigest() {
		t.Error("absent state wrong")
	}
	// A prefix of a name is not a directory: "code/script" is not "code/scripts".
	if entryDigest(t, tr, "path:code/script") != key.AbsentDigest() {
		t.Error("a name prefix was read as a directory")
	}
	if runtime.GOOS != "windows" {
		if err := os.Chmod(filepath.Join(f.dir, "code/scripts/run.sh"), 0o755); err != nil {
			t.Fatal(err)
		}
		ex := f.traced(baseGraph())
		if entryDigest(t, ex, "path:code/scripts/run.sh") != key.FileDigest(true, content) {
			t.Error("exec bit not in the state digest")
		}
		if err := os.Symlink("../shared/cfg.json", filepath.Join(f.dir, "code/scripts/ln")); err != nil {
			t.Skip("symlinks unavailable")
		}
		want, _ := key.LinkDigest("../shared/cfg.json")
		if entryDigest(t, f.traced(baseGraph()), "path:code/scripts/ln") != want {
			t.Error("link state wrong")
		}
	}
}

func h32sum(s string) [32]byte { return sha256Sum([]byte(s)) }

func TestListDigests(t *testing.T) {
	f := newTracedFixture(t)
	tr := f.traced(baseGraph())
	got := entryDigest(t, tr, "list:code/grammars/g")
	want, _ := key.ListingDigest([]key.ListingEntry{{Name: "one.txt", Kind: key.ListFile}, {Name: "sub", Kind: key.ListDir}})
	if got != want {
		t.Error("listing of code/grammars/g is wrong")
	}
	root := entryDigest(t, tr, "list:.")
	wantRoot, _ := key.ListingDigest([]key.ListingEntry{{Name: "README.md", Kind: key.ListFile}, {Name: "code", Kind: key.ListDir},
		{Name: "go.work", Kind: key.ListFile}, {Name: "mise.toml", Kind: key.ListFile}})
	if root != wantRoot {
		t.Error("root listing is wrong")
	}
	notDir := key.NotDirectoryListingDigest()
	for _, n := range []string{"list:code/grammars/none", "list:README.md", "list:code/gram"} {
		if entryDigest(t, tr, n) != notDir {
			t.Errorf("%s is not the not-a-directory listing", n)
		}
		if notDir == got {
			t.Fatal("a directory listing equals the not-a-directory digest")
		}
	}
	// A new child changes the listing; a change below a child does not; a new
	// directory with a file below it does.
	f.write("code/grammars/g/sub/three.txt", "3")
	if entryDigest(t, f.traced(baseGraph()), "list:code/grammars/g") != got {
		t.Error("a change inside a child directory moved its parent's listing")
	}
	f.write("code/grammars/g/newdir/x", "x")
	if entryDigest(t, f.traced(baseGraph()), "list:code/grammars/g") == got {
		t.Error("a new child directory did not change the listing")
	}
	// A name sorting between a file and a directory of the same stem.
	f.write("code/grammars/g/sub.txt", "x")
	l := entryDigest(t, f.traced(baseGraph()), "list:code/grammars/g")
	want, _ = key.ListingDigest([]key.ListingEntry{{Name: "newdir", Kind: key.ListDir}, {Name: "one.txt", Kind: key.ListFile},
		{Name: "sub", Kind: key.ListDir}, {Name: "sub.txt", Kind: key.ListFile}})
	if l != want {
		t.Error("listing with a stem-sharing file and directory is wrong")
	}
	if runtime.GOOS != "windows" {
		if err := os.Symlink("one.txt", filepath.Join(f.dir, "code/grammars/g/lnk")); err != nil {
			t.Skip("symlinks unavailable")
		}
		want, _ = key.ListingDigest([]key.ListingEntry{{Name: "lnk", Kind: key.ListLink}, {Name: "newdir", Kind: key.ListDir}, {Name: "one.txt", Kind: key.ListFile},
			{Name: "sub", Kind: key.ListDir}, {Name: "sub.txt", Kind: key.ListFile}})
		if entryDigest(t, f.traced(baseGraph()), "list:code/grammars/g") != want {
			t.Error("a link child is not kind 2")
		}
	}
}

func TestPackageDigests(t *testing.T) {
	f := newTracedFixture(t)
	tr := f.traced(baseGraph())
	// x/c has one nested package (x/c-n at c/ext/n); x/outer has x/b nested.
	own, err := tr.ownDigest(tr.pl.byPath[pkgC])
	if err != nil {
		t.Fatal(err)
	}
	want, _ := key.TracedPackageDigest(own, []string{"code/packages/x/c/ext/n"})
	if got := entryDigest(t, tr, "pkg:"+pkgC); got != want {
		t.Error("T(x/c) does not hold its nested package")
	}
	if entryDigest(t, tr, "pkg:code/packages/x/gone") != key.MissingPackageDigest() {
		t.Error("a package that does not exist is not the missing digest")
	}
	// A nested package is not a direct child of an outer ancestor: nothing sits
	// between x/outer and x/b, so the digest of x/outer holds x/b.
	outerOwn, _ := tr.ownDigest(tr.pl.byPath["code/packages/x/outer"])
	wantOuter, _ := key.TracedPackageDigest(outerOwn, []string{"code/packages/x/outer/b"})
	if entryDigest(t, tr, "pkg:code/packages/x/outer") != wantOuter {
		t.Error("T(x/outer) is wrong")
	}
	// D(q) is the package's own files only.
	if entryDigest(t, tr, "pkg:"+pkgZ) == entryDigest(t, tr, "pkg:"+pkgA) {
		t.Error("two packages share T(q)")
	}
}

func TestNestedPackageChangesTq(t *testing.T) {
	f := newTracedFixture(t)
	tr0 := f.traced(baseGraph())
	before := entryDigest(t, tr0, "pkg:"+pkgZ)
	oldOwn, _ := tr0.ownDigest(tr0.pl.byPath[pkgZ])
	// A new package appears inside x/z.  Its only file is its BUILD, which D(x/z)
	// never counted (it was not there), so D(x/z) is unchanged: only the nested
	// list moves T.
	f.write(pkgZ+"/sub/BUILD", "echo sub\n")
	g := baseGraph()
	g.Packages = append(g.Packages, mkpkg("x/z-sub", pkgZ+"/sub", "x"))
	tr := f.traced(g)
	newOwn, _ := tr.ownDigest(tr.pl.byPath[pkgZ])
	if oldOwn != newOwn {
		t.Fatal("test premise: D(q) should not move")
	}
	if entryDigest(t, tr, "pkg:"+pkgZ) == before {
		t.Fatal("a new nested package did not change T(q)")
	}
	// A tracked file appearing where an action found nothing changes D(q), hence T(q).
	h := newTracedFixture(t)
	hb := entryDigest(t, h.traced(baseGraph()), "pkg:"+pkgZ)
	h.write(pkgZ+"/new-file.txt", "x")
	if entryDigest(t, h.traced(baseGraph()), "pkg:"+pkgZ) == hb {
		t.Error("a new tracked file in the package did not change T(q)")
	}
}

func TestEntryDigestRefusesBadNames(t *testing.T) {
	tr := newTracedFixture(t).traced(baseGraph())
	for _, n := range []string{"weak:x", "tool:build-tool", "path:../x", "x", "list:", "pkg:/abs"} {
		if _, err := tr.EntryDigest(n); err == nil {
			t.Errorf("%q accepted", n)
		}
	}
	if _, err := tr.CurrentDigests([]string{"path:a", "weak:b"}); err == nil {
		t.Error("CurrentDigests accepted a weak: entry")
	}
}

func TestTrackedTreeOnly(t *testing.T) {
	hasGit(t)
	f := newTracedFixture(t)
	gitInit(t, f.dir)
	src := source.NewGit(f.dir)
	o := f.tracedOpts()
	o.Source = src
	mk := func() *Traced {
		tr, err := NewTraced(baseGraph(), o, f.root(), nil)
		if err != nil {
			t.Fatal(err)
		}
		return tr
	}
	before := entryDigest(t, mk(), "list:code/scripts")
	// An untracked file on disk (a build product) must not appear anywhere.
	f.write("code/scripts/product.o", "obj")
	f.write(pkgA+"/untracked.txt", "u")
	tr := mk()
	if entryDigest(t, tr, "list:code/scripts") != before {
		t.Error("an untracked file entered a listing")
	}
	if entryDigest(t, tr, "path:code/scripts/product.o") != key.AbsentDigest() {
		t.Error("an untracked file is not absent")
	}
	if entryDigest(t, tr, "path:code/scripts") != key.AbsentDigest() {
		t.Error("an untracked-only directory is not absent")
	}
	if entryDigest(t, tr, "pkg:"+pkgA) != entryDigest(t, mk(), "pkg:"+pkgA) {
		t.Error("unstable")
	}
}

func TestStrongKeyFlow(t *testing.T) {
	f := newTracedFixture(t)
	f.write("code/scripts/run.sh", "echo hi\n")
	tr := f.traced(baseGraph())
	w := mustWeak(t, tr, pkgA)
	names := []string{"pkg:code/packages/x/c", "path:code/scripts/run.sh", "list:code/grammars/g", "pkg:code/packages/x/gone"}
	s, err := tr.StrongKey(w, names)
	if err != nil {
		t.Fatal(err)
	}
	if s.Key == w.Key {
		t.Fatal("strong key equals weak key")
	}
	sa := s.Action
	// Every section but consumed is the weak key's.
	if sa.OS != w.Action.OS || sa.Target != w.Action.Target || string(sa.Recipe) != string(w.Action.Recipe) ||
		len(sa.Tools) != len(w.Action.Tools) || len(sa.Env) != len(w.Action.Env) || sa.SemanticsVersion != 2 ||
		len(sa.Inputs) != 0 || len(sa.Reads) != 0 || *sa.HostImageID != *w.Action.HostImageID {
		t.Error("strong key's other sections differ from the weak key's")
	}
	// consumed: weak: plus the entries only; tool:/policy: are not repeated.
	var got []string
	for _, c := range sa.Consumed {
		got = append(got, c.Target)
	}
	if strings.Join(got, ",") != "list:code/grammars/g,path:code/scripts/run.sh,pkg:code/packages/x/c,pkg:code/packages/x/gone,weak:" {
		t.Errorf("consumed %v", got)
	}
	// Deterministic.
	s2, _ := f.traced(baseGraph()).StrongKey(mustWeak(t, f.traced(baseGraph()), pkgA), names)
	if s2.Key != s.Key {
		t.Error("strong key not deterministic")
	}
	// Each case runs in its own checkout (kiln:root is part of the key) and is
	// compared with that checkout's own baseline.
	strongAfter := func(mutate func(g *fixture)) (before, after [32]byte) {
		g := newTracedFixture(t)
		g.write("code/scripts/run.sh", "echo hi\n")
		run := func() [32]byte {
			tr := g.traced(baseGraph())
			s, err := tr.StrongKey(mustWeak(t, tr, pkgA), names)
			if err != nil {
				t.Fatal(err)
			}
			return s.Key
		}
		before = run()
		mutate(g)
		return before, run()
	}
	// Something no entry covers does not move it.
	if b, a := strongAfter(func(g *fixture) { g.write(pkgZ+"/unrelated.txt", "z"); g.write("code/other/x", "y") }); a != b {
		t.Error("a change nothing in the manifest covers moved the strong key")
	}
	// A tracked file under a path that is not a package does not make it one.
	if b, a := strongAfter(func(g *fixture) { g.write("code/packages/x/gone/src.txt", "echo\n") }); a != b {
		t.Error("a missing package's digest moved although no package appeared")
	}
	// Each entry's digest moves it.
	for name, mutate := range map[string]func(g *fixture){
		"pkg (own file)": func(g *fixture) { g.write("code/packages/x/c/src.txt", "changed") },
		"path":           func(g *fixture) { g.write("code/scripts/run.sh", "echo changed\n") },
		"list":           func(g *fixture) { g.write("code/grammars/g/added.txt", "a") },
	} {
		if b, a := strongAfter(mutate); a == b {
			t.Errorf("%s: strong key did not move", name)
		}
	}
	// The weak key is independent of manifest entry digests.
	f.write("code/scripts/run.sh", "echo changed\n")
	if mustWeak(t, f.traced(baseGraph()), pkgA).Key != w.Key {
		t.Error("the weak key moved when a path entry's content changed")
	}
	if _, err := tr.StrongKey(w, []string{"weak:zz"}); err == nil {
		t.Error("StrongKey accepted a weak: entry")
	}
	if _, err := w.Strong([]key.EntryDigest{{Name: "path:a"}, {Name: "path:a"}}); err == nil {
		t.Error("Strong accepted a duplicate entry")
	}
}

func TestMemoisedDigests(t *testing.T) {
	f := newTracedFixture(t)
	tr := f.traced(baseGraph())
	a, err := tr.CurrentDigests([]string{"pkg:" + pkgC, "list:.", "path:README.md"})
	if err != nil {
		t.Fatal(err)
	}
	// Disk changes after the first computation are not seen again: the
	// provider describes the snapshot it was built over.
	f.write("README.md", "changed")
	b, _ := tr.CurrentDigests([]string{"pkg:" + pkgC, "list:.", "path:README.md"})
	for i := range a {
		if a[i] != b[i] {
			t.Errorf("%s recomputed", a[i].Name)
		}
	}
}

func sha256Sum(b []byte) [32]byte { return sha256.Sum256(b) }

// A package that did not exist when its manifest was recorded may exist now:
// the entry then digests to T(q), not the missing value, so lookup misses.
func TestMissingPackageBecomesReal(t *testing.T) {
	f := newTracedFixture(t)
	missing := entryDigest(t, f.traced(baseGraph()), "pkg:code/packages/x/fresh")
	f.write("code/packages/x/fresh/BUILD", "echo\n")
	g := baseGraph()
	g.Packages = append(g.Packages, mkpkg("x/fresh", "code/packages/x/fresh", "x"))
	if entryDigest(t, f.traced(g), "pkg:code/packages/x/fresh") == missing {
		t.Error("a package that now exists still digests as missing")
	}
}

// The listing digest stands alone: a directory that becomes a file changes it,
// with no paired path: entry to notice.
func TestListingNeedsNoPathEntry(t *testing.T) {
	f := newTracedFixture(t)
	f.write("code/scripts/a/x.txt", "x")
	asDir := entryDigest(t, f.traced(baseGraph()), "list:code/scripts/a")
	if err := os.RemoveAll(filepath.Join(f.dir, "code/scripts/a")); err != nil {
		t.Fatal(err)
	}
	f.write("code/scripts/a", "now a file")
	asFile := entryDigest(t, f.traced(baseGraph()), "list:code/scripts/a")
	if asDir == asFile {
		t.Fatal("a directory that became a tracked file kept its list: digest")
	}
	if asFile != key.NotDirectoryListingDigest() {
		t.Error("a file's listing is not the not-a-directory digest")
	}
}

func TestGraphBuildFilesMustExist(t *testing.T) {
	h := newTracedFixture(t)
	h.remove("code/packages/x/z/BUILD")
	if _, err := NewTraced(baseGraph(), h.tracedOpts(), h.root(), nil); err == nil || !strings.Contains(err.Error(), "not in the tree") {
		t.Errorf("missing BUILD: %v", err)
	}
}

// A BUILD file in a package's subdirectory that the graph does not list (a test
// fixture, say) is an ordinary file of that package: it is in D(q) and the
// package is not in the nested list.
func TestStrayBuildIsAnOrdinaryFile(t *testing.T) {
	f := newTracedFixture(t)
	tr0 := f.traced(baseGraph())
	before := entryDigest(t, tr0, "pkg:"+pkgZ)
	oldOwn, _ := tr0.ownDigest(tr0.pl.byPath[pkgZ])
	f.write(pkgZ+"/tests/fixtures/BUILD", "echo fixture\n")
	f.write(pkgZ+"/tests/fixtures/BUILD_mac", "echo fixture\n")
	tr, err := NewTraced(baseGraph(), f.tracedOpts(), f.root(), nil)
	if err != nil {
		t.Fatalf("a stray BUILD was refused: %v", err)
	}
	z := tr.pl.byPath[pkgZ]
	newOwn, _ := tr.ownDigest(z)
	if newOwn == oldOwn {
		t.Error("a stray BUILD is not in D(q)")
	}
	own := tr.pl.own[z]
	found := false
	for _, p := range own {
		found = found || p == pkgZ+"/tests/fixtures/BUILD"
	}
	if !found {
		t.Error("a stray BUILD is not among the package's own files")
	}
	want, _ := key.TracedPackageDigest(newOwn, nil)
	if got := entryDigest(t, tr, "pkg:"+pkgZ); got != want || got == before {
		t.Error("T(q) should hold the new D(q) and an empty nested list")
	}
}

func TestWeakKeyIsACopy(t *testing.T) {
	f := newTracedFixture(t)
	tr := f.traced(baseGraph())
	a := mustWeak(t, tr, pkgA)
	want := a.Key
	// Scribble over every part of the copy.
	a.Action.Recipe[0] ^= 0xff
	a.Action.Consumed[0].Key[0] ^= 0xff
	a.Action.Tools[0].Identity[0] ^= 0xff
	a.Action.Env[0].Value = "tampered"
	*a.Action.HostImageID = "tampered"
	a.Scrubbed[0].Value = "tampered"
	a.Key = [32]byte{}
	b := mustWeak(t, tr, pkgA)
	if b.Key != want {
		t.Fatal("caller mutation reached the memoised key")
	}
	k, err := key.Key(b.Action)
	if err != nil || k != want {
		t.Fatalf("memoised action corrupted: %v", err)
	}
	// Strong shares nothing with the weak key it came from.
	s, err := b.Strong([]key.EntryDigest{{Name: "path:a", Digest: [32]byte{1}}})
	if err != nil {
		t.Fatal(err)
	}
	s.Action.Recipe[0] ^= 0xff
	s.Action.Tools[0].Identity[0] ^= 0xff
	s.Action.Env[0].Value = "tampered"
	*s.Action.HostImageID = "tampered"
	if k, _ := key.Key(b.Action); k != want {
		t.Fatal("mutating a strong key changed its weak key")
	}
}
