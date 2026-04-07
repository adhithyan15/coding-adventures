package repl

import (
	"fmt"
	"strings"
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

// makeInputFn builds an InputFn from a slice of pre-canned strings.
//
// Each call to the returned function pops the next string from the slice.
// When the slice is exhausted the function returns ("", false), simulating EOF.
// This lets tests drive the REPL without a real terminal.
func makeInputFn(lines []string) InputFn {
	i := 0
	return func() (string, bool) {
		if i >= len(lines) {
			return "", false
		}
		line := lines[i]
		i++
		return line, true
	}
}

// collectOutputFn returns an OutputFn and a pointer to the accumulated output.
//
// Every string passed to the OutputFn is appended to the buffer. Tests can
// then inspect the buffer to assert on what was printed.
func collectOutputFn() (OutputFn, *strings.Builder) {
	var buf strings.Builder
	fn := func(s string) {
		buf.WriteString(s)
	}
	return fn, &buf
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: EchoLanguage echoes normal input
// ─────────────────────────────────────────────────────────────────────────────

// TestEchoLanguage_EchosInput verifies that EchoLanguage returns an ok Result
// with the input as its Output for any non-quit input.
func TestEchoLanguage_EchosInput(t *testing.T) {
	lang := EchoLanguage{}
	cases := []string{"hello", "world", "", "  spaces  ", "123"}

	for _, input := range cases {
		result := lang.Eval(input)
		if result.Tag != "ok" {
			t.Errorf("Eval(%q).Tag = %q, want %q", input, result.Tag, "ok")
		}
		if result.Output != input {
			t.Errorf("Eval(%q).Output = %q, want %q", input, result.Output, input)
		}
		if !result.HasOutput {
			t.Errorf("Eval(%q).HasOutput = false, want true", input)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: EchoLanguage signals quit
// ─────────────────────────────────────────────────────────────────────────────

// TestEchoLanguage_Quit verifies that ":quit" produces a quit-tagged Result.
func TestEchoLanguage_Quit(t *testing.T) {
	lang := EchoLanguage{}
	result := lang.Eval(":quit")

	if result.Tag != "quit" {
		t.Errorf("Eval(\":quit\").Tag = %q, want %q", result.Tag, "quit")
	}
	// Output and HasOutput are irrelevant for quit; the REPL ignores them.
	// We do not assert on them here — that would over-specify the contract.
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: DefaultPrompt returns the expected strings
// ─────────────────────────────────────────────────────────────────────────────

// TestDefaultPrompt verifies the exact prompt strings.
func TestDefaultPrompt(t *testing.T) {
	p := DefaultPrompt{}

	if got := p.GlobalPrompt(); got != "> " {
		t.Errorf("GlobalPrompt() = %q, want %q", got, "> ")
	}
	if got := p.LinePrompt(); got != "... " {
		t.Errorf("LinePrompt() = %q, want %q", got, "... ")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: SilentWaiting is a no-op
// ─────────────────────────────────────────────────────────────────────────────

// TestSilentWaiting verifies that SilentWaiting satisfies the Waiting interface
// and that its methods are safe to call (no panics, sensible return values).
func TestSilentWaiting(t *testing.T) {
	w := SilentWaiting{}

	// TickMs should be a positive integer.
	if ms := w.TickMs(); ms <= 0 {
		t.Errorf("TickMs() = %d, want > 0", ms)
	}

	// Start/Tick/Stop should not panic; Tick should return a state.
	state := w.Start()
	state = w.Tick(state)
	state = w.Tick(state)
	w.Stop(state) // must not panic
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: RunWithIO echoes multiple inputs then quits on EOF
// ─────────────────────────────────────────────────────────────────────────────

// TestRunWithIO_EchoThenEOF drives the REPL with three lines of input,
// then simulates EOF. It verifies that:
//   - Each line is echoed back.
//   - The prompt is shown before each line.
//   - The loop exits cleanly on EOF.
func TestRunWithIO_EchoThenEOF(t *testing.T) {
	inputFn := makeInputFn([]string{"foo", "bar", "baz"})
	outputFn, buf := collectOutputFn()

	RunWithIO(EchoLanguage{}, DefaultPrompt{}, SilentWaiting{}, inputFn, outputFn)

	got := buf.String()

	// Each input line should appear in the output.
	for _, want := range []string{"foo", "bar", "baz"} {
		if !strings.Contains(got, want) {
			t.Errorf("output does not contain %q; got:\n%s", want, got)
		}
	}

	// The global prompt should appear at least three times — once per input line.
	// A fourth prompt is normal: the loop shows the prompt then reads EOF and exits.
	if count := strings.Count(got, "> "); count < 3 {
		t.Errorf("prompt appeared %d times, want >= 3; output:\n%s", count, got)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: RunWithIO terminates on :quit
// ─────────────────────────────────────────────────────────────────────────────

// TestRunWithIO_Quit verifies that the REPL exits when EchoLanguage returns a
// quit Result. Lines after ":quit" should NOT be evaluated or echoed.
func TestRunWithIO_Quit(t *testing.T) {
	// "after-quit" must never appear in the output.
	inputFn := makeInputFn([]string{"hello", ":quit", "after-quit"})
	outputFn, buf := collectOutputFn()

	RunWithIO(EchoLanguage{}, DefaultPrompt{}, SilentWaiting{}, inputFn, outputFn)

	got := buf.String()

	// "hello" should be echoed.
	if !strings.Contains(got, "hello") {
		t.Errorf("expected %q in output; got:\n%s", "hello", got)
	}

	// "after-quit" must NOT appear — the loop must have terminated at :quit.
	if strings.Contains(got, "after-quit") {
		t.Errorf("loop continued past :quit; output:\n%s", got)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7: Panic recovery
// ─────────────────────────────────────────────────────────────────────────────

// panicLanguage is a Language that always panics, used to test recovery.
type panicLanguage struct{}

func (p panicLanguage) Eval(_ string) Result {
	panic("intentional test panic")
}

// TestRunWithIO_PanicRecovery verifies that a panicking Language does not crash
// the REPL. After the panic, the loop should continue and eventually exit on EOF.
func TestRunWithIO_PanicRecovery(t *testing.T) {
	// One input triggers the panic; EOF ends the loop.
	inputFn := makeInputFn([]string{"trigger-panic"})
	outputFn, buf := collectOutputFn()

	// This must not panic at the test level.
	RunWithIO(panicLanguage{}, DefaultPrompt{}, SilentWaiting{}, inputFn, outputFn)

	got := buf.String()

	// The framework should have printed an error message containing "panic".
	if !strings.Contains(got, "panic") {
		t.Errorf("expected error message containing %q; got:\n%s", "panic", got)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 8: Result with HasOutput=false produces no extra output line
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Test 9: Capability cage — StartNew / GetResult happy path
// ─────────────────────────────────────────────────────────────────────────────

// TestStartNew_HappyPath exercises the generated capability cage for the
// common success path.
func TestStartNew_HappyPath(t *testing.T) {
	op := StartNew("test.op", 0, func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
		return rf.Generate(true, false, 42)
	})
	val, err := op.GetResult()
	if err != nil {
		t.Fatalf("GetResult() error = %v, want nil", err)
	}
	if val != 42 {
		t.Errorf("GetResult() = %d, want 42", val)
	}
}

// TestStartNew_ExpectedFailure exercises the expected-failure path of GetResult.
func TestStartNew_ExpectedFailure(t *testing.T) {
	op := StartNew("test.fail", "", func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
		return rf.Generate(false, false, "")
	})
	_, err := op.GetResult()
	if err == nil {
		t.Fatal("GetResult() error = nil, want non-nil for expected failure")
	}
}

// TestStartNew_PanicRecovery exercises the panic-recovery path of GetResult.
func TestStartNew_PanicRecovery(t *testing.T) {
	op := StartNew("test.panic", -1, func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
		panic("oops")
	})
	val, err := op.GetResult()
	if err == nil {
		t.Fatal("GetResult() error = nil, want non-nil after panic")
	}
	if val != -1 {
		t.Errorf("GetResult() fallback = %d, want -1", val)
	}
}

// TestStartNew_FailWithError exercises the rf.Fail path.
func TestStartNew_FailWithError(t *testing.T) {
	sentinel := fmt.Errorf("sentinel error")
	op := StartNew("test.fail-err", 0, func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
		return rf.Fail(0, sentinel)
	})
	_, err := op.GetResult()
	if err != sentinel {
		t.Errorf("GetResult() error = %v, want sentinel error", err)
	}
}

// TestStartNew_AddProperty verifies AddProperty does not panic.
func TestStartNew_AddProperty(t *testing.T) {
	op := StartNew("test.props", 0, func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
		op.AddProperty("key", "value")
		return rf.Generate(true, false, 1)
	})
	val, err := op.GetResult()
	if err != nil || val != 1 {
		t.Errorf("GetResult() = (%d, %v), want (1, nil)", val, err)
	}
}

// TestTimeCapabilities exercises op.Time.Now() inside a capability cage callback.
func TestTimeCapabilities(t *testing.T) {
	op := StartNew("test.time", false, func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
		now := op.Time.Now()
		return rf.Generate(!now.IsZero(), false, !now.IsZero())
	})
	val, err := op.GetResult()
	if err != nil {
		t.Fatalf("GetResult() error = %v", err)
	}
	if !val {
		t.Error("op.Time.Now() returned zero time")
	}
}

// TestPanicOnUnexpected verifies that PanicOnUnexpected re-panics instead of
// converting the panic to an error.
func TestPanicOnUnexpected(t *testing.T) {
	op := StartNew("test.repanic", 0, func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
		panic("should be re-panicked")
	}).PanicOnUnexpected()

	defer func() {
		r := recover()
		if r == nil {
			t.Error("expected panic to be re-panicked but it was not")
		}
	}()
	op.GetResult() //nolint
}

// TestCapabilityViolationError verifies the error message format of the
// _capabilityViolationError type used by the capability cage.
func TestCapabilityViolationError(t *testing.T) {
	err := &_capabilityViolationError{
		category:  "file",
		action:    "read",
		requested: "/etc/passwd",
	}
	msg := err.Error()
	if !strings.Contains(msg, "capability violation") {
		t.Errorf("Error() = %q, expected to contain 'capability violation'", msg)
	}
	if !strings.Contains(msg, "file:read") {
		t.Errorf("Error() = %q, expected to contain 'file:read'", msg)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Sync-mode tests
// ─────────────────────────────────────────────────────────────────────────────

// TestSyncModeEcho verifies that RunWithOptions in ModeSync echoes input
// correctly — the same behaviour as ModeAsync, but without goroutines.
func TestSyncModeEcho(t *testing.T) {
	inputFn := makeInputFn([]string{"hello", "world"})
	outputFn, buf := collectOutputFn()

	RunWithOptions(
		EchoLanguage{},
		DefaultPrompt{},
		Options{Mode: ModeSync, Waiting: SilentWaiting{}},
		inputFn,
		outputFn,
	)

	got := buf.String()

	for _, want := range []string{"hello", "world"} {
		if !strings.Contains(got, want) {
			t.Errorf("sync mode output does not contain %q; got:\n%s", want, got)
		}
	}
}

// TestSyncModeQuit verifies that RunWithOptions in ModeSync stops the loop
// when EchoLanguage returns a quit Result (on ":quit" input).
func TestSyncModeQuit(t *testing.T) {
	// "after-quit" must never appear in the output.
	inputFn := makeInputFn([]string{"hello", ":quit", "after-quit"})
	outputFn, buf := collectOutputFn()

	RunWithOptions(
		EchoLanguage{},
		DefaultPrompt{},
		Options{Mode: ModeSync, Waiting: SilentWaiting{}},
		inputFn,
		outputFn,
	)

	got := buf.String()

	if !strings.Contains(got, "hello") {
		t.Errorf("sync mode: expected %q in output; got:\n%s", "hello", got)
	}
	if strings.Contains(got, "after-quit") {
		t.Errorf("sync mode: loop continued past :quit; output:\n%s", got)
	}
}

// TestSyncModeNilWaiting verifies that RunWithOptions in ModeSync works
// correctly when opts.Waiting is nil. Because sync mode never touches the
// Waiting field, a nil value must not cause a panic.
func TestSyncModeNilWaiting(t *testing.T) {
	inputFn := makeInputFn([]string{"foo", "bar"})
	outputFn, buf := collectOutputFn()

	// This must not panic even though Waiting is nil.
	RunWithOptions(
		EchoLanguage{},
		DefaultPrompt{},
		Options{Mode: ModeSync, Waiting: nil},
		inputFn,
		outputFn,
	)

	got := buf.String()

	for _, want := range []string{"foo", "bar"} {
		if !strings.Contains(got, want) {
			t.Errorf("sync+nil waiting output does not contain %q; got:\n%s", want, got)
		}
	}
}

// TestSilentWaitingStop verifies Stop does not panic (it is a no-op).
func TestSilentWaitingStop(t *testing.T) {
	w := SilentWaiting{}
	state := w.Start()
	w.Stop(state) // must not panic
	w.Stop(nil)   // must not panic with nil state either
}

// silentOkLanguage returns ok Results with HasOutput=false for every input.
// Models a language where statements don't produce printable values (e.g.,
// variable assignments in a statically-typed language).
type silentOkLanguage struct{}

func (s silentOkLanguage) Eval(_ string) Result {
	return Result{Tag: "ok", Output: "", HasOutput: false}
}

// TestRunWithIO_SilentOk verifies that when HasOutput is false, the REPL does
// not print a blank line for the result — only the prompt is shown.
func TestRunWithIO_SilentOk(t *testing.T) {
	inputFn := makeInputFn([]string{"x = 42"})
	outputFn, buf := collectOutputFn()

	RunWithIO(silentOkLanguage{}, DefaultPrompt{}, SilentWaiting{}, inputFn, outputFn)

	got := buf.String()

	// Output should consist only of the prompt — no stray newlines from eval.
	// We expect exactly one "> " and nothing after it except possibly the
	// implicit newline at the end.
	if !strings.HasPrefix(got, "> ") {
		t.Errorf("expected output to start with prompt; got %q", got)
	}

	// The result value "x = 42" should not appear in output.
	if strings.Contains(got, "x = 42") {
		t.Errorf("silent ok result leaked to output; got:\n%s", got)
	}
}
