package grammartools

import (
	"reflect"
	"testing"
)

func TestParseTokenGrammar(t *testing.T) {
	source := `
NAME = /[a-zA-Z_]+/
EQUALS = "="
keywords:
  if
  else
`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse token grammar: %v", err)
	}

	if len(grammar.Definitions) != 2 {
		t.Fatalf("Expected 2 definitions, got %d", len(grammar.Definitions))
	}
	if grammar.Definitions[0].Name != "NAME" || grammar.Definitions[0].Pattern != "[a-zA-Z_]+" {
		t.Errorf("Mismatch mapping regex definition natively translating boundaries properly")
	}

	if len(grammar.Keywords) != 2 || grammar.Keywords[0] != "if" {
		t.Errorf("Mismatch parsing keywords extracting configuration spaces natively %v", grammar.Keywords)
	}
}

func TestParseModeDirective(t *testing.T) {
	source := "mode: indentation\nNAME = /[a-z]+/"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if grammar.Mode != "indentation" {
		t.Errorf("Expected mode 'indentation', got %q", grammar.Mode)
	}
}

func TestParseSkipSection(t *testing.T) {
	source := "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.SkipDefinitions) != 2 {
		t.Fatalf("Expected 2 skip defs, got %d", len(grammar.SkipDefinitions))
	}
	if grammar.SkipDefinitions[0].Name != "WHITESPACE" {
		t.Errorf("Expected 'WHITESPACE', got %q", grammar.SkipDefinitions[0].Name)
	}
}

func TestParseReservedSection(t *testing.T) {
	source := "NAME = /[a-z]+/\nreserved:\n  class\n  import"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.ReservedKeywords) != 2 {
		t.Fatalf("Expected 2 reserved, got %d", len(grammar.ReservedKeywords))
	}
	if grammar.ReservedKeywords[0] != "class" {
		t.Errorf("Expected 'class', got %q", grammar.ReservedKeywords[0])
	}
}

func TestParseAlias(t *testing.T) {
	source := `STRING_DQ = /"[^"]*"/ -> STRING`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if grammar.Definitions[0].Alias != "STRING" {
		t.Errorf("Expected alias 'STRING', got %q", grammar.Definitions[0].Alias)
	}
}

func TestParseLiteralAlias(t *testing.T) {
	source := `PLUS_SIGN = "+" -> PLUS`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if grammar.Definitions[0].Alias != "PLUS" {
		t.Errorf("Expected alias 'PLUS', got %q", grammar.Definitions[0].Alias)
	}
}

func TestTokenNamesIncludesAliases(t *testing.T) {
	source := `STRING_DQ = /"[^"]*"/ -> STRING`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	names := grammar.TokenNames()
	if !names["STRING_DQ"] || !names["STRING"] {
		t.Errorf("TokenNames should include both original and alias")
	}
}

func TestParseModeMissingValue(t *testing.T) {
	_, err := ParseTokenGrammar("mode:")
	if err == nil {
		t.Fatal("Expected error for missing mode value")
	}
}

func TestParseAliasMissing(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = /x/ ->")
	if err == nil {
		t.Fatal("Expected error for missing alias")
	}
}

func TestParseUnclosedRegex(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = /unclosed")
	if err == nil {
		t.Fatal("Expected error for unclosed regex")
	}
}

func TestParseUnclosedLiteral(t *testing.T) {
	_, err := ParseTokenGrammar(`FOO = "unclosed`)
	if err == nil {
		t.Fatal("Expected error for unclosed literal")
	}
}

func TestParseSkipMissingEquals(t *testing.T) {
	_, err := ParseTokenGrammar("skip:\n  BAD_PATTERN")
	if err == nil {
		t.Fatal("Expected error for skip without equals")
	}
}

func TestParseSkipIncomplete(t *testing.T) {
	_, err := ParseTokenGrammar("skip:\n  BAD =")
	if err == nil {
		t.Fatal("Expected error for incomplete skip")
	}
}

func TestParseStarlarkTokens(t *testing.T) {
	// Test with the real starlark.tokens file if available
	source := `
mode: indentation

NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
INT = /[0-9]+/
EQUALS = "="
PLUS = "+"
COLON = ":"
LPAREN = "("
RPAREN = ")"
COMMA = ","

keywords:
  def
  return
  if
  else
  for
  in
  pass

reserved:
  class
  import

skip:
  WHITESPACE = /[ \t]+/
  COMMENT = /#[^\n]*/
`
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse starlark-like tokens: %v", err)
	}
	if grammar.Mode != "indentation" {
		t.Errorf("Expected mode 'indentation'")
	}
	if len(grammar.ReservedKeywords) != 2 {
		t.Errorf("Expected 2 reserved keywords, got %d", len(grammar.ReservedKeywords))
	}
	if len(grammar.SkipDefinitions) != 2 {
		t.Errorf("Expected 2 skip definitions, got %d", len(grammar.SkipDefinitions))
	}
}

// -----------------------------------------------------------------------
// Token grammar: additional error paths
// -----------------------------------------------------------------------

func TestParseMissingTokenName(t *testing.T) {
	_, err := ParseTokenGrammar(" = /abc/")
	if err == nil {
		t.Fatal("Expected error for missing token name")
	}
}

func TestParseMissingPattern(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = ")
	if err == nil {
		t.Fatal("Expected error for missing pattern after =")
	}
}

func TestParseBadDelimiter(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = xyz")
	if err == nil {
		t.Fatal("Expected error for bad delimiter")
	}
}

func TestParseEmptyRegex(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = //")
	if err == nil {
		t.Fatal("Expected error for empty regex")
	}
}

func TestParseEmptyLiteral(t *testing.T) {
	_, err := ParseTokenGrammar(`FOO = ""`)
	if err == nil {
		t.Fatal("Expected error for empty literal")
	}
}

func TestParseUnexpectedTextAfterRegex(t *testing.T) {
	_, err := ParseTokenGrammar("FOO = /abc/ extra")
	if err == nil {
		t.Fatal("Expected error for unexpected text after regex")
	}
}

func TestParseUnexpectedTextAfterLiteral(t *testing.T) {
	_, err := ParseTokenGrammar(`FOO = "+" extra`)
	if err == nil {
		t.Fatal("Expected error for unexpected text after literal")
	}
}

func TestParseLiteralAliasMissing(t *testing.T) {
	_, err := ParseTokenGrammar(`FOO = "+" ->`)
	if err == nil {
		t.Fatal("Expected error for missing alias on literal")
	}
}

func TestParseNoEquals(t *testing.T) {
	_, err := ParseTokenGrammar("FOO /abc/")
	if err == nil {
		t.Fatal("Expected error for line without equals")
	}
}

func TestParseSectionExitOnNonIndented(t *testing.T) {
	// After keywords section, a non-indented definition line should exit the section
	source := "keywords:\n  if\nNAME = /[a-z]+/"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.Keywords) != 1 || grammar.Keywords[0] != "if" {
		t.Errorf("Expected 1 keyword 'if', got %v", grammar.Keywords)
	}
	if len(grammar.Definitions) != 1 || grammar.Definitions[0].Name != "NAME" {
		t.Errorf("Expected 1 definition 'NAME', got %v", grammar.Definitions)
	}
}

func TestParseCommentLines(t *testing.T) {
	source := "# comment\nNAME = /[a-z]+/\n# another comment"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.Definitions) != 1 {
		t.Errorf("Expected 1 definition, got %d", len(grammar.Definitions))
	}
}

func TestParseKeywordsAlternateHeader(t *testing.T) {
	// keywords : (with space before colon)
	source := "NAME = /[a-z]+/\nkeywords :\n  for"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.Keywords) != 1 || grammar.Keywords[0] != "for" {
		t.Errorf("Expected keyword 'for', got %v", grammar.Keywords)
	}
}

func TestParseReservedAlternateHeader(t *testing.T) {
	source := "NAME = /[a-z]+/\nreserved :\n  yield"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.ReservedKeywords) != 1 || grammar.ReservedKeywords[0] != "yield" {
		t.Errorf("Expected reserved 'yield', got %v", grammar.ReservedKeywords)
	}
}

func TestParseSkipAlternateHeader(t *testing.T) {
	source := "NAME = /[a-z]+/\nskip :\n  WS = /[ ]+/"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.SkipDefinitions) != 1 {
		t.Errorf("Expected 1 skip def, got %d", len(grammar.SkipDefinitions))
	}
}

// -----------------------------------------------------------------------
// Parser grammar: comprehensive tests
// -----------------------------------------------------------------------

func TestParseParserGrammar(t *testing.T) {
	source := `
expression = term { ( PLUS | MINUS ) term } ;
term = NUMBER ;
`
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse grammar structurally safely %v", err)
	}

	if len(grammar.Rules) != 2 {
		t.Fatalf("Expected 2 rules, got %v", len(grammar.Rules))
	}

	expectedType := reflect.TypeOf(Sequence{})
	actualType := reflect.TypeOf(grammar.Rules[0].Body)
	if expectedType != actualType {
		t.Errorf("Mismatch evaluating nested blocks dynamically translating sequences explicitly.")
	}
}

func TestParserGrammarAlternation(t *testing.T) {
	source := "expr = NUMBER | NAME ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	_, ok := grammar.Rules[0].Body.(Alternation)
	if !ok {
		t.Errorf("Expected Alternation body, got %T", grammar.Rules[0].Body)
	}
}

func TestParserGrammarOptional(t *testing.T) {
	source := "expr = NUMBER [ PLUS NUMBER ] ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	seq, ok := grammar.Rules[0].Body.(Sequence)
	if !ok {
		t.Fatalf("Expected Sequence body, got %T", grammar.Rules[0].Body)
	}
	_, ok = seq.Elements[1].(Optional)
	if !ok {
		t.Errorf("Expected second element to be Optional, got %T", seq.Elements[1])
	}
}

func TestParserGrammarRepetition(t *testing.T) {
	source := "list = { NUMBER } ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	_, ok := grammar.Rules[0].Body.(Repetition)
	if !ok {
		t.Errorf("Expected Repetition body, got %T", grammar.Rules[0].Body)
	}
}

func TestParserGrammarGroup(t *testing.T) {
	source := "expr = ( NUMBER ) ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	_, ok := grammar.Rules[0].Body.(Group)
	if !ok {
		t.Errorf("Expected Group body, got %T", grammar.Rules[0].Body)
	}
}

func TestParserGrammarLiteral(t *testing.T) {
	source := `expr = NUMBER "+" NUMBER ;`
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	seq, ok := grammar.Rules[0].Body.(Sequence)
	if !ok {
		t.Fatalf("Expected Sequence, got %T", grammar.Rules[0].Body)
	}
	lit, ok := seq.Elements[1].(Literal)
	if !ok {
		t.Errorf("Expected Literal, got %T", seq.Elements[1])
	}
	if lit.Value != "+" {
		t.Errorf("Expected literal '+', got %q", lit.Value)
	}
}

func TestParserGrammarRuleReference(t *testing.T) {
	// lowercase = rule reference, UPPER = token reference
	source := "program = expr ; expr = NUMBER ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	ref, ok := grammar.Rules[0].Body.(RuleReference)
	if !ok {
		t.Fatalf("Expected RuleReference, got %T", grammar.Rules[0].Body)
	}
	if ref.IsToken {
		t.Error("lowercase reference should not be a token")
	}
	if ref.Name != "expr" {
		t.Errorf("Expected 'expr', got %q", ref.Name)
	}
}

func TestParserGrammarTokenReference(t *testing.T) {
	source := "expr = NUMBER ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	ref, ok := grammar.Rules[0].Body.(RuleReference)
	if !ok {
		t.Fatalf("Expected RuleReference, got %T", grammar.Rules[0].Body)
	}
	if !ref.IsToken {
		t.Error("UPPER reference should be a token")
	}
}

func TestParserGrammarMultipleRules(t *testing.T) {
	source := "a = NUMBER ; b = NAME ; c = STRING ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.Rules) != 3 {
		t.Errorf("Expected 3 rules, got %d", len(grammar.Rules))
	}
}

func TestParserGrammarComments(t *testing.T) {
	source := "# comment\nexpr = NUMBER ; # inline comment"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.Rules) != 1 {
		t.Errorf("Expected 1 rule, got %d", len(grammar.Rules))
	}
}

func TestParserGrammarUnterminatedString(t *testing.T) {
	source := `expr = "unclosed ;`
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for unterminated string")
	}
}

func TestParserGrammarUnexpectedChar(t *testing.T) {
	source := "expr = @ ;"
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for unexpected character")
	}
}

func TestParserGrammarMissingSemicolon(t *testing.T) {
	source := "expr = NUMBER"
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for missing semicolon")
	}
}

func TestParserGrammarMissingEquals(t *testing.T) {
	source := "expr NUMBER ;"
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for missing equals")
	}
}

func TestParserGrammarEmptySequence(t *testing.T) {
	source := "expr = | NUMBER ;"
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for empty sequence before pipe")
	}
}

func TestParserGrammarUnclosedBrace(t *testing.T) {
	source := "expr = { NUMBER ;"
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for unclosed brace")
	}
}

func TestParserGrammarUnclosedBracket(t *testing.T) {
	source := "expr = [ NUMBER ;"
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for unclosed bracket")
	}
}

func TestParserGrammarUnclosedParen(t *testing.T) {
	source := "expr = ( NUMBER ;"
	_, err := ParseParserGrammar(source)
	if err == nil {
		t.Fatal("Expected error for unclosed paren")
	}
}

func TestParserGrammarNestedAlternation(t *testing.T) {
	source := "expr = ( NUMBER | NAME | STRING ) ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	grp, ok := grammar.Rules[0].Body.(Group)
	if !ok {
		t.Fatalf("Expected Group, got %T", grammar.Rules[0].Body)
	}
	alt, ok := grp.Element.(Alternation)
	if !ok {
		t.Fatalf("Expected Alternation inside group, got %T", grp.Element)
	}
	if len(alt.Choices) != 3 {
		t.Errorf("Expected 3 choices, got %d", len(alt.Choices))
	}
}

func TestParserGrammarLineNumbers(t *testing.T) {
	source := "\nexpr = NUMBER ;\nterm = NAME ;"
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if grammar.Rules[0].LineNumber != 2 {
		t.Errorf("Expected line 2, got %d", grammar.Rules[0].LineNumber)
	}
	if grammar.Rules[1].LineNumber != 3 {
		t.Errorf("Expected line 3, got %d", grammar.Rules[1].LineNumber)
	}
}

func TestParserGrammarComplexExpression(t *testing.T) {
	source := `
program = { statement } ;
statement = assignment | expr_stmt ;
assignment = NAME EQUALS expression NEWLINE ;
expr_stmt = expression NEWLINE ;
expression = term { ( PLUS | MINUS ) term } ;
term = factor { ( STAR | SLASH ) factor } ;
factor = NUMBER | NAME | "(" expression ")" ;
`
	grammar, err := ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	if len(grammar.Rules) != 7 {
		t.Errorf("Expected 7 rules, got %d", len(grammar.Rules))
	}
}
