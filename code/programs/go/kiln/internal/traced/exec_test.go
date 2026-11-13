package traced

import (
	"debug/elf"
	"encoding/binary"
	"strings"
	"testing"
	"time"
)

const loader = "/lib64/ld-linux-x86-64.so.2"

// An exec reads the program, its "#!" interpreter, and the ELF loader.
func TestExecOfELFReadsTheLoader(t *testing.T) {
	h := newHarness(t)
	h.fs.bytes("/usr/bin/cc", buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.image("/usr/bin/cc", loader)
	rec := h.run(execve("/usr/bin/cc", "cc"))
	wantNoReasons(t, rec)

	// The loader is read like any file: modified after boot is a host-read.
	h.fs.info[loader] = FileInfo{Kind: KindFile, Ctime: jobStart.Add(time.Hour)}
	rec = h.run(execve("/usr/bin/cc", "cc"))
	eq(t, "reasons", rec.Reasons, "host-read:setup-modified:"+loader)

	// 32-bit and big-endian images parse too.
	h.fs.info[loader] = FileInfo{Kind: KindFile, Ctime: oldCtime}
	for _, b := range [][]byte{
		buildELF(loader, elf.ELFCLASS32, binary.LittleEndian),
		buildELF(loader, elf.ELFCLASS64, binary.BigEndian),
		buildELF(loader, elf.ELFCLASS32, binary.BigEndian),
	} {
		h.fs.bytes("/usr/bin/cc", b)
		h.fs.info["/usr/bin/cc"] = FileInfo{Kind: KindFile, Ctime: oldCtime}
		wantNoReasons(t, h.run(execve("/usr/bin/cc", "cc")))
	}
	// An image whose loader is not an image file at all.
	h.fs.bytes("/usr/bin/cc", buildELF("/opt/loader/ld.so", elf.ELFCLASS64, binary.LittleEndian))
	wantReason(t, h.run(execve("/usr/bin/cc", "cc")), "host-read:/opt/loader/ld.so")
}

func TestExecOfStaticELFHasNoLoader(t *testing.T) {
	h := newHarness(t)
	// A header with no PT_INTERP.
	b := buildELF(loader, elf.ELFCLASS64, binary.LittleEndian)
	binary.LittleEndian.PutUint32(b[64+56:], uint32(elf.PT_NOTE)) // second program header
	h.fs.bytes("/usr/bin/static", b)
	h.image("/usr/bin/static")
	wantNoReasons(t, h.run(execve("/usr/bin/static", "static")))
}

func TestExecOfMalformedELFFailsClosed(t *testing.T) {
	h := newHarness(t)
	good := buildELF(loader, elf.ELFCLASS64, binary.LittleEndian)
	h.image("/usr/bin/x")
	mut := map[string]func([]byte) []byte{
		"truncated header": func(b []byte) []byte { return b[:30] },
		"tiny":             func(b []byte) []byte { return b[:8] },
		"bad data":         func(b []byte) []byte { b[elf.EI_DATA] = 9; return b },
		"bad class":        func(b []byte) []byte { b[elf.EI_CLASS] = 9; return b },
		"phdr past end":    func(b []byte) []byte { binary.LittleEndian.PutUint64(b[32:], 1<<20); return b },
		"interp past end":  func(b []byte) []byte { return b[:64+2*56+3] },
		"interp relative": func(b []byte) []byte {
			b[64+2*56] = 'x'
			return b
		},
		"interp empty": func(b []byte) []byte { b[64+2*56] = 0; return b },
		"interp zero size": func(b []byte) []byte {
			binary.LittleEndian.PutUint64(b[64+56+32:], 0)
			return b
		},
	}
	for name, f := range mut {
		b := f(append([]byte(nil), good...))
		h.fs.bytes("/usr/bin/x", b)
		h.fs.info["/usr/bin/x"] = FileInfo{Kind: KindFile, Ctime: oldCtime}
		rec := h.run(execve("/usr/bin/x", "x"))
		if !hasReason(rec, "trace-incomplete:exec-elf") {
			t.Errorf("%s: %q", name, rec.Reasons)
		}
	}
	// A 32-bit header truncated the same way.
	b32 := buildELF(loader, elf.ELFCLASS32, binary.LittleEndian)
	for _, cut := range []int{20, 52 + 2*32 + 3} {
		h.fs.bytes("/usr/bin/x", b32[:cut])
		wantReason(t, h.run(execve("/usr/bin/x", "x")), "trace-incomplete:exec-elf")
	}
	// A program header table that runs off the end of the head.
	b := append([]byte(nil), b32...)
	binary.LittleEndian.PutUint32(b[28:], 52+32+8) // phoff: second entry would straddle the end
	b = b[:52+32+8+20]
	h.fs.bytes("/usr/bin/x", b)
	wantReason(t, h.run(execve("/usr/bin/x", "x")), "trace-incomplete:exec-elf")
}

func TestShebangInterpreterInAnotherPackage(t *testing.T) {
	h := newHarness(t)
	// code/scripts/gen.py is tracked (outside every package) and starts with
	// "#!/usr/bin/env python3": env is read, and env is an ELF.
	h.fs.bytes("/usr/bin/env", buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.image("/usr/bin/env", loader)
	rec := h.run(execve("code/scripts/gen.py", "gen.py"))
	eq(t, "entries", rec.Entries, "path:code/scripts/gen.py")
	wantNoReasons(t, rec)

	// A script whose interpreter is a tracked script in ANOTHER package: the
	// package is read, and so is that script's own interpreter.
	h.tree.file("code/p/b/interp.sh", "#!/bin/sh\n")
	h.tree.file("code/p/a/run.sh", "#!/w/repo/code/p/b/interp.sh -x\n")
	h.fs.bytes("/bin/sh", buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.image("/bin/sh")
	rec = h.run(execve("code/p/a/run.sh", "run.sh"))
	eq(t, "entries", rec.Entries, "pkg:code/p/a", "pkg:code/p/b")
	wantNoReasons(t, rec)
}

func TestShebangShapes(t *testing.T) {
	for in, want := range map[string]string{
		"#!/bin/sh\necho":           "/bin/sh",
		"#!  /bin/sh -e\n":          "/bin/sh",
		"#!\t/usr/bin/env python\n": "/usr/bin/env",
		"#!/bin/sh\r\n":             "/bin/sh",
		"#!./rel/interp":            "./rel/interp",
	} {
		got, ok := shebangInterpreter([]byte(in))
		if !ok || got != want {
			t.Errorf("%q: %q %v, want %q", in, got, ok, want)
		}
	}
	for _, in := range []string{"#!\n", "#!   \n", "#! \x00/bin/sh", "#!"} {
		if _, ok := shebangInterpreter([]byte(in)); ok {
			t.Errorf("%q should have no interpreter", in)
		}
	}
	// Only 255 bytes of the line count.
	long := "#!/" + strings.Repeat("a", 300)
	got, _ := shebangInterpreter([]byte(long))
	if len(got) != 253 {
		t.Errorf("interpreter %d bytes", len(got))
	}
}

func TestShebangDepth(t *testing.T) {
	h := newHarness(t)
	// s0 -> s1 -> s2 -> s3 -> s4 (an ELF): four hops, the kernel's limit.
	for i := 0; i < 4; i++ {
		next := "/w/repo/code/p/b/s" + string(rune('1'+i))
		h.tree.file("code/p/b/s"+string(rune('0'+i)), "#!"+next+"\n")
	}
	h.fs.bytes(abs("code/p/b/s4"), buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.tree.file("code/p/b/s4", "\x7fELF")
	h.tree.data["code/p/b/s4"] = buildELF(loader, elf.ELFCLASS64, binary.LittleEndian)
	h.image(loader)
	rec := h.run(execve("code/p/b/s0", "s0"))
	wantNoReasons(t, rec)
	// One more hop is too many.
	h.tree.file("code/p/b/s4", "#!/w/repo/code/p/b/s5\n")
	h.tree.file("code/p/b/s5", "\x7fELF")
	rec = h.run(execve("code/p/b/s0", "s0"))
	wantReason(t, rec, "trace-incomplete:exec-depth")
}

func TestExecFailsClosed(t *testing.T) {
	h := newHarness(t)
	// A file the classifier cannot read.
	wantReason(t, h.run(execve("/usr/bin/ghost", "g")), "trace-incomplete:exec-unreadable")
	// A format nobody here understands.
	h.fs.file("/usr/bin/blob", "MZ....")
	h.image("/usr/bin/blob")
	wantReason(t, h.run(execve("/usr/bin/blob", "b")), "trace-incomplete:exec-format")
	// A "#!" with no interpreter.
	h.tree.file("code/p/b/bad", "#!\n")
	wantReason(t, h.run(execve("code/p/b/bad", "b")), "trace-incomplete:exec-format")
	// A failed exec is just an observation of the path.
	rec := h.run(fail(execve("/usr/bin/ghost", "g"), "ENOENT"), fail(execve("code/p/b/nothing", "g"), "ENOENT"))
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	wantNoReasons(t, rec) // an absent probe under an image root is fine
	// /proc/self/exe is allowed to be named but cannot be read.
	wantReason(t, h.run(execve("/proc/self/exe", "self")), "trace-incomplete:exec-unreadable")
	// A "#!" pointing into /proc.
	h.tree.file("code/p/b/procsh", "#!/proc/self/exe\n")
	wantReason(t, h.run(execve("code/p/b/procsh", "p")), "trace-incomplete:exec-unreadable")
}

func TestExecOfFileTheActionMade(t *testing.T) {
	h := newHarness(t)
	// A compiled-then-run program (autoconf's conftest) is the action's own,
	// but what the kernel loaded for it is still an input.
	h.fs.bytes(abs("code/p/a/conftest"), buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.image(loader)
	rec := h.run(
		open("code/p/a/conftest", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/a/conftest")),
		execve("code/p/a/conftest", "conftest"),
	)
	eq(t, "entries", rec.Entries)
	wantNoReasons(t, rec)
	// If it is gone afterwards and was never tracked, nothing can say what its
	// loader was.
	delete(h.fs.data, abs("code/p/a/conftest"))
	delete(h.fs.info, abs("code/p/a/conftest"))
	wantReason(t, h.run(
		open("code/p/a/conftest", "O_WRONLY|O_CREAT|O_TRUNC", abs("code/p/a/conftest")),
		execve("code/p/a/conftest", "conftest"),
	), "trace-incomplete:exec-unreadable")
}

func TestExecHeadFromTheTreeBeatsTheDisk(t *testing.T) {
	h := newHarness(t)
	// The BUILD rewrote the script on disk; the tree says what was tracked.
	h.fs.file(abs("code/p/b/script.sh"), "#!/evil/interp\n")
	h.fs.bytes("/bin/sh", buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.image("/bin/sh", loader)
	rec := h.run(execve("code/p/b/script.sh", "script.sh"))
	wantNoReasons(t, rec)
	// A tracked path whose bytes cannot be read from the tree falls back to the disk.
	h.tree.nodes["code/p/b/unreadable"] = Node{Kind: KindFile}
	h.fs.bytes(abs("code/p/b/unreadable"), buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	wantNoReasons(t, h.run(execve("code/p/b/unreadable", "u")))
}

func TestExecatWithEmptyPath(t *testing.T) {
	h := newHarness(t)
	h.fs.bytes("/usr/bin/cc", buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	h.image("/usr/bin/cc", loader)
	// fexecve: the parser makes the descriptor's -y path the path.
	e := execve("/usr/bin/cc", "cc")
	e.Syscall = "execveat"
	wantNoReasons(t, h.run(e))
}

func TestHeadLimit(t *testing.T) {
	if headLimit < 4096 {
		t.Fatal("headLimit must cover a program header table")
	}
}
