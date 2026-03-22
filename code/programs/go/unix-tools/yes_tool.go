// =========================================================================
// yes — Repeatedly Output a Line
// =========================================================================
//
// The `yes` utility outputs a string repeatedly until killed. By default,
// it outputs "y" — which is useful for piping into programs that ask for
// confirmation:
//
//   yes | rm -i *.tmp      # auto-answer "y" to every prompt
//
// You can also specify a custom string:
//
//   yes "I agree" | head -5    # prints "I agree" five times
//
// Multiple arguments are joined with spaces, just like echo:
//
//   yes hello world            # prints "hello world" repeatedly
//
// # Why does yes exist?
//
// Many Unix programs prompt for confirmation on each action (e.g.,
// `rm -i`, `fsck`). When you know you want to answer "yes" to every
// question, piping `yes` into the command automates that. It's a simple
// but elegant solution to interactive-to-batch conversion.
//
// # Architecture
//
//   yes.json (spec)           yes_tool.go (this file)
//   ┌──────────────────┐     ┌──────────────────────────────┐
//   │ variadic STRING   │     │ join args with spaces         │
//   │ default: "y"      │────>│ or default to "y"             │
//   │ help, version     │     │ print line repeatedly         │
//   └──────────────────┘     └──────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// yesOutput — the testable core output function
// =========================================================================
//
// This function writes the output string repeatedly, up to maxLines times.
// In production, maxLines is set to 0 (unlimited), but for testing we can
// cap the output to verify correct behavior without infinite loops.
//
// Parameters:
//   - args: the strings to output (joined by spaces). If empty, defaults to "y".
//   - writer: where to write output (stdout in production, buffer in tests).
//   - maxLines: maximum number of lines to output. 0 means unlimited.

func yesOutput(args []string, writer io.Writer, maxLines int) {
	// Determine the output line. If no arguments were given, default to "y".
	// If arguments were given, join them with spaces (same behavior as echo).
	line := "y"
	if len(args) > 0 {
		line = strings.Join(args, " ")
	}

	// Output the line repeatedly. If maxLines is 0, loop forever (the caller
	// will typically kill the process with a signal or broken pipe).
	if maxLines <= 0 {
		for {
			_, err := fmt.Fprintln(writer, line)
			if err != nil {
				// Broken pipe or other write error — exit silently.
				// This is normal when piped into `head` or similar.
				return
			}
		}
	}

	// When maxLines is set (for testing), output exactly that many lines.
	for i := 0; i < maxLines; i++ {
		_, err := fmt.Fprintln(writer, line)
		if err != nil {
			return
		}
	}
}

// =========================================================================
// runYes — the testable entry point for the yes tool
// =========================================================================
//
// The yes tool:
//   1. Parses arguments using cli-builder.
//   2. Extracts the variadic "string" argument (or defaults to "y").
//   3. Calls yesOutput to print the line repeatedly.
//   4. Always returns 0 (yes never fails in normal operation).

func runYes(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "yes: %s\n", err)
		return 1
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		// Extract the variadic "string" argument.
		// The spec declares it as variadic with default "y", but the parser
		// returns whatever the user passed. If nothing was passed, we get
		// an empty slice and default to "y" in yesOutput.
		var parts []string
		if args, ok := r.Arguments["string"]; ok {
			if argSlice, ok := args.([]interface{}); ok {
				for _, a := range argSlice {
					if s, ok := a.(string); ok {
						parts = append(parts, s)
					}
				}
			}
		}

		// In production, maxLines=0 means infinite output.
		// The process will be terminated by SIGPIPE when the reader closes.
		yesOutput(parts, stdout, 0)
		return 0

	default:
		fmt.Fprintf(stderr, "yes: unexpected result type: %T\n", result)
		return 1
	}
}
