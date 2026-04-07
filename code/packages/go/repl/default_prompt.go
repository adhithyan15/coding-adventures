package repl

// DefaultPrompt provides the classic interactive-shell prompt strings.
//
// # The two-prompt convention
//
// Most REPLs use two different prompts to distinguish context:
//
//   - Primary prompt ("> ")   — shown at the start of every new statement.
//     The user sees this when the REPL is waiting for fresh input.
//
//   - Continuation prompt ("... ") — shown when a statement spans multiple
//     lines. The leading dots signal "I'm waiting for you to finish the
//     expression you started."
//
// This two-prompt design traces back to the original Unix shells (sh, csh)
// and was popularised by Python's interactive interpreter. The exact strings
// "> " and "... " are the same ones Python 3 uses.
//
// # When to use DefaultPrompt
//
// DefaultPrompt is appropriate for:
//   - Quick prototypes and examples.
//   - Any REPL that does not need dynamic prompt customisation.
//
// For more sophisticated prompts — coloured output, git branch display,
// virtualenv indicators — implement the Prompt interface directly.
type DefaultPrompt struct{}

// GlobalPrompt returns "> ", the primary prompt string.
//
// The trailing space separates the prompt character from the user's input,
// making the line easier to read at a glance.
func (p DefaultPrompt) GlobalPrompt() string {
	return "> "
}

// LinePrompt returns "... ", the continuation prompt string.
//
// Three dots followed by a space mirrors Python's interactive interpreter.
// It visually aligns with the two-character "> " prompt while making it
// obvious that the current line is a continuation.
func (p DefaultPrompt) LinePrompt() string {
	return "... "
}
