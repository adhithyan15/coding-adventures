// =========================================================================
// tr — Translate or Delete Characters
// =========================================================================
//
// The `tr` utility translates, squeezes, or deletes characters from
// standard input. It operates on individual characters, not words or lines.
//
// # Basic usage
//
//   echo "hello" | tr 'a-z' 'A-Z'     Convert lowercase to uppercase
//   echo "hello" | tr -d 'l'          Delete all 'l' characters => "heo"
//   echo "aabbcc" | tr -s 'a-z'       Squeeze repeated chars => "abc"
//   echo "hello" | tr -c 'a-z' '_'    Replace non-lowercase with '_'
//
// # Character sets
//
// tr takes two character set arguments:
//   SET1: characters to translate from (or delete)
//   SET2: characters to translate to
//
// Characters are mapped positionally: SET1[0] -> SET2[0], SET1[1] -> SET2[1], etc.
//
// # Special notations
//
// tr supports ranges like 'a-z' and some POSIX classes. For simplicity,
// this implementation handles:
//   - Literal characters: "abc"
//   - Ranges: "a-z", "A-Z", "0-9"
//
// # Architecture
//
//   tr.json (spec)              tr_tool.go (this file)
//   ┌──────────────────┐      ┌────────────────────────────────┐
//   │ flags: -c,-d,-s  │      │ read stdin char by char         │
//   │ -t               │─────>│ translate/delete/squeeze        │
//   │ args: SET1,SET2  │      │ write to stdout                │
//   └──────────────────┘      └────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// expandSet — expand a character set specification into individual chars
// =========================================================================
//
// This function expands range notations like "a-z" into the full set of
// characters. For example:
//   "a-z"    => "abcdefghijklmnopqrstuvwxyz"
//   "a-d0-3" => "abcd0123"
//   "abc"    => "abc"
//
// The expansion algorithm:
//   1. Walk through the string character by character
//   2. When we see a '-' between two characters, expand the range
//   3. Otherwise, add the literal character

func expandSet(set string) string {
	runes := []rune(set)
	var result []rune

	i := 0
	for i < len(runes) {
		// Check for range notation: "a-z"
		// A range requires at least 3 characters: start, '-', end
		if i+2 < len(runes) && runes[i+1] == '-' {
			start := runes[i]
			end := runes[i+2]

			// Expand the range from start to end (inclusive).
			if start <= end {
				for c := start; c <= end; c++ {
					result = append(result, c)
				}
			} else {
				// Reverse range (e.g., "z-a") — treat as literals.
				result = append(result, runes[i], runes[i+1], runes[i+2])
			}
			i += 3
		} else {
			// Handle backslash escapes for common special characters.
			if runes[i] == '\\' && i+1 < len(runes) {
				switch runes[i+1] {
				case 'n':
					result = append(result, '\n')
					i += 2
					continue
				case 't':
					result = append(result, '\t')
					i += 2
					continue
				case 'r':
					result = append(result, '\r')
					i += 2
					continue
				case '\\':
					result = append(result, '\\')
					i += 2
					continue
				}
			}
			result = append(result, runes[i])
			i++
		}
	}

	return string(result)
}

// =========================================================================
// translateContent — translate characters in content using SET1 and SET2
// =========================================================================
//
// This function maps each character in the input: if it appears in SET1,
// replace it with the corresponding character in SET2. If SET2 is shorter
// than SET1, the last character of SET2 is used for the remaining mappings.

func translateContent(content string, set1, set2 string, complement bool) string {
	expandedSet1 := expandSet(set1)
	expandedSet2 := expandSet(set2)

	var result strings.Builder

	for _, ch := range content {
		idx := strings.IndexRune(expandedSet1, ch)
		inSet := idx >= 0

		// With -c (complement), we translate characters NOT in SET1.
		if complement {
			inSet = !inSet
		}

		if inSet {
			if len(expandedSet2) > 0 {
				set2Runes := []rune(expandedSet2)
				if complement {
					// For complement mode, always use the first char of SET2.
					result.WriteRune(set2Runes[0])
				} else if idx < len(set2Runes) {
					result.WriteRune(set2Runes[idx])
				} else {
					// SET2 is shorter — use the last character.
					result.WriteRune(set2Runes[len(set2Runes)-1])
				}
			}
		} else {
			result.WriteRune(ch)
		}
	}

	return result.String()
}

// =========================================================================
// deleteChars — delete characters in SET1 from content
// =========================================================================

func deleteChars(content, set1 string, complement bool) string {
	expanded := expandSet(set1)
	var result strings.Builder

	for _, ch := range content {
		inSet := strings.ContainsRune(expanded, ch)
		if complement {
			inSet = !inSet
		}
		if !inSet {
			result.WriteRune(ch)
		}
	}

	return result.String()
}

// =========================================================================
// squeezeRepeats — replace runs of repeated characters with single occurrences
// =========================================================================

func squeezeRepeats(content, set string) string {
	expanded := expandSet(set)
	var result strings.Builder
	var lastChar rune
	first := true

	for _, ch := range content {
		if !first && ch == lastChar && strings.ContainsRune(expanded, ch) {
			// Skip repeated characters that are in the squeeze set.
			continue
		}
		result.WriteRune(ch)
		lastChar = ch
		first = false
	}

	return result.String()
}

// =========================================================================
// runTr — the testable core of the tr tool
// =========================================================================

func runTr(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runTrWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runTrWithStdin is the inner implementation that accepts a custom stdin.

func runTrWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "tr: %s\n", err)
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
		complement := getBool(r.Flags, "complement")
		deleteMode := getBool(r.Flags, "delete")
		squeeze := getBool(r.Flags, "squeeze_repeats")

		// Extract SET1 and SET2.
		set1, _ := r.Arguments["set1"].(string)
		set2, _ := r.Arguments["set2"].(string)

		// Read all input from stdin.
		inputBytes, err := io.ReadAll(stdin)
		if err != nil {
			fmt.Fprintf(stderr, "tr: error reading input: %s\n", err)
			return 1
		}
		content := string(inputBytes)

		// Apply the requested transformation.
		var output string

		if deleteMode {
			// -d: delete characters in SET1.
			output = deleteChars(content, set1, complement)
			// If -s is also set, squeeze SET2 characters.
			if squeeze && set2 != "" {
				output = squeezeRepeats(output, set2)
			}
		} else if squeeze && set2 == "" {
			// -s without SET2: squeeze characters in SET1.
			output = squeezeRepeats(content, set1)
		} else {
			// Translate mode: replace SET1 chars with SET2 chars.
			output = translateContent(content, set1, set2, complement)
			// If -s is also set, squeeze SET2 characters.
			if squeeze {
				output = squeezeRepeats(output, set2)
			}
		}

		fmt.Fprint(stdout, output)
		return 0

	default:
		fmt.Fprintf(stderr, "tr: unexpected result type: %T\n", result)
		return 1
	}
}
