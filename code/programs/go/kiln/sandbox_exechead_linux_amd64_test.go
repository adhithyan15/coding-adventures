//go:build linux && amd64

package main

// KILN04 exec-head SPIKE (code/specs/KILN04-exec-head-spike.md).
//
// Problem: the kernel opens an exec'd binary's ELF interpreter (PT_INTERP) and a
// "#!" interpreter without any system call strace can see, so the classifier
// reads the binary's head from the disk AFTER the step.  A binary the step made
// under the fresh tmpfs and then deleted (Go's `$TMPDIR/go-build*/b001/pkg.test`)
// has no head left, and the action is refused as
// trace-incomplete:exec-unreadable:under-tmp.
//
// Two mechanisms could record the head (or the loader chain) at exec time:
//
//	A. fanotify FAN_OPEN_EXEC / FAN_OPEN_EXEC_PERM events, which the kernel
//	   raises for every file it opens for execution, the interpreter included.
//	B. seccomp user notification on execve/execveat: a supervisor pins the file
//	   (opens it) before the kernel runs it, then confirms through
//	   /proc/<pid>/exe that the pinned inode is what runs.
//
// This test ANSWERS EMPIRICAL QUESTIONS; it asserts nothing about the mechanisms.
// It runs the same probe in three environments and logs facts as
// "PROBE: exechead <env> ..." lines:
//
//	host     a re-executed copy of the test binary, unprivileged, no namespace
//	userns   the same, in a fresh user+mount namespace where it is uid 0 with a
//	         full capability set and a tmpfs of its own (what the sandbox's init
//	         has before it drops privileges, minus the seccomp filter)
//	sandbox  the REAL sandbox's step: no capabilities, the real seccomp filter,
//	         the fresh tmpfs as $TMPDIR (what a step, and so anything the step
//	         could do for itself, has)
//
// A mechanism the kernel or the filter refuses is a FACT, logged and skipped; the
// test fails only if the harness itself cannot run (the sandbox fixture is
// skipped or failed exactly like every other sandbox test).  The facts are
// read from the CI log; ci/assert_sandbox_tests.py prints the PROBE lines of a
// passing test.

import (
	"bytes"
	"context"
	"debug/elf"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"testing"
	"time"
	"unsafe"
)

const spikeCommand = "sandbox-exechead"

// fanotify constants (include/uapi/linux/fanotify.h).
const (
	fanCloexec  = 0x1
	fanNonblock = 0x2

	fanClassNotif   = 0x0
	fanClassContent = 0x4

	fanReportFid     = 0x200
	fanReportDirFid  = 0x400
	fanReportName    = 0x800
	fanReportDfidNam = fanReportDirFid | fanReportName

	fanMarkAdd        = 0x1
	fanMarkMount      = 0x10
	fanMarkFilesystem = 0x100

	fanOpenExec     = 0x1000
	fanOpenExecPerm = 0x40000
	fanEventOnChild = 0x08000000

	fanAllow = 1
)

// seccomp user-notification constants (include/uapi/linux/seccomp.h).
const (
	secSetModeFilter    = 1
	secFilterNewListen  = 1 << 3
	secRetUserNotif     = 0x7fc00000
	secRetAllow         = 0x7fff0000
	secNotifFlagContinu = 1
	auditArchX8664      = 0xc000003e
	sysOpenat2          = 437
	resolveInRoot       = 0x10
	sysSeccomp          = 317
)

func spikeErr(e error) string {
	if e == nil {
		return "ok"
	}
	if en, ok := e.(syscall.Errno); ok {
		return strings.ReplaceAll(en.Error(), " ", "_")
	}
	return strings.ReplaceAll(e.Error(), " ", "_")
}

func ioc(dir, size, nr uintptr) uintptr { return dir<<30 | size<<16 | '!'<<8 | nr }

// ---------------------------------------------------------------- the parent

// TestSandboxExecHeadSpike runs the probe in the three environments and logs
// what each kernel said.  See the file comment.
func TestSandboxExecHeadSpike(t *testing.T) {
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	exe, _ = filepath.EvalSymlinks(exe)
	var u syscall.Utsname
	if syscall.Uname(&u) == nil {
		var rel []byte
		for _, c := range u.Release {
			if c == 0 {
				break
			}
			rel = append(rel, byte(c))
		}
		t.Logf("PROBE: exechead kernel=%s uid=%d", rel, os.Getuid())
	}
	logLines := func(env, out, errs string, runErr error) {
		n := 0
		for _, l := range strings.Split(out, "\n") {
			if strings.HasPrefix(l, "PROBE: exechead ") {
				t.Log(l)
				n++
			}
		}
		if runErr != nil || n == 0 {
			t.Logf("PROBE: exechead %s harness exit=%v lines=%d stderr=%q", env, runErr, n, truncate(errs, 200))
		}
	}
	runChild := func(env string, attr *syscall.SysProcAttr, dir string, name string) {
		ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
		defer cancel()
		cmd := exec.CommandContext(ctx, name, spikeCommand, env, dir)
		cmd.Env = withCover(t, []string{"PATH=/usr/bin:/bin", "TMPDIR=" + dir})
		cmd.SysProcAttr = attr
		var out, errb bytes.Buffer
		cmd.Stdout, cmd.Stderr = &out, &errb
		err := cmd.Run()
		logLines(env, out.String(), errb.String(), err)
	}

	// host: no namespace.
	runChild("host", nil, t.TempDir(), exe)
	// userns: uid 0 with every capability of a user namespace of its own.
	runChild("userns", &syscall.SysProcAttr{
		Cloneflags:  syscall.CLONE_NEWUSER | syscall.CLONE_NEWNS,
		UidMappings: []syscall.SysProcIDMap{{ContainerID: 0, HostID: os.Getuid(), Size: 1}},
		GidMappings: []syscall.SysProcIDMap{{ContainerID: 0, HostID: os.Getgid(), Size: 1}},
	}, t.TempDir(), exe)

	// sandbox: the real step.
	f := newSandboxFixture(t)
	b, err := os.ReadFile(f.exe)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(f.pkg, "probe-bin"), b, 0o755); err != nil {
		t.Fatal(err)
	}
	r := f.sandboxRunTimeout(t, 90*time.Second, "pkg", filepath.Join(f.pkg, "probe-bin"), spikeCommand, "sandbox")
	var rerr error
	if r.code != 0 {
		rerr = fmt.Errorf("exit %d", r.code)
	}
	logLines("sandbox", r.stdout, r.stderr, rerr)
}

func truncate(s string, n int) string {
	if len(s) > n {
		return s[:n] + "..."
	}
	return s
}

// ---------------------------------------------------------------- the child

type spikeSay func(format string, args ...any)

// spikeMain is the probe, run as a child of the test (TestMain dispatches
// spikeCommand here).  args: env label, working directory ("" = $TMPDIR).
func spikeMain(args []string, out *os.File) int {
	label, dir := "?", ""
	if len(args) > 0 {
		label = args[0]
	}
	if len(args) > 1 {
		dir = args[1]
	}
	if dir == "" {
		dir = os.Getenv("TMPDIR")
	}
	var mu sync.Mutex
	say := func(format string, a ...any) {
		mu.Lock()
		defer mu.Unlock()
		fmt.Fprintf(out, "PROBE: exechead "+label+" "+format+"\n", a...)
	}
	// A watchdog: whatever hangs, the child ends and the parent logs what it had.
	time.AfterFunc(60*time.Second, func() { say("watchdog fired, probe abandoned"); os.Exit(3) })

	if label == "userns" {
		if err := syscall.Mount("", "/", "", syscall.MS_REC|syscall.MS_PRIVATE, ""); err != nil {
			say("setup make-private failed: %s", spikeErr(err))
			return 0
		}
		if err := syscall.Mount("tmpfs", dir, "tmpfs", syscall.MS_NOSUID|syscall.MS_NODEV, "mode=1777"); err != nil {
			say("setup tmpfs mount failed: %s", spikeErr(err))
			return 0
		}
	}
	say("id uid=%d %s", os.Getuid(), procStatusFacts())

	work := filepath.Join(dir, "spike")
	if err := os.MkdirAll(work, 0o755); err != nil {
		say("setup mkdir %s failed: %s", work, spikeErr(err))
		return 0
	}
	trueReal, _ := filepath.EvalSymlinks("/usr/bin/true")
	sleepReal, _ := filepath.EvalSymlinks("/usr/bin/sleep")
	shReal, _ := filepath.EvalSymlinks("/bin/sh")
	binTrue := filepath.Join(work, "bin-true")
	binSleep := filepath.Join(work, "bin-sleep")
	script := filepath.Join(work, "s.sh")
	for src, dst := range map[string]string{trueReal: binTrue, sleepReal: binSleep} {
		b, err := os.ReadFile(src)
		if err == nil {
			err = os.WriteFile(dst, b, 0o755)
		}
		if err != nil {
			say("setup copy %s failed: %s", src, spikeErr(err))
			return 0
		}
	}
	if err := os.WriteFile(script, []byte("#!/bin/sh\nexec /usr/bin/sleep 0.4\n"), 0o755); err != nil {
		say("setup script failed: %s", spikeErr(err))
		return 0
	}
	interp := spikeInterp(binTrue)
	interpReal, _ := filepath.EvalSymlinks(interp)
	say("prep interp=%s resolved=%s sh=%s", interp, interpReal, shReal)

	spikeFanotify(work, binTrue, script, interpReal, shReal, say)
	spikeNotif(binTrue, binSleep, script, say)
	return 0
}

func procStatusFacts() string {
	b, _ := os.ReadFile("/proc/self/status")
	var keep []string
	for _, l := range strings.Split(string(b), "\n") {
		for _, p := range []string{"CapEff:", "NoNewPrivs:", "Seccomp:"} {
			if strings.HasPrefix(l, p) {
				keep = append(keep, strings.ReplaceAll(strings.Join(strings.Fields(l), "="), ":=", "="))
			}
		}
	}
	return strings.Join(keep, " ")
}

// spikeInterp is the PT_INTERP of the ELF at path ("" for none or an error).
func spikeInterp(path string) string {
	f, err := os.Open(path)
	if err != nil {
		return ""
	}
	defer f.Close()
	return spikeInterpOf(f)
}

func spikeInterpOf(r io.ReaderAt) string {
	ef, err := elf.NewFile(r)
	if err != nil {
		return ""
	}
	for _, p := range ef.Progs {
		if p.Type == elf.PT_INTERP {
			b := make([]byte, p.Filesz)
			if _, err := p.ReadAt(b, 0); err != nil && err != io.EOF {
				return ""
			}
			return strings.TrimRight(string(b), "\x00")
		}
	}
	return ""
}

func runQuiet(path string, args ...string) string {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := exec.CommandContext(ctx, path, args...).Run(); err != nil {
		return "err:" + spikeErr(err)
	}
	return "ok"
}

// ------------------------------------------------------------- mechanism A

type fanEvent struct {
	phase string
	mask  uint64
	pid   int32
	name  string
}

type fanSink struct {
	mu     sync.Mutex
	phase  string
	events []fanEvent
}

func le16(b []byte) int    { return int(binary.LittleEndian.Uint16(b)) }
func le32(b []byte) uint32 { return binary.LittleEndian.Uint32(b) }
func le64(b []byte) uint64 { return binary.LittleEndian.Uint64(b) }

// parseFan decodes the events in b (struct fanotify_event_metadata and its
// info records), answers permission events, and appends to the sink.
func (s *fanSink) parseFan(group int, b []byte) {
	for len(b) >= 24 {
		evlen := int(le32(b))
		if evlen < 24 || evlen > len(b) {
			return
		}
		mlen := le16(b[6:])
		ev := fanEvent{mask: le64(b[8:]), pid: int32(le32(b[20:]))}
		fd := int32(le32(b[16:]))
		var parts []string
		if fd >= 0 {
			p, err := os.Readlink(fmt.Sprintf("/proc/self/fd/%d", fd))
			if err != nil {
				p = "?" + spikeErr(err)
			}
			parts = append(parts, "path="+p)
			if ev.mask&fanOpenExecPerm != 0 {
				var resp [8]byte
				binary.LittleEndian.PutUint32(resp[0:], uint32(fd))
				binary.LittleEndian.PutUint32(resp[4:], fanAllow)
				syscall.Write(group, resp[:])
			}
			syscall.Close(int(fd))
		}
		for off := mlen; off+4 <= evlen; {
			typ, l := b[off], le16(b[off+2:])
			if l < 4 || off+l > evlen {
				break
			}
			body := b[off+4 : off+l]
			if (typ == 1 || typ == 2 || typ == 3) && len(body) >= 16 {
				hb := int(le32(body[8:]))
				if hs := 16 + hb; hs <= len(body) {
					h := body[16:hs]
					if len(h) > 8 {
						h = h[:8]
					}
					parts = append(parts, fmt.Sprintf("info%d fid=%s", typ, hex.EncodeToString(h)))
					if typ == 2 {
						n := body[hs:]
						if i := bytes.IndexByte(n, 0); i >= 0 {
							n = n[:i]
						}
						parts = append(parts, "name="+string(n))
					}
				}
			}
			off += l
		}
		ev.name = strings.Join(parts, " ")
		s.mu.Lock()
		ev.phase = s.phase
		s.events = append(s.events, ev)
		s.mu.Unlock()
		b = b[evlen:]
	}
}

func (s *fanSink) setPhase(p string) { s.mu.Lock(); s.phase = p; s.mu.Unlock() }

// summary: for the phase, "n=3 mask=0x1000 pid0=0 [names...]".
func (s *fanSink) summary(phase string) string {
	s.mu.Lock()
	defer s.mu.Unlock()
	n, pid0 := 0, 0
	var mask uint64
	seen := map[string]bool{}
	var names []string
	for _, e := range s.events {
		if e.phase != phase {
			continue
		}
		n++
		mask |= e.mask
		if e.pid == 0 {
			pid0++
		}
		if !seen[e.name] {
			seen[e.name] = true
			names = append(names, e.name)
		}
	}
	sort.Strings(names)
	if len(names) > 4 {
		names = names[:4]
	}
	return fmt.Sprintf("%s{n=%d mask=%#x pid0=%d [%s]}", phase, n, mask, pid0, strings.Join(names, " | "))
}

type fanGroup struct {
	name  string
	flags uintptr
	mask  uint64
}

type fanKind struct {
	name  string
	flag  uintptr
	extra uint64
}

func spikeFanotify(work, binTrue, script, interpReal, shReal string, say spikeSay) {
	targets := []string{work}
	for _, d := range []string{filepath.Dir(interpReal), filepath.Dir(shReal)} {
		dup := false
		for _, t := range targets {
			dup = dup || t == d
		}
		if d != "." && !dup {
			targets = append(targets, d)
		}
	}
	groups := []fanGroup{
		{"notif", fanClassNotif, fanOpenExec},
		{"notif+fid", fanClassNotif | fanReportFid, fanOpenExec},
		{"notif+dfidname", fanClassNotif | fanReportFid | fanReportDfidNam, fanOpenExec},
		{"perm", fanClassContent, fanOpenExecPerm},
	}
	kinds := []fanKind{{"inode", 0, fanEventOnChild}, {"mount", fanMarkMount, 0}, {"fs", fanMarkFilesystem, 0}}

	var initFail, markFail []string
	initOK := map[string]bool{}
	for _, g := range groups {
		fd, _, e := syscall.RawSyscall(syscall.SYS_FANOTIFY_INIT, g.flags|fanCloexec|fanNonblock, 0, 0)
		if e != 0 {
			initFail = append(initFail, g.name+"="+spikeErr(e))
			continue
		}
		syscall.Close(int(fd))
		initOK[g.name] = true
	}
	if len(initFail) > 0 {
		say("fanotify_init refused: %s", strings.Join(initFail, " "))
	}
	if len(initOK) == 0 {
		return
	}
	var okInit []string
	for _, g := range groups {
		if initOK[g.name] {
			okInit = append(okInit, g.name)
		}
	}
	say("fanotify_init accepted: %s", strings.Join(okInit, " "))

	for _, g := range groups {
		if !initOK[g.name] {
			continue
		}
		for _, k := range kinds {
			fd, _, e := syscall.RawSyscall(syscall.SYS_FANOTIFY_INIT, g.flags|fanCloexec|fanNonblock, 0, 0)
			if e != 0 {
				continue
			}
			group := int(fd)
			okMarks := 0
			errs := map[string]bool{}
			for _, tg := range targets {
				p, _ := syscall.BytePtrFromString(tg)
				_, _, e := syscall.RawSyscall6(syscall.SYS_FANOTIFY_MARK, fd, uintptr(fanMarkAdd)|k.flag,
					uintptr(g.mask|k.extra), ^uintptr(99), uintptr(unsafe.Pointer(p)), 0)
				if e == 0 {
					okMarks++
				} else {
					errs[spikeErr(e)] = true
				}
			}
			if okMarks == 0 {
				var el []string
				for e := range errs {
					el = append(el, e)
				}
				sort.Strings(el)
				markFail = append(markFail, g.name+"/"+k.name+"="+strings.Join(el, "|"))
				syscall.Close(group)
				continue
			}
			sink := &fanSink{}
			stop := make(chan struct{})
			done := make(chan struct{})
			go func() {
				defer close(done)
				buf := make([]byte, 64<<10)
				for {
					n, err := syscall.Read(group, buf)
					if n > 0 {
						sink.parseFan(group, buf[:n])
						continue
					}
					_ = err
					select {
					case <-stop:
						return
					default:
						time.Sleep(3 * time.Millisecond)
					}
				}
			}()
			sink.setPhase("bin")
			r1 := runQuiet(binTrue)
			time.Sleep(120 * time.Millisecond)
			sink.setPhase("script")
			r2 := runQuiet(script)
			time.Sleep(120 * time.Millisecond)
			close(stop)
			<-done
			syscall.Close(group)
			say("fan %s/%s marks=%d/%d run=%s,%s %s %s", g.name, k.name, okMarks, len(targets), r1, r2,
				sink.summary("bin"), sink.summary("script"))
		}
	}
	if len(markFail) > 0 {
		say("fanotify_mark refused: %s", strings.Join(markFail, " "))
	}
}

// ------------------------------------------------------------- mechanism B

type seccompData struct {
	Nr   int32
	Arch uint32
	IP   uint64
	Args [6]uint64
}

type seccompNotif struct {
	ID    uint64
	Pid   uint32
	Flags uint32
	Data  seccompData
}

type seccompResp struct {
	ID    uint64
	Val   int64
	Error int32
	Flags uint32
}

type execRec struct {
	pid      int
	path     string
	pin      string
	head     string
	interp   string
	pinUS    int64
	confirm  string
	etxtbsy  string
	ldMaps   string
	stable   string
	pinned   *os.File
	dev, ino uint64
}

type notifSup struct {
	fd      int
	mu      sync.Mutex
	recs    []*execRec
	confirm atomic.Bool
	stop    atomic.Bool
	wg      sync.WaitGroup
}

type pollfd struct {
	fd      int32
	events  int16
	revents int16
}

func (s *notifSup) run() {
	for !s.stop.Load() {
		p := pollfd{fd: int32(s.fd), events: 1}
		n, _, e := syscall.Syscall(syscall.SYS_POLL, uintptr(unsafe.Pointer(&p)), 1, 50)
		if e != 0 && e != syscall.EINTR {
			return
		}
		if int(n) <= 0 || p.revents&1 == 0 {
			continue
		}
		var nt seccompNotif
		_, _, e = syscall.Syscall(syscall.SYS_IOCTL, uintptr(s.fd), ioc(3, unsafe.Sizeof(nt), 0), uintptr(unsafe.Pointer(&nt)))
		if e != 0 {
			continue
		}
		s.handle(&nt)
	}
}

func (s *notifSup) valid(id uint64) bool {
	_, _, e := syscall.Syscall(syscall.SYS_IOCTL, uintptr(s.fd), ioc(1, 8, 2), uintptr(unsafe.Pointer(&id)))
	return e == 0
}

func (s *notifSup) handle(nt *seccompNotif) {
	start := time.Now()
	rec := &execRec{pid: int(nt.Pid), pin: "n/a"}
	if nt.Data.Nr == syscall.SYS_EXECVE {
		s.pin(nt, rec)
	} else {
		rec.pin = "execveat-not-handled"
	}
	rec.pinUS = time.Since(start).Microseconds()
	valid := s.valid(nt.ID)
	resp := seccompResp{ID: nt.ID, Flags: secNotifFlagContinu}
	syscall.Syscall(syscall.SYS_IOCTL, uintptr(s.fd), ioc(3, unsafe.Sizeof(resp), 1), uintptr(unsafe.Pointer(&resp)))
	if !valid {
		rec.pin += "(id-invalid)"
	}
	s.mu.Lock()
	s.recs = append(s.recs, rec)
	s.mu.Unlock()
	if rec.pinned != nil && s.confirm.Load() {
		s.wg.Add(1)
		go func() { defer s.wg.Done(); s.confirmExec(rec) }()
	} else if rec.pinned != nil {
		rec.pinned.Close()
	}
}

// pin reads the path from the stopped process and opens it, resolved in THAT
// process's root (openat2 RESOLVE_IN_ROOT against /proc/<pid>/root).
func (s *notifSup) pin(nt *seccompNotif, rec *execRec) {
	mem, err := os.Open(fmt.Sprintf("/proc/%d/mem", nt.Pid))
	if err != nil {
		rec.pin = "mem:" + spikeErr(err)
		return
	}
	defer mem.Close()
	buf := make([]byte, 4096)
	n, _ := mem.ReadAt(buf, int64(nt.Data.Args[0]))
	i := bytes.IndexByte(buf[:n], 0)
	if i < 0 {
		rec.pin = "mem:no-path"
		return
	}
	path := string(buf[:i])
	if !strings.HasPrefix(path, "/") {
		cwd, err := os.Readlink(fmt.Sprintf("/proc/%d/cwd", nt.Pid))
		if err != nil {
			rec.pin = "cwd:" + spikeErr(err)
			return
		}
		path = cwd + "/" + path
	}
	rec.path = path
	root, err := syscall.Open(fmt.Sprintf("/proc/%d/root", nt.Pid), 0x200000|syscall.O_DIRECTORY|syscall.O_CLOEXEC, 0)
	if err != nil {
		rec.pin = "root:" + spikeErr(err)
		return
	}
	defer syscall.Close(root)
	pp, _ := syscall.BytePtrFromString(path)
	how := struct{ flags, mode, resolve uint64 }{flags: syscall.O_RDONLY | syscall.O_CLOEXEC, resolve: resolveInRoot}
	fd, _, e := syscall.Syscall6(sysOpenat2, uintptr(root), uintptr(unsafe.Pointer(pp)), uintptr(unsafe.Pointer(&how)), unsafe.Sizeof(how), 0, 0)
	if e != 0 {
		rec.pin = "openat2:" + spikeErr(e)
		return
	}
	f := os.NewFile(fd, path)
	var st syscall.Stat_t
	if err := syscall.Fstat(int(fd), &st); err != nil {
		rec.pin = "fstat:" + spikeErr(err)
		f.Close()
		return
	}
	head := make([]byte, 256)
	hn, _ := f.ReadAt(head, 0)
	head = head[:hn]
	rec.pin, rec.pinned, rec.dev, rec.ino = "ok", f, uint64(st.Dev), st.Ino
	switch {
	case bytes.HasPrefix(head, []byte("\x7fELF")):
		rec.head = "elf"
		rec.interp = spikeInterpOf(f)
	case bytes.HasPrefix(head, []byte("#!")):
		line, _, _ := bytes.Cut(head, []byte("\n"))
		rec.head = "shebang:" + string(line)
	default:
		rec.head = "other"
	}
	rec.stable = string(head) // compared with the post-confirm re-read
}

// confirmExec waits for /proc/<pid>/exe to be the pinned inode, then (still
// alive) probes ETXTBSY, reads the loader from /proc/<pid>/maps and re-reads
// the head from the pinned descriptor.
func (s *notifSup) confirmExec(rec *execRec) {
	defer rec.pinned.Close()
	exe := fmt.Sprintf("/proc/%d/exe", rec.pid)
	start := time.Now()
	for time.Since(start) < 800*time.Millisecond {
		var st syscall.Stat_t
		if err := syscall.Stat(exe, &st); err != nil {
			rec.confirm = "gone:" + spikeErr(err)
			return
		}
		if uint64(st.Dev) == rec.dev && st.Ino == rec.ino {
			rec.confirm = fmt.Sprintf("exe-match-after-%dus", time.Since(start).Microseconds())
			wfd, err := syscall.Open(exe, syscall.O_WRONLY, 0)
			if err == nil {
				syscall.Close(wfd)
			}
			rec.etxtbsy = spikeErr(err)
			if maps, err := os.ReadFile(fmt.Sprintf("/proc/%d/maps", rec.pid)); err == nil {
				for _, l := range strings.Split(string(maps), "\n") {
					if i := strings.Index(l, "/"); i >= 0 && (strings.Contains(l, "/ld-") || strings.Contains(l, "ld-linux")) {
						rec.ldMaps = l[i:]
						break
					}
				}
			} else {
				rec.ldMaps = "maps:" + spikeErr(err)
			}
			head := make([]byte, 256)
			n, _ := rec.pinned.ReadAt(head, 0)
			var st2 syscall.Stat_t
			alive := syscall.Stat(exe, &st2) == nil && uint64(st2.Dev) == rec.dev && st2.Ino == rec.ino
			rec.stable = fmt.Sprintf("head_same=%v alive_after_reread=%v", string(head[:n]) == rec.stable, alive)
			return
		}
		time.Sleep(500 * time.Microsecond)
	}
	l, _ := os.Readlink(exe)
	rec.confirm = "exe-mismatch exe=" + l
	rec.stable = "-"
}

func installNotifFilter() (int, error) {
	prog := []struct {
		code   uint16
		jt, jf uint8
		k      uint32
	}{
		{0x20, 0, 0, 4}, // ld arch
		{0x15, 1, 0, auditArchX8664},
		{0x06, 0, 0, secRetAllow},
		{0x20, 0, 0, 0}, // ld nr
		{0x15, 2, 0, uint32(syscall.SYS_EXECVE)},
		{0x15, 1, 0, 322}, // execveat
		{0x06, 0, 0, secRetAllow},
		{0x06, 0, 0, secRetUserNotif},
	}
	fprog := struct {
		length uint16
		filter unsafe.Pointer
	}{uint16(len(prog)), unsafe.Pointer(&prog[0])}
	if _, _, e := syscall.RawSyscall6(syscall.SYS_PRCTL, 38, 1, 0, 0, 0, 0); e != 0 {
		return -1, fmt.Errorf("no_new_privs: %v", spikeErr(e))
	}
	fd, _, e := syscall.RawSyscall(sysSeccomp, secSetModeFilter, secFilterNewListen, uintptr(unsafe.Pointer(&fprog)))
	runtime.KeepAlive(prog)
	if e != 0 {
		return -1, e
	}
	return int(fd), nil
}

func timeExecs(path string, n int) int64 {
	start := time.Now()
	for i := 0; i < n; i++ {
		exec.Command(path).Run()
	}
	return time.Since(start).Microseconds() / int64(n)
}

func spikeNotif(binTrue, binSleep, script string, say spikeSay) {
	const n = 60
	base := timeExecs(binTrue, n)
	done := make(chan struct{})
	go func() {
		defer close(done)
		// The filter is per THREAD (inherited by what the thread creates), so
		// every exec that should be seen starts from this locked thread.  The
		// thread is not unlocked: it ends with the goroutine.
		runtime.LockOSThread()
		fd, err := installNotifFilter()
		if err != nil {
			say("notif NEW_LISTENER refused: %s", spikeErr(err))
			return
		}
		say("notif NEW_LISTENER accepted fd=%d", fd)
		sup := &notifSup{fd: fd}
		go sup.run()
		defer func() { sup.stop.Store(true); time.Sleep(80 * time.Millisecond) }()

		// Cost: the same loop as the baseline, with the supervisor pinning each exec.
		filtered := timeExecs(binTrue, n)
		say("notif perf per-exec baseline_us=%d with_pin_us=%d n=%d", base, filtered, n)

		sup.confirm.Store(true)
		cases := []struct {
			name string
			run  func()
		}{
			{"elf-live-then-deleted", func() {
				cmd := exec.Command(binSleep, "0.5")
				if cmd.Start() == nil {
					os.Remove(binSleep) // the Go test binary's life: made, run, deleted
					cmd.Wait()
				}
			}},
			{"elf-fast-exit", func() { exec.Command(binTrue).Run() }},
			{"script", func() { exec.Command(script).Run() }},
		}
		for _, c := range cases {
			sup.mu.Lock()
			sup.recs = nil
			sup.mu.Unlock()
			c.run()
			sup.wg.Wait()
			sup.mu.Lock()
			recs := append([]*execRec(nil), sup.recs...)
			sup.mu.Unlock()
			shown := 0
			for _, r := range recs {
				if r.pin != "ok" && shown > 0 {
					continue
				}
				shown++
				if shown > 3 {
					break
				}
				say("notif case=%s path=%s pin=%s head=%s interp=%s pin_us=%d confirm=%s etxtbsy=%s ld_maps=%s %s",
					c.name, r.path, r.pin, r.head, r.interp, r.pinUS, orDash(r.confirm), orDash(r.etxtbsy), orDash(r.ldMaps), orDash(r.stable))
			}
			if len(recs) == 0 {
				say("notif case=%s no execve notification arrived", c.name)
			}
		}
	}()
	<-done
}

func orDash(s string) string {
	if s == "" {
		return "-"
	}
	return s
}
