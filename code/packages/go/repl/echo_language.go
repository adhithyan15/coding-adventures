package repl

// EchoLanguage is the simplest possible Language implementation.
//
// # What it does
//
// EchoLanguage mirrors the user's input back as its output — like the Unix
// `cat` command or a parrot. Its only special behaviour is recognising the
// ":quit" command, which signals the REPL to terminate.
//
// # When to use it
//
// EchoLanguage is useful for:
//   - Smoke-testing the REPL framework itself (does the loop work?).
//   - Bootstrapping a new REPL back-end before wiring in the real evaluator.
//   - Unit tests that need a predictable, side-effect-free Language.
//
// # The :quit convention
//
// Many REPLs use a special keyword to signal exit. Common choices include
// ":quit", ":exit", "quit()", and Ctrl-D (EOF). EchoLanguage follows the
// ":quit" convention. The leading colon prevents accidental exit when the
// user types the word "quit" as data.
type EchoLanguage struct{}

// Eval echoes the input back to the user, or signals quit.
//
// Behaviour by input:
//   - ":quit" → returns Result{Tag: "quit"}
//   - anything else → returns Result{Tag: "ok", Output: input, HasOutput: true}
//
// Empty input is echoed as an empty string with HasOutput=true, producing a
// blank line in the output. This is intentional: the user pressed Enter and
// deserves to see the cursor advance.
func (e EchoLanguage) Eval(input string) Result {
	// ── Quit signal ──────────────────────────────────────────────────────────
	//
	// ":quit" is the canonical exit command for this language. When the user
	// types it, we return a quit Result so the REPL loop terminates cleanly.
	if input == ":quit" {
		return Result{Tag: "quit"}
	}

	// ── Echo ──────────────────────────────────────────────────────────────────
	//
	// Every other input is returned verbatim. HasOutput=true ensures the
	// framework always prints the line, even if it is an empty string.
	return Result{
		Tag:       "ok",
		Output:    input,
		HasOutput: true,
	}
}
