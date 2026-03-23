// =========================================================================
// cmp — Compare Two Files Byte by Byte
// =========================================================================
//
// The `cmp` utility compares two files byte by byte and reports the
// first location where they differ. Unlike `diff`, which works on lines,
// `cmp` works at the byte level — making it suitable for binary files.
//
// # Basic usage
//
//   cmp file1 file2          Report first difference
//   cmp -l file1 file2       List all differing bytes
//   cmp -s file1 file2       Silent: exit status only
//
// # Exit codes
//
//   0 — files are identical
//   1 — files differ
//   2 — an error occurred
//
// # How byte comparison works
//
// The algorithm is straightforward:
//   1. Open both files
//   2. Read one byte at a time from each
//   3. Compare the bytes
//   4. Report the first (or all) differences
//
// We track both the byte offset and the line number (by counting
// newlines) to give useful error messages.
//
// # Architecture
//
//   cmp.json (spec)              cmp_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -l,-s,-b  │       │ open both files                  │
//   │ -i,-n            │──────>│ read byte by byte                │
//   │ args: FILE1 FILE2│       │ compare and report differences   │
//   └──────────────────┘       └──────────────────────────────────┘

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
// CmpOptions — configuration for the cmp operation
// =========================================================================

type CmpOptions struct {
	List      bool // -l: list all differences (verbose)
	Silent    bool // -s: suppress all output
	PrintByte bool // -b: print differing bytes as characters
	Skip      int  // -i: skip initial bytes in both files
	MaxBytes  int  // -n: maximum number of bytes to compare (0 = unlimited)
}

// =========================================================================
// cmpFiles — compare two files byte by byte
// =========================================================================
//
// This is the core comparison function. It reads both files simultaneously
// using buffered readers for efficiency, and tracks byte offset and line
// number for reporting.
//
// The function returns:
//   0 if files are identical (within the compared range)
//   1 if files differ
//   2 if an error occurred

func cmpFiles(fileA, fileB string, opts CmpOptions, stdout, stderr io.Writer) int {
	// Open file A.
	fA, err := os.Open(fileA)
	if err != nil {
		if !opts.Silent {
			fmt.Fprintf(stderr, "cmp: %s: %s\n", fileA, err)
		}
		return 2
	}
	defer fA.Close()

	// Open file B.
	fB, err := os.Open(fileB)
	if err != nil {
		if !opts.Silent {
			fmt.Fprintf(stderr, "cmp: %s: %s\n", fileB, err)
		}
		return 2
	}
	defer fB.Close()

	readerA := bufio.NewReader(fA)
	readerB := bufio.NewReader(fB)

	// Skip initial bytes if requested (-i flag).
	// We read and discard the specified number of bytes from each file.
	for i := 0; i < opts.Skip; i++ {
		_, errA := readerA.ReadByte()
		_, errB := readerB.ReadByte()
		if errA != nil || errB != nil {
			break
		}
	}

	// Compare bytes one at a time.
	byteOffset := int64(1) // 1-based, as is traditional for cmp
	lineNum := int64(1)    // 1-based line numbers
	hasDiff := false
	bytesRead := 0

	for {
		// Check byte limit.
		if opts.MaxBytes > 0 && bytesRead >= opts.MaxBytes {
			break
		}

		byteA, errA := readerA.ReadByte()
		byteB, errB := readerB.ReadByte()

		// Both files ended at the same point — they're identical.
		if errA == io.EOF && errB == io.EOF {
			break
		}

		// One file ended before the other.
		if errA == io.EOF {
			if !opts.Silent {
				fmt.Fprintf(stderr, "cmp: EOF on %s after byte %d, line %d\n",
					fileA, byteOffset-1, lineNum)
			}
			return 1
		}
		if errB == io.EOF {
			if !opts.Silent {
				fmt.Fprintf(stderr, "cmp: EOF on %s after byte %d, line %d\n",
					fileB, byteOffset-1, lineNum)
			}
			return 1
		}

		// Handle read errors.
		if errA != nil {
			if !opts.Silent {
				fmt.Fprintf(stderr, "cmp: %s: %s\n", fileA, errA)
			}
			return 2
		}
		if errB != nil {
			if !opts.Silent {
				fmt.Fprintf(stderr, "cmp: %s: %s\n", fileB, errB)
			}
			return 2
		}

		bytesRead++

		// Compare the bytes.
		if byteA != byteB {
			hasDiff = true

			if opts.Silent {
				// In silent mode, exit as soon as we find a difference.
				return 1
			}

			if opts.List {
				// Verbose mode: print every difference.
				// Format: byte_offset octal_A octal_B
				if opts.PrintByte {
					fmt.Fprintf(stdout, "%d %3o %-4c %3o %c\n",
						byteOffset, byteA, byteA, byteB, byteB)
				} else {
					fmt.Fprintf(stdout, "%d %3o %3o\n",
						byteOffset, byteA, byteB)
				}
			} else {
				// Default mode: report just the first difference.
				if opts.PrintByte {
					fmt.Fprintf(stdout, "%s %s differ: byte %d, line %d is %3o %c %3o %c\n",
						fileA, fileB, byteOffset, lineNum, byteA, byteA, byteB, byteB)
				} else {
					fmt.Fprintf(stdout, "%s %s differ: byte %d, line %d\n",
						fileA, fileB, byteOffset, lineNum)
				}
				return 1
			}
		}

		// Track line numbers by counting newlines.
		if byteA == '\n' {
			lineNum++
		}

		byteOffset++
	}

	if hasDiff {
		return 1
	}
	return 0
}

// =========================================================================
// runCmp — the testable core of the cmp tool
// =========================================================================

func runCmp(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runCmpWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

func runCmpWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "cmp: %s\n", err)
		return 2
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 2
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
		opts := CmpOptions{
			List:      getBool(r.Flags, "list"),
			Silent:    getBool(r.Flags, "silent"),
			PrintByte: getBool(r.Flags, "print_bytes"),
		}

		// Parse skip count (-i flag).
		if skipStr, ok := r.Flags["ignore_initial"].(string); ok && skipStr != "" {
			// Support SKIP1:SKIP2 format, but we'll use same skip for both.
			parts := strings.SplitN(skipStr, ":", 2)
			if n, err := strconv.Atoi(parts[0]); err == nil {
				opts.Skip = n
			}
		}

		// Parse max bytes (-n flag).
		if n, ok := getInt(r.Flags, "max_bytes"); ok {
			opts.MaxBytes = n
		}

		// Extract file arguments.
		fileA, _ := r.Arguments["file1"].(string)
		fileB, _ := r.Arguments["file2"].(string)

		if fileA == "" {
			fmt.Fprintf(stderr, "cmp: missing operand\n")
			return 2
		}

		// If file2 is missing or "-", use stdin by writing it to a temp file.
		if fileB == "" || fileB == "-" {
			tmpFile, err := os.CreateTemp("", "cmp-stdin-*")
			if err != nil {
				if !opts.Silent {
					fmt.Fprintf(stderr, "cmp: cannot create temp file: %s\n", err)
				}
				return 2
			}
			defer os.Remove(tmpFile.Name())
			defer tmpFile.Close()
			if _, err := io.Copy(tmpFile, stdin); err != nil {
				if !opts.Silent {
					fmt.Fprintf(stderr, "cmp: error reading stdin: %s\n", err)
				}
				return 2
			}
			tmpFile.Close()
			fileB = tmpFile.Name()
		}

		return cmpFiles(fileA, fileB, opts, stdout, stderr)

	default:
		fmt.Fprintf(stderr, "cmp: unexpected result type: %T\n", result)
		return 2
	}
}
