package lispparser

import (
	"path/filepath"
	"testing"
)

func TestParseLispDefinition(t *testing.T) {
	ast, err := ParseLisp("(define x 42)")
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if ast.RuleName != "program" || len(ast.Children) == 0 {
		t.Fatalf("unexpected AST root: %#v", ast)
	}
}

func TestParseLispQuotedForm(t *testing.T) {
	ast, err := ParseLisp("'(a b c)")
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if ast.RuleName != "program" {
		t.Fatalf("expected program root, got %s", ast.RuleName)
	}
}

func TestCreateLispParserDottedPair(t *testing.T) {
	lispParser, err := CreateLispParser("(a . b)")
	if err != nil {
		t.Fatalf("create parser failed: %v", err)
	}
	ast, err := lispParser.Parse()
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if ast.RuleName != "program" {
		t.Fatalf("expected program root, got %s", ast.RuleName)
	}
}

func TestParseLispMalformedList(t *testing.T) {
	_, err := ParseLisp("(a b")
	if err == nil {
		t.Fatal("expected parse error")
	}
}

func TestGrammarPath(t *testing.T) {
	path := getGrammarPath()
	if filepath.Base(path) != "lisp.grammar" {
		t.Fatalf("expected lisp.grammar path, got %s", path)
	}
}
