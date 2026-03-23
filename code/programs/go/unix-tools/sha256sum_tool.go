// =========================================================================
// sha256sum — Compute and Check SHA-256 Message Digests
// =========================================================================
//
// The `sha256sum` utility computes or verifies SHA-256 (256-bit) checksums.
// SHA-256 is part of the SHA-2 family designed by the NSA.
//
// # SHA-256 vs MD5
//
//   Property        MD5              SHA-256
//   ─────────────   ──────────────   ─────────────────
//   Output bits     128              256
//   Hex chars       32               64
//   Speed           Faster           Slower
//   Security        Broken           Secure (as of 2024)
//   Use case        Integrity only   Integrity + security
//
// # Examples
//
//   $ sha256sum file.txt
//   e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  file.txt
//
//   $ sha256sum -c checksums.txt
//   file.txt: OK
//
// # Architecture
//
//   sha256sum.json (spec)         sha256sum_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ same flags as     │       │ same structure as md5sum         │
//   │ md5sum            │──────>│ but uses crypto/sha256           │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"bufio"
	"crypto/sha256"
	"fmt"
	"io"
	"os"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// computeSHA256 — compute the SHA-256 hash of data from a reader
// =========================================================================
//
// Works exactly like computeMD5 but uses SHA-256. Returns a 64-character
// hex string (256 bits = 32 bytes = 64 hex chars).

func computeSHA256(reader io.Reader) (string, error) {
	hasher := sha256.New()
	if _, err := io.Copy(hasher, reader); err != nil {
		return "", err
	}
	return fmt.Sprintf("%x", hasher.Sum(nil)), nil
}

// =========================================================================
// checkSHA256 — verify SHA-256 checksums from a check file
// =========================================================================
//
// Same format as MD5 check files, just with 64-char hashes instead of 32.

func checkSHA256(reader io.Reader) ([]CheckResult, error) {
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

		f, err := os.Open(filename)
		if err != nil {
			results = append(results, CheckResult{Filename: filename, OK: false, Err: err})
			continue
		}

		actual, err := computeSHA256(f)
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
// runSha256sum — the testable core of the sha256sum tool
// =========================================================================

func runSha256sum(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runSha256sumWithStdin(specPath, argv, stdout, stderr, os.Stdin)
}

// runSha256sumWithStdin is the inner implementation with injectable stdin.

func runSha256sumWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "sha256sum: %s\n", err)
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
			return runSha256sumCheck(files, quiet, status, stdout, stderr, stdin)
		}

		return runSha256sumCompute(files, stdout, stderr, stdin)

	default:
		fmt.Fprintf(stderr, "sha256sum: unexpected result type: %T\n", result)
		return 1
	}
}

// =========================================================================
// runSha256sumCompute — compute SHA-256 hashes for the given files
// =========================================================================

func runSha256sumCompute(files []string, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	exitCode := 0

	for _, filename := range files {
		var reader io.Reader
		if filename == "-" {
			reader = stdin
			filename = "-"
		} else {
			f, err := os.Open(filename)
			if err != nil {
				fmt.Fprintf(stderr, "sha256sum: %s: %s\n", filename, err)
				exitCode = 1
				continue
			}
			defer f.Close()
			reader = f
		}

		hash, err := computeSHA256(reader)
		if err != nil {
			fmt.Fprintf(stderr, "sha256sum: %s: %s\n", filename, err)
			exitCode = 1
			continue
		}

		fmt.Fprintf(stdout, "%s  %s\n", hash, filename)
	}

	return exitCode
}

// =========================================================================
// runSha256sumCheck — verify SHA-256 checksums from check files
// =========================================================================

func runSha256sumCheck(files []string, quiet, statusOnly bool, stdout io.Writer, stderr io.Writer, stdin io.Reader) int {
	exitCode := 0

	for _, filename := range files {
		var reader io.Reader
		if filename == "-" {
			reader = stdin
		} else {
			f, err := os.Open(filename)
			if err != nil {
				fmt.Fprintf(stderr, "sha256sum: %s: %s\n", filename, err)
				exitCode = 1
				continue
			}
			defer f.Close()
			reader = f
		}

		results, err := checkSHA256(reader)
		if err != nil {
			fmt.Fprintf(stderr, "sha256sum: %s: %s\n", filename, err)
			exitCode = 1
			continue
		}

		for _, r := range results {
			if r.Err != nil {
				if !statusOnly {
					fmt.Fprintf(stderr, "sha256sum: %s: %s\n", r.Filename, r.Err)
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
