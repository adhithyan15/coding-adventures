//go:build unix

package record

// Regression tests from the review of `kiln record step` (KILN04 4.8, P12):
// parallel first steps, rerun markers whatever the step numbers, the record
// directory's lock, strace's own pipes, LD_LIBRARY_PATH, the saved strace
// stderr and the cross-check of the action files against the steps export.

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

// The build tool runs independent packages in parallel, so supervisors of
// DIFFERENT actions start together, and the first of them used to race to
// create traces/: one Mkdir won and the others got EEXIST and were refused (895
// of 1200 in the review's proof of concept), which voids a shard.
func TestParallelFirstStepsDoNotRaceOnTheTracesDirectory(t *testing.T) {
	const rounds = 300
	var mu sync.Mutex
	lost := 0
	for r := 0; r < rounds; r++ {
		d := canonTemp(t)
		var wg sync.WaitGroup
		start := make(chan struct{})
		for i := 0; i < 4; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				<-start
				f, err := createTrace(d, "pkg/"+string(rune('a'+i)), 0)
				if err == nil {
					f.Close()
				} else {
					mu.Lock()
					lost++
					mu.Unlock()
					t.Logf("refused: %v", err)
				}
			}(i)
		}
		close(start)
		wg.Wait()
	}
	if lost > 0 {
		t.Fatalf("%d of %d parallel first steps refused", lost, rounds*4)
	}
}

// The same through RunStep, with traces/ absent (as an older prepare left it)
// and three actions started together.
func TestParallelRunStepsOfDifferentActions(t *testing.T) {
	f := newStepFix(t)
	if err := os.Remove(filepath.Join(f.p.dir, "traces")); err != nil {
		t.Fatal(err)
	}
	var wg sync.WaitGroup
	errs := make([]error, 3)
	for i, a := range []string{actA, actB, actZ} {
		wg.Add(1)
		count := 1
		if a == actB {
			count = 2
		}
		go func(i int, a string, count int) {
			defer wg.Done()
			o := f.opts(a, 0, count, "exit 0")
			o.Stdout, o.Stderr = nil, nil
			_, errs[i] = RunStep(o)
		}(i, a, count)
	}
	wg.Wait()
	for i, e := range errs {
		if e != nil {
			t.Errorf("action %d: %v", i, e)
		}
	}
}

// prepare makes traces/ itself, 0700, outside the sealed pre/.
func TestPrepareCreatesTheTracesDirectory(t *testing.T) {
	f := newStepFix(t)
	st, err := os.Lstat(filepath.Join(f.p.dir, "traces"))
	if err != nil || !st.IsDir() || st.Mode().Perm() != 0o700 {
		t.Fatalf("%v %v", st, err)
	}
	if _, ok := f.pre.Sums["traces"]; ok {
		t.Fatal("traces/ is sealed")
	}
}

func TestEnsureDir(t *testing.T) {
	d := canonTemp(t)
	p := filepath.Join(d, "x")
	for i := 0; i < 2; i++ { // created, then found
		if err := ensureDir(p); err != nil {
			t.Fatal(err)
		}
	}
	if err := ensureDir(filepath.Join(d, "no", "parent")); err == nil || errors.Is(err, os.ErrExist) {
		t.Fatalf("%v", err)
	}
}

// A rerun or out-of-order refusal leaves the marker whatever the step numbers
// are: ReadDir sorts "10.json" before "2.json", which once made an early return
// skip it.
func TestRerunWithoutMarker(t *testing.T) {
	d := canonTemp(t)
	act := "pkg/a"
	for _, step := range []int{2, 10} {
		f, err := createTrace(d, act, step)
		if err != nil {
			t.Fatal(err)
		}
		f.Close()
	}
	for j := 0; j < 2; j++ {
		writeBytes(t, StepFile(d, act, j, "json"), []byte("{}"), 0o600)
	}
	err := checkOrder(d, act, 2)
	if err == nil || !strings.Contains(err.Error(), "already run") {
		t.Fatalf("%v", err)
	}
	if !exists(StepFile(d, act, 2, "rerun")) {
		t.Fatal("step 2 was asked to run twice and no .rerun marker was left")
	}
	if exists(StepFile(d, act, 10, "rerun")) {
		t.Fatal("a marker for a step that was not asked twice")
	}
}

func TestOutOfOrderLeavesTheMarkerToo(t *testing.T) {
	d := canonTemp(t)
	act := "pkg/a"
	f, err := createTrace(d, act, 12)
	if err != nil {
		t.Fatal(err)
	}
	f.Close()
	for j := 0; j < 3; j++ {
		writeBytes(t, StepFile(d, act, j, "json"), []byte("{}"), 0o600)
	}
	if err := checkOrder(d, act, 3); err == nil || !strings.Contains(err.Error(), "later step") {
		t.Fatalf("%v", err)
	}
	if !exists(StepFile(d, act, 3, "rerun")) {
		t.Fatal("no marker")
	}
	// Steps compare as numbers: step 9 is not "after" step 10 by spelling.
	d2 := canonTemp(t)
	for j := 0; j < 10; j++ {
		writeBytes(t, StepFile(d2, act, j, "json"), []byte("{}"), 0o600)
	}
	if err := checkOrder(d2, act, 10); err != nil {
		t.Fatalf("step 10 after steps 0 to 9: %v", err)
	}
}

// Two supervisors of the same step: the one that loses the O_EXCL leaves the
// marker, and exactly one of them runs.
func TestRacingSupervisorsOfOneStep(t *testing.T) {
	d := canonTemp(t)
	var wg sync.WaitGroup
	var mu sync.Mutex
	won, lost := 0, 0
	start := make(chan struct{})
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			<-start
			f, err := createTrace(d, "pkg/a", 0)
			mu.Lock()
			defer mu.Unlock()
			if err == nil {
				won++
				f.Close()
			} else if errors.Is(err, os.ErrExist) {
				lost++
			}
		}()
	}
	close(start)
	wg.Wait()
	if won != 1 || lost != 7 || !exists(StepFile(d, "pkg/a", 0, "rerun")) {
		t.Fatalf("won %d lost %d", won, lost)
	}
}

func TestStepRefusesWhileCollectHoldsTheDirectory(t *testing.T) {
	f := newStepFix(t)
	unlock, err := LockDir(f.p.dir, true)
	if err != nil {
		t.Fatal(err)
	}
	f.refused(f.opts(actZ, 0, 1, "exit 0"), "cannot be locked")
	unlock()
	f.must(actZ, 0, 1, "exit 0") // and the lock is gone with the supervisor
	// Shared locks coexist; an exclusive one needs them all gone.
	u1, err := LockDir(f.p.dir, false)
	if err != nil {
		t.Fatal(err)
	}
	u2, err := LockDir(f.p.dir, false)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := LockDir(f.p.dir, true); err == nil {
		t.Fatal("an exclusive lock over shared ones")
	}
	u1()
	u2()
	u3, err := LockDir(f.p.dir, true)
	if err != nil {
		t.Fatal(err)
	}
	u3()
	if _, err := LockDir(filepath.Join(f.p.dir, "absent"), true); err == nil {
		t.Fatal("locked a missing directory")
	}
}

// A step that is running holds the shared lock, so collect (exclusive,
// non-blocking) refuses while it runs.
func TestStepHoldsTheLockWhileItRuns(t *testing.T) {
	f := newStepFix(t)
	seen := make(chan error, 1)
	o := f.opts(actZ, 0, 1, "sleep 1")
	go func() {
		// The step's trace is created under the lock and the step sleeps for a
		// second: ask for the exclusive lock in that window.
		for i := 0; i < 500; i++ {
			if exists(StepFile(f.p.dir, actZ, 0, "trace")) {
				_, err := LockDir(f.p.dir, true)
				seen <- err
				return
			}
			time.Sleep(10 * time.Millisecond)
		}
		seen <- errors.New("the trace never appeared")
	}()
	if _, err := RunStep(o); err != nil {
		t.Fatal(err)
	}
	if err := <-seen; err == nil || strings.Contains(err.Error(), "never appeared") {
		t.Fatalf("collect could lock the directory while a step held it: %v", err)
	}
}

// Security LOW-1: a survivor holding strace's OWN standard error is noticed
// whatever strace's exit status was.
func TestStepASurvivorHoldingStracesOwnPipesIsNoticedEvenWhenStraceFails(t *testing.T) {
	f := newStepFix(t)
	o := f.opts(actZ, 0, 1, "(sleep 6 >/dev/null 2>&2 &); exit 3")
	o.Drain = 300e6
	res, err := RunStep(o)
	if err != nil {
		t.Fatal(err)
	}
	if res.ExitCode != 3 || res.Sidecar.Clean || !containsStr(res.Sidecar.Unclean, UncleanPipesHeld) {
		t.Fatalf("%d %+v", res.ExitCode, res.Sidecar)
	}
}

// The validated LD_LIBRARY_PATH reaches strace, and nothing else of
// build_tool_env does.
func TestStepPassesTheValidatedLoaderPathToStrace(t *testing.T) {
	f := newStepFixEnv(t, "LD_LIBRARY_PATH=/usr/lib")
	var ld string
	for _, e := range f.pre.Tools.BuildToolEnv {
		if strings.HasPrefix(e, "LD_LIBRARY_PATH=") {
			ld = e
		}
	}
	if ld == "" {
		t.Skip("prepare did not keep LD_LIBRARY_PATH here")
	}
	f.must(actZ, 0, 1, "exit 0")
	data, _ := os.ReadFile(f.envf)
	if !strings.Contains(string(data), ld+"\n") {
		t.Fatalf("strace's environment lacks %s:\n%s", ld, data)
	}
	if strings.Contains(string(data), "HOME=") {
		t.Fatalf("the build tool's HOME reached strace:\n%s", data)
	}
}

func TestStepSavesStracesStderr(t *testing.T) {
	f := newStepFix(t)
	f.must(actZ, 0, 1, "echo boom >&2")
	data, err := os.ReadFile(StepFile(f.p.dir, actZ, 0, "strace-stderr"))
	if err != nil || string(data) != "boom\n" {
		t.Fatalf("%q %v", data, err)
	}
	// A clean step leaves none.
	f2 := newStepFix(t)
	f2.must(actZ, 0, 1, "exit 0")
	if exists(StepFile(f2.p.dir, actZ, 0, "strace-stderr")) {
		t.Fatal("a stderr file for a clean step")
	}
}

func TestStepWritesTheTraceThroughThePipe(t *testing.T) {
	f := newStepFix(t)
	res := f.must(actZ, 0, 1, "printf 'second line\\n' >&$n")
	data, err := os.ReadFile(StepFile(f.p.dir, actZ, 0, "trace"))
	if err != nil || string(data) != "fake trace\nsecond line\n" || res.Sidecar.TraceBytes != int64(len(data)) {
		t.Fatalf("%q %v %+v", data, err, res.Sidecar)
	}
}

// VerifyPre cross-checks every action file against the sealed steps export.
func TestVerifyPreChecksActionsAgainstTheStepsExport(t *testing.T) {
	for name, steps := range map[string][]string{
		"another line":   {"exit 1"},
		"one step more":  {"exit 0", "exit 0"},
		"no steps":       {},
		"the wrong line": {"exit 0 "},
	} {
		t.Run(name, func(t *testing.T) {
			f := newStepFix(t)
			a, _ := f.pre.Action(actZ)
			a.Steps = steps
			data, err := EncodeAction(a)
			if err != nil {
				t.Fatal(err)
			}
			f.reseal(map[string][]byte{ActionFile(actZ): data}, false)
			f.refused(f.opts(actZ, 0, 1, "exit 0"), "steps export")
		})
	}
	t.Run("a steps export that does not decode", func(t *testing.T) {
		f := newStepFix(t)
		f.reseal(map[string][]byte{"steps.json": []byte(`{}`)}, true)
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "steps")
	})
	_ = fmt.Sprint
}

// A relay stuck writing to the caller (a pipe nobody reads) must not hold the
// supervisor: after the drain timeout and a short grace it is abandoned, and
// the sidecar says a pipe was held.
func TestStepAnUnreadableCallerDoesNotHoldTheSupervisor(t *testing.T) {
	f := newStepFix(t)
	o := f.opts(actZ, 0, 1, "echo hello >&3")
	block := make(chan struct{})
	defer close(block)
	o.Stdout = blockingWriter{block}
	o.Drain = 300 * time.Millisecond
	start := time.Now()
	res, err := RunStep(o)
	if err != nil {
		t.Fatal(err)
	}
	if time.Since(start) > 10*time.Second {
		t.Fatalf("waited %s", time.Since(start))
	}
	if res.ExitCode != 0 || res.Sidecar.Clean || !containsStr(res.Sidecar.Unclean, UncleanPipesHeld) {
		t.Fatalf("%d %+v", res.ExitCode, res.Sidecar)
	}
	// The trace was complete: the sink was waited for.
	if res.Sidecar.TraceBytes != int64(len("fake trace\n")) {
		t.Fatalf("%+v", res.Sidecar)
	}
}

type blockingWriter struct{ release chan struct{} }

func (b blockingWriter) Write([]byte) (int, error) {
	<-b.release
	return 0, errors.New("released")
}
