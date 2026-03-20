package analyzer

// CLI support for the capability analyzer.
//
// This file provides the RunCLI function that implements three subcommands:
//
//   - detect: Scan Go source files and report all detected capabilities
//   - check:  Compare detected capabilities against a manifest
//   - banned: Scan for banned constructs
//
// The CLI uses Go's standard `flag` package for argument parsing.
// Each subcommand gets its own flag set.
//
// # Why three subcommands?
//
// These map to different stages of the CI pipeline:
//
//   1. "detect" is used during development to understand what capabilities
//      a package uses. Run it to see what you need to declare.
//
//   2. "check" is used in CI to verify that declared capabilities match
//      actual usage. It's the gate that blocks undeclared access.
//
//   3. "banned" is an independent check for constructs that are forbidden
//      regardless of capability declarations.

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
)

// RunCLI is the main entry point for the command-line interface.
//
// It parses the first argument as a subcommand (detect, check, banned)
// and dispatches to the appropriate handler. Output goes to the provided
// writer (typically os.Stdout, but can be a buffer for testing).
//
// Returns an exit code: 0 for success, 1 for errors, 2 for usage errors.
func RunCLI(args []string, out io.Writer) int {
	if len(args) < 1 {
		fmt.Fprintln(out, "Usage: ca-capability-analyzer <command> [options]")
		fmt.Fprintln(out, "")
		fmt.Fprintln(out, "Commands:")
		fmt.Fprintln(out, "  detect   Scan Go files and report detected capabilities")
		fmt.Fprintln(out, "  check    Compare detected capabilities against a manifest")
		fmt.Fprintln(out, "  banned   Scan for banned constructs")
		return 2
	}

	command := args[0]
	remaining := args[1:]

	switch command {
	case "detect":
		return runDetect(remaining, out)
	case "check":
		return runCheck(remaining, out)
	case "banned":
		return runBanned(remaining, out)
	default:
		fmt.Fprintf(out, "Unknown command: %s\n", command)
		fmt.Fprintln(out, "Available commands: detect, check, banned")
		return 2
	}
}

// runDetect implements the "detect" subcommand.
//
// It scans one or more Go source files (or a directory) and prints all
// detected capabilities. Output format is either human-readable text
// or JSON.
//
// Usage:
//
//	ca-capability-analyzer detect [--json] [--exclude-tests] <path> [path...]
func runDetect(args []string, out io.Writer) int {
	fs := flag.NewFlagSet("detect", flag.ContinueOnError)
	fs.SetOutput(out)
	jsonOutput := fs.Bool("json", false, "Output in JSON format")
	excludeTests := fs.Bool("exclude-tests", false, "Exclude test files")

	if err := fs.Parse(args); err != nil {
		return 2
	}

	paths := fs.Args()
	if len(paths) == 0 {
		fmt.Fprintln(out, "Usage: ca-capability-analyzer detect [--json] [--exclude-tests] <path> [path...]")
		return 2
	}

	var allDetected []DetectedCapability
	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			fmt.Fprintf(out, "Error: %v\n", err)
			return 1
		}

		if info.IsDir() {
			detected, err := AnalyzeDirectory(path, *excludeTests)
			if err != nil {
				fmt.Fprintf(out, "Error analyzing directory %s: %v\n", path, err)
				return 1
			}
			allDetected = append(allDetected, detected...)
		} else {
			detected, err := AnalyzeFile(path)
			if err != nil {
				fmt.Fprintf(out, "Error analyzing file %s: %v\n", path, err)
				return 1
			}
			allDetected = append(allDetected, detected...)
		}
	}

	if *jsonOutput {
		enc := json.NewEncoder(out)
		enc.SetIndent("", "  ")
		if err := enc.Encode(allDetected); err != nil {
			fmt.Fprintf(out, "Error encoding JSON: %v\n", err)
			return 1
		}
	} else {
		if len(allDetected) == 0 {
			fmt.Fprintln(out, "No capabilities detected (pure code).")
		} else {
			fmt.Fprintf(out, "Detected %d capability(ies):\n", len(allDetected))
			for _, cap := range allDetected {
				fmt.Fprintf(out, "  %s:%d: %s (%s)\n", cap.File, cap.Line, cap.String(), cap.Evidence)
			}
		}
	}

	return 0
}

// runCheck implements the "check" subcommand.
//
// It scans source files for capabilities, loads a manifest, and compares
// them. Exits with code 0 if all capabilities are declared, 1 if there
// are violations.
//
// Usage:
//
//	ca-capability-analyzer check --manifest <path> [--exclude-tests] <path> [path...]
func runCheck(args []string, out io.Writer) int {
	fs := flag.NewFlagSet("check", flag.ContinueOnError)
	fs.SetOutput(out)
	manifestPath := fs.String("manifest", "", "Path to required_capabilities.json")
	excludeTests := fs.Bool("exclude-tests", false, "Exclude test files")
	jsonOutput := fs.Bool("json", false, "Output in JSON format")

	if err := fs.Parse(args); err != nil {
		return 2
	}

	if *manifestPath == "" {
		fmt.Fprintln(out, "Usage: ca-capability-analyzer check --manifest <path> [--exclude-tests] <path> [path...]")
		return 2
	}

	paths := fs.Args()
	if len(paths) == 0 {
		fmt.Fprintln(out, "Usage: ca-capability-analyzer check --manifest <path> [--exclude-tests] <path> [path...]")
		return 2
	}

	// Load manifest
	manifest, err := LoadManifest(*manifestPath)
	if err != nil {
		fmt.Fprintf(out, "Error loading manifest: %v\n", err)
		return 1
	}

	// Detect capabilities
	var allDetected []DetectedCapability
	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			fmt.Fprintf(out, "Error: %v\n", err)
			return 1
		}

		if info.IsDir() {
			detected, err := AnalyzeDirectory(path, *excludeTests)
			if err != nil {
				fmt.Fprintf(out, "Error: %v\n", err)
				return 1
			}
			allDetected = append(allDetected, detected...)
		} else {
			detected, err := AnalyzeFile(path)
			if err != nil {
				fmt.Fprintf(out, "Error: %v\n", err)
				return 1
			}
			allDetected = append(allDetected, detected...)
		}
	}

	// Compare
	result := CompareCapabilities(allDetected, manifest)

	if *jsonOutput {
		enc := json.NewEncoder(out)
		enc.SetIndent("", "  ")
		if err := enc.Encode(result); err != nil {
			fmt.Fprintf(out, "Error encoding JSON: %v\n", err)
			return 1
		}
	} else {
		fmt.Fprint(out, result.Summary())
	}

	if result.Passed {
		return 0
	}
	return 1
}

// runBanned implements the "banned" subcommand.
//
// It scans source files for banned constructs (reflect.Value.Call,
// plugin.Open, //go:linkname, unsafe.Pointer, import "C").
//
// Usage:
//
//	ca-capability-analyzer banned [--json] <path> [path...]
func runBanned(args []string, out io.Writer) int {
	fs := flag.NewFlagSet("banned", flag.ContinueOnError)
	fs.SetOutput(out)
	jsonOutput := fs.Bool("json", false, "Output in JSON format")

	if err := fs.Parse(args); err != nil {
		return 2
	}

	paths := fs.Args()
	if len(paths) == 0 {
		fmt.Fprintln(out, "Usage: ca-capability-analyzer banned [--json] <path> [path...]")
		return 2
	}

	var allViolations []BannedConstructViolation
	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			fmt.Fprintf(out, "Error: %v\n", err)
			return 1
		}

		if info.IsDir() {
			// Walk directory for .go files
			violations, err := detectBannedDirectory(path)
			if err != nil {
				fmt.Fprintf(out, "Error: %v\n", err)
				return 1
			}
			allViolations = append(allViolations, violations...)
		} else {
			violations, err := DetectBannedFile(path)
			if err != nil {
				fmt.Fprintf(out, "Error: %v\n", err)
				return 1
			}
			allViolations = append(allViolations, violations...)
		}
	}

	if *jsonOutput {
		enc := json.NewEncoder(out)
		enc.SetIndent("", "  ")
		if err := enc.Encode(allViolations); err != nil {
			fmt.Fprintf(out, "Error encoding JSON: %v\n", err)
			return 1
		}
	} else {
		if len(allViolations) == 0 {
			fmt.Fprintln(out, "No banned constructs detected.")
		} else {
			fmt.Fprintf(out, "Found %d banned construct(s):\n", len(allViolations))
			for _, v := range allViolations {
				fmt.Fprintf(out, "  %s:%d: BANNED %s (%s)\n", v.File, v.Line, v.Construct, v.Evidence)
			}
		}
	}

	if len(allViolations) > 0 {
		return 1
	}
	return 0
}

// detectBannedDirectory walks a directory and scans all .go files for
// banned constructs.
func detectBannedDirectory(dir string) ([]BannedConstructViolation, error) {
	var allViolations []BannedConstructViolation

	err := walkGoFiles(dir, false, func(path string) error {
		violations, err := DetectBannedFile(path)
		if err != nil {
			return nil // skip unparseable files
		}
		allViolations = append(allViolations, violations...)
		return nil
	})

	return allViolations, err
}

// walkGoFiles walks a directory tree, calling fn for each .go file.
// If excludeTests is true, *_test.go files are skipped.
func walkGoFiles(dir string, excludeTests bool, fn func(string) error) error {
	skipDirs := map[string]bool{
		"vendor":      true,
		".git":        true,
		"node_modules": true,
		"testdata":    true,
	}

	return filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			if skipDirs[info.Name()] {
				return filepath.SkipDir
			}
			return nil
		}
		if filepath.Ext(path) != ".go" {
			return nil
		}
		if excludeTests && isTestFile(path) {
			return nil
		}
		return fn(path)
	})
}

// isTestFile returns true if the path ends with _test.go.
func isTestFile(path string) bool {
	base := filepath.Base(path)
	return len(base) > 8 && base[len(base)-8:] == "_test.go"
}
