package haskellparser

import "testing"

func TestParseHaskellDefaultVersion(t *testing.T) {
	ast, err := ParseHaskell("x", "")
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if ast.RuleName != "file" {
		t.Fatalf("expected file root, got %s", ast.RuleName)
	}
}

func TestParseHaskellExplicitLayout(t *testing.T) {
	parser, err := CreateHaskellParser("let { x = y } in x", "2010")
	if err != nil {
		t.Fatalf("create parser failed: %v", err)
	}
	ast, err := parser.Parse()
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if ast.RuleName != "file" {
		t.Fatalf("expected file root, got %s", ast.RuleName)
	}
}

func TestParseHaskellVersions(t *testing.T) {
	for _, version := range ValidVersions() {
		t.Run(version, func(t *testing.T) {
			ast, err := ParseHaskell("x", version)
			if err != nil {
				t.Fatalf("parse %s failed: %v", version, err)
			}
			if ast.RuleName != "file" {
				t.Fatalf("expected file root for %s, got %s", version, ast.RuleName)
			}
		})
	}
}

func TestCreateHaskellParser(t *testing.T) {
	parser, err := CreateHaskellParser("x", "98")
	if err != nil {
		t.Fatalf("create parser failed: %v", err)
	}
	if parser == nil {
		t.Fatal("expected non-nil parser")
	}
}

func TestParseHaskellUnknownVersion(t *testing.T) {
	_, err := ParseHaskell("x", "2020")
	if err == nil {
		t.Fatal("expected error for unknown version")
	}
}
