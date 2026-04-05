// Package mosaicemitreact emits TypeScript React functional components (.tsx)
// from a Mosaic IR via the MosaicVM.
//
// Architecture: String Stack
// --------------------------
//
// The renderer maintains a stack of string buffers, one per open node.
// When BeginNode is called, a new buffer is pushed. When EndNode is called,
// the buffer is popped, wrapped in a JSX element string, and appended to the
// parent buffer. This handles arbitrary nesting without lookahead:
//
//	BeginComponent("Card")         → stack: [component-buf]
//	BeginNode("Column")            → stack: [component-buf, column-buf]
//	BeginNode("Text")              → stack: [component-buf, column-buf, text-buf]
//	EndNode("Text")                → pop text-buf → "<span>...</span>"
//	                                 append to column-buf
//	EndNode("Column")              → pop column-buf → "<div>...<span>...</span></div>"
//	                                 append to component-buf
//	EndComponent()                 → no-op; component-buf holds the root JSX
//	Emit()                         → wrap component-buf in the full function file
//
// Primitive Node → JSX Element Mapping
// -------------------------------------
//
//	Box     → <div>
//	Column  → <div style={{ display: 'flex', flexDirection: 'column' }}>
//	Row     → <div style={{ display: 'flex', flexDirection: 'row' }}>
//	Text    → <span>
//	Image   → <img src={...} />
//	Spacer  → <div style={{ flex: 1 }} />
//	Scroll  → <div style={{ overflow: 'auto' }}>
//	Divider → <hr />
//	Stack   → <div style={{ position: 'relative' }}>
//	Icon    → <span>
//
// Colors are emitted as rgba(r, g, b, alpha/255). Dimensions use px for dp/sp.
//
// Usage:
//
//	ir, _ := mosaicanalyzer.Analyze(source)
//	vm := mosaicvm.New(ir)
//	renderer := mosaicemitreact.NewReactRenderer()
//	result, _ := vm.Run(renderer)
//	fmt.Println(result.Code) // the generated .tsx file
package mosaicemitreact

import (
	"fmt"
	"strings"
	"unicode"

	mosaicanalyzer "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer"
	mosaicvm "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-vm"
)

// ============================================================================
// Stack Frame Types
// ============================================================================

// componentFrame is the root frame created in BeginComponent.
// It holds the generated lines for the component function body.
type componentFrame struct {
	name  string
	slots []mosaicanalyzer.MosaicSlot
	lines []string
}

// nodeFrame is created in BeginNode and popped in EndNode.
// openTag is the JSX opening tag. lines accumulates JSX child content.
type nodeFrame struct {
	tag         string
	openTag     string
	selfClosing bool
	lines       []string
}

// ============================================================================
// ReactRenderer
// ============================================================================

// ReactRenderer implements MosaicRenderer and produces a TypeScript React
// functional component (.tsx) file.
type ReactRenderer struct {
	componentFrame *componentFrame
	nodeStack      []*nodeFrame
}

// NewReactRenderer creates a new ReactRenderer.
func NewReactRenderer() *ReactRenderer {
	return &ReactRenderer{}
}

// currentNodeLines returns a pointer to the lines slice of the current frame.
// If a nodeFrame is on the stack, returns its lines; otherwise the component lines.
func (r *ReactRenderer) currentLines() *[]string {
	if len(r.nodeStack) > 0 {
		return &r.nodeStack[len(r.nodeStack)-1].lines
	}
	return &r.componentFrame.lines
}

// appendLine appends a JSX line to the current frame.
func (r *ReactRenderer) appendLine(line string) {
	*r.currentLines() = append(*r.currentLines(), line)
}

// BeginComponent initializes the renderer for a new component.
func (r *ReactRenderer) BeginComponent(name string, slots []mosaicanalyzer.MosaicSlot) {
	r.componentFrame = &componentFrame{name: name, slots: slots}
	r.nodeStack = nil
}

// EndComponent is a no-op; the component frame holds the root JSX.
func (r *ReactRenderer) EndComponent() {}

// BeginNode opens a JSX element and pushes a new frame.
func (r *ReactRenderer) BeginNode(tag string, isPrimitive bool, props []mosaicvm.ResolvedProperty, ctx mosaicvm.SlotContext) {
	jsxTag, selfClosing := primitiveToJSXTag(tag, isPrimitive)
	style := buildReactStyle(props)
	attrs := buildReactAttrs(props, ctx)

	var openTag string
	if style != "" && attrs != "" {
		openTag = fmt.Sprintf("<%s style={%s} %s>", jsxTag, style, attrs)
	} else if style != "" {
		openTag = fmt.Sprintf("<%s style={%s}>", jsxTag, style)
	} else if attrs != "" {
		openTag = fmt.Sprintf("<%s %s>", jsxTag, attrs)
	} else {
		openTag = fmt.Sprintf("<%s>", jsxTag)
	}

	frame := &nodeFrame{
		tag:         tag,
		openTag:     openTag,
		selfClosing: selfClosing,
	}
	r.nodeStack = append(r.nodeStack, frame)

	// Add content prop for Text nodes
	contentProp := findResolvedProp(props, "content")
	if contentProp != nil {
		frame.lines = append(frame.lines, resolvedValueToJSX(*contentProp))
	}
}

// EndNode pops the current frame and appends its JSX to the parent frame.
func (r *ReactRenderer) EndNode(tag string) {
	if len(r.nodeStack) == 0 {
		return
	}

	frame := r.nodeStack[len(r.nodeStack)-1]
	r.nodeStack = r.nodeStack[:len(r.nodeStack)-1]

	jsxTag, _ := primitiveToJSXTag(tag, false)
	_ = jsxTag

	// Build the complete JSX for this node
	var sb strings.Builder
	if frame.selfClosing && len(frame.lines) == 0 {
		// Self-closing: <img src="..." />
		closeTag := strings.Replace(frame.openTag, ">", " />", 1)
		if !strings.HasSuffix(frame.openTag, ">") {
			closeTag = frame.openTag + " />"
		}
		// Replace the > at the end with />
		if strings.HasSuffix(frame.openTag, ">") {
			closeTag = frame.openTag[:len(frame.openTag)-1] + " />"
		} else {
			closeTag = frame.openTag + " />"
		}
		sb.WriteString(closeTag)
	} else {
		sb.WriteString(frame.openTag)
		for _, line := range frame.lines {
			sb.WriteString(line)
		}
		// Determine the closing tag from the open tag
		idx := strings.Index(frame.openTag, " ")
		if idx < 0 {
			idx = strings.Index(frame.openTag, ">")
		}
		jsxTagName := frame.openTag[1:idx]
		sb.WriteString(fmt.Sprintf("</%s>", jsxTagName))
	}

	r.appendLine(sb.String())
}

// RenderSlotChild emits a JSX expression for a slot used as a child.
func (r *ReactRenderer) RenderSlotChild(slotName string, slotType mosaicanalyzer.MosaicType, ctx mosaicvm.SlotContext) {
	camel := toCamelCase(slotName)
	r.appendLine(fmt.Sprintf("{%s}", camel))
}

// BeginWhen emits the opening of a conditional expression.
func (r *ReactRenderer) BeginWhen(slotName string, ctx mosaicvm.SlotContext) {
	camel := toCamelCase(slotName)
	r.appendLine(fmt.Sprintf("{%s && (", camel))
}

// EndWhen closes a conditional expression.
func (r *ReactRenderer) EndWhen() {
	r.appendLine(")}")
}

// BeginEach emits the opening of a map() iteration.
func (r *ReactRenderer) BeginEach(slotName string, itemName string, elementType mosaicanalyzer.MosaicType, ctx mosaicvm.SlotContext) {
	camel := toCamelCase(slotName)
	r.appendLine(fmt.Sprintf("{%s.map((%s, _idx) => (", camel, itemName))
}

// EndEach closes a map() iteration.
func (r *ReactRenderer) EndEach() {
	r.appendLine("))}")
}

// Emit finalizes the output and returns the generated .tsx file.
func (r *ReactRenderer) Emit() mosaicvm.EmitResult {
	if r.componentFrame == nil {
		return mosaicvm.EmitResult{}
	}

	name := r.componentFrame.name
	var sb strings.Builder

	// File header
	sb.WriteString("// Auto-generated by mosaic-emit-react. DO NOT EDIT.\n")
	sb.WriteString("import React from 'react';\n\n")

	// Props interface
	sb.WriteString(fmt.Sprintf("interface %sProps {\n", name))
	for _, slot := range r.componentFrame.slots {
		typStr := mosaicTypeToTS(slot.Type)
		optional := ""
		if !slot.Required {
			optional = "?"
		}
		sb.WriteString(fmt.Sprintf("  %s%s: %s;\n", toCamelCase(slot.Name), optional, typStr))
	}
	sb.WriteString("}\n\n")

	// Function component
	sb.WriteString(fmt.Sprintf("export function %s({ ", name))
	slotNames := make([]string, len(r.componentFrame.slots))
	for i, slot := range r.componentFrame.slots {
		slotNames[i] = toCamelCase(slot.Name)
	}
	sb.WriteString(strings.Join(slotNames, ", "))
	sb.WriteString(fmt.Sprintf(" }: %sProps): JSX.Element {\n", name))
	sb.WriteString("  return (\n")

	// Root JSX
	for _, line := range r.componentFrame.lines {
		sb.WriteString("    ")
		sb.WriteString(line)
		sb.WriteString("\n")
	}

	sb.WriteString("  );\n}\n")

	fileName := toKebabCase(name) + ".tsx"
	return mosaicvm.EmitResult{
		Code:     sb.String(),
		FileName: fileName,
	}
}

// ============================================================================
// Helpers
// ============================================================================

// primitiveToJSXTag maps Mosaic primitive node names to JSX element names.
// Returns (jsxTag, selfClosing).
func primitiveToJSXTag(tag string, isPrimitive bool) (string, bool) {
	if !isPrimitive {
		// Non-primitive: use the component tag name as-is (PascalCase = React component)
		return tag, false
	}
	switch tag {
	case "Row":
		return "div", false
	case "Column":
		return "div", false
	case "Box":
		return "div", false
	case "Stack":
		return "div", false
	case "Text":
		return "span", false
	case "Image":
		return "img", true
	case "Spacer":
		return "div", true
	case "Divider":
		return "hr", true
	case "Scroll":
		return "div", false
	case "Icon":
		return "span", false
	default:
		return "div", false
	}
}

// buildReactStyle builds the inline style object string from resolved properties.
// Returns an empty string if there are no style properties.
func buildReactStyle(props []mosaicvm.ResolvedProperty) string {
	styleProps := make(map[string]string)

	// Add default styles for layout nodes based on properties
	for _, p := range props {
		switch p.Name {
		case "padding":
			styleProps["padding"] = resolvedDimToCSS(p.Value)
		case "margin":
			styleProps["margin"] = resolvedDimToCSS(p.Value)
		case "background", "background-color":
			styleProps["backgroundColor"] = resolvedColorToCSS(p.Value)
		case "color":
			styleProps["color"] = resolvedColorToCSS(p.Value)
		case "font-size":
			styleProps["fontSize"] = resolvedDimToCSS(p.Value)
		case "width":
			styleProps["width"] = resolvedDimToCSS(p.Value)
		case "height":
			styleProps["height"] = resolvedDimToCSS(p.Value)
		case "corner-radius":
			styleProps["borderRadius"] = resolvedDimToCSS(p.Value)
		case "gap":
			styleProps["gap"] = resolvedDimToCSS(p.Value)
		}
	}

	if len(styleProps) == 0 {
		return ""
	}

	parts := make([]string, 0, len(styleProps))
	for k, v := range styleProps {
		parts = append(parts, fmt.Sprintf("%s: %s", k, v))
	}
	return "{ " + strings.Join(parts, ", ") + " }"
}

// buildReactAttrs builds non-style attribute strings (src for img, aria-*, etc.)
func buildReactAttrs(props []mosaicvm.ResolvedProperty, ctx mosaicvm.SlotContext) string {
	var parts []string
	for _, p := range props {
		switch p.Name {
		case "source":
			parts = append(parts, fmt.Sprintf("src={%s}", resolvedValueToJSXExpr(p.Value)))
		case "a11y-label":
			parts = append(parts, fmt.Sprintf("aria-label={%s}", resolvedValueToJSXExpr(p.Value)))
		case "a11y-hidden":
			parts = append(parts, "aria-hidden")
		}
	}
	return strings.Join(parts, " ")
}

// resolvedDimToCSS converts a ResolvedValue dimension to a CSS string.
func resolvedDimToCSS(v mosaicvm.ResolvedValue) string {
	if v.Kind == "dimension" {
		unit := v.Unit
		if unit == "dp" || unit == "sp" {
			unit = "px"
		}
		return fmt.Sprintf("'%g%s'", v.NumValue, unit)
	}
	if v.Kind == "string" {
		return fmt.Sprintf("'%s'", v.StrValue)
	}
	if v.Kind == "slot_ref" {
		return fmt.Sprintf("{%s}", toCamelCase(v.SlotName))
	}
	return "'auto'"
}

// resolvedColorToCSS converts a ResolvedValue color to a CSS rgba() string.
func resolvedColorToCSS(v mosaicvm.ResolvedValue) string {
	if v.Kind == "color" {
		alpha := float64(v.A) / 255.0
		return fmt.Sprintf("'rgba(%d, %d, %d, %.2f)'", v.R, v.G, v.B, alpha)
	}
	if v.Kind == "slot_ref" {
		return fmt.Sprintf("{%s}", toCamelCase(v.SlotName))
	}
	return "'inherit'"
}

// resolvedValueToJSX returns the JSX text content for a property value.
// Used for "content" properties of Text nodes.
func resolvedValueToJSX(p mosaicvm.ResolvedProperty) string {
	switch p.Value.Kind {
	case "string":
		return p.Value.StrValue
	case "slot_ref":
		return fmt.Sprintf("{%s}", toCamelCase(p.Value.SlotName))
	case "number":
		return fmt.Sprintf("{%g}", p.Value.NumValue)
	case "bool":
		if p.Value.BoolValue {
			return "{true}"
		}
		return "{false}"
	}
	return ""
}

// resolvedValueToJSXExpr returns a JSX expression for a value (for use in attributes).
// For string literals used as src, we reject javascript: URLs at code-generation time
// to prevent the generated component from containing XSS vectors.
func resolvedValueToJSXExpr(v mosaicvm.ResolvedValue) string {
	switch v.Kind {
	case "string":
		s := v.StrValue
		// Reject javascript: URLs in src attributes at compile time
		lower := strings.ToLower(strings.TrimSpace(s))
		if strings.HasPrefix(lower, "javascript:") {
			return "'about:blank'"
		}
		// Escape backslash and single-quote to prevent JS string injection
		escaped := strings.ReplaceAll(s, `\`, `\\`)
		escaped = strings.ReplaceAll(escaped, `'`, `\'`)
		return fmt.Sprintf("'%s'", escaped)
	case "slot_ref":
		return toCamelCase(v.SlotName)
	case "number":
		return fmt.Sprintf("%g", v.NumValue)
	}
	return "''"
}

// findResolvedProp finds a resolved property by name.
func findResolvedProp(props []mosaicvm.ResolvedProperty, name string) *mosaicvm.ResolvedProperty {
	for i := range props {
		if props[i].Name == name {
			return &props[i]
		}
	}
	return nil
}

// mosaicTypeToTS converts a MosaicType to a TypeScript type string.
func mosaicTypeToTS(t mosaicanalyzer.MosaicType) string {
	switch t.Kind {
	case "text":
		return "string"
	case "number":
		return "number"
	case "bool":
		return "boolean"
	case "image":
		return "string"
	case "color":
		return "string"
	case "node":
		return "React.ReactNode"
	case "component":
		return "React.ReactNode"
	case "list":
		if t.Element != nil {
			return mosaicTypeToTS(*t.Element) + "[]"
		}
		return "unknown[]"
	}
	return "unknown"
}

// toCamelCase converts a kebab-case name to camelCase.
// e.g., "avatar-url" → "avatarUrl", "display-name" → "displayName"
func toCamelCase(s string) string {
	parts := strings.Split(s, "-")
	if len(parts) == 1 {
		return s
	}
	var sb strings.Builder
	sb.WriteString(parts[0])
	for _, part := range parts[1:] {
		if len(part) > 0 {
			sb.WriteString(strings.ToUpper(part[:1]) + part[1:])
		}
	}
	return sb.String()
}

// toKebabCase converts a PascalCase name to kebab-case.
// e.g., "ProfileCard" → "profile-card"
func toKebabCase(s string) string {
	var sb strings.Builder
	for i, r := range s {
		if unicode.IsUpper(r) && i > 0 {
			sb.WriteRune('-')
		}
		sb.WriteRune(unicode.ToLower(r))
	}
	return sb.String()
}
