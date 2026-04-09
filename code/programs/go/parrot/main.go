// Package main implements the Parrot REPL — the world's simplest REPL.
//
// # What is a Parrot?
//
// A parrot repeats everything you say. Whatever you type into this REPL,
// the program echoes it back unchanged. Type ":quit" to end the session.
//
// This is intentionally trivial — the point is not the parrot, but the
// framework underneath it. The coding-adventures REPL framework handles:
//   - The read-eval-print loop
//   - Async evaluation with goroutine isolation
//   - Panic recovery (a crashing evaluator doesn't kill the session)
//   - I/O injection (the same loop works for terminal, tests, and pipes)
//
// The program only needs to supply the personality: a Language that echoes,
// a Prompt that shows parrot text, and a Waiting that stays silent.
//
// # Components wired together
//
//	repl.EchoLanguage{}   — evaluates input by echoing it back
//	ParrotPrompt{}        — provides parrot-themed prompt strings
//	repl.SilentWaiting{}  — shows nothing while the evaluator "runs"
//
// # Architecture diagram
//
//	stdin ──► inputFn ──► RunWithIO ──► EchoLanguage.Eval
//	                           │
//	                           ▼
//	                      ParrotPrompt (writes "🦜 > " before each line)
//	                           │
//	                           ▼
//	                      outputFn ──► stdout
//
// The framework runs Eval in a goroutine and polls for the result every
// 100 ms (SilentWaiting's tick interval). For EchoLanguage this is
// instantaneous, but the architecture scales to slow evaluators.
package main

import (
	"bufio"
	"fmt"
	"os"

	"github.com/adhithyan15/coding-adventures/code/packages/go/repl"
)

// main is the entry point for the Parrot REPL.
//
// It wires up three components and hands them to the REPL framework:
//   - EchoLanguage: the language back-end (echo = the "parrot" behaviour)
//   - ParrotPrompt: the personality layer (prompt strings with 🦜)
//   - SilentWaiting: the waiting animation (none — parrot is fast)
//
// I/O is provided by os.Stdin and os.Stdout via simple closure wrappers.
// The framework takes ownership of the loop; main only sets up the plumbing.
func main() {
	// ── Input ─────────────────────────────────────────────────────────────────
	//
	// bufio.Scanner reads line-by-line from stdin. It handles both Unix (\n)
	// and Windows (\r\n) line endings automatically. Each call to inputFn
	// blocks until the user presses Enter (or EOF is reached).
	scanner := bufio.NewScanner(os.Stdin)

	// inputFn matches the repl.InputFn signature: func() (string, bool).
	// It returns (line, true) when a line was read and ("", false) on EOF.
	// Returning false is the signal to the framework to terminate the loop.
	inputFn := func() (string, bool) {
		if scanner.Scan() {
			return scanner.Text(), true
		}
		// EOF or read error — signal the loop to stop.
		return "", false
	}

	// ── Output ────────────────────────────────────────────────────────────────
	//
	// outputFn matches the repl.OutputFn signature: func(string).
	// fmt.Print (not Println) is used because the framework already appends
	// newlines to result strings; adding another would produce blank lines.
	outputFn := func(text string) {
		fmt.Print(text)
	}

	// ── Banner ────────────────────────────────────────────────────────────────
	//
	// Print a welcome message before starting the loop. This is separate from
	// the per-line GlobalPrompt so the banner appears exactly once.
	fmt.Println("🦜 Parrot REPL")
	fmt.Println("I repeat everything you say! Type :quit to exit.")
	fmt.Println()

	// ── REPL loop ─────────────────────────────────────────────────────────────
	//
	// RunWithIO is the fully-injectable entry point. It runs until:
	//   - EchoLanguage.Eval returns a quit Result (user typed ":quit"), or
	//   - inputFn returns ok=false (EOF / Ctrl-D / pipe closed).
	repl.RunWithIO(
		repl.EchoLanguage{},
		ParrotPrompt{},
		repl.SilentWaiting{},
		inputFn,
		outputFn,
	)

	// ── Goodbye ───────────────────────────────────────────────────────────────
	//
	// Printed after the loop exits, regardless of whether the user typed
	// ":quit" or closed stdin. Gives the session a polite ending.
	fmt.Println("Goodbye! 🦜")
}
