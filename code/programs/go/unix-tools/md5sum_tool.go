// =========================================================================
// md5sum — Compute and Check MD5 Message Digests
// =========================================================================
//
// The `md5sum` utility computes or verifies MD5 (128-bit) checksums.
// MD5 is a cryptographic hash function that produces a 32-character
// hexadecimal fingerprint for any input data.
//
// # What is MD5?
//
// MD5 (Message Digest 5) was designed by Ronald Rivest in 1991. It takes
// an arbitrary-length input and produces a fixed 128-bit (16-byte) hash.
//
//   Input:  "Hello, World!\n"
//   MD5:    bea8252ff4e80f41719ea13cdf007273
//
// # IMPORTANT: MD5 is broken for security
//
// MD5 is cryptographically broken — collisions can be found in seconds.
// DO NOT use MD5 for security purposes (password hashing, digital
// signatures, etc.). Use SHA-256 or better for security.
//
// MD5 is still useful for:
//   - Verifying file integrity (accidental corruption, not malicious)
//   - Checksumming backups
//   - Quick data deduplication
//
// # Usage modes
//
//   1. Compute mode (default):
//      $ md5sum file1.txt file2.txt
//      d41d8cd98f00b204e9800998ecf8427e  file1.txt
//      7d793037a0760186574b0282f2f435e7  file2.txt
//
//   2. Check mode (-c):
//      $ md5sum -c checksums.txt
//      file1.txt: OK
//      file2.txt: FAILED
//
// # Architecture
//
//   md5sum.json (spec)           md5sum_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -c,-b,-t  │       │ compute: hash each file          │
//   │ --quiet,--status │──────>│ check: read sums, verify files   │
//   │ --strict,--warn  │       │ output: hash + filename          │
//   │ -z               │       │                                  │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"bufio"
	"crypto/md5"
	"fmt"
	"io"
	"os"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// computeMD5 — compute the MD5 hash of data from a reader
// =========================================================================
//
// Reads all data from the reader and returns the hex-encoded MD5 hash.
// Uses io.Copy for efficient streaming — the entire file doesn't need
// to fit in memory.

func computeMD5(reader io.Reader) (string, error) {
	hasher := md5.New()
	if _, err := io.Copy(hasher, reader); err != nil {
		return "", err
	}
	return fmt.Sprintf("%x", hasher.Sum(nil)), nil
}

// =========================================================================
// CheckResult — result of verifying one checksum line
// =========================================================================

type CheckResult struct {
	Filename string // the filename from the checksum file
	OK       bool   // whether the hash matched
	Err      error  // any error opening/reading the file
}

// =========================================================================
// checkMD5 — verify checksums from a check file
// =========================================================================
//
// Each line in the check file has the format:
//   <hash>  <filename>
//   or
//   <hash> *<filename>  (binary mode indicator)
//
// We compute the MD5 of each file and compare it to the expected hash.

func checkMD5(reader io.Reader) ([]CheckResult, error) {
	scanner := bufio.NewScanner(reader)
	var results []CheckResult

	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			continue
		}

		hash, filename, err := parseChecksumLine(line)
		if err != nil {
			results = append(results, CheckResult{Filename: line, OK: false, Err: err})
			continue
		}

		// Open the file and compute its hash.
		f, err := os.Open(filename)
		if err != nil {
			results = append(results, CheckResult{Filename: filename, OK: false, Err: err})
			continue
		}

		actual, err := computeMD5(f)
		f.Close()
		if err != nil {
			results = append(results, CheckResult{Filename: filename, OK: false, Err: err})
			continue
		}

		results = append(results, CheckResult{
			Filename: filename,
			OK:       actual == hash,
		})
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	return results, nil
}

// =========================================================================
// parseChecksumLine — parse a single line from a checksum file
// =========================================================================
//
// Formats supported:
//   hash  filename     (text mode, two spaces)
//   hash *filename     (binary mode, space + asterisk)

func parseChecksumLine(line string) (hash, filename string, err error) {
	// The hash is always the first 32 hex characters for MD5.
	// But we'll be more flexible and split on whitespace.
	parts := strings.SplitN(line, " ", 2)
	if len(parts) != 2 {
		return "", "", fmt.Errorf("invalid checksum line: %s", line)
	}

	hash = parts[0]
	filename = strings.TrimLeft(parts[1], " *")

	if hash == "" || filename == "" {
		return "", "", fmt.Errorf("invalid checksum line: %s", line)
	}

	return hash, filename, nil
}

// =========================================================================
// runMd5sum — the testable core of the md5sum tool
// =========================================================================

func runMd5sum(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runMd5sumWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runMd5sumWithStdin is the inner implementation with injectable stdin.

func runMd5sumWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "md5sum: %s\n", err)
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
		checkMode := getBool(r.Flags, "check")
		quiet := getBool(r.Flags, "quiet")
		status := getBool(r.Flags, "status")

		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			files = []string{"-"}
		}

		if checkMode {
			return runMd5sumCheck(files, quiet, status, stdout, stderr, stdin)
		}

		return runMd5sumCompute(files, stdout, stderr, stdin)

	default:
		fmt.Fprintf(stderr, "md5sum: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// runMd5sumCompute — compute MD5 hashes for the given files
// =========================================================================

func runMd5sumCompute(files []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	exitCode := 0

	for _, filename := range files {
		var reader io.Reader
		if filename == "-" {
			reader = stdin
			filename = "-"
		} else {
			f, err := os.Open(filename)
			if err != nil {
				fmt.Fprintf(stderr, "md5sum: %s: %s\n", filename, err)
				exitCode = 1
				continue
			}
			defer f.Close()
			reader = f
		}

		hash, err := computeMD5(reader)
		if err != nil {
			fmt.Fprintf(stderr, "md5sum: %s: %s\n", filename, err)
			exitCode = 1
			continue
		}

		// Output format: "hash  filename\n" (two spaces for text mode).
		fmt.Fprintf(stdout, "%s  %s\n", hash, filename)
	}

	return exitCode
}

// =========================================================================
// runMd5sumCheck — verify MD5 checksums from check files
// =========================================================================

func runMd5sumCheck(files []string, quiet, statusOnly bool, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	exitCode := 0

	for _, filename := range files {
		var reader io.Reader
		if filename == "-" {
			reader = stdin
		} else {
			f, err := os.Open(filename)
			if err != nil {
				fmt.Fprintf(stderr, "md5sum: %s: %s\n", filename, err)
				exitCode = 1
				continue
			}
			defer f.Close()
			reader = f
		}

		results, err := checkMD5(reader)
		if err != nil {
			fmt.Fprintf(stderr, "md5sum: %s: %s\n", filename, err)
			exitCode = 1
			continue
		}

		for _, r := range results {
			if r.Err != nil {
				if !statusOnly {
					fmt.Fprintf(stderr, "md5sum: %s: %s\n", r.Filename, r.Err)
				}
				exitCode = 1
			} else if !r.OK {
				if !statusOnly {
					fmt.Fprintf(stdout, "%s: FAILED\n", r.Filename)
				}
				exitCode = 1
			} else {
				if !statusOnly && !quiet {
					fmt.Fprintf(stdout, "%s: OK\n", r.Filename)
				}
			}
		}
	}

	return exitCode
}
