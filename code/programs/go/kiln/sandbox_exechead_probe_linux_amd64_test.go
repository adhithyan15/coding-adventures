//go:build linux && amd64

package main

// The step side of the exec-head tests: a probe that runs INSIDE the sandbox, as
// (part of) the step, and makes the exec calls the supervisor has to judge.  It
// is this test binary re-run with the `sandbox-exechead-probe` command.
//
// Two facts about Go shape it:
//
//   - A Go program's main goroutine may run on any thread, and an exec from a
//     thread that is not the thread-group leader is a NON-LEADER exec (the tid
//     of the notification is not the process's pid).  init() locks the main
//     goroutine to the main thread, so "the process execs" means the leader does,
//     and the tests that want a non-leader exec (and a second thread) make it on
//     purpose with a locked goroutine.
//   - syscall.Exec takes a runtime lock around the call, so two threads cannot be
//     inside it at once, and a thread held in a notification would hold it for
//     the other.  The probes that need two threads in execve at the same time use
//     syscall.Syscall(SYS_EXECVE) directly, which has no such lock and, being a
//     blocking call, hands the processor to the runtime meanwhile.

import (
	"fmt"
	"os"
	"os/signal"
	"runtime"
	"strconv"
	"sync/atomic"
	"syscall"
	"time"
	"unsafe"
)

const execHeadProbeCommand = "sandbox-exechead-probe"

func init() { runtime.LockOSThread() }

// rawExecve calls execve(path, [path], []) without the Go runtime's exec lock.
// It returns the errno (it returns only on failure).
func rawExecve(path string) syscall.Errno {
	p, err := syscall.BytePtrFromString(path)
	if err != nil {
		return syscall.EINVAL
	}
	argv := []*byte{p, nil}
	envv := []*byte{nil}
	_, _, e := syscall.Syscall(syscall.SYS_EXECVE, uintptr(unsafe.Pointer(p)), uintptr(unsafe.Pointer(&argv[0])), uintptr(unsafe.Pointer(&envv[0])))
	return e
}

func sleepMS(n int) { time.Sleep(time.Duration(n) * time.Millisecond) }

// ehProbe is the probe's main.  Modes:
//
//	done                          exit 0
//	report                        print ino=<inode of /proc/self/exe>
//	selfexec                      exec this very file (a self re-exec)
//	chain <path>                  exec <path>
//	race <a> <b>                  exec a path whose last byte another thread flips between a's and b's
//	hold [ms]                     stay alive (default 1000 ms): a tmpfs binary that outlives the poller.
//	                              (Not `cp /bin/sleep`: on Ubuntu 26.04 /bin/sleep is a multi-call
//	                              binary that dispatches on argv[0] and exits when copied.)
//	racepath <path> <k>           print try=<k>, exec <path> (another PROCESS renames files over it
//	                              meanwhile); the new image reports ino=<inode of its exe)
//	execfail <path>               exec <path> (which fails), stay alive 1.5 s
//	bypass <path>                 the same-thread bypass (see the test)
//	held <path>                   a different thread's exec held behind a pending one
//	dethread <path>               a held notification whose thread dies
//	signal                        a signal interrupts a notified exec
//	overlap <path>                two threads exec, then the leader execs <path>
//	misc                          NULL, missing, non-UTF-8 paths, then fexecve of /usr/bin/true
//	execveat <mode> <dir> <name>  execveat with dirfd mode dir, high or cwd
//	nonleader <path> <arg...>     a non-leader thread execs <path>
func ehProbe(args []string, out *os.File) int {
	say := func(format string, a ...any) { fmt.Fprintf(out, format+"\n", a...) }
	if len(args) == 0 {
		return 2
	}
	env := os.Environ()
	switch args[0] {
	case "done":
		return 0
	case "report":
		var st syscall.Stat_t
		if err := syscall.Stat("/proc/self/exe", &st); err != nil {
			say("report-error=%v", err)
			return 1
		}
		say("ino=%d", st.Ino)
		return 0
	case "selfexec":
		exe := os.Args[0]
		err := syscall.Exec(exe, []string{exe, execHeadProbeCommand, "done"}, env)
		say("selfexec=%v", err)
		return 1
	case "chain":
		err := syscall.Exec(args[1], []string{args[1], "1"}, env)
		say("chain=%v", err)
		return 1
	case "race":
		a, b := args[1], args[2]
		last := len(a) - 1
		buf := append([]byte(a), 0)
		var stop atomic.Bool
		go func() {
			for !stop.Load() {
				buf[last] = a[last]
				buf[last] = b[last]
			}
		}()
		sleepMS(3) // let the flipper start on another thread
		self, _ := syscall.BytePtrFromString(execHeadProbeCommand)
		mode, _ := syscall.BytePtrFromString("report")
		argv := []*byte{&buf[0], self, mode, nil}
		envv := []*byte{nil}
		_, _, e := syscall.Syscall(syscall.SYS_EXECVE, uintptr(unsafe.Pointer(&buf[0])), uintptr(unsafe.Pointer(&argv[0])), uintptr(unsafe.Pointer(&envv[0])))
		stop.Store(true)
		say("race=%v", e)
		return 1
	case "hold":
		ms := 1000
		if len(args) > 1 {
			ms, _ = strconv.Atoi(args[1])
		}
		sleepMS(ms)
		return 0
	case "racepath":
		say("try=%s", args[2])
		err := syscall.Exec(args[1], []string{args[1], execHeadProbeCommand, "report"}, env)
		say("racepath=%v", err)
		return 1
	case "execfail":
		say("execfail=%v", rawExecve(args[1]))
		sleepMS(1500)
		return 0
	case "bypass":
		start := time.Now()
		go func() {
			runtime.LockOSThread()
			sleepMS(50)
			rawExecve("/nonexistent/c") // a different thread: held behind the pending exec
		}()
		say("a=%v", rawExecve(args[1])) // fails at once; its poller is slow (the test's PollStart)
		sleepMS(100)
		say("b=%v", rawExecve("/nonexistent/b")) // the SAME thread: settles a, bypasses the queue
		say("b-returned-ms=%d", time.Since(start).Milliseconds())
		sleepMS(150)
		return 0
	case "held":
		go func() {
			runtime.LockOSThread()
			sleepMS(50)
			say("h=%v", rawExecve("/nonexistent/h"))
		}()
		say("a=%v", rawExecve(args[1]))
		sleepMS(1200)
		return 0
	case "dethread":
		go func() {
			runtime.LockOSThread()
			sleepMS(50)
			rawExecve("/nonexistent/c")
		}()
		say("a=%v", rawExecve(args[1]))
		sleepMS(200)
		os.Exit(0) // exit_group kills the thread held in its notification
	case "signal":
		sigs := make(chan os.Signal, 4)
		signal.Notify(sigs, syscall.SIGUSR1)
		pid, tid := syscall.Getpid(), syscall.Gettid()
		go func() {
			sleepMS(50)
			syscall.Tgkill(pid, tid, syscall.SIGUSR1)
		}()
		say("sig=%v", rawExecve("/nonexistent/sig"))
		return 0
	case "overlap":
		done := make(chan struct{})
		go func() {
			runtime.LockOSThread()
			say("a1=%v", rawExecve("/nonexistent/a1"))
			close(done)
		}()
		<-done
		done = make(chan struct{})
		go func() {
			runtime.LockOSThread()
			say("b1=%v", rawExecve("/nonexistent/b1"))
			close(done)
		}()
		<-done
		err := syscall.Exec(args[1], []string{args[1]}, env)
		say("overlap-exec=%v", err)
		return 1
	case "misc":
		_, _, e := syscall.Syscall(syscall.SYS_EXECVE, 0, 0, 0)
		say("null=%v", e)
		say("missing=%v", rawExecve("/nonexistent/x"))
		bad := []byte{'/', 't', 'm', 'p', '/', 0xff, 0}
		_, _, e = syscall.Syscall(syscall.SYS_EXECVE, uintptr(unsafe.Pointer(&bad[0])), 0, 0)
		say("nonutf8=%v", e)
		fd, err := syscall.Open("/usr/bin/true", syscall.O_RDONLY|syscall.O_CLOEXEC, 0)
		if err != nil {
			say("open=%v", err)
			return 1
		}
		empty := []byte{0}
		prog, _ := syscall.BytePtrFromString("true")
		argv := []*byte{prog, nil}
		envv := []*byte{nil}
		_, _, e = syscall.Syscall6(322, uintptr(fd), uintptr(unsafe.Pointer(&empty[0])), uintptr(unsafe.Pointer(&argv[0])), uintptr(unsafe.Pointer(&envv[0])), 0x1000 /* AT_EMPTY_PATH */, 0)
		say("fexecve=%v", e)
		return 1
	case "execveat":
		mode, dir, name := args[1], args[2], args[3]
		fd, err := syscall.Open(dir, syscall.O_RDONLY|syscall.O_DIRECTORY|syscall.O_CLOEXEC, 0)
		if err != nil {
			say("open=%v", err)
			return 1
		}
		dirfd := uintptr(fd)
		switch mode {
		case "high":
			dirfd |= 1 << 32 // the kernel's dirfd is an int: the high bits are ignored
		case "cwd":
			if err := syscall.Chdir(dir); err != nil {
				say("chdir=%v", err)
				return 1
			}
			dirfd = ^uintptr(99) // AT_FDCWD, -100
		}
		p, _ := syscall.BytePtrFromString(name)
		probe, _ := syscall.BytePtrFromString(execHeadProbeCommand)
		hold, _ := syscall.BytePtrFromString("hold")
		argv := []*byte{p, probe, hold, nil}
		envv := []*byte{nil}
		_, _, e := syscall.Syscall6(322, dirfd, uintptr(unsafe.Pointer(p)), uintptr(unsafe.Pointer(&argv[0])), uintptr(unsafe.Pointer(&envv[0])), 0, 0)
		say("execveat=%v", e)
		return 1
	case "nonleader":
		done := make(chan error, 1)
		go func() {
			runtime.LockOSThread() // a thread of its own, which is not the leader
			done <- syscall.Exec(args[1], args[1:], env)
		}()
		err := <-done
		say("nonleader=%v", err)
		return 1
	}
	say("unknown mode %q", args[0])
	return 2
}
