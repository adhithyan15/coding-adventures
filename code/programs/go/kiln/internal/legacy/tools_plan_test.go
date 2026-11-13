package legacy

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// planWith builds the baseline plan after writing the given policy files.
func (f *fixture) planOpts(g *Graph, mod func(*Options)) (*Plan, error) {
	o := f.opts("linux")
	if mod != nil {
		mod(&o)
	}
	return BuildPlan(g, o)
}

func TestToolsAndEnvAreInTheKey(t *testing.T) {
	f := newFixture(t)
	f.write("code/kiln/legacy/pinned.json", `{"languages": ["other", "x"]}`)
	base := f.plan(baseGraph())
	a := base.rec(t, "x/a")
	if got := strings.Join(a.Tools, " "); got != "cc@image sh@image xtool@1.2.3" {
		t.Fatalf("tools = %q", got)
	}
	if strings.Join(base.Env, ",") != "FIXTURE_A,LANG" {
		t.Fatalf("env names = %v", base.Env)
	}
	if contains(a.Reasons, "m1-incomplete:tools-env") || !contains(a.Reasons, ReasonTrace) {
		t.Fatalf("reasons = %v", a.Reasons)
	}
	if a.Components.Tools == "" || a.Components.Env == "" || a.Components.Tools == a.Components.Env {
		t.Fatalf("components %+v", a.Components)
	}

	// A pin bump for a language in the closure changes the key, the tools
	// digest and (through policy:toolchains.json) the consumed digest.
	f.write(ToolchainsPath, strings.Replace(baseToolchains, `"xtool": "1.2.3"`, `"xtool": "1.2.4"`, 1))
	bumped := f.plan(baseGraph())
	b := bumped.rec(t, "x/a")
	if b.Key == a.Key || b.Components.Tools == a.Components.Tools || b.Components.Consumed == a.Components.Consumed {
		t.Fatalf("pin bump did not reach the key: %+v vs %+v", a.Components, b.Components)
	}
	// ...and toolchains.json is in EVERY key, so even a package with no tool
	// change moves (the static key's deliberate difference from KILN04's weak key).
	if bumped.rec(t, "x/z").Key == base.rec(t, "x/z").Key {
		t.Fatal("policy:toolchains.json is not in the key")
	}
	f.write(ToolchainsPath, baseToolchains)

	// A different language set means different tools: x/z ("other") vs x/a.
	g := baseGraph()
	g.Packages[7].Language = "other"
	z := f.plan(g).rec(t, "x/z")
	if strings.Join(z.Tools, " ") != "cc@image otool@image sh@image" || z.Components.Tools == a.Components.Tools {
		t.Fatalf("tools = %v", z.Tools)
	}
	// Every member of the closure counts: x/a depends on a package in "other".
	g = baseGraph()
	g.Packages[2].Language = "other" // x/c is in x/a's closure
	if got := f.plan(g).rec(t, "x/a").Tools; !contains(got, "otool@image") || !contains(got, "xtool@1.2.3") {
		t.Fatalf("closure languages missing from tools: %v", got)
	}

	// The host image is the identity of image-pinned tools; it is already in the key.
	f2, _ := f.planOpts(baseGraph(), func(o *Options) { o.Host = imager("test/other-image") })
	if f2.rec(t, "x/a").Components.Tools == a.Components.Tools {
		t.Fatal("image id is not the identity of image-pinned tools")
	}
}

func TestEnvIsInTheKeyAndNeverPrinted(t *testing.T) {
	f := newFixture(t)
	const secretish = "value-that-must-not-leak-9f2c"
	p1, err := f.planOpts(baseGraph(), func(o *Options) { o.Environ = []string{"FIXTURE_A=" + secretish, "HOME=/h"} })
	if err != nil {
		t.Fatal(err)
	}
	p2, _ := f.planOpts(baseGraph(), func(o *Options) { o.Environ = []string{"FIXTURE_A=other", "HOME=/h"} })
	p3, _ := f.planOpts(baseGraph(), func(o *Options) { o.Environ = []string{"FIXTURE_A=" + secretish, "HOME=/elsewhere", "UNLISTED=1"} })
	r1, r2, r3 := p1.rec(t, "x/a"), p2.rec(t, "x/a"), p3.rec(t, "x/a")
	if r1.Key == r2.Key || r1.Components.Env == r2.Components.Env {
		t.Fatal("an allowed variable's value is not in the key")
	}
	if r1.Key != r3.Key || r1.Components.Env != r3.Components.Env {
		t.Fatal("an unlisted variable changed the key")
	}
	// Unset and empty differ.
	p4, _ := f.planOpts(baseGraph(), func(o *Options) { o.Environ = []string{"FIXTURE_A="} })
	p5, _ := f.planOpts(baseGraph(), func(o *Options) { o.Environ = nil })
	if p4.rec(t, "x/a").Key == p5.rec(t, "x/a").Key {
		t.Fatal("empty and unset are the same")
	}
	if len(p5.Env) != 0 || p5.Env == nil {
		t.Fatalf("no variables must give an empty list, got %#v", p5.Env)
	}
	// Names and digests appear in the plan; values never do.
	data, err := p1.Marshal()
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(data), secretish) || !strings.Contains(string(data), `"FIXTURE_A"`) {
		t.Fatalf("plan leaks or omits env: %s", data[:min(len(data), 400)])
	}
	// env.json's own digest is a consumed entry.
	f.write(EnvPath, `{"names": ["FIXTURE_*", "LANG", "TZ"]}`)
	p6, _ := f.planOpts(baseGraph(), func(o *Options) { o.Environ = []string{"FIXTURE_A=" + secretish} })
	if p6.rec(t, "x/a").Components.Consumed == r1.Components.Consumed {
		t.Fatal("policy:env.json is not in the key")
	}
	// A secret-like variable matching a wildcard is an error, naming the variable only.
	f.write(EnvPath, `{"names": ["FIXTURE_*"]}`)
	_, err = f.planOpts(baseGraph(), func(o *Options) { o.Environ = []string{"FIXTURE_TOKEN=" + secretish} })
	if err == nil || !strings.Contains(err.Error(), "FIXTURE_TOKEN") || strings.Contains(err.Error(), secretish) {
		t.Fatalf("err = %v", err)
	}
}

func TestUnknownToolchainIsHard(t *testing.T) {
	f := newFixture(t)
	f.write("code/kiln/legacy/pinned.json", `{"languages": ["nosuch", "x"]}`)
	g := baseGraph()
	g.Packages[2].Language = "nosuch" // x/c, a dep of x/b, which x/a depends on
	g.Packages[7].ExtraToolchains = []string{"alsonot", "x"}
	p := f.plan(g)
	for _, n := range []string{"x/c", "x/b", "x/a"} {
		r := p.rec(t, n)
		if r.Class != ClassUncacheable || !contains(r.Reasons, "unknown-toolchain:nosuch") || r.Key == "" {
			t.Fatalf("%s: %+v", n, r)
		}
	}
	z := p.rec(t, "x/z")
	if z.Class != ClassUncacheable || !contains(z.Reasons, "unknown-toolchain:alsonot") || contains(z.Reasons, "unknown-toolchain:nosuch") {
		t.Fatalf("x/z: %+v", z)
	}
	// An unknown entry is never "no tools": the known ones are still listed.
	if !contains(z.Tools, "sh@image") {
		t.Fatalf("%v", z.Tools)
	}
	count := 0
	for _, r := range p.Packages {
		if hasPrefixReason(r.Reasons, "unknown-toolchain:") {
			count++
		}
	}
	if count < 4 || p.Histogram.Reasons["unknown-toolchain"] != count {
		t.Fatalf("histogram %v, %d packages", p.Histogram.Reasons, count)
	}
	// A package that merely contains no language entry problem stays clean.
	if r := p.rec(t, "x/u"); hasPrefixReason(r.Reasons, "unknown-toolchain:") {
		t.Fatalf("x/u: %v", r.Reasons)
	}
}

func TestPolicyFilesAreRequiredAndStrict(t *testing.T) {
	for _, path := range []string{ToolchainsPath, EnvPath, HostRootsPath} {
		f := newFixture(t)
		f.remove(path)
		if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
			t.Errorf("a missing %s was accepted", path)
		}
		f.write(path, `[]`)
		if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil || !strings.Contains(err.Error(), path) {
			t.Errorf("a malformed %s: %v", path, err)
		}
	}
	// host-roots.json is cross-checked against toolchains.json.
	f := newFixture(t)
	f.write(HostRootsPath, `{"roots": [{"path": "/a", "kind": "tool", "tool": "nope"}], "ephemeral": [], "absent-ok": []}`)
	if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
		t.Error("host-roots with an unpinned tool accepted")
	}
}

func TestHostRootsDigestDoesNotEnterTheStaticKey(t *testing.T) {
	f := newFixture(t)
	a := f.plan(baseGraph()).rec(t, "x/a")
	f.write(HostRootsPath, `{"roots": [{"path": "/usr", "kind": "image"}, {"path": "/zzz", "kind": "image"}], "ephemeral": [], "absent-ok": []}`)
	b := f.plan(baseGraph()).rec(t, "x/a")
	if a.Key != b.Key {
		t.Fatal("host-roots.json changed the static key (it is for the KILN04 weak key)")
	}
	pol, err := loadPolicy(f.opts("linux").Source, map[string]bool{})
	if err != nil || pol.roots.Digest == ([32]byte{}) {
		t.Fatalf("%v", err)
	}
}

func TestProbesInThePlan(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("fake executables")
	}
	f := newFixture(t)
	bin := filepath.Join(f.dir, "fakebin")
	if err := os.MkdirAll(bin, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(bin, "xtool"), []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	f.write(ToolchainsPath, strings.Replace(baseToolchains, `"/opt/x/bin"`, `"`+filepath.ToSlash(bin)+`"`, 1))
	// A tree with a fake tool in it is "dirty" for a Git source but not a Folder.
	out := "xtool 1.2.3"
	probe := func(o *Options) {
		o.Probe = &ProbeOptions{Root: f.dir + "/nowhere-else", Run: func(ProbeRun) ([]byte, error) { return []byte(out), nil }}
	}
	if err := os.MkdirAll(f.dir+"/nowhere-else", 0o755); err != nil {
		t.Fatal(err)
	}
	// The fake tool is inside the checkout here, so it is refused: no verdict.
	insideRoot := func(o *Options) {
		o.Probe = &ProbeOptions{Root: f.dir, Run: func(ProbeRun) ([]byte, error) { t.Fatal("ran inside the checkout"); return nil, nil }}
	}
	p, err := f.planOpts(baseGraph(), insideRoot)
	if err != nil || results(p.Probes)["xtool"] != ProbeRefused || hasPrefixReason(p.rec(t, "x/a").Reasons, "probe-mismatch:") {
		t.Fatalf("%v %v", err, p.Probes)
	}
	// Off by default: no probes, no probe reasons.
	off := f.plan(baseGraph())
	if off.Probes != nil {
		t.Fatalf("probes ran without --probe: %v", off.Probes)
	}
	on, err := f.planOpts(baseGraph(), probe)
	if err != nil {
		t.Fatal(err)
	}
	if results(on.Probes)["xtool"] != ProbeOK || hasPrefixReason(on.rec(t, "x/a").Reasons, "probe-mismatch:") {
		t.Fatalf("%v", on.Probes)
	}
	out = "xtool 1.2.4"
	bad, err := f.planOpts(baseGraph(), probe)
	if err != nil {
		t.Fatal(err)
	}
	a := bad.rec(t, "x/a")
	if !contains(a.Reasons, "probe-mismatch:xtool") {
		t.Fatalf("no soft reason: %v", a.Reasons)
	}
	// A soft reason is reported only: same class, same key.
	if a.Class != on.rec(t, "x/a").Class || a.Key != on.rec(t, "x/a").Key || a.Key != off.rec(t, "x/a").Key {
		t.Fatal("a probe finding changed the class or the key")
	}
	if bad.Histogram.Reasons["probe-mismatch"] == 0 {
		t.Fatalf("histogram %v", bad.Histogram.Reasons)
	}
	// A probe that cannot even start (unreadable root) fails the plan loudly.
	if _, err := f.planOpts(baseGraph(), func(o *Options) { o.Probe = &ProbeOptions{Root: f.dir + "/missing"} }); err == nil {
		t.Fatal("bad probe root accepted")
	}
	// Round trip: a plan with probes parses back strictly.
	data, _ := bad.Marshal()
	if back, err := ParsePlan(data); err != nil || len(back.Probes) == 0 {
		t.Fatalf("%v", err)
	}
}
