//go:build linux && amd64

package main

// The sandbox filter answers clone3 with ENOSYS (KILN04 section 4.3 item 5), so
// glibc falls back to clone.  strace still prints the failed call, and the
// parser used to refuse any clone3 whatever its result, which made every
// threaded tool unrecordable.  The parser now tolerates a clone3 that failed
// with exactly the filter's ENOSYS.  That rests on what strace really prints
// for the call, which no canned trace can show, so this test runs a raw clone3
// as the step, under the recorder's own strace argv, and requires that the trace
// parses (runStraced fails the test, with the lines around it, on a refusal)
// and really holds the failed call.

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"syscall"
	"testing"
	"unsafe"
)

const clone3ProbeCommand = "sandbox-clone3"

// clone3Probe makes one raw clone3 call shaped like glibc's posix_spawn (three
// flags and an exit_signal) and prints its errno.  It runs INSIDE the sandbox,
// as the step.  A child of a call that should have failed exits at once.
func clone3Probe(out *os.File) int {
	var args struct {
		flags, pidfd, childTid, parentTid, exitSignal, stack, stackSize, tls, setTid, setTidSize, cgroup uint64
	}
	const cloneVM, cloneVfork, cloneClearSighand = 0x100, 0x4000, 0x100000000
	args.flags = cloneVM | cloneVfork | cloneClearSighand
	args.exitSignal = uint64(syscall.SIGCHLD)
	r, _, e := syscall.RawSyscall(435, uintptr(unsafe.Pointer(&args)), unsafe.Sizeof(args), 0)
	if e == 0 && r == 0 {
		syscall.RawSyscall(syscall.SYS_EXIT_GROUP, 0, 0, 0)
	}
	out.WriteString("clone3=" + e.Error() + "\n")
	return 0
}

// TestSandboxStraceClone3Enosys: a step that makes a raw clone3 call produces a
// trace that parses and classifies with no refusal, and the trace really holds
// the failed call.
func TestSandboxStraceClone3Enosys(t *testing.T) {
	r := runStracedWith(t, "{pkg}/probe-bin "+clone3ProbeCommand, func(f *sandboxFixture) {
		// The test binary lives under /tmp, which the step does not see: run a
		// copy from inside the checkout.
		exe, err := os.Executable()
		if err != nil {
			t.Fatal(err)
		}
		b, err := os.ReadFile(exe)
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(f.pkg, "probe-bin"), b, 0o755); err != nil {
			t.Fatal(err)
		}
	})
	if !strings.Contains(r.stdout, "clone3=function not implemented") {
		t.Errorf("the step's output lacks the ENOSYS: %q (stderr %q)", r.stdout, r.stderr)
	}
	// The call must be in the trace, or the parse above proved nothing.
	raw, err := os.ReadFile(r.trf)
	if err != nil {
		t.Fatal(err)
	}
	re := regexp.MustCompile(`(?m)clone3\(.*(\) *= -1 ENOSYS|<unfinished \.\.\.>)`)
	if !re.Match(raw) {
		t.Fatalf("the trace has no failed clone3 line:\n%s", r.tail(40))
	}
	t.Logf("clone3 lines: %v", re.FindAllString(string(raw), 3))
	// And it is a failure the parser tolerated, not a refusal it let by.
	for _, e := range r.tr.Events {
		if e.Syscall == "clone3" {
			t.Errorf("a failed clone3 left an event: %+v", e)
		}
	}
}
