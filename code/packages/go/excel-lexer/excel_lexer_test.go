package excellexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func TestTokenizeExcelFormula(t *testing.T) {
	tokens, err := TokenizeExcelFormula("=SUM(A1)")
	if err != nil {
		t.Fatalf("Failed to tokenize Excel formula: %v", err)
	}

	if tokens[1].TypeName != "FUNCTION_NAME" || tokens[1].Value != "SUM" {
		t.Fatalf("Expected FUNCTION_NAME SUM, got %s %q", tokens[1].TypeName, tokens[1].Value)
	}
}

func TestTokenizeStructuredReferenceTableName(t *testing.T) {
	tokens, err := TokenizeExcelFormula("DeptSales[Sales Amount]")
	if err != nil {
		t.Fatalf("Failed to tokenize structured reference: %v", err)
	}

	if tokens[0].TypeName != "TABLE_NAME" || tokens[0].Value != "DeptSales" {
		t.Fatalf("Expected TABLE_NAME DeptSales, got %s %q", tokens[0].TypeName, tokens[0].Value)
	}
}

func TestTokenizeKeepsCellReferences(t *testing.T) {
	tokens, err := TokenizeExcelFormula("A1 + 1")
	if err != nil {
		t.Fatalf("Failed to tokenize basic formula: %v", err)
	}

	if tokens[0].Type != lexer.TokenName || tokens[0].TypeName != "CELL" {
		t.Fatalf("Expected CELL token, got %v %q", tokens[0].Type, tokens[0].TypeName)
	}
}
