package legacy

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// cleanFolder is a Folder that vouches for its tree, so lookup accepts it (a
// plain Folder cannot, and is refused).
type cleanFolder struct{ *source.Folder }

func (cleanFolder) Untracked() ([]string, error) { return nil, nil }

// flakyFolder fails to open one tracked file, to reach the digest-error path.
type flakyFolder struct {
	cleanFolder
	bad string
}

func (f flakyFolder) Open(p string) (io.ReadCloser, error) {
	if p == f.bad {
		return nil, fmt.Errorf("injected open failure")
	}
	return f.cleanFolder.Open(p)
}

type localAuth struct{}

func (localAuth) Trusted() bool           { return false }
func (localAuth) WritesLocalLedger() bool { return true }

var lkNow = time.Date(2026, 11, 12, 12, 0, 0, 0, time.UTC)

const (
	relZ   = "code/packages/x/z"
	relU   = "code/packages/x/u"
	entZ   = "pkg:code/packages/x/z"
	entSrc = "path:code/packages/x/z/src.txt"
)

// lkEnv is a fixture, its graph and a ledger directory.
type lkEnv struct {
	t      *testing.T
	f      *fixture
	g      *Graph
	dir    string
	runs   int
	mod    func(*Options)
	flags  []string
	probes bool
}

func newLk(t *testing.T) *lkEnv {
	return &lkEnv{t: t, f: newTracedFixture(t), g: baseGraph(), dir: t.TempDir(), flags: []string{"-exec-wrapper=/k"}}
}

func (e *lkEnv) opts(osName string) Options {
	o := e.f.tracedOpts()
	o.OS = osName
	o.Source = cleanFolder{source.NewFolder(e.f.dir)}
	if e.mod != nil {
		e.mod(&o)
	}
	return o
}

func (e *lkEnv) traced() *Traced {
	e.t.Helper()
	tr, err := NewTraced(e.g, e.opts("linux"), e.f.root(), e.flags)
	if err != nil {
		e.t.Fatalf("NewTraced: %v", err)
	}
	return tr
}

func (e *lkEnv) weak(rel string) *WeakKey { return mustWeak(e.t, e.traced(), rel) }

// manifest records a manifest for rel's weak key; later calls are newer.
func (e *lkEnv) manifest(rel string, entries ...string) string {
	e.t.Helper()
	e.runs++
	entries = sortedCopy(entries)
	m := ledger.Manifest{Weak: e.weak(rel).Key, Target: rel, OS: "linux", Arch: "x64", Entries: entries,
		Revision: "rev", Run: fmt.Sprint(100 + e.runs), At: lkNow.Add(-time.Hour)}
	if err := ledger.OpenManifests(e.dir, ledger.Local).Record([]ledger.Manifest{m}, localAuth{}, lkNow); err != nil {
		e.t.Fatalf("record manifest: %v", err)
	}
	return key.Hex(m.ID())
}

// pass records a pass for rel and entries, keyed by the REAL encoders from the
// tree as it is now.
func (e *lkEnv) pass(rel string, entries ...string) [32]byte {
	e.t.Helper()
	tr := e.traced()
	sk, err := tr.StrongKey(mustWeak(e.t, tr, rel), sortedCopy(entries))
	if err != nil {
		e.t.Fatalf("StrongKey: %v", err)
	}
	r := ledger.Record{Key: sk.Key, Target: rel, OS: "linux", Arch: "x64", Kiln: "kiln-test",
		Revision: "rev", Run: "900", At: lkNow.Add(-time.Minute), Kind: ledger.KindPass}
	if err := ledger.Open(e.dir, ledger.Local).Record([]ledger.Record{r}, localAuth{}, lkNow); err != nil {
		e.t.Fatalf("record pass: %v", err)
	}
	return sk.Key
}

func sortedCopy(in []string) []string {
	out := append([]string(nil), in...)
	sort.Strings(out)
	return out
}

func (e *lkEnv) lookupOpts() *LookupOptions {
	return &LookupOptions{
		Manifests: ledger.OpenManifests(e.dir, ledger.Local), Passes: ledger.Open(e.dir, ledger.Local),
		Now: lkNow, Root: e.f.root(), ExecutorFlags: e.flags,
	}
}

func (e *lkEnv) planOS(osName string, mod func(*LookupOptions)) (*Plan, error) {
	o := e.opts(osName)
	o.Lookup = e.lookupOpts()
	if mod != nil {
		mod(o.Lookup)
	}
	return BuildPlan(e.g, o)
}

func (e *lkEnv) plan() *Plan {
	e.t.Helper()
	p, err := e.planOS("linux", nil)
	if err != nil {
		e.t.Fatalf("BuildPlan: %v", err)
	}
	return p
}

func lk(t *testing.T, p *Plan, name string) *Lookup {
	t.Helper()
	l := p.rec(t, name).Lookup
	if l == nil {
		t.Fatalf("%s has no lookup result", name)
	}
	return l
}

func TestLookupHit(t *testing.T) {
	e := newLk(t)
	id := e.manifest(relZ, "list:"+relZ, entZ, entSrc)
	k := e.pass(relZ, "list:"+relZ, entZ, entSrc)
	p := e.plan()
	l := lk(t, p, "x/z")
	if l.Result != ResultHit || l.Hit == nil || l.Hit.Manifest != id || l.Hit.Pass.Key != key.Hex(k) ||
		l.Hit.Pass.Target != relZ || l.Hit.Pass.Run != "900" || l.Hit.Pass.Kiln != "kiln-test" ||
		l.Hit.Pass.At != lkNow.Add(-time.Minute).Format(time.RFC3339) {
		t.Fatalf("lookup %+v hit %+v", l, l.Hit)
	}
	if l.Weak != key.Hex(e.weak(relZ).Key) || len(l.Manifests) != 1 || l.Manifests[0].ID != id || l.Manifests[0].Strong != key.Hex(k) {
		t.Errorf("weak or tried manifests: %+v", l)
	}
	if got := p.Histogram.Lookup; got[ResultHit] != 1 || got[ResultNoManifest] != len(p.Packages)-1 {
		t.Errorf("histogram %v", got)
	}
	if p.Lookup == nil || p.Lookup.Scope != "local" || p.Lookup.ProbesRan || p.Lookup.Manifests.Retained != 1 || p.Lookup.Manifests.Ignored != 0 || p.Lookup.Note == "" {
		t.Errorf("plan lookup info %+v", p.Lookup)
	}
	// Lookup skips nothing and changes nothing else: the classes still say
	// that no package is cacheable.
	for _, r := range p.Packages {
		if r.Class == ClassCacheable {
			t.Errorf("%s became cacheable", r.Name)
		}
	}
}

func TestLookupMissWhenAManifestEntryChanges(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ, "path:README.md")
	e.pass(relZ, entZ, "path:README.md")
	if got := lk(t, e.plan(), "x/z").Result; got != ResultHit {
		t.Fatalf("baseline %s", got)
	}
	e.f.write("README.md", "changed\n")
	l := lk(t, e.plan(), "x/z")
	if l.Result != ResultMiss || l.Hit != nil || len(l.Manifests) != 1 || l.Manifests[0].Strong == "" {
		t.Fatalf("%+v", l)
	}
	// The miss lists the entries and where each stands now.  It does not
	// pretend to know which one changed.
	want := []EntryState{{Name: "path:README.md", Now: "present"}, {Name: entZ, Now: "present"}}
	if !reflect.DeepEqual(l.Manifests[0].Entries, want) {
		t.Errorf("entries %+v", l.Manifests[0].Entries)
	}
}

func TestLookupHitWhenAFileOutsideEveryEntryChanges(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ, "path:README.md")
	e.pass(relZ, entZ, "path:README.md")
	before := lk(t, e.plan(), "x/z")
	// A file in no entry (another package's), and an untracked-looking new file.
	e.f.write(relU+"/src.txt", "u changed")
	e.f.write("code/shared/unrelated.txt", "new")
	after := lk(t, e.plan(), "x/z")
	if after.Result != ResultHit || after.Weak != before.Weak {
		t.Fatalf("%+v", after)
	}
}

func TestLookupMissWhenANestedPackageAppearsUnderAPkgEntry(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ)
	e.pass(relZ, entZ)
	before := lk(t, e.plan(), "x/z")
	if before.Result != ResultHit {
		t.Fatalf("baseline %+v", before)
	}
	// T(q): a package newly nested under a pkg: entry changes its digest even
	// though no listed file changed (KILN04 5.2).
	e.f.write(relZ+"/n/BUILD", "echo n\n")
	e.f.write(relZ+"/n/src.txt", "n")
	e.g.Packages = append(e.g.Packages, mkpkg("x/zn", relZ+"/n", "x"))
	sortPackages(e.g)
	after := lk(t, e.plan(), "x/z")
	if after.Result != ResultMiss || after.Weak != before.Weak {
		t.Fatalf("a new nested package must miss on the same weak key: %+v", after)
	}
}

func sortPackages(g *Graph) {
	for i := 1; i < len(g.Packages); i++ {
		for j := i; j > 0 && g.Packages[j].Name < g.Packages[j-1].Name; j-- {
			g.Packages[j], g.Packages[j-1] = g.Packages[j-1], g.Packages[j]
		}
	}
}

func TestLookupNoManifest(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ)
	p := e.plan()
	l := lk(t, p, "x/a")
	if l.Result != ResultNoManifest || l.Weak == "" || len(l.Manifests) != 0 || l.Hit != nil {
		t.Fatalf("%+v", l)
	}
	if p.Lookup.Manifests.Ignored != 0 || p.Lookup.Manifests.Retained != 1 {
		t.Errorf("%+v", p.Lookup.Manifests)
	}
}

func TestLookupManifestsOfOtherKeysAreIgnored(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ)
	// A manifest for a weak key no package has (a recorder from another world).
	stray := ledger.Manifest{Weak: [32]byte{9}, Target: relZ, OS: "linux", Arch: "x64", Entries: []string{entZ},
		Revision: "r", Run: "1", At: lkNow.Add(-time.Hour)}
	if err := ledger.OpenManifests(e.dir, ledger.Local).Record([]ledger.Manifest{stray}, localAuth{}, lkNow); err != nil {
		t.Fatal(err)
	}
	p := e.plan()
	if m := p.Lookup.Manifests; m.Read != 2 || m.Ignored != 1 || m.Retained != 1 {
		t.Fatalf("%+v", m)
	}
}

func TestLookupKNewestFirst(t *testing.T) {
	e := newLk(t)
	var ids []string // oldest first
	for i := 0; i < 5; i++ {
		ids = append(ids, e.manifest(relZ, entZ, fmt.Sprintf("path:%s/v%d", relZ, i)))
	}
	// Only the oldest has a pass; it was the fifth-newest, so it is no longer
	// retained (K = 4) and cannot hit.
	e.pass(relZ, entZ, "path:"+relZ+"/v0")
	l := lk(t, e.plan(), "x/z")
	if l.Result != ResultMiss || len(l.Manifests) != MaxLookupManifests {
		t.Fatalf("%+v", l)
	}
	for i, m := range l.Manifests { // newest first: v4, v3, v2, v1
		if m.ID != ids[4-i] {
			t.Errorf("manifest %d is %s, want %s", i, m.ID, ids[4-i])
		}
	}
	// Passes for v1 and v3: the NEWEST matching manifest (v3) hits and the
	// older v1 is never reached, so the tried list ends at the hit.
	e.pass(relZ, entZ, "path:"+relZ+"/v1")
	e.pass(relZ, entZ, "path:"+relZ+"/v3")
	l = lk(t, e.plan(), "x/z")
	if l.Result != ResultHit || l.Hit.Manifest != ids[3] || len(l.Manifests) != 2 ||
		l.Manifests[0].ID != ids[4] || l.Manifests[1].ID != ids[3] {
		t.Fatalf("%+v", l)
	}
}

func TestLookupOutsideStaticClosure(t *testing.T) {
	e := newLk(t)
	// z has no dependency on u and nothing named ghost exists, yet its trace
	// read both: the old graph is wrong about z.  x/outer/b IS in a's closure.
	e.manifest(relZ, entZ, "pkg:"+relU, "pkg:code/packages/x/ghost", "path:README.md")
	e.manifest("code/packages/x/a", "pkg:code/packages/x/a", "pkg:code/packages/x/outer/b")
	p := e.plan()
	l := lk(t, p, "x/z")
	want := []string{"pkg:code/packages/x/ghost", "pkg:" + relU}
	if !reflect.DeepEqual(l.OutsideStaticClosure, want) {
		t.Fatalf("outside %v, want %v", l.OutsideStaticClosure, want)
	}
	if a := lk(t, p, "x/a"); len(a.OutsideStaticClosure) != 0 {
		t.Errorf("x/a: %v", a.OutsideStaticClosure)
	}
	if p.Histogram.OutsideStaticClosure != 1 {
		t.Errorf("histogram count %d", p.Histogram.OutsideStaticClosure)
	}
	states := map[string]string{}
	for _, en := range l.Manifests[0].Entries {
		states[en.Name] = en.Now
	}
	if states["pkg:code/packages/x/ghost"] != "no-package" || states["pkg:"+relU] != "present" {
		t.Errorf("states %v", states)
	}
}

func TestLookupEntryStates(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, "list:"+relZ, "list:README.md", "path:"+relZ, "path:nothing/here")
	l := lk(t, e.plan(), "x/z")
	got := map[string]string{}
	for _, en := range l.Manifests[0].Entries {
		got[en.Name] = en.Now
	}
	want := map[string]string{
		"list:" + relZ: "present", "list:README.md": "not-directory", "path:" + relZ: "directory", "path:nothing/here": "absent",
	}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("states %v", got)
	}
}

func TestLookupIneligible(t *testing.T) {
	// Each gate wins over a hit that is otherwise waiting in the ledger.
	arm := func(e *lkEnv) { e.manifest(relZ, entZ); e.pass(relZ, entZ) }

	t.Run("never list", func(t *testing.T) {
		e := newLk(t)
		arm(e)
		e.f.write(NeverPath, `{"packages": ["x/z"]}`)
		p := e.plan()
		if got := lk(t, p, "x/z"); got.Result != "ineligible:never:x/z" || got.Hit != nil || got.Weak != "" {
			t.Fatalf("%+v", got)
		}
		if p.Histogram.Lookup["ineligible:never"] != 1 {
			t.Errorf("histogram %v", p.Histogram.Lookup)
		}
	})
	t.Run("hard reason", func(t *testing.T) {
		e := newLk(t)
		arm(e)
		for i := range e.g.Packages {
			if e.g.Packages[i].Name == "x/z" {
				e.g.Packages[i].BuildFiles["linux"] = nil
			}
		}
		if got := lk(t, e.plan(), "x/z").Result; got != "ineligible:no-build-file:linux" {
			t.Fatalf("%s", got)
		}
	})
	t.Run("unknown toolchain", func(t *testing.T) {
		e := newLk(t)
		arm(e)
		for i := range e.g.Packages {
			if e.g.Packages[i].Name == "x/z" {
				e.g.Packages[i].Language = "nolang"
			}
		}
		if got := lk(t, e.plan(), "x/z").Result; got != "ineligible:unknown-toolchain:nolang" {
			t.Fatalf("%s", got)
		}
	})
	t.Run("never beats other hard reasons", func(t *testing.T) {
		e := newLk(t)
		e.f.write(NeverPath, `{"packages": ["x/z"]}`)
		for i := range e.g.Packages {
			if e.g.Packages[i].Name == "x/z" {
				e.g.Packages[i].BuildFiles["linux"] = nil
			}
		}
		if got := lk(t, e.plan(), "x/z").Result; got != "ineligible:never:x/z" {
			t.Fatalf("%s", got)
		}
	})
	t.Run("unknown image", func(t *testing.T) {
		e := newLk(t)
		arm(e)
		e.mod = func(o *Options) { o.Host = imager("unknown/self-hosted") }
		// The recorder cannot key an unknown image, so there is nothing to
		// arm; every package is refused.
		p, err := e.planOS("linux", nil)
		if err != nil {
			t.Fatal(err)
		}
		for _, r := range p.Packages {
			if r.Lookup.Result != "ineligible:host-unknown" {
				t.Errorf("%s: %s", r.Name, r.Lookup.Result)
			}
		}
		if p.Histogram.Lookup["ineligible:host-unknown"] != len(p.Packages) {
			t.Errorf("histogram %v", p.Histogram.Lookup)
		}
	})
}

// probeEnv makes xtool resolvable so a probe has a verdict.
func (e *lkEnv) probeEnv(inside bool, output string) {
	e.t.Helper()
	if runtime.GOOS == "windows" {
		e.t.Skip("fake executables")
	}
	bindir := filepath.Join(e.t.TempDir(), "bin")
	root := e.t.TempDir()
	if inside {
		bindir, root = filepath.Join(e.f.dir, "fakebin"), e.f.dir
	}
	if err := os.MkdirAll(bindir, 0o755); err != nil {
		e.t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(bindir, "xtool"), []byte("#!/bin/sh\n"), 0o755); err != nil {
		e.t.Fatal(err)
	}
	e.f.write(ToolchainsPath, strings.Replace(baseToolchains, `"/opt/x/bin"`, `"`+filepath.ToSlash(bindir)+`"`, 1))
	e.mod = func(o *Options) {
		o.Probe = &ProbeOptions{Root: root, Run: func(ProbeRun) ([]byte, error) { return []byte(output), nil }}
	}
}

func TestLookupIneligibleOnProbeDisagreement(t *testing.T) {
	t.Run("mismatch", func(t *testing.T) {
		e := newLk(t)
		e.probeEnv(false, "xtool 9.9.9")
		e.manifest(relZ, entZ)
		e.pass(relZ, entZ)
		p := e.plan()
		if got := lk(t, p, "x/z").Result; got != "ineligible:probe-mismatch:xtool" {
			t.Fatalf("%s", got)
		}
		if p.Histogram.Lookup["ineligible:probe-mismatch"] != len(p.Packages) { // every package is of language x
			t.Errorf("histogram %v", p.Histogram.Lookup)
		}
	})
	t.Run("agreement leaves lookup open", func(t *testing.T) {
		e := newLk(t)
		e.probeEnv(false, "xtool 1.2.3")
		e.manifest(relZ, entZ)
		e.pass(relZ, entZ)
		if got := lk(t, e.plan(), "x/z").Result; got != ResultHit {
			t.Fatalf("%s", got)
		}
	})
	t.Run("refused", func(t *testing.T) {
		e := newLk(t)
		e.probeEnv(true, "xtool 1.2.3") // the tool lives inside the checkout
		e.manifest(relZ, entZ)
		p := e.plan()
		if got := lk(t, p, "x/z").Result; got != "ineligible:probe-refused:xtool" {
			t.Fatalf("%s", got)
		}
		// A refusal is gated but, as before, is not added to the reasons.
		if hasPrefixReason(p.rec(t, "x/z").Reasons, "probe-refused:") {
			t.Errorf("reasons changed: %v", p.rec(t, "x/z").Reasons)
		}
	})
}

func TestLookupTimeoutIsGated(t *testing.T) {
	e := newLk(t)
	e.probeEnv(false, "xtool 1.2.3")
	e.mod = func(o *Options) {
		o.Probe = &ProbeOptions{Root: e.t.TempDir(), Total: time.Nanosecond,
			Run: func(ProbeRun) ([]byte, error) { time.Sleep(2 * time.Millisecond); return []byte("xtool 1.2.3"), nil }}
	}
	p := e.plan()
	found := false
	for _, r := range p.Packages {
		found = found || strings.HasPrefix(r.Lookup.Result, "ineligible:probe-timeout:")
	}
	if !found {
		t.Skipf("no probe timed out on this machine: %v", p.Probes)
	}
}

func TestLookupUntracedOS(t *testing.T) {
	e := newLk(t)
	p, err := e.planOS("darwin", nil)
	if err != nil {
		t.Fatal(err)
	}
	for _, r := range p.Packages {
		if r.Lookup == nil || r.Lookup.Result != ResultUntracedOS {
			t.Errorf("%s: %+v", r.Name, r.Lookup)
		}
	}
	if p.Histogram.Lookup[ResultUntracedOS] != len(p.Packages) || p.Lookup == nil {
		t.Errorf("%v", p.Histogram.Lookup)
	}
}

func TestLookupRefusesATreeThatCannotBeVouchedFor(t *testing.T) {
	e := newLk(t)
	o := e.f.tracedOpts() // a plain Folder
	o.Lookup = e.lookupOpts()
	_, err := BuildPlan(e.g, o)
	if err == nil || !strings.Contains(err.Error(), "refusing --ledger") || !strings.Contains(err.Error(), "clean") {
		t.Fatalf("%v", err)
	}
	// Without lookup, a plain folder still plans, as before.
	o.Lookup = nil
	if _, err := BuildPlan(e.g, o); err != nil {
		t.Fatal(err)
	}
}

func TestLookupOptionsAreValidated(t *testing.T) {
	e := newLk(t)
	for name, mod := range map[string]func(*LookupOptions){
		"no manifests": func(l *LookupOptions) { l.Manifests = nil },
		"no passes":    func(l *LookupOptions) { l.Passes = nil },
		"scope":        func(l *LookupOptions) { l.Passes = ledger.Open(e.dir, ledger.Shared) },
		"bad scope":    func(l *LookupOptions) { l.Manifests, l.Passes = ledger.OpenManifests(e.dir, 0), ledger.Open(e.dir, 0) },
		"clock":        func(l *LookupOptions) { l.Now = time.Time{} },
		"relative":     func(l *LookupOptions) { l.Root = "rel/path" },
		"flags":        func(l *LookupOptions) { l.ExecutorFlags = []string{"bad"} },
	} {
		if _, err := e.planOS("linux", mod); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// A graph that lags the tree is refused (partial guard of KILN04 11.1).
	for i := range e.g.Packages {
		if e.g.Packages[i].Name == "x/z" {
			b := "MISSING"
			e.g.Packages[i].BuildFiles["linux"] = &b
		}
	}
	if _, err := e.planOS("linux", nil); err == nil {
		t.Error("a stale graph was accepted")
	}
}

func TestLookupUnusableStores(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("symlinks")
	}
	e := newLk(t)
	e.manifest(relZ, entZ)
	e.pass(relZ, entZ)
	link := filepath.Join(t.TempDir(), "link")
	if err := os.Symlink(e.dir, link); err != nil {
		t.Skip(err)
	}
	// A symlinked manifest directory: no hits, said out loud.
	p, err := e.planOS("linux", func(l *LookupOptions) { l.Manifests = ledger.OpenManifests(link, ledger.Local) })
	if err != nil {
		t.Fatal(err)
	}
	if got := lk(t, p, "x/z"); got.Result != "ineligible:manifest-store-unusable" || got.Weak == "" ||
		len(p.Lookup.Errors) != 1 || !strings.HasPrefix(p.Lookup.Errors[0], "manifest store:") {
		t.Fatalf("%+v %v", got, p.Lookup.Errors)
	}
	// A symlinked pass ledger: the packages with manifests cannot be judged.
	p, err = e.planOS("linux", func(l *LookupOptions) { l.Passes = ledger.Open(link, ledger.Local) })
	if err != nil {
		t.Fatal(err)
	}
	if got := lk(t, p, "x/z"); got.Result != "ineligible:pass-ledger-unusable" ||
		len(p.Lookup.Errors) != 1 || !strings.HasPrefix(p.Lookup.Errors[0], "pass ledger:") {
		t.Fatalf("%+v %v", got, p.Lookup.Errors)
	}
	if got := lk(t, p, "x/a").Result; got != ResultNoManifest {
		t.Errorf("a package with no manifest is still no-manifest: %s", got)
	}
}

func TestLookupADigestFailureIsAMissNotACrash(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ, "path:README.md")
	e.mod = func(o *Options) {
		o.Source = flakyFolder{cleanFolder{source.NewFolder(e.f.dir)}, "README.md"}
	}
	l := lk(t, e.plan(), "x/z")
	if l.Result != ResultMiss || l.Manifests[0].Error == "" || l.Manifests[0].Strong != "" {
		t.Fatalf("%+v", l)
	}
	for _, en := range l.Manifests[0].Entries {
		if en.Now != "unknown" {
			t.Errorf("%+v", en)
		}
	}
}

func TestLookupRootAndFlagsAreInTheKey(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ)
	e.pass(relZ, entZ)
	// A lookup that disagrees with the recorder about the checkout path or
	// the executor flags cannot see its manifests: a miss of the weak key,
	// never a hit.
	for name, mod := range map[string]func(*LookupOptions){
		// filepath.Abs makes it absolute on the host too (C:\elsewhere\... on Windows).
		"root":  func(l *LookupOptions) { l.Root, _ = filepath.Abs("/elsewhere/checkout") },
		"flags": func(l *LookupOptions) { l.ExecutorFlags = []string{"-exec-wrapper=/other"} },
	} {
		p, err := e.planOS("linux", mod)
		if err != nil {
			t.Fatal(err)
		}
		if got := lk(t, p, "x/z").Result; got != ResultNoManifest {
			t.Errorf("%s: %s", name, got)
		}
	}
}

func TestLookupPlanIsDeterministicAndRoundTrips(t *testing.T) {
	e := newLk(t)
	e.manifest(relZ, entZ, "pkg:"+relU, "path:README.md")
	e.manifest(relZ, entZ, "path:README.md")
	e.pass(relZ, entZ, "path:README.md")
	var first []byte
	for i := 1; i <= 4; i++ {
		e.mod = func(o *Options) { o.Workers = i }
		p := e.plan()
		data, err := p.Marshal()
		if err != nil {
			t.Fatal(err)
		}
		if first == nil {
			first = data
		} else if !bytes.Equal(first, data) {
			t.Fatalf("workers=%d changed the plan", i)
		}
		back, err := ParsePlan(data)
		if err != nil {
			t.Fatal(err)
		}
		if l := lk(t, back, "x/z"); l.Result != ResultHit || len(l.Manifests) != 1 {
			t.Fatalf("%+v", l)
		}
	}
}

func TestPlanWithoutLookupIsUnchanged(t *testing.T) {
	f := newFixture(t)
	p := f.plan(baseGraph())
	data, err := p.Marshal()
	if err != nil {
		t.Fatal(err)
	}
	if bytes.Contains(data, []byte(`"lookup"`)) || bytes.Contains(data, []byte("outside_static_closure")) {
		t.Fatal("a plan without --ledger mentions lookup")
	}
	for _, r := range p.Packages {
		if r.Lookup != nil {
			t.Errorf("%s has a lookup result", r.Name)
		}
	}
}

func TestGateOrdering(t *testing.T) {
	var g gates
	if g.first() != "" {
		t.Fatal("empty gate")
	}
	g.add(gateProbe, "probe-mismatch:a")
	g.add(gateHard, "symlink:z")
	g.add(gateHard, "no-build-file:linux")
	if g.first() != "no-build-file:linux" {
		t.Errorf("%s", g.first())
	}
	g.add(gateNever, "never:p")
	g.add(gateNever, "never:a")
	if g.first() != "never:a" {
		t.Errorf("%s", g.first())
	}
}

func TestLookupTruncatesEntriesButNotTheKeyOrTheClosureDiff(t *testing.T) {
	e := newLk(t)
	n := MaxPlanEntries + 44
	entries := []string{entZ, "pkg:code/packages/x/zzz-last"} // the out-of-closure one sorts last
	for i := 0; i < n; i++ {
		entries = append(entries, fmt.Sprintf("path:code/packages/x/z/f%04d", i))
	}
	e.manifest(relZ, entries...)
	k := e.pass(relZ, entries...)
	l := lk(t, e.plan(), "x/z")
	m := l.Manifests[0]
	if l.Result != ResultHit || l.Hit.Pass.Key != key.Hex(k) {
		t.Fatalf("the strong key must use every entry: %+v", l)
	}
	if m.EntryCount != n+2 || !m.EntriesTruncated || len(m.Entries) != MaxPlanEntries {
		t.Fatalf("count %d truncated %v emitted %d", m.EntryCount, m.EntriesTruncated, len(m.Entries))
	}
	if !reflect.DeepEqual(l.OutsideStaticClosure, []string{"pkg:code/packages/x/zzz-last"}) {
		t.Errorf("the closure diff must see the full list: %v", l.OutsideStaticClosure)
	}
	// A small manifest is not truncated.
	e2 := newLk(t)
	e2.manifest(relZ, entZ)
	if m := lk(t, e2.plan(), "x/z").Manifests[0]; m.EntriesTruncated || m.EntryCount != 1 {
		t.Errorf("%+v", m)
	}
}

func TestLookupProbesRan(t *testing.T) {
	e := newLk(t)
	e.probeEnv(false, "xtool 1.2.3")
	if p := e.plan(); !p.Lookup.ProbesRan {
		t.Error("probes_ran is false though --probe ran")
	}
}

func TestNewLookupOptionsAgreesOnScope(t *testing.T) {
	lo := NewLookupOptions(t.TempDir(), ledger.Shared, lkNow, "/r", nil)
	if lo.Manifests.Scope() != ledger.Shared || lo.Passes.Scope() != ledger.Shared {
		t.Fatal("scopes diverge")
	}
	if ledger.Scope(0).String() != "invalid" || ledger.Local.String() != "local" {
		t.Error("scope names")
	}
}

func TestLookupOutsideListIsBoundedButCountedInFull(t *testing.T) {
	e := newLk(t)
	entries := []string{entZ}
	for i := 0; i < MaxPlanEntries+10; i++ {
		entries = append(entries, fmt.Sprintf("pkg:code/packages/x/ghost%04d", i))
	}
	e.manifest(relZ, entries...)
	p := e.plan()
	l := lk(t, p, "x/z")
	if len(l.OutsideStaticClosure) != MaxPlanEntries || l.OutsideCount != MaxPlanEntries+10 || !l.OutsideTruncated {
		t.Fatalf("emitted %d count %d truncated %v", len(l.OutsideStaticClosure), l.OutsideCount, l.OutsideTruncated)
	}
	if l.OutsideStaticClosure[0] != "pkg:code/packages/x/ghost0000" || p.Histogram.OutsideStaticClosure != 1 {
		t.Errorf("first %s, histogram %d", l.OutsideStaticClosure[0], p.Histogram.OutsideStaticClosure)
	}
	// A short list is complete and says nothing about truncation.
	e2 := newLk(t)
	e2.manifest(relZ, entZ, "pkg:code/packages/x/ghost")
	if l := lk(t, e2.plan(), "x/z"); l.OutsideTruncated || l.OutsideCount != 1 {
		t.Errorf("%+v", l)
	}
}
