package grammartools

import (
	"reflect"
	"strings"
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

// -----------------------------------------------------------------------
// Pattern groups: parsing tests
// -----------------------------------------------------------------------

func TestParseBasicGroup(t *testing.T) {
	// A simple group section is parsed into a PatternGroup with the
	// correct name and definitions.
	source := "TEXT = /[^<]+/\nTAG_OPEN = \"<\"\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n  TAG_CLOSE = \">\"\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	// Default group patterns
	if len(grammar.Definitions) != 2 {
		t.Fatalf("Expected 2 top-level definitions, got %d", len(grammar.Definitions))
	}
	if grammar.Definitions[0].Name != "TEXT" {
		t.Errorf("Expected first def 'TEXT', got %q", grammar.Definitions[0].Name)
	}
	if grammar.Definitions[1].Name != "TAG_OPEN" {
		t.Errorf("Expected second def 'TAG_OPEN', got %q", grammar.Definitions[1].Name)
	}

	// Named group
	group, exists := grammar.Groups["tag"]
	if !exists {
		t.Fatal("Expected group 'tag' to exist")
	}
	if group.Name != "tag" {
		t.Errorf("Expected group name 'tag', got %q", group.Name)
	}
	if len(group.Definitions) != 2 {
		t.Fatalf("Expected 2 group definitions, got %d", len(group.Definitions))
	}
	if group.Definitions[0].Name != "TAG_NAME" {
		t.Errorf("Expected group def 'TAG_NAME', got %q", group.Definitions[0].Name)
	}
	if group.Definitions[1].Name != "TAG_CLOSE" {
		t.Errorf("Expected group def 'TAG_CLOSE', got %q", group.Definitions[1].Name)
	}
}

func TestParseMultipleGroups(t *testing.T) {
	// Multiple groups can be defined in the same file.
	source := "TEXT = /[^<]+/\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n\ngroup cdata:\n  CDATA_TEXT = /[^]]+/\n  CDATA_END = \"]]>\"\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	if len(grammar.Groups) != 2 {
		t.Fatalf("Expected 2 groups, got %d", len(grammar.Groups))
	}
	if _, exists := grammar.Groups["tag"]; !exists {
		t.Error("Expected group 'tag' to exist")
	}
	if _, exists := grammar.Groups["cdata"]; !exists {
		t.Error("Expected group 'cdata' to exist")
	}
	if len(grammar.Groups["tag"].Definitions) != 1 {
		t.Errorf("Expected 1 def in 'tag', got %d", len(grammar.Groups["tag"].Definitions))
	}
	if len(grammar.Groups["cdata"].Definitions) != 2 {
		t.Errorf("Expected 2 defs in 'cdata', got %d", len(grammar.Groups["cdata"].Definitions))
	}
}

func TestParseGroupWithAlias(t *testing.T) {
	// Definitions inside groups support -> ALIAS syntax.
	source := "TEXT = /[^<]+/\n\ngroup tag:\n  ATTR_VALUE_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n  ATTR_VALUE_SQ = /'[^']*'/ -> ATTR_VALUE\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	group := grammar.Groups["tag"]
	if group.Definitions[0].Name != "ATTR_VALUE_DQ" {
		t.Errorf("Expected 'ATTR_VALUE_DQ', got %q", group.Definitions[0].Name)
	}
	if group.Definitions[0].Alias != "ATTR_VALUE" {
		t.Errorf("Expected alias 'ATTR_VALUE', got %q", group.Definitions[0].Alias)
	}
	if group.Definitions[1].Name != "ATTR_VALUE_SQ" {
		t.Errorf("Expected 'ATTR_VALUE_SQ', got %q", group.Definitions[1].Name)
	}
	if group.Definitions[1].Alias != "ATTR_VALUE" {
		t.Errorf("Expected alias 'ATTR_VALUE', got %q", group.Definitions[1].Alias)
	}
}

func TestParseGroupWithLiteralPatterns(t *testing.T) {
	// Groups support both regex and literal patterns.
	source := "TEXT = /[^<]+/\n\ngroup tag:\n  EQUALS = \"=\"\n  TAG_NAME = /[a-zA-Z]+/\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	group := grammar.Groups["tag"]
	if group.Definitions[0].IsRegex {
		t.Error("Expected first def to be literal (IsRegex=false)")
	}
	if group.Definitions[0].Pattern != "=" {
		t.Errorf("Expected pattern '=', got %q", group.Definitions[0].Pattern)
	}
	if !group.Definitions[1].IsRegex {
		t.Error("Expected second def to be regex (IsRegex=true)")
	}
}

func TestNoGroupsBackwardCompat(t *testing.T) {
	// Files without groups have an empty (but non-nil) groups map.
	source := "NUMBER = /[0-9]+/\nPLUS = \"+\"\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	if grammar.Groups == nil {
		t.Fatal("Expected non-nil Groups map")
	}
	if len(grammar.Groups) != 0 {
		t.Errorf("Expected 0 groups, got %d", len(grammar.Groups))
	}
	if len(grammar.Definitions) != 2 {
		t.Errorf("Expected 2 definitions, got %d", len(grammar.Definitions))
	}
}

func TestGroupsWithSkipSection(t *testing.T) {
	// skip: and group: sections coexist correctly.
	source := "skip:\n  WS = /[ \\t]+/\n\nTEXT = /[^<]+/\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	if len(grammar.SkipDefinitions) != 1 {
		t.Errorf("Expected 1 skip def, got %d", len(grammar.SkipDefinitions))
	}
	if len(grammar.Definitions) != 1 {
		t.Errorf("Expected 1 definition, got %d", len(grammar.Definitions))
	}
	if len(grammar.Groups) != 1 {
		t.Errorf("Expected 1 group, got %d", len(grammar.Groups))
	}
}

func TestTokenNamesIncludesGroups(t *testing.T) {
	// TokenNames() includes names from all groups, including aliases.
	source := "TEXT = /[^<]+/\n\ngroup tag:\n  TAG_NAME = /[a-zA-Z]+/\n  ATTR_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	names := grammar.TokenNames()
	for _, expected := range []string{"TEXT", "TAG_NAME", "ATTR_DQ", "ATTR_VALUE"} {
		if !names[expected] {
			t.Errorf("Expected TokenNames to contain %q", expected)
		}
	}
}

func TestEffectiveTokenNamesIncludesGroups(t *testing.T) {
	// EffectiveTokenNames() includes aliased names from groups.
	source := "TEXT = /[^<]+/\n\ngroup tag:\n  ATTR_DQ = /\"[^\"]*\"/ -> ATTR_VALUE\n"
	grammar, err := ParseTokenGrammar(source)
	if err != nil {
		t.Fatalf("Failed to parse: %v", err)
	}

	names := grammar.EffectiveTokenNames()
	if !names["TEXT"] {
		t.Error("Expected EffectiveTokenNames to contain 'TEXT'")
	}
	if !names["ATTR_VALUE"] {
		t.Error("Expected EffectiveTokenNames to contain 'ATTR_VALUE'")
	}
	if names["ATTR_DQ"] {
		t.Error("EffectiveTokenNames should NOT contain 'ATTR_DQ' (alias replaces name)")
	}
}

// -----------------------------------------------------------------------
// Pattern groups: validation tests
// -----------------------------------------------------------------------

func TestValidateGroupBadRegex(t *testing.T) {
	// Definitions in groups are validated (e.g., bad regex detected).
	grammar := &TokenGrammar{
		Groups: map[string]*PatternGroup{
			"tag": {
				Name: "tag",
				Definitions: []TokenDefinition{
					{Name: "BAD", Pattern: "[invalid", IsRegex: true, LineNumber: 5},
				},
			},
		},
	}
	issues := ValidateTokenGrammar(grammar)
	found := false
	for _, issue := range issues {
		if strings.Contains(issue, "Invalid regex") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("Expected 'Invalid regex' issue, got: %v", issues)
	}
}

func TestValidateEmptyGroupWarning(t *testing.T) {
	// An empty group produces a validation warning.
	grammar := &TokenGrammar{
		Groups: map[string]*PatternGroup{
			"empty": {Name: "empty", Definitions: nil},
		},
	}
	issues := ValidateTokenGrammar(grammar)
	found := false
	for _, issue := range issues {
		if strings.Contains(issue, "Empty pattern group") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("Expected 'Empty pattern group' issue, got: %v", issues)
	}
}

// -----------------------------------------------------------------------
// Pattern groups: error handling tests
// -----------------------------------------------------------------------

func TestParseGroupMissingName(t *testing.T) {
	// "group :" with no name raises an error.
	_, err := ParseTokenGrammar("TEXT = /abc/\ngroup :\n  FOO = /x/\n")
	if err == nil {
		t.Fatal("Expected error for missing group name")
	}
	if !strings.Contains(err.Error(), "Missing group name") {
		t.Errorf("Expected 'Missing group name' error, got: %v", err)
	}
}

func TestParseGroupInvalidNameUppercase(t *testing.T) {
	// Uppercase group names are rejected.
	_, err := ParseTokenGrammar("TEXT = /abc/\ngroup Tag:\n  FOO = /x/\n")
	if err == nil {
		t.Fatal("Expected error for uppercase group name")
	}
	if !strings.Contains(err.Error(), "Invalid group name") {
		t.Errorf("Expected 'Invalid group name' error, got: %v", err)
	}
}

func TestParseGroupInvalidNameStartsWithDigit(t *testing.T) {
	// Group names starting with a digit are rejected.
	_, err := ParseTokenGrammar("TEXT = /abc/\ngroup 1tag:\n  FOO = /x/\n")
	if err == nil {
		t.Fatal("Expected error for digit-starting group name")
	}
	if !strings.Contains(err.Error(), "Invalid group name") {
		t.Errorf("Expected 'Invalid group name' error, got: %v", err)
	}
}

func TestParseGroupReservedNameDefault(t *testing.T) {
	// "group default:" is rejected as reserved.
	_, err := ParseTokenGrammar("TEXT = /abc/\ngroup default:\n  FOO = /x/\n")
	if err == nil {
		t.Fatal("Expected error for reserved group name 'default'")
	}
	if !strings.Contains(err.Error(), "Reserved group name") {
		t.Errorf("Expected 'Reserved group name' error, got: %v", err)
	}
}

func TestParseGroupReservedNameSkip(t *testing.T) {
	// "group skip:" is rejected as reserved.
	_, err := ParseTokenGrammar("TEXT = /abc/\ngroup skip:\n  FOO = /x/\n")
	if err == nil {
		t.Fatal("Expected error for reserved group name 'skip'")
	}
	if !strings.Contains(err.Error(), "Reserved group name") {
		t.Errorf("Expected 'Reserved group name' error, got: %v", err)
	}
}

func TestParseGroupReservedNameKeywords(t *testing.T) {
	// "group keywords:" is rejected as reserved.
	_, err := ParseTokenGrammar("TEXT = /abc/\ngroup keywords:\n  FOO = /x/\n")
	if err == nil {
		t.Fatal("Expected error for reserved group name 'keywords'")
	}
	if !strings.Contains(err.Error(), "Reserved group name") {
		t.Errorf("Expected 'Reserved group name' error, got: %v", err)
	}
}

func TestParseGroupDuplicateName(t *testing.T) {
	// Two groups with the same name raises an error.
	source := "TEXT = /abc/\ngroup tag:\n  FOO = /x/\ngroup tag:\n  BAR = /y/\n"
	_, err := ParseTokenGrammar(source)
	if err == nil {
		t.Fatal("Expected error for duplicate group name")
	}
	if !strings.Contains(err.Error(), "Duplicate group name") {
		t.Errorf("Expected 'Duplicate group name' error, got: %v", err)
	}
}

func TestParseGroupBadDefinition(t *testing.T) {
	// Invalid definition inside a group raises an error.
	source := "TEXT = /abc/\ngroup tag:\n  not a definition\n"
	_, err := ParseTokenGrammar(source)
	if err == nil {
		t.Fatal("Expected error for bad definition in group")
	}
	if !strings.Contains(err.Error(), "Expected token definition") {
		t.Errorf("Expected 'Expected token definition' error, got: %v", err)
	}
}

func TestParseGroupIncompleteDefinition(t *testing.T) {
	// Missing pattern in group definition raises an error.
	source := "TEXT = /abc/\ngroup tag:\n  FOO = \n"
	_, err := ParseTokenGrammar(source)
	if err == nil {
		t.Fatal("Expected error for incomplete definition in group")
	}
	if !strings.Contains(err.Error(), "Incomplete definition") {
		t.Errorf("Expected 'Incomplete definition' error, got: %v", err)
	}
}
