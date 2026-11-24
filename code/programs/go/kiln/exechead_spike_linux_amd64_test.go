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
// It is GATED: it skips unless KILN_EXECHEAD_SPIKE=1, its name has no "Sandbox"
// in it (so the kiln-sandbox job's `-run Sandbox` never runs it), and it is not in
// ci/assert_sandbox_tests.py.  The manual workflow kiln-exechead-spike.yml runs it.
// It logs facts as "PROBE: exechead <env> ..." lines in four environments:
//
//	host     a re-executed copy of the test binary, unprivileged, no namespace
//	userns   the same, in a fresh user+mount namespace where it is uid 0 with a
//	         full capability set and a tmpfs of its own (what the sandbox's init
//	         has before it drops privileges, minus the seccomp filter)
//	xns      like userns, but the seccomp listener is handed (SCM_RIGHTS) to THIS
//	         process, which is outside that user namespace, as the real
//	         supervisor would be; it then tries /proc/<pid>/mem,
//	         process_vm_readv, /proc/<pid>/root, pidfd_getfd and the confirm
//	sandbox  the REAL sandbox's step: no capabilities, the real seccomp filter,
//	         the fresh tmpfs as $TMPDIR (what a step, and so anything the step
//	         could do for itself, has)
//
// SAFETY.  The probes are written so a hang or a privileged run cannot hurt the
// machine or the job:
//
//   - Every child runs in its own process group, which the parent kills on a
//     deadline the child's scheduler cannot delay.  The children also turn the
//     garbage collector off: the seccomp supervisor is a goroutine, and a GC
//     stop-the-world would wait for the thread stuck in a vfork (waiting for the
//     supervisor) and never finish.
//   - When this process is root or holds CAP_SYS_ADMIN, only the sandbox child
//     runs (a privileged fanotify permission group on a mount would stall every
//     exec on the machine until answered).
//   - Permission events and mount and filesystem marks are tried only with
//     KILN_EXECHEAD_SPIKE_MARKS=1.

import (
	"bufio"
	"bytes"
	"context"
	"debug/elf"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"runtime/debug"
	"sort"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"testing"
	"time"
	"unsafe"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

const (
	spikeCommand       = "sandbox-exechead"
	spikeFexecCommand  = "sandbox-exechead-fexec"
	spikeGateEnv       = "KILN_EXECHEAD_SPIKE"
	spikeMarksEnv      = "KILN_EXECHEAD_SPIKE_MARKS"
	spikeChildDeadline = 45 * time.Second
)

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
	sysSeccomp          = 317
	sysPidfdOpen        = 434
	sysPidfdGetfd       = 438
	resolveInRoot       = 0x10
	atEmptyPath         = 0x1000
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

// clean makes s safe for one log line: no control character survives, so no
// text a step controls (a path, a name, a command) can start a new line.
func clean(s string) string {
	return strings.Map(func(r rune) rune {
		if r < 0x20 || r == 0x7f {
			return '?'
		}
		return r
	}, s)
}

// spikeCapEff is this process's effective capability set (all ones when it
// cannot be read: be careful).
func spikeCapEff() uint64 {
	b, _ := os.ReadFile("/proc/self/status")
	for _, l := range strings.Split(string(b), "\n") {
		if strings.HasPrefix(l, "CapEff:") {
			if v, err := strconv.ParseUint(strings.TrimSpace(strings.TrimPrefix(l, "CapEff:")), 16, 64); err == nil {
				return v
			}
		}
	}
	return ^uint64(0)
}

// spikePrivileged reports whether this process is root or holds CAP_SYS_ADMIN.
func spikePrivileged() bool {
	if os.Geteuid() == 0 {
		return true
	}
	b, _ := os.ReadFile("/proc/self/status")
	for _, l := range strings.Split(string(b), "\n") {
		if strings.HasPrefix(l, "CapEff:") {
			v, err := strconv.ParseUint(strings.TrimSpace(strings.TrimPrefix(l, "CapEff:")), 16, 64)
			return err != nil || v&(1<<21) != 0
		}
	}
	return true // unknown: be careful
}

// ---------------------------------------------------------------- the parent

// spikeRun starts cmd in its own process group and kills the group at the
// deadline, whatever the child's scheduler is doing.
func spikeRun(cmd *exec.Cmd) (stdout, stderr string, err error) {
	var out, errb capBuf
	cmd.Stdout, cmd.Stderr = &out, &errb
	cmd.WaitDelay = 5 * time.Second
	if cmd.SysProcAttr == nil {
		cmd.SysProcAttr = &syscall.SysProcAttr{}
	}
	cmd.SysProcAttr.Setpgid = true
	if err = cmd.Start(); err != nil {
		return "", "", err
	}
	reaped := killGroupAfter(cmd.Process.Pid, spikeChildDeadline)
	err = cmd.Wait()
	if reaped() {
		err = fmt.Errorf("killed at the %v deadline: %v", spikeChildDeadline, err)
	}
	return out.String(), errb.String(), err
}

// killGroupAfter kills the process group of pid after d unless the child was
// reaped first.  The kill and the reaped mark share a lock, so the group is
// never signalled after the leader's pid could have been reused.  The returned
// function marks the child reaped (call it right after Wait) and says whether
// the deadline fired.
func killGroupAfter(pid int, d time.Duration) (reaped func() bool) {
	var mu sync.Mutex
	done, fired := false, false
	t := time.AfterFunc(d, func() {
		mu.Lock()
		defer mu.Unlock()
		if !done {
			fired = true
			syscall.Kill(-pid, syscall.SIGKILL)
		}
	})
	return func() bool {
		mu.Lock()
		defer mu.Unlock()
		done = true
		t.Stop()
		return fired
	}
}

// TestExecHeadSpike runs the probe in the environments and logs what each
// kernel said.  See the file comment.  It skips unless KILN_EXECHEAD_SPIKE=1.
func TestExecHeadSpike(t *testing.T) {
	if os.Getenv(spikeGateEnv) != "1" {
		t.Skip(spikeGateEnv + "=1 runs the exec-head spike (kiln-exechead-spike.yml)")
	}
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	exe, _ = filepath.EvalSymlinks(exe)
	marks := "0"
	if os.Getenv(spikeMarksEnv) == "1" {
		marks = "1"
	}
	var u syscall.Utsname
	if syscall.Uname(&u) == nil {
		var rel []byte
		for _, c := range u.Release {
			if c == 0 {
				break
			}
			rel = append(rel, byte(c))
		}
		t.Logf("PROBE: exechead kernel=%s uid=%d marks=%s", clean(string(rel)), os.Getuid(), marks)
	}
	logLines := func(env, out, errs string, runErr error) {
		n := 0
		for _, l := range strings.Split(out, "\n") {
			if strings.HasPrefix(l, "PROBE: exechead ") {
				t.Log(clean(l))
				n++
			}
		}
		if runErr != nil || n == 0 {
			t.Logf("PROBE: exechead %s harness exit=%q lines=%d stderr=%q", env, fmt.Sprint(runErr), n, truncate(errs, 200))
		}
	}
	childEnv := func(dir string) []string {
		return withCover(t, []string{"PATH=/usr/bin:/bin", "TMPDIR=" + dir})
	}
	nsAttr := func(pidNS bool) *syscall.SysProcAttr {
		flags := uintptr(syscall.CLONE_NEWUSER | syscall.CLONE_NEWNS)
		if pidNS {
			// The sandbox's step has a pid namespace of its own: the supervisor
			// outside it sees different pids from the ones inside.
			flags |= syscall.CLONE_NEWPID
		}
		return &syscall.SysProcAttr{
			Cloneflags:  flags,
			UidMappings: []syscall.SysProcIDMap{{ContainerID: 0, HostID: os.Getuid(), Size: 1}},
			GidMappings: []syscall.SysProcIDMap{{ContainerID: 0, HostID: os.Getgid(), Size: 1}},
		}
	}

	// The sandbox fixture first: its skip check ends the test before anything
	// else runs on a machine that cannot run the sandbox.
	f := newSandboxFixture(t)

	if spikePrivileged() {
		t.Logf("PROBE: exechead host, userns, xns and fdpath children not run: this process is root or holds CAP_SYS_ADMIN")
	} else {
		// host: no namespace.
		dir := t.TempDir()
		cmd := exec.Command(exe, spikeCommand, "host", dir, marks)
		cmd.Env = childEnv(dir)
		o, e, err := spikeRun(cmd)
		logLines("host", o, e, err)

		// userns: uid 0 with every capability of a user namespace of its own.
		dir = t.TempDir()
		cmd = exec.Command(exe, spikeCommand, "userns", dir, marks)
		cmd.Env = childEnv(dir)
		cmd.SysProcAttr = nsAttr(false)
		o, e, err = spikeRun(cmd)
		logLines("userns", o, e, err)

		// xns: the listener handed to this process, outside that namespace.
		spikeCrossNS(t, exe, nsAttr(true), childEnv)

		// fdpath: an inherited socket through strace -f, listener sent back.
		spikeFdPath(t, exe, childEnv)
	}

	// sandbox: the real step, run like sandboxRunTimeout does, but in its own
	// process group so a hang is killed on the parent's clock.
	b, err := os.ReadFile(f.exe)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(f.pkg, "probe-bin"), b, 0o755); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command(f.exe, sandbox.ExecCommand, "--config", f.cfg, "--",
		filepath.Join(f.pkg, "probe-bin"), spikeCommand, "sandbox", "", marks)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"})
	o, e, err := spikeRun(cmd)
	logLines("sandbox", o, e, err)

	// fdpath through the real sandbox root and init (safe when privileged: it is
	// the same unprivileged-step sandbox as above).
	spikeFdPathSandbox(t, f, marks)
}

func truncate(s string, n int) string {
	if len(s) > n {
		return s[:n] + "..."
	}
	return s
}

// spikeRecvListener receives one descriptor over the socket (SCM_RIGHTS), waiting
// at most 20 seconds, with close-on-exec set on it; any extra descriptor sent
// with it is closed.  It returns -1 and the error when none arrived.
func spikeRecvListener(parentEnd int) (int, error) {
	tv := syscall.Timeval{Sec: 20}
	syscall.SetsockoptTimeval(parentEnd, syscall.SOL_SOCKET, syscall.SO_RCVTIMEO, &tv)
	oob := make([]byte, syscall.CmsgSpace(4))
	var p [1]byte
	_, oobn, _, _, err := syscall.Recvmsg(parentEnd, p[:], oob, 0x40000000 /* MSG_CMSG_CLOEXEC */)
	lfd := -1
	if err == nil {
		if msgs, perr := syscall.ParseSocketControlMessage(oob[:oobn]); perr == nil {
			for _, m := range msgs {
				if rs, rerr := syscall.ParseUnixRights(&m); rerr == nil && len(rs) > 0 {
					if lfd < 0 {
						lfd = rs[0]
						rs = rs[1:]
					}
					for _, extra := range rs {
						syscall.Close(extra)
					}
				}
			}
		}
	}
	return lfd, err
}

// spikeFdPath is the descriptor-path probe.  This process makes a socketpair and
// starts `strace -f <test binary> fdpath` (the test binary alone when strace is
// absent) with one end deliberately inherited on descriptor 3 (os/exec dup2s an
// ExtraFiles entry, which clears close-on-exec on the copy).  The child, still
// under strace, installs the notify filter and sends the listener back over that
// socket; this process is then the supervisor for the child's exec of a binary
// it made, ran and deleted, exactly as in the xns case.  Line:
//
//	fdpath strace=<attached|absent> inherited=<ok|errno> listener=<received|errno> confirm=<...>
func spikeFdPath(t *testing.T, exe string, childEnv func(string) []string) {
	say := func(format string, a ...any) {
		t.Log(clean(fmt.Sprintf("PROBE: exechead fdpath "+format, a...)))
	}
	fds, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_STREAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		say("socketpair failed: %s", spikeErr(err))
		return
	}
	parentEnd := fds[0]
	defer syscall.Close(parentEnd)
	childFile := os.NewFile(uintptr(fds[1]), "fdpath-child")
	dir := t.TempDir()
	traceOut := filepath.Join(dir, "fdpath-strace.out")
	straceState := "absent"
	var cmd *exec.Cmd
	if st, lerr := exec.LookPath("strace"); lerr == nil {
		straceState = "attached"
		cmd = exec.Command(st, "-f", "-e", straceProbeSet, "-o", traceOut, exe, spikeCommand, "fdpath", dir, "0")
	} else {
		cmd = exec.Command(exe, spikeCommand, "fdpath", dir, "0")
	}
	cmd.Env = childEnv(dir)
	cmd.ExtraFiles = []*os.File{childFile}
	var out, errb capBuf
	cmd.Stdout, cmd.Stderr = &out, &errb
	cmd.WaitDelay = 5 * time.Second
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	if err := cmd.Start(); err != nil {
		childFile.Close()
		say("start failed: %s", spikeErr(err))
		return
	}
	childFile.Close()
	reaped := killGroupAfter(cmd.Process.Pid, spikeChildDeadline)

	lfd, rerr := spikeRecvListener(parentEnd)
	listener, confirm := "received", "-"
	var sup *notifSup
	supDone := make(chan struct{})
	if lfd < 0 {
		listener = "none-received"
		if rerr != nil {
			listener = spikeErr(rerr)
		}
		close(supDone)
	} else {
		sup = newNotifSup(lfd)
		sup.confirm.Store(true)
		go func() { sup.run(); close(supDone) }()
	}
	werr := cmd.Wait()
	reaped()
	var recs []*execRec
	if sup != nil {
		sup.stop.Store(true)
		<-supDone
		sup.wg.Wait()
		syscall.Close(lfd)
		sup.mu.Lock()
		recs = append(recs, sup.recs...)
		sup.mu.Unlock()
		// The record that matters is the exec of the binary the child made and
		// deleted ("sleep" in its spike directory); fall back to the first pin.
		var pick *execRec
		for _, r := range recs {
			if r.pin == "ok" && strings.HasSuffix(r.path, "/spike/sleep") {
				pick = r
			}
		}
		for _, r := range recs {
			if pick == nil && r.pin == "ok" {
				pick = r
			}
		}
		switch {
		case pick != nil:
			confirm = fmt.Sprintf("%q", orDash(pick.confirm)+" etxtbsy="+orDash(pick.etxtbsy)+" "+orDash(pick.stable))
		case len(recs) == 0:
			confirm = `"no-execve-notification"`
		default:
			confirm = fmt.Sprintf("%q", "no-pin:"+recs[0].pin)
		}
	}
	inherited := "unreported"
	for _, l := range strings.Split(out.String(), "\n") {
		if !strings.HasPrefix(l, "PROBE: exechead ") {
			continue
		}
		t.Log(clean(l))
		if i := strings.Index(l, " fdpath child inherited="); i >= 0 {
			inherited = clean(strings.Fields(l[i+len(" fdpath child inherited="):])[0])
		}
	}
	if sup != nil && len(recs) > 0 {
		say("%s", clean(recLine("outside-supervisor", recs[len(recs)-1])))
	}
	if werr != nil || inherited == "unreported" {
		say("child exit=%q stderr=%q", fmt.Sprint(werr), truncate(errb.String(), 200))
	}
	if straceState == "attached" {
		b, trunc := readTrace(traceOut)
		if bytes.Count(b, []byte("execve(")) == 0 {
			straceState = "absent"
		}
		if trunc {
			say("strace-trace truncated=true limit=%d", maxTrace)
		}
	}
	say("strace=%s inherited=%s listener=%s confirm=%s", straceState, inherited, listener, confirm)
}

// spikeFdPathSandbox asks whether an inherited descriptor can reach the sandbox's
// step through the REAL sandbox-exec root and init, with no change to them.  The
// socket is an ExtraFiles entry of the sandbox-exec command (descriptor 3 of the
// root); the step lists the descriptors it holds.  Static reasons it should not
// arrive are in the line below, from the code (sandbox_linux_amd64.go).
func spikeFdPathSandbox(t *testing.T, f *sandboxFixture, marks string) {
	say := func(format string, a ...any) {
		t.Log(clean(fmt.Sprintf("PROBE: exechead fdpath "+format, a...)))
	}
	say("sandbox-path=%q", "RunExec marks every descriptor of 3 and up close-on-exec (close_range) before the spawn and maps only /dev/null, --stdout-fd, --stderr-fd and the liveness pipe into the init; RunInit marks 3 and up close-on-exec again before the step; CheckOutputFD accepts only a write-only pipe for --stdout-fd/--stderr-fd. A socket cannot be inherited into the step without a production change.")
	fds, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_STREAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		say("sandbox socketpair failed: %s", spikeErr(err))
		return
	}
	defer syscall.Close(fds[0])
	childFile := os.NewFile(uintptr(fds[1]), "fdpath-sandbox")
	defer childFile.Close()
	cmd := exec.Command(f.exe, sandbox.ExecCommand, "--config", f.cfg, "--",
		filepath.Join(f.pkg, "probe-bin"), spikeCommand, "fdpath-sandbox", "", marks)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"})
	cmd.ExtraFiles = []*os.File{childFile}
	o, e, err := spikeRun(cmd)
	n := 0
	for _, l := range strings.Split(o, "\n") {
		if strings.HasPrefix(l, "PROBE: exechead fdpath-sandbox ") {
			t.Log(clean(l))
			n++
		}
	}
	if err != nil || n == 0 {
		say("sandbox harness exit=%q lines=%d stderr=%q", fmt.Sprint(err), n, truncate(clean(e), 200))
	}
}

// spikeCrossNS: a child in a user and mount namespace of its own installs the
// notification filter and sends the listener to this process (outside the
// namespace) over a socketpair.  This process is then the supervisor, and every
// access it makes to the notifying process is a cross-namespace one.
func spikeCrossNS(t *testing.T, exe string, attr *syscall.SysProcAttr, childEnv func(string) []string) {
	say := func(format string, a ...any) {
		t.Log(clean(fmt.Sprintf("PROBE: exechead xns-super "+format, a...)))
	}
	fds, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_STREAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		say("socketpair failed: %s", spikeErr(err))
		return
	}
	parentEnd := fds[0]
	defer syscall.Close(parentEnd)
	childFile := os.NewFile(uintptr(fds[1]), "xns-child")
	dir := t.TempDir()
	cmd := exec.Command(exe, spikeCommand, "xns", dir, "0")
	cmd.Env = childEnv(dir)
	cmd.SysProcAttr = attr
	cmd.ExtraFiles = []*os.File{childFile}
	var out, errb capBuf
	cmd.Stdout, cmd.Stderr = &out, &errb
	cmd.WaitDelay = 5 * time.Second
	cmd.SysProcAttr.Setpgid = true
	if err := cmd.Start(); err != nil {
		childFile.Close()
		say("start failed: %s", spikeErr(err))
		return
	}
	childFile.Close()
	reaped := killGroupAfter(cmd.Process.Pid, spikeChildDeadline)

	lfd, err := spikeRecvListener(parentEnd)
	if lfd < 0 {
		say("no listener fd received (recvmsg=%s)", spikeErr(err))
		cmd.Wait()
		reaped()
		o := out.String()
		for _, l := range strings.Split(o, "\n") {
			if strings.HasPrefix(l, "PROBE: exechead ") {
				t.Log(clean(l))
			}
		}
		say("child stderr=%q", truncate(errb.String(), 200))
		return
	}
	say("listener fd received over SCM_RIGHTS from a child in another user namespace")
	sup := newNotifSup(lfd)
	sup.extra = true
	sup.confirm.Store(true)
	supDone := make(chan struct{})
	go func() { sup.run(); close(supDone) }()
	werr := cmd.Wait()
	reaped()
	sup.stop.Store(true)
	<-supDone
	sup.wg.Wait()
	syscall.Close(lfd)
	for _, l := range strings.Split(out.String(), "\n") {
		if strings.HasPrefix(l, "PROBE: exechead ") {
			t.Log(clean(l))
		}
	}
	if werr != nil {
		say("child exit=%q stderr=%q", fmt.Sprint(werr), truncate(errb.String(), 200))
	}
	sup.mu.Lock()
	recs := append([]*execRec(nil), sup.recs...)
	sup.mu.Unlock()
	shown := 0
	seenPid := map[int]bool{}
	var pids []string
	for _, r := range recs {
		if !seenPid[r.pid] {
			seenPid[r.pid] = true
			pids = append(pids, strconv.Itoa(r.pid))
		}
		if r.pin != "ok" && shown > 0 {
			continue
		}
		shown++
		if shown > 6 {
			break
		}
		say("%s", recLine("outside-supervisor", r))
	}
	// Which pid the outside supervisor saw, and whether a ptrace stop (strace -f)
	// and a seccomp notification coexisted on the same exec (the child's report).
	stState, stLines := "unknown", "?"
	for _, l := range strings.Split(out.String(), "\n") {
		if i := strings.Index(l, " xns strace="); i >= 0 {
			for _, f := range strings.Fields(l[i:]) {
				if v, ok := strings.CutPrefix(f, "strace="); ok {
					stState = v
				}
				if v, ok := strings.CutPrefix(f, "strace-exec-lines="); ok {
					stLines = v
				}
			}
		}
	}
	say("notif-pid=%s strace=%s strace-exec-lines=%s", strings.Join(pids, ","), clean(stState), clean(stLines))
	if len(recs) == 0 {
		say("no execve notification arrived")
	}
}

// ---------------------------------------------------------------- the child

type spikeSay func(format string, args ...any)

// spikeMain is the probe, run as a child of the test (TestMain dispatches
// spikeCommand here).  args: env label, working directory ("" = $TMPDIR),
// "1" to try permission events and mount and filesystem marks.
func spikeMain(args []string, out *os.File) int {
	// The seccomp supervisor is a goroutine and the thread that execs is stuck
	// in a vfork until it answers: a GC stop-the-world would wait for that
	// thread and never finish.  No collection, no stop-the-world.
	debug.SetGCPercent(-1)
	label, dir, marks := "?", "", false
	if len(args) > 0 {
		label = args[0]
	}
	if len(args) > 1 {
		dir = args[1]
	}
	if len(args) > 2 {
		marks = args[2] == "1"
	}
	if dir == "" {
		dir = os.Getenv("TMPDIR")
	}
	var mu sync.Mutex
	say := func(format string, a ...any) {
		mu.Lock()
		defer mu.Unlock()
		fmt.Fprintln(out, clean(fmt.Sprintf("PROBE: exechead "+label+" "+format, a...)))
	}
	// A watchdog for a hang the parent's process-group kill would also catch.
	time.AfterFunc(spikeChildDeadline-5*time.Second, func() { say("watchdog fired, probe abandoned"); os.Exit(3) })

	if label == "userns" || label == "xns" {
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
	if label == "fdpath-sandbox" {
		fdScanChild(say)
		return 0
	}

	work := filepath.Join(dir, "spike")
	if err := os.MkdirAll(work, 0o755); err != nil {
		say("setup mkdir failed: %s", spikeErr(err))
		return 0
	}
	trueReal, _ := filepath.EvalSymlinks("/usr/bin/true")
	sleepReal, _ := filepath.EvalSymlinks("/usr/bin/sleep")
	shReal, _ := filepath.EvalSymlinks("/bin/sh")
	// The copies keep the REAL program names ("true", "sleep").  On a runner image
	// whose coreutils is a multi-call binary (uutils on ubuntu-26.04: /usr/bin/true
	// resolves to one "coreutils" executable that picks the applet from argv[0]),
	// a copy named "bin-sleep" is an unknown applet and exits 1 at once.
	binTrue := filepath.Join(work, "true")
	binSleep := filepath.Join(work, "sleep")
	script := filepath.Join(work, "s.sh")
	for src, dst := range map[string]string{trueReal: binTrue, sleepReal: binSleep} {
		b, err := os.ReadFile(src)
		if err == nil {
			err = os.WriteFile(dst, b, 0o755)
		}
		if err != nil {
			say("setup copy failed: %s", spikeErr(err))
			return 0
		}
	}
	if err := os.WriteFile(script, []byte("#!/bin/sh\nexec /usr/bin/sleep 0.4\n"), 0o755); err != nil {
		say("setup script failed: %s", spikeErr(err))
		return 0
	}

	say("prep true_real=%q sleep_real=%q multicall=%v", trueReal, sleepReal,
		filepath.Base(trueReal) != "true" || filepath.Base(sleepReal) != "sleep")
	if label == "xns" {
		xnsChild(binTrue, binSleep, say)
		return 0
	}
	if label == "fdpath" {
		fdPathChild(binSleep, say)
		return 0
	}

	interp := spikeInterp(binTrue)
	interpReal, _ := filepath.EvalSymlinks(interp)
	say("prep interp=%q resolved=%q sh=%q", interp, interpReal, shReal)

	if label == "host" || label == "userns" {
		spikeWriter(work, say)
	}
	spikeFanotify(work, interpReal, shReal, binTrue, script, marks, say)
	spikeNotif(binTrue, binSleep, script, say)

	// PR_SET_MM_EXE_FILE re-points /proc/<pid>/exe (it would defeat the confirm).
	// It needs CAP_SYS_RESOURCE (and CAP_CHECKPOINT_RESTORE) in the initial user
	// namespace; the step has dropped every capability.  Last, because it would
	// change /proc/self/exe if it ever succeeded.
	if f, err := os.Open("/usr/bin/true"); err == nil {
		_, _, e := syscall.RawSyscall6(syscall.SYS_PRCTL, 35 /*PR_SET_MM*/, 13 /*PR_SET_MM_EXE_FILE*/, f.Fd(), 0, 0, 0)
		say("prctl PR_SET_MM_EXE_FILE=%s", spikeErr(errnoOrNil(e)))
		// The map form carries exe_fd and is checked against the caller's OWN
		// user namespace.  All other fields are zero: a permitted call then fails
		// validation with EINVAL and changes nothing, so EPERM means the
		// capability check failed and EINVAL means it passed.
		var mm struct {
			start, end, startData, endData, startBrk, brk, startStack, argStart, argEnd, envStart, envEnd uint64
			auxv                                                                                          uint64
			auxvSize, exeFd                                                                               uint32
		}
		mm.exeFd = uint32(f.Fd())
		_, _, e = syscall.RawSyscall6(syscall.SYS_PRCTL, 35 /*PR_SET_MM*/, 14 /*PR_SET_MM_MAP*/, uintptr(unsafe.Pointer(&mm)), unsafe.Sizeof(mm), 0, 0)
		say("prctl PR_SET_MM_MAP(zero-fields,exe_fd)=%s (inconclusive: address validation may precede the capability check)", spikeErr(errnoOrNil(e)))
		f.Close()
	}
	// The valid map, last of all: if it ever succeeds this process's /proc/self/exe
	// changes.  This process is a short-lived child of the test, never the runner.
	spikeSetMMMapValid(label, binTrue, say)
	return 0
}

// mmMap is struct prctl_mm_map (include/uapi/linux/prctl.h): eleven addresses,
// the auxv pointer, its size, and the exe descriptor: 104 bytes.
type mmMap struct {
	start, end, startData, endData, startBrk, brk, startStack, argStart, argEnd, envStart, envEnd uint64
	auxv                                                                                          uint64
	auxvSize, exeFd                                                                               uint32
}

// spikeSetMMMapValid calls PR_SET_MM_MAP with this process's OWN values and the
// descriptor of a different regular file as exe_fd, so that a capability-holding
// caller gets past address validation.  Outcomes (KILN04-exec-head-spike.md
// section 7): EPERM means the capability check refused; EBUSY means it passed but
// the kernel will not re-point an exe whose old file is still mapped; EINVAL
// means the fields or the descriptor failed validation (the probe is wrong, not
// an answer); ok means /proc/self/exe now names the other file.
func spikeSetMMMapValid(label, exePath string, say spikeSay) {
	// PR_SET_MM is governed by CAP_SYS_RESOURCE (bit 24).  The userns child holds
	// it in its own namespace on purpose; any other environment that has it
	// (a privileged host child) must not re-point its exe.
	if label != "userns" && spikeCapEff()&(1<<24) != 0 {
		say("prctl PR_SET_MM_MAP(valid,exe_fd)=skipped(cap_sys_resource)")
		return
	}
	var size uint32
	_, _, e := syscall.RawSyscall6(syscall.SYS_PRCTL, 35 /*PR_SET_MM*/, 15 /*PR_SET_MM_MAP_SIZE*/, uintptr(unsafe.Pointer(&size)), 0, 0, 0)
	if e != 0 {
		say("prctl PR_SET_MM_MAP_SIZE=%s", spikeErr(e))
		return
	}
	say("prctl PR_SET_MM_MAP_SIZE=%d sizeof=%d match=%v", size, unsafe.Sizeof(mmMap{}), uintptr(size) == unsafe.Sizeof(mmMap{}))
	if uintptr(size) != unsafe.Sizeof(mmMap{}) {
		say("prctl PR_SET_MM_MAP(valid,exe_fd)=skipped(size-mismatch)")
		return
	}
	statB, err := os.ReadFile("/proc/self/stat")
	if err != nil {
		say("prctl PR_SET_MM_MAP(valid,exe_fd)=skipped(stat:%s)", spikeErr(err))
		return
	}
	// The fields after the last ")" start at proc(5) field 3 (state).
	i := bytes.LastIndexByte(statB, ')')
	if i < 0 {
		say("prctl PR_SET_MM_MAP(valid,exe_fd)=skipped(stat-format)")
		return
	}
	fields := strings.Fields(string(statB[i+1:]))
	field := func(n int) uint64 { // n is proc(5)'s 1-based field number
		if n-3 >= len(fields) {
			return 0
		}
		v, _ := strconv.ParseUint(fields[n-3], 10, 64)
		return v
	}
	auxv, err := os.ReadFile("/proc/self/auxv")
	if err != nil || len(auxv) == 0 {
		say("prctl PR_SET_MM_MAP(valid,exe_fd)=skipped(auxv:%s)", spikeErr(err))
		return
	}
	brk, _, _ := syscall.RawSyscall(syscall.SYS_BRK, 0, 0, 0)
	f, err := os.Open(exePath)
	if err != nil {
		say("prctl PR_SET_MM_MAP(valid,exe_fd)=skipped(open:%s)", spikeErr(err))
		return
	}
	defer f.Close()
	m := mmMap{
		start: field(26), end: field(27), startStack: field(28),
		startData: field(45), endData: field(46), startBrk: field(47), brk: uint64(brk),
		argStart: field(48), argEnd: field(49), envStart: field(50), envEnd: field(51),
		auxv: uint64(uintptr(unsafe.Pointer(&auxv[0]))), auxvSize: uint32(len(auxv)), exeFd: uint32(f.Fd()),
	}
	_, _, e = syscall.RawSyscall6(syscall.SYS_PRCTL, 35 /*PR_SET_MM*/, 14 /*PR_SET_MM_MAP*/, uintptr(unsafe.Pointer(&m)), unsafe.Sizeof(m), 0, 0)
	runtime.KeepAlive(auxv)
	after, rerr := os.Readlink("/proc/self/exe")
	if rerr != nil {
		after = "?" + spikeErr(rerr)
	}
	say("prctl PR_SET_MM_MAP(valid,exe_fd)=%s exe_after=%q", spikeErr(errnoOrNil(e)), after)
}

func errnoOrNil(e syscall.Errno) error {
	if e == 0 {
		return nil
	}
	return e
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

// spikeCmd is exec.Command with a deadline and a bounded wait.
func spikeCmd(path string, args ...string) (*exec.Cmd, func()) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	cmd := exec.CommandContext(ctx, path, args...)
	cmd.WaitDelay = 3 * time.Second
	return cmd, cancel
}

func runQuiet(path string, args ...string) string {
	cmd, cancel := spikeCmd(path, args...)
	defer cancel()
	if err := cmd.Run(); err != nil {
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
// info records), answers permission events, and appends to the sink.  Names
// and paths are quoted: they are text a step controls.
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
			parts = append(parts, "path="+strconv.Quote(p))
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
						parts = append(parts, "name="+strconv.Quote(string(n)))
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

func spikeFanotify(work, interpReal, shReal, binTrue, script string, marks bool, say spikeSay) {
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
	}
	kinds := []fanKind{{"inode", 0, fanEventOnChild}}
	if marks {
		// A permission group blocks every exec under its mark until answered,
		// and mount and filesystem marks reach beyond this probe's files: only
		// on request, and never from a privileged process (the test refuses).
		groups = append(groups, fanGroup{"perm", fanClassContent, fanOpenExecPerm})
		kinds = append(kinds, fanKind{"mount", fanMarkMount, 0}, fanKind{"fs", fanMarkFilesystem, 0})
	}
	// The event descriptors are O_RDONLY|O_CLOEXEC, so none leaks into a
	// concurrently forked child.
	const eventFFlags = syscall.O_RDONLY | syscall.O_CLOEXEC

	var initFail, markFail []string
	initOK := map[string]bool{}
	for _, g := range groups {
		fd, _, e := syscall.RawSyscall(syscall.SYS_FANOTIFY_INIT, g.flags|fanCloexec|fanNonblock, eventFFlags, 0)
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
			fd, _, e := syscall.RawSyscall(syscall.SYS_FANOTIFY_INIT, g.flags|fanCloexec|fanNonblock, eventFFlags, 0)
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
				stopped := false
				for {
					// Looked at on every pass, whether or not the last read
					// returned events.
					if !stopped {
						select {
						case <-stop:
							stopped = true
						default:
						}
					}
					n, _ := syscall.Read(group, buf)
					if n > 0 {
						sink.parseFan(group, buf[:n])
						continue
					}
					if stopped {
						return
					}
					time.Sleep(3 * time.Millisecond)
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
	extra    string
	first    []byte
	pinned   *os.File
	dev, ino uint64
}

type notifSup struct {
	fd      int
	mu      sync.Mutex
	recs    []*execRec
	confirm atomic.Bool
	stop    atomic.Bool
	extra   bool // also try the cross-namespace accesses
	wg      sync.WaitGroup
	memBuf  []byte // preallocated: the supervisor allocates as little as it can
}

func newNotifSup(fd int) *notifSup { return &notifSup{fd: fd, memBuf: make([]byte, 4096)} }

type pollfd struct {
	fd      int32
	events  int16
	revents int16
}

func (s *notifSup) run() {
	var nt seccompNotif
	for !s.stop.Load() {
		p := pollfd{fd: int32(s.fd), events: 1}
		_, _, e := syscall.Syscall(syscall.SYS_POLL, uintptr(unsafe.Pointer(&p)), 1, 50)
		if e != 0 && e != syscall.EINTR {
			return
		}
		if p.revents&1 == 0 {
			if p.revents&(0x8|0x10|0x20) != 0 {
				return // POLLERR, POLLHUP or POLLNVAL without POLLIN: the listener is gone
			}
			continue
		}
		nt = seccompNotif{}
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
	switch nt.Data.Nr {
	case syscall.SYS_EXECVE:
		s.pin(nt, rec, nt.Data.Args[0])
	case 322: // execveat
		s.execveat(nt, rec)
	}
	if s.extra {
		s.crossProbes(nt, rec)
	}
	rec.pinUS = time.Since(start).Microseconds()
	if !s.valid(nt.ID) {
		rec.pin += "(id-invalid)"
	}
	// The record and the wait group are in place BEFORE the answer: once the
	// kernel runs the exec, the case that started it may finish and look.
	confirming := rec.pinned != nil && s.confirm.Load()
	s.mu.Lock()
	s.recs = append(s.recs, rec)
	if confirming {
		s.wg.Add(1)
	}
	s.mu.Unlock()
	resp := seccompResp{ID: nt.ID, Flags: secNotifFlagContinu}
	syscall.Syscall(syscall.SYS_IOCTL, uintptr(s.fd), ioc(3, unsafe.Sizeof(resp), 1), uintptr(unsafe.Pointer(&resp)))
	if confirming {
		go func() { defer s.wg.Done(); s.confirmExec(rec) }()
	} else if rec.pinned != nil {
		rec.pinned.Close()
	}
}

// execveat: fexecve is execveat(fd, "", ..., AT_EMPTY_PATH).  The spike does
// not pin it; it records what the call looked like and whether /proc/<pid>/fd/N
// could be opened, so the design note can say how it would be handled.
func (s *notifSup) execveat(nt *seccompNotif, rec *execRec) {
	path := s.readStr(int(nt.Pid), nt.Data.Args[1])
	flags := nt.Data.Args[4]
	dirfd := int32(nt.Data.Args[0])
	if path == "" || flags&atEmptyPath != 0 {
		fd, err := syscall.Open(fmt.Sprintf("/proc/%d/fd/%d", nt.Pid, dirfd), syscall.O_RDONLY|syscall.O_NONBLOCK|syscall.O_CLOEXEC, 0)
		if err == nil {
			syscall.Close(fd)
		}
		rec.pin = fmt.Sprintf("execveat-empty-path(dirfd=%d flags=%#x open_proc_fd=%s)", dirfd, flags, spikeErr(err))
		return
	}
	rec.path = path
	rec.pin = fmt.Sprintf("execveat-with-path(dirfd=%d flags=%#x)", dirfd, flags)
}

// readStr reads a NUL-terminated string from the stopped process's memory.
func (s *notifSup) readStr(pid int, addr uint64) string {
	mem, err := syscall.Open("/proc/"+strconv.Itoa(pid)+"/mem", syscall.O_RDONLY|syscall.O_CLOEXEC, 0)
	if err != nil {
		return ""
	}
	defer syscall.Close(mem)
	n, _ := syscall.Pread(mem, s.memBuf, int64(addr))
	if n <= 0 {
		return ""
	}
	if i := bytes.IndexByte(s.memBuf[:n], 0); i >= 0 {
		return string(s.memBuf[:i])
	}
	return ""
}

// pin reads the path from the stopped process and opens it, resolved in THAT
// process's root (openat2 RESOLVE_IN_ROOT against /proc/<pid>/root).  The open
// is non-blocking and the result must be a regular file, so a FIFO named as the
// program cannot block the supervisor.
func (s *notifSup) pin(nt *seccompNotif, rec *execRec, addr uint64) {
	mem, err := syscall.Open("/proc/"+strconv.Itoa(int(nt.Pid))+"/mem", syscall.O_RDONLY|syscall.O_CLOEXEC, 0)
	if err != nil {
		rec.pin = "mem:" + spikeErr(err)
		return
	}
	n, _ := syscall.Pread(mem, s.memBuf, int64(addr))
	syscall.Close(mem)
	if n <= 0 {
		rec.pin = "mem:short-read"
		return
	}
	i := bytes.IndexByte(s.memBuf[:n], 0)
	if i < 0 {
		rec.pin = "mem:no-path"
		return
	}
	path := string(s.memBuf[:i])
	if !strings.HasPrefix(path, "/") {
		cwd, err := os.Readlink("/proc/" + strconv.Itoa(int(nt.Pid)) + "/cwd")
		if err != nil {
			rec.pin = "cwd:" + spikeErr(err)
			return
		}
		path = cwd + "/" + path
	}
	rec.path = path
	root, err := syscall.Open("/proc/"+strconv.Itoa(int(nt.Pid))+"/root", 0x200000|syscall.O_DIRECTORY|syscall.O_CLOEXEC, 0)
	if err != nil {
		rec.pin = "root:" + spikeErr(err)
		return
	}
	defer syscall.Close(root)
	pp, _ := syscall.BytePtrFromString(path)
	how := struct{ flags, mode, resolve uint64 }{flags: syscall.O_RDONLY | syscall.O_CLOEXEC | syscall.O_NONBLOCK, resolve: resolveInRoot}
	fd, _, e := syscall.Syscall6(sysOpenat2, uintptr(root), uintptr(unsafe.Pointer(pp)), uintptr(unsafe.Pointer(&how)), unsafe.Sizeof(how), 0, 0)
	if e != 0 {
		rec.pin = "openat2:" + spikeErr(e)
		return
	}
	f := os.NewFile(fd, "pinned")
	var st syscall.Stat_t
	if err := syscall.Fstat(int(fd), &st); err != nil {
		rec.pin = "fstat:" + spikeErr(err)
		f.Close()
		return
	}
	if st.Mode&syscall.S_IFMT != syscall.S_IFREG {
		rec.pin = "not-a-regular-file"
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
	rec.first = head
}

// crossProbes is what the supervisor tries when it is OUTSIDE the notifying
// process's user namespace, besides pin: process_vm_readv for the path, the
// plain /proc/<pid>/root/<path> string, and pidfd_getfd.
func (s *notifSup) crossProbes(nt *seccompNotif, rec *execRec) {
	var parts []string
	if nt.Data.Nr == syscall.SYS_EXECVE {
		buf := make([]byte, 64)
		type iovec struct {
			base uintptr
			len  uint64
		}
		local := [1]iovec{{uintptr(unsafe.Pointer(&buf[0])), uint64(len(buf))}}
		remote := [1]iovec{{uintptr(nt.Data.Args[0]), uint64(len(buf))}}
		n, _, e := syscall.Syscall6(310, // process_vm_readv
			uintptr(nt.Pid), uintptr(unsafe.Pointer(&local[0])), 1,
			uintptr(unsafe.Pointer(&remote[0])), 1, 0)
		if e != 0 {
			parts = append(parts, "process_vm_readv="+spikeErr(e))
		} else {
			parts = append(parts, fmt.Sprintf("process_vm_readv=ok(%dB)", n))
		}
		if rec.path != "" {
			fd, err := syscall.Open("/proc/"+strconv.Itoa(int(nt.Pid))+"/root"+rec.path, syscall.O_RDONLY|syscall.O_NONBLOCK|syscall.O_CLOEXEC, 0)
			if err == nil {
				syscall.Close(fd)
			}
			parts = append(parts, "open_proc_root_string="+spikeErr(err))
		}
	}
	pfd, _, e := syscall.Syscall(sysPidfdOpen, uintptr(nt.Pid), 0, 0)
	if e != 0 {
		parts = append(parts, "pidfd_open="+spikeErr(e))
	} else {
		g, _, e := syscall.Syscall(sysPidfdGetfd, pfd, 0, 0)
		if e == 0 {
			syscall.Close(int(g))
		}
		parts = append(parts, "pidfd_getfd(fd0)="+spikeErr(errnoOrNil(e)))
		syscall.Close(int(pfd))
	}
	rec.extra = strings.Join(parts, " ")
}

// confirmExec waits for /proc/<pid>/exe to be the pinned inode, then (still
// alive) probes ETXTBSY, reads the loader from /proc/<pid>/maps and re-reads
// the head from the pinned descriptor.
func (s *notifSup) confirmExec(rec *execRec) {
	defer rec.pinned.Close()
	exe := "/proc/" + strconv.Itoa(rec.pid) + "/exe"
	start := time.Now()
	for time.Since(start) < 800*time.Millisecond {
		var st syscall.Stat_t
		if err := syscall.Stat(exe, &st); err != nil {
			rec.confirm = "gone:" + spikeErr(err)
			return
		}
		if uint64(st.Dev) == rec.dev && st.Ino == rec.ino {
			rec.confirm = fmt.Sprintf("exe-match-after-%dus", time.Since(start).Microseconds())
			wfd, err := syscall.Open(exe, syscall.O_WRONLY|syscall.O_CLOEXEC|syscall.O_NONBLOCK, 0)
			if err == nil {
				syscall.Close(wfd)
			}
			rec.etxtbsy = spikeErr(err)
			if maps, err := os.ReadFile("/proc/" + strconv.Itoa(rec.pid) + "/maps"); err == nil {
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
			rec.stable = fmt.Sprintf("head_same=%v alive_after_reread=%v", bytes.Equal(head[:n], rec.first), alive)
			return
		}
		time.Sleep(500 * time.Microsecond)
	}
	l, _ := os.Readlink(exe)
	rec.confirm = "exe-mismatch exe=" + strconv.Quote(l)
	rec.stable = "-"
}

// recLine is the PROBE text for one exec notification (every free-text field is
// quoted).
func recLine(where string, r *execRec) string {
	return fmt.Sprintf("notif %s path=%q pin=%q head=%q interp=%q pin_us=%d confirm=%s etxtbsy=%s ld_maps=%q %s %s",
		where, r.path, r.pin, r.head, r.interp, r.pinUS, orDash(r.confirm), orDash(r.etxtbsy), orDash(r.ldMaps), orDash(r.stable), orDash(r.extra))
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
	// The listener must not be inherited by what the step execs: close-on-exec
	// at once, under the fork lock so no concurrent fork sees it without.
	syscall.ForkLock.RLock()
	fd, _, e := syscall.RawSyscall(sysSeccomp, secSetModeFilter, secFilterNewListen, uintptr(unsafe.Pointer(&fprog)))
	runtime.KeepAlive(prog)
	if e != 0 {
		syscall.ForkLock.RUnlock()
		return -1, e
	}
	syscall.CloseOnExec(int(fd))
	syscall.ForkLock.RUnlock()
	return int(fd), nil
}

func timeExecs(path string, n int) int64 {
	start := time.Now()
	for i := 0; i < n; i++ {
		cmd, cancel := spikeCmd(path)
		cmd.Run()
		cancel()
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
		sup := newNotifSup(fd)
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
				cmd, cancel := spikeCmd(binSleep, "0.5")
				defer cancel()
				if cmd.Start() == nil {
					os.Remove(binSleep) // the Go test binary's life: made, run, deleted
					cmd.Wait()
				}
			}},
			{"elf-fast-exit", func() { c, cancel := spikeCmd(binTrue); defer cancel(); c.Run() }},
			{"script", func() { c, cancel := spikeCmd(script); defer cancel(); c.Run() }},
			{"fexecve", func() {
				fx, err := os.Open(binTrue)
				if err != nil {
					return
				}
				defer fx.Close()
				exe, _ := os.Executable()
				c, cancel := spikeCmd(exe, spikeFexecCommand)
				defer cancel()
				c.ExtraFiles = []*os.File{fx}
				var eb capBuf
				c.Stderr = &eb
				c.Run()
				if eb.Len() > 0 {
					say("notif fexecve helper stderr=%q", truncate(eb.String(), 120))
				}
			}},
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
				interesting := r.pin == "ok" || strings.HasPrefix(r.pin, "execveat")
				if !interesting && shown > 0 {
					continue
				}
				shown++
				if shown > 3 {
					break
				}
				say("%s", recLine("case="+c.name, r))
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

// spikeFexec is the helper for the fexecve case: it execs the file open on fd 3
// with execveat(fd, "", AT_EMPTY_PATH).  It returns only if the call failed.
func spikeFexec() int {
	empty, _ := syscall.BytePtrFromString("")
	argv0, _ := syscall.BytePtrFromString("fexec")
	argv := []*byte{argv0, nil}
	envp := []*byte{nil}
	_, _, e := syscall.RawSyscall6(322, 3, uintptr(unsafe.Pointer(empty)), uintptr(unsafe.Pointer(&argv[0])),
		uintptr(unsafe.Pointer(&envp[0])), atEmptyPath, 0)
	fmt.Fprintf(os.Stderr, "execveat=%s", spikeErr(e))
	return 1
}

// ------------------------------------------------------- cross-namespace child

// xnsChild runs in a user and mount namespace of its own.  It installs the
// notification filter, sends the listener to the process on the other end of the
// socketpair (fd 3, outside the namespace), drops every capability as the
// sandbox does, and then execs: a binary it made, runs and deletes, and one that
// exits at once.  The supervisor's findings are logged by the other side.
func xnsChild(binTrue, binSleep string, say spikeSay) {
	// A second copy of sleep for the strace case (the first is deleted while it runs).
	// It sits in a directory of its own so that it can keep the name "sleep".
	sleep2 := filepath.Join(filepath.Dir(binSleep), "s2", "sleep")
	if err := os.MkdirAll(filepath.Dir(sleep2), 0o755); err != nil {
		say("xns setup s2 failed: %s", spikeErr(err))
	} else if b, err := os.ReadFile(binSleep); err == nil {
		os.WriteFile(sleep2, b, 0o755)
	}
	done := make(chan struct{})
	go func() {
		defer close(done)
		runtime.LockOSThread()
		fd, err := installNotifFilter()
		if err != nil {
			say("xns NEW_LISTENER refused: %s", spikeErr(err))
			return
		}
		rights := syscall.UnixRights(fd)
		if err := syscall.Sendmsg(3, []byte{0}, rights, nil, 0); err != nil {
			say("xns sendmsg of the listener failed: %s", spikeErr(err))
			return
		}
		say("xns listener sent; pid inside the namespace=%d", os.Getpid())
		// Drop all capabilities, as the sandbox's init does before the step.
		hdr := struct {
			version uint32
			pid     int32
		}{0x20080522, 0}
		var data [2]struct{ eff, perm, inh uint32 }
		_, _, e := syscall.RawSyscall(syscall.SYS_CAPSET, uintptr(unsafe.Pointer(&hdr)), uintptr(unsafe.Pointer(&data[0])), 0)
		say("xns capset-empty=%s %s", spikeErr(errnoOrNil(e)), procStatusFacts())
		c, cancel := spikeCmd(binSleep, "0.5")
		defer cancel()
		if c.Start() == nil {
			os.Remove(binSleep)
			c.Wait()
		}
		c2, cancel2 := spikeCmd(binTrue)
		defer cancel2()
		c2.Run()
		// The same, under strace -f: do a ptrace stop and a seccomp
		// notification coexist on one exec?
		if st, err := exec.LookPath("strace"); err != nil {
			say("xns strace=absent strace-exec-lines=0")
		} else {
			out := filepath.Join(filepath.Dir(binSleep), "strace.out")
			c3, cancel3 := spikeCmd(st, "-f", "-e", straceProbeSet, "-o", out, sleep2, "0.3")
			defer cancel3()
			var se capBuf
			c3.Stderr = &se
			runErr := c3.Run()
			b, trunc := readTrace(out)
			n := bytes.Count(b, []byte("execve("))
			state := "absent"
			if n > 0 {
				state = "attached"
			}
			// strace exits with its tracee's status, so a non-zero run is the
			// TRACEE's unless strace_stderr says strace itself failed (its own
			// diagnostics start "strace:").  The tracee's status is read from the
			// trace: the last "+++ exited with N +++" or "+++ killed by SIG +++".
			say("xns strace=%s strace-exec-lines=%d run=%s strace_stderr=%q tracee_exit=%s truncated=%v",
				state, n, spikeErr(runErr), truncate(clean(se.String()), 512), straceTraceeExit(b), trunc)
			// The first exec and the first write to descriptor 2 in the trace say
			// what the tracee was and what it complained about.
			say("xns strace-trace first_execve=%q first_stderr_write=%q", straceLine(b, "execve("), straceLine(b, "write(2,"))
		}
		time.Sleep(100 * time.Millisecond)
	}()
	<-done
}

// straceTraceeExit is the last exit or kill line of a strace -f -o file: the
// first process in the file is the tracee, and the LAST such line is its own
// when it spawns nothing.  "none" if the trace has no exit line.
func straceTraceeExit(trace []byte) string {
	last := ""
	traceLines(trace, func(l string) bool {
		if i := strings.Index(l, "+++ "); i >= 0 && strings.HasSuffix(strings.TrimSpace(l), "+++") {
			last = strings.TrimSpace(strings.Trim(strings.TrimSpace(l[i:]), "+"))
		}
		return true
	})
	if last == "" {
		return "none"
	}
	return strconv.Quote(clean(last))
}

// straceLine is the first trace line containing needle, cut short ("" if none).
func straceLine(trace []byte, needle string) string {
	found := ""
	traceLines(trace, func(l string) bool {
		if strings.Contains(l, needle) {
			found = truncate(clean(l), 200)
			return false
		}
		return true
	})
	return found
}

// straceProbeSet is the narrowest -e trace set the probes read: the exec lines
// (counted, and the first one shown), the writes to descriptor 2 (the first
// one shown).  The "+++ exited with N +++" lines are printed whatever the set.
const straceProbeSet = "trace=execve,execveat,exit_group,write"

// maxTrace bounds how much of a strace output file, a stderr or a child's
// stdout the probes keep.
const (
	maxTrace = 4 << 20
	maxCap   = 64 << 10
)

// readTrace reads at most maxTrace bytes of path; truncated says the file was
// longer.
func readTrace(path string) (b []byte, truncated bool) {
	f, err := os.Open(path)
	if err != nil {
		return nil, false
	}
	defer f.Close()
	b, _ = io.ReadAll(io.LimitReader(f, maxTrace+1))
	if len(b) > maxTrace {
		return b[:maxTrace], true
	}
	return b, false
}

// traceLines calls fn for each line of trace (a line longer than 64 KiB ends the
// scan) until fn returns false.
func traceLines(trace []byte, fn func(string) bool) {
	sc := bufio.NewScanner(bytes.NewReader(trace))
	sc.Buffer(make([]byte, 0, 4096), 64<<10)
	for sc.Scan() {
		if !fn(sc.Text()) {
			return
		}
	}
}

// capBuf is a writer that keeps the first maxCap bytes and discards the rest,
// reporting every write as complete so the child is never blocked or failed.
type capBuf struct{ b bytes.Buffer }

func (c *capBuf) Write(p []byte) (int, error) {
	if room := maxCap - c.b.Len(); room > 0 {
		c.b.Write(p[:min(room, len(p))])
	}
	return len(p), nil
}
func (c *capBuf) String() string { return c.b.String() }
func (c *capBuf) Len() int       { return c.b.Len() }

// fdPathChild runs under `strace -f` (or alone when strace is absent) with an
// inherited socket on descriptor 3.  It reports whether the descriptor arrived,
// installs the notification filter, sends the listener back over the inherited
// socket, and execs a program for the supervisor in the test process to pin.
func fdPathChild(binSleep string, say spikeSay) {
	var st syscall.Stat_t
	inherited := "ok"
	if err := syscall.Fstat(3, &st); err != nil {
		inherited = spikeErr(err)
	} else if st.Mode&syscall.S_IFMT != syscall.S_IFSOCK {
		inherited = "not-a-socket"
	}
	say("child inherited=%s", inherited)
	if inherited != "ok" {
		return
	}
	done := make(chan struct{})
	go func() {
		defer close(done)
		runtime.LockOSThread()
		fd, err := installNotifFilter()
		if err != nil {
			say("child NEW_LISTENER refused: %s", spikeErr(err))
			return
		}
		if err := syscall.Sendmsg(3, []byte{0}, syscall.UnixRights(fd), nil, 0); err != nil {
			say("child listener-sent=%s", spikeErr(err))
			return
		}
		say("child listener-sent=ok")
		c, cancel := spikeCmd(binSleep, "0.5")
		defer cancel()
		if c.Start() == nil {
			os.Remove(binSleep) // made, run, deleted
			c.Wait()
		}
		time.Sleep(100 * time.Millisecond)
	}()
	<-done
}

// fdScanChild runs as the sandbox's STEP, with a socket the test process opened
// before sandbox-exec: which descriptors of 3 and up are open, and is any a
// socket?  The root and the init mark every descriptor of 3 and up close-on-exec
// and the spawn passes only its own four, so none is expected.
func fdScanChild(say spikeSay) {
	open, socks := 0, 0
	var list []string
	for fd := 3; fd < 64; fd++ {
		var st syscall.Stat_t
		if syscall.Fstat(fd, &st) != nil {
			continue
		}
		open++
		kind := "other"
		if st.Mode&syscall.S_IFMT == syscall.S_IFSOCK {
			socks++
			kind = "socket"
		}
		list = append(list, strconv.Itoa(fd)+":"+kind)
	}
	say("scan inherited=%s open_3_63=%d sockets=%d fds=%q", map[bool]string{true: "ok", false: "bad_file_descriptor"}[socks > 0], open, socks, strings.Join(list, ","))
}

// spikeWriter: does the kernel refuse to exec a file somebody already has open
// for writing?  Case A holds an O_WRONLY descriptor.  Case B opens O_RDWR, maps
// the file MAP_SHARED with PROT_WRITE, and closes the descriptor.  If either exec
// is allowed, a writer that existed BEFORE the exec can change the bytes after
// they were loaded, and a head read from a pinned descriptor is not proof of
// what ran (KILN04-exec-head-spike.md section 3.3).
func spikeWriter(work string, say spikeSay) {
	src, err := filepath.EvalSymlinks("/usr/bin/true")
	if err != nil {
		say("exec-with-writer setup failed: %s", spikeErr(err))
		return
	}
	b, err := os.ReadFile(src)
	if err != nil {
		say("exec-with-writer setup failed: %s", spikeErr(err))
		return
	}
	execErr := func(path string) string {
		cmd, cancel := spikeCmd(path)
		defer cancel()
		err := cmd.Run()
		var en syscall.Errno
		switch {
		case err == nil:
			return "allowed"
		case errors.As(err, &en):
			return spikeErr(en)
		default:
			return "other:" + spikeErr(err)
		}
	}
	// Case A.
	a := filepath.Join(work, "writer-a")
	if err := os.WriteFile(a, b, 0o755); err != nil {
		say("exec-with-writer setup failed: %s", spikeErr(err))
		return
	}
	fdState := "open-failed"
	if f, err := os.OpenFile(a, os.O_WRONLY, 0); err == nil {
		fdState = execErr(a)
		f.Close()
	}
	// Case B.
	m := filepath.Join(work, "writer-b")
	if err := os.WriteFile(m, b, 0o755); err != nil {
		say("exec-with-writer setup failed: %s", spikeErr(err))
		return
	}
	mapState := "map-failed"
	if f, err := os.OpenFile(m, os.O_RDWR, 0); err == nil {
		mem, merr := syscall.Mmap(int(f.Fd()), 0, len(b), syscall.PROT_READ|syscall.PROT_WRITE, syscall.MAP_SHARED)
		f.Close()
		if merr == nil {
			mapState = execErr(m)
			syscall.Munmap(mem)
		} else {
			mapState = "mmap:" + spikeErr(merr)
		}
	}
	say("exec-with-writer fd=%s shared-map=%s", fdState, mapState)
}
