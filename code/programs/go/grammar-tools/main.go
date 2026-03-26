package main

import (
	_ "embed"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"unicode"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func main() {
	os.Exit(run(os.Args))
}

func run(args []string) int {
	monorepoRoot, err := findMonorepoRoot()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error finding monorepo root: %v\n", err)
		return 1
	}
	specPath := filepath.Join(monorepoRoot, "code", "specs", "grammar-tools.cli.json")
	parser, err := clibuilder.NewParser(specPath, args)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Spec Error: %v\n", err)
		return 1
	}

	resultAny, err := parser.Parse()
	if err != nil {
		if parseErrs, ok := err.(*clibuilder.ParseErrors); ok {
			for _, e := range parseErrs.Errors {
				fmt.Fprintf(os.Stderr, "Error: %s\n", e.Message)
				if e.Suggestion != "" {
					fmt.Fprintf(os.Stderr, "  %s\n", e.Suggestion)
				}
			}
		} else {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		}
		return 2
	}

	switch res := resultAny.(type) {
	case *clibuilder.HelpResult:
		fmt.Print(res.Text)
		return 0
	case *clibuilder.VersionResult:
		fmt.Println(res.Version)
		return 0
	case *clibuilder.ParseResult:
		cmd := res.CommandPath[len(res.CommandPath)-1]

		switch cmd {
		case "validate":
			return validateCommand(res.Arguments["tokens_file"].(string), res.Arguments["grammar_file"].(string))
		case "validate-tokens":
			return validateTokensOnly(res.Arguments["tokens_file"].(string))
		case "validate-grammar":
			return validateGrammarOnly(res.Arguments["grammar_file"].(string))
		case "compile-tokens":
			return compileTokensOnly(res.Arguments["tokens_file"].(string), res.Arguments["export_name"].(string))
		case "compile-grammar":
			return compileGrammarOnly(res.Arguments["grammar_file"].(string), res.Arguments["export_name"].(string))
		case "generate":
			return generateCommand()
		default:
			fmt.Fprintf(os.Stderr, "Error: Unknown command '%s'\n", cmd)
			return 2
		}
	}
	return 1
}

func compileTokensOnly(tokensPath, exportName string) int {
	source, ok := readFile(tokensPath)
	if !ok {
		return 1
	}
	tg, err := grammartools.ParseTokenGrammar(source)
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Println(" ", err)
		return 1
	}
	issues := grammartools.ValidateTokenGrammar(tg)
	if countErrors(issues) > 0 {
		fmt.Println("Error: Cannot compile invalid grammar file.")
		printIssues(issues)
		return 1
	}
	
	gn := strings.TrimSuffix(filepath.Base(tokensPath), ".tokens")
	pkgName := strings.ReplaceAll(gn, "-", "") + "lexer"
	if gn == "xml_rust" {
		pkgName = "xmllexer"
	}

	fmt.Print(grammartools.CompileTokensToGo(tg, pkgName, exportName))
	return 0
}

func compileGrammarOnly(grammarPath, exportName string) int {
	source, ok := readFile(grammarPath)
	if !ok {
		return 1
	}
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		fmt.Println("PARSE ERROR")
		fmt.Println(" ", err)
		return 1
	}
	issues := grammartools.ValidateParserGrammar(pg, nil)
	if countErrors(issues) > 0 {
		fmt.Println("Error: Cannot compile invalid grammar file.")
		printIssues(issues)
		return 1
	}
	
	gn := strings.TrimSuffix(filepath.Base(grammarPath), ".grammar")
	pkgName := strings.ReplaceAll(gn, "-", "") + "parser"

	fmt.Print(grammartools.CompileParserToGo(pg, pkgName, exportName))
	return 0
}

func toCamelCase(snakeStr string) string {
	components := strings.Split(strings.ReplaceAll(snakeStr, "-", "_"), "_")
	var result string
	for _, c := range components {
		if len(c) > 0 {
			result += string(unicode.ToUpper(rune(c[0]))) + c[1:]
		}
	}
	return result
}

func findMonorepoRoot() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}
	for {
		if info, err := os.Stat(filepath.Join(dir, "code", "grammars")); err == nil && info.IsDir() {
			return dir, nil
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	return "", fmt.Errorf("could not find monorepo root")
}

func generateCommand() int {
	hasErrors := false
	monorepoRoot, err := findMonorepoRoot()
	if err != nil {
		fmt.Fprintln(os.Stderr, "Error:", err)
		return 1
	}
	grammarsDir := filepath.Join(monorepoRoot, "code", "grammars")
	langDir := filepath.Join(monorepoRoot, "code", "packages", "go")

	entries, err := os.ReadDir(grammarsDir)
	if err != nil {
		fmt.Fprintln(os.Stderr, "Error reading grammars dir:", err)
		return 1
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		name := entry.Name()
		ext := filepath.Ext(name)
		if ext != ".tokens" && ext != ".grammar" {
			continue
		}
		isTokens := ext == ".tokens"
		kind := "parser"
		if isTokens {
			kind = "lexer"
		}
		gn := strings.TrimSuffix(name, ext)
		
		targetDir := ""
		possibleDirs := []string{
			filepath.Join(langDir, fmt.Sprintf("%s-%s", gn, kind)),
			filepath.Join(langDir, fmt.Sprintf("%s_%s", gn, kind)),
		}
		for _, pd := range possibleDirs {
			if info, err := os.Stat(pd); err == nil && info.IsDir() {
				targetDir = pd
				break
			}
		}
		if targetDir == "" {
			continue
		}

		fmt.Printf("Generating for %s ...\n", name)
		
		pkgName := strings.ReplaceAll(gn, "-", "") + kind
		if gn == "xml_rust" {
			pkgName = "xmllexer"
		}
		varSuffix := "Grammar"
		if isTokens {
			varSuffix = "Tokens"
		}
		exportName := toCamelCase(gn) + varSuffix
		fnameBase := fmt.Sprintf("%s_%s.go", gn, "grammar")
		if isTokens {
			fnameBase = fmt.Sprintf("%s_%s.go", gn, "tokens")
		}
		outPath := filepath.Join(targetDir, fnameBase)

		source, ok := readFile(filepath.Join(grammarsDir, name))
		if !ok {
			hasErrors = true
			continue
		}

		var code string
		if isTokens {
			tg, err := grammartools.ParseTokenGrammar(source)
			if err != nil {
				fmt.Printf("Error: parse failed for %s: %v\n", name, err)
				hasErrors = true
				continue
			}
			issues := grammartools.ValidateTokenGrammar(tg)
			if countErrors(issues) > 0 {
				fmt.Printf("Error: Cannot compile invalid grammar file %s\n", name)
				printIssues(issues)
				hasErrors = true
				continue
			}
			code = grammartools.CompileTokensToGo(tg, pkgName, exportName)
		} else {
			pg, err := grammartools.ParseParserGrammar(source)
			if err != nil {
				fmt.Printf("Error: parse failed for %s: %v\n", name, err)
				hasErrors = true
				continue
			}
			issues := grammartools.ValidateParserGrammar(pg, nil)
			if countErrors(issues) > 0 {
				fmt.Printf("Error: Cannot compile invalid grammar file %s\n", name)
				printIssues(issues)
				hasErrors = true
				continue
			}
			code = grammartools.CompileParserToGo(pg, pkgName, exportName)
		}

		err = os.WriteFile(outPath, []byte(code), 0644)
		if err != nil {
			fmt.Printf("Error writing %s: %v\n", outPath, err)
			hasErrors = true
		} else {
			fmt.Printf("  -> Saved %s\n", outPath)
		}
	}

	if hasErrors {
		return 1
	}
	return 0
}

// countErrors counts how many issues are actual errors (not warnings).
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
func validateCommand(tokensPath, grammarPath string) int {
	totalErrors := 0

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
		parts := []string{fmt.Sprintf("%d tokens", nTokens)}
		if nSkip > 0 {
			parts = append(parts, fmt.Sprintf("%d skip", nSkip))
		}
		if nError > 0 {
			parts = append(parts, fmt.Sprintf("%d error", nError))
		}
		fmt.Printf("OK (%s)\n", strings.Join(parts, ", "))
	}

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
