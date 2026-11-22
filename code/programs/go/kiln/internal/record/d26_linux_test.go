//go:build linux

package record

// D26 on the real kernel: the sealed value (this process's own start time) and
// the comparands (other processes' start times) are one field of one file, in
// one unit, on one epoch. The tests below pin that, which no fake /proc can.

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
)

// startOfChild starts `sleep 1` and returns its start time as the S3 gate reads
// it, /proc/<pid>/stat through startTicks.
func startOfChild(t *testing.T) int64 {
	t.Helper()
	child := exec.Command("sleep", "1")
	if err := child.Start(); err != nil {
		t.Skipf("no sleep: %v", err)
	}
	t.Cleanup(func() { child.Process.Kill(); child.Wait() })
	stat, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", child.Process.Pid))
	if err != nil {
		t.Fatal(err)
	}
	got, err := startTicks(stat)
	if err != nil {
		t.Fatal(err)
	}
	return got
}

func TestSelfStartTicksAndAChildAreOnTheSameClock(t *testing.T) {
	self, err := ReadSelfStartTicks()
	if err != nil {
		t.Fatal(err)
	}
	got := startOfChild(t)
	// The child started after we did, and not by a wild margin: a test binary
	// lives seconds, not minutes, before it forks `sleep`. A units or epoch
	// mismatch (seconds against ticks, uptime against boot) would miss by far
	// more than this.
	if got < self {
		t.Fatalf("a child started at tick %d, before its parent at %d", got, self)
	}
	if got-self > 30*60*ClockTicksPerSecond {
		t.Fatalf("a child started %d ticks after its parent", got-self)
	}
	if again, _ := ReadSelfStartTicks(); again != self {
		t.Fatalf("our own start time changed: %d then %d", self, again)
	}
}

// The S3 bound derived from our own start, run through the comparison collect
// itself uses (scanSurvivors, over a fake /proc holding REAL stat files): a
// process started after us is counted, and pid 1, when it is older than the
// bound, is excluded.
func TestStartBoundSeparatesBeforeFromAfterOnTheRealKernel(t *testing.T) {
	self, err := ReadSelfStartTicks()
	if err != nil {
		t.Fatal(err)
	}
	q := surQuery
	q.SinceTicks = self - SurvivorSlackTicks
	denied := &os.PathError{Op: "readlink", Path: "x", Err: syscall.EACCES}

	// An uninspectable process of ours is S3's case: flagged only when new.
	scan := func(stat []byte) []string {
		f := host()
		f.procs[30] = &fakeEntry{owner: 1000, cmdline: "x\x00", nsErr: denied, stat: string(stat)}
		got, err := scanSurvivors(f, q)
		if err != nil {
			t.Fatal(err)
		}
		return got
	}

	child := exec.Command("sleep", "1")
	if err := child.Start(); err != nil {
		t.Skipf("no sleep: %v", err)
	}
	t.Cleanup(func() { child.Process.Kill(); child.Wait() })
	cstat, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", child.Process.Pid))
	if err != nil {
		t.Fatal(err)
	}
	if got := scan(cstat); len(got) != 1 || !strings.Contains(got[0], "S3") {
		t.Fatalf("a process started after prepare was not counted: %v", got)
	}

	init1, err := os.ReadFile("/proc/1/stat")
	if err != nil {
		t.Log("cannot read /proc/1/stat; the old-process half is not checked")
		return
	}
	if t1, err := startTicks(init1); err != nil || t1 >= q.SinceTicks {
		t.Logf("pid 1 is not older than the bound (start %d, bound %d, %v); the old-process half is not checked", t1, q.SinceTicks, err)
		return
	}
	if got := scan(init1); len(got) != 0 {
		t.Fatalf("pid 1, older than prepare, was counted: %v", got)
	}
}
