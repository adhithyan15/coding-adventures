package parser

import (
	"reflect"
	"testing"

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
