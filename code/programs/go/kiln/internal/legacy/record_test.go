package legacy

import (
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

func TestRecordExecutorArgsAreCanonicalFlags(t *testing.T) {
	s, err := CanonicalExecutorArgs(RecordExecutorArgs)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(s, "-clippy=false") || !strings.Contains(s, "-prune-leaf-targets=false") {
		t.Fatal(s)
	}
}

func TestRecordEnviron(t *testing.T) {
	f := newTracedFixture(t) // env.json allows FIXTURE_* and LANG
	src := source.NewFolder(f.dir)
	got, err := RecordEnviron(src, []string{
		"FIXTURE_B=2", "FIXTURE_A=1", "OTHER=x", "PATH=/evil/bin", "HOME=/first", "HOME=/home/runner",
		"TMPDIR=/somewhere/else", "LANG=C.UTF-8",
	}, "/tmp/kiln-tmp")
	if err != nil {
		t.Fatal(err)
	}
	want := []string{"FIXTURE_A=1", "FIXTURE_B=2", "HOME=/home/runner", "LANG=C.UTF-8", "TMPDIR=/tmp/kiln-tmp"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	// Without HOME the entry is simply absent.
	got, err = RecordEnviron(src, []string{"FIXTURE_A=1"}, "/t")
	if err != nil || !reflect.DeepEqual(got, []string{"FIXTURE_A=1", "TMPDIR=/t"}) {
		t.Fatalf("%v %v", got, err)
	}
}

func TestRecordEnvironRefusals(t *testing.T) {
	f := newTracedFixture(t)
	src := source.NewFolder(f.dir)
	// A value that looks like a credential is an error, not a silent skip.
	if _, err := RecordEnviron(src, []string{"FIXTURE_A=ghp_abcdef"}, "/t"); err == nil {
		t.Fatal("credential-like value accepted")
	}
	// env.json missing: the facts cannot be derived.
	f.remove("code/kiln/legacy/env.json")
	if _, err := RecordEnviron(source.NewFolder(f.dir), nil, "/t"); err == nil {
		t.Fatal("missing env.json accepted")
	}
}

// The environment RecordEnviron selects, fed back to ScrubbedEnv, gives the
// variables a traced run gets: that is the property the merge's recomputation
// of the weak key depends on.
func TestRecordEnvironRoundTripsThroughTheWeakKey(t *testing.T) {
	f := newTracedFixture(t)
	src := source.NewFolder(f.dir)
	process := []string{"FIXTURE_A=1", "OTHER=2", "HOME=/home/runner", "PATH=/evil", "UNLISTED=1"}
	facts, err := RecordEnviron(src, process, "/tmp/kiln-tmp")
	if err != nil {
		t.Fatal(err)
	}
	direct := f.opts("linux")
	direct.Environ = append(append([]string{}, process...), "TMPDIR=/tmp/kiln-tmp")
	viaFacts := f.opts("linux")
	viaFacts.Environ = facts
	a, err := NewTraced(baseGraph(), direct, f.root(), RecordExecutorArgs)
	if err != nil {
		t.Fatal(err)
	}
	b, err := NewTraced(baseGraph(), viaFacts, f.root(), RecordExecutorArgs)
	if err != nil {
		t.Fatal(err)
	}
	wa, wb := mustWeak(t, a, pkgA), mustWeak(t, b, pkgA)
	if wa.Key != wb.Key {
		t.Fatal("the weak key depends on environment the host facts do not carry")
	}
}

func TestTracedRecordEnvAndHostRoots(t *testing.T) {
	f := newTracedFixture(t)
	tr := f.traced(baseGraph())
	env, tools, err := tr.RecordEnv(pkgA)
	if err != nil {
		t.Fatal(err)
	}
	w := mustWeak(t, tr, pkgA)
	if !reflect.DeepEqual(EnvironOf(env), w.Environ()) || len(tools) == 0 {
		t.Fatalf("RecordEnv differs from the weak key's environment: %v vs %v", EnvironOf(env), w.Environ())
	}
	if _, _, err := tr.RecordEnv("code/packages/x/nope"); err == nil {
		t.Fatal("unknown package accepted")
	}
	if hr := tr.HostRoots(); hr == nil || len(hr.Roots) != 1 || hr.Roots[0].Path != "/usr" {
		t.Fatalf("%+v", hr)
	}
}

// A package whose key is refused (here: its closure names an unknown
// toolchain) still runs, so it still gets an environment.
func TestRecordEnvForARefusedPackage(t *testing.T) {
	f := newTracedFixture(t)
	g := baseGraph()
	for i := range g.Packages {
		if g.Packages[i].RelPath == pkgZ {
			g.Packages[i].Language = "nonesuch"
		}
	}
	tr := f.traced(g)
	if _, err := tr.WeakKey(pkgZ); err == nil {
		t.Fatal("expected a refusal")
	} else if _, ok := IsRefusal(err); !ok {
		t.Fatal(err)
	}
	env, _, err := tr.RecordEnv(pkgZ)
	if err != nil {
		t.Fatal(err)
	}
	if v, ok := envOf(env, "LANG"); !ok || v != "C.UTF-8" {
		t.Fatalf("%+v", env)
	}
}
