// Package repl provides a language-agnostic Read–Eval–Print Loop framework.
//
// # What is a REPL?
//
// A REPL (Read–Eval–Print Loop) is the interactive shell you find in Python,
// Ruby, Elixir's iex, and many other language environments. The loop:
//
//  1. Reads a line of user input.
//  2. Evaluates it by handing it off to a language back-end.
//  3. Prints the result.
//  4. Loops back to step 1.
//
// This package provides the skeletal framework — the scaffolding that handles
// I/O, concurrency, and the main loop. You plug in your own Language
// implementation and the framework handles the rest.
//
// # Architecture
//
// The framework is built around three pluggable interfaces:
//
//   - [Language]  — evaluates one input string and returns a [Result].
//   - [Prompt]    — provides the "> " and "... " prompt strings.
//   - [Waiting]   — animates a spinner (or does nothing) while eval runs.
//
// Evaluation is asynchronous: the framework launches eval in a goroutine and
// polls for its completion via a select loop. This keeps the UI responsive
// even for long-running evaluations and makes it easy to add spinners.
//
// I/O is fully injectable via [InputFn] and [OutputFn], so the REPL can be
// driven from tests, pipes, or network connections without touching os.Stdin.
//
// # Panic Safety
//
// If the Language goroutine panics, the panic is recovered and converted into
// an error [Result], ensuring the REPL loop survives a misbehaving evaluator.
package repl

import (
	"bufio"
	"fmt"
	"os"
	"strings"
	"time"
)

// ─────────────────────────────────────────────────────────────────────────────
// Mode — sync vs async evaluation
// ─────────────────────────────────────────────────────────────────────────────

// Mode controls how the REPL dispatches evaluation.
//
// In ModeAsync (the default), Eval is called in a goroutine and the main
// goroutine drives the Waiting animation while it waits. This keeps the UI
// responsive during long-running evaluations.
//
// In ModeSync, Eval is called directly on the calling goroutine. There are no
// spawned goroutines, channels, or ticker loops. This is simpler and faster
// for contexts where concurrency is unnecessary — for example, scripted test
// harnesses or single-threaded embedding scenarios. The Waiting field in
// Options is ignored in sync mode.
type Mode int

const (
	// ModeAsync is the default mode: Eval runs in a goroutine with a
	// Waiting animation on the main goroutine.
	ModeAsync Mode = iota

	// ModeSync calls Eval directly on the calling goroutine. No spinner,
	// no channels, no goroutines.
	ModeSync
)

// Options bundles together the optional configuration for RunWithOptions.
//
// When using RunWithIO (the convenience wrapper) you do not need Options —
// it uses ModeAsync with the Waiting you pass directly.
type Options struct {
	// Mode selects sync or async evaluation. Defaults to ModeAsync.
	Mode Mode

	// Waiting animates the terminal while eval is in flight.
	// Only used when Mode == ModeAsync. May be nil when Mode == ModeSync.
	Waiting Waiting
}

// ─────────────────────────────────────────────────────────────────────────────
// Core types
// ─────────────────────────────────────────────────────────────────────────────

// Result is the outcome of one evaluation cycle.
//
// The Tag field drives control flow:
//   - "ok"    — evaluation succeeded; Output may contain a value to display.
//   - "error" — evaluation failed; Output contains a human-readable message.
//   - "quit"  — the user (or language) requested exit; the loop terminates.
//
// HasOutput is a separate boolean rather than testing len(Output) == 0 because
// some languages legitimately produce an empty string as a valid result (e.g.,
// a function that returns an empty list). By being explicit, we avoid silently
// swallowing those results.
type Result struct {
	// Tag classifies the outcome: "ok", "error", or "quit".
	Tag string

	// Output is the text to display to the user.
	// Populated when Tag is "ok" (and HasOutput is true) or "error".
	Output string

	// HasOutput signals that Output should be printed even if it is empty.
	// For Tag=="ok", set this to true whenever there is something to show.
	// For Tag=="error", this is implicitly true (Output always printed).
	HasOutput bool
}

// Language is the pluggable evaluator that gives the REPL its personality.
//
// Implementing this interface is all you need to build a new REPL back-end.
// Examples: a calculator, a Lua interpreter, a SQL query runner.
//
// Eval must be safe to call from a goroutine. It must not hold any locks
// that would block the REPL's main goroutine. If Eval panics, the framework
// will recover the panic and surface it as an error Result.
type Language interface {
	// Eval processes one input string and returns a Result.
	// The input is a single logical "unit" as typed by the user.
	Eval(input string) Result
}

// Prompt supplies the strings displayed at the beginning of each input line.
//
// Two prompt styles are common:
//   - GlobalPrompt: shown at the top of each new statement (e.g., "> ").
//   - LinePrompt:   shown for continuation lines inside a multi-line block
//     (e.g., "... ").
//
// Using an interface instead of plain strings allows prompts to change
// dynamically — for example, showing the current git branch or database name.
type Prompt interface {
	// GlobalPrompt is shown at the start of each new top-level expression.
	GlobalPrompt() string

	// LinePrompt is shown for continuation lines (multi-line input mode).
	LinePrompt() string
}

// Waiting animates the terminal while the Language goroutine is running.
//
// The lifecycle is:
//
//  1. Start()        — called once when eval begins; returns initial state.
//  2. Tick(state)    — called every TickMs() milliseconds; returns next state.
//  3. Stop(state)    — called once when eval completes; cleans up the display.
//
// The state value is opaque to the framework — it is threaded through Start →
// Tick → Stop as-is, allowing implementations to track spinner position,
// elapsed time, etc., without package-level variables.
//
// SilentWaiting (provided in this package) satisfies this interface with
// no-ops, making it suitable for non-interactive or test environments.
type Waiting interface {
	// Start initialises the waiting animation. Returns the initial state.
	Start() interface{}

	// Tick advances the animation by one frame. Returns the updated state.
	Tick(state interface{}) interface{}

	// TickMs is the interval between Tick calls, in milliseconds.
	TickMs() int

	// Stop finalises the animation (e.g., erases the spinner line).
	Stop(state interface{})
}

// ─────────────────────────────────────────────────────────────────────────────
// I/O injection
// ─────────────────────────────────────────────────────────────────────────────

// InputFn reads one line of user input.
//
// Returns the line and ok=true on success.
// Returns ("", false) at EOF, on an unrecoverable read error, or when the
// user closes the input stream — any of these causes the REPL loop to exit.
//
// The function should NOT include the trailing newline in the returned string.
type InputFn func() (string, bool)

// OutputFn writes one string to the user.
//
// The framework calls this for prompts, evaluation results, and error messages.
// Implementations may append a newline or not — by convention the framework
// passes complete lines (already newline-terminated) for results, and bare
// strings (without a trailing newline) for inline prompts.
type OutputFn func(string)

// ─────────────────────────────────────────────────────────────────────────────
// Run — the main entry point
// ─────────────────────────────────────────────────────────────────────────────

// Run starts the REPL loop, reading from os.Stdin and writing to os.Stdout.
//
// This is the high-level convenience entry point. For tests or embedded use,
// prefer [RunWithIO] which allows full I/O injection.
//
// The loop runs until:
//   - The Language returns a Result with Tag == "quit".
//   - The input source returns ok=false (EOF or error).
//
// Parameters:
//   - language — the evaluator for the language being hosted.
//   - prompt   — supplies the prompt strings.
//   - waiting  — animates the display while eval is in flight.
func Run(language Language, prompt Prompt, waiting Waiting) {
	// Wrap os.Stdin / os.Stdout with simple closures so RunWithIO can do the
	// real work. This keeps all loop logic in one place.
	inputFn := stdinReader()
	outputFn := stdoutWriter()
	RunWithIO(language, prompt, waiting, inputFn, outputFn)
}

// RunWithIO is the fully injectable REPL loop.
//
// This is a convenience wrapper around [RunWithOptions] that always uses
// ModeAsync (goroutine + channel + waiting animation). All existing callers
// continue to work without any changes.
//
// The loop:
//
//  1. Displays a prompt via outputFn.
//  2. Reads a line via inputFn.
//  3. Dispatches the line to language.Eval in a fresh goroutine.
//  4. Waits for the result, ticking waiting.Tick every waiting.TickMs() ms.
//  5. Prints the result (if any) via outputFn.
//  6. Repeats until quit or EOF.
//
// Panic safety: if language.Eval panics, the panic is caught inside the
// goroutine and converted to an error Result so the loop continues.
func RunWithIO(language Language, prompt Prompt, waiting Waiting, inputFn InputFn, outputFn OutputFn) {
	RunWithOptions(language, prompt, Options{Mode: ModeAsync, Waiting: waiting}, inputFn, outputFn)
}

// RunWithOptions is the fully injectable REPL loop with explicit mode control.
//
// Use opts.Mode to select between:
//   - ModeAsync (default): Eval runs in a goroutine; opts.Waiting animates.
//   - ModeSync:            Eval runs on the calling goroutine; opts.Waiting
//     is ignored and may be nil.
//
// Panic safety applies in both modes: panics from language.Eval are caught
// and converted to error Results so the loop continues.
func RunWithOptions(language Language, prompt Prompt, opts Options, inputFn InputFn, outputFn OutputFn) {
	for {
		// ── Step 1: Display the prompt ────────────────────────────────────────
		//
		// We write the prompt without a trailing newline so the cursor stays
		// on the same line as the user's input.
		outputFn(prompt.GlobalPrompt())

		// ── Step 2: Read input ────────────────────────────────────────────────
		//
		// If the input function signals EOF or an error we exit cleanly.
		input, ok := inputFn()
		if !ok {
			return
		}

		// ── Step 3: Evaluate — async or sync depending on opts.Mode ──────────
		var result Result

		if opts.Mode == ModeSync {
			// Sync mode: call Eval directly on this goroutine.
			// We still wrap in a deferred recover so a panicking evaluator
			// surfaces as an error rather than crashing the caller.
			result = evalSync(language, input)
		} else {
			// Async mode: run Eval in a goroutine so the waiting animation can
			// tick on the main goroutine. The channel is buffered (capacity 1)
			// so the goroutine can deposit its result and exit even if we
			// haven't read it yet.
			resultCh := make(chan Result, 1)
			go func() {
				// Recover any panic from the language evaluator and convert it
				// to an error Result, keeping the REPL alive.
				defer func() {
					if r := recover(); r != nil {
						resultCh <- Result{
							Tag:       "error",
							Output:    fmt.Sprintf("panic in evaluator: %v", r),
							HasOutput: true,
						}
					}
				}()
				resultCh <- language.Eval(input)
			}()

			// Wait for the result, ticking the spinner on each interval.
			result = waitForResult(resultCh, opts.Waiting)
		}

		// ── Step 4: Handle the result ─────────────────────────────────────────
		switch result.Tag {
		case "quit":
			// Language or user requested exit — terminate the loop.
			return

		case "error":
			// Surface the error to the user; continue the loop.
			outputFn(result.Output + "\n")

		case "ok":
			// Only print output when the language produced something worth
			// showing. Some expressions (assignments, void functions) return
			// nothing and we should not print a blank line.
			if result.HasOutput {
				outputFn(result.Output + "\n")
			}
		}
		// Any unrecognised Tag is silently ignored — forward compatibility.
	}
}

// evalSync calls language.Eval on the current goroutine, recovering any panic
// and converting it to an error Result.
//
// This is the sync-mode counterpart to the goroutine+channel path in
// RunWithOptions. Keeping it in a named function makes the defer/recover
// hygiene clear and testable independently.
func evalSync(language Language, input string) (result Result) {
	defer func() {
		if r := recover(); r != nil {
			result = Result{
				Tag:       "error",
				Output:    fmt.Sprintf("panic in evaluator: %v", r),
				HasOutput: true,
			}
		}
	}()
	return language.Eval(input)
}

// ─────────────────────────────────────────────────────────────────────────────
// Default I/O helpers (used by Run)
// ─────────────────────────────────────────────────────────────────────────────

// stdinReader returns an InputFn that reads lines from os.Stdin.
//
// It uses a bufio.Scanner under the hood, which handles both Unix (\n) and
// Windows (\r\n) line endings transparently.
func stdinReader() InputFn {
	scanner := bufio.NewScanner(os.Stdin)
	return func() (string, bool) {
		if !scanner.Scan() {
			return "", false
		}
		// strings.TrimRight removes any stray \r on platforms that don't strip it.
		return strings.TrimRight(scanner.Text(), "\r"), true
	}
}

// stdoutWriter returns an OutputFn that writes directly to os.Stdout.
func stdoutWriter() OutputFn {
	return func(s string) {
		fmt.Fprint(os.Stdout, s)
	}
}

// waitForResult blocks until the evaluation goroutine sends a Result,
// ticking the Waiting animation at the configured interval.
//
// This function encapsulates the select loop so it can be reasoned about
// (and tested) independently of the full Run loop.
func waitForResult(resultCh <-chan Result, waiting Waiting) Result {
	state := waiting.Start()

	ticker := time.NewTicker(time.Duration(waiting.TickMs()) * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case result := <-resultCh:
			// Evaluation complete — stop the animation and return the result.
			waiting.Stop(state)
			return result
		case <-ticker.C:
			// Tick the animation while we wait.
			state = waiting.Tick(state)
		}
	}
}
