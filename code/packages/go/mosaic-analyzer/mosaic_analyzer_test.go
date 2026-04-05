// Tests for the Mosaic analyzer.
//
// The analyzer converts a parsed Mosaic AST into a typed MosaicIR,
// resolving types, normalizing values, and identifying primitives.
package mosaicanalyzer

import (
	"testing"
)

// mustAnalyze is a helper that calls Analyze and fatalf on error.
func mustAnalyze(t *testing.T, source string) *MosaicIR {
	t.Helper()
	ir, err := Analyze(source)
	if err != nil {
		t.Fatalf("Analyze error: %v", err)
	}
	if ir == nil {
		t.Fatal("Analyze returned nil")
	}
	return ir
}

// =============================================================================
// TestAnalyzeComponentName
// =============================================================================
//
// The component name should be extracted correctly from the component_decl.
func TestAnalyzeComponentName(t *testing.T) {
	ir := mustAnalyze(t, `component ProfileCard { Column {} }`)
	if ir.Component.Name != "ProfileCard" {
		t.Errorf("Expected name 'ProfileCard', got %q", ir.Component.Name)
	}
}

// =============================================================================
// TestAnalyzeTextSlot
// =============================================================================
//
// A slot with type 'text' should produce Kind="text" in the IR.
func TestAnalyzeTextSlot(t *testing.T) {
	ir := mustAnalyze(t, `
component Label {
  slot title: text;
  Text {}
}`)
	if len(ir.Component.Slots) != 1 {
		t.Fatalf("Expected 1 slot, got %d", len(ir.Component.Slots))
	}
	slot := ir.Component.Slots[0]
	if slot.Name != "title" {
		t.Errorf("Expected slot name 'title', got %q", slot.Name)
	}
	if slot.Type.Kind != "text" {
		t.Errorf("Expected type kind 'text', got %q", slot.Type.Kind)
	}
	if !slot.Required {
		t.Error("Expected slot to be required (no default)")
	}
}

// =============================================================================
// TestAnalyzeAllPrimitiveTypes
// =============================================================================
//
// Each primitive type keyword should map to the correct MosaicType.
func TestAnalyzeAllPrimitiveTypes(t *testing.T) {
	cases := []struct {
		typeName string
		expected string
	}{
		{"text", "text"},
		{"number", "number"},
		{"bool", "bool"},
		{"image", "image"},
		{"color", "color"},
		{"node", "node"},
	}
	for _, tc := range cases {
		source := `component Foo { slot s: ` + tc.typeName + `; Column {} }`
		ir := mustAnalyze(t, source)
		if len(ir.Component.Slots) != 1 {
			t.Errorf("Type %q: expected 1 slot, got %d", tc.typeName, len(ir.Component.Slots))
			continue
		}
		if ir.Component.Slots[0].Type.Kind != tc.expected {
			t.Errorf("Type %q: expected kind %q, got %q", tc.typeName, tc.expected, ir.Component.Slots[0].Type.Kind)
		}
	}
}

// =============================================================================
// TestAnalyzeListType
// =============================================================================
//
// A slot with list<text> type should produce Kind="list" with Element.Kind="text".
func TestAnalyzeListType(t *testing.T) {
	ir := mustAnalyze(t, `
component List {
  slot items: list<text>;
  Column {}
}`)
	if len(ir.Component.Slots) != 1 {
		t.Fatalf("Expected 1 slot, got %d", len(ir.Component.Slots))
	}
	slot := ir.Component.Slots[0]
	if slot.Type.Kind != "list" {
		t.Errorf("Expected kind 'list', got %q", slot.Type.Kind)
	}
	if slot.Type.Element == nil {
		t.Fatal("Expected Element to be set for list type")
	}
	if slot.Type.Element.Kind != "text" {
		t.Errorf("Expected element kind 'text', got %q", slot.Type.Element.Kind)
	}
}

// =============================================================================
// TestAnalyzeSlotWithDefault
// =============================================================================
//
// A slot with a default value should have Required=false and DefaultValue set.
func TestAnalyzeSlotWithDefault(t *testing.T) {
	ir := mustAnalyze(t, `
component Counter {
  slot count: number = 0;
  Text {}
}`)
	slot := ir.Component.Slots[0]
	if slot.Required {
		t.Error("Expected slot to be optional (has default)")
	}
	if slot.DefaultValue == nil {
		t.Fatal("Expected DefaultValue to be set")
	}
	if slot.DefaultValue.Kind != "number" {
		t.Errorf("Expected default kind 'number', got %q", slot.DefaultValue.Kind)
	}
	if slot.DefaultValue.NumValue != 0 {
		t.Errorf("Expected default value 0, got %v", slot.DefaultValue.NumValue)
	}
}

// =============================================================================
// TestAnalyzeBoolDefault
// =============================================================================
//
// A slot with a bool default value.
func TestAnalyzeBoolDefault(t *testing.T) {
	ir := mustAnalyze(t, `
component Conditional {
  slot visible: bool = true;
  Column {}
}`)
	slot := ir.Component.Slots[0]
	if slot.DefaultValue == nil {
		t.Fatal("Expected DefaultValue")
	}
	if slot.DefaultValue.Kind != "bool" {
		t.Errorf("Expected 'bool', got %q", slot.DefaultValue.Kind)
	}
	if !slot.DefaultValue.BoolValue {
		t.Error("Expected default BoolValue=true")
	}
}

// =============================================================================
// TestAnalyzePrimitiveNode
// =============================================================================
//
// Built-in layout nodes (Row, Column, Text, etc.) should have IsPrimitive=true.
func TestAnalyzePrimitiveNode(t *testing.T) {
	primitives := []string{"Row", "Column", "Box", "Stack", "Text", "Image", "Icon", "Spacer", "Divider", "Scroll"}
	for _, tag := range primitives {
		source := `component Foo { ` + tag + ` {} }`
		ir := mustAnalyze(t, source)
		if !ir.Component.Tree.IsPrimitive {
			t.Errorf("Tag %q: expected IsPrimitive=true", tag)
		}
	}
}

// =============================================================================
// TestAnalyzeNonPrimitiveNode
// =============================================================================
//
// Custom/imported component nodes should have IsPrimitive=false.
func TestAnalyzeNonPrimitiveNode(t *testing.T) {
	ir := mustAnalyze(t, `component Card { Button {} }`)
	if ir.Component.Tree.IsPrimitive {
		t.Error("Expected IsPrimitive=false for imported component 'Button'")
	}
	if ir.Component.Tree.Tag != "Button" {
		t.Errorf("Expected tag 'Button', got %q", ir.Component.Tree.Tag)
	}
}

// =============================================================================
// TestAnalyzePropertyDimension
// =============================================================================
//
// A dimension property like `padding: 16dp;` should produce Kind="dimension".
func TestAnalyzePropertyDimension(t *testing.T) {
	ir := mustAnalyze(t, `
component Foo {
  Column {
    padding: 16dp;
  }
}`)
	props := ir.Component.Tree.Properties
	if len(props) != 1 {
		t.Fatalf("Expected 1 property, got %d", len(props))
	}
	prop := props[0]
	if prop.Name != "padding" {
		t.Errorf("Expected 'padding', got %q", prop.Name)
	}
	if prop.Value.Kind != "dimension" {
		t.Errorf("Expected 'dimension', got %q", prop.Value.Kind)
	}
	if prop.Value.NumValue != 16 {
		t.Errorf("Expected NumValue=16, got %v", prop.Value.NumValue)
	}
	if prop.Value.Unit != "dp" {
		t.Errorf("Expected Unit='dp', got %q", prop.Value.Unit)
	}
}

// =============================================================================
// TestAnalyzePropertyColorHex
// =============================================================================
//
// A color property like `background: #2563eb;` should produce Kind="color_hex".
func TestAnalyzePropertyColorHex(t *testing.T) {
	ir := mustAnalyze(t, `
component Foo {
  Column {
    background: #2563eb;
  }
}`)
	props := ir.Component.Tree.Properties
	if len(props) != 1 {
		t.Fatalf("Expected 1 property, got %d", len(props))
	}
	if props[0].Value.Kind != "color_hex" {
		t.Errorf("Expected 'color_hex', got %q", props[0].Value.Kind)
	}
	if props[0].Value.StrValue != "#2563eb" {
		t.Errorf("Expected '#2563eb', got %q", props[0].Value.StrValue)
	}
}

// =============================================================================
// TestAnalyzePropertySlotRef
// =============================================================================
//
// A slot reference in a property value like `content: @title;`.
func TestAnalyzePropertySlotRef(t *testing.T) {
	ir := mustAnalyze(t, `
component Label {
  slot title: text;
  Text { content: @title; }
}`)
	props := ir.Component.Tree.Properties
	if len(props) != 1 {
		t.Fatalf("Expected 1 property, got %d", len(props))
	}
	if props[0].Value.Kind != "slot_ref" {
		t.Errorf("Expected 'slot_ref', got %q", props[0].Value.Kind)
	}
	if props[0].Value.SlotName != "title" {
		t.Errorf("Expected SlotName='title', got %q", props[0].Value.SlotName)
	}
}

// =============================================================================
// TestAnalyzeChildNodes
// =============================================================================
//
// Child nodes in the tree should produce MosaicChild with Kind="node".
func TestAnalyzeChildNodes(t *testing.T) {
	ir := mustAnalyze(t, `
component Card {
  Column {
    Text {}
    Image {}
  }
}`)
	children := ir.Component.Tree.Children
	if len(children) != 2 {
		t.Fatalf("Expected 2 children, got %d", len(children))
	}
	if children[0].Kind != "node" {
		t.Errorf("Expected Kind='node', got %q", children[0].Kind)
	}
	if children[0].Node.Tag != "Text" {
		t.Errorf("Expected tag 'Text', got %q", children[0].Node.Tag)
	}
}

// =============================================================================
// TestAnalyzeSlotRefChild
// =============================================================================
//
// A slot reference used as a child `@action;` produces Kind="slot_ref".
func TestAnalyzeSlotRefChild(t *testing.T) {
	ir := mustAnalyze(t, `
component Card {
  slot action: node;
  Column {
    @action;
  }
}`)
	children := ir.Component.Tree.Children
	if len(children) != 1 {
		t.Fatalf("Expected 1 child, got %d", len(children))
	}
	if children[0].Kind != "slot_ref" {
		t.Errorf("Expected Kind='slot_ref', got %q", children[0].Kind)
	}
	if children[0].SlotName != "action" {
		t.Errorf("Expected SlotName='action', got %q", children[0].SlotName)
	}
}

// =============================================================================
// TestAnalyzeWhenBlock
// =============================================================================
//
// A when block should produce Kind="when" with the correct slot name and body.
func TestAnalyzeWhenBlock(t *testing.T) {
	ir := mustAnalyze(t, `
component Conditional {
  slot show: bool;
  Column {
    when @show {
      Text {}
    }
  }
}`)
	children := ir.Component.Tree.Children
	if len(children) != 1 {
		t.Fatalf("Expected 1 child (when block), got %d", len(children))
	}
	when := children[0]
	if when.Kind != "when" {
		t.Errorf("Expected Kind='when', got %q", when.Kind)
	}
	if when.SlotName != "show" {
		t.Errorf("Expected SlotName='show', got %q", when.SlotName)
	}
	if len(when.Body) != 1 {
		t.Errorf("Expected 1 node in when body, got %d", len(when.Body))
	}
}

// =============================================================================
// TestAnalyzeEachBlock
// =============================================================================
//
// An each block should produce Kind="each" with slot name, item name, and body.
func TestAnalyzeEachBlock(t *testing.T) {
	ir := mustAnalyze(t, `
component List {
  slot items: list<text>;
  Column {
    each @items as item {
      Text {}
    }
  }
}`)
	children := ir.Component.Tree.Children
	if len(children) != 1 {
		t.Fatalf("Expected 1 child (each block), got %d", len(children))
	}
	each := children[0]
	if each.Kind != "each" {
		t.Errorf("Expected Kind='each', got %q", each.Kind)
	}
	if each.SlotName != "items" {
		t.Errorf("Expected SlotName='items', got %q", each.SlotName)
	}
	if each.ItemName != "item" {
		t.Errorf("Expected ItemName='item', got %q", each.ItemName)
	}
}

// =============================================================================
// TestAnalyzeImport
// =============================================================================
//
// Import declarations should appear in the IR's Imports slice.
func TestAnalyzeImport(t *testing.T) {
	ir := mustAnalyze(t, `
import Button from "./button.mosaic";
component Card {
  Column {}
}`)
	if len(ir.Imports) != 1 {
		t.Fatalf("Expected 1 import, got %d", len(ir.Imports))
	}
	imp := ir.Imports[0]
	if imp.Name != "Button" {
		t.Errorf("Expected Name='Button', got %q", imp.Name)
	}
	if imp.Path != "./button.mosaic" {
		t.Errorf("Expected Path='./button.mosaic', got %q", imp.Path)
	}
}

// =============================================================================
// TestAnalyzeMultipleSlots
// =============================================================================
//
// A component with multiple slots should produce them all in order.
func TestAnalyzeMultipleSlots(t *testing.T) {
	ir := mustAnalyze(t, `
component Card {
  slot title: text;
  slot count: number = 0;
  slot visible: bool = true;
  Column {}
}`)
	if len(ir.Component.Slots) != 3 {
		t.Fatalf("Expected 3 slots, got %d", len(ir.Component.Slots))
	}
	if ir.Component.Slots[0].Name != "title" {
		t.Errorf("Slot 0: expected 'title', got %q", ir.Component.Slots[0].Name)
	}
	if ir.Component.Slots[1].Name != "count" {
		t.Errorf("Slot 1: expected 'count', got %q", ir.Component.Slots[1].Name)
	}
	if ir.Component.Slots[2].Name != "visible" {
		t.Errorf("Slot 2: expected 'visible', got %q", ir.Component.Slots[2].Name)
	}
}
