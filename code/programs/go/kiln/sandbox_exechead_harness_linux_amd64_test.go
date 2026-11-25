//go:build linux && amd64

package main

// KILN04 D48 step 4, the harness: a REAL supervisor (internal/exechead) against
// a real kernel, with real strace and the real sandbox.
//
// It starts the step the way `kiln record step` does:
//
//	strace -f ... kiln sandbox-exec --config C --stdout-fd 3 --stderr-fd 4
//	                                --exec-notify-fd 5 -- <step>
//
// with the socket pair's child end as strace's descriptor 5 (the harness has no
// trace pipe, so it is the third ExtraFiles entry, where `record step` has the
// fourth: the number is only an argument), and the supervisor's end held by an
// exechead.Supervisor.  It then decodes the `<step>.exechead` file with the
// STRICT decoder PR 5 will use, so every test also checks the header, the
// sequence numbers and every line's shape.
//
// Every child runs in its own process group with a deadline; the group is killed
// at the deadline and at the end of the test.

import (
	"bytes"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"sync/atomic"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// ehOpt are the knobs of one run.
type ehOpt struct {
	timing exechead.Timing
	// noOption passes the socket to strace but NOT --exec-notify-fd to the
	// sandbox: the hand-over is lost, and strace's copy of the socket keeps it open.
	noOption bool
	// abortAt, when positive, calls Abort() on the supervisor after that long.
	abortAt time.Duration
	// limit bounds the whole run (default 60 s).
	limit time.Duration
}

// ehRun is what a run left.
type ehRun struct {
	t       *testing.T
	f       *sandboxFixture
	res     exechead.Result
	file    *exechead.File
	hdr     exechead.Header
	stdout  string
	stderr  string
	exit    int
	elapsed time.Duration
	bin     string // the probe binary in the checkout ({bin} in a script)
}

// ehKernel is `uname -r`.
func ehKernel(t *testing.T) string {
	t.Helper()
	var u syscall.Utsname
	if err := syscall.Uname(&u); err != nil {
		t.Fatal(err)
	}
	var b []byte
	for _, c := range u.Release {
		if c == 0 {
			break
		}
		b = append(b, byte(c))
	}
	return string(b)
}

// ehScript runs `sh -c script` as the step.  "{bin}" in the script is the probe
// binary (this test binary, copied into the checkout), "{tmp}" is $TMPDIR.
func (f *sandboxFixture) ehScript(t *testing.T, script string, o ehOpt) *ehRun {
	t.Helper()
	r := &ehRun{t: t, f: f}
	if strings.Contains(script, "{bin}") {
		r.bin = f.stepBinary(t)
		script = strings.ReplaceAll(script, "{bin}", r.bin)
	}
	script = strings.ReplaceAll(script, "{tmp}", f.tmp)
	return f.ehArgv(t, r, []string{"sh", "-c", script}, o)
}

// ehArgv runs argv as the step.
func (f *sandboxFixture) ehArgv(t *testing.T, r *ehRun, argv []string, o ehOpt) *ehRun {
	t.Helper()
	straceBin, err := exec.LookPath("strace")
	if err != nil {
		skipOrFail(t, "strace is not installed")
	}
	limit := o.limit
	if limit <= 0 {
		limit = 60 * time.Second
	}
	// A directory of its own per run: a test may run several steps.
	runDir, err := os.MkdirTemp(f.base, "eh")
	if err != nil {
		t.Fatal(err)
	}
	traceFile := filepath.Join(runDir, "trace.txt")
	sargs, err := sandbox.StraceArgs(traceFile)
	if err != nil {
		t.Fatal(err)
	}
	var raw [16]byte
	if _, err := rand.Read(raw[:]); err != nil {
		t.Fatal(err)
	}
	action := sha256.Sum256([]byte("pkg"))
	r.hdr = exechead.Header{V: exechead.Version, Kernel: ehKernel(t), Nonce: hex.EncodeToString(raw[:]), ActionSHA256: hex.EncodeToString(action[:]), Step: 0}
	ehPath := filepath.Join(runDir, "0.exechead")
	out, err := os.OpenFile(ehPath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		t.Fatal(err)
	}
	defer out.Close()
	tmpfs := append([]string{f.home, f.tmp}, sandbox.ScratchDirs...)
	sup, err := exechead.New(exechead.Config{Out: out, Header: r.hdr, Tmpfs: tmpfs, Timing: o.timing})
	if err != nil {
		t.Fatal(err)
	}

	sandboxArgs := []string{f.exe, sandbox.ExecCommand, "--config", f.cfg, "--stdout-fd", "3", "--stderr-fd", "4"}
	if !o.noOption {
		sandboxArgs = append(sandboxArgs, "--exec-notify-fd", "5")
	}
	args := append(append(append(sargs, sandboxArgs...), "--"), argv...)

	outR, outW, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	errR, errW, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command(straceBin, args...)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"})
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.ExtraFiles = []*os.File{outW, errW, sup.ChildEnd()}
	var strOut, strErr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &strOut, &strErr
	var pid atomic.Int64
	killGroup := func() {
		if p := int(pid.Load()); p > 0 {
			syscall.Kill(-p, syscall.SIGKILL)
		}
	}
	sup.SetKillStrace(killGroup)
	start := time.Now()
	if err := cmd.Start(); err != nil {
		sup.Finish()
		t.Fatalf("starting strace: %v", err)
	}
	pid.Store(int64(cmd.Process.Pid))
	defer killGroup() // whatever happens, nothing of the step outlives the test
	outW.Close()
	errW.Close()
	sup.Start()

	read := func(f *os.File, into *string) <-chan struct{} {
		done := make(chan struct{})
		go func() {
			b, _ := io.ReadAll(f)
			*into = string(b)
			close(done)
		}()
		return done
	}
	doneOut, doneErr := read(outR, &r.stdout), read(errR, &r.stderr)
	if o.abortAt > 0 {
		time.AfterFunc(o.abortAt, sup.Abort)
	}
	deadline := time.AfterFunc(limit, killGroup)
	defer deadline.Stop()
	waitErr := cmd.Wait()
	timedOut := time.Since(start) >= limit
	r.res = sup.Finish()
	<-doneOut
	<-doneErr
	outR.Close()
	errR.Close()
	r.elapsed = time.Since(start)
	if ee, ok := waitErr.(*exec.ExitError); ok {
		r.exit = ee.ExitCode()
	}
	t.Logf("step stdout: %q\nstep stderr: %q\nstrace stderr: %q\nexit %d, %s; supervisor: %+v", r.stdout, r.stderr, strErr.String(), r.exit, r.elapsed.Round(time.Millisecond), r.res)
	if timedOut {
		t.Fatalf("the run hit its %s deadline", limit)
	}
	data, err := os.ReadFile(ehPath)
	if err != nil {
		t.Fatal(err)
	}
	r.file, err = exechead.Decode(data, exechead.Limits{}, exechead.Expect{Nonce: r.hdr.Nonce, ActionSHA256: r.hdr.ActionSHA256, Step: 0})
	if err != nil {
		t.Fatalf("the sidecar does not decode: %v\n%s", err, clipText(string(data), 4000))
	}
	// What the step sidecar will bind: the digest and length of the whole file.
	if r.file.SHA256 != r.res.SHA256 || r.file.Bytes != r.res.Bytes || len(r.file.Lines) != r.res.Lines {
		t.Fatalf("the supervisor reported %d lines, %d bytes, %s; the file has %d lines, %d bytes, %s",
			r.res.Lines, r.res.Bytes, r.res.SHA256, len(r.file.Lines), r.file.Bytes, r.file.SHA256)
	}
	if r.file.Header.Kernel != ehKernel(t) || r.file.Header.Kernel == "" {
		t.Fatalf("header kernel %q, uname says %q", r.file.Header.Kernel, ehKernel(t))
	}
	for i, l := range r.file.Lines {
		if l.Seq != i {
			t.Fatalf("line %d has seq %d", i, l.Seq)
		}
	}
	r.dump()
	return r
}

func clipText(s string, n int) string {
	if len(s) > n {
		return s[:n] + "..."
	}
	return s
}

// dump logs the lines, heads shortened, so a failure shows what the supervisor saw.
func (r *ehRun) dump() {
	var b strings.Builder
	for _, l := range r.file.Lines {
		b.WriteString(clipText(strings.TrimSpace(formatLine(l)), 220))
		b.WriteByte('\n')
	}
	r.t.Logf("exechead lines:\n%s", b.String())
}

func formatLine(l exechead.Line) string {
	s := strings.Join([]string{strconv.Itoa(l.Seq), "tid", strconv.Itoa(l.Tid), "tgid", strconv.Itoa(l.Tgid), l.Syscall, string(l.Status), l.Path}, " ")
	if l.Status == exechead.StatusConfirmed {
		s += " ino=" + strconv.FormatUint(l.Ino, 10) + " head=" + strconv.Itoa(len(l.Head)) + "B"
	}
	return s
}

// byPath are the lines whose path is exactly p, in order.
func (r *ehRun) byPath(p string) []exechead.Line {
	var out []exechead.Line
	for _, l := range r.file.Lines {
		if l.Path == p {
			out = append(out, l)
		}
	}
	return out
}

// one is the only line with path p; it fails the test otherwise.
func (r *ehRun) one(p string) exechead.Line {
	r.t.Helper()
	ls := r.byPath(p)
	if len(ls) != 1 {
		r.t.Fatalf("%d lines for %q, want exactly 1", len(ls), p)
	}
	return ls[0]
}

// index is the position of the first line with the path and status, or -1.
func (r *ehRun) index(p string, st exechead.Status) int {
	for i, l := range r.file.Lines {
		if l.Path == p && l.Status == st {
			return i
		}
	}
	return -1
}

// none fails if any line has the status.
func (r *ehRun) none(st exechead.Status) {
	r.t.Helper()
	for _, l := range r.file.Lines {
		if l.Status == st {
			r.t.Fatalf("a %s line where none is expected: %s", st, formatLine(l))
		}
	}
}

// clean requires the supervisor's half of the step to be clean.
func (r *ehRun) clean() {
	r.t.Helper()
	if len(r.res.Unclean) != 0 {
		r.t.Fatalf("the supervisor reports the step unclean: %v (%v)", r.res.Unclean, r.res.Notes)
	}
	if !r.res.Received {
		r.t.Fatal("the listener was never received")
	}
}

// unclean requires the reason among the supervisor's.
func (r *ehRun) unclean(reason string) {
	r.t.Helper()
	for _, u := range r.res.Unclean {
		if u == reason {
			return
		}
	}
	r.t.Fatalf("unclean reasons %v do not include %s (%v)", r.res.Unclean, reason, r.res.Notes)
}

// printed returns the value of "key=value" lines the step printed.
func (r *ehRun) printed(key string) []string {
	var out []string
	for _, l := range strings.Split(r.stdout, "\n") {
		if v, ok := strings.CutPrefix(l, key+"="); ok {
			out = append(out, strings.TrimSpace(v))
		}
	}
	return out
}
