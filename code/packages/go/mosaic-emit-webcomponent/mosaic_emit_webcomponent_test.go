// Tests for the Mosaic Web Component emitter.
//
// The WebComponentRenderer produces TypeScript Custom Element classes (.ts) from
// a Mosaic IR. These tests verify that the generated code contains the expected
// elements: class declaration, shadow DOM, slot setters, _render method.
package mosaicemitwebcomponent

import (
	"strings"
	"testing"

	mosaicanalyzer "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer"
	mosaicvm "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-vm"
)

// emitWC analyzes and compiles Mosaic source to Web Component TypeScript.
func emitWC(t *testing.T, source string) string {
	t.Helper()
	ir, err := mosaicanalyzer.Analyze(source)
	if err != nil {
		t.Fatalf("Analyze error: %v", err)
	}
	vm := mosaicvm.New(ir)
	renderer := NewWebComponentRenderer()
	result, err := vm.Run(renderer)
	if err != nil {
		t.Fatalf("Run error: %v", err)
	}
	return result.Code
}

// =============================================================================
// TestWCExtendsHTMLElement
// =============================================================================
//
// The generated class must extend HTMLElement.
func TestWCExtendsHTMLElement(t *testing.T) {
	code := emitWC(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "extends HTMLElement") {
		t.Errorf("Expected 'extends HTMLElement', got:\n%s", code)
	}
}

// =============================================================================
// TestWCConnectedCallback
// =============================================================================
//
// The generated class must have a connectedCallback with shadow DOM setup.
func TestWCConnectedCallback(t *testing.T) {
	code := emitWC(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "connectedCallback") {
		t.Errorf("Expected 'connectedCallback', got:\n%s", code)
	}
	if !strings.Contains(code, "attachShadow") {
		t.Errorf("Expected 'attachShadow', got:\n%s", code)
	}
}

// =============================================================================
// TestWCClassNameFromComponent
// =============================================================================
//
// The class name should match the component name.
func TestWCClassNameFromComponent(t *testing.T) {
	code := emitWC(t, `component ProfileCard { Column {} }`)
	if !strings.Contains(code, "class ProfileCard") {
		t.Errorf("Expected 'class ProfileCard', got:\n%s", code)
	}
}

// =============================================================================
// TestWCCustomElementRegistration
// =============================================================================
//
// The file must end with customElements.define(...) registration.
func TestWCCustomElementRegistration(t *testing.T) {
	code := emitWC(t, `component ProfileCard { Column {} }`)
	if !strings.Contains(code, "customElements.define") {
		t.Errorf("Expected 'customElements.define', got:\n%s", code)
	}
	// Tag name: mosaic-profile-card
	if !strings.Contains(code, "mosaic-profile-card") {
		t.Errorf("Expected 'mosaic-profile-card' tag name, got:\n%s", code)
	}
}

// =============================================================================
// TestWCFileName
// =============================================================================
//
// The EmitResult.FileName should be the mosaic-kebab-case name with .ts extension.
func TestWCFileName(t *testing.T) {
	ir, _ := mosaicanalyzer.Analyze(`component ProfileCard { Column {} }`)
	vm := mosaicvm.New(ir)
	renderer := NewWebComponentRenderer()
	result, _ := vm.Run(renderer)

	if result.FileName != "mosaic-profile-card.ts" {
		t.Errorf("Expected 'mosaic-profile-card.ts', got %q", result.FileName)
	}
}

// =============================================================================
// TestWCSlotSetter
// =============================================================================
//
// Each slot should generate a property setter in the class.
func TestWCSlotSetter(t *testing.T) {
	code := emitWC(t, `
component Label {
  slot title: text;
  Text {}
}`)
	if !strings.Contains(code, "set title") {
		t.Errorf("Expected 'set title' setter, got:\n%s", code)
	}
}

// =============================================================================
// TestWCPrivateField
// =============================================================================
//
// Each slot should generate a private field declaration.
func TestWCPrivateField(t *testing.T) {
	code := emitWC(t, `
component Foo {
  slot count: number = 0;
  Column {}
}`)
	if !strings.Contains(code, "_count") {
		t.Errorf("Expected '_count' private field, got:\n%s", code)
	}
}

// =============================================================================
// TestWCRenderMethod
// =============================================================================
//
// The class must have a _render() method that builds innerHTML.
func TestWCRenderMethod(t *testing.T) {
	code := emitWC(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "_render()") {
		t.Errorf("Expected '_render()' method, got:\n%s", code)
	}
	if !strings.Contains(code, "innerHTML") {
		t.Errorf("Expected 'innerHTML' assignment, got:\n%s", code)
	}
}

// =============================================================================
// TestWCEscapeHtmlHelper
// =============================================================================
//
// The class must include a _escapeHtml helper for XSS prevention.
func TestWCEscapeHtmlHelper(t *testing.T) {
	code := emitWC(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "_escapeHtml") {
		t.Errorf("Expected '_escapeHtml' helper, got:\n%s", code)
	}
}

// =============================================================================
// TestWCAutoGenHeader
// =============================================================================
//
// The generated file should include an auto-generated warning header.
func TestWCAutoGenHeader(t *testing.T) {
	code := emitWC(t, `component Foo { Column {} }`)
	if !strings.Contains(code, "Auto-generated") && !strings.Contains(code, "DO NOT EDIT") {
		t.Errorf("Expected auto-generated header, got:\n%s", code)
	}
}

// =============================================================================
// TestWCToCustomElementName
// =============================================================================
//
// The toCustomElementName helper should produce correct custom element names.
func TestWCToCustomElementName(t *testing.T) {
	cases := []struct {
		input    string
		expected string
	}{
		{"Button", "mosaic-button"},
		{"ProfileCard", "mosaic-profile-card"},
		{"HowItWorks", "mosaic-how-it-works"},
	}
	for _, tc := range cases {
		got := toCustomElementName(tc.input)
		if got != tc.expected {
			t.Errorf("toCustomElementName(%q) = %q, want %q", tc.input, got, tc.expected)
		}
	}
}

// =============================================================================
// TestWCToWCField
// =============================================================================
//
// The toWCField helper converts kebab-case slot names to camelCase JS fields.
func TestWCToWCField(t *testing.T) {
	cases := []struct {
		input    string
		expected string
	}{
		{"title", "title"},
		{"avatar-url", "avatarUrl"},
		{"display-name", "displayName"},
		{"show-header", "showHeader"},
	}
	for _, tc := range cases {
		got := toWCField(tc.input)
		if got != tc.expected {
			t.Errorf("toWCField(%q) = %q, want %q", tc.input, got, tc.expected)
		}
	}
}
