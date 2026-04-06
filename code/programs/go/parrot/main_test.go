// Package main — main_test.go
//
// Integration tests for the Parrot REPL program.
//
// # Testing strategy
//
// All tests drive the REPL through injected I/O rather than real stdin/stdout.
// The inputs are fed as a fixed string slice; the outputs are collected into
// another slice. This makes tests deterministic, fast, and side-effect-free:
//
//	inputs []string  →  inputFn  →  RunWithOptions  →  outputFn  →  outputs []string
//
// No terminal emulation, no pipe setup, no goroutine leaks between tests.
//
// # Coverage areas
//
//   - Echo behaviour (single line, multiple lines, empty string, spaces)
//   - Quit handling (:quit stops the loop; subsequent inputs are not processed)
//   - EOF handling (exhausted input slice signals EOF and exits cleanly)
//   - Sync vs Async mode parity (both modes produce the same echo output)
//   - Prompt string content (emoji, ">", trailing space)
//   - Output ordering (echoed lines appear in input order)
//   - Edge cases (only :quit, empty inputs)
package main

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/repl"
)

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

// runParrot drives the Parrot REPL with a fixed slice of inputs in ModeAsync
// and collects all strings passed to outputFn into a returned slice.
//
// The REPL loop ends when the input slice is exhausted (EOF) or when
// EchoLanguage receives ":quit". In most tests we end with ":quit" to
// exercise the explicit-quit path.
func runParrot(t *testing.T, inputs []string) []string {
	t.Helper()
	return runParrotMode(t, inputs, repl.ModeAsync)
}

// runParrotMode is like runParrot but accepts an explicit repl.Mode.
// This lets tests verify that sync and async modes produce identical results.
func runParrotMode(t *testing.T, inputs []string, mode repl.Mode) []string {
	t.Helper()

	var outputs []string
	idx := 0

	// inputFn mimics the user typing one line at a time.
	// When the slice is exhausted it returns ("", false) — the same signal
	// a real bufio.Scanner would produce at end-of-file.
	inputFn := func() (string, bool) {
		if idx >= len(inputs) {
			return "", false
		}
		s := inputs[idx]
		idx++
		return s, true
	}

	// outputFn collects every string the framework passes to "the terminal".
	// We store them all so tests can assert on order and content.
	outputFn := func(text string) {
		outputs = append(outputs, text)
	}

	// Build opts for the mode under test. SilentWaiting is always used so
	// tests do not produce terminal animations.
	opts := repl.Options{
		Mode:    mode,
		Waiting: repl.SilentWaiting{},
	}

	// RunWithOptions lets us choose the mode explicitly while reusing the
	// same ParrotPrompt (defined in prompt.go, same package).
	repl.RunWithOptions(
		repl.EchoLanguage{},
		ParrotPrompt{},
		opts,
		inputFn,
		outputFn,
	)

	return outputs
}

// joinOutputs concatenates all collected output strings into a single string
// for easier contains/prefix assertions.
func joinOutputs(outputs []string) string {
	return strings.Join(outputs, "")
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Echo — basic input is echoed back
// ─────────────────────────────────────────────────────────────────────────────

func TestEchoBasicInput(t *testing.T) {
	// The most fundamental property: whatever the user types comes back.
	out := runParrot(t, []string{"hello", ":quit"})
	full := joinOutputs(out)

	if !strings.Contains(full, "hello") {
		t.Errorf("expected 'hello' in output, got %v", out)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Quit — :quit ends the session immediately
// ─────────────────────────────────────────────────────────────────────────────

func TestQuitEndsSession(t *testing.T) {
	// After :quit the loop stops. Any subsequent inputs are NOT echoed.
	out := runParrot(t, []string{":quit", "should-not-appear"})
	full := joinOutputs(out)

	if strings.Contains(full, "should-not-appear") {
		t.Errorf("expected loop to stop at :quit, but got extra output: %v", out)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Multiple inputs — each line is echoed in order
// ─────────────────────────────────────────────────────────────────────────────

func TestMultipleInputsEchoed(t *testing.T) {
	inputs := []string{"alpha", "beta", "gamma", ":quit"}
	out := runParrot(t, inputs)
	full := joinOutputs(out)

	for _, word := range []string{"alpha", "beta", "gamma"} {
		if !strings.Contains(full, word) {
			t.Errorf("expected %q in output, got %v", word, out)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Sync mode — same echo behaviour as async
// ─────────────────────────────────────────────────────────────────────────────

func TestSyncModeEchoes(t *testing.T) {
	// ModeSync calls Eval on the calling goroutine — no spawned goroutines.
	// The echoed output should contain the same content as async mode.
	out := runParrotMode(t, []string{"sync-test", ":quit"}, repl.ModeSync)
	full := joinOutputs(out)

	if !strings.Contains(full, "sync-test") {
		t.Errorf("sync mode: expected 'sync-test' in output, got %v", out)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Async mode — default mode echoes correctly
// ─────────────────────────────────────────────────────────────────────────────

func TestAsyncModeEchoes(t *testing.T) {
	// ModeAsync is the default: Eval runs in a goroutine.
	out := runParrotMode(t, []string{"async-test", ":quit"}, repl.ModeAsync)
	full := joinOutputs(out)

	if !strings.Contains(full, "async-test") {
		t.Errorf("async mode: expected 'async-test' in output, got %v", out)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. GlobalPrompt contains the parrot emoji
// ─────────────────────────────────────────────────────────────────────────────

func TestGlobalPromptContainsParrotEmoji(t *testing.T) {
	// The framework calls outputFn(prompt.GlobalPrompt()) before each read.
	// So the parrot emoji should appear in the collected output.
	out := runParrot(t, []string{":quit"})
	full := joinOutputs(out)

	if !strings.Contains(full, "🦜") {
		t.Errorf("expected parrot emoji in output, got %q", full)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. LinePrompt contains emoji
// ─────────────────────────────────────────────────────────────────────────────

func TestLinePromptContainsEmoji(t *testing.T) {
	p := ParrotPrompt{}
	lp := p.LinePrompt()

	if !strings.Contains(lp, "🦜") {
		t.Errorf("LinePrompt should contain parrot emoji, got %q", lp)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. EOF exits gracefully — no panic, no hang
// ─────────────────────────────────────────────────────────────────────────────

func TestEOFExitsGracefully(t *testing.T) {
	// An empty input slice means inputFn immediately returns ("", false).
	// The loop should exit cleanly without panicking or blocking.
	// We just verify the function returns (no hang or panic).
	out := runParrot(t, []string{})
	_ = out // nothing to assert — this test passes if it returns at all
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Empty string is echoed
// ─────────────────────────────────────────────────────────────────────────────

func TestEmptyStringEchoed(t *testing.T) {
	// EchoLanguage returns Ok{HasOutput: true} for an empty string.
	// The framework prints a newline for the blank line, so we expect
	// at least two output calls: the prompt + the echoed empty line.
	out := runParrot(t, []string{"", ":quit"})

	if len(out) < 2 {
		t.Errorf("expected at least 2 output calls for empty-string input, got %d: %v", len(out), out)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Multiple inputs before quit — all appear in output
// ─────────────────────────────────────────────────────────────────────────────

func TestMultipleInputsBeforeQuit(t *testing.T) {
	inputs := []string{"one", "two", "three", "four", "five", ":quit"}
	out := runParrot(t, inputs)
	full := joinOutputs(out)

	for _, word := range []string{"one", "two", "three", "four", "five"} {
		if !strings.Contains(full, word) {
			t.Errorf("expected %q in output but it was missing; full output: %q", word, full)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. ParrotPrompt.GlobalPrompt content
// ─────────────────────────────────────────────────────────────────────────────

func TestGlobalPromptContent(t *testing.T) {
	p := ParrotPrompt{}
	gp := p.GlobalPrompt()

	// The global prompt must contain the parrot emoji and the ">" cursor.
	if !strings.Contains(gp, "🦜") {
		t.Errorf("GlobalPrompt should contain 🦜, got %q", gp)
	}
	if !strings.Contains(gp, ">") {
		t.Errorf("GlobalPrompt should contain '>', got %q", gp)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. ParrotPrompt.LinePrompt format
// ─────────────────────────────────────────────────────────────────────────────

func TestLinePromptFormat(t *testing.T) {
	p := ParrotPrompt{}
	lp := p.LinePrompt()

	// LinePrompt should be non-empty and end with a space (conventional
	// prompts always have a trailing space so the cursor is one space away
	// from the prompt character).
	if len(lp) == 0 {
		t.Error("LinePrompt should not be empty")
	}
	if !strings.HasSuffix(lp, " ") {
		t.Errorf("LinePrompt should end with a space, got %q", lp)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 13. Session ends on :quit — no further output after the quit line
// ─────────────────────────────────────────────────────────────────────────────

func TestSessionEndsOnQuit(t *testing.T) {
	// We put a detectable sentinel after :quit to confirm it is not echoed.
	out := runParrot(t, []string{"before", ":quit", "after-sentinel"})
	full := joinOutputs(out)

	if strings.Contains(full, "after-sentinel") {
		t.Errorf("loop should stop at :quit; 'after-sentinel' should not appear: %q", full)
	}
	if !strings.Contains(full, "before") {
		t.Errorf("'before' should appear before :quit: %q", full)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 14. Output is collected in order
// ─────────────────────────────────────────────────────────────────────────────

func TestOutputCollectedInOrder(t *testing.T) {
	// The REPL processes inputs sequentially. The echo outputs should appear
	// in the same order as the inputs. We use sync mode to eliminate any
	// theoretical reordering from goroutine scheduling.
	inputs := []string{"first", "second", "third", ":quit"}
	out := runParrotMode(t, inputs, repl.ModeSync)

	// Filter out prompt strings (they contain "🦜") to isolate echoed lines.
	var echoed []string
	for _, s := range out {
		if !strings.Contains(s, "🦜") {
			trimmed := strings.TrimSpace(s)
			if trimmed != "" {
				echoed = append(echoed, trimmed)
			}
		}
	}

	// We expect exactly 3 echoed lines in order.
	if len(echoed) != 3 {
		t.Fatalf("expected 3 echoed lines, got %d: %v", len(echoed), echoed)
	}
	order := []string{"first", "second", "third"}
	for i, expected := range order {
		if echoed[i] != expected {
			t.Errorf("echoed[%d]: expected %q, got %q", i, expected, echoed[i])
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 15. Sync and async modes produce the same echo output
// ─────────────────────────────────────────────────────────────────────────────

func TestSyncAndAsyncProduceSameOutput(t *testing.T) {
	inputs := []string{"parrot", "squawk", ":quit"}

	syncOut := runParrotMode(t, inputs, repl.ModeSync)
	asyncOut := runParrotMode(t, inputs, repl.ModeAsync)

	// Extract echoed (non-prompt) lines from each mode's output.
	filterEchoed := func(outputs []string) []string {
		var result []string
		for _, s := range outputs {
			if !strings.Contains(s, "🦜") {
				result = append(result, s)
			}
		}
		return result
	}

	syncEchoed := filterEchoed(syncOut)
	asyncEchoed := filterEchoed(asyncOut)

	if len(syncEchoed) != len(asyncEchoed) {
		t.Fatalf("sync echoed %d lines, async echoed %d lines", len(syncEchoed), len(asyncEchoed))
	}
	for i := range syncEchoed {
		if syncEchoed[i] != asyncEchoed[i] {
			t.Errorf("line %d: sync=%q async=%q", i, syncEchoed[i], asyncEchoed[i])
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 16. Input with spaces is echoed verbatim
// ─────────────────────────────────────────────────────────────────────────────

func TestInputWithSpacesEchoedVerbatim(t *testing.T) {
	// EchoLanguage does not trim whitespace from normal input (only from
	// the :quit sentinel). Internal spaces must be preserved.
	input := "hello   world   with   spaces"
	out := runParrot(t, []string{input, ":quit"})
	full := joinOutputs(out)

	if !strings.Contains(full, input) {
		t.Errorf("expected %q verbatim in output, got %q", input, full)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 17. Single-item input (just :quit) produces no echoed lines
// ─────────────────────────────────────────────────────────────────────────────

func TestQuitOnlyProducesNoEchoedLines(t *testing.T) {
	out := runParrot(t, []string{":quit"})

	// After filtering out prompt strings, there should be nothing left.
	for _, s := range out {
		if !strings.Contains(s, "🦜") {
			t.Errorf("unexpected non-prompt output when input was only ':quit': %q", s)
		}
	}
}
