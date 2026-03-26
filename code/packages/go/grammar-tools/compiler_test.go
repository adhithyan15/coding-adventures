package grammartools

// compiler_test.go — Tests for CompileTokenGrammar and CompileParserGrammar.
//
// These tests verify that the generated Go source code:
//   - Contains the expected header comments ("DO NOT EDIT", source filename)
//   - Imports the grammar-tools package
//   - Defines the expected constants (TokenGrammarData, ParserGrammarData)
//   - Correctly encodes all grammar fields (version, definitions, patterns, etc.)
//
// Note: We cannot exec() Go code in tests the way Python can, so we verify
// the generated text contains expected substrings rather than round-tripping
// the grammar through the generated code. The Python compiler tests provide
// the authoritative round-trip verification.

import (
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// CompileTokenGrammar — header and structure checks
// ---------------------------------------------------------------------------

func TestCompileTokenGrammarDoNotEditHeader(t *testing.T) {
	g := &TokenGrammar{}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "DO NOT EDIT") {
		t.Error("expected 'DO NOT EDIT' in generated code")
	}
}

func TestCompileTokenGrammarSourceLinePresent(t *testing.T) {
	g := &TokenGrammar{}
	code := CompileTokenGrammar(g, "json.tokens", "generated")
	if !strings.Contains(code, "json.tokens") {
		t.Error("expected source filename in generated code")
	}
}

func TestCompileTokenGrammarSourceLineOmitted(t *testing.T) {
	g := &TokenGrammar{}
	code := CompileTokenGrammar(g, "", "generated")
	if strings.Contains(code, "// Source:") {
		t.Error("expected no '// Source:' line when source is empty")
	}
}

func TestCompileTokenGrammarImportsPackage(t *testing.T) {
	g := &TokenGrammar{}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "grammar-tools") {
		t.Error("expected import of grammar-tools package")
	}
}

func TestCompileTokenGrammarDefinesConstant(t *testing.T) {
	g := &TokenGrammar{}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "TokenGrammarData") {
		t.Error("expected 'TokenGrammarData' constant in generated code")
	}
}

func TestCompileTokenGrammarPackageName(t *testing.T) {
	g := &TokenGrammar{}
	code := CompileTokenGrammar(g, "", "mypkg")
	if !strings.Contains(code, "package mypkg") {
		t.Error("expected 'package mypkg' declaration")
	}
}

func TestCompileTokenGrammarVersion(t *testing.T) {
	g := &TokenGrammar{Version: 7}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "7") {
		t.Error("expected version 7 in generated code")
	}
}

func TestCompileTokenGrammarDefinitionName(t *testing.T) {
	g := &TokenGrammar{
		Definitions: []TokenDefinition{
			{Name: "MYTOKEN", Pattern: "[a-z]+", IsRegex: true, LineNumber: 1},
		},
	}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "MYTOKEN") {
		t.Error("expected token name 'MYTOKEN' in generated code")
	}
}

func TestCompileTokenGrammarPattern(t *testing.T) {
	g := &TokenGrammar{
		Definitions: []TokenDefinition{
			{Name: "NUM", Pattern: "[0-9]+", IsRegex: true, LineNumber: 1},
		},
	}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "[0-9]+") {
		t.Error("expected pattern '[0-9]+' in generated code")
	}
}

func TestCompileTokenGrammarAlias(t *testing.T) {
	g := &TokenGrammar{
		Definitions: []TokenDefinition{
			{Name: "STRING_DQ", Pattern: `"[^"]*"`, IsRegex: true, LineNumber: 1, Alias: "STRING"},
		},
	}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "STRING") {
		t.Error("expected alias 'STRING' in generated code")
	}
}

func TestCompileTokenGrammarKeywords(t *testing.T) {
	g := &TokenGrammar{Keywords: []string{"if", "else", "while"}}
	code := CompileTokenGrammar(g, "", "generated")
	// keywords appear as backtick-quoted raw strings (e.g. `if`)
	if !strings.Contains(code, "if") || !strings.Contains(code, "else") {
		t.Error("expected keyword words in generated code")
	}
}

func TestCompileTokenGrammarSkipDefinitions(t *testing.T) {
	g := &TokenGrammar{
		SkipDefinitions: []TokenDefinition{
			{Name: "WHITESPACE", Pattern: `[ \t]+`, IsRegex: true, LineNumber: 5},
		},
	}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "WHITESPACE") {
		t.Error("expected 'WHITESPACE' in generated skip definitions")
	}
}

func TestCompileTokenGrammarPatternGroups(t *testing.T) {
	g := &TokenGrammar{
		Groups: map[string]*PatternGroup{
			"tag": {
				Name: "tag",
				Definitions: []TokenDefinition{
					{Name: "ATTR", Pattern: "[a-z]+", IsRegex: true, LineNumber: 10},
				},
			},
		},
	}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "tag") {
		t.Error("expected group name 'tag' in generated code")
	}
	if !strings.Contains(code, "ATTR") {
		t.Error("expected group token 'ATTR' in generated code")
	}
}

func TestCompileTokenGrammarCaseSensitive(t *testing.T) {
	g := &TokenGrammar{CaseSensitive: true}
	code := CompileTokenGrammar(g, "", "generated")
	if !strings.Contains(code, "CaseSensitive") {
		t.Error("expected 'CaseSensitive' field in generated code")
	}
}

// ---------------------------------------------------------------------------
// CompileParserGrammar — header and structure checks
// ---------------------------------------------------------------------------

func TestCompileParserGrammarDoNotEditHeader(t *testing.T) {
	g := &ParserGrammar{}
	code := CompileParserGrammar(g, "", "generated")
	if !strings.Contains(code, "DO NOT EDIT") {
		t.Error("expected 'DO NOT EDIT' in generated code")
	}
}

func TestCompileParserGrammarDefinesConstant(t *testing.T) {
	g := &ParserGrammar{}
	code := CompileParserGrammar(g, "", "generated")
	if !strings.Contains(code, "ParserGrammarData") {
		t.Error("expected 'ParserGrammarData' constant")
	}
}

func TestCompileParserGrammarImportsPackage(t *testing.T) {
	g := &ParserGrammar{}
	code := CompileParserGrammar(g, "", "generated")
	if !strings.Contains(code, "grammar-tools") {
		t.Error("expected import of grammar-tools package")
	}
}

func TestCompileParserGrammarRuleName(t *testing.T) {
	source := "value = NUMBER ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "value") {
		t.Error("expected rule name 'value' in generated code")
	}
}

func TestCompileParserGrammarRuleReference(t *testing.T) {
	source := "start = NUMBER ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "NUMBER") {
		t.Error("expected token reference 'NUMBER' in generated code")
	}
	if !strings.Contains(code, "RuleReference") {
		t.Error("expected 'RuleReference' struct in generated code")
	}
}

func TestCompileParserGrammarAlternation(t *testing.T) {
	source := "value = A | B | C ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "Alternation") {
		t.Error("expected 'Alternation' in generated code")
	}
}

func TestCompileParserGrammarSequence(t *testing.T) {
	source := "pair = KEY COLON VALUE ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "Sequence") {
		t.Error("expected 'Sequence' in generated code")
	}
}

func TestCompileParserGrammarRepetition(t *testing.T) {
	source := "stmts = { stmt } ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "Repetition") {
		t.Error("expected 'Repetition' in generated code")
	}
}

func TestCompileParserGrammarOptional(t *testing.T) {
	source := "expr = NUMBER [ PLUS NUMBER ] ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "Optional") {
		t.Error("expected 'Optional' in generated code")
	}
}

func TestCompileParserGrammarGroup(t *testing.T) {
	source := "term = NUMBER { ( PLUS | MINUS ) NUMBER } ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "Group") {
		t.Error("expected 'Group' in generated code")
	}
}

func TestCompileParserGrammarLiteral(t *testing.T) {
	source := `start = "hello" ;`
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "Literal") {
		t.Error("expected 'Literal' in generated code")
	}
	if !strings.Contains(code, "hello") {
		t.Error("expected literal value 'hello' in generated code")
	}
}

func TestCompileParserGrammarVersion(t *testing.T) {
	source := "# @version 5\nvalue = NUMBER ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "", "generated")
	if !strings.Contains(code, "5") {
		t.Error("expected version 5 in generated code")
	}
}

func TestCompileParserGrammarJSONRules(t *testing.T) {
	source := `
value    = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
object   = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair     = STRING COLON value ;
array    = LBRACKET [ value { COMMA value } ] RBRACKET ;
`
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileParserGrammar(grammar, "json.grammar", "generated")
	// All four rule names must appear.
	for _, name := range []string{"value", "object", "pair", "array"} {
		if !strings.Contains(code, name) {
			t.Errorf("expected rule name %q in generated code", name)
		}
	}
	// Source file in header.
	if !strings.Contains(code, "json.grammar") {
		t.Error("expected source file 'json.grammar' in header")
	}
}

// ---------------------------------------------------------------------------
// goStringLit
// ---------------------------------------------------------------------------

func TestGoStringLitNoBacktick(t *testing.T) {
	// Patterns without backticks → raw string literal.
	result := goStringLit(`[0-9]+`)
	if result[0] != '`' {
		t.Errorf("expected raw string, got %q", result)
	}
}

func TestGoStringLitWithBacktick(t *testing.T) {
	// Patterns with backticks → double-quoted string.
	result := goStringLit("hello`world")
	if result[0] != '"' {
		t.Errorf("expected double-quoted string, got %q", result)
	}
}
