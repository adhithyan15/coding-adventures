// main.go — grammar-tools CLI program.
//
// This program wraps the grammar-tools library behind a cli-builder-powered
// interface. It validates .tokens and .grammar files and reports errors in a
// human-readable format. It also compiles grammar files into Go source code.
//
// Usage:
//
//	grammar-tools validate <file.tokens> <file.grammar>
//	grammar-tools validate-tokens <file.tokens>
//	grammar-tools validate-grammar <file.grammar>
//	grammar-tools compile-tokens <file.tokens> [-o <output.go>]
//	grammar-tools compile-grammar <file.grammar> [-o <output.go>]
//	grammar-tools --help
//
// Exit codes:
//
//	0  All checks passed / compilation succeeded.
//	1  One or more validation errors found / compile error.
//	2  Usage error (wrong number of arguments, unknown command).
//
// Why a program instead of a cmd/ in the library?
// ------------------------------------------------
//
// Moving the CLI to code/programs/ cleanly separates the library (parse and
// validate grammar files) from the executable (run a user-facing CLI tool).
// The library stays in code/packages/go/grammar-tools/ and is unchanged.
// Other packages that import the library are not affected.
//
// Using cli-builder ensures --help, --version, and error formatting are
// consistent with cowsay, unix-tools, and every other program in this repo.
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// defaultPkgName is the Go package name used in generated files when --package
// is not specified.  Callers that write to a real package directory should
// always pass --package to get a correct package declaration.
const defaultPkgName = "generated"

// ============================================================================
// Helpers
// ============================================================================

// countErrors returns the number of issues that are actual errors (not
// informational warnings). Issues starting with "Warning" are informational
// and do not cause non-zero exit.
func countErrors(issues []string) int {
	n := 0
	for _, i := range issues {
		if !strings.HasPrefix(i, "Warning") {
			n++
		}
	}
	return n
}

// printIssues writes a list of issues to stdout with two-space indentation.
func printIssues(issues []string) {
	for _, i := range issues {
		fmt.Printf("  %s\n", i)
	}
}

func printUsage() {
	fmt.Fprintln(os.Stderr, "Usage: grammar-tools <command> [args...]")
	fmt.Fprintln(os.Stderr)
	fmt.Fprintln(os.Stderr, "Commands:")
	fmt.Fprintln(os.Stderr, "  validate <file.tokens> <file.grammar>       Validate a token/grammar pair")
	fmt.Fprintln(os.Stderr, "  validate-tokens <file.tokens>                Validate just a .tokens file")
	fmt.Fprintln(os.Stderr, "  validate-grammar <file.grammar>              Validate just a .grammar file")
	fmt.Fprintln(os.Stderr, "  compile-tokens <file.tokens> [-o out.go] [-p pkg]    Compile tokens to Go")
	fmt.Fprintln(os.Stderr, "  compile-grammar <file.grammar> [-o out.go] [-p pkg]  Compile grammar to Go")
	fmt.Fprintln(os.Stderr)
	fmt.Fprintln(os.Stderr, "Run 'grammar-tools --help' for full help text.")
}

// findRoot walks up the directory tree from the current working directory
// until it finds code/specs/grammar-tools.json, which identifies the repo root.
func findRoot() string {
	curr, _ := os.Getwd()
	for i := 0; i < 20; i++ {
		if _, err := os.Stat(filepath.Join(curr, "code", "specs", "grammar-tools.json")); err == nil {
			return curr
		}
		parent := filepath.Dir(curr)
		if parent == curr {
			break
		}
		curr = parent
	}
	wd, _ := os.Getwd()
	return wd
}

// ============================================================================
// validate — cross-validate a .tokens/.grammar pair
// ============================================================================

// validateCommand validates a .tokens file and a .grammar file together.
//
// Three checks in sequence:
//  1. Parse and validate the .tokens file (syntax, duplicates, bad regexes).
//  2. Parse and validate the .grammar file (undefined refs, duplicates).
//  3. Cross-validate the pair (missing/extra token definitions).
//
// Returns 0 on success, 1 if any errors are found.
func validateCommand(tokensPath, grammarPath string) int {
	totalIssues := 0

	// Step 1: parse and validate .tokens
	tokensContent, err := os.ReadFile(tokensPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: File not found: %s\n", tokensPath)
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(tokensPath))
	tokenGrammar, err := grammartools.ParseTokenGrammar(string(tokensContent))
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Printf("  %s\n", err)
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
		totalIssues += tokenErrors
	} else {
		parts := []string{fmt.Sprintf("%d tokens", nTokens)}
		if nSkip > 0 {
			parts = append(parts, fmt.Sprintf("%d skip", nSkip))
		}
		if nError > 0 {
			parts = append(parts, fmt.Sprintf("%d error", nError))
		}
		fmt.Printf("OK (%s)\n", strings.Join(parts, ", "))
	}

	// Step 2: parse and validate .grammar
	grammarContent, err := os.ReadFile(grammarPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: File not found: %s\n", grammarPath)
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(grammarPath))
	parserGrammar, err := grammartools.ParseParserGrammar(string(grammarContent))
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Printf("  %s\n", err)
		return 1
	}

	parserIssues := grammartools.ValidateParserGrammar(parserGrammar, tokenGrammar.TokenNames())
	nRules := len(parserGrammar.Rules)
	parserErrors := countErrors(parserIssues)

	if parserErrors > 0 {
		fmt.Printf("%d error(s)\n", parserErrors)
		printIssues(parserIssues)
		totalIssues += parserErrors
	} else {
		fmt.Printf("OK (%d rules)\n", nRules)
	}

	// Step 3: cross-validate
	fmt.Print("Cross-validating ... ")
	crossIssues := grammartools.CrossValidate(tokenGrammar, parserGrammar)
	crossErrors := countErrors(crossIssues)
	crossWarnings := len(crossIssues) - crossErrors

	switch {
	case crossErrors > 0:
		fmt.Printf("%d error(s)\n", crossErrors)
		printIssues(crossIssues)
		totalIssues += crossErrors
	case crossWarnings > 0:
		fmt.Printf("OK (%d warning(s))\n", crossWarnings)
		printIssues(crossIssues)
	default:
		fmt.Println("OK")
	}

	fmt.Println()
	if totalIssues > 0 {
		fmt.Printf("Found %d error(s). Fix them and try again.\n", totalIssues)
		return 1
	}
	fmt.Println("All checks passed.")
	return 0
}

// ============================================================================
// validate-tokens — validate just a .tokens file
// ============================================================================

func validateTokensOnly(tokensPath string) int {
	content, err := os.ReadFile(tokensPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: File not found: %s\n", tokensPath)
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(tokensPath))
	tokenGrammar, err := grammartools.ParseTokenGrammar(string(content))
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Printf("  %s\n", err)
		return 1
	}

	issues := grammartools.ValidateTokenGrammar(tokenGrammar)
	nTokens := len(tokenGrammar.Definitions)
	nSkip := len(tokenGrammar.SkipDefinitions)
	nError := len(tokenGrammar.ErrorDefinitions)
	errors := countErrors(issues)

	if errors > 0 {
		fmt.Printf("%d error(s)\n", errors)
		printIssues(issues)
		fmt.Println()
		fmt.Printf("Found %d error(s). Fix them and try again.\n", errors)
		return 1
	}

	parts := []string{fmt.Sprintf("%d tokens", nTokens)}
	if nSkip > 0 {
		parts = append(parts, fmt.Sprintf("%d skip", nSkip))
	}
	if nError > 0 {
		parts = append(parts, fmt.Sprintf("%d error", nError))
	}
	fmt.Printf("OK (%s)\n", strings.Join(parts, ", "))
	fmt.Println()
	fmt.Println("All checks passed.")
	return 0
}

// ============================================================================
// validate-grammar — validate just a .grammar file
// ============================================================================

func validateGrammarOnly(grammarPath string) int {
	content, err := os.ReadFile(grammarPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: File not found: %s\n", grammarPath)
		return 1
	}

	fmt.Printf("Validating %s ... ", filepath.Base(grammarPath))
	parserGrammar, err := grammartools.ParseParserGrammar(string(content))
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Printf("  %s\n", err)
		return 1
	}

	// Without token names, only rule-level checks run.
	issues := grammartools.ValidateParserGrammar(parserGrammar, nil)
	nRules := len(parserGrammar.Rules)
	errors := countErrors(issues)

	if errors > 0 {
		fmt.Printf("%d error(s)\n", errors)
		printIssues(issues)
		fmt.Println()
		fmt.Printf("Found %d error(s). Fix them and try again.\n", errors)
		return 1
	}

	fmt.Printf("OK (%d rules)\n", nRules)
	fmt.Println()
	fmt.Println("All checks passed.")
	return 0
}

// ============================================================================
// compile-tokens — compile a .tokens file to Go source code
// ============================================================================

// compileTokensCommand parses and compiles a .tokens file into Go source code.
//
// The generated Go file embeds the TokenGrammar as native data structures.
// If outputPath is non-empty the code is written there; otherwise it is
// printed to stdout.
//
// Returns 0 on success, 1 on error.
func compileTokensCommand(tokensPath, outputPath, pkgName string, force bool) int {
	content, err := os.ReadFile(tokensPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: File not found: %s\n", tokensPath)
		return 1
	}

	fmt.Fprintf(os.Stderr, "Compiling %s ... ", filepath.Base(tokensPath))
	tokenGrammar, err := grammartools.ParseTokenGrammar(string(content))
	if err != nil {
		fmt.Fprintln(os.Stderr, "PARSE ERROR")
		fmt.Fprintf(os.Stderr, "  %s\n", err)
		return 1
	}

	if !force {
		issues := grammartools.ValidateTokenGrammar(tokenGrammar)
		if n := countErrors(issues); n > 0 {
			fmt.Fprintf(os.Stderr, "%d error(s)\n", n)
			printIssues(issues)
			return 1
		}
	}

	code := grammartools.CompileTokenGrammar(tokenGrammar, filepath.Base(tokensPath), pkgName)

	if outputPath != "" {
		if err := os.WriteFile(outputPath, []byte(code), 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "Error writing %s: %v\n", outputPath, err)
			return 1
		}
		fmt.Fprintf(os.Stderr, "OK → %s\n", outputPath)
	} else {
		fmt.Fprintln(os.Stderr, "OK")
		fmt.Print(code)
	}
	return 0
}

// ============================================================================
// compile-grammar — compile a .grammar file to Go source code
// ============================================================================

// compileGrammarCommand parses and compiles a .grammar file into Go source code.
func compileGrammarCommand(grammarPath, outputPath, pkgName string, force bool) int {
	content, err := os.ReadFile(grammarPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: File not found: %s\n", grammarPath)
		return 1
	}

	fmt.Fprintf(os.Stderr, "Compiling %s ... ", filepath.Base(grammarPath))
	parserGrammar, err := grammartools.ParseParserGrammar(string(content))
	if err != nil {
		fmt.Fprintln(os.Stderr, "PARSE ERROR")
		fmt.Fprintf(os.Stderr, "  %s\n", err)
		return 1
	}

	if !force {
		issues := grammartools.ValidateParserGrammar(parserGrammar, nil)
		if n := countErrors(issues); n > 0 {
			fmt.Fprintf(os.Stderr, "%d error(s)\n", n)
			printIssues(issues)
			return 1
		}
	}

	code := grammartools.CompileParserGrammar(parserGrammar, filepath.Base(grammarPath), pkgName)

	if outputPath != "" {
		if err := os.WriteFile(outputPath, []byte(code), 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "Error writing %s: %v\n", outputPath, err)
			return 1
		}
		fmt.Fprintf(os.Stderr, "OK → %s\n", outputPath)
	} else {
		fmt.Fprintln(os.Stderr, "OK")
		fmt.Print(code)
	}
	return 0
}

// ============================================================================
// dispatch
// ============================================================================

func dispatch(command string, files []string, outputPath, pkgName string, force bool) int {
	switch command {
	case "validate":
		if len(files) != 2 {
			fmt.Fprintln(os.Stderr, "Error: 'validate' requires two arguments: <tokens> <grammar>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return validateCommand(files[0], files[1])

	case "validate-tokens":
		if len(files) != 1 {
			fmt.Fprintln(os.Stderr, "Error: 'validate-tokens' requires one argument: <tokens>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return validateTokensOnly(files[0])

	case "validate-grammar":
		if len(files) != 1 {
			fmt.Fprintln(os.Stderr, "Error: 'validate-grammar' requires one argument: <grammar>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return validateGrammarOnly(files[0])

	case "compile-tokens":
		if len(files) != 1 {
			fmt.Fprintln(os.Stderr, "Error: 'compile-tokens' requires one argument: <tokens>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return compileTokensCommand(files[0], outputPath, pkgName, force)

	case "compile-grammar":
		if len(files) != 1 {
			fmt.Fprintln(os.Stderr, "Error: 'compile-grammar' requires one argument: <grammar>")
			fmt.Fprintln(os.Stderr)
			printUsage()
			return 2
		}
		return compileGrammarCommand(files[0], outputPath, pkgName, force)

	default:
		fmt.Fprintf(os.Stderr, "Error: Unknown command '%s'\n", command)
		fmt.Fprintln(os.Stderr)
		printUsage()
		return 2
	}
}

// ============================================================================
// main
// ============================================================================

func main() {
	root := findRoot()
	specPath := filepath.Join(root, "code", "specs", "grammar-tools.json")

	parser, err := clibuilder.NewParser(specPath, os.Args)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(2)
	}

	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(2)
	}

	switch r := result.(type) {
	case *clibuilder.HelpResult:
		fmt.Print(r.Text)
		os.Exit(0)

	case *clibuilder.VersionResult:
		fmt.Println(r.Version)
		os.Exit(0)

	case *clibuilder.ParseResult:
		command, _ := r.Arguments["command"].(string)

		var files []string
		if raw, ok := r.Arguments["files"]; ok && raw != nil {
			switch v := raw.(type) {
			case []any:
				for _, item := range v {
					if s, ok := item.(string); ok {
						files = append(files, s)
					}
				}
			case string:
				files = []string{v}
			}
		}

		outputPath, _ := r.Flags["output"].(string)
		pkgName, _ := r.Flags["package"].(string)
		if pkgName == "" {
			pkgName = defaultPkgName
		}
		force, _ := r.Flags["force"].(bool)

		os.Exit(dispatch(command, files, outputPath, pkgName, force))
	}
}
