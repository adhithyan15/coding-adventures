//go:build linux && amd64

package sandbox

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"runtime/debug"
	"strconv"
	"strings"
	"syscall"
	"time"
	"unsafe"
)

// Supported reports whether this platform can run the sandbox.
func Supported() bool { return true }

// Raw system calls and constants package syscall does not name.
const (
	sysMountSetattr = 442
	sysCloseRange   = 436
	oPath           = 0x200000

	atFdcwd         = -100
	atRecursive     = 0x8000
	mountAttrRdonly = 1

	prSetPdeathsig    = 1
	prCapbsetRead     = 23
	prCapbsetDrop     = 24
	prSetSeccomp      = 22
	prGetNoNewPrivs   = 39
	prSetNoNewPrivs   = 38
	prCapAmbient      = 47
	prCapAmbientClear = 4
	seccompModeFilter = 2
	closeRangeCloexec = 4
	cloneFS           = 0x200
	linuxCapabilityV3 = 0x20080522
)

// RunExec is `kiln sandbox-exec --config <path> -- <argv...>`: the ROOT of the
// traced tree.  Its whole job is to start the Go runtime's threads, spawn
// exactly one child (the init of a new PID namespace) and wait for it.  The
// trace parser's model of the root allows nothing else after the spawn, so
// nothing here makes a traced call once the child exists.
func RunExec(args []string, stderr io.Writer) int {
	fail := func(err error) int {
		fmt.Fprintf(stderr, "kiln sandbox-exec: %v\n", err)
		return ExitFailure
	}
	ea, err := ParseExecArgs(args)
	if err != nil {
		return fail(err)
	}
	action := os.Getenv(ActionEnv)
	if action == "" {
		return fail(errors.New(ActionEnv + " is not set"))
	}
	data, sum, err := ReadConfig(ea.Config)
	if err != nil {
		return fail(err)
	}
	cfg, err := ParseConfig(data)
	if err != nil {
		return fail(err)
	}
	if _, err := cfg.EnvFor(action); err != nil {
		return fail(err)
	}
	// The step's output descriptors (S2), checked before anything is spawned.
	// fstat and fcntl are descriptor calls, which the trace set does not print.
	outFDs := [2]int{1, 2}
	for i, o := range []struct {
		flag string
		fd   int
	}{{"--stdout-fd", ea.StdoutFD}, {"--stderr-fd", ea.StderrFD}} {
		if o.fd == NoFD {
			continue
		}
		if err := inspectOutputFD(o.flag, o.fd); err != nil {
			return fail(err)
		}
		outFDs[i] = o.fd
	}
	exe, err := os.Readlink("/proc/self/exe")
	if err != nil {
		return fail(err)
	}
	cwd, err := syscall.Getwd()
	if err != nil {
		return fail(err)
	}
	if err := cfg.CheckHost(action, HostFacts{Exe: exe, Cwd: cwd, ConfigPath: ea.Config}); err != nil {
		return fail(err)
	}
	if _, err := Plan(cfg); err != nil {
		return fail(err)
	}
	// No garbage-collector worker threads appear late in the root.
	debug.SetGCPercent(-1)
	devnull, err := syscall.Open("/dev/null", syscall.O_RDONLY|syscall.O_CLOEXEC, 0)
	if err != nil {
		return fail(err)
	}
	// The liveness pipe: the init holds the read end and refuses to run the
	// step if the write end (held only by this process) is gone.
	var alive [2]int
	if err := syscall.Pipe2(alive[:], syscall.O_CLOEXEC); err != nil {
		return fail(err)
	}
	warmThreads()

	initArgv := append([]string{
		"kiln", InitCommand,
		"--config", ea.Config, "--sha256", sum, "--action", action,
		"--uid", strconv.Itoa(os.Getuid()), "--gid", strconv.Itoa(os.Getgid()),
		"--",
	}, ea.Argv...)
	// One clone creates the five namespaces.  Network, IPC and UTS isolate the
	// step from the host's services (docker.sock, D-Bus, abstract sockets,
	// loopback listeners, SysV and POSIX IPC).  The decisions behind the rest:
	//
	//   - No UidMappings: Go would write them from THIS process after the
	//     clone (an openat the parser's model of the root forbids), so the init
	//     maps its own ids, which the kernel allows for a single id.
	//   - No Pdeathsig: Go's child checks getppid() against the parent's pid,
	//     which is 0 in a new PID namespace, and would kill itself.  The init
	//     sets PR_SET_PDEATHSIG itself and checks the liveness pipe.
	//   - Setsid: the step must not share the session or process group of
	//     strace, this process and the build tool, or kill(0, sig) would reach
	//     them (strace dying detaches its tracees).
	//   - AmbientCaps: the init keeps its new user namespace's capabilities
	//     across the exec of a not-yet-mapped uid.
	//   - Env is EMPTY: the init gets everything on its command line.
	//   - syscall.ForkExec, not os.StartProcess: os.StartProcess probes pidfd
	//     support with a throwaway clone, a second spawn in the root.  Nothing
	//     else before this point clones a process (the only other clones are
	//     warmThreads' CLONE_THREAD threads).
	pid, err := syscall.ForkExec("/proc/self/exe", initArgv, &syscall.ProcAttr{
		Env: []string{},
		// The step's 1 and 2 are the root's own, or the descriptors named by
		// --stdout-fd and --stderr-fd: the clone puts them where the init's
		// (and then the step's) descriptors 1 and 2 are, so the init needs no
		// dup2 and the root makes no extra traced call.
		Files: []uintptr{uintptr(devnull), uintptr(outFDs[0]), uintptr(outFDs[1]), uintptr(alive[0])},
		Sys: &syscall.SysProcAttr{
			Cloneflags: syscall.CLONE_NEWUSER | syscall.CLONE_NEWNS | syscall.CLONE_NEWPID |
				syscall.CLONE_NEWIPC | syscall.CLONE_NEWUTS | syscall.CLONE_NEWNET,
			Setsid:      true,
			AmbientCaps: StartCaps(),
		},
	})
	if err != nil {
		return fail(fmt.Errorf("cannot start the sandbox (are unprivileged user namespaces enabled?): %v", err))
	}
	syscall.Close(devnull)
	syscall.Close(alive[0])
	// The root has no more use for the output descriptors; closing them here
	// (once, if both name the same one) lets the reader see end of file when
	// the step is done, not only when the root exits.
	for i, fd := range outFDs {
		if fd > 2 && (i == 0 || fd != outFDs[0]) {
			syscall.Close(fd)
		}
	}
	var ws syscall.WaitStatus
	for {
		_, err = syscall.Wait4(pid, &ws, 0, nil)
		if err != syscall.EINTR {
			break
		}
	}
	if err != nil {
		return fail(err)
	}
	if ws.Signaled() {
		return 128 + int(ws.Signal())
	}
	return ws.ExitStatus()
}

// inspectOutputFD fstat()s and F_GETFL()s a descriptor the caller named for
// the step's output and applies CheckOutputFD.  A descriptor that is not open
// is its own error.
func inspectOutputFD(flag string, fd int) error {
	what := fmt.Sprintf("%s %d", flag, fd)
	var st syscall.Stat_t
	if err := syscall.Fstat(fd, &st); err != nil {
		if err == syscall.EBADF {
			return fmt.Errorf("sandbox-exec: %s is not an open descriptor", what)
		}
		return fmt.Errorf("sandbox-exec: %s: %v", what, err)
	}
	flags, _, e := syscall.RawSyscall(syscall.SYS_FCNTL, uintptr(fd), syscall.F_GETFL, 0)
	if e != 0 {
		return fmt.Errorf("sandbox-exec: %s: F_GETFL: %v", what, e)
	}
	return CheckOutputFD(what, st.Mode, int(flags))
}

// warmThreads starts the Go runtime's worker threads BEFORE the child is
// spawned.  A thread started later is a clone() in the root after the spawn,
// which the parser refuses (helper-activity).  Several goroutines block in a
// system call at once, which makes the scheduler start threads to run the
// goroutines behind them; the threads then stay parked and are reused.  It is
// best effort: a late thread makes the trace incomplete (fail closed), never
// wrong.
func warmThreads() {
	var p [2]int
	if err := syscall.Pipe2(p[:], syscall.O_CLOEXEC); err != nil {
		return
	}
	n := runtime.GOMAXPROCS(0) + 2
	done := make(chan struct{}, n)
	for i := 0; i < n; i++ {
		go func() {
			var b [1]byte
			_, _ = syscall.Read(p[0], b[:])
			done <- struct{}{}
		}()
	}
	time.Sleep(50 * time.Millisecond)
	syscall.Close(p[1]) // end of file: every reader returns
	for i := 0; i < n; i++ {
		<-done
	}
	syscall.Close(p[0])
}

// RunInit is `kiln sandbox-init ...`: PID 1 of the new namespaces.  It prepares
// the sandbox and execs the step.  It returns only on failure.
func RunInit(args []string, stderr io.Writer) int {
	// Every setup step is made on this thread, the thread-group leader, before
	// any other goroutine starts: the final exec must come from the leader.
	runtime.LockOSThread()
	debug.SetGCPercent(-1)
	fail := func(err error) int {
		fmt.Fprintf(stderr, "kiln sandbox-init: %v\n", err)
		return ExitFailure
	}
	ia, err := ParseInitArgs(args)
	if err != nil {
		return fail(err)
	}
	if syscall.Getpid() != 1 || syscall.Gettid() != 1 {
		return fail(errors.New("not PID 1 of a new PID namespace; this command is internal to sandbox-exec"))
	}
	// If the root dies, so does the init (and, as PID 1, everything with it).
	// The root created this process from its locked main thread, which lives
	// as long as the root does.  The setting survives the exec of the step.
	if err := prctl(prSetPdeathsig, uintptr(syscall.SIGKILL), 0, 0); err != nil {
		return fail(fmt.Errorf("PR_SET_PDEATHSIG: %v", err))
	}
	// The signal is not enough: a root that died before the line above leaves
	// no parent to signal this process.  The liveness pipe says so.
	if err := checkRootAlive(); err != nil {
		return fail(err)
	}
	// A crashing step must never invoke the host's core_pattern handler.  A
	// limit of 0 would not stop a piped handler (a pipe ignores the limit);
	// the kernel treats the value 1 specially and skips a piped dump.  Soft and
	// hard are both 1, so the step cannot raise it.
	if err := syscall.Setrlimit(syscall.RLIMIT_CORE, &syscall.Rlimit{Cur: 1, Max: 1}); err != nil {
		return fail(fmt.Errorf("RLIMIT_CORE: %v", err))
	}
	if err := mapIDs(ia.UID, ia.GID); err != nil {
		return fail(err)
	}
	data, sum, err := ReadConfig(ia.Config)
	if err != nil {
		return fail(err)
	}
	if sum != ia.SHA256 {
		return fail(errors.New("the configuration changed between the root and the init"))
	}
	cfg, err := ParseConfig(data)
	if err != nil {
		return fail(err)
	}
	if err := loopbackUp(); err != nil {
		return fail(err)
	}
	env, err := cfg.EnvFor(ia.Action)
	if err != nil {
		return fail(err)
	}
	exe, err := os.Readlink("/proc/self/exe")
	if err != nil {
		return fail(err)
	}
	cwd, err := syscall.Getwd()
	if err != nil {
		return fail(err)
	}
	if err := cfg.CheckHost(ia.Action, HostFacts{Exe: exe, Cwd: cwd, ConfigPath: ia.Config}); err != nil {
		return fail(err)
	}
	plan, err := Plan(cfg)
	if err != nil {
		return fail(err)
	}
	// The Go runtime's threads share this process's working directory, and the
	// trace parser calls a chdir on a shared one a race.  A private copy for
	// this thread (the one that will exec) makes the setup chdirs private.
	if _, _, e := syscall.RawSyscall(syscall.SYS_UNSHARE, cloneFS, 0, 0); e != 0 {
		return fail(fmt.Errorf("unshare(CLONE_FS): %v", e))
	}
	if err := applyPlan(cfg, plan); err != nil {
		return fail(err)
	}
	if err := dropPrivileges(); err != nil {
		return fail(err)
	}
	if err := installSeccomp(); err != nil {
		return fail(err)
	}
	if err := redirectStdinAndCloseFDs(); err != nil {
		return fail(err)
	}
	if err := syscall.Chdir(cwd); err != nil {
		return fail(fmt.Errorf("chdir %q: %v", clip(cwd), err))
	}
	// Reaching here means the exec failed: report it like a shell would.
	fmt.Fprintf(stderr, "kiln sandbox-init: %v\n", execStep(ia.Argv, env))
	return 127
}

// mapIDs maps the caller's uid and gid to InsideID in this process's own new
// user namespace.  A process may write a single-id map for its creator's
// effective ids without privilege over the parent namespace, provided
// setgroups is denied first (so a map cannot be used to drop group
// restrictions).
func mapIDs(uid, gid uint32) error {
	writes := []struct{ file, text string }{
		{"/proc/self/setgroups", "deny"},
		{"/proc/self/uid_map", fmt.Sprintf("%d %d 1\n", InsideID, uid)},
		{"/proc/self/gid_map", fmt.Sprintf("%d %d 1\n", InsideID, gid)},
	}
	for _, w := range writes {
		fd, err := syscall.Open(w.file, syscall.O_WRONLY, 0)
		if err != nil {
			return fmt.Errorf("open %s: %v", w.file, err)
		}
		_, err = syscall.Write(fd, []byte(w.text))
		syscall.Close(fd)
		if err != nil {
			return fmt.Errorf("write %s: %v", w.file, err)
		}
	}
	if syscall.Getuid() != InsideID || syscall.Getgid() != InsideID {
		return errors.New("the id map did not take effect")
	}
	return nil
}

// applyPlan performs the mount plan.
func applyPlan(c *Config, plan []Mount) error {
	stage := StageRoot(c)
	at := func(p string) string { return stage + p } // a path of the new root, seen from the old
	for _, m := range plan {
		var err error
		switch m.Kind {
		case KindMakePrivate:
			err = syscall.Mount("", m.Target, "", m.Flags, "")
		case KindHostMkdir:
			if err = os.MkdirAll(m.Target, 0o755); err == nil {
				err = mustBeCanonical("state root", m.Target)
			}
		case KindStageTmpfs:
			err = syscall.Mount(m.FSType, m.Target, m.FSType, m.Flags, m.Data)
		case KindStageDir:
			err = os.Mkdir(m.Target, 0o755)
		case KindRootTmpfs:
			err = syscall.Mount(m.FSType, m.Target, m.FSType, m.Flags, m.Data)
		case KindRORoot:
			err = applyRORoot(m.Source, at(m.Target))
		case KindTmpfs:
			err = syscall.Mount(m.FSType, at(m.Target), m.FSType, m.Flags, m.Data)
		case KindEnsureDir:
			err = os.MkdirAll(at(m.Target), 0o755)
		case KindBind:
			err = syscall.Mount(m.Source, at(m.Target), "", m.Flags, "")
		case KindSealDir:
			err = setReadOnly(at(m.Target))
		case KindMask:
			err = applyMask(at(m.Target), m)
		case KindDev:
			err = applyDev(at(m.Target), m)
		case KindVerifyAbsent:
			err = verifyAbsent()
		case KindSeal:
			err = syscall.Mount("", m.Target, "", m.Flags, "")
		case KindProc:
			err = syscall.Mount(m.Source, at(m.Target), m.FSType, m.Flags, m.Data)
		case KindPivot:
			err = pivot(m.Target)
		default:
			err = errors.New("unknown plan step")
		}
		if err != nil {
			return fmt.Errorf("%s %q: %v", m.Kind, clip(m.Target), err)
		}
	}
	return nil
}

// setReadOnly makes one mount read-only with mount_setattr, which sets only
// that attribute.  A plain remount would have to repeat every inherited mount
// flag, and a user namespace locks them.
func setReadOnly(target string) error {
	p, err := syscall.BytePtrFromString(target)
	if err != nil {
		return err
	}
	attr := struct{ set, clr, propagation, userns uint64 }{set: mountAttrRdonly}
	dirfd := atFdcwd
	_, _, e := syscall.Syscall6(sysMountSetattr, uintptr(dirfd), uintptr(unsafe.Pointer(p)), 0,
		uintptr(unsafe.Pointer(&attr)), unsafe.Sizeof(attr), 0)
	if e != 0 {
		return e
	}
	return nil
}

// applyRORoot makes one allowed host root visible, read-only, at the same path
// in the new root.  A symlink (/bin on most images) is recreated, its target
// resolving inside the new root; a directory or file is bound, NON-recursively
// (a bind that would carry a mount beneath it along, which the kernel refuses
// when that mount is locked, is a failure rather than a surprise), then made
// read-only.  The missing parents are made on the root tmpfs.
func applyRORoot(src, dst string) error {
	st, err := os.Lstat(src)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return err
	}
	if st.Mode()&os.ModeSymlink != 0 {
		target, err := os.Readlink(src)
		if err != nil {
			return err
		}
		return os.Symlink(target, dst)
	}
	if st.IsDir() {
		if err := os.Mkdir(dst, 0o755); err != nil && !errors.Is(err, os.ErrExist) {
			return err
		}
	} else {
		f, err := os.OpenFile(dst, os.O_CREATE|os.O_WRONLY, 0o644)
		if err != nil {
			return err
		}
		f.Close()
	}
	if err := syscall.Mount(src, dst, "", MsBind, ""); err != nil {
		return err
	}
	return setReadOnly(dst)
}

// applyMask hides target: an empty read-only tmpfs over a directory, the empty
// /dev/null over anything else.  The target is opened ONCE, O_PATH|O_NOFOLLOW,
// and everything after goes through that descriptor (/proc/self/fd/N), so what
// was inspected is what is mounted over: there is no look-then-mount race.  A
// symlink is refused, and so is an absent target.
func applyMask(target string, m Mount) error {
	fd, err := syscall.Open(target, oPath|syscall.O_NOFOLLOW|syscall.O_CLOEXEC, 0)
	if err != nil {
		return err
	}
	defer syscall.Close(fd)
	var st syscall.Stat_t
	if err := syscall.Fstat(fd, &st); err != nil {
		return err
	}
	fdPath := "/proc/self/fd/" + strconv.Itoa(fd)
	switch st.Mode & syscall.S_IFMT {
	case syscall.S_IFLNK:
		return errors.New("is a symlink")
	case syscall.S_IFDIR:
		return syscall.Mount(m.FSType, fdPath, m.FSType, m.Flags, m.Data)
	}
	return syscall.Mount("/dev/null", fdPath, "", MsBind, "")
}

// applyDev builds the minimal /dev: a fresh tmpfs with the DevNodes bound from
// the host's /dev and the DevLinks, then read-only.  There is no /dev/pts, no
// /dev/kvm, no /dev/fuse, no disks.
func applyDev(dev string, m Mount) error {
	if st, err := os.Lstat(dev); err != nil || !st.IsDir() {
		return fmt.Errorf("%s is not a directory", dev)
	}
	if err := syscall.Mount(m.FSType, dev, m.FSType, m.Flags, m.Data); err != nil {
		return err
	}
	for _, n := range DevNodes {
		f, err := os.OpenFile(dev+"/"+n, os.O_CREATE|os.O_WRONLY|os.O_EXCL, 0o666)
		if err != nil {
			return err
		}
		f.Close()
		if err := syscall.Mount("/dev/"+n, dev+"/"+n, "", MsBind, ""); err != nil {
			return fmt.Errorf("bind /dev/%s: %v", n, err)
		}
	}
	for _, l := range DevLinks {
		if err := os.Symlink(l.Target, dev+"/"+l.Name); err != nil {
			return err
		}
	}
	return syscall.Mount("", dev, "", syscall.MS_REMOUNT|MsRdonly|m.Flags, "")
}

// verifyAbsent refuses to go on if a way to a host service survived: the
// mounts of RunDirs should have hidden them.  It runs after the pivot, so it
// looks at what the step will see.
func verifyAbsent() error {
	for _, p := range HostIPCPaths {
		if _, err := os.Lstat(p); err == nil {
			return fmt.Errorf("%s exists in the sandbox", p)
		} else if !errors.Is(err, os.ErrNotExist) {
			return fmt.Errorf("%s: %v", p, err)
		}
	}
	return nil
}

// loopbackUp brings "lo" up in the new network namespace, which starts with
// only a down loopback.  The step has no other network (KILN04 section 4.7);
// tools that bind or connect on 127.0.0.1 within the step still work.
func loopbackUp() error {
	fd, err := syscall.Socket(syscall.AF_INET, syscall.SOCK_DGRAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		return fmt.Errorf("loopback socket: %v", err)
	}
	defer syscall.Close(fd)
	var ifr struct {
		name  [16]byte
		flags uint16
		_     [22]byte
	}
	copy(ifr.name[:], "lo")
	const siocgifflags, siocsifflags, iffUp = 0x8913, 0x8914, 0x1
	if _, _, e := syscall.Syscall(syscall.SYS_IOCTL, uintptr(fd), siocgifflags, uintptr(unsafe.Pointer(&ifr))); e != 0 {
		return fmt.Errorf("SIOCGIFFLAGS lo: %v", e)
	}
	ifr.flags |= iffUp
	if _, _, e := syscall.Syscall(syscall.SYS_IOCTL, uintptr(fd), siocsifflags, uintptr(unsafe.Pointer(&ifr))); e != 0 {
		return fmt.Errorf("SIOCSIFFLAGS lo: %v", e)
	}
	return nil
}

// livenessFD is the descriptor the root's liveness pipe arrives on.
const livenessFD = 3

// checkRootAlive reads the liveness pipe without blocking: no data and "would
// block" means the root still holds the write end; end of file means it is
// gone.
func checkRootAlive() error { return checkPipeAlive(livenessFD) }

// checkPipeAlive is checkRootAlive for any descriptor.
func checkPipeAlive(fd int) error {
	if err := syscall.SetNonblock(fd, true); err != nil {
		return fmt.Errorf("liveness pipe: %v", err)
	}
	var b [1]byte
	n, err := syscall.Read(fd, b[:])
	switch {
	case err == syscall.EAGAIN:
		return nil
	case err == nil && n == 0:
		return errors.New("the sandbox-exec root is gone")
	}
	return fmt.Errorf("liveness pipe: unexpected state (n=%d, err=%v)", n, err)
}

// pivot makes newRoot the root and detaches the old one.  pivot_root(".", ".")
// needs no directory to park the old root in.
func pivot(newRoot string) error {
	if err := syscall.Chdir(newRoot); err != nil {
		return err
	}
	if err := syscall.PivotRoot(".", "."); err != nil {
		return err
	}
	if err := syscall.Unmount(".", syscall.MNT_DETACH); err != nil {
		return err
	}
	return syscall.Chdir("/")
}

// dropPrivileges runs PrivSteps and then checks the result, so a kernel that
// ignored a step is an error and not a quiet hole.
func dropPrivileges() error {
	for _, s := range PrivSteps() {
		switch s.Op {
		case PrivNoNewPrivs:
			if err := prctl(prSetNoNewPrivs, 1, 0, 0); err != nil {
				return fmt.Errorf("PR_SET_NO_NEW_PRIVS: %v", err)
			}
		case PrivBoundingDrop:
			for c := uintptr(0); c < uintptr(s.Arg); c++ {
				err := prctl(prCapbsetDrop, c, 0, 0)
				if err == syscall.EINVAL {
					break // past the kernel's last capability
				}
				if err != nil {
					return fmt.Errorf("PR_CAPBSET_DROP %d: %v", c, err)
				}
			}
		case PrivAmbientClear:
			if err := prctl(prCapAmbient, prCapAmbientClear, 0, 0); err != nil {
				return fmt.Errorf("PR_CAP_AMBIENT_CLEAR_ALL: %v", err)
			}
		case PrivCapsetEmpty:
			if err := capsetEmpty(); err != nil {
				return fmt.Errorf("capset: %v", err)
			}
		}
	}
	// Verify.
	if r, _, _ := syscall.RawSyscall6(syscall.SYS_PRCTL, prGetNoNewPrivs, 0, 0, 0, 0, 0); r != 1 {
		return errors.New("no_new_privs is not set")
	}
	for c := uintptr(0); c < MaxCapability; c++ {
		r, _, e := syscall.RawSyscall6(syscall.SYS_PRCTL, prCapbsetRead, c, 0, 0, 0, 0)
		if e == syscall.EINVAL {
			break
		}
		if e != 0 || r != 0 {
			return fmt.Errorf("capability %d is still in the bounding set", c)
		}
	}
	var hdr capHeader
	var data [2]capData
	hdr.version, hdr.pid = linuxCapabilityV3, 0
	if _, _, e := syscall.RawSyscall(syscall.SYS_CAPGET, uintptr(unsafe.Pointer(&hdr)), uintptr(unsafe.Pointer(&data[0])), 0); e != 0 {
		return fmt.Errorf("capget: %v", e)
	}
	if data != [2]capData{} {
		return errors.New("capabilities remain after capset")
	}
	return nil
}

type capHeader struct {
	version uint32
	pid     int32
}

type capData struct {
	effective, permitted, inheritable uint32
}

func capsetEmpty() error {
	hdr := capHeader{version: linuxCapabilityV3}
	var data [2]capData
	_, _, e := syscall.RawSyscall(syscall.SYS_CAPSET, uintptr(unsafe.Pointer(&hdr)), uintptr(unsafe.Pointer(&data[0])), 0)
	if e != 0 {
		return e
	}
	return nil
}

func prctl(option, a2, a3, a4 uintptr) error {
	_, _, e := syscall.RawSyscall6(syscall.SYS_PRCTL, option, a2, a3, a4, 0, 0)
	if e != 0 {
		return e
	}
	return nil
}

// installSeccomp loads the filter on this thread; the exec carries it to the
// step (a filter is inherited across fork, clone and execve and cannot be
// removed).
func installSeccomp() error {
	prog, err := BuildFilter()
	if err != nil {
		return err
	}
	fprog := struct {
		length uint16
		filter *Instruction
	}{uint16(len(prog)), &prog[0]}
	err = prctl(prSetSeccomp, seccompModeFilter, uintptr(unsafe.Pointer(&fprog)), 0)
	runtime.KeepAlive(prog)
	if err != nil {
		return fmt.Errorf("seccomp: %v", err)
	}
	return nil
}

// redirectStdinAndCloseFDs makes standard input /dev/null and marks every
// descriptor from 3 up close-on-exec, so none reaches the step.  Close-on-exec
// rather than close: Go's runtime holds descriptors of its own (the network
// poller's epoll fd) that its monitor thread may still use until the exec, and
// closing one under it is a fatal runtime error.  After the exec the effect is
// the same.
func redirectStdinAndCloseFDs() error {
	fd, err := syscall.Open("/dev/null", syscall.O_RDONLY|syscall.O_CLOEXEC, 0)
	if err != nil {
		return fmt.Errorf("open /dev/null: %v", err)
	}
	if fd != 0 {
		if err := syscall.Dup3(fd, 0, 0); err != nil {
			return fmt.Errorf("dup3: %v", err)
		}
	}
	if _, _, e := syscall.RawSyscall(sysCloseRange, 3, 0xffffffff, closeRangeCloexec); e != 0 {
		// Only a kernel without close_range (before 5.11) or without its
		// CLOEXEC flag (before 5.11 too) gets the slow path; any other error
		// is a failure, not a reason to guess.
		if e != syscall.ENOSYS && e != syscall.EINVAL {
			return fmt.Errorf("close_range: %v", e)
		}
		ents, err := os.ReadDir("/proc/self/fd")
		if err != nil {
			return fmt.Errorf("listing /proc/self/fd: %v", err)
		}
		for _, ent := range ents {
			n, err := strconv.Atoi(ent.Name())
			if err != nil || n < 3 {
				continue
			}
			if _, _, e := syscall.RawSyscall(syscall.SYS_FCNTL, uintptr(n), syscall.F_SETFD, syscall.FD_CLOEXEC); e != 0 && e != syscall.EBADF {
				return fmt.Errorf("fcntl %d: %v", n, e)
			}
		}
	}
	return nil
}

// execStep execs the step with exactly env.  A name with no slash is looked up
// in env's own PATH, never the helper's, trying each candidate with a real
// execve (as execvp does); the failures are real probes the trace records.
func execStep(argv, env []string) error {
	if strings.Contains(argv[0], "/") {
		return syscall.Exec(argv[0], argv, env)
	}
	last := error(syscall.ENOENT)
	for _, cand := range PathLookup(argv[0], env) {
		if err := syscall.Exec(cand, argv, env); err != nil {
			last = err
		}
	}
	return fmt.Errorf("exec %q: %v", clip(argv[0]), last)
}
