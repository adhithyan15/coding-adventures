package excelparser

import (
	"testing"
)

func TestParseExcelFormula(t *testing.T) {
	formula, err := ParseExcelFormula("=SUM(A1:B2)")
	if err != nil {
		t.Fatalf("Failed to parse Excel formula: %v", err)
	}

	if formula.RuleName != "formula" {
		t.Fatalf("Expected formula rule at root, got %s", formula.RuleName)
	}
}

func TestParseColumnRange(t *testing.T) {
	formula, err := ParseExcelFormula("A:C")
	if err != nil {
		t.Fatalf("Failed to parse column range: %v", err)
	}

	if formula.RuleName != "formula" {
		t.Fatalf("Expected formula rule at root, got %s", formula.RuleName)
	}
}

func TestParseRowRange(t *testing.T) {
	formula, err := ParseExcelFormula("1:3")
	if err != nil {
		t.Fatalf("Failed to parse row range: %v", err)
	}

	if formula.RuleName != "formula" {
		t.Fatalf("Expected formula rule at root, got %s", formula.RuleName)
	}
}
