// =========================================================================
// echo — Display a Line of Text
// =========================================================================
//
// The `echo` utility writes its arguments to standard output, separated
// by spaces, followed by a newline. It's one of the most commonly used
// commands in shell scripting.
//
// # Basic behavior
//
//   echo hello world        =>  "hello world\n"
//   echo                    =>  "\n"  (just a newline)
//   echo -n hello           =>  "hello"  (no trailing newline)
//
// # Flags
//
//   -n    Do not output the trailing newline. Useful when building
//         prompts or combining output with other commands:
//           echo -n "Enter name: "
//           read name
//
//   -e    Enable interpretation of backslash escapes. Without this flag,
//         backslashes are printed literally:
//           echo "hello\nworld"   => "hello\nworld"   (literal)
//           echo -e "hello\nworld" => "hello"          (with newline)
//                                    "world"
//
//   -E    Disable interpretation of backslash escapes (the default).
//         This exists so you can explicitly override -e if needed.
//
// # Supported escape sequences (with -e)
//
//	┌──────────┬──────────────────────────────────────┐
//	│ Escape   │ Meaning                              │
//	├──────────┼──────────────────────────────────────┤
//	│ \\       │ Backslash                            │
//	│ \a       │ Alert (bell, BEL, 0x07)              │
//	│ \b       │ Backspace (0x08)                     │
//	│ \f       │ Form feed (0x0C)                     │
//	│ \n       │ Newline (0x0A)                       │
//	│ \r       │ Carriage return (0x0D)               │
//	│ \t       │ Horizontal tab (0x09)                │
//	│ \v       │ Vertical tab (0x0B)                  │
//	│ \0nnn    │ Octal value (0-3 octal digits)       │
//	└──────────┴──────────────────────────────────────┘
//
// # Architecture
//
//	echo.json (spec)          echo_tool.go (this file)
//	┌──────────────────┐     ┌──────────────────────────────┐
//	│ flags: -n, -e, -E│     │ join args with spaces         │
//	│ variadic STRING  │────>│ if -e: process escapes        │
//	│ help, version    │     │ if !-n: append newline        │
//	└──────────────────┘     └──────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// processEscapes — interpret backslash escape sequences
// =========================================================================
//
// This function takes a string containing backslash escape sequences
// (like "\n", "\t", "\\") and returns a new string with those sequences
// replaced by their actual character values.
//
// For example:
//   processEscapes(`hello\nworld`)  =>  "hello\nworld"  (with real newline)
//   processEscapes(`tab\there`)     =>  "tab\there"     (with real tab)
//   processEscapes(`back\\slash`)   =>  "back\slash"    (single backslash)
//
// This mirrors the behavior of GNU echo with the -e flag enabled.
//
// The function processes the string character by character. When it
// encounters a backslash, it looks at the next character to determine
// which escape sequence to produce. If the character after the backslash
// is not a recognized escape, both the backslash and the character are
// kept as-is.

func processEscapes(s string) string {
	// We use a strings.Builder for efficient string concatenation.
	// Unlike string += which creates a new string each time (O(n^2) total),
	// Builder accumulates bytes in a buffer and builds the string once (O(n)).
	var result strings.Builder

	// We need index-based iteration because some escapes consume more
	// than one character (e.g., \0nnn consumes up to 4 characters).
	i := 0
	for i < len(s) {
		// If this character is not a backslash, copy it directly.
		if s[i] != '\\' {
			result.WriteByte(s[i])
			i++
			continue
		}

		// We found a backslash. Check if there's a character after it.
		// A trailing backslash (at the end of the string) is kept as-is.
		if i+1 >= len(s) {
			result.WriteByte('\\')
			i++
			continue
		}

		// Look at the character after the backslash to determine the
		// escape sequence.
		next := s[i+1]
		switch next {
		case '\\':
			// \\ => literal backslash
			result.WriteByte('\\')
			i += 2
		case 'a':
			// \a => alert (bell character, ASCII 7)
			// This makes the terminal beep. Rarely used in modern programs
			// but part of the POSIX echo spec.
			result.WriteByte('\a')
			i += 2
		case 'b':
			// \b => backspace (ASCII 8)
			// Moves the cursor one position to the left. Used in text
			// formatting and progress indicators.
			result.WriteByte('\b')
			i += 2
		case 'f':
			// \f => form feed (ASCII 12)
			// Originally meant "advance to the next page" on printers.
			// In terminals, it usually clears the screen.
			result.WriteByte('\f')
			i += 2
		case 'n':
			// \n => newline (ASCII 10)
			// The most common escape. Starts a new line.
			result.WriteByte('\n')
			i += 2
		case 'r':
			// \r => carriage return (ASCII 13)
			// Moves the cursor to the beginning of the current line.
			// Used in progress bars: echo -e "Loading...\rDone!     "
			result.WriteByte('\r')
			i += 2
		case 't':
			// \t => horizontal tab (ASCII 9)
			// Advances to the next tab stop (usually every 8 columns).
			result.WriteByte('\t')
			i += 2
		case 'v':
			// \v => vertical tab (ASCII 11)
			// Advances to the next vertical tab stop. Rarely used today.
			result.WriteByte('\v')
			i += 2
		case '0':
			// \0nnn => octal value
			// Interprets up to 3 octal digits after \0 as a byte value.
			// Examples:
			//   \0101 => 'A' (65 in decimal, 101 in octal)
			//   \0    => null byte (0)
			//   \07   => bell (7)
			i += 2 // skip past \0
			val := byte(0)
			digits := 0
			for digits < 3 && i < len(s) && s[i] >= '0' && s[i] <= '7' {
				val = val*8 + (s[i] - '0')
				i++
				digits++
			}
			result.WriteByte(val)
		default:
			// Unrecognized escape sequence — keep both the backslash
			// and the following character as-is.
			result.WriteByte('\\')
			result.WriteByte(next)
			i += 2
		}
	}

	return result.String()
}

// =========================================================================
// runEcho — the testable core of the echo tool
// =========================================================================
//
// The echo tool:
//   1. Joins all positional arguments with spaces.
//   2. If -e is set, processes backslash escape sequences.
//   3. Writes the result to stdout.
//   4. If -n is NOT set, appends a trailing newline.
//   5. Always returns 0 (echo never fails in normal operation).

func runEcho(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "echo: %s\n", err)
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
		// Extract flags.
		//
		// noNewline: if true, suppress the trailing newline.
		// enableEscapes: if true, interpret backslash sequences.
		noNewline, _ := r.Flags["no_newline"].(bool)
		enableEscapes, _ := r.Flags["enable_escapes"].(bool)

		// Extract the positional arguments (the strings to echo).
		//
		// The "strings" argument is variadic, so it comes back as a
		// []interface{} (a slice of interface values). We need to convert
		// each element to a string.
		var parts []string
		if args, ok := r.Arguments["strings"]; ok {
			if argSlice, ok := args.([]interface{}); ok {
				for _, a := range argSlice {
					if s, ok := a.(string); ok {
						parts = append(parts, s)
					}
				}
			}
		}

		// Join all parts with spaces, just like the real echo does.
		// If no arguments were given, output is just "" (empty).
		output := strings.Join(parts, " ")

		// If -e is enabled, process escape sequences in the output.
		if enableEscapes {
			output = processEscapes(output)
		}

		// Write the output. Add a trailing newline unless -n was set.
		if noNewline {
			fmt.Fprint(stdout, output)
		} else {
			fmt.Fprintln(stdout, output)
		}

		return 0

	default:
		fmt.Fprintf(stderr, "echo: unexpected result type: %T\n", result)
		return 1
	}
}
