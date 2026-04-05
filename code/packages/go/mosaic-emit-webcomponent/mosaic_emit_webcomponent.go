// Package mosaicemitwebcomponent emits TypeScript Custom Element classes (.ts)
// from a Mosaic IR via the MosaicVM.
//
// Architecture: Fragment List with html+= Render Method
// -------------------------------------------------------
//
// Unlike the React backend (which uses a JSX string stack), the Web Components
// renderer builds a flat list of RenderFragments during VM traversal and
// serializes them into a _render() method body during Emit().
//
// The _render() method uses a mutable `let html = ''` accumulator. Each
// fragment type contributes html+= statements:
//
//	- open_tag:  html += '<div style="...">';
//	- close_tag: html += '</div>';
//	- slot_proj: html += '<slot name="..."></slot>';
//	- when_open: if (this._show) {
//	- when_close: }
//	- each_open: this._items.forEach(item => {
//	- each_close: });
//
// Tag Name Convention
// -------------------
//
// PascalCase component names map to kebab-case element names with a mosaic- prefix:
//
//	ProfileCard → <mosaic-profile-card>
//	Button      → <mosaic-button>
//
// Security
// --------
//
// All text slot values are escaped via _escapeHtml() before insertion into innerHTML.
// Colors from the VM are emitted as rgba() strings, never raw user strings.
//
// Usage:
//
//	ir, _ := mosaicanalyzer.Analyze(source)
//	vm := mosaicvm.New(ir)
//	renderer := mosaicemitwebcomponent.NewWebComponentRenderer()
//	result, _ := vm.Run(renderer)
//	fmt.Println(result.Code) // the generated .ts file
package mosaicemitwebcomponent

import (
	"fmt"
	"strings"
	"unicode"

	mosaicanalyzer "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-analyzer"
	mosaicvm "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-vm"
)

// ============================================================================
// Render Fragment Types
// ============================================================================

// renderFragment represents one logical piece of the _render() method body.
type renderFragment struct {
	kind       string // "open_tag", "close_tag", "slot_ref_text", "slot_proj", "when_open", "when_close", "each_open", "each_close"
	htmlStr    string // for open_tag, slot_ref_text
	tag        string // for close_tag
	slotName   string // for slot_proj, when_open
	field      string // for when_open, each_open: the JS field name (this._field)
	itemName   string // for each_open: the loop variable
	isNodeList bool   // for each_open: true if list element type is node/component
}

// ============================================================================
// Stack Frame Types
// ============================================================================

type componentFrameWC struct {
	name      string
	slots     []mosaicanalyzer.MosaicSlot
	fragments []renderFragment
}

type nodeFrameWC struct {
	tag     string
	htmlTag string
	frags   []renderFragment
}

// ============================================================================
// WebComponentRenderer
// ============================================================================

// WebComponentRenderer implements MosaicRenderer and produces a TypeScript
// Custom Element class (.ts) file.
type WebComponentRenderer struct {
	compFrame *componentFrameWC
	nodeStack []*nodeFrameWC
}

// NewWebComponentRenderer creates a new WebComponentRenderer.
func NewWebComponentRenderer() *WebComponentRenderer {
	return &WebComponentRenderer{}
}

// currentFragments returns a pointer to the active fragments slice.
func (r *WebComponentRenderer) currentFragments() *[]renderFragment {
	if len(r.nodeStack) > 0 {
		return &r.nodeStack[len(r.nodeStack)-1].frags
	}
	return &r.compFrame.fragments
}

func (r *WebComponentRenderer) appendFrag(f renderFragment) {
	*r.currentFragments() = append(*r.currentFragments(), f)
}

// BeginComponent initializes the renderer.
func (r *WebComponentRenderer) BeginComponent(name string, slots []mosaicanalyzer.MosaicSlot) {
	r.compFrame = &componentFrameWC{name: name, slots: slots}
	r.nodeStack = nil
}

// EndComponent is a no-op.
func (r *WebComponentRenderer) EndComponent() {}

// BeginNode opens an HTML element and pushes a new frame.
func (r *WebComponentRenderer) BeginNode(tag string, isPrimitive bool, props []mosaicvm.ResolvedProperty, ctx mosaicvm.SlotContext) {
	htmlTag := primitiveToHTMLTag(tag, isPrimitive)
	style := buildWCStyle(tag, props)

	var openHtml string
	if style != "" {
		openHtml = fmt.Sprintf("<%s style=\"%s\">", htmlTag, style)
	} else {
		openHtml = fmt.Sprintf("<%s>", htmlTag)
	}

	// Add content property for Text nodes
	contentProp := findWCProp(props, "content")

	frame := &nodeFrameWC{tag: tag, htmlTag: htmlTag}
	r.nodeStack = append(r.nodeStack, frame)
	r.appendFrag(renderFragment{kind: "open_tag", htmlStr: openHtml})

	// For Text nodes with a content property, add it as escaped text
	if contentProp != nil {
		textExpr := resolvedValueToWCExpr(*contentProp)
		r.appendFrag(renderFragment{kind: "slot_ref_text", htmlStr: textExpr})
	}
}

// EndNode pops the current frame, then appends all frame fragments plus
// the close tag to the parent's fragment list.
func (r *WebComponentRenderer) EndNode(tag string) {
	if len(r.nodeStack) == 0 {
		return
	}

	// Pop the frame
	frame := r.nodeStack[len(r.nodeStack)-1]
	r.nodeStack = r.nodeStack[:len(r.nodeStack)-1]

	// The frame's fragments include the open_tag plus all child content.
	// Append them all to the parent, then append the close tag.
	parent := r.currentFragments()
	*parent = append(*parent, frame.frags...)
	*parent = append(*parent, renderFragment{kind: "close_tag", tag: frame.htmlTag})
}

// RenderSlotChild emits a <slot> projection for a named slot.
func (r *WebComponentRenderer) RenderSlotChild(slotName string, slotType mosaicanalyzer.MosaicType, ctx mosaicvm.SlotContext) {
	r.appendFrag(renderFragment{
		kind:     "slot_proj",
		slotName: slotName,
	})
}

// BeginWhen emits the opening of a conditional block.
func (r *WebComponentRenderer) BeginWhen(slotName string, ctx mosaicvm.SlotContext) {
	r.appendFrag(renderFragment{
		kind:  "when_open",
		field: "_" + toWCField(slotName),
	})
}

// EndWhen closes the conditional block.
func (r *WebComponentRenderer) EndWhen() {
	r.appendFrag(renderFragment{kind: "when_close"})
}

// BeginEach opens a forEach iteration.
func (r *WebComponentRenderer) BeginEach(slotName string, itemName string, elementType mosaicanalyzer.MosaicType, ctx mosaicvm.SlotContext) {
	isNodeList := elementType.Kind == "node" || elementType.Kind == "component"
	r.appendFrag(renderFragment{
		kind:       "each_open",
		field:      "_" + toWCField(slotName),
		itemName:   itemName,
		isNodeList: isNodeList,
	})
}

// EndEach closes a forEach iteration.
func (r *WebComponentRenderer) EndEach() {
	r.appendFrag(renderFragment{kind: "each_close"})
}

// Emit finalizes the output and returns the generated .ts file.
func (r *WebComponentRenderer) Emit() mosaicvm.EmitResult {
	if r.compFrame == nil {
		return mosaicvm.EmitResult{}
	}

	name := r.compFrame.name
	tagName := toCustomElementName(name)

	var sb strings.Builder

	// File header
	sb.WriteString("// Auto-generated by mosaic-emit-webcomponent. DO NOT EDIT.\n\n")

	// Class declaration
	sb.WriteString(fmt.Sprintf("export class %s extends HTMLElement {\n", name))

	// Private field declarations
	for _, slot := range r.compFrame.slots {
		tsType := mosaicTypeToWCTS(slot.Type)
		fieldName := "_" + toWCField(slot.Name)
		if slot.DefaultValue != nil {
			defaultVal := defaultValueToWC(*slot.DefaultValue)
			sb.WriteString(fmt.Sprintf("  private %s: %s = %s;\n", fieldName, tsType, defaultVal))
		} else {
			sb.WriteString(fmt.Sprintf("  private %s!: %s;\n", fieldName, tsType))
		}
	}
	sb.WriteString("\n")

	// connectedCallback
	sb.WriteString("  connectedCallback(): void {\n")
	sb.WriteString("    this.attachShadow({ mode: 'open' });\n")
	sb.WriteString("    this._render();\n")
	sb.WriteString("  }\n\n")

	// Slot setters
	for _, slot := range r.compFrame.slots {
		fieldName := "_" + toWCField(slot.Name)
		setterName := toCamelCaseWC(slot.Name)
		tsType := mosaicTypeToWCTS(slot.Type)
		sb.WriteString(fmt.Sprintf("  set %s(value: %s) {\n", setterName, tsType))
		sb.WriteString(fmt.Sprintf("    this.%s = value;\n", fieldName))
		sb.WriteString("    if (this.shadowRoot) this._render();\n")
		sb.WriteString("  }\n\n")
	}

	// _render method
	sb.WriteString("  private _render(): void {\n")
	sb.WriteString("    let html = '';\n")
	for _, frag := range r.compFrame.fragments {
		sb.WriteString(fragmentToWCCode(frag))
	}
	sb.WriteString("    this.shadowRoot!.innerHTML = html;\n")
	sb.WriteString("  }\n\n")

	// _escapeHtml helper
	sb.WriteString("  private _escapeHtml(s: string): string {\n")
	sb.WriteString("    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')\n")
	sb.WriteString("             .replace(/\"/g, '&quot;').replace(/'/g, '&#39;');\n")
	sb.WriteString("  }\n")

	sb.WriteString("}\n\n")

	// Custom element registration
	sb.WriteString(fmt.Sprintf("customElements.define('%s', %s);\n", tagName, name))

	fileName := tagName + ".ts"
	return mosaicvm.EmitResult{
		Code:     sb.String(),
		FileName: fileName,
	}
}

// ============================================================================
// Helpers
// ============================================================================

// fragmentToWCCode converts a renderFragment to TypeScript _render() code.
func fragmentToWCCode(f renderFragment) string {
	switch f.kind {
	case "open_tag":
		// Use single-quoted JS string: escape backslash and single-quote only.
		// This keeps HTML double-quotes (role="button") unescaped and readable.
		escaped := strings.ReplaceAll(f.htmlStr, `\`, `\\`)
		escaped = strings.ReplaceAll(escaped, `'`, `\'`)
		return fmt.Sprintf("    html += '%s';\n", escaped)
	case "close_tag":
		return fmt.Sprintf("    html += '</%s>';\n", f.tag)
	case "slot_ref_text":
		return fmt.Sprintf("    html += this._escapeHtml(String(%s));\n", f.htmlStr)
	case "slot_proj":
		return fmt.Sprintf("    html += '<slot name=\"%s\"></slot>';\n", f.slotName)
	case "when_open":
		return fmt.Sprintf("    if (this.%s) {\n", f.field)
	case "when_close":
		return "    }\n"
	case "each_open":
		return fmt.Sprintf("    this.%s.forEach((%s) => {\n", f.field, f.itemName)
	case "each_close":
		return "    });\n"
	}
	return ""
}

// primitiveToHTMLTag maps Mosaic primitive node names to HTML element names.
func primitiveToHTMLTag(tag string, isPrimitive bool) string {
	if !isPrimitive {
		return toCustomElementName(tag)
	}
	switch tag {
	case "Row":
		return "div"
	case "Column":
		return "div"
	case "Box":
		return "div"
	case "Stack":
		return "div"
	case "Text":
		return "span"
	case "Image":
		return "img"
	case "Spacer":
		return "div"
	case "Divider":
		return "hr"
	case "Scroll":
		return "div"
	case "Icon":
		return "span"
	default:
		return "div"
	}
}

// buildWCStyle builds an inline style string from resolved properties.
func buildWCStyle(tag string, props []mosaicvm.ResolvedProperty) string {
	var parts []string

	// Add layout styles for primitive nodes
	switch tag {
	case "Column":
		parts = append(parts, "display:flex;flex-direction:column")
	case "Row":
		parts = append(parts, "display:flex;flex-direction:row")
	case "Spacer":
		parts = append(parts, "flex:1")
	case "Stack":
		parts = append(parts, "position:relative")
	case "Scroll":
		parts = append(parts, "overflow:auto")
	}

	for _, p := range props {
		switch p.Name {
		case "padding":
			parts = append(parts, fmt.Sprintf("padding:%s", dimToCSS(p.Value)))
		case "margin":
			parts = append(parts, fmt.Sprintf("margin:%s", dimToCSS(p.Value)))
		case "background", "background-color":
			parts = append(parts, fmt.Sprintf("background-color:%s", colorToCSS(p.Value)))
		case "color":
			parts = append(parts, fmt.Sprintf("color:%s", colorToCSS(p.Value)))
		case "font-size":
			parts = append(parts, fmt.Sprintf("font-size:%s", dimToCSS(p.Value)))
		case "corner-radius":
			parts = append(parts, fmt.Sprintf("border-radius:%s", dimToCSS(p.Value)))
		case "gap":
			parts = append(parts, fmt.Sprintf("gap:%s", dimToCSS(p.Value)))
		}
	}

	return strings.Join(parts, ";")
}

func dimToCSS(v mosaicvm.ResolvedValue) string {
	if v.Kind == "dimension" {
		unit := v.Unit
		if unit == "dp" || unit == "sp" {
			unit = "px"
		}
		return fmt.Sprintf("%g%s", v.NumValue, unit)
	}
	if v.Kind == "string" {
		// CSS keyword allowlist — only permit known safe values to prevent " injection
		// into the style attribute that would break the HTML attribute quoting.
		safe := map[string]bool{
			"auto": true, "none": true, "inherit": true, "initial": true,
			"100%": true, "fit-content": true, "max-content": true, "min-content": true,
		}
		if safe[v.StrValue] {
			return v.StrValue
		}
		return "auto"
	}
	return "auto"
}

func colorToCSS(v mosaicvm.ResolvedValue) string {
	if v.Kind == "color" {
		alpha := float64(v.A) / 255.0
		return fmt.Sprintf("rgba(%d,%d,%d,%.2f)", v.R, v.G, v.B, alpha)
	}
	return "inherit"
}

// resolvedValueToWCExpr converts a ResolvedValue to a TS expression for use in
// html+= `${...}` template literals.
func resolvedValueToWCExpr(p mosaicvm.ResolvedProperty) string {
	switch p.Value.Kind {
	case "slot_ref":
		return "this._" + toWCField(p.Value.SlotName)
	case "string":
		return fmt.Sprintf("%q", p.Value.StrValue)
	case "number":
		return fmt.Sprintf("%g", p.Value.NumValue)
	}
	return ""
}

// findWCProp finds a resolved property by name.
func findWCProp(props []mosaicvm.ResolvedProperty, name string) *mosaicvm.ResolvedProperty {
	for i := range props {
		if props[i].Name == name {
			return &props[i]
		}
	}
	return nil
}

// mosaicTypeToWCTS converts a MosaicType to a TypeScript type string.
func mosaicTypeToWCTS(t mosaicanalyzer.MosaicType) string {
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
		return "HTMLElement | null"
	case "component":
		return "HTMLElement | null"
	case "list":
		if t.Element != nil {
			return mosaicTypeToWCTS(*t.Element) + "[]"
		}
		return "unknown[]"
	}
	return "unknown"
}

// defaultValueToWC converts a MosaicValue to a TypeScript default value string.
func defaultValueToWC(v mosaicanalyzer.MosaicValue) string {
	switch v.Kind {
	case "string":
		return fmt.Sprintf("%q", v.StrValue)
	case "number":
		return fmt.Sprintf("%g", v.NumValue)
	case "bool":
		if v.BoolValue {
			return "true"
		}
		return "false"
	case "color_hex":
		return fmt.Sprintf("%q", v.StrValue)
	}
	return "null"
}

// toCustomElementName converts a PascalCase name to a mosaic-kebab-case custom element name.
// e.g., "ProfileCard" → "mosaic-profile-card"
func toCustomElementName(s string) string {
	kebab := pascalToKebab(s)
	return "mosaic-" + kebab
}

// pascalToKebab converts PascalCase to kebab-case.
func pascalToKebab(s string) string {
	var sb strings.Builder
	for i, r := range s {
		if unicode.IsUpper(r) && i > 0 {
			sb.WriteRune('-')
		}
		sb.WriteRune(unicode.ToLower(r))
	}
	return sb.String()
}

// toWCField converts a kebab-case slot name to a camelCase JS field name.
// e.g., "avatar-url" → "avatarUrl"
func toWCField(s string) string {
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

// toCamelCaseWC converts a kebab-case name to camelCase (same as toWCField).
func toCamelCaseWC(s string) string {
	return toWCField(s)
}
