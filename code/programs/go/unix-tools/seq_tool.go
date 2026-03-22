// =========================================================================
// seq — Print a Sequence of Numbers
// =========================================================================
//
// The `seq` utility prints a sequence of numbers, one per line (by default).
// It's commonly used in shell scripts for loop iteration.
//
// # Basic usage
//
//   seq 5               =>  1, 2, 3, 4, 5  (one per line)
//   seq 2 5             =>  2, 3, 4, 5
//   seq 1 2 10          =>  1, 3, 5, 7, 9
//   seq 5 -1 1          =>  5, 4, 3, 2, 1  (counting down)
//
// # Argument patterns
//
// seq accepts 1, 2, or 3 positional arguments:
//
//   ┌───────────────────────┬───────────┬───────────┬──────┐
//   │ Arguments             │ FIRST     │ INCREMENT │ LAST │
//   ├───────────────────────┼───────────┼───────────┼──────┤
//   │ seq 5                 │ 1         │ 1         │ 5    │
//   │ seq 2 5               │ 2         │ 1         │ 5    │
//   │ seq 1 2 10            │ 1         │ 2         │ 10   │
//   └───────────────────────┴───────────┴───────────┴──────┘
//
// # Flags
//
//   -s STRING    Use STRING as separator (default: newline).
//   -w           Equalize width by padding with leading zeros.
//                For example: seq -w 1 10 => 01, 02, ..., 10
//   -f FORMAT    Use a printf-style format string (e.g., "%03g").
//
// # Architecture
//
//   seq.json (spec)           seq_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -s,-w,-f  │      │ parse FIRST, INCREMENT, LAST   │
//   │ variadic NUMBER  │─────>│ generate number sequence       │
//   │ help, version    │      │ format and output               │
//   └──────────────────┘      └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"math"
	"strconv"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// decimalPlaces — count the number of decimal places in a number string
// =========================================================================
//
// Used for the equal-width (-w) flag to determine zero-padding width.

func decimalPlaces(s string) int {
	dot := strings.Index(s, ".")
	if dot == -1 {
		return 0
	}
	return len(s) - dot - 1
}

// =========================================================================
// formatSeqNumber — format a number for seq output
// =========================================================================
//
// This function formats a floating-point number for display. If the number
// is an integer (no fractional part), it's displayed without decimal places.
// Otherwise, it shows the appropriate number of decimal places.

func formatSeqNumber(n float64, useFormat bool, format string, equalWidth bool, width int) string {
	if useFormat {
		return fmt.Sprintf(format, n)
	}

	// Check if the number is effectively an integer.
	if n == math.Trunc(n) && !strings.Contains(fmt.Sprintf("%g", n), ".") {
		s := strconv.Itoa(int(n))
		if equalWidth {
			for len(s) < width {
				s = "0" + s
			}
		}
		return s
	}

	// Float formatting — use %g for compact representation.
	s := fmt.Sprintf("%g", n)
	if equalWidth {
		for len(s) < width {
			s = "0" + s
		}
	}
	return s
}

// =========================================================================
// runSeq — the testable core of the seq tool
// =========================================================================

func runSeq(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "seq: %s\n", err)
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
		equalWidth := getBool(r.Flags, "equal_width")

		separator := "\n"
		if v, ok := r.Flags["separator"]; ok {
			if s, ok := v.(string); ok {
				separator = s
			}
		}

		useFormat := false
		format := ""
		if v, ok := r.Flags["format"]; ok {
			if s, ok := v.(string); ok && s != "" {
				useFormat = true
				format = s
			}
		}

		// Extract positional arguments (numbers).
		numbers := getStringSlice(r.Arguments, "numbers")
		if len(numbers) == 0 || len(numbers) > 3 {
			fmt.Fprintf(stderr, "seq: expected 1-3 arguments, got %d\n", len(numbers))
			return 1
		}

		// Parse the 1-3 number arguments into FIRST, INCREMENT, LAST.
		//
		// Pattern matching:
		//   1 arg:  seq LAST        => FIRST=1, INC=1, LAST=arg
		//   2 args: seq FIRST LAST  => FIRST=arg1, INC=1, LAST=arg2
		//   3 args: seq FIRST INC LAST
		var first, increment, last float64

		switch len(numbers) {
		case 1:
			last, err = strconv.ParseFloat(numbers[0], 64)
			if err != nil {
				fmt.Fprintf(stderr, "seq: invalid floating point argument: %q\n", numbers[0])
				return 1
			}
			first = 1
			increment = 1
		case 2:
			first, err = strconv.ParseFloat(numbers[0], 64)
			if err != nil {
				fmt.Fprintf(stderr, "seq: invalid floating point argument: %q\n", numbers[0])
				return 1
			}
			last, err = strconv.ParseFloat(numbers[1], 64)
			if err != nil {
				fmt.Fprintf(stderr, "seq: invalid floating point argument: %q\n", numbers[1])
				return 1
			}
			increment = 1
		case 3:
			first, err = strconv.ParseFloat(numbers[0], 64)
			if err != nil {
				fmt.Fprintf(stderr, "seq: invalid floating point argument: %q\n", numbers[0])
				return 1
			}
			increment, err = strconv.ParseFloat(numbers[1], 64)
			if err != nil {
				fmt.Fprintf(stderr, "seq: invalid floating point argument: %q\n", numbers[1])
				return 1
			}
			last, err = strconv.ParseFloat(numbers[2], 64)
			if err != nil {
				fmt.Fprintf(stderr, "seq: invalid floating point argument: %q\n", numbers[2])
				return 1
			}
		}

		// Validate: increment must not be zero.
		if increment == 0 {
			fmt.Fprintf(stderr, "seq: zero increment\n")
			return 1
		}

		// Determine the width for equal-width mode.
		width := 0
		if equalWidth {
			// Width is determined by the wider of FIRST and LAST.
			firstStr := formatSeqNumber(first, false, "", false, 0)
			lastStr := formatSeqNumber(last, false, "", false, 0)
			width = len(firstStr)
			if len(lastStr) > width {
				width = len(lastStr)
			}
		}

		// Generate and output the sequence.
		//
		// We use a loop that checks direction:
		//   - If increment > 0, continue while current <= last
		//   - If increment < 0, continue while current >= last
		//
		// We use a small epsilon for floating-point comparison to avoid
		// missing the last value due to rounding errors.
		isFirst := true
		current := first
		epsilon := math.Abs(increment) * 1e-10

		for {
			// Check termination condition.
			if increment > 0 && current > last+epsilon {
				break
			}
			if increment < 0 && current < last-epsilon {
				break
			}

			// Print separator before all values except the first.
			if !isFirst {
				fmt.Fprint(stdout, separator)
			}
			isFirst = false

			// Format and print the number.
			fmt.Fprint(stdout, formatSeqNumber(current, useFormat, format, equalWidth, width))

			current += increment
		}

		// Print a final newline (unless the separator is not a newline).
		if !isFirst {
			fmt.Fprintln(stdout)
		}

		return 0

	default:
		fmt.Fprintf(stderr, "seq: unexpected result type: %T\n", result)
		return 1
	}
}
