// Package main — prompt.go
//
// This file defines ParrotPrompt, the personality layer for the Parrot REPL.
// It lives in its own file (rather than main.go) so that tests in the
// main_test package can reference it by its exported name without importing
// an unexported symbol from package main.
//
// # Why a separate file?
//
// Go test files with the suffix `_test.go` and the declaration
// `package main_test` form a separate package that is compiled alongside the
// main package. That external test package can only see exported identifiers
// from the package under test. Keeping ParrotPrompt in its own source file
// makes it trivially accessible from tests while keeping main.go clean.
package main

import "github.com/adhithyan15/coding-adventures/code/packages/go/repl"

// Compile-time assertion: ParrotPrompt must satisfy the repl.Prompt interface.
//
// If the interface ever changes and ParrotPrompt falls out of sync, the build
// will fail with a clear "does not implement" error rather than a mysterious
// runtime failure.
var _ repl.Prompt = ParrotPrompt{}

// ParrotPrompt implements repl.Prompt with parrot-themed text and emoji.
//
// # The two prompt strings
//
// A Prompt implementation must supply two strings:
//
//   - GlobalPrompt: shown once before each new user input. In Parrot's case,
//     this is the single-line prompt "🦜 > " that the user types next to.
//
//   - LinePrompt: used by multi-line language back-ends for continuation
//     lines (e.g., Python's "... "). EchoLanguage never produces multi-line
//     input, but we provide a reasonable string for completeness.
//
// # Design choice: prompt-in-GlobalPrompt
//
// Some REPL frameworks print the prompt inside the I/O read loop; others
// expect the prompt string to be returned from the Language side. The
// coding-adventures framework delegates prompt strings to the Prompt
// interface and prints them before each read. This means the user always
// sees a fresh prompt on every new line, regardless of whether the
// evaluation produced output.
type ParrotPrompt struct{}

// GlobalPrompt returns the primary input prompt shown before each new line.
//
// The parrot emoji (🦜) signals that this is the Parrot REPL.
// The " > " suffix is the conventional shell-style input indicator.
func (p ParrotPrompt) GlobalPrompt() string {
	return "🦜 > "
}

// LinePrompt returns the continuation prompt for multi-line expressions.
//
// EchoLanguage never enters multi-line mode, but a well-formed Prompt must
// return a sensible string here. We use the same parrot emoji with a "..." to
// keep the visual language consistent.
func (p ParrotPrompt) LinePrompt() string {
	return "🦜 ... "
}
