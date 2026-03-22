// =========================================================================
// sleep — Delay for a Specified Amount of Time
// =========================================================================
//
// The `sleep` utility pauses execution for a specified duration. It's one
// of the most commonly used utilities in shell scripting for introducing
// delays between operations.
//
// # Basic usage
//
//   sleep 5        # sleep for 5 seconds
//   sleep 2.5      # sleep for 2.5 seconds (fractional)
//   sleep 1m       # sleep for 1 minute
//   sleep 1h       # sleep for 1 hour
//   sleep 1d       # sleep for 1 day
//
// # Multiple durations
//
// Multiple arguments are summed together:
//
//   sleep 1m 30s   # sleep for 1 minute and 30 seconds (90 seconds)
//   sleep 1h 30m   # sleep for 1 hour and 30 minutes
//
// # Duration suffixes
//
//   ┌────────┬────────────────────────────────────────┐
//   │ Suffix │ Meaning                                │
//   ├────────┼────────────────────────────────────────┤
//   │ s      │ Seconds (default if no suffix given)   │
//   │ m      │ Minutes (60 seconds)                   │
//   │ h      │ Hours (3600 seconds)                   │
//   │ d      │ Days (86400 seconds)                   │
//   └────────┴────────────────────────────────────────┘
//
// # Architecture
//
//   sleep.json (spec)          sleep_tool.go (this file)
//   ┌──────────────────┐     ┌──────────────────────────────┐
//   │ variadic STRING   │     │ parse each duration string    │
//   │ (required, min 1) │────>│ sum all durations             │
//   │ help, version     │     │ call time.Sleep()             │
//   └──────────────────┘     └──────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"strconv"
	"strings"
	"time"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// parseDuration — parse a single duration string into seconds
// =========================================================================
//
// This function takes a string like "5", "2.5s", "1m", "3h", or "7d"
// and returns the equivalent number of seconds as a float64.
//
// If no suffix is provided, seconds is assumed (the default).
//
// Examples:
//   parseDuration("5")    => 5.0, nil
//   parseDuration("2.5s") => 2.5, nil
//   parseDuration("1m")   => 60.0, nil
//   parseDuration("3h")   => 10800.0, nil
//   parseDuration("7d")   => 604800.0, nil
//   parseDuration("")     => 0, error
//   parseDuration("abc")  => 0, error

func parseDuration(s string) (float64, error) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0, fmt.Errorf("empty duration string")
	}

	// Check if the last character is a known suffix.
	// If so, strip it off and apply the multiplier.
	multiplier := 1.0
	last := s[len(s)-1]

	switch last {
	case 's':
		// Seconds — the default, multiplier stays 1.
		s = s[:len(s)-1]
	case 'm':
		// Minutes — 60 seconds per minute.
		multiplier = 60.0
		s = s[:len(s)-1]
	case 'h':
		// Hours — 3600 seconds per hour.
		multiplier = 3600.0
		s = s[:len(s)-1]
	case 'd':
		// Days — 86400 seconds per day.
		multiplier = 86400.0
		s = s[:len(s)-1]
	default:
		// No suffix — assume seconds.
		// The last character should be a digit or dot.
	}

	// Parse the numeric part.
	if s == "" {
		return 0, fmt.Errorf("missing numeric value in duration")
	}

	val, err := strconv.ParseFloat(s, 64)
	if err != nil {
		return 0, fmt.Errorf("invalid duration number %q: %w", s, err)
	}

	if val < 0 {
		return 0, fmt.Errorf("negative duration: %f", val)
	}

	return val * multiplier, nil
}

// =========================================================================
// runSleep — the testable entry point for the sleep tool
// =========================================================================
//
// The sleep tool:
//   1. Parses arguments using cli-builder.
//   2. Parses each duration string and sums them.
//   3. Calls time.Sleep() with the total duration.
//   4. Returns 0 on success, 1 on error.

func runSleep(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "sleep: %s\n", err)
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
		// Extract the variadic "duration" argument.
		var durations []string
		if args, ok := r.Arguments["duration"]; ok {
			if argSlice, ok := args.([]interface{}); ok {
				for _, a := range argSlice {
					if s, ok := a.(string); ok {
						durations = append(durations, s)
					}
				}
			}
		}

		if len(durations) == 0 {
			fmt.Fprintln(stderr, "sleep: missing operand")
			return 1
		}

		// Parse and sum all duration values.
		totalSeconds := 0.0
		for _, d := range durations {
			secs, err := parseDuration(d)
			if err != nil {
				fmt.Fprintf(stderr, "sleep: invalid time interval %q: %s\n", d, err)
				return 1
			}
			totalSeconds += secs
		}

		// Sleep for the total duration.
		// time.Duration is in nanoseconds, so we multiply by 1e9.
		time.Sleep(time.Duration(totalSeconds * float64(time.Second)))
		return 0

	default:
		fmt.Fprintf(stderr, "sleep: unexpected result type: %T\n", result)
		return 1
	}
}
