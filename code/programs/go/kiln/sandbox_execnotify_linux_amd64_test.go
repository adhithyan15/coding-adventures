//go:build linux && amd64

package main

// KILN04 D48 step 3, end to end: `kiln sandbox-exec --exec-notify-fd N`.
//
// With the option the sandbox's init installs a SECOND seccomp filter whose
// verdict for execve and execveat is USER_NOTIF, hands the listener to the
// recorder over the named socket (SCM_RIGHTS, one descriptor, one data byte) and
// closes both before it execs the step.  The supervisor is PR 4; here a minimal
// RESPONDER stands in for it: it receives the listener, reads each notification
// and answers CONTINUE, so the kernel runs the exec.
//
// EVERY test that passes the option runs a responder (or deliberately has none
// and says so), because a notified exec with nobody answering waits forever.
// Every child has a deadline and is killed by process group.
//
// What these tests pin, in the order of the PR's brief:
//
//	(a) an exec notifies and CONTINUE lets the step run;
//	(b) the first filter's refusals are unchanged with the option on;
//	(c) a dead listener: the step does NOT run; exit status 125 and a message
//	    that starts "exec-notify:" (documented on TestSandboxExecNotifyDeadListener);
//	(d) the step holds no socket and no listener;
//	(e) a configured but missing, or wrong-kind, descriptor is refused first;
//	(f) a step's own filter that errors execve only loses its execs, and a second
//	    listener is refused (EPERM from the filter; the kernel would say EBUSY);
//	(g) a real strace run with the option parses and cross-checks clean;
//	(h) PR_SET_MM is refused (TestSandboxSeccompRefusals, and the filter unit test);
//	(i) with the option absent there is no second filter.

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"testing"
	"time"
	"unsafe"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

const execNotifyProbeCommand = "sandbox-execnotify-probe"

// ------------------------------------------------------------- the responder

type respMode int

const (
	// respContinue receives the listener and answers every notification with
	// SECCOMP_USER_NOTIF_FLAG_CONTINUE.
	respContinue respMode = iota
	// respDropListener receives the listener and closes it at once: the
	// supervisor that died (or was killed) after the hand-over.
	respDropListener
)

// execResponder is the test's stand-in for PR 4's supervisor.  It owns the
// parent end of the socket pair.
type execResponder struct {
	t    *testing.T
	sock int
	mode respMode

	started  atomic.Bool
	stopFlag atomic.Bool
	done     chan struct{}
	stopOnce sync.Once

	mu   sync.Mutex
	nrs  []int32 // system-call number of each notification, in order
	hand handshake
}

// handshake is what the receive saw: the contract of the hand-over.
type handshake struct {
	ok         bool // a listener arrived
	dataBytes  int
	dataByte   byte
	fds        int // descriptors in the SCM_RIGHTS message
	truncated  bool
	eofAfter   bool // end of file once the sandbox is done: nobody else holds the other end
	recvErrStr string
}

// newExecResponder makes the socket pair (AF_UNIX, SOCK_STREAM, both ends
// close-on-exec) and returns the responder, which owns the parent end, and the
// child end as an *os.File for the caller to pass in ExtraFiles.  The caller
// closes that file after the child has started.
func newExecResponder(t *testing.T, mode respMode) (*execResponder, *os.File) {
	t.Helper()
	pair, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_STREAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		t.Fatalf("socketpair: %v", err)
	}
	r := &execResponder{t: t, sock: pair[0], mode: mode, done: make(chan struct{})}
	t.Cleanup(r.stop)
	return r, os.NewFile(uintptr(pair[1]), "exec-notify-child-end")
}

// begin starts the receiving goroutine.
func (r *execResponder) begin() {
	if r.started.Swap(true) {
		return
	}
	go r.run()
}

// stop ends the responder and closes its descriptors.  It is idempotent and is
// also a t.Cleanup, so no path leaks.
func (r *execResponder) stop() {
	r.stopOnce.Do(func() {
		r.stopFlag.Store(true)
		if r.started.Load() {
			select {
			case <-r.done:
			case <-time.After(10 * time.Second):
				r.t.Errorf("the responder did not stop")
			}
		}
		// End of file is the proof the sandbox closed ITS end of the socket
		// (the init does it before it execs the step, so nothing the step ran
		// could hold it).  A non-blocking read: 0 bytes is EOF; EAGAIN means a
		// holder is still alive.
		if r.hand.ok {
			var p [8]byte
			n, _, _, _, err := syscall.Recvmsg(r.sock, p[:], nil, syscall.MSG_DONTWAIT)
			r.mu.Lock()
			r.hand.eofAfter = err == nil && n == 0
			r.mu.Unlock()
		}
		syscall.Close(r.sock)
	})
}

func (r *execResponder) numbers() []int32 {
	r.mu.Lock()
	defer r.mu.Unlock()
	return append([]int32(nil), r.nrs...)
}

func (r *execResponder) handshake() handshake {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.hand
}

// recvListener waits for the one message and returns the listener descriptor.
func (r *execResponder) recvListener() int {
	tv := syscall.Timeval{Usec: 100000}
	syscall.SetsockoptTimeval(r.sock, syscall.SOL_SOCKET, syscall.SO_RCVTIMEO, &tv)
	deadline := time.Now().Add(60 * time.Second)
	for !r.stopFlag.Load() && time.Now().Before(deadline) {
		// Room for more than the contract allows, so an extra byte or an extra
		// descriptor is SEEN and not silently cut.
		p := make([]byte, 8)
		oob := make([]byte, syscall.CmsgSpace(4*4))
		n, oobn, flags, _, err := syscall.Recvmsg(r.sock, p, oob, 0x40000000 /* MSG_CMSG_CLOEXEC */)
		if err == syscall.EAGAIN || err == syscall.EINTR {
			continue
		}
		r.mu.Lock()
		defer r.mu.Unlock()
		if err != nil {
			r.hand.recvErrStr = err.Error()
			return -1
		}
		r.hand.dataBytes = n
		if n > 0 {
			r.hand.dataByte = p[0]
		}
		r.hand.truncated = flags&0x8 != 0 // MSG_CTRUNC
		listener := -1
		if msgs, perr := syscall.ParseSocketControlMessage(oob[:oobn]); perr == nil {
			for _, m := range msgs {
				rs, rerr := syscall.ParseUnixRights(&m)
				if rerr != nil {
					continue
				}
				for _, fd := range rs {
					r.hand.fds++
					if listener < 0 {
						listener = fd
					} else {
						syscall.Close(fd)
					}
				}
			}
		}
		r.hand.ok = listener >= 0
		return listener
	}
	return -1
}

func (r *execResponder) run() {
	defer close(r.done)
	lfd := r.recvListener()
	if lfd < 0 {
		return
	}
	defer syscall.Close(lfd)
	if r.mode == respDropListener {
		return // the deferred close is the whole point
	}
	for !r.stopFlag.Load() {
		p := pollfd{fd: int32(lfd), events: 1}
		_, _, e := syscall.Syscall(syscall.SYS_POLL, uintptr(unsafe.Pointer(&p)), 1, 50)
		if e != 0 && e != syscall.EINTR {
			return
		}
		if p.revents&1 == 0 {
			if p.revents&(0x8|0x10|0x20) != 0 {
				return // POLLERR, POLLHUP or POLLNVAL: every task of the filter is gone
			}
			continue
		}
		var nt seccompNotif
		if _, _, e = syscall.Syscall(syscall.SYS_IOCTL, uintptr(lfd), ioc(3, unsafe.Sizeof(nt), 0), uintptr(unsafe.Pointer(&nt))); e != 0 {
			continue
		}
		r.mu.Lock()
		r.nrs = append(r.nrs, nt.Data.Nr)
		r.mu.Unlock()
		resp := seccompResp{ID: nt.ID, Flags: secNotifFlagContinu}
		syscall.Syscall(syscall.SYS_IOCTL, uintptr(lfd), ioc(3, unsafe.Sizeof(resp), 1), uintptr(unsafe.Pointer(&resp)))
	}
}

// ------------------------------------------------------------ running a child

type notifyRun struct {
	stdout, stderr string
	code           int
	resp           *execResponder
}

// notifyRunWith runs `kiln sandbox-exec --config cfg --exec-notify-fd 3 -- argv`
// with the responder's socket as the root's descriptor 3.  The child is its own
// process group and the group is killed at the deadline; a deadline is a
// failure that shows what was printed.
func (f *sandboxFixture) notifyRunWith(t *testing.T, mode respMode, argv ...string) notifyRun {
	t.Helper()
	resp, child := newExecResponder(t, mode)
	return f.notifyRunOn(t, resp, child, []string{"--exec-notify-fd", "3"}, argv...)
}

// notifyRunOn is notifyRunWith with the flags chosen by the caller; child, if
// not nil, is the root's descriptor 3.
func (f *sandboxFixture) notifyRunOn(t *testing.T, resp *execResponder, child *os.File, flags []string, argv ...string) notifyRun {
	t.Helper()
	args := append([]string{sandbox.ExecCommand, "--config", f.cfg}, flags...)
	args = append(append(args, "--"), argv...)
	cmd := exec.Command(f.exe, args...)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"})
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	if child != nil {
		cmd.ExtraFiles = []*os.File{child}
	}
	var out, errb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &errb
	if err := cmd.Start(); err != nil {
		if child != nil {
			child.Close()
		}
		t.Fatalf("start: %v", err)
	}
	if child != nil {
		child.Close() // the sandbox's copy is the only one left on that side
	}
	resp.begin()
	waited := make(chan error, 1)
	go func() { waited <- cmd.Wait() }()
	var err error
	select {
	case err = <-waited:
	case <-time.After(60 * time.Second):
		syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		<-waited
		resp.stop()
		t.Fatalf("timed out; stdout %q stderr %q", out.String(), errb.String())
	}
	resp.stop()
	r := notifyRun{stdout: out.String(), stderr: errb.String(), resp: resp}
	if ee, ok := err.(*exec.ExitError); ok {
		r.code = ee.ExitCode()
	} else if err != nil {
		t.Fatalf("%v\nstderr: %s", err, r.stderr)
	}
	return r
}

// stepBinary copies the test binary into the checkout (the step cannot see the
// test's own directory) and returns its path inside the sandbox.
func (f *sandboxFixture) stepBinary(t *testing.T) string {
	t.Helper()
	self, err := os.ReadFile(f.exe)
	if err != nil {
		t.Fatal(err)
	}
	bin := filepath.Join(f.pkg, "probe-bin")
	if err := os.WriteFile(bin, self, 0o755); err != nil {
		t.Fatal(err)
	}
	return bin
}

// ------------------------------------------------------------- the probe step

// notifyProbe runs INSIDE the sandbox, as the step.  Modes:
//
//	info    what the step can see of the exec listener: its seccomp filter count,
//	        its descriptors, and whether it can install a listener of its own
//	filter  the step installs a filter of its own that errors execve
func notifyProbe(mode string, out *os.File) int {
	say := func(format string, a ...any) { fmt.Fprintf(out, format+"\n", a...) }
	switch mode {
	case "info":
		say("filters=%s", seccompFilterCount())
		ents, err := os.ReadDir("/proc/self/fd")
		if err != nil {
			say("fdscan=%v", err)
			return 1
		}
		for _, e := range ents {
			target, _ := os.Readlink("/proc/self/fd/" + e.Name())
			say("fdlink=%s %s", e.Name(), target)
		}
		say("new-listener=%s", tryNewListener())
		return 0
	case "filter", "usernotif":
		runtime.LockOSThread() // a filter is per thread: exec from this one
		// "filter": the step's own filter errors execve with EACCES.
		// "usernotif": it answers USER_NOTIF for execve WITHOUT a listener (the
		// equal-action tie with the supervisor's filter; the newest filter's
		// verdict is kept, and with no listener the kernel fails the call with
		// ENOSYS).
		verdict := uint32(0x00050000 | 13)
		if mode == "usernotif" {
			verdict = 0x7fc00000
		}
		say("exec-before=%s", forkExecTrue())
		say("second-listener=%s", tryNewListener())
		// The step's own filter: execve and execveat fail with EACCES.
		prog := []struct {
			code   uint16
			jt, jf uint8
			k      uint32
		}{
			{0x20, 0, 0, 0},
			{0x15, 2, 0, 59},
			{0x15, 1, 0, 322},
			{0x06, 0, 0, 0x7fff0000},
			{0x06, 0, 0, verdict},
		}
		fprog := struct {
			length uint16
			filter unsafe.Pointer
		}{uint16(len(prog)), unsafe.Pointer(&prog[0])}
		if _, _, e := syscall.RawSyscall6(syscall.SYS_PRCTL, 22, 2, uintptr(unsafe.Pointer(&fprog)), 0, 0, 0); e != 0 {
			say("own-filter=%s", e)
			return 1
		}
		runtime.KeepAlive(prog)
		say("own-filter=ok")
		say("exec-after=%s", forkExecTrue())
		say("filters-after=%s", seccompFilterCount())
		return 0
	}
	say("unknown mode %q", mode)
	return 2
}

// seccompFilterCount reads "Seccomp_filters" of /proc/self/status (Linux 5.9+),
// "n/a" when the kernel does not print it.
func seccompFilterCount() string {
	b, err := os.ReadFile("/proc/self/status")
	if err != nil {
		return "n/a"
	}
	for _, l := range strings.Split(string(b), "\n") {
		if strings.HasPrefix(l, "Seccomp_filters:") {
			return strings.TrimSpace(strings.TrimPrefix(l, "Seccomp_filters:"))
		}
	}
	return "n/a"
}

// tryNewListener installs an allow-everything filter with NEW_LISTENER on this
// thread and reports "ok" (and closes the listener) or the errno.  With a
// listener already in the stack the kernel answers EBUSY.
func tryNewListener() string {
	runtime.LockOSThread()
	prog := []struct {
		code   uint16
		jt, jf uint8
		k      uint32
	}{{0x06, 0, 0, 0x7fff0000}}
	fprog := struct {
		length uint16
		filter unsafe.Pointer
	}{1, unsafe.Pointer(&prog[0])}
	fd, _, e := syscall.RawSyscall(sysSeccomp, secSetModeFilter, secFilterNewListen, uintptr(unsafe.Pointer(&fprog)))
	runtime.KeepAlive(prog)
	if e != 0 {
		return e.Error()
	}
	syscall.Close(int(fd))
	return "ok"
}

// forkExecTrue runs /usr/bin/true from this thread and reports "ok" or the
// error of the exec (the child's errno comes back through the fork's pipe).
func forkExecTrue() string {
	pid, err := syscall.ForkExec("/usr/bin/true", []string{"true"}, &syscall.ProcAttr{Files: []uintptr{0, 1, 2}})
	if err != nil {
		return err.Error()
	}
	var ws syscall.WaitStatus
	syscall.Wait4(pid, &ws, 0, nil)
	if ws.ExitStatus() != 0 {
		return "exit " + strconv.Itoa(ws.ExitStatus())
	}
	return "ok"
}

// -------------------------------------------------------------------- tests

// (a) An exec notifies, and a responder that answers CONTINUE lets the step
// run.  Also the hand-over contract: exactly one data byte and one descriptor,
// nothing truncated, and end of file afterwards (the init closed the socket).
func TestSandboxExecNotifyNotifies(t *testing.T) {
	f := newSandboxFixture(t)
	marker := filepath.Join(f.pkg, "ran")
	r := f.notifyRunWith(t, respContinue, "sh", "-c", "/usr/bin/true; /usr/bin/true; echo ran > "+marker+"; echo step-done")
	if r.code != 0 || r.stdout != "step-done\n" {
		t.Fatalf("exit %d, stdout %q, stderr %q", r.code, r.stdout, r.stderr)
	}
	if _, err := os.Stat(marker); err != nil {
		t.Errorf("the step did not run: %v", err)
	}
	// The init's execve of sh, then sh's exec of true twice.  (A shell may exec
	// its last command directly: the count of execs is the same either way.)
	nrs := r.resp.numbers()
	if len(nrs) < 3 {
		t.Fatalf("%d notifications %v, want at least 3 (sh and two true)", len(nrs), nrs)
	}
	for _, nr := range nrs {
		if nr != 59 && nr != 322 {
			t.Errorf("a notification for call %d, not execve or execveat", nr)
		}
	}
	h := r.resp.handshake()
	if !h.ok || h.dataBytes != 1 || (h.dataByte != sandbox.ExecNotifyByte && h.dataByte != sandbox.ExecNotifyByteKillable) || h.fds != 1 || h.truncated {
		t.Errorf("hand-over %+v: want one byte %q and one descriptor, nothing truncated", h, sandbox.ExecNotifyByte)
	}
	if !h.eofAfter {
		t.Error("no end of file on the socket after the run: something still holds the sandbox's end")
	}
	if r.stderr != "" {
		t.Errorf("stderr %q", r.stderr)
	}
}

// (b) The first filter's refusals are unchanged with the option on, and
// PR_SET_MM is refused with and without it (h).
func TestSandboxExecNotifyFirstFilterUnchanged(t *testing.T) {
	f := newSandboxFixture(t)
	bin := f.stepBinary(t)
	r := f.notifyRunWith(t, respContinue, bin, probeCommand)
	if r.code != 0 {
		t.Fatalf("exit %d: %s\n%s", r.code, r.stderr, r.stdout)
	}
	for _, want := range probeRefusals {
		if !strings.Contains(r.stdout, want) {
			t.Errorf("with the option, the output lacks %q:\n%s", want, r.stdout)
		}
	}
	if len(r.resp.numbers()) == 0 {
		t.Error("the step's exec was not notified")
	}
}

// (d) and (f, second listener): the step holds no socket and no listener, by a
// descriptor scan inside the step, and it cannot install a listener of its own
// because the stack already has one.  (i): with the option absent the same step
// has ONE filter and CAN install a listener, so no second filter was installed.
func TestSandboxExecNotifyStepHoldsNothing(t *testing.T) {
	f := newSandboxFixture(t)
	bin := f.stepBinary(t)
	// The glob is expanded once, before the loop, and the shell's own directory
	// descriptor for it (a 3) is in the list but closed by the time a link is
	// read.  Reading each link WITHOUT a command substitution matters: `$(...)`
	// makes a pipe, which takes the number the glob's directory just freed, and
	// "fd 3 -> pipe:[N]" is then the scan's own artifact (CI saw exactly that).
	// A descriptor the step really holds has a live target; a stale entry has
	// none (readlink fails).
	scan := `for f in /proc/$$/fd/*; do printf 'fd %s -> ' "${f##*/}"; readlink "$f" 2>/dev/null || echo; done`

	// With the option.
	r := f.notifyRunWith(t, respContinue, "sh", "-c", scan)
	if r.code != 0 {
		t.Fatalf("exit %d: %s", r.code, r.stderr)
	}
	for _, l := range strings.Split(strings.TrimSpace(r.stdout), "\n") {
		fields := strings.Fields(l)
		if len(fields) < 3 || fields[0] != "fd" || fields[2] != "->" {
			t.Errorf("odd line %q", l)
			continue
		}
		if n, _ := strconv.Atoi(fields[1]); n > 2 && len(fields) > 3 {
			t.Errorf("the step holds descriptor %d: %q", n, l)
		}
		if strings.Contains(l, "socket:") || strings.Contains(l, "seccomp") || strings.Contains(l, "anon_inode") {
			t.Errorf("the step holds a socket or a listener: %q", l)
		}
	}
	r = f.notifyRunWith(t, respContinue, bin, execNotifyProbeCommand, "info")
	if r.code != 0 {
		t.Fatalf("info: exit %d: %s\n%s", r.code, r.stderr, r.stdout)
	}
	for _, l := range strings.Split(r.stdout, "\n") {
		if strings.HasPrefix(l, "fdlink=") && (strings.Contains(l, "socket:") || strings.Contains(l, "seccomp notify")) {
			t.Errorf("the step (the Go probe) holds a socket or a listener: %q", l)
		}
	}
	if got := probeLine(r.stdout, "filters="); got != "n/a" && got != "2" {
		t.Errorf("with the option the step has %s seccomp filters, want 2 (the first and the listener's)", got)
	}
	// The second filter itself answers EPERM to seccomp(NEW_LISTENER) (defence
	// in depth; it runs before the kernel's own one-listener-per-task check,
	// has_duplicate_listener, which would say EBUSY).
	if got := probeLine(r.stdout, "new-listener="); got != syscall.EPERM.Error() {
		t.Errorf("a second listener: %q, want %q: the filter refuses NEW_LISTENER", got, syscall.EPERM.Error())
	}
	if !r.resp.handshake().eofAfter {
		t.Error("no end of file on the socket after the run")
	}

	// (i) Without the option: one filter, and a listener can be installed.
	plain := f.sandboxRun(t, "pkg", bin, execNotifyProbeCommand, "info")
	if plain.code != 0 {
		t.Fatalf("plain info: exit %d: %s\n%s", plain.code, plain.stderr, plain.stdout)
	}
	if got := probeLine(plain.stdout, "filters="); got != "n/a" && got != "1" {
		t.Errorf("without the option the step has %s seccomp filters, want 1", got)
	}
	if got := probeLine(plain.stdout, "new-listener="); got != "ok" {
		t.Errorf("without the option the step's own listener: %q, want ok (no listener may exist in its stack)", got)
	}
}

// probeLine is the value of the first output line that starts with prefix.
func probeLine(out, prefix string) string {
	for _, l := range strings.Split(out, "\n") {
		if strings.HasPrefix(l, prefix) {
			return strings.TrimPrefix(l, prefix)
		}
	}
	return ""
}

// (c) A dead listener.  The supervisor receives the listener and closes it (it
// died, or was killed, after the hand-over).  The kernel answers a notified call
// with no listener with ENOSYS, so the init's execve of the step fails, and the
// init reports it as its own refusal:
//
//	exit status 125 (sandbox.ExitFailure), the step NEVER ran, and stderr has
//	"kiln sandbox-init: exec-notify: the listener is gone (execve returned
//	ENOSYS); the step did not run".
//
// In the trace the init has a failed execve (ENOSYS) and no step exec, so the
// parser says helper-no-exec, which with exit 125 the recorder reads as
// "sandbox-refused" (classify_step.go).  PR 4 recognises the case by the message
// prefix sandbox.ExecNotifyRefused (and its own knowledge that it closed the
// listener); the exit status alone is the generic "the step did not run".
//
// The same status and prefix cover a supervisor that is gone BEFORE the send
// (the socket's other end closed: EPIPE, "exec-notify: sending the listener").
func TestSandboxExecNotifyDeadListener(t *testing.T) {
	f := newSandboxFixture(t)
	marker := filepath.Join(f.pkg, "ran")
	step := []string{"sh", "-c", "touch " + marker}

	r := f.notifyRunWith(t, respDropListener, step...)
	if r.code != sandbox.ExitFailure || r.stdout != "" {
		t.Errorf("a closed listener: exit %d, stdout %q, stderr %q; want %d", r.code, r.stdout, r.stderr, sandbox.ExitFailure)
	}
	if want := sandbox.ExecNotifyRefused + " " + sandbox.ExecNotifyListenerGone; !strings.Contains(r.stderr, want) {
		t.Errorf("stderr %q lacks %q", r.stderr, want)
	}
	if _, err := os.Stat(marker); err == nil {
		t.Error("the step ran with a dead listener")
	}
	if !r.resp.handshake().ok {
		t.Errorf("the hand-over did not happen: %+v", r.resp.handshake())
	}

	// A supervisor that is gone before the hand-over: its end is closed, there
	// is nobody to answer.  Either the root refuses the socket (not connected) or
	// the init's send fails; either way 125, no step.
	resp, child := newExecResponder(t, respContinue)
	syscall.Close(resp.sock)
	resp.sock = -1
	r = f.notifyRunOn(t, resp, child, []string{"--exec-notify-fd", "3"}, step...)
	if r.code != sandbox.ExitFailure || r.stdout != "" {
		t.Errorf("a gone peer: exit %d, stdout %q, stderr %q; want %d", r.code, r.stdout, r.stderr, sandbox.ExitFailure)
	}
	if !strings.Contains(r.stderr, "exec-notify") {
		t.Errorf("a gone peer: stderr %q names neither the option nor the exec-notify refusal", r.stderr)
	}
	t.Logf("gone peer: exit %d, stderr %q", r.code, strings.TrimSpace(r.stderr))
	if _, err := os.Stat(marker); err == nil {
		t.Error("the step ran with its supervisor gone")
	}
}

// (e) A configured descriptor that is missing, or is not a connected AF_UNIX
// stream socket, is refused BEFORE the step runs: exit 125, the message names
// the option and what is wrong, and the step (it would create a file) never
// runs.
func TestSandboxExecNotifyRefusesBadDescriptor(t *testing.T) {
	f := newSandboxFixture(t)
	marker := filepath.Join(f.pkg, "ran")
	step := []string{"sh", "-c", "touch " + marker}

	// A valid socket nobody uses, for the case where the option names another
	// descriptor than the one that is open.
	pr, pw, _ := os.Pipe()
	defer pr.Close()
	defer pw.Close()
	reg, err := os.Create(filepath.Join(f.base, "regular"))
	if err != nil {
		t.Fatal(err)
	}
	defer reg.Close()
	mk := func(domain, typ int) *os.File {
		fd, err := syscall.Socket(domain, typ|syscall.SOCK_CLOEXEC, 0)
		if err != nil {
			t.Fatalf("socket(%d, %d): %v", domain, typ, err)
		}
		return os.NewFile(uintptr(fd), "sock")
	}
	dgram := mk(syscall.AF_UNIX, syscall.SOCK_DGRAM)
	defer dgram.Close()
	inet := mk(syscall.AF_INET, syscall.SOCK_STREAM)
	defer inet.Close()
	unconnected := mk(syscall.AF_UNIX, syscall.SOCK_STREAM)
	defer unconnected.Close()
	pair, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_DGRAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		t.Fatal(err)
	}
	dpA, dpB := os.NewFile(uintptr(pair[0]), "dgram-a"), os.NewFile(uintptr(pair[1]), "dgram-b")
	defer dpA.Close()
	defer dpB.Close()

	type refusal struct {
		name  string
		file  *os.File // descriptor 3, or nil
		flags []string
		want  string
	}
	for _, c := range []refusal{
		{"not open", nil, []string{"--exec-notify-fd", "9"}, "not an open descriptor"},
		{"a pipe", pw, []string{"--exec-notify-fd", "3"}, "not a socket"},
		{"a regular file", reg, []string{"--exec-notify-fd", "3"}, "not a socket"},
		{"an unconnected AF_UNIX datagram socket", dgram, []string{"--exec-notify-fd", "3"}, "SOCK_STREAM"},
		{"a connected AF_UNIX datagram pair", dpA, []string{"--exec-notify-fd", "3"}, "SOCK_STREAM"},
		{"an AF_INET socket", inet, []string{"--exec-notify-fd", "3"}, "AF_UNIX"},
		{"an unconnected AF_UNIX stream socket", unconnected, []string{"--exec-notify-fd", "3"}, "not a connected socket"},
		{"a standard stream", nil, []string{"--exec-notify-fd", "2"}, "out of range"},
		{"the same descriptor as stdout", pw, []string{"--stdout-fd", "3", "--stderr-fd", "3", "--exec-notify-fd", "3"}, "also an output descriptor"},
	} {
		// A responder is attached to a good socket the option does NOT name,
		// so that a step that wrongly ran would not hang on its own exec.
		resp, child := newExecResponder(t, respContinue)
		args := c.flags
		extra := c.file
		if extra == nil {
			extra = child // keeps descriptor 3 occupied by a valid socket
		}
		r := f.notifyRunExtra(t, resp, child, []*os.File{extra}, args, step...)
		if r.code != sandbox.ExitFailure || r.stdout != "" || !strings.Contains(r.stderr, c.want) || !strings.Contains(r.stderr, "--exec-notify-fd") {
			t.Errorf("%s: exit %d, stdout %q, stderr %q; want 125 and a message with %q and --exec-notify-fd", c.name, r.code, r.stdout, r.stderr, c.want)
		}
		if _, err := os.Stat(marker); err == nil {
			t.Errorf("%s: the refused invocation still ran the step", c.name)
			os.Remove(marker)
		}
		if h := r.resp.handshake(); h.ok {
			t.Errorf("%s: a listener was handed over although the step was refused", c.name)
		}
	}
}

// notifyRunExtra is notifyRunOn with the root's extra descriptors chosen by the
// caller.  The responder's child end is closed here whether or not it is passed.
func (f *sandboxFixture) notifyRunExtra(t *testing.T, resp *execResponder, child *os.File, extra []*os.File, flags []string, argv ...string) notifyRun {
	t.Helper()
	args := append([]string{sandbox.ExecCommand, "--config", f.cfg}, flags...)
	args = append(append(args, "--"), argv...)
	ctx, cancel := context.WithTimeout(context.Background(), 45*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, f.exe, args...)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"})
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Cancel = func() error { return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL) }
	cmd.ExtraFiles = extra
	var out, errb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &errb
	err := cmd.Start()
	if err != nil {
		child.Close()
		t.Fatalf("start: %v", err)
	}
	child.Close() // never the sandbox's to keep: the responder sees EOF
	resp.begin()
	err = cmd.Wait()
	resp.stop()
	if ctx.Err() != nil {
		t.Fatalf("timed out; stdout %q stderr %q", out.String(), errb.String())
	}
	r := notifyRun{stdout: out.String(), stderr: errb.String(), resp: resp}
	if ee, ok := err.(*exec.ExitError); ok {
		r.code = ee.ExitCode()
	} else if err != nil {
		t.Fatalf("%v\nstderr: %s", err, r.stderr)
	}
	return r
}

// (f) A step that installs its own filter erroring execve loses its execs and
// only those: the kernel's order (ERRNO beats USER_NOTIF) means the supervisor
// is never even asked, the step's exec of /usr/bin/true fails with EACCES, and a
// second listener is REFUSED with EBUSY (documented: "a second listener is
// refused": only one filter of a task's stack may hold a listener, so a step
// cannot take the supervisor's place).
func TestSandboxExecNotifyStepFilterOnlyLosesItsExecs(t *testing.T) {
	f := newSandboxFixture(t)
	bin := f.stepBinary(t)
	r := f.notifyRunWith(t, respContinue, bin, execNotifyProbeCommand, "filter")
	if r.code != 0 {
		t.Fatalf("exit %d: %s\n%s", r.code, r.stderr, r.stdout)
	}
	for prefix, want := range map[string]string{
		"exec-before=":     "ok",
		"second-listener=": syscall.EPERM.Error(),
		"own-filter=":      "ok",
		"exec-after=":      syscall.EACCES.Error(),
	} {
		if got := probeLine(r.stdout, prefix); got != want {
			t.Errorf("%s%q, want %q\n%s", prefix, got, want, r.stdout)
		}
	}
	// Notified: the init's exec of the probe, and the probe's first exec of
	// true.  The second exec of true died in the step's own filter: not asked.
	if n := len(r.resp.numbers()); n != 2 {
		t.Errorf("%d notifications %v, want exactly 2 (the step and its first true)", n, r.resp.numbers())
	}
}

// The equal-action tie (security review of PR 3).  A step installs a filter of
// its own that answers USER_NOTIF for execve, hoping the kernel keeps the newest
// filter's verdict and sends the notifications to a listener the step owns.  It
// cannot own one: (1) seccomp(NEW_LISTENER) is refused, by the second filter
// with EPERM (and, were that absent, by the kernel's one-listener-per-task rule,
// has_duplicate_listener, with EBUSY); (2) a USER_NOTIF filter WITHOUT a listener
// makes the exec fail with ENOSYS, so it is lost and nothing runs unseen: fail
// closed.  The supervisor's responder sees no notification for that exec.
func TestSandboxExecNotifyStepUserNotifFailsClosed(t *testing.T) {
	f := newSandboxFixture(t)
	bin := f.stepBinary(t)
	r := f.notifyRunWith(t, respContinue, bin, execNotifyProbeCommand, "usernotif")
	if r.code != 0 {
		t.Fatalf("exit %d: %s\n%s", r.code, r.stderr, r.stdout)
	}
	for prefix, want := range map[string]string{
		"exec-before=":     "ok",
		"second-listener=": syscall.EPERM.Error(),
		"own-filter=":      "ok",
		"exec-after=":      syscall.ENOSYS.Error(),
	} {
		if got := probeLine(r.stdout, prefix); got != want {
			t.Errorf("%s%q, want %q\n%s", prefix, got, want, r.stdout)
		}
	}
	// Only the init's exec of the probe and its first exec of true reached the
	// supervisor; the stolen-notification attempt reached nobody.
	if n := len(r.resp.numbers()); n != 2 {
		t.Errorf("%d notifications %v, want exactly 2", n, r.resp.numbers())
	}
}

// (g) With real strace, a run with the option parses and cross-checks clean:
// the new calls of the root and the init (the socket checks, seccomp with
// NEW_LISTENER, sendmsg with SCM_RIGHTS, the closes) are tolerated in the
// helper's setup, and the notifications are answered while strace is attached.
func TestSandboxExecNotifyRealStrace(t *testing.T) {
	r := runStracedImpl(t, "cat {data}", nil, nil, true)
	if r.stdout != "hello\n" {
		t.Fatalf("step output %q", r.stdout)
	}
	if ex := r.tr.StepExit(); ex == nil || ex.Killed || ex.Code != 0 {
		t.Errorf("step exit %+v", ex)
	}
	var sawCat bool
	for _, e := range r.tr.Events {
		if e.Syscall == "execve" && e.Result.OK && strings.HasSuffix(e.Path, "/cat") {
			sawCat = true
		}
	}
	if !sawCat {
		t.Error("the step's exec of cat is not in the trace")
	}
	if len(r.notifs) < 2 {
		t.Errorf("%d exec notifications %v, want at least 2 (sh and cat)", len(r.notifs), r.notifs)
	}
	// The cross-check: both readers over the raw trace must agree, with no
	// stricter-only divergence either (KILN_TRACE_DIFF=1 does this for every
	// in-process parse in CI; this makes it unconditional).
	fh, err := os.Open(r.trf)
	if err != nil {
		t.Fatal(err)
	}
	defer fh.Close()
	tr, perr, mismatch, allowed := trace.ParseCrossChecked(fh, r.f.pkg, trace.Options{HelperRoot: true, HelperExe: r.f.exe})
	if perr != nil || tr == nil {
		t.Fatalf("the cross-checked parse refused the trace: %v", perr)
	}
	if mismatch != nil || allowed != 0 {
		t.Fatalf("cross-check: mismatch %v, allowed %d", mismatch, allowed)
	}
	// The argv strace recorded for both helper execs carries the option.
	if !strings.Contains(strings.Join(r.tr.RootExecArgv, " "), "--exec-notify-fd 5") {
		t.Errorf("root argv %q", r.tr.RootExecArgv)
	}
	if !strings.Contains(strings.Join(r.tr.InitExecArgv, " "), "--exec-notify-fd 4") {
		t.Errorf("init argv %q", r.tr.InitExecArgv)
	}
}
