// Tests for the Mosaic React emitter.
//
// The ReactRenderer produces TypeScript React functional components (.tsx) from
// a Mosaic IR. These tests verify that the generated code contains the expected
// elements: React import, props interface, function component, JSX tree.
package mosaicemitreact

import (
	"strings"
	"testing"

	mosaicanalyzer "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer"
	mosaicvm "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-vm"
)

// emitSource analyzes and compiles Mosaic source to React TSX.
func emitSource(t *testing.T, source string) string {
	t.Helper()
	ir, err := mosaicanalyzer.Analyze(source)
	if err != nil {
		t.Fatalf("Analyze error: %v", err)
	}
	vm := mosaicvm.New(ir)
	renderer := NewReactRenderer()
	result, err := vm.Run(renderer)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}
	return result.Code
}

// =============================================================================
// TestReactEmitReactImport
// =============================================================================
//
// The generated file must import React.
func TestReactEmitReactImport(t *testing.T) {
	code := emitSource(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "import React from 'react'") {
		t.Errorf("Expected React import, got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitPropsInterface
// =============================================================================
//
// The generated file must include a TypeScript props interface.
func TestReactEmitPropsInterface(t *testing.T) {
	code := emitSource(t, `
component Label {
  slot title: text;
  Text { content: @title; }
}`)
	if !strings.Contains(code, "interface LabelProps") {
		t.Errorf("Expected LabelProps interface, got:\n%s", code)
	}
	if !strings.Contains(code, "title: string") {
		t.Errorf("Expected 'title: string' in props interface, got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitFunctionComponent
// =============================================================================
//
// The generated file must include an exported function component.
func TestReactEmitFunctionComponent(t *testing.T) {
	code := emitSource(t, `component Button { Column {} }`)
	if !strings.Contains(code, "export function Button") {
		t.Errorf("Expected 'export function Button', got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitFileName
// =============================================================================
//
// The EmitResult.FileName should be the kebab-case component name with .tsx extension.
func TestReactEmitFileName(t *testing.T) {
	ir, _ := mosaicanalyzer.Analyze(`component ProfileCard { Column {} }`)
	vm := mosaicvm.New(ir)
	renderer := NewReactRenderer()
	result, _ := vm.Run(renderer)

	if result.FileName != "profile-card.tsx" {
		t.Errorf("Expected 'profile-card.tsx', got %q", result.FileName)
	}
}

// =============================================================================
// TestReactEmitColumnDiv
// =============================================================================
//
// Column nodes should map to <div> elements.
func TestReactEmitColumnDiv(t *testing.T) {
	code := emitSource(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "<div>") && !strings.Contains(code, "<div ") {
		t.Errorf("Expected '<div>' for Column, got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitTextSpan
// =============================================================================
//
// Text nodes should map to <span> elements.
func TestReactEmitTextSpan(t *testing.T) {
	code := emitSource(t, `
component Label {
  slot title: text;
  Column {
    Text { content: @title; }
  }
}`)
	if !strings.Contains(code, "<span>") && !strings.Contains(code, "<span ") {
		t.Errorf("Expected '<span>' for Text, got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitSlotProp
// =============================================================================
//
// A required text slot should appear in the props interface.
func TestReactEmitSlotProp(t *testing.T) {
	code := emitSource(t, `
component Card {
  slot title: text;
  slot count: number = 0;
  Column {}
}`)
	if !strings.Contains(code, "title: string") {
		t.Errorf("Expected 'title: string' in props, got:\n%s", code)
	}
	// count has a default so it should be optional (count?)
	if !strings.Contains(code, "count?") {
		t.Errorf("Expected optional 'count?' in props, got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitListProp
// =============================================================================
//
// A list<text> slot should become string[] in the props interface.
func TestReactEmitListProp(t *testing.T) {
	code := emitSource(t, `
component List {
  slot items: list<text>;
  Column {}
}`)
	if !strings.Contains(code, "string[]") {
		t.Errorf("Expected 'string[]' for list<text>, got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitAutoGenHeader
// =============================================================================
//
// The generated file should include an auto-generated warning header.
func TestReactEmitAutoGenHeader(t *testing.T) {
	code := emitSource(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "Auto-generated") && !strings.Contains(code, "DO NOT EDIT") {
		t.Errorf("Expected auto-generated header, got:\n%s", code)
	}
}

// =============================================================================
// TestReactEmitToCamelCase
// =============================================================================
//
// The toCamelCase helper should convert kebab-case names correctly.
func TestReactEmitToCamelCase(t *testing.T) {
	cases := []struct {
		input    string
		expected string
	}{
		{"title", "title"},
		{"avatar-url", "avatarUrl"},
		{"display-name", "displayName"},
		{"a11y-label", "a11yLabel"},
		{"first-second-third", "firstSecondThird"},
	}
	for _, tc := range cases {
		got := toCamelCase(tc.input)
		if got != tc.expected {
			t.Errorf("toCamelCase(%q) = %q, want %q", tc.input, got, tc.expected)
		}
	}
}

// =============================================================================
// TestReactEmitToKebabCase
// =============================================================================
//
// The toKebabCase helper should convert PascalCase names correctly.
func TestReactEmitToKebabCase(t *testing.T) {
	cases := []struct {
		input    string
		expected string
	}{
		{"Button", "button"},
		{"ProfileCard", "profile-card"},
		{"HowItWorks", "how-it-works"},
		{"MyComponent", "my-component"},
	}
	for _, tc := range cases {
		got := toKebabCase(tc.input)
		if got != tc.expected {
			t.Errorf("toKebabCase(%q) = %q, want %q", tc.input, got, tc.expected)
		}
	}
}
