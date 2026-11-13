package legacy

import (
	"bytes"
	"errors"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// faulty wraps a Source and fails selected operations.
type faulty struct {
	Source
	filesErr     error
	failDescribe map[string]bool
	failOpen     map[string]bool
	badRead      map[string]bool
	untracked    []string
	untrackedErr error
	clean        bool // implements Cleaner when true
}

type errReader struct{}

func (errReader) Read([]byte) (int, error) { return 0, errors.New("read failed") }
func (errReader) Close() error             { return nil }

func (f *faulty) Files(root string, pats []string) ([]string, error) {
	if f.filesErr != nil {
		return nil, f.filesErr
	}
	return f.Source.Files(root, pats)
}

func (f *faulty) Describe(p string) (source.Info, error) {
	if f.failDescribe[p] {
		return source.Info{}, errors.New("describe failed")
	}
	return f.Source.Describe(p)
}

func (f *faulty) Open(p string) (io.ReadCloser, error) {
	if f.failOpen[p] {
		return nil, errors.New("open failed")
	}
	if f.badRead[p] {
		return errReader{}, nil
	}
	return f.Source.Open(p)
}

// cleaner adds Cleaner on top of faulty.
type cleaner struct{ *faulty }

func (c cleaner) Untracked() ([]string, error) { return c.untracked, c.untrackedErr }

func TestBuildPlanOptionErrors(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	o := f.opts("linux")
	for name, mod := range map[string]func(*Options){
		"no source": func(o *Options) { o.Source = nil },
		"no host":   func(o *Options) { o.Host = nil },
		"bad os":    func(o *Options) { o.OS = "plan9" },
		"bad arch":  func(o *Options) { o.Arch = "mips" },
	} {
		oo := o
		mod(&oo)
		if _, err := BuildPlan(g, oo); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func TestFaultInjection(t *testing.T) {
	f := newFixture(t)
	base := Source(source.NewFolder(f.dir))
	cases := map[string]*faulty{
		"files":               {Source: base, filesErr: errors.New("list failed")},
		"describe own file":   {Source: base, failDescribe: map[string]bool{"code/packages/x/a/src.txt": true}},
		"open own file":       {Source: base, failOpen: map[string]bool{"code/packages/x/a/src.txt": true}},
		"read own file":       {Source: base, badRead: map[string]bool{"code/packages/x/a/src.txt": true}},
		"open recipe":         {Source: base, failOpen: map[string]bool{"code/packages/x/a/BUILD": true}},
		"open pinned":         {Source: base, failOpen: map[string]bool{PinnedPath: true}},
		"read pinned":         {Source: base, badRead: map[string]bool{PinnedPath: true}},
		"open boundary":       {Source: base, failOpen: map[string]bool{BoundaryPath: true}},
		"open build tool mod": {Source: base, failOpen: map[string]bool{BuildToolPath + "/go.mod": true}},
		"describe read file":  {Source: base, failDescribe: map[string]bool{"code/grammars/g/one.txt": true}},
		"describe tool file":  {Source: base, failDescribe: map[string]bool{BuildToolPath + "/main.go": true}},
		"describe glob match": {Source: base, failDescribe: map[string]bool{"code/lang/x.mll": true}},
		"describe ancestor":   {Source: base, failDescribe: map[string]bool{"mise.toml": true}},
	}
	for name, fs := range cases {
		t.Run(name, func(t *testing.T) {
			o := f.opts("linux")
			o.Source = fs
			if _, err := BuildPlan(baseGraph(), o); err == nil {
				t.Fatal("accepted")
			}
		})
	}
	// A file-valued repo_input whose Describe fails.
	g := baseGraph()
	g.Packages[2].RepoInputs = []string{"code/shared/cfg.json"}
	o := f.opts("linux")
	o.Source = &faulty{Source: base, failDescribe: map[string]bool{"code/shared/cfg.json": true}}
	if _, err := BuildPlan(g, o); err == nil {
		t.Fatal("accepted")
	}
}

func TestRefuseDirty(t *testing.T) {
	f := newFixture(t)
	base := &faulty{Source: source.NewFolder(f.dir)}
	o := f.opts("linux")
	o.Source = cleaner{&faulty{Source: base.Source, untracked: []string{"b", "a", "c", "d"}}}
	_, err := BuildPlan(baseGraph(), o)
	if err == nil || !strings.Contains(err.Error(), "not clean") || !strings.Contains(err.Error(), "4 untracked") {
		t.Fatalf("err = %v", err)
	}
	o.Source = cleaner{&faulty{Source: base.Source, untrackedErr: errors.New("status failed")}}
	if _, err := BuildPlan(baseGraph(), o); err == nil {
		t.Fatal("status failure accepted")
	}
	o.Source = cleaner{&faulty{Source: base.Source}}
	if _, err := BuildPlan(baseGraph(), o); err != nil {
		t.Fatalf("clean source refused: %v", err)
	}
}

func TestGitDirtyTreeRefused(t *testing.T) {
	f := newFixture(t)
	gitInit(t, f.dir)
	o := f.opts("linux")
	o.Source = source.NewGit(f.dir)
	if _, err := BuildPlan(baseGraph(), o); err != nil {
		t.Fatalf("clean tree refused: %v", err)
	}
	f.write("code/packages/x/a/target/out.o", "build product")
	o.Source = source.NewGit(f.dir)
	if _, err := BuildPlan(baseGraph(), o); err == nil || !strings.Contains(err.Error(), "not clean") {
		t.Fatalf("dirty tree: %v", err)
	}
}

func TestGitAndFolderAgree(t *testing.T) {
	f := newFixture(t)
	if runtime.GOOS != "windows" {
		if err := os.WriteFile(filepath.Join(f.dir, "code/packages/x/a/run.sh"), []byte("#!/bin/sh\n"), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.Symlink("src.txt", filepath.Join(f.dir, "code/packages/x/a/link")); err != nil {
			t.Fatal(err)
		}
	}
	gitInit(t, f.dir)
	of := f.opts("linux")
	og := f.opts("linux")
	og.Source = source.NewGit(f.dir)
	pf, err := BuildPlan(baseGraph(), of)
	if err != nil {
		t.Fatal(err)
	}
	pg, err := BuildPlan(baseGraph(), og)
	if err != nil {
		t.Fatal(err)
	}
	bf, _ := pf.Marshal()
	bg, _ := pg.Marshal()
	if !bytes.Equal(bf, bg) {
		t.Fatal("folder and git plans differ on the same committed tree")
	}
}

func TestSymlinkEntriesCoverTargetText(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("symlinks need privileges on Windows")
	}
	f := newFixture(t)
	link := filepath.Join(f.dir, "code/packages/x/a/link")
	if err := os.Symlink("src.txt", link); err != nil {
		t.Fatal(err)
	}
	a := f.plan(baseGraph()).rec(t, "x/a").Key
	if err := os.Remove(link); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink("BUILD", link); err != nil {
		t.Fatal(err)
	}
	if f.plan(baseGraph()).rec(t, "x/a").Key == a {
		t.Fatal("link target text must be in the key")
	}
}

func TestExecBitIsInTheKey(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("no exec bit on Windows")
	}
	f := newFixture(t)
	a := f.plan(baseGraph()).rec(t, "x/a").Key
	if err := os.Chmod(filepath.Join(f.dir, "code/packages/x/a/src.txt"), 0o755); err != nil {
		t.Fatal(err)
	}
	if f.plan(baseGraph()).rec(t, "x/a").Key == a {
		t.Fatal("exec bit not in key")
	}
}

func TestBuildToolReplaceMayBeAFile(t *testing.T) {
	f := newFixture(t)
	f.write(BuildToolPath+"/go.mod", "module bt\nreplace a => ../../../../go.work\n")
	pl, err := f.planner()
	if err != nil {
		t.Fatal(err)
	}
	if files, err := pl.buildToolFiles(); err != nil || !contains(files, "go.work") {
		t.Fatalf("%v %v", files, err)
	}
	// An empty build tool directory is refused, not hashed as nothing.
	f2 := newFixture(t)
	f2.remove(BuildToolPath + "/main.go")
	f2.remove(BuildToolPath + "/go.mod")
	f2.write(BuildToolPath+"/go.mod", "module bt\n")
	f2.remove(BuildToolPath + "/go.mod")
	if _, err := BuildPlan(baseGraph(), f2.opts("linux")); err == nil {
		t.Fatal("empty build tool accepted")
	}
}

func TestPlanRoundTrip(t *testing.T) {
	f := newFixture(t)
	p := f.plan(baseGraph())
	data, err := p.Marshal()
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.HasSuffix(data, []byte("}\n")) {
		t.Fatal("no trailing newline")
	}
	q, err := ParsePlan(data)
	if err != nil {
		t.Fatal(err)
	}
	if len(q.Packages) != len(p.Packages) || q.Packages[0].Key != p.Packages[0].Key {
		t.Fatal("round trip changed the plan")
	}
	if _, ok := q.Find("x/a"); !ok {
		t.Fatal("Find failed")
	}
	for _, name := range []string{"nope", "", "x/zz", "0"} {
		if _, ok := q.Find(name); ok {
			t.Fatalf("Find(%q) found something", name)
		}
	}
	dir := t.TempDir()
	path := filepath.Join(dir, "plan.json")
	if err := WriteFileAtomic(path, data); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadPlanFile(path); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadPlanFile(filepath.Join(dir, "none")); err == nil {
		t.Fatal("missing plan accepted")
	}
	if _, err := ReadPlanFile(dir); err == nil {
		t.Fatal("directory accepted as plan")
	}
	if _, err := ParsePlan([]byte(`{"schema":"x"}`)); err == nil {
		t.Fatal("wrong schema accepted")
	}
	if _, err := ParsePlan([]byte(`{"schema":"kiln-legacy-plan/1","extra":1}`)); err == nil {
		t.Fatal("unknown field accepted")
	}
	big := filepath.Join(dir, "big")
	fh, _ := os.Create(big)
	_ = fh.Truncate(MaxPlanBytes + 10)
	fh.Close()
	if _, err := ReadPlanFile(big); err == nil {
		t.Fatal("oversized plan accepted")
	}
}

func TestWriteFileAtomic(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "out.json")
	if err := WriteFileAtomic(path, []byte("one")); err != nil {
		t.Fatal(err)
	}
	if err := WriteFileAtomic(path, []byte("two")); err != nil {
		t.Fatal(err)
	}
	if b, _ := os.ReadFile(path); string(b) != "two" {
		t.Fatalf("got %q", b)
	}
	if ents, _ := os.ReadDir(dir); len(ents) != 1 {
		t.Fatalf("temp file left behind: %v", ents)
	}
	if err := WriteFileAtomic(filepath.Join(dir, "no", "such", "out"), []byte("x")); err == nil {
		t.Fatal("missing directory accepted")
	}
	if err := WriteFileAtomic(dir, []byte("x")); err == nil {
		t.Fatal("directory target accepted")
	}
	if runtime.GOOS != "windows" {
		target := filepath.Join(dir, "victim")
		if err := os.WriteFile(target, []byte("keep"), 0o644); err != nil {
			t.Fatal(err)
		}
		link := filepath.Join(dir, "link")
		if err := os.Symlink(target, link); err != nil {
			t.Fatal(err)
		}
		if err := WriteFileAtomic(link, []byte("x")); err == nil {
			t.Fatal("symlink target accepted")
		}
		if b, _ := os.ReadFile(target); string(b) != "keep" {
			t.Fatal("symlink target was written through")
		}
	}
}

func TestHistogramAndClasses(t *testing.T) {
	recs := make([]Record, 0, 10)
	for i := 1; i <= 10; i++ {
		recs = append(recs, Record{Class: ClassUnpinned, ClosureSize: i, Reasons: []string{"unpinned:x", "unknown-repo-inputs:a", "unknown-repo-inputs:b"}})
	}
	h := histogram(recs)
	if h.ClosureSizes != (Sizes{Min: 1, Median: 6, P90: 9, Max: 10}) {
		t.Fatalf("%+v", h.ClosureSizes)
	}
	if h.Classes[ClassUnpinned] != 10 || h.Reasons["unknown-repo-inputs"] != 10 || h.Reasons["unpinned:x"] != 10 {
		t.Fatalf("%+v", h)
	}
	if got := histogram(nil); got.ClosureSizes != (Sizes{}) {
		t.Fatalf("%+v", got)
	}
	if classify(true, true) != ClassUncacheable || classify(false, true) != ClassUnpinned || classify(false, false) != ClassUncacheable {
		t.Fatal("classify")
	}
	if reasonGroup("never:x") != "never" || reasonGroup(ReasonTrace) != ReasonTrace || reasonGroup("probe-mismatch:go") != "probe-mismatch" {
		t.Fatal("reasonGroup")
	}
}

func TestForEachAndWorkers(t *testing.T) {
	if err := forEach(0, 4, func(int) error { return errors.New("never") }); err != nil {
		t.Fatal(err)
	}
	want := errors.New("boom")
	if err := forEach(50, 4, func(i int) error {
		if i == 7 {
			return want
		}
		return nil
	}); err != want {
		t.Fatalf("got %v", err)
	}
	sum := make([]int, 100)
	if err := forEach(100, 8, func(i int) error { sum[i] = i; return nil }); err != nil || sum[99] != 99 {
		t.Fatal("not all indexes ran")
	}
	if (Options{Workers: 1000}).workers() != 32 || (Options{}).workers() < 1 {
		t.Fatal("workers bounds")
	}
	if a, err := HostArch(); err == nil && a != "x64" && a != "arm64" {
		t.Fatalf("arch %q", a)
	}
	if keyOS("darwin") != "macos" || keyOS("linux") != "linux" {
		t.Fatal("keyOS")
	}
}

func TestSymlinkedBuildFileIsPerPackageEscape(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("symlinks need privileges on Windows")
	}
	f := newFixture(t)
	f.remove("code/packages/x/z/BUILD")
	if err := os.Symlink("../a/BUILD", filepath.Join(f.dir, "code/packages/x/z/BUILD")); err != nil {
		t.Fatal(err)
	}
	p := f.plan(baseGraph()) // must not abort
	r := p.rec(t, "x/z")
	if r.Key != "" || r.Class != ClassUncacheable || !contains(r.Reasons, "symlink:BUILD") || r.Components.Recipe != "" {
		t.Fatalf("%+v", r)
	}
	if p.rec(t, "x/a").Key == "" {
		t.Fatal("other packages must still be keyed")
	}
	// A BUILD that cannot be described at all is still a hard error.
	o := f.opts("linux")
	o.Source = &faulty{Source: o.Source, failDescribe: map[string]bool{"code/packages/x/a/BUILD": true}}
	if _, err := BuildPlan(baseGraph(), o); err == nil {
		t.Fatal("describe failure accepted")
	}
}
