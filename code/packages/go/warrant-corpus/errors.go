// Package warrantcorpus is the Go lane runner for the language-neutral Warrant
// evidence corpus (VER04i, WD-103).
//
// # What this package is
//
// The Rust compiler (warrant-grammar) turns a package's `.warrant` file into a
// `warrant-corpus-v1` directory: `corpus.json` (domains, model IR, digests) and
// `cases.tfv` (examples). This package is the *Go lane* of that design. It
// loads the corpus, re-enumerates every exhaustive domain in Go, verifies the
// input and expected-value digests against the corpus, drives a Go library
// through a compiled registry plus an `evidence.bindings` file, runs the
// negative controls that prove the runner can fail, and writes the lane's
// `warrant-lane-evidence-v1` manifest.
//
// It mirrors the Python (WD-101) and TypeScript (WD-102) lanes check for check.
// The reading order for newcomers:
//
//	values -> tfv -> domains -> ir -> corpus -> faults -> binding
//	       -> runner -> manifest -> session -> cli
//
// It uses only the Go standard library and depends on no library it tests.
package warrantcorpus

import (
	"fmt"
	"sort"
)

// # Error types and the closed set of failure ids
//
// Two different things can go wrong in a lane run, and they must never be
// confused:
//
//   - The run cannot be trusted to start. The corpus is malformed, the binding
//     names an export that does not exist, the command line is wrong. These are
//     a *LaneError (a Kind says which) and the command exits with status 2. No
//     verdict is produced, because there is nothing to give a verdict about.
//   - The library under test is wrong. A case disagrees with the model. That is
//     a verdict (`not ok <id> <failure-id>`, exit status 1). It is data, not an
//     error, because a negative control wants it.
//
// The failure ids are a closed set (VER04i section 6.2), so a manifest checker
// can reject an id it has never heard of instead of guessing.
const (
	CountMismatch     = "count-mismatch"
	StreamMismatch    = "stream-mismatch"
	CallCountMismatch = "call-count-mismatch"
	ResultMismatch    = "result-mismatch"
	TypeMismatch      = "type-mismatch"
	MissingError      = "missing-error"
	UnexpectedError   = "unexpected-error"
	UncaughtFault     = "uncaught-fault"
)

// FailureIDs is the closed set of verdict failure ids.
var FailureIDs = map[string]bool{
	CountMismatch:     true,
	StreamMismatch:    true,
	CallCountMismatch: true,
	ResultMismatch:    true,
	TypeMismatch:      true,
	MissingError:      true,
	UnexpectedError:   true,
	UncaughtFault:     true,
}

// Kind classifies why a run was refused.
type Kind int

// The three refusal kinds. A Kind is only for tests and for the message prefix;
// every one of them exits with status 2.
const (
	// CorpusKind: the corpus, its TFV file or its model IR is malformed or
	// inconsistent.
	CorpusKind Kind = iota
	// BindingKind: the binding file or registry is malformed, incomplete or
	// does not match the code.
	BindingKind
	// UsageKind: the command line or environment asks for something refused.
	UsageKind
)

// LaneError means the run cannot proceed; the command exits with status 2.
//
// Code is a short stable slug ("malformed-json", "digest" ...) so tests and
// callers can match on the cause without parsing prose. The message never
// echoes untrusted input wholesale: it names the kind of problem.
type LaneError struct {
	Kind Kind
	Code string
	Msg  string
}

// Error implements error.
func (e *LaneError) Error() string { return e.Code + ": " + e.Msg }

func laneError(kind Kind, code, format string, args ...any) *LaneError {
	return &LaneError{Kind: kind, Code: code, Msg: fmt.Sprintf(format, args...)}
}

// fail aborts the current computation with a corpus error.
//
// The runner treats the corpus as untrusted and validates it in many small
// steps; threading an error return through every one would bury each check in
// plumbing. Instead a check panics with a *LaneError and the entry points
// (guard) convert it back into an ordinary error exactly once. Nothing outside
// this package ever sees the panic.
func fail(code, format string, args ...any) {
	panic(laneError(CorpusKind, code, format, args...))
}

// need panics with a corpus error unless condition holds. Corpus validation is a
// long list of "this must be true"; one helper keeps each of them to a line.
func need(condition bool, code, format string, args ...any) {
	if !condition {
		fail(code, format, args...)
	}
}

func bindingFail(code, format string, args ...any) {
	panic(laneError(BindingKind, code, format, args...))
}

func usageFail(code, format string, args ...any) {
	panic(laneError(UsageKind, code, format, args...))
}

// guard runs f and converts a *LaneError panic into a returned error. Any other
// panic is a bug in the runner, not a verdict about the library, and is turned
// into an "internal" refusal so it exits 2 rather than 1 (which is reserved for
// a failed case).
func guard(f func()) (err error) {
	defer func() {
		switch r := recover().(type) {
		case nil:
		case *LaneError:
			err = r
		default:
			err = fmt.Errorf("internal: runner bug (%T)", r)
		}
	}()
	f()
	return nil
}

func sortStrings(items []string) { sort.Strings(items) }
