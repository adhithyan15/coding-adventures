//go:build linux && amd64

package sandbox

import (
	"errors"
	"fmt"
	"io"
	"os"
	"runtime"
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
	fallbackFdCeiling = 4096
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
	devnull, err := os.Open("/dev/null")
	if err != nil {
		return fail(err)
	}
	warmThreads()

	initArgv := append([]string{
		"kiln", InitCommand,
		"--config", ea.Config, "--sha256", sum, "--action", action,
		"--uid", strconv.Itoa(os.Getuid()), "--gid", strconv.Itoa(os.Getgid()),
		"--",
	}, ea.Argv...)
	// Cloneflags creates the three namespaces in the one clone.  There are no
	// UidMappings here on purpose: Go would write them from THIS process after
	// the clone (an openat the parser's model of the root forbids), so the
	// init maps its own ids, which the kernel allows for a single id.
	// No Pdeathsig here either: Go's child checks getppid() against the
	// parent's pid, which is 0 in a new PID namespace, and would kill itself.
	// The init sets PR_SET_PDEATHSIG for itself.
	// AmbientCaps lets the init keep its new user namespace's capabilities
	// across the exec of a non-root uid.  The environment is EMPTY: the init
	// gets everything it needs on its command line.
	proc, err := os.StartProcess("/proc/self/exe", initArgv, &os.ProcAttr{
		Env:   []string{},
		Files: []*os.File{devnull, os.Stdout, os.Stderr},
		Sys: &syscall.SysProcAttr{
			Cloneflags:  syscall.CLONE_NEWUSER | syscall.CLONE_NEWNS | syscall.CLONE_NEWPID,
			AmbientCaps: StartCaps(),
		},
	})
	if err != nil {
		return fail(fmt.Errorf("cannot start the sandbox (are unprivileged user namespaces enabled?): %v", err))
	}
	state, err := proc.Wait()
	if err != nil {
		return fail(err)
	}
	if ws, ok := state.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
		return 128 + int(ws.Signal())
	}
	return state.ExitCode()
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
		case KindRootBind:
			err = syscall.Mount(m.Source, m.Target, "", m.Flags, "")
		case KindReadOnly:
			err = setReadOnlyRecursive(m.Target)
		case KindTmpfs:
			err = syscall.Mount(m.FSType, at(m.Target), m.FSType, m.Flags, m.Data)
		case KindEnsureDir:
			err = os.MkdirAll(at(m.Target), 0o755)
		case KindBind:
			err = syscall.Mount(m.Source, at(m.Target), "", m.Flags, "")
		case KindMask:
			err = applyMask(at(m.Target), m)
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

// setReadOnlyRecursive makes a mount tree read-only in one call.  A plain
// remount of each bind would have to repeat every inherited mount flag (they
// are locked in a user namespace); mount_setattr only sets the one attribute.
func setReadOnlyRecursive(target string) error {
	p, err := syscall.BytePtrFromString(target)
	if err != nil {
		return err
	}
	attr := struct{ set, clr, propagation, userns uint64 }{set: mountAttrRdonly}
	dirfd := atFdcwd
	_, _, e := syscall.Syscall6(sysMountSetattr, uintptr(dirfd), uintptr(unsafe.Pointer(p)), atRecursive,
		uintptr(unsafe.Pointer(&attr)), unsafe.Sizeof(attr), 0)
	if e != 0 {
		return e
	}
	return nil
}

// applyMask hides target: an empty read-only tmpfs over a directory, the empty
// /dev/null over anything else.  A target that is absent is skipped outside the
// checkout and refused inside it.
func applyMask(target string, m Mount) error {
	st, err := os.Lstat(target)
	switch {
	case errors.Is(err, os.ErrNotExist):
		if m.InRepo {
			return errors.New("absent inside the checkout")
		}
		return nil
	case err != nil:
		return err
	case st.Mode()&os.ModeSymlink != 0:
		return errors.New("is a symlink")
	case st.IsDir():
		return syscall.Mount(m.FSType, target, m.FSType, m.Flags, m.Data)
	}
	return syscall.Mount("/dev/null", target, "", MsBind, "")
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
		// No close_range (before Linux 5.11): mark them one by one.
		for f := 3; f < fallbackFdCeiling; f++ {
			syscall.RawSyscall(syscall.SYS_FCNTL, uintptr(f), syscall.F_SETFD, syscall.FD_CLOEXEC)
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
