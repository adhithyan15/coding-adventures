//go:build linux && amd64

package main

// S2 (KILN04 section 4.7): --stdout-fd and --stderr-fd, end to end.

import (
	"bytes"
	"context"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// outFDRun runs `sandbox-exec` with the given extra files as descriptors 3, 4,
// ... of the root, plus args between the config and the `--`.  It returns the
// root's own standard output and error, and what was read from each extra file
// (when it is a pipe's read end the caller passes in `readers`).
func (f *sandboxFixture) outFDRun(t *testing.T, flags []string, extra []*os.File, argv ...string) (out, errb string, code int) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 45*time.Second)
	defer cancel()
	args := append([]string{sandbox.ExecCommand, "--config", f.cfg}, flags...)
	args = append(append(args, "--"), argv...)
	cmd := exec.CommandContext(ctx, f.exe, args...)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"})
	cmd.ExtraFiles = extra
	var ob, eb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &ob, &eb
	err := cmd.Run()
	if ee, ok := err.(*exec.ExitError); ok {
		code = ee.ExitCode()
	} else if err != nil {
		t.Fatalf("%v\nstderr: %s", err, eb.String())
	}
	return ob.String(), eb.String(), code
}

// TestSandboxOutputDescriptors: what the step writes to its descriptors 1 and 2
// lands in the two pipes named by --stdout-fd and --stderr-fd, the root's own
// standard output and error stay empty (so a recorder can check that strace
// wrote nothing), the step has no other descriptor, and a descriptor that is
// not a writable pipe is refused before anything runs.
func TestSandboxOutputDescriptors(t *testing.T) {
	f := newSandboxFixture(t)

	// The two pipes, as a recorder makes them.
	outR, outW, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	errR, errW, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	read := func(r *os.File) <-chan string {
		ch := make(chan string, 1)
		go func() { b, _ := io.ReadAll(r); ch <- string(b) }()
		return ch
	}
	gotOut, gotErr := read(outR), read(errR)
	script := `echo to-out; echo to-err >&2; ` +
		`test -p /dev/fd/1 && echo out-is-pipe; test -p /dev/fd/2 && echo err-is-pipe >&2; ` +
		`ls /proc/self/fd | sort -n | tr '\n' ' ' | sed 's/^/fds:/'; echo; ` +
		`readlink /proc/self/fd/0`
	out, errb, code := f.outFDRun(t, []string{"--stdout-fd", "3", "--stderr-fd", "4"}, []*os.File{outW, errW}, "sh", "-c", script)
	outW.Close()
	errW.Close() // the parent's copies; the sandbox's are closed when it is done
	if code != 0 {
		t.Fatalf("exit %d, stderr %q", code, errb)
	}
	if out != "" || errb != "" {
		t.Errorf("the root's own streams are not empty: stdout %q, stderr %q", out, errb)
	}
	select {
	case s := <-gotOut:
		if !strings.Contains(s, "to-out\n") || !strings.Contains(s, "out-is-pipe\n") || strings.Contains(s, "to-err") {
			t.Errorf("stdout pipe: %q", s)
		}
		// 0, 1, 2 and ls's own directory descriptor: nothing from the root.
		fds := strings.Fields(strings.TrimPrefix(strings.SplitN(s, "\n", 4)[2], "fds:"))
		for _, n := range fds {
			if v, _ := strconv.Atoi(n); v > 3 {
				t.Errorf("descriptor %d reached the step: %q", v, s)
			}
		}
	case <-time.After(30 * time.Second):
		t.Fatal("no end of file on the stdout pipe: a descriptor leaked to a process that is still alive")
	}
	select {
	case s := <-gotErr:
		if !strings.Contains(s, "to-err\n") || !strings.Contains(s, "err-is-pipe\n") || strings.Contains(s, "to-out") {
			t.Errorf("stderr pipe: %q", s)
		}
	case <-time.After(30 * time.Second):
		t.Fatal("no end of file on the stderr pipe")
	}

	// One pipe for both: the same descriptor may be named twice.
	both, bothW, _ := os.Pipe()
	gotBoth := read(both)
	_, errb, code = f.outFDRun(t, []string{"--stdout-fd", "3", "--stderr-fd", "3"}, []*os.File{bothW}, "sh", "-c", "echo a; echo b >&2")
	bothW.Close()
	if code != 0 || errb != "" {
		t.Errorf("same pipe twice: exit %d, stderr %q", code, errb)
	}
	if s := <-gotBoth; s != "a\nb\n" {
		t.Errorf("same pipe twice: %q", s)
	}

	// Refusals: each exits 125 with a message on the root's stderr, and the
	// step never runs (it would have created a file).
	marker := filepath.Join(f.pkg, "ran")
	notRun := []string{"sh", "-c", "touch " + marker}
	pr, pw, _ := os.Pipe()
	defer pr.Close()
	defer pw.Close()
	reg, err := os.Create(filepath.Join(f.base, "regular"))
	if err != nil {
		t.Fatal(err)
	}
	defer reg.Close()
	dir, err := os.Open(f.base)
	if err != nil {
		t.Fatal(err)
	}
	defer dir.Close()
	null, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err != nil {
		t.Fatal(err)
	}
	defer null.Close()
	fifo := filepath.Join(f.base, "rw.fifo")
	if err := syscall.Mkfifo(fifo, 0o600); err != nil {
		t.Fatal(err)
	}
	rwFifo, err := os.OpenFile(fifo, os.O_RDWR, 0) // does not block: it is both ends
	if err != nil {
		t.Fatal(err)
	}
	defer rwFifo.Close()
	// Both flags are always given (a lone one is refused first, L6), the bad
	// descriptor to one and a valid write-only pipe to the other, in both
	// positions, so each flag's own check is reached.
	type refusal struct {
		name string
		bad  *os.File // descriptor 3, or nil
		badN string   // the number given when bad is nil
		want string
	}
	for _, r := range []refusal{
		{"regular file", reg, "", "a regular file"},
		{"directory", dir, "", "a directory"},
		{"/dev/null", null, "", "character device"},
		{"pipe read end", pr, "", "not open write-only"},
		{"read-write FIFO (L4)", rwFifo, "", "not open write-only"},
		{"not open", nil, "9", "not an open descriptor"},
		{"standard stream", nil, "1", "out of range"},
	} {
		for _, stdoutBad := range []bool{true, false} {
			bad, good := "3", "4"
			extra := []*os.File{r.bad, pw}
			if r.bad == nil {
				bad, extra = r.badN, []*os.File{pw}
				good = "3"
			}
			flag, args := "--stderr-fd", []string{"--stdout-fd", good, "--stderr-fd", bad}
			if stdoutBad {
				flag, args = "--stdout-fd", []string{"--stdout-fd", bad, "--stderr-fd", good}
			}
			out, errb, code := f.outFDRun(t, args, extra, notRun...)
			if code != sandbox.ExitFailure || out != "" || !strings.Contains(errb, r.want) || !strings.Contains(errb, flag) {
				t.Errorf("%s (bad %s): exit %d, stdout %q, stderr %q; want 125, %q and %q", r.name, flag, code, out, errb, r.want, flag)
			}
		}
	}
	// A lone flag is refused before any descriptor is looked at (L6).
	for _, args := range [][]string{{"--stdout-fd", "3"}, {"--stderr-fd", "3"}} {
		out, errb, code := f.outFDRun(t, args, []*os.File{pw}, notRun...)
		if code != sandbox.ExitFailure || out != "" || !strings.Contains(errb, "together or not at all") {
			t.Errorf("%v: exit %d, stdout %q, stderr %q", args, code, out, errb)
		}
	}
	if _, err := os.Stat(marker); err == nil {
		t.Error("a refused invocation still ran the step")
	}
}
