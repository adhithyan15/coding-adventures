package traced

import (
	"bytes"
	"debug/elf"
	"encoding/binary"
	"errors"
	"io/fs"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// The fixture repository, as the traced action saw it:
//
//	/w/repo                          the checkout
//	  code/p/a            package a  (README, src/x.go, a link "lnk" -> src/x.go)
//	  code/p/a/nested     package nested, inside a
//	  code/p/b            package b  (b.txt, script.sh with a #! line)
//	  code/scripts/gen.py             outside every package
//	  go.work                         outside every package
const testRoot = "/w/repo"

// memTree is an in-memory Tree.
type memTree struct {
	nodes map[string]Node
	data  map[string][]byte
	dirs  map[string]bool
}

func newMemTree() *memTree {
	return &memTree{nodes: map[string]Node{}, data: map[string][]byte{}, dirs: map[string]bool{}}
}

func (t *memTree) add(rel string, n Node, data string) *memTree {
	t.nodes[rel] = n
	t.data[rel] = []byte(data)
	for d := rel; ; {
		i := strings.LastIndexByte(d, '/')
		if i < 0 {
			break
		}
		d = d[:i]
		t.dirs[d] = true
	}
	return t
}

func (t *memTree) file(rel string, data ...string) *memTree {
	return t.add(rel, Node{Kind: KindFile}, strings.Join(data, ""))
}

func (t *memTree) link(rel, target string) *memTree {
	return t.add(rel, Node{Kind: KindLink, Target: target}, "")
}

func (t *memTree) Stat(rel string) Node {
	if rel == "" {
		return Node{Kind: KindDir}
	}
	if n, ok := t.nodes[rel]; ok {
		return n
	}
	if t.dirs[rel] {
		return Node{Kind: KindDir}
	}
	return Node{}
}

func (t *memTree) Head(rel string, n int) ([]byte, error) {
	b, ok := t.data[rel]
	if !ok || t.nodes[rel].Kind != KindFile {
		return nil, errors.New("no such tracked file")
	}
	if len(b) > n {
		b = b[:n]
	}
	return b, nil
}

// memFS is an in-memory post-run disk.
type memFS struct {
	info  map[string]FileInfo
	links map[string]string
	data  map[string][]byte
	errs  map[string]error // a path whose lstat fails with this error
	tree  *memTree         // the post-run disk holds every tracked file too
}

func newMemFS() *memFS {
	return &memFS{info: map[string]FileInfo{}, links: map[string]string{}, data: map[string][]byte{}, errs: map[string]error{}}
}

var oldCtime = time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)

func (f *memFS) dir(p string) *memFS {
	f.info[p] = FileInfo{Kind: KindDir, Ctime: oldCtime}
	return f
}

func (f *memFS) file(p string, data ...string) *memFS {
	f.info[p] = FileInfo{Kind: KindFile, Ctime: oldCtime}
	f.data[p] = []byte(strings.Join(data, ""))
	return f
}

func (f *memFS) bytes(p string, b []byte) *memFS {
	f.info[p] = FileInfo{Kind: KindFile, Ctime: oldCtime}
	f.data[p] = b
	return f
}

func (f *memFS) link(p, target string) *memFS {
	f.info[p] = FileInfo{Kind: KindLink, Ctime: oldCtime}
	f.links[p] = target
	return f
}

func (f *memFS) Lstat(p string) (FileInfo, error) {
	if err, ok := f.errs[p]; ok {
		return FileInfo{}, err
	}
	if i, ok := f.info[p]; ok {
		return i, nil
	}
	if p == "/w" || p == testRoot {
		return FileInfo{Kind: KindDir, Ctime: oldCtime}, nil
	}
	if rel, ok := strings.CutPrefix(p, testRoot+"/"); ok && f.tree != nil {
		if n := f.tree.Stat(rel); n.Kind != KindAbsent {
			return FileInfo{Kind: n.Kind, Ctime: oldCtime}, nil
		}
	}
	return FileInfo{}, fs.ErrNotExist
}

func (f *memFS) Readlink(p string) (string, error) {
	if t, ok := f.links[p]; ok {
		return t, nil
	}
	if rel, ok := strings.CutPrefix(p, testRoot+"/"); ok && f.tree != nil && f.tree.Stat(rel).Kind == KindLink {
		return f.tree.Stat(rel).Target, nil
	}
	return "", errors.New("not a link")
}

func (f *memFS) ReadHead(p string, n int) ([]byte, error) {
	b, ok := f.data[p]
	if !ok {
		return nil, fs.ErrNotExist
	}
	if len(b) > n {
		b = b[:n]
	}
	return b, nil
}

// harness bundles a tree, a disk and a config.
type harness struct {
	t    *testing.T
	tree *memTree
	fs   *memFS
	cfg  Config
}

var jobStart = time.Date(2026, 6, 1, 0, 0, 0, 0, time.UTC)

func newHarness(t *testing.T) *harness {
	t.Helper()
	tree := newMemTree().
		file("code/p/a/README", "a\n").
		file("code/p/a/src/x.go", "package a\n").
		link("code/p/a/lnk", "src/x.go").
		file("code/p/a/nested/n.txt", "n\n").
		file("code/p/b/b.txt", "b\n").
		file("code/p/b/script.sh", "#!/bin/sh\necho b\n").
		file("code/scripts/gen.py", "#!/usr/bin/env python3\n").
		file("go.work", "go 1.26\n")
	h := &harness{t: t, tree: tree, fs: newMemFS()}
	h.fs.tree = tree
	h.cfg = Config{
		Root:     testRoot,
		Packages: NewPackageIndex([]string{"code/p/a", "code/p/a/nested", "code/p/b"}),
		Tree:     tree,
		FS:       h.fs,
		HostRoots: &legacy.HostRoots{
			Roots: []legacy.HostRoot{
				{Path: "/bin", Kind: legacy.RootImage},
				{Path: "/etc", Kind: legacy.RootImage},
				{Path: "/lib64", Kind: legacy.RootImage},
				{Path: "/usr", Kind: legacy.RootImage},
				{Path: "/home/runner/.cargo/registry", Kind: legacy.RootState, Ecosystem: "rust"},
				{Path: "/opt/tc/go/1.26", Kind: legacy.RootTool, Tool: "go"},
				{Path: "/opt/tc/ruby/3.4", Kind: legacy.RootTool, Tool: "ruby"},
				{Path: "/usr/special", Kind: legacy.RootEphemeral},
			},
			Ephemeral: []string{"/dev/fd/*", "/dev/null", "/proc/cpuinfo", "/proc/self/exe", "/sys/devices/system/cpu/**"},
			AbsentOK:  []string{"/home/runner/.config", "/home/runner/.npmrc"},
		},
		Pinned:   map[string]bool{"rust": true},
		Tools:    map[string]bool{"go": true},
		Home:     "/home/runner",
		Tmpdir:   "/tmp/kiln-1",
		JobStart: jobStart,
	}
	return h
}

// image marks paths as present, unmodified image files.
func (h *harness) image(paths ...string) {
	for _, p := range paths {
		h.fs.info[p] = FileInfo{Kind: KindFile, Ctime: oldCtime}
	}
}

// step builds a one-process trace around events.
func step(events ...trace.Event) *trace.Trace {
	for i := range events {
		if events[i].Pid == 0 {
			events[i].Pid = 1
		}
		events[i].Line = i + 1
	}
	return &trace.Trace{
		RootPid: 1,
		Events:  events,
		Procs:   map[int]*trace.Process{1: {Pid: 1, Exit: &trace.Exit{Pid: 1}}},
		Order:   []int{1},
		Exits:   []trace.Exit{{Pid: 1}},
	}
}

// run classifies one step and returns the record.
func (h *harness) run(events ...trace.Event) *RawRecord {
	h.t.Helper()
	return h.runSteps(step(events...))
}

func (h *harness) runSteps(steps ...*trace.Trace) *RawRecord {
	h.t.Helper()
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: steps})
	if err != nil {
		h.t.Fatalf("Classify: %v", err)
	}
	return rec
}

// Event constructors.  Every event runs in the checkout root.

func base(sys string, kind trace.Kind, path string) trace.Event {
	return trace.Event{Syscall: sys, Kind: kind, Cwd: testRoot, Path: path, Result: trace.Result{OK: true}}
}

func fail(e trace.Event, errno string) trace.Event {
	e.Result = trace.Result{Errno: errno}
	return e
}

// open is a successful open whose -y annotation is ret.
func open(path, flags, ret string) trace.Event {
	e := base("openat", trace.KindOpen, path)
	e.Flags, e.ReturnFDPath = flags, ret
	e.Result.Value = 3
	return e
}

// openAbs opens an absolute path that resolves to itself.
func openAbs(path string) trace.Event { return open(path, "O_RDONLY", path) }

func stat(path string) trace.Event { return base("stat", trace.KindMetadata, path) }

func lstat(path string) trace.Event { return base("lstat", trace.KindMetadata, path) }

func write(sys, path string) trace.Event { return base(sys, trace.KindWrite, path) }

func mkdir(path string) trace.Event { return write("mkdir", path) }

func symlink(target, path string) trace.Event {
	e := write("symlink", path)
	e.Target = target
	return e
}

func rename(src, dst string) trace.Event {
	e := write("rename", src)
	e.Path2 = dst
	return e
}

func link(src, dst string) trace.Event {
	e := write("link", src)
	e.Path2 = dst
	return e
}

func getdents(dir string) trace.Event { return base("getdents64", trace.KindListing, dir) }

func execve(path string, argv ...string) trace.Event {
	e := base("execve", trace.KindProcess, path)
	e.Argv = argv
	return e
}

func inet(sys, addr string, port int) trace.Event {
	e := base(sys, trace.KindNetwork, "")
	e.Addr = &trace.Addr{Family: "AF_INET", Address: addr, Port: port}
	return e
}

func unix(sys, path string) trace.Event {
	e := base(sys, trace.KindNetwork, "")
	e.Addr = &trace.Addr{Family: "AF_UNIX", Address: path, Port: -1}
	return e
}

func (h *harness) has(rec *RawRecord, field []string, want ...string) {
	h.t.Helper()
	have := map[string]bool{}
	for _, s := range field {
		have[s] = true
	}
	for _, w := range want {
		if !have[w] {
			h.t.Errorf("missing %q in %q", w, field)
		}
	}
}

// eq compares a slice with the expected members exactly.
func eq(t *testing.T, what string, got []string, want ...string) {
	t.Helper()
	if len(got) != len(want) {
		t.Errorf("%s = %q, want %q", what, got, want)
		return
	}
	for i := range got {
		if got[i] != want[i] {
			t.Errorf("%s = %q, want %q", what, got, want)
			return
		}
	}
}

func hasReason(rec *RawRecord, r string) bool {
	for _, x := range rec.Reasons {
		if x == r {
			return true
		}
	}
	return false
}

func wantReason(t *testing.T, rec *RawRecord, r string) {
	t.Helper()
	if !hasReason(rec, r) {
		t.Errorf("reasons %q lack %q", rec.Reasons, r)
	}
}

func wantNoReasons(t *testing.T, rec *RawRecord) {
	t.Helper()
	if len(rec.Reasons) != 0 {
		t.Errorf("unexpected reasons %q", rec.Reasons)
	}
}

// buildELF builds the head of an ELF image with one PT_INTERP header.
func buildELF(interp string, class elf.Class, order binary.ByteOrder) []byte {
	var buf bytes.Buffer
	ident := [elf.EI_NIDENT]byte{0x7f, 'E', 'L', 'F', byte(class), byte(elf.ELFDATA2LSB), 1}
	if order == binary.BigEndian {
		ident[elf.EI_DATA] = byte(elf.ELFDATA2MSB)
	}
	interpBytes := append([]byte(interp), 0)
	if class == elf.ELFCLASS64 {
		h := elf.Header64{Ident: ident, Type: uint16(elf.ET_EXEC), Machine: uint16(elf.EM_X86_64), Version: 1,
			Phoff: 64, Ehsize: 64, Phentsize: 56, Phnum: 2}
		binary.Write(&buf, order, h)
		binary.Write(&buf, order, elf.Prog64{Type: uint32(elf.PT_LOAD), Flags: 5, Filesz: 4096})
		binary.Write(&buf, order, elf.Prog64{Type: uint32(elf.PT_INTERP), Off: 64 + 2*56, Filesz: uint64(len(interpBytes))})
	} else {
		h := elf.Header32{Ident: ident, Type: uint16(elf.ET_EXEC), Machine: uint16(elf.EM_386), Version: 1,
			Phoff: 52, Ehsize: 52, Phentsize: 32, Phnum: 2}
		binary.Write(&buf, order, h)
		binary.Write(&buf, order, elf.Prog32{Type: uint32(elf.PT_LOAD), Flags: 5, Filesz: 4096})
		binary.Write(&buf, order, elf.Prog32{Type: uint32(elf.PT_INTERP), Off: 52 + 2*32, Filesz: uint32(len(interpBytes))})
	}
	buf.Write(interpBytes)
	return buf.Bytes()
}
