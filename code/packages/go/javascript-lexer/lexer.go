package javascriptlexer

import (
	"fmt"
	"path/filepath"
	"runtime"

	"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// validVersions is the set of ECMAScript / JavaScript version strings the
// lexer recognises. Each maps to a versioned grammar file under
// code/grammars/ecmascript/.
//
// JavaScript has always been the implementation of the ECMAScript standard.
// The version names here follow the ECMAScript edition naming convention that
// became conventional after ES2015:
//
//   es1    — ECMAScript 1   (June 1997)    the first standard
//   es3    — ECMAScript 3   (December 1999) first widely-implemented standard
//   es5    — ECMAScript 5   (December 2009) strict mode, JSON support
//   es2015 — ECMAScript 6   (June 2015)    classes, modules, arrow functions
//   es2016 — ECMAScript 7   (June 2016)    exponentiation operator
//   es2017 — ECMAScript 8   (June 2017)    async/await
//   es2018 — ECMAScript 9   (June 2018)    rest/spread for objects
//   es2019 — ECMAScript 10  (June 2019)    Array.flat, optional catch binding
//   es2020 — ECMAScript 11  (June 2020)    BigInt, optional chaining
//   es2021 — ECMAScript 12  (June 2021)    logical assignment, numeric separators
//   es2022 — ECMAScript 13  (June 2022)    class fields, at(), Object.hasOwn
//   es2023 — ECMAScript 14  (June 2023)    Array.findLast, Symbols as WeakMap keys
//   es2024 — ECMAScript 15  (June 2024)    Promise.withResolvers, Object.groupBy
//   es2025 — ECMAScript 16  (June 2025)    latest stable
var validVersions = map[string]bool{
	"es1":    true,
	"es3":    true,
	"es5":    true,
	"es2015": true,
	"es2016": true,
	"es2017": true,
	"es2018": true,
	"es2019": true,
	"es2020": true,
	"es2021": true,
	"es2022": true,
	"es2023": true,
	"es2024": true,
	"es2025": true,
}

// getGrammarPath resolves the absolute path to the .tokens grammar file for
// the given JavaScript / ECMAScript version string.
//
// When version is "" (empty string) the generic grammar at
// code/grammars/javascript.tokens is used — this preserves backward-compatible
// behaviour for callers that do not care about a specific version.
//
// Any other non-empty string must appear in validVersions; an unknown version
// returns a descriptive error so that typos produce actionable messages.
func getGrammarPath(version string) (string, error) {
	_, filename, _, _ := runtime.Caller(0)
	parent := filepath.Dir(filename)
	root := filepath.Join(parent, "..", "..", "..", "grammars")
	if version == "" {
		return filepath.Join(root, "javascript.tokens"), nil
	}
	if !validVersions[version] {
		return "", fmt.Errorf(
			"unknown JavaScript version %q: valid versions are es1, es3, es5, es2015–es2025",
			version,
		)
	}
	return filepath.Join(root, "ecmascript", version+".tokens"), nil
}

// CreateJavascriptLexer constructs a GrammarLexer ready to tokenise the given
// JavaScript source string.
//
// version selects the ECMAScript grammar file:
//   - ""      — generic grammar (javascript.tokens); same as pre-0.2.0 behaviour
//   - "es1", "es3", "es5" — classic ECMAScript editions
//   - "es2015" through "es2025" — modern ECMAScript yearly editions
//
// An error is returned if the version string is unrecognised or if the grammar
// file cannot be read.
func CreateJavascriptLexer(source string, version string) (*lexer.GrammarLexer, error) {
	grammarPath, err := getGrammarPath(version)
	if err != nil {
		return nil, err
	}
	return StartNew[*lexer.GrammarLexer]("javascriptlexer.CreateJavascriptLexer", nil,
		func(op *Operation[*lexer.GrammarLexer], rf *ResultFactory[*lexer.GrammarLexer]) *OperationResult[*lexer.GrammarLexer] {
			bytes, err := op.File.ReadFile(grammarPath)
			if err != nil {
				return rf.Fail(nil, err)
			}
			grammar, err := grammartools.ParseTokenGrammar(string(bytes))
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, lexer.NewGrammarLexer(source, grammar))
		}).GetResult()
}

// TokenizeJavascript is the main entry point for lexing JavaScript source code.
//
// It tokenises source using the grammar for the given ECMAScript version and
// returns the flat token slice produced by the underlying GrammarLexer.
// Pass version="" to use the generic grammar, which covers the superset of all
// supported versions and is the best choice when version is unknown.
//
// Example — tokenise with the generic grammar:
//
//	tokens, err := TokenizeJavascript("let x = 1;", "")
//
// Example — tokenise with a specific version:
//
//	tokens, err := TokenizeJavascript("const x = 1;", "es2022")
func TokenizeJavascript(source string, version string) ([]lexer.Token, error) {
	javascriptLexer, err := CreateJavascriptLexer(source, version)
	if err != nil {
		return nil, err
	}
	return javascriptLexer.Tokenize(), nil
}
