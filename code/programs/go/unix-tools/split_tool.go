// =========================================================================
// split — Split a File into Pieces
// =========================================================================
//
// The `split` utility splits a file into multiple smaller files. By default,
// it splits into 1000-line chunks, but it can also split by byte count.
//
// # Basic usage
//
//   split file.txt                   Split into 1000-line chunks (xaa, xab, ...)
//   split -l 100 file.txt            Split into 100-line chunks
//   split -b 1024 file.txt           Split into 1024-byte chunks
//   split -l 50 file.txt part_       Custom prefix: part_aa, part_ab, ...
//   split -d file.txt                Numeric suffixes: x00, x01, ...
//
// # Output filenames
//
// Split generates output filenames by combining a prefix (default "x")
// with a suffix. The suffix type depends on flags:
//
//   Default:       alphabetic (aa, ab, ac, ..., az, ba, bb, ...)
//   -d:            numeric (00, 01, 02, ..., 99)
//   -x:            hexadecimal (00, 01, ..., 0f, 10, ...)
//
// The suffix length is controlled by -a (default 2).
//
// # Architecture
//
//   split.json (spec)              split_tool.go (this file)
//   ┌──────────────────┐        ┌──────────────────────────────────┐
//   │ flags: -l,-b,-n  │        │ splitByLines(): line-based split │
//   │ -a,-d,-x         │───────>│ splitByBytes(): byte-based split │
//   │ FILE, PREFIX     │        │ generateSuffix(): name generation│
//   └──────────────────┘        └──────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// SplitOptions — configuration for split operations
// =========================================================================

type SplitOptions struct {
	SuffixLength     int    // Length of the suffix (default 2)
	NumericSuffixes  bool   // Use numeric suffixes (00, 01, ...)
	HexSuffixes      bool   // Use hexadecimal suffixes (00, 01, ..., 0f)
	AdditionalSuffix string // Extra suffix appended to filenames
	Verbose          bool   // Print diagnostic before each output file
}

// =========================================================================
// generateSuffix — generate the suffix for a given file index
// =========================================================================
//
// The suffix generation works differently for each mode:
//
// Alphabetic (default):
//   With suffix length 2: aa, ab, ..., az, ba, bb, ..., zz
//   This is essentially base-26 counting with 'a' as the zero digit.
//   For length N, we can generate 26^N unique suffixes.
//
// Numeric:
//   With suffix length 2: 00, 01, ..., 99
//   Zero-padded decimal numbers.
//
// Hexadecimal:
//   With suffix length 2: 00, 01, ..., ff
//   Zero-padded hexadecimal numbers.

func generateSuffix(index int, opts SplitOptions) string {
	if opts.NumericSuffixes {
		return fmt.Sprintf("%0*d", opts.SuffixLength, index)
	}

	if opts.HexSuffixes {
		return fmt.Sprintf("%0*x", opts.SuffixLength, index)
	}

	// Alphabetic suffix: convert index to base-26.
	// Each position uses letters a-z (26 symbols).
	suffix := make([]byte, opts.SuffixLength)
	val := index
	for i := opts.SuffixLength - 1; i >= 0; i-- {
		suffix[i] = byte('a' + val%26)
		val /= 26
	}

	return string(suffix)
}

// =========================================================================
// splitByLines — split a file by line count
// =========================================================================
//
// This function reads lines from the reader and writes them to output
// files, creating a new output file every `n` lines.
//
// The algorithm:
//   1. Open the first output file
//   2. Read lines one at a time
//   3. Write each line to the current output file
//   4. Every `n` lines, close the current file and open a new one
//   5. Continue until all input is consumed

func splitByLines(reader io.Reader, n int, prefix string, opts SplitOptions, stderr io.Writer) error {
	scanner := bufio.NewScanner(reader)
	fileIndex := 0
	lineCount := 0
	var currentFile *os.File

	for scanner.Scan() {
		// Open a new file when needed.
		if lineCount%n == 0 {
			if currentFile != nil {
				currentFile.Close()
			}

			suffix := generateSuffix(fileIndex, opts)
			filename := prefix + suffix + opts.AdditionalSuffix

			if opts.Verbose {
				fmt.Fprintf(stderr, "creating file '%s'\n", filename)
			}

			var err error
			currentFile, err = os.Create(filename)
			if err != nil {
				return fmt.Errorf("cannot create '%s': %w", filename, err)
			}
			fileIndex++
		}

		// Write the line (with newline).
		fmt.Fprintln(currentFile, scanner.Text())
		lineCount++
	}

	if currentFile != nil {
		currentFile.Close()
	}

	return scanner.Err()
}

// =========================================================================
// splitByBytes — split a file by byte count
// =========================================================================
//
// Similar to splitByLines, but counts bytes instead of lines.
// Uses a fixed-size buffer to read chunks of data.

func splitByBytes(reader io.Reader, n int, prefix string, opts SplitOptions, stderr io.Writer) error {
	buf := make([]byte, n)
	fileIndex := 0

	for {
		// Read up to n bytes.
		bytesRead, err := io.ReadFull(reader, buf)
		if bytesRead == 0 {
			break
		}

		suffix := generateSuffix(fileIndex, opts)
		filename := prefix + suffix + opts.AdditionalSuffix

		if opts.Verbose {
			fmt.Fprintf(stderr, "creating file '%s'\n", filename)
		}

		outFile, createErr := os.Create(filename)
		if createErr != nil {
			return fmt.Errorf("cannot create '%s': %w", filename, createErr)
		}

		_, writeErr := outFile.Write(buf[:bytesRead])
		outFile.Close()
		if writeErr != nil {
			return fmt.Errorf("error writing to '%s': %w", filename, writeErr)
		}

		fileIndex++

		if err != nil {
			if err == io.EOF || err == io.ErrUnexpectedEOF {
				break
			}
			return fmt.Errorf("error reading input: %w", err)
		}
	}

	return nil
}

// =========================================================================
// parseByteSize — parse a byte size string with optional suffix
// =========================================================================
//
// Supports suffixes like:
//   1024    -> 1024 bytes
//   1K      -> 1024 bytes
//   1M      -> 1048576 bytes
//   1G      -> 1073741824 bytes
//   1KB     -> 1000 bytes
//   1MB     -> 1000000 bytes

func parseByteSize(s string) (int, error) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0, fmt.Errorf("empty size")
	}

	multipliers := map[string]int{
		"":   1,
		"B":  1,
		"K":  1024,
		"KB": 1000,
		"M":  1024 * 1024,
		"MB": 1000 * 1000,
		"G":  1024 * 1024 * 1024,
		"GB": 1000 * 1000 * 1000,
	}

	// Find where the number ends and the suffix begins.
	numEnd := len(s)
	for i, c := range s {
		if c < '0' || c > '9' {
			numEnd = i
			break
		}
	}

	numStr := s[:numEnd]
	suffix := strings.ToUpper(s[numEnd:])

	n, err := strconv.Atoi(numStr)
	if err != nil {
		return 0, fmt.Errorf("invalid number: %s", numStr)
	}

	mult, ok := multipliers[suffix]
	if !ok {
		return 0, fmt.Errorf("invalid suffix: %s", suffix)
	}

	return n * mult, nil
}

// =========================================================================
// runSplit — the testable core of the split tool
// =========================================================================

func runSplit(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "split: %s\n", err)
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
		opts := SplitOptions{
			SuffixLength:    2, // Default
			NumericSuffixes: getBool(r.Flags, "numeric_suffixes"),
			HexSuffixes:     getBool(r.Flags, "hex_suffixes"),
			Verbose:         getBool(r.Flags, "verbose"),
		}

		// Suffix length.
		if v := r.Flags["suffix_length"]; v != nil {
			if n, ok := v.(int64); ok {
				suffixLength, err := intFromInt64(n)
				if err != nil {
					fmt.Fprintf(stderr, "split: invalid suffix length: %s\n", err)
					return 1
				}
				opts.SuffixLength = suffixLength
			}
		}

		// Additional suffix.
		if s, ok := r.Flags["additional_suffix"].(string); ok {
			opts.AdditionalSuffix = s
		}

		// Get input file and prefix.
		inputFile := "-"
		prefix := "x"

		if f, ok := r.Arguments["file"].(string); ok && f != "" {
			inputFile = f
		}
		if p, ok := r.Arguments["prefix"].(string); ok && p != "" {
			prefix = p
		}

		// Open input.
		var reader io.Reader
		if inputFile == "-" {
			reader = os.Stdin
		} else {
			f, err := os.Open(inputFile)
			if err != nil {
				fmt.Fprintf(stderr, "split: cannot open '%s': %s\n", inputFile, err)
				return 1
			}
			defer f.Close()
			reader = f
		}

		// Determine split mode: by bytes or by lines.
		if bytesStr, ok := r.Flags["bytes"].(string); ok && bytesStr != "" {
			byteCount, err := parseByteSize(bytesStr)
			if err != nil {
				fmt.Fprintf(stderr, "split: invalid byte count: %s\n", err)
				return 1
			}
			if byteCount <= 0 {
				fmt.Fprintf(stderr, "split: byte count must be positive\n")
				return 1
			}
			err = splitByBytes(reader, byteCount, prefix, opts, stderr)
			if err != nil {
				fmt.Fprintf(stderr, "split: %s\n", err)
				return 1
			}
		} else {
			// Default: split by lines.
			lineCount := 1000
			if v := r.Flags["lines"]; v != nil {
				if n, ok := v.(int64); ok && n > 0 {
					parsedLineCount, err := intFromInt64(n)
					if err != nil {
						fmt.Fprintf(stderr, "split: invalid line count: %s\n", err)
						return 1
					}
					lineCount = parsedLineCount
				}
			}
			err = splitByLines(reader, lineCount, prefix, opts, stderr)
			if err != nil {
				fmt.Fprintf(stderr, "split: %s\n", err)
				return 1
			}
		}

		return 0

	default:
		fmt.Fprintf(stderr, "split: unexpected result type: %T\n", result)
		return 1
	}
}
