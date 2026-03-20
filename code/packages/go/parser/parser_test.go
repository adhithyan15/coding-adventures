package parser

import (
	"reflect"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestParseExpression(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}

	parser := NewParser(tokens)
	prog := parser.Parse()

	if len(prog.Statements) != 1 {
		t.Fatalf("Expected 1 statement, got %d", len(prog.Statements))
	}

	stmt, ok := prog.Statements[0].(ExpressionStmt)
	if !ok {
		t.Fatalf("Expected ExpressionStmt, got %T", prog.Statements[0])
	}

	expectedExpr := BinaryOp{
		Left:  NumberLiteral{Value: 1},
		Op:    "+",
		Right: NumberLiteral{Value: 2},
	}

	if !reflect.DeepEqual(stmt.Expression, expectedExpr) {
		t.Errorf("Expected expression %v, got %v", expectedExpr, stmt.Expression)
	}
}

func TestParseAssignment(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenEquals, Value: "=", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 7},
	}

	parser := NewParser(tokens)
	prog := parser.Parse()

	if len(prog.Statements) != 1 {
		t.Fatalf("Expected 1 statement, got %d", len(prog.Statements))
	}

	stmt, ok := prog.Statements[0].(Assignment)
	if !ok {
		t.Fatalf("Expected Assignment, got %T", prog.Statements[0])
	}

	if stmt.Target.Name != "x" {
		t.Errorf("Expected target name 'x', got '%s'", stmt.Target.Name)
	}

	val, ok := stmt.Value.(NumberLiteral)
	if !ok || val.Value != 42 {
		t.Errorf("Expected NumberLiteral 42, got %v", stmt.Value)
	}
}

// -----------------------------------------------------------------------
// Grammar parser: packrat memoization
// -----------------------------------------------------------------------

func TestGrammarParserMemoization(t *testing.T) {
	grammarSource := `
program = { statement } ;
statement = assignment | expression_stmt ;
assignment = NAME EQUALS expression ;
expression_stmt = expression ;
expression = NUMBER ;
`
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed to parse grammar: %v", err)
	}

	// Parse the same input twice to exercise memoization
	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 3},
	}
	parser1 := NewGrammarParser(tokens, pg)
	ast1, err := parser1.Parse()
	if err != nil {
		t.Fatalf("First parse failed: %v", err)
	}
	parser2 := NewGrammarParser(tokens, pg)
	ast2, err := parser2.Parse()
	if err != nil {
		t.Fatalf("Second parse failed: %v", err)
	}
	if ast1.RuleName != ast2.RuleName {
		t.Errorf("Memoization produced different results")
	}
}

// -----------------------------------------------------------------------
// Grammar parser: string-based token types
// -----------------------------------------------------------------------

func TestGrammarParserStringTypes(t *testing.T) {
	grammarSource := "expr = INT ;"
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "42", Line: 1, Column: 1, TypeName: "INT"},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 3, TypeName: "EOF"},
	}
	parser := NewGrammarParser(tokens, pg)
	ast, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "expr" {
		t.Errorf("Expected rule 'expr', got %q", ast.RuleName)
	}
}

// -----------------------------------------------------------------------
// Grammar parser: significant newlines
// -----------------------------------------------------------------------

func TestGrammarParserNewlinesSignificant(t *testing.T) {
	grammarSource := "file = { NAME NEWLINE } ;"
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenNewline, Value: "\\n", Line: 1, Column: 2},
		{Type: lexer.TokenEOF, Value: "", Line: 2, Column: 1},
	}
	parser := NewGrammarParser(tokens, pg)
	if !parser.NewlinesSignificant() {
		t.Error("Expected newlines significant")
	}
	ast, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "file" {
		t.Errorf("Expected 'file', got %q", ast.RuleName)
	}
}

func TestGrammarParserNewlinesInsignificant(t *testing.T) {
	grammarSource := "expr = NUMBER ;"
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	tokens := []lexer.Token{
		{Type: lexer.TokenNewline, Value: "\\n", Line: 1, Column: 1},
		{Type: lexer.TokenNumber, Value: "42", Line: 2, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 2, Column: 3},
	}
	parser := NewGrammarParser(tokens, pg)
	if parser.NewlinesSignificant() {
		t.Error("Expected newlines insignificant")
	}
	ast, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "expr" {
		t.Errorf("Expected 'expr', got %q", ast.RuleName)
	}
}

// -----------------------------------------------------------------------
// Grammar parser: error handling
// -----------------------------------------------------------------------

func TestGrammarParserEmptyGrammar(t *testing.T) {
	pg := &grammartools.ParserGrammar{Rules: []grammartools.GrammarRule{}}
	tokens := []lexer.Token{
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 1},
	}
	parser := NewGrammarParser(tokens, pg)
	_, err := parser.Parse()
	if err == nil {
		t.Fatal("Expected error for empty grammar")
	}
}
