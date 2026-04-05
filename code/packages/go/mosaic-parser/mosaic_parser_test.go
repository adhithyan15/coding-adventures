// Tests for the Mosaic parser.
//
// The Mosaic grammar defines these top-level rules:
//   - file: { import_decl } component_decl
//   - component_decl: component Name { slots... node_tree }
//   - slot_decl: slot name: type [= default];
//   - node_element: Name { node_content... }
//   - node_content: property | child_node | slot_reference | when_block | each_block
package mosaicparser

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// helper: parse and fatalf on error
func mustParse(t *testing.T, source string) *parser.ASTNode {
	t.Helper()
	ast, err := Parse(source)
	if err != nil {
		t.Fatalf("Parse error: %v", err)
	}
	if ast == nil {
		t.Fatal("Parse returned nil AST")
	}
	return ast
}

// findRule performs a depth-first search for the first ASTNode with the given
// rule name within the tree rooted at node.
func findRule(node *parser.ASTNode, ruleName string) *parser.ASTNode {
	if node.RuleName == ruleName {
		return node
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			if found := findRule(childNode, ruleName); found != nil {
				return found
			}
		}
	}
	return nil
}

// findAllRules collects all ASTNodes with the given rule name in a depth-first
// traversal of the tree rooted at node.
func findAllRules(node *parser.ASTNode, ruleName string) []*parser.ASTNode {
	var result []*parser.ASTNode
	if node.RuleName == ruleName {
		result = append(result, node)
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			result = append(result, findAllRules(childNode, ruleName)...)
		}
	}
	return result
}

// tokenValue finds the first token child with the given TypeName and returns its Value.
func tokenValue(node *parser.ASTNode, typeName string) string {
	for _, child := range node.Children {
		if tok, ok := child.(interface{ TypeName() string }); ok {
			_ = tok
		}
	}
	// Use the lexer.Token type assertion
	for _, child := range node.Children {
		type tokenIface interface {
			GetTypeName() string
			GetValue() string
		}
		_ = child
	}
	return ""
}

// =============================================================================
// TestParseMinimalComponent
// =============================================================================
//
// The simplest valid Mosaic component: a name and an empty root node.
func TestParseMinimalComponent(t *testing.T) {
	source := `component Foo { Column {} }`
	ast := mustParse(t, source)
	if ast.RuleName != "file" {
		t.Errorf("Expected root rule 'file', got %q", ast.RuleName)
	}
}

// =============================================================================
// TestParseComponentDecl
// =============================================================================
//
// The AST should contain a component_decl node.
func TestParseComponentDecl(t *testing.T) {
	source := `component ProfileCard { Column {} }`
	ast := mustParse(t, source)
	compDecl := findRule(ast, "component_decl")
	if compDecl == nil {
		t.Fatal("Expected component_decl node in AST")
	}
}

// =============================================================================
// TestParseSlotDeclarations
// =============================================================================
//
// A component with multiple slot declarations should produce multiple
// slot_decl nodes in the AST.
func TestParseSlotDeclarations(t *testing.T) {
	source := `
component Card {
  slot title: text;
  slot count: number;
  slot active: bool;
  Text {}
}`
	ast := mustParse(t, source)
	slots := findAllRules(ast, "slot_decl")
	if len(slots) != 3 {
		t.Errorf("Expected 3 slot_decl nodes, got %d", len(slots))
	}
}

// =============================================================================
// TestParseSlotWithDefault
// =============================================================================
//
// A slot with a default value should have a default_value node.
func TestParseSlotWithDefault(t *testing.T) {
	source := `
component Counter {
  slot count: number = 0;
  Text {}
}`
	ast := mustParse(t, source)
	defaultVal := findRule(ast, "default_value")
	if defaultVal == nil {
		t.Fatal("Expected default_value node for slot with default")
	}
}

// =============================================================================
// TestParseListType
// =============================================================================
//
// A slot with list<text> type should produce a list_type node.
func TestParseListType(t *testing.T) {
	source := `
component List {
  slot items: list<text>;
  Column {}
}`
	ast := mustParse(t, source)
	listType := findRule(ast, "list_type")
	if listType == nil {
		t.Fatal("Expected list_type node in AST")
	}
}

// =============================================================================
// TestParseNodeElement
// =============================================================================
//
// The node tree should contain a node_element.
func TestParseNodeElement(t *testing.T) {
	source := `component Foo { Row {} }`
	ast := mustParse(t, source)
	elem := findRule(ast, "node_element")
	if elem == nil {
		t.Fatal("Expected node_element in AST")
	}
}

// =============================================================================
// TestParsePropertyAssignment
// =============================================================================
//
// Property assignments inside a node should produce property_assignment nodes.
func TestParsePropertyAssignment(t *testing.T) {
	source := `
component Card {
  Column {
    padding: 16dp;
    background: #2563eb;
  }
}`
	ast := mustParse(t, source)
	props := findAllRules(ast, "property_assignment")
	if len(props) != 2 {
		t.Errorf("Expected 2 property_assignment nodes, got %d", len(props))
	}
}

// =============================================================================
// TestParseSlotRef
// =============================================================================
//
// A slot reference in a property value should produce a slot_ref node.
func TestParseSlotRef(t *testing.T) {
	source := `
component Label {
  slot title: text;
  Text { content: @title; }
}`
	ast := mustParse(t, source)
	slotRef := findRule(ast, "slot_ref")
	if slotRef == nil {
		t.Fatal("Expected slot_ref node in AST")
	}
}

// =============================================================================
// TestParseChildNode
// =============================================================================
//
// Nested nodes should produce child_node entries.
func TestParseChildNode(t *testing.T) {
	source := `
component Card {
  Column {
    Text {}
    Text {}
  }
}`
	ast := mustParse(t, source)
	childNodes := findAllRules(ast, "child_node")
	if len(childNodes) != 2 {
		t.Errorf("Expected 2 child_node nodes, got %d", len(childNodes))
	}
}

// =============================================================================
// TestParseSlotReference
// =============================================================================
//
// A slot used as a child (not a property value) should be a slot_reference.
func TestParseSlotReference(t *testing.T) {
	source := `
component Card {
  slot action: node;
  Column {
    @action;
  }
}`
	ast := mustParse(t, source)
	slotRef := findRule(ast, "slot_reference")
	if slotRef == nil {
		t.Fatal("Expected slot_reference node in AST")
	}
}

// =============================================================================
// TestParseWhenBlock
// =============================================================================
//
// A when block should produce a when_block node.
func TestParseWhenBlock(t *testing.T) {
	source := `
component Conditional {
  slot show: bool;
  Column {
    when @show {
      Text {}
    }
  }
}`
	ast := mustParse(t, source)
	whenBlock := findRule(ast, "when_block")
	if whenBlock == nil {
		t.Fatal("Expected when_block node in AST")
	}
}

// =============================================================================
// TestParseEachBlock
// =============================================================================
//
// An each block should produce an each_block node.
func TestParseEachBlock(t *testing.T) {
	source := `
component ItemList {
  slot items: list<text>;
  Column {
    each @items as item {
      Text {}
    }
  }
}`
	ast := mustParse(t, source)
	eachBlock := findRule(ast, "each_block")
	if eachBlock == nil {
		t.Fatal("Expected each_block node in AST")
	}
}

// =============================================================================
// TestParseImportDecl
// =============================================================================
//
// An import declaration should produce an import_decl node.
func TestParseImportDecl(t *testing.T) {
	source := `
import Button from "./button.mosaic";
component Card {
  Column {}
}`
	ast := mustParse(t, source)
	importDecl := findRule(ast, "import_decl")
	if importDecl == nil {
		t.Fatal("Expected import_decl node in AST")
	}
}

// =============================================================================
// TestParseEnumValue
// =============================================================================
//
// An enum-style property value (align.center) should parse correctly.
func TestParseEnumValue(t *testing.T) {
	source := `
component Foo {
  Row {
    align: center;
    style: heading.medium;
  }
}`
	ast := mustParse(t, source)
	// enum_value is NAME DOT NAME — should parse as enum_value rule
	enumVal := findRule(ast, "enum_value")
	if enumVal == nil {
		t.Fatal("Expected enum_value node for 'heading.medium' property")
	}
}

// =============================================================================
// TestParseCreateParser
// =============================================================================
//
// CreateParser returns a non-nil GrammarParser that can be used separately.
func TestParseCreateParser(t *testing.T) {
	source := `component Foo { Column {} }`
	p, err := CreateParser(source)
	if err != nil {
		t.Fatalf("CreateParser error: %v", err)
	}
	if p == nil {
		t.Fatal("CreateParser returned nil")
	}
	ast, err := p.Parse()
	if err != nil {
		t.Fatalf("parser.Parse() error: %v", err)
	}
	if ast == nil {
		t.Fatal("parser.Parse() returned nil AST")
	}
}

// =============================================================================
// TestParseFullComponent
// =============================================================================
//
// A comprehensive component exercises all grammar rules in one pass.
func TestParseFullComponent(t *testing.T) {
	source := `
import Button from "./button.mosaic";

component ProfileCard {
  slot avatar-url: image;
  slot display-name: text;
  slot bio: text;
  slot follower-count: number = 0;
  slot items: list<text>;
  slot show-bio: bool = true;
  slot action: Button;

  Column {
    padding: 16dp;
    background: #ffffff;
    Text { content: @display-name; font-size: 18sp; }
    when @show-bio {
      Text { content: @bio; color: #666666; }
    }
    each @items as item {
      Text { content: @item; }
    }
    @action;
  }
}`
	ast := mustParse(t, source)

	// Check we have the expected structure
	if findRule(ast, "import_decl") == nil {
		t.Error("Expected import_decl")
	}
	if findRule(ast, "component_decl") == nil {
		t.Error("Expected component_decl")
	}
	slots := findAllRules(ast, "slot_decl")
	if len(slots) != 7 {
		t.Errorf("Expected 7 slot_decl nodes, got %d", len(slots))
	}
	if findRule(ast, "when_block") == nil {
		t.Error("Expected when_block")
	}
	if findRule(ast, "each_block") == nil {
		t.Error("Expected each_block")
	}
	if findRule(ast, "slot_reference") == nil {
		t.Error("Expected slot_reference")
	}
}
