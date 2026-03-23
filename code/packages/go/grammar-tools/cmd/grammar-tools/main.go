// main.go — CLI entry point for grammar-tools validation.
//
// Usage:
//
//	grammar-tools validate <file.tokens> <file.grammar>
//	grammar-tools validate-tokens <file.tokens>
//	grammar-tools validate-grammar <file.grammar>
//
// This tool parses and validates .tokens and .grammar files, reporting errors
// in a human-readable format. It is the Go equivalent of running
// `python -m grammar_tools validate ...`.
//
// Why a dedicated CLI?
// --------------------
//
// The library functions (ValidateTokenGrammar, ValidateParserGrammar,
// CrossValidate) exist for programmatic use, but when editing grammar files
// you want a fast command-line check — like a compiler's -fsyntax-only flag.
//
// Exit codes:
//
//	0 — all checks passed
//	1 — one or more errors found
//	2 — usage error (wrong number of arguments, unknown command)
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

func main() {
	os.Exit(run(os.Args[1:]))
}

// run is the testable entry point. It takes the argument slice (not including
// argv[0]) and returns an exit code.
func run(args []string) int {
	if len(args) == 0 || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
		printUsage()
		return 0
	}

	command := args[0]

	switch command {
	case "validate":
		if len(args) != 3 {
			fmt.Fprintln(os.Stderr, "Error: 'validate' requires two arguments: <tokens> <grammar>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return validateCommand(args[1], args[2])

	case "validate-tokens":
		if len(args) != 2 {
			fmt.Fprintln(os.Stderr, "Error: 'validate-tokens' requires one argument: <tokens>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return validateTokensOnly(args[1])

	case "validate-grammar":
		if len(args) != 2 {
			fmt.Fprintln(os.Stderr, "Error: 'validate-grammar' requires one argument: <grammar>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return validateGrammarOnly(args[1])

	default:
		fmt.Fprintf(os.Stderr, "Error: Unknown command '%s'\n", command)
		fmt.Fprintln(os.Stderr)
		printUsage()
		return 2
	}
}

// printUsage prints the help text to stderr.
func printUsage() {
	fmt.Fprintln(os.Stderr, "Usage: grammar-tools <command> [args...]")
	fmt.Fprintln(os.Stderr)
	fmt.Fprintln(os.Stderr, "Commands:")
	fmt.Fprintln(os.Stderr, "  validate <file.tokens> <file.grammar>   Validate a token/grammar pair")
	fmt.Fprintln(os.Stderr, "  validate-tokens <file.tokens>            Validate just a .tokens file")
	fmt.Fprintln(os.Stderr, "  validate-grammar <file.grammar>          Validate just a .grammar file")
	fmt.Fprintln(os.Stderr)
	fmt.Fprintln(os.Stderr, "Examples:")
	fmt.Fprintln(os.Stderr, "  grammar-tools validate css.tokens css.grammar")
	fmt.Fprintln(os.Stderr, "  grammar-tools validate-tokens css.tokens")
	fmt.Fprintln(os.Stderr, "  grammar-tools validate-grammar css.grammar")
}

// countErrors counts how many issues are actual errors (not warnings).
// Issues starting with "Warning:" are informational. Everything else is an error.
func countErrors(issues []string) int {
	count := 0
	for _, issue := range issues {
		if !strings.HasPrefix(issue, "Warning:") {
			count++
		}
	}
	return count
}

// printIssues prints a list of issues with two-space indentation.
func printIssues(issues []string) {
	for _, issue := range issues {
		fmt.Println(" ", issue)
	}
}

// readFile reads a file and returns its content, printing an error and
// returning "" on failure.
func readFile(path string) (string, bool) {
	data, err := os.ReadFile(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: Cannot read file: %s\n", path)
		return "", false
	}
	return string(data), true
}

// validateCommand validates a .tokens and .grammar file pair.
//
// Output format (success):
//
//	Validating lattice.tokens ... OK (N tokens, M skip, K error)
//	Validating lattice.grammar ... OK (P rules)
//	Cross-validating ... OK
//	All checks passed.
//
// Output format (failure):
//
//	Validating broken.tokens ... 2 error(s)
//	  Line 5: Duplicate token name 'IDENT' ...
//	Found 4 error(s). Fix them and try again.
func validateCommand(tokensPath, grammarPath string) int {
	totalErrors := 0

	// --- Parse and validate the .tokens file ---
	tokensSource, ok := readFile(tokensPath)
	if !ok {
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(tokensPath))
	tokenGrammar, err := grammartools.ParseTokenGrammar(tokensSource)
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Println(" ", err)
		return 1
	}

	tokenIssues := grammartools.ValidateTokenGrammar(tokenGrammar)
	nTokens := len(tokenGrammar.Definitions)
	nSkip := len(tokenGrammar.SkipDefinitions)
	nError := len(tokenGrammar.ErrorDefinitions)
	tokenErrors := countErrors(tokenIssues)
	if tokenErrors > 0 {
		fmt.Printf("%d error(s)\n", tokenErrors)
		printIssues(tokenIssues)
		totalErrors += tokenErrors
	} else {
		// Build the OK message: "OK (N tokens)" or "OK (N tokens, M skip)"
		// or "OK (N tokens, M skip, K error)"
		parts := []string{fmt.Sprintf("%d tokens", nTokens)}
		if nSkip > 0 {
			parts = append(parts, fmt.Sprintf("%d skip", nSkip))
		}
		if nError > 0 {
			parts = append(parts, fmt.Sprintf("%d error", nError))
		}
		fmt.Printf("OK (%s)\n", strings.Join(parts, ", "))
	}

	// --- Parse and validate the .grammar file ---
	grammarSource, ok := readFile(grammarPath)
	if !ok {
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(grammarPath))
	parserGrammar, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Println(" ", err)
		return 1
	}

	// Pass token names so undefined token references are caught.
	parserIssues := grammartools.ValidateParserGrammar(parserGrammar, tokenGrammar.TokenNames())
	nRules := len(parserGrammar.Rules)
	parserErrors := countErrors(parserIssues)
	if parserErrors > 0 {
		fmt.Printf("%d error(s)\n", parserErrors)
		printIssues(parserIssues)
		totalErrors += parserErrors
	} else {
		fmt.Printf("OK (%d rules)\n", nRules)
	}

	// --- Cross-validate ---
	fmt.Print("Cross-validating ... ")
	crossIssues := grammartools.CrossValidate(tokenGrammar, parserGrammar)
	crossErrors := countErrors(crossIssues)
	crossWarnings := len(crossIssues) - crossErrors
	if crossErrors > 0 {
		fmt.Printf("%d error(s)\n", crossErrors)
		printIssues(crossIssues)
		totalErrors += crossErrors
	} else if crossWarnings > 0 {
		fmt.Printf("OK (%d warning(s))\n", crossWarnings)
		printIssues(crossIssues)
	} else {
		fmt.Println("OK")
	}

	// --- Summary ---
	if totalErrors > 0 {
		fmt.Printf("\nFound %d error(s). Fix them and try again.\n", totalErrors)
		return 1
	}
	fmt.Println("\nAll checks passed.")
	return 0
}

// validateTokensOnly validates just a .tokens file.
func validateTokensOnly(tokensPath string) int {
	source, ok := readFile(tokensPath)
	if !ok {
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(tokensPath))
	tokenGrammar, err := grammartools.ParseTokenGrammar(source)
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Println(" ", err)
		return 1
	}

	issues := grammartools.ValidateTokenGrammar(tokenGrammar)
	nTokens := len(tokenGrammar.Definitions)
	errors := countErrors(issues)
	if errors > 0 {
		fmt.Printf("%d error(s)\n", errors)
		printIssues(issues)
		fmt.Printf("\nFound %d error(s). Fix them and try again.\n", errors)
		return 1
	}
	fmt.Printf("OK (%d tokens)\n", nTokens)
	fmt.Println("\nAll checks passed.")
	return 0
}

// validateGrammarOnly validates just a .grammar file.
func validateGrammarOnly(grammarPath string) int {
	source, ok := readFile(grammarPath)
	if !ok {
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(grammarPath))
	parserGrammar, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Println(" ", err)
		return 1
	}

	// Without a tokens file, we can only check rule-level issues.
	issues := grammartools.ValidateParserGrammar(parserGrammar, nil)
	nRules := len(parserGrammar.Rules)
	errors := countErrors(issues)
	if errors > 0 {
		fmt.Printf("%d error(s)\n", errors)
		printIssues(issues)
		fmt.Printf("\nFound %d error(s). Fix them and try again.\n", errors)
		return 1
	}
	fmt.Printf("OK (%d rules)\n", nRules)
	fmt.Println("\nAll checks passed.")
	return 0
}
