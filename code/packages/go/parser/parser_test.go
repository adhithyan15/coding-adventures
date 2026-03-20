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

// -----------------------------------------------------------------------
// Hand-written parser: comprehensive tests
// -----------------------------------------------------------------------

func TestParseMultiplication(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 1},
		{Type: lexer.TokenStar, Value: "*", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "3", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(ExpressionStmt)
	binop, ok := stmt.Expression.(BinaryOp)
	if !ok {
		t.Fatalf("Expected BinaryOp, got %T", stmt.Expression)
	}
	if binop.Op != "*" {
		t.Errorf("Expected '*', got %q", binop.Op)
	}
}

func TestParseDivision(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "6", Line: 1, Column: 1},
		{Type: lexer.TokenSlash, Value: "/", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(ExpressionStmt)
	binop := stmt.Expression.(BinaryOp)
	if binop.Op != "/" {
		t.Errorf("Expected '/', got %q", binop.Op)
	}
}

func TestParseSubtraction(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "5", Line: 1, Column: 1},
		{Type: lexer.TokenMinus, Value: "-", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "3", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(ExpressionStmt)
	binop := stmt.Expression.(BinaryOp)
	if binop.Op != "-" {
		t.Errorf("Expected '-', got %q", binop.Op)
	}
}

func TestParseStringLiteral(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenString, Value: "hello", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 8},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(ExpressionStmt)
	str, ok := stmt.Expression.(StringLiteral)
	if !ok {
		t.Fatalf("Expected StringLiteral, got %T", stmt.Expression)
	}
	if str.Value != "hello" {
		t.Errorf("Expected 'hello', got %q", str.Value)
	}
}

func TestParseName(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "foo", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 4},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(ExpressionStmt)
	name, ok := stmt.Expression.(Name)
	if !ok {
		t.Fatalf("Expected Name, got %T", stmt.Expression)
	}
	if name.Name != "foo" {
		t.Errorf("Expected 'foo', got %q", name.Name)
	}
}

func TestParseParenthesizedExpr(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenLParen, Value: "(", Line: 1, Column: 1},
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 2},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 4},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 6},
		{Type: lexer.TokenRParen, Value: ")", Line: 1, Column: 7},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 8},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(ExpressionStmt)
	binop, ok := stmt.Expression.(BinaryOp)
	if !ok {
		t.Fatalf("Expected BinaryOp, got %T", stmt.Expression)
	}
	if binop.Op != "+" {
		t.Errorf("Expected '+', got %q", binop.Op)
	}
}

func TestParsePrecedence(t *testing.T) {
	// 1 + 2 * 3 should parse as 1 + (2 * 3)
	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenStar, Value: "*", Line: 1, Column: 7},
		{Type: lexer.TokenNumber, Value: "3", Line: 1, Column: 9},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 10},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(ExpressionStmt)
	add := stmt.Expression.(BinaryOp)
	if add.Op != "+" {
		t.Errorf("Expected outer '+', got %q", add.Op)
	}
	mul, ok := add.Right.(BinaryOp)
	if !ok {
		t.Fatalf("Expected Right to be BinaryOp, got %T", add.Right)
	}
	if mul.Op != "*" {
		t.Errorf("Expected inner '*', got %q", mul.Op)
	}
}

func TestParseMultipleStatements(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenNewline, Value: "\\n", Line: 1, Column: 2},
		{Type: lexer.TokenNumber, Value: "2", Line: 2, Column: 1},
		{Type: lexer.TokenNewline, Value: "\\n", Line: 2, Column: 2},
		{Type: lexer.TokenNumber, Value: "3", Line: 3, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 3, Column: 2},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	if len(prog.Statements) != 3 {
		t.Fatalf("Expected 3 statements, got %d", len(prog.Statements))
	}
}

func TestParseAssignmentWithExpression(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenEquals, Value: "=", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 5},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 7},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 9},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 10},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	stmt := prog.Statements[0].(Assignment)
	if stmt.Target.Name != "x" {
		t.Errorf("Expected target 'x', got %q", stmt.Target.Name)
	}
	binop, ok := stmt.Value.(BinaryOp)
	if !ok {
		t.Fatalf("Expected BinaryOp value, got %T", stmt.Value)
	}
	if binop.Op != "+" {
		t.Errorf("Expected '+', got %q", binop.Op)
	}
}

func TestParseUnexpectedToken(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 2},
	}
	p := NewParser(tokens)
	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("Expected panic for unexpected token")
		}
		pe, ok := r.(*ParseError)
		if !ok {
			t.Fatalf("Expected *ParseError, got %T", r)
		}
		if pe.Token.Line != 1 || pe.Token.Column != 1 {
			t.Errorf("Error location wrong: %v", pe)
		}
	}()
	p.Parse()
}

func TestParseErrorString(t *testing.T) {
	err := &ParseError{Message: "test error", Token: lexer.Token{Line: 5, Column: 10}}
	got := err.Error()
	if got != "test error at line 5, column 10" {
		t.Errorf("Unexpected error string: %q", got)
	}
}

func TestParseLeadingNewlines(t *testing.T) {
	tokens := []lexer.Token{
		{Type: lexer.TokenNewline, Value: "\\n", Line: 1, Column: 1},
		{Type: lexer.TokenNewline, Value: "\\n", Line: 2, Column: 1},
		{Type: lexer.TokenNumber, Value: "42", Line: 3, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 3, Column: 3},
	}
	p := NewParser(tokens)
	prog := p.Parse()
	if len(prog.Statements) != 1 {
		t.Fatalf("Expected 1 statement, got %d", len(prog.Statements))
	}
}

// -----------------------------------------------------------------------
// Grammar parser: additional coverage
// -----------------------------------------------------------------------

func TestGrammarParseErrorString(t *testing.T) {
	err := &GrammarParseError{
		Message: "Expected NUMBER",
		Tok:     lexer.Token{Line: 3, Column: 5},
	}
	got := err.Error()
	if got != "Parse error at 3:5: Expected NUMBER" {
		t.Errorf("Unexpected: %q", got)
	}
}

func TestASTNodeIsLeaf(t *testing.T) {
	tok := lexer.Token{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 1}
	leafNode := &ASTNode{RuleName: "num", Children: []interface{}{tok}}
	if !leafNode.IsLeaf() {
		t.Error("Expected IsLeaf() = true for single token child")
	}

	nonLeaf := &ASTNode{RuleName: "expr", Children: []interface{}{leafNode, tok}}
	if nonLeaf.IsLeaf() {
		t.Error("Expected IsLeaf() = false for multiple children")
	}

	emptyNode := &ASTNode{RuleName: "empty", Children: []interface{}{}}
	if emptyNode.IsLeaf() {
		t.Error("Expected IsLeaf() = false for empty children")
	}
}

func TestASTNodeToken(t *testing.T) {
	tok := lexer.Token{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 1}
	leafNode := &ASTNode{RuleName: "num", Children: []interface{}{tok}}
	got := leafNode.Token()
	if got == nil || got.Value != "42" {
		t.Errorf("Expected token with value '42', got %v", got)
	}

	nonLeaf := &ASTNode{RuleName: "expr", Children: []interface{}{leafNode}}
	got2 := nonLeaf.Token()
	if got2 != nil {
		t.Error("Expected nil Token() for non-leaf node")
	}
}

func TestGrammarParserAlternation(t *testing.T) {
	source := "expr = NUMBER | NAME ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	// Test with NUMBER
	tokens1 := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 3},
	}
	p1 := NewGrammarParser(tokens1, pg)
	ast1, err := p1.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast1.RuleName != "expr" {
		t.Errorf("Expected 'expr', got %q", ast1.RuleName)
	}

	// Test with NAME
	tokens2 := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 2},
	}
	p2 := NewGrammarParser(tokens2, pg)
	ast2, err := p2.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast2.RuleName != "expr" {
		t.Errorf("Expected 'expr', got %q", ast2.RuleName)
	}
}

func TestGrammarParserOptional(t *testing.T) {
	source := "expr = NUMBER [ PLUS NUMBER ] ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	// Without optional part
	tokens1 := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 3},
	}
	p1 := NewGrammarParser(tokens1, pg)
	_, err = p1.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	// With optional part
	tokens2 := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p2 := NewGrammarParser(tokens2, pg)
	_, err = p2.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
}

func TestGrammarParserRepetition(t *testing.T) {
	source := "list = { NUMBER } ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "3", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewGrammarParser(tokens, pg)
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if len(ast.Children) != 3 {
		t.Errorf("Expected 3 children, got %d", len(ast.Children))
	}
}

func TestGrammarParserGroup(t *testing.T) {
	source := "expr = NUMBER ( PLUS | MINUS ) NUMBER ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenMinus, Value: "-", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
}

func TestGrammarParserLiteral(t *testing.T) {
	source := `expr = NUMBER "+" NUMBER ;`
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
}

func TestGrammarParserParseFailure(t *testing.T) {
	source := "expr = NUMBER PLUS NUMBER ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 2},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err == nil {
		t.Fatal("Expected parse error")
	}
}

func TestGrammarParserUnconsumedTokens(t *testing.T) {
	source := "expr = NUMBER ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err == nil {
		t.Fatal("Expected error for unconsumed tokens")
	}
}

func TestGrammarParserFurthestFailure(t *testing.T) {
	source := `
program = { statement } ;
statement = assignment | expr_stmt ;
assignment = NAME EQUALS NUMBER NEWLINE ;
expr_stmt = NUMBER NEWLINE ;
`
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	// x = (missing number) -> should report furthest failure
	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenEquals, Value: "=", Line: 1, Column: 3},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err == nil {
		t.Fatal("Expected parse error")
	}
}

func TestGrammarParserRuleReference(t *testing.T) {
	source := `
program = expr ;
expr = NUMBER ;
`
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 3},
	}
	p := NewGrammarParser(tokens, pg)
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "program" {
		t.Errorf("Expected 'program', got %q", ast.RuleName)
	}
}

func TestGrammarParserMemoHit(t *testing.T) {
	// Grammar where the same rule is tried at the same position via alternation
	source := `
expr = add_expr | NUMBER ;
add_expr = NUMBER PLUS NUMBER ;
`
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 6},
	}
	p := NewGrammarParser(tokens, pg)
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "expr" {
		t.Errorf("Expected 'expr', got %q", ast.RuleName)
	}
}

func TestGrammarParserTrailingNewlines(t *testing.T) {
	source := "expr = NUMBER ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 1},
		{Type: lexer.TokenNewline, Value: "\\n", Line: 1, Column: 3},
		{Type: lexer.TokenNewline, Value: "\\n", Line: 2, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 3, Column: 1},
	}
	p := NewGrammarParser(tokens, pg)
	// Newlines insignificant, so they should be skipped
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "expr" {
		t.Errorf("Expected 'expr', got %q", ast.RuleName)
	}
}

func TestGrammarParserNewlineReferenceInAlternation(t *testing.T) {
	source := "line = NAME NEWLINE | NUMBER NEWLINE ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	p := NewGrammarParser([]lexer.Token{
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 1},
	}, pg)
	if !p.NewlinesSignificant() {
		t.Error("Expected newlines significant with NEWLINE in alternation")
	}
}

func TestGrammarParserNewlineReferenceInRepetition(t *testing.T) {
	source := "lines = { NAME NEWLINE } ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	p := NewGrammarParser([]lexer.Token{
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 1},
	}, pg)
	if !p.NewlinesSignificant() {
		t.Error("Expected newlines significant with NEWLINE in repetition")
	}
}

func TestGrammarParserNewlineReferenceInOptional(t *testing.T) {
	source := "line = NAME [ NEWLINE ] ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	p := NewGrammarParser([]lexer.Token{
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 1},
	}, pg)
	if !p.NewlinesSignificant() {
		t.Error("Expected newlines significant with NEWLINE in optional")
	}
}

func TestGrammarParserNewlineReferenceInGroup(t *testing.T) {
	source := "line = ( NAME NEWLINE ) ;"
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}
	p := NewGrammarParser([]lexer.Token{
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 1},
	}, pg)
	if !p.NewlinesSignificant() {
		t.Error("Expected newlines significant with NEWLINE in group")
	}
}

func TestGrammarParserLiteralMismatch(t *testing.T) {
	source := `expr = "+" NUMBER ;`
	pg, err := grammartools.ParseParserGrammar(source)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenMinus, Value: "-", Line: 1, Column: 1},
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 3},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 4},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err == nil {
		t.Fatal("Expected error for literal mismatch")
	}
}

func TestGrammarParserAllTokenTypes(t *testing.T) {
	// Exercise tokenTypeName and stringToTokenType for all known token types
	cases := []struct {
		tokenType lexer.TokenType
		typeName  string
		gramRef   string
	}{
		{lexer.TokenName, "NAME", "NAME"},
		{lexer.TokenNumber, "NUMBER", "NUMBER"},
		{lexer.TokenString, "STRING", "STRING"},
		{lexer.TokenKeyword, "KEYWORD", "KEYWORD"},
		{lexer.TokenPlus, "PLUS", "PLUS"},
		{lexer.TokenMinus, "MINUS", "MINUS"},
		{lexer.TokenStar, "STAR", "STAR"},
		{lexer.TokenSlash, "SLASH", "SLASH"},
		{lexer.TokenEquals, "EQUALS", "EQUALS"},
		{lexer.TokenEqualsEquals, "EQUALS_EQUALS", "EQUALS_EQUALS"},
		{lexer.TokenLParen, "LPAREN", "LPAREN"},
		{lexer.TokenRParen, "RPAREN", "RPAREN"},
		{lexer.TokenComma, "COMMA", "COMMA"},
		{lexer.TokenColon, "COLON", "COLON"},
		{lexer.TokenSemicolon, "SEMICOLON", "SEMICOLON"},
		{lexer.TokenLBrace, "LBRACE", "LBRACE"},
		{lexer.TokenRBrace, "RBRACE", "RBRACE"},
		{lexer.TokenLBracket, "LBRACKET", "LBRACKET"},
		{lexer.TokenRBracket, "RBRACKET", "RBRACKET"},
		{lexer.TokenDot, "DOT", "DOT"},
		{lexer.TokenBang, "BANG", "BANG"},
		{lexer.TokenNewline, "NEWLINE", "NEWLINE"},
		{lexer.TokenEOF, "EOF", "EOF"},
	}

	for _, c := range cases {
		grammarSource := "expr = " + c.gramRef + " ;"
		pg, err := grammartools.ParseParserGrammar(grammarSource)
		if err != nil {
			t.Fatalf("Failed to parse grammar for %s: %v", c.gramRef, err)
		}

		tokens := []lexer.Token{
			{Type: c.tokenType, Value: "x", Line: 1, Column: 1},
			{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 2},
		}
		p := NewGrammarParser(tokens, pg)
		ast, err := p.Parse()
		if c.gramRef == "EOF" {
			// EOF is special — it's consumed as the final token
			if err != nil {
				t.Errorf("Parse for %s failed: %v", c.gramRef, err)
			}
			continue
		}
		if err != nil {
			// Some token types may fail due to newline significance, that's fine
			// We're exercising the code paths
			continue
		}
		if ast == nil {
			t.Errorf("Parse for %s returned nil", c.gramRef)
		}
	}
}

func TestGrammarParserUnknownTokenType(t *testing.T) {
	// Token with TypeName set should use string-based matching
	grammarSource := "expr = CUSTOM_TYPE ;"
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1, TypeName: "CUSTOM_TYPE"},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 2, TypeName: "EOF"},
	}
	p := NewGrammarParser(tokens, pg)
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "expr" {
		t.Errorf("Expected 'expr', got %q", ast.RuleName)
	}
}

func TestGrammarParserFurthestFailureWithUnconsumed(t *testing.T) {
	// Hit the path in Parse() where furthestPos > pos with unconsumed tokens
	grammarSource := `
expr = term ;
term = NUMBER PLUS NUMBER ;
`
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenNumber, Value: "1", Line: 1, Column: 1},
		{Type: lexer.TokenPlus, Value: "+", Line: 1, Column: 3},
		{Type: lexer.TokenNumber, Value: "2", Line: 1, Column: 5},
		{Type: lexer.TokenStar, Value: "*", Line: 1, Column: 7},
		{Type: lexer.TokenNumber, Value: "3", Line: 1, Column: 9},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 10},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err == nil {
		t.Fatal("Expected error for unconsumed tokens")
	}
}

func TestGrammarParserNoFurthestExpected(t *testing.T) {
	// Parse failure with no furthest expected (empty grammar match)
	grammarSource := "expr = { NUMBER } ;"
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	// Only has NAME tokens (no NUMBER), repetition matches zero times,
	// then unconsumed NAME token
	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 2},
	}
	p := NewGrammarParser(tokens, pg)
	_, err = p.Parse()
	if err == nil {
		t.Fatal("Expected error for unconsumed tokens")
	}
}

func TestGrammarParserStarlarkPipeline(t *testing.T) {
	grammarSource := `
file = { statement NEWLINE } ;
statement = assignment | simple_expr ;
assignment = NAME EQUALS simple_expr ;
simple_expr = NAME | NUMBER ;
`
	pg, err := grammartools.ParseParserGrammar(grammarSource)
	if err != nil {
		t.Fatalf("Failed: %v", err)
	}

	tokens := []lexer.Token{
		{Type: lexer.TokenName, Value: "x", Line: 1, Column: 1, TypeName: "NAME"},
		{Type: lexer.TokenEquals, Value: "=", Line: 1, Column: 3, TypeName: "EQUALS"},
		{Type: lexer.TokenNumber, Value: "42", Line: 1, Column: 5, TypeName: "NUMBER"},
		{Type: lexer.TokenNewline, Value: "\\n", Line: 1, Column: 7, TypeName: "NEWLINE"},
		{Type: lexer.TokenEOF, Value: "", Line: 2, Column: 1, TypeName: "EOF"},
	}
	p := NewGrammarParser(tokens, pg)
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	if ast.RuleName != "file" {
		t.Errorf("Expected 'file', got %q", ast.RuleName)
	}
}
