package cssparser

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

func findNodes(node *parser.ASTNode, ruleName string) []*parser.ASTNode {
	var results []*parser.ASTNode
	if node.RuleName == ruleName {
		results = append(results, node)
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			results = append(results, findNodes(childNode, ruleName)...)
		}
	}
	return results
}

func findTokens(node *parser.ASTNode, tokenType string) []lexer.Token {
	var results []lexer.Token
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			results = append(results, findTokens(value, tokenType)...)
		case lexer.Token:
			if tokenType == "" || value.TypeName == tokenType {
				results = append(results, value)
			}
		}
	}
	return results
}

func tokenValues(tokens []lexer.Token) []string {
	values := make([]string, 0, len(tokens))
	for _, token := range tokens {
		values = append(values, token.Value)
	}
	return values
}

func contains(values []string, want string) bool {
	for _, value := range values {
		if value == want {
			return true
		}
	}
	return false
}

func TestCreateCSSParser(t *testing.T) {
	cssParser, err := CreateCSSParser("h1 { color: red; }")
	if err != nil {
		t.Fatalf("CreateCSSParser returned error: %v", err)
	}

	ast, err := cssParser.Parse()
	if err != nil {
		t.Fatalf("Parse returned error: %v", err)
	}
	if ast.RuleName != "stylesheet" {
		t.Fatalf("expected stylesheet root, got %q", ast.RuleName)
	}
}

func TestEmptyStylesheets(t *testing.T) {
	for _, source := range []string{"", "  \n\t "} {
		ast, err := ParseCSS(source)
		if err != nil {
			t.Fatalf("ParseCSS(%q) returned error: %v", source, err)
		}
		if ast.RuleName != "stylesheet" || len(ast.Children) != 0 {
			t.Fatalf("expected empty stylesheet, got %#v", ast)
		}
	}
}

func TestQualifiedRuleWithDeclarations(t *testing.T) {
	ast, err := ParseCSS("h1 { color: red; margin: 0; }")
	if err != nil {
		t.Fatalf("ParseCSS returned error: %v", err)
	}

	if got := len(findNodes(ast, "qualified_rule")); got != 1 {
		t.Fatalf("expected one qualified rule, got %d", got)
	}
	if got := len(findNodes(ast, "declaration")); got != 2 {
		t.Fatalf("expected two declarations, got %d", got)
	}
	if !contains(tokenValues(findTokens(ast, "IDENT")), "color") {
		t.Fatalf("expected IDENT token for color")
	}
}

func TestSelectors(t *testing.T) {
	ast, err := ParseCSS("h1, .active, #main { display: block; }")
	if err != nil {
		t.Fatalf("ParseCSS returned error: %v", err)
	}

	if got := len(findNodes(ast, "complex_selector")); got != 3 {
		t.Fatalf("expected three complex selectors, got %d", got)
	}
	if got := len(findNodes(ast, "class_selector")); got != 1 {
		t.Fatalf("expected one class selector, got %d", got)
	}
	if got := len(findNodes(ast, "id_selector")); got != 1 {
		t.Fatalf("expected one id selector, got %d", got)
	}
}

func TestAdvancedSelectors(t *testing.T) {
	ast, err := ParseCSS(`nav > a[href^="https"]:hover::before { content: "go"; }`)
	if err != nil {
		t.Fatalf("ParseCSS returned error: %v", err)
	}

	checks := map[string]int{
		"combinator":         1,
		"attribute_selector": 1,
		"pseudo_class":       1,
		"pseudo_element":     1,
	}
	for ruleName, want := range checks {
		if got := len(findNodes(ast, ruleName)); got != want {
			t.Fatalf("expected %d %s nodes, got %d", want, ruleName, got)
		}
	}
}

func TestAtRulesAndNestedRules(t *testing.T) {
	ast, err := ParseCSS("@media screen { .parent { color: red; & .child { color: blue; } } }")
	if err != nil {
		t.Fatalf("ParseCSS returned error: %v", err)
	}

	if got := len(findNodes(ast, "at_rule")); got != 1 {
		t.Fatalf("expected one at-rule, got %d", got)
	}
	if got := len(findNodes(ast, "qualified_rule")); got < 2 {
		t.Fatalf("expected nested qualified rules, got %d", got)
	}
}

func TestFunctionsAndPriority(t *testing.T) {
	ast, err := ParseCSS(":root { --gap: 12px; width: calc(100% - var(--gap)); color: red !important; }")
	if err != nil {
		t.Fatalf("ParseCSS returned error: %v", err)
	}

	if got := len(findNodes(ast, "function_call")); got != 1 {
		t.Fatalf("expected one top-level function call, got %d", got)
	}
	if got := len(findNodes(ast, "priority")); got != 1 {
		t.Fatalf("expected one priority node, got %d", got)
	}
	if got := len(findTokens(ast, "FUNCTION")); got != 2 {
		t.Fatalf("expected calc() and var() function tokens, got %d", got)
	}
	if !contains(tokenValues(findTokens(ast, "CUSTOM_PROPERTY")), "--gap") {
		t.Fatalf("expected custom property token")
	}
}

func TestInvalidCSS(t *testing.T) {
	_, err := ParseCSS("h1 { color: red;")
	if err == nil {
		t.Fatal("expected invalid CSS to fail")
	}
	if !strings.Contains(err.Error(), "Parse error") {
		t.Fatalf("expected grammar parse error, got %v", err)
	}
}
