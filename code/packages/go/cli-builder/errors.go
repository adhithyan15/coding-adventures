// Package clibuilder implements a declarative CLI argument parser driven by
// directed graphs and modal state machines.
//
// # Overview
//
// CLI Builder separates two concerns that most CLI libraries conflate:
//
//  1. What the tool accepts — described in a JSON specification file.
//  2. What the tool does — the business logic the caller implements.
//
// The developer writes a JSON spec describing their CLI's structure.
// CLI Builder reads that spec, validates it, builds internal data structures,
// and is ready to parse argv on every invocation.
//
// # Architecture
//
// Command routing is modeled as a directed graph (G_cmd): nodes are commands,
// edges are labeled by the subcommand token that triggers the transition.
// Phase 1 of parsing traverses G_cmd to find the resolved command node.
//
// Flag dependencies form a second directed graph (G_flag) per scope.
// A cycle in G_flag means the spec is self-contradictory and is caught at
// load time.
//
// Parse-mode tracking uses a Modal State Machine with four modes:
// SCANNING, FLAG_VALUE, END_OF_FLAGS (and conceptually ROUTING in Phase 1).
//
// # Usage
//
//	parser, err := clibuilder.NewParser("my-cli.json", os.Args)
//	if err != nil {
//	    fmt.Fprintln(os.Stderr, err)
//	    os.Exit(1)
//	}
//	result, err := parser.Parse()
//	if err != nil {
//	    fmt.Fprintln(os.Stderr, err)
//	    os.Exit(1)
//	}
//	switch r := result.(type) {
//	case *clibuilder.ParseResult:
//	    // use r.Flags, r.Arguments, r.CommandPath
//	case *clibuilder.HelpResult:
//	    fmt.Println(r.Text)
//	    os.Exit(0)
//	case *clibuilder.VersionResult:
//	    fmt.Println(r.Version)
//	    os.Exit(0)
//	}
package clibuilder

import (
	"fmt"
	"strings"
)

// =========================================================================
// Error types
// =========================================================================
//
// CLI Builder surfaces two categories of errors:
//
//  1. SpecError — the JSON specification itself is invalid. These are
//     programmer errors caught at load time. The library refuses to parse
//     any argv when the spec is invalid.
//
//  2. ParseError / ParseErrors — the user's invocation does not match
//     the spec. These are user-facing errors with human-readable messages
//     and optional corrective suggestions.
//
// Both implement the standard Go `error` interface so callers can use
// them uniformly with `if err != nil { ... }`.

// SpecError is returned when the JSON specification file contains a
// structural or semantic error.
//
// Examples: duplicate flag IDs, circular `requires` dependencies,
// missing required fields, unknown spec version.
type SpecError struct {
	// Message is a human-readable description of what is wrong.
	Message string
}

// Error implements the error interface.
func (e *SpecError) Error() string {
	return fmt.Sprintf("spec error: %s", e.Message)
}

// ParseErrorType is a snake_case string identifying the category of a
// parse error. Machine-readable — callers can switch on this value.
type ParseErrorType string

const (
	// ErrUnknownCommand: a token in subcommand position matches no known
	// command or alias at that level.
	ErrUnknownCommand ParseErrorType = "unknown_command"

	// ErrUnknownFlag: a flag token matches no known flag in scope.
	ErrUnknownFlag ParseErrorType = "unknown_flag"

	// ErrMissingRequiredFlag: a flag declared `required: true` is absent
	// and its `required_unless` exemption is not satisfied.
	ErrMissingRequiredFlag ParseErrorType = "missing_required_flag"

	// ErrMissingRequiredArgument: a positional argument declared
	// `required: true` is absent and `required_unless_flag` is not satisfied.
	ErrMissingRequiredArgument ParseErrorType = "missing_required_argument"

	// ErrConflictingFlags: two flags that list each other in `conflicts_with`
	// are both present in the same invocation.
	ErrConflictingFlags ParseErrorType = "conflicting_flags"

	// ErrMissingDependencyFlag: a flag is present but a flag it `requires`
	// (directly or transitively via G_flag) is absent.
	ErrMissingDependencyFlag ParseErrorType = "missing_dependency_flag"

	// ErrTooFewArguments: a variadic argument receives fewer values than
	// its `variadic_min` setting.
	ErrTooFewArguments ParseErrorType = "too_few_arguments"

	// ErrTooManyArguments: more positional tokens exist than argument slots,
	// or a variadic argument exceeds its `variadic_max`.
	ErrTooManyArguments ParseErrorType = "too_many_arguments"

	// ErrInvalidValue: a value fails type coercion (e.g., "abc" for integer).
	ErrInvalidValue ParseErrorType = "invalid_value"

	// ErrInvalidEnumValue: a value is not in the flag's `enum_values` list.
	ErrInvalidEnumValue ParseErrorType = "invalid_enum_value"

	// ErrExclusiveGroupViolation: multiple flags from a `mutually_exclusive_group`
	// are present simultaneously.
	ErrExclusiveGroupViolation ParseErrorType = "exclusive_group_violation"

	// ErrMissingExclusiveGroup: a `required: true` mutually_exclusive_group
	// has no flags present.
	ErrMissingExclusiveGroup ParseErrorType = "missing_exclusive_group"

	// ErrDuplicateFlag: a non-repeatable flag appears more than once.
	ErrDuplicateFlag ParseErrorType = "duplicate_flag"

	// ErrInvalidStack: a stacked-flag sequence contains an unknown character
	// or a non-boolean flag in the wrong position.
	ErrInvalidStack ParseErrorType = "invalid_stack"
)

// ParseError describes a single error encountered during argv parsing.
//
// CLI Builder collects all errors it can find before returning, so the
// user sees everything wrong with their invocation at once rather than
// fixing errors one by one.
type ParseError struct {
	// ErrorType identifies the error category (machine-readable).
	ErrorType ParseErrorType

	// Message is a human-readable sentence explaining the error.
	// Written in natural language, suitable for printing to stderr.
	Message string

	// Suggestion is an optional corrective hint. For unknown_command
	// and unknown_flag errors, this contains the closest fuzzy match
	// if one exists within edit distance 2.
	Suggestion string

	// Context is the command_path at the point where the error was
	// detected. Helps the user understand which subcommand is involved.
	Context []string
}

// ParseErrors is the error type returned by Parser.Parse() when one or
// more parse errors are encountered.
//
// It implements the error interface so it can be returned as a plain
// `error`. Callers that need the individual errors can type-assert to
// *ParseErrors and inspect the Errors slice.
type ParseErrors struct {
	Errors []ParseError
}

// Error implements the error interface. Returns all error messages
// joined by newlines, prefixed with a summary count.
func (pe *ParseErrors) Error() string {
	if len(pe.Errors) == 1 {
		return fmt.Sprintf("parse error: %s", pe.Errors[0].Message)
	}
	msgs := make([]string, len(pe.Errors))
	for i, e := range pe.Errors {
		msgs[i] = fmt.Sprintf("  - %s", e.Message)
	}
	return fmt.Sprintf("%d parse errors:\n%s", len(pe.Errors), strings.Join(msgs, "\n"))
}

// levenshtein computes the Levenshtein edit distance between two strings.
//
// The edit distance is the minimum number of single-character insertions,
// deletions, or substitutions needed to transform s into t.
//
// We use the standard dynamic-programming approach with a two-row rolling
// array for O(min(m,n)) space.
//
// Example distances:
//
//	levenshtein("commit", "comit")  = 1   (one deletion)
//	levenshtein("--message", "--mesage") = 1   (one deletion)
//	levenshtein("hello", "world")   = 4
func levenshtein(s, t string) int {
	sr := []rune(s)
	tr := []rune(t)
	m := len(sr)
	n := len(tr)
	if m == 0 {
		return n
	}
	if n == 0 {
		return m
	}

	// prev[j] = edit distance between s[0..i-1] and t[0..j-1]
	prev := make([]int, n+1)
	curr := make([]int, n+1)
	for j := 0; j <= n; j++ {
		prev[j] = j
	}

	for i := 1; i <= m; i++ {
		curr[0] = i
		for j := 1; j <= n; j++ {
			if sr[i-1] == tr[j-1] {
				curr[j] = prev[j-1]
			} else {
				ins := curr[j-1] + 1
				del := prev[j] + 1
				sub := prev[j-1] + 1
				min := ins
				if del < min {
					min = del
				}
				if sub < min {
					min = sub
				}
				curr[j] = min
			}
		}
		prev, curr = curr, prev
	}
	return prev[n]
}

// fuzzyMatch finds the best match among candidates for the unknown token.
// Returns the candidate and true if edit distance ≤ 2, or "" and false.
//
// This implements the §8.3 fuzzy matching requirement: for unknown_command
// and unknown_flag errors, suggest the closest known name.
func fuzzyMatch(unknown string, candidates []string) (string, bool) {
	best := ""
	bestDist := 3 // only report if dist ≤ 2
	for _, c := range candidates {
		d := levenshtein(unknown, c)
		if d < bestDist {
			bestDist = d
			best = c
		}
	}
	if best != "" {
		return best, true
	}
	return "", false
}
