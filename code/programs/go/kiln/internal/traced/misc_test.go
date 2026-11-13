package traced

import (
	"debug/elf"
	"encoding/binary"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func shStep(line string) *trace.Trace {
	return step(execve("/usr/bin/sh", "sh", "-c", line))
}

func TestCheckRecipe(t *testing.T) {
	a, b := shStep("make"), shStep("make test")
	if got := CheckRecipe([]*trace.Trace{a, b}, []string{"make", "make test"}); got != nil {
		t.Fatalf("%q", got)
	}
	bad := []string{"trace-incomplete:recipe-mismatch"}
	for name, c := range map[string]struct {
		steps []*trace.Trace
		lines []string
	}{
		"fewer traces":  {[]*trace.Trace{a}, []string{"make", "make test"}},
		"more traces":   {[]*trace.Trace{a, b}, []string{"make"}},
		"other line":    {[]*trace.Trace{a, b}, []string{"make", "make check"}},
		"byte for byte": {[]*trace.Trace{a}, []string{"make "}},
		"nil trace":     {[]*trace.Trace{nil}, []string{"make"}},
		"no step exec":  {[]*trace.Trace{step(stat("go.work"))}, []string{"make"}},
		"not sh -c":     {[]*trace.Trace{step(execve("/usr/bin/sh", "sh", "-e", "make"))}, []string{"make"}},
		"extra args":    {[]*trace.Trace{step(execve("/usr/bin/sh", "sh", "-c", "make", "x"))}, []string{"make"}},
		"undecoded":     {[]*trace.Trace{step(execve("/usr/bin/sh"))}, []string{"make"}},
	} {
		got := CheckRecipe(c.steps, c.lines)
		if len(got) != 1 || got[0] != bad[0] {
			t.Errorf("%s: %q", name, got)
		}
	}
	// A shell started by an absolute name is still the step.
	if got := CheckRecipe([]*trace.Trace{step(execve("/bin/sh", "/bin/sh", "-c", "x"))}, []string{"x"}); got != nil {
		t.Fatal(got)
	}
	// Classify runs the check when given a recipe.
	h := newHarness(t)
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{a}, Recipe: []string{"make clean"}})
	if err != nil || !hasReason(rec, "trace-incomplete:recipe-mismatch") {
		t.Fatalf("%v %q", err, rec.Reasons)
	}
	rec, _ = Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{a}, Recipe: []string{"make"}})
	if hasReason(rec, "trace-incomplete:recipe-mismatch") {
		t.Fatal("matching recipe")
	}
}

func TestPackageIndex(t *testing.T) {
	ix := NewPackageIndex([]string{"a/b", "a/b/c", "x"})
	for in, want := range map[string]string{
		"a/b": "a/b", "a/b/f": "a/b", "a/b/c": "a/b/c", "a/b/c/d/e": "a/b/c", "a/b/cc": "a/b",
		"x/y": "x", "x": "x",
	} {
		if got, ok := ix.Enclosing(in); !ok || got != want {
			t.Errorf("%s: %q %v, want %q", in, got, ok, want)
		}
	}
	for _, in := range []string{"", "a", "xx", "y/x", "a/bb"} {
		if got, ok := ix.Enclosing(in); ok {
			t.Errorf("%s: %q", in, got)
		}
	}
}

func TestSourceTreeOverAFolder(t *testing.T) {
	dir := t.TempDir()
	write := func(rel, body string) {
		p := filepath.Join(dir, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(p, []byte(body), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	write("pkg/a.txt", "hello")
	write("pkg/sub/b.txt", "world")
	if runtime.GOOS != "windows" {
		if err := os.Symlink("a.txt", filepath.Join(dir, "pkg", "lnk")); err != nil {
			t.Fatal(err)
		}
	}
	tr, err := NewSourceTree(source.NewFolder(dir))
	if err != nil {
		t.Fatal(err)
	}
	if tr.Stat("").Kind != KindDir || tr.Stat("pkg").Kind != KindDir || tr.Stat("pkg/sub").Kind != KindDir {
		t.Fatal("directories")
	}
	if tr.Stat("pkg/a.txt").Kind != KindFile || tr.Stat("pkg/nope").Kind != KindAbsent || tr.Stat("pkg/sub/b.txt/x").Kind != KindAbsent {
		t.Fatal("files")
	}
	if runtime.GOOS != "windows" {
		if n := tr.Stat("pkg/lnk"); n.Kind != KindLink || n.Target != "a.txt" {
			t.Fatalf("link %+v", n)
		}
		if _, err := tr.Head("pkg/lnk", 10); err == nil {
			t.Fatal("a link has no head")
		}
	}
	if b, err := tr.Head("pkg/a.txt", 3); err != nil || string(b) != "hel" {
		t.Fatalf("%q %v", b, err)
	}
	if b, err := tr.Head("pkg/a.txt", 100); err != nil || string(b) != "hello" {
		t.Fatalf("%q %v", b, err)
	}
	if _, err := tr.Head("pkg", 3); err == nil {
		t.Fatal("a directory has no head")
	}
	if _, err := tr.Head("pkg/gone", 3); err == nil {
		t.Fatal("absent")
	}
	// An empty file has an empty head.
	write("empty", "")
	tr, _ = NewSourceTree(source.NewFolder(dir))
	if b, err := tr.Head("empty", 10); err != nil || len(b) != 0 {
		t.Fatalf("%q %v", b, err)
	}
	// Source errors surface.
	if _, err := NewSourceTree(failingSource{}); err == nil {
		t.Fatal("expected an error")
	}
	if _, err := NewSourceTree(describeFails{}); err == nil {
		t.Fatal("expected a describe error")
	}
	// A source that cannot describe still lists plain files.
	pt, err := NewSourceTree(plainSource{})
	if err != nil || pt.Stat("a/f").Kind != KindFile {
		t.Fatalf("%v", err)
	}
	if _, err := pt.Head("a/f", 1); err == nil {
		t.Fatal("open error")
	}
}

type failingSource struct{ source.Source }

func (failingSource) Files(string, []string) ([]string, error) { return nil, os.ErrInvalid }

type plainSource struct{ source.Source }

func (plainSource) Files(string, []string) ([]string, error) { return []string{"a/f"}, nil }
func (plainSource) Open(string) (io.ReadCloser, error)       { return nil, os.ErrNotExist }

type describeFails struct{ plainSource }

func (describeFails) Describe(string) (source.Info, error) { return source.Info{}, os.ErrInvalid }

// A real strace-shaped trace, parsed by the stage 1a parser and classified.
func TestFromParsedStraceText(t *testing.T) {
	text := strings.Join([]string{
		`100  execve("/usr/bin/sh", ["sh", "-c", "cc -c src/x.go"], 0x7ffd /* 3 vars */) = 0`,
		`100  openat(AT_FDCWD</w/repo/code/p/a>, "src/x.go", O_RDONLY) = 3</w/repo/code/p/a/src/x.go>`,
		`100  newfstatat(AT_FDCWD</w/repo/code/p/a>, "../b/missing", 0x7ffd, 0) = -1 ENOENT (No such file or directory)`,
		`100  openat(AT_FDCWD</w/repo/code/p/a>, "x.o", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 4</w/repo/code/p/a/x.o>`,
		`100  getdents64(5</w/repo/code/p/a/src>, 0x55 /* 2 entries */, 32768) = 48`,
		`100  connect(6<socket:[9]>, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("93.184.216.34")}, 16) = -1 EINPROGRESS (Operation now in progress)`,
		`100  +++ exited with 0 +++`,
		``,
	}, "\n")
	tr, err := trace.Parse(strings.NewReader(text), testRoot+"/code/p/a", trace.Limits{})
	if err != nil {
		t.Fatal(err)
	}
	h := newHarness(t)
	h.image("/usr/bin/sh")
	h.fs.bytes("/usr/bin/sh", buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.image("/usr/bin/sh", loader)
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{tr}, Recipe: []string{"cc -c src/x.go"}})
	if err != nil {
		t.Fatal(err)
	}
	eq(t, "entries", rec.Entries, "list:code/p/a/src", "pkg:code/p/a", "pkg:code/p/b")
	eq(t, "writes", rec.Writes, "code/p/a/x.o")
	eq(t, "reasons", rec.Reasons, "network:/usr/bin/sh")
	// The default dump of a record is stable JSON-able data.
	if rec.Package != "code/p/a" {
		t.Fatal(rec.Package)
	}
}
