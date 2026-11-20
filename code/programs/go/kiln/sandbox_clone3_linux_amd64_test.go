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
	"os/exec"
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
	requireFailedClone3(t, r)
	// And it is a failure the parser tolerated, not a refusal it let by.
	for _, e := range r.tr.Events {
		if e.Syscall == "clone3" {
			t.Errorf("a failed clone3 left an event: %+v", e)
		}
	}
}

var (
	clone3Inline = regexp.MustCompile(`(?m)^\d+ +clone3\(.*\) *= -1 ENOSYS`)
	clone3Open   = regexp.MustCompile(`(?m)^\d+ +clone3\(.*<unfinished \.\.\.>`)
	clone3Resume = regexp.MustCompile(`(?m)^\d+ +<\.\.\. clone3 resumed>.*\) *= -1 ENOSYS`)
)

// requireFailedClone3 fails the test unless the raw trace holds a clone3 that
// failed with ENOSYS: an inline line, or a split one whose "<unfinished ...>"
// half is matched by a "<... clone3 resumed>" half carrying the ENOSYS.  A bare
// unfinished half proves nothing about the filter's answer.
func requireFailedClone3(t *testing.T, r *stracedRun) {
	t.Helper()
	raw, err := os.ReadFile(r.trf)
	if err != nil {
		t.Fatal(err)
	}
	inline, open, resumed := len(clone3Inline.FindAll(raw, -1)), len(clone3Open.FindAll(raw, -1)), len(clone3Resume.FindAll(raw, -1))
	if open > 0 && resumed == 0 {
		t.Fatalf("a split clone3 was opened but no resumed half ends in ENOSYS:\n%s", r.tail(40))
	}
	if inline+resumed == 0 {
		t.Fatalf("the trace has no clone3 that failed with ENOSYS:\n%s", r.tail(40))
	}
	t.Logf("clone3 failed with ENOSYS: %d inline, %d split (%d opened)", inline, resumed, open)
}

// TestSandboxStraceGlibcThreadClone3 runs a real glibc thread creation as the
// step: python3's threading calls pthread_create, which tries clone3 first, so
// strace prints the split form the parser must tolerate, and the thread is then
// made with clone.  python3 is on every runner image (in /usr, a read-only root
// of the sandbox); where it is not, the test skips locally and FAILS under
// KILN_REQUIRE_SANDBOX=1, so it cannot pass vacuously on CI.
func TestSandboxStraceGlibcThreadClone3(t *testing.T) {
	if _, err := exec.LookPath("python3"); err != nil {
		skipOrFail(t, "python3 is not installed")
	}
	r := runStraced(t, "/usr/bin/python3 -S -E -c 'import threading; t=threading.Thread(target=lambda: None); t.start(); t.join(); print(\"threaded\")'")
	if !strings.Contains(r.stdout, "threaded") {
		t.Fatalf("the step did not create its thread: stdout %q stderr %q\n%s", r.stdout, r.stderr, r.tail(30))
	}
	requireFailedClone3(t, r)
}
