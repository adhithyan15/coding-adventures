package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// D28: the /proc magic-link attack.  /proc/self/cwd, /proc/self/root,
// /proc/<pid>/fd/N, exe and friends are links the kernel follows to wherever
// the process points them; a ".." after one is taken from the TARGET's parent.
// Read lexically, /proc/self/cwd/../../filesystems is the allowed
// /proc/filesystems-style name while the open really reads a file two levels
// above the working directory.  Every case here must fail closed.

func TestProcMagicLinkDotDotIsRefused(t *testing.T) {
	secret := abs("code/p/b/b.txt")
	paths := []string{
		"/proc/self/cwd/../../cpuinfo",
		"/proc/self/cwd/../../filesystems",
		"/proc/self/root/../proc/cpuinfo",
		"/proc/1234/cwd/../cpuinfo",
		"/proc/1234/root/..",
		"/proc/self/fd/3/../../cpuinfo",
		"/proc/self/exe/../../cpuinfo",
		"/proc/self/map_files/0-1/../../cpuinfo",
		"/proc/self/task/5/cwd/../../../cpuinfo",
		"/proc/thread-self/../cpuinfo", // thread-self links to /proc/<pid>/task/<tid>
		"/proc/net/../cpuinfo",         // net links to self/net
		"/proc/self/cwd/..",
		"/dev/fd/..",
		"/dev/fd/3/../cpuinfo",
		"/proc/self/status/../cpuinfo",
	}
	for _, p := range paths {
		for name, ev := range map[string]trace.Event{
			"open":    open(p, "O_RDONLY", secret),
			"stat":    stat(p),
			"lstat":   lstat(p),
			"statx":   base("statx", trace.KindMetadata, p),
			"readlnk": base("readlink", trace.KindMetadata, p),
		} {
			h := newHarness(t)
			rec := h.run(ev)
			if !hasReason(rec, "trace-incomplete:proc-path") {
				t.Errorf("%s %s: reasons %v entries %v, want proc-path", name, p, rec.Reasons, rec.Entries)
			}
		}
		h := newHarness(t)
		wantReason(t, h.run(execve(p, "x")), "trace-incomplete:proc-path")
	}
}

func TestProcMagicLinkViaDirfd(t *testing.T) {
	h := newHarness(t)
	// openat(dirfd -> /proc/self/cwd, "../../cpuinfo")
	e := open("../../cpuinfo", "O_RDONLY", abs("code/p/b/b.txt"))
	e.DirfdPath = "/proc/self/cwd"
	wantReason(t, h.run(e), "trace-incomplete:proc-path")
	// openat(dirfd -> /proc/self, "cwd/../x")
	h = newHarness(t)
	e = open("cwd/../../cpuinfo", "O_RDONLY", abs("code/p/b/b.txt"))
	e.DirfdPath = "/proc/self"
	wantReason(t, h.run(e), "trace-incomplete:proc-path")
}

func TestProcMagicLinkViaRepositorySymlink(t *testing.T) {
	// A link in the repository whose target is a /proc magic link walks into
	// the virtual tree like a literal /proc path does.
	h := newHarness(t)
	h.fs.link(abs("code/p/a/lnk"), "/proc/self/cwd")
	wantReason(t, h.run(stat("code/p/a/lnk/../../x")), "trace-incomplete:proc-path")
}

func TestProcOpenReturningAnotherFileIsClassifiedByItsRealPath(t *testing.T) {
	// An ephemeral name whose descriptor is some OTHER file: the real file is
	// what was read.  Inside the repository that is an entry; outside every
	// root it is a host read.
	h := newHarness(t)
	rec := h.run(open("/proc/cpuinfo", "O_RDONLY", abs("code/p/b/b.txt")))
	eq(t, "entries", rec.Entries, "pkg:code/p/b")
	h = newHarness(t)
	wantReason(t, h.run(open("/proc/cpuinfo", "O_RDONLY", "/var/secret")), "host-read:/var/secret")
	// Another /proc file is refused outright.
	h = newHarness(t)
	wantReason(t, h.run(open("/proc/cpuinfo", "O_RDONLY", "/proc/1/environ")), "trace-incomplete:proc-path")
	// /proc/self/exe opened: the binary is the real file (inside a tool root).
	h = newHarness(t)
	wantNoReasons(t, h.run(open("/proc/self/exe", "O_RDONLY", "/opt/tc/go/1.26/bin/go")))
	h = newHarness(t)
	wantReason(t, h.run(open("/proc/self/exe", "O_RDONLY", "/var/evil")), "host-read:/var/evil")
}

func TestProcNegativeControls(t *testing.T) {
	// Plain ephemeral names, with and without the pid spelled out by -y.
	h := newHarness(t)
	wantNoReasons(t, h.run(openAbs("/proc/cpuinfo"), stat("/proc/cpuinfo"), openAbs("/proc/self/exe")))
	// self vs the pid form strace prints: the same file, not refused.
	h = newHarness(t)
	wantNoReasons(t, h.run(open("/proc/self/exe", "O_RDONLY", "/proc/self/exe")))
	h = newHarness(t)
	h.cfg.HostRoots.Ephemeral = append(h.cfg.HostRoots.Ephemeral, "/proc/self/maps")
	wantNoReasons(t, h.run(open("/proc/self/maps", "O_RDONLY", "/proc/77/maps")))
	// ".." that is provably exact keeps working.
	h = newHarness(t)
	h.fs.link("/etc/l", "/etc/target")
	h.image("/etc/l", "/etc/target")
	wantNoReasons(t, h.run(stat("/proc/../etc/l")))
	h = newHarness(t)
	wantNoReasons(t, h.run(open("/proc/self/../cpuinfo", "O_RDONLY", "/proc/cpuinfo")))
	h = newHarness(t)
	wantNoReasons(t, h.run(open("/proc/42/../cpuinfo", "O_RDONLY", "/proc/cpuinfo")))
	// Everything else under /proc is still proc-path.
	for _, p := range []string{"/proc/mounts", "/proc/self/mountinfo", "/proc/filesystems"} {
		h = newHarness(t)
		wantReason(t, h.run(stat(p)), "trace-incomplete:proc-path")
	}
}

func TestProcSameFile(t *testing.T) {
	for _, c := range []struct {
		abs, ret string
		want     bool
	}{
		{"/proc/cpuinfo", "/proc/cpuinfo", true},
		{"/proc/self/maps", "/proc/77/maps", true},
		{"/proc/self/maps", "/proc/77/environ", false},
		{"/proc/self/maps", "/proc/self/maps", true},
		{"/proc/thread-self/maps", "/proc/77/task/78/maps", true},
		{"/proc/thread-self/maps", "/proc/77/maps", false},
		{"/proc/mounts", "/proc/77/mounts", true},
		{"/proc/net/dev", "/proc/77/net/dev", true},
		{"/proc/net/dev", "/proc/77/net/arp", false},
		{"/proc/mounts", "/proc/77/maps", false},
		{"/proc/cpuinfo", "/proc/1/environ", false},
		{"/proc/self/maps", "/etc/passwd", false},
		{"/proc/self/maps", "/proc/x/maps", false},
	} {
		if got := procSameFile(c.abs, c.ret); got != c.want {
			t.Errorf("procSameFile(%q,%q)=%v want %v", c.abs, c.ret, got, c.want)
		}
	}
}

func TestProcMagicBeyond(t *testing.T) {
	for p, want := range map[string]bool{
		"/proc/self/exe":             false,
		"/proc/self/cwd":             false,
		"/proc/self/cwd/x":           true,
		"/proc/self/root/etc/passwd": true,
		"/proc/self/exe/x":           true,
		"/proc/self/fd/3":            false,
		"/proc/self/fd/3/x":          true,
		"/proc/self/task/5/cwd/x":    true,
		"/proc/self/task/5/maps":     false,
		"/proc/self/map_files/1-2/x": true,
		"/proc/self/ns/mnt":          false,
		"/proc/cpuinfo":              false,
		"/proc/self/mountinfo":       false,
		"/dev/fd/3":                  false,
	} {
		if got := procMagicBeyond(p); got != want {
			t.Errorf("procMagicBeyond(%q)=%v want %v", p, got, want)
		}
	}
}

// The ephemeral list must never let a path THROUGH a magic link pass, even
// when a pattern matches it.
func TestEphemeralPatternCannotCoverThroughAMagicLink(t *testing.T) {
	h := newHarness(t)
	h.cfg.HostRoots.Ephemeral = append(h.cfg.HostRoots.Ephemeral, "/proc/self/**")
	rec := h.run(stat("/proc/self/cwd/anything"))
	if !hasReason(rec, "trace-incomplete:proc-path") {
		t.Fatalf("reasons %v", rec.Reasons)
	}
}

func TestDevFdEphemeralGlobCannotCoverThroughAMagicLink(t *testing.T) {
	// /dev/fd/N/x is /proc/self/fd/N/x: fd N may be a directory anywhere.
	for _, ev := range []trace.Event{
		stat("/dev/fd/3/b.txt"),
		open("/dev/fd/3/b.txt", "O_RDONLY", ""),
	} {
		h := newHarness(t)
		h.cfg.HostRoots.Ephemeral = append(h.cfg.HostRoots.Ephemeral, "/dev/fd/**")
		wantReason(t, h.run(ev), "trace-incomplete:proc-path")
	}
}

func TestProcOpenOfMagicLinkWithoutDashYPath(t *testing.T) {
	for _, p := range []string{
		"/proc/self/cwd/", "/proc/self/cwd", "/proc/self/root", "/proc/self/exe",
		"/proc/self/fd/3", "/proc/self/map_files/1-2", "/proc/thread-self/cwd",
		"/proc/self/task/5/root", "/dev/fd/3",
	} {
		for _, ret := range []string{"", "weird"} {
			h := newHarness(t)
			h.cfg.HostRoots.Ephemeral = append(h.cfg.HostRoots.Ephemeral, "/proc/**", "/dev/fd/**")
			wantReason(t, h.run(open(p, "O_RDONLY", ret)), "trace-incomplete:proc-path")
		}
	}
}
