// MosaicEmitWebcomponent.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MosaicEmitWebcomponent — Web Components backend for the Mosaic compiler
// ============================================================================
//
// This module implements `MosaicRenderer` and emits a TypeScript Custom Element
// class (.ts file) from a MosaicComponent IR.
//
// Architecture
// ------------
//
// The renderer accumulates an array of RenderFragment values during VM
// traversal, then serializes them into a `_render()` method body during emit().
// The `_render()` method uses `let html = ''; html += ...; this.shadowRoot!.innerHTML = html;`.
//
// Tag Name Convention
// -------------------
//
// PascalCase component names map to kebab-case with a `mosaic-` prefix:
//   ProfileCard → <mosaic-profile-card>
//   Button      → <mosaic-button>
//
// Property setters
// ----------------
//
// Each slot becomes a private field and a public setter. Setting a slot
// calls `this._render()` to re-render.
//
// ============================================================================

import MosaicVm
import MosaicAnalyzer

// ============================================================================
// String utility
// ============================================================================

private extension String {
    func trimmingWhitespace() -> String {
        var s = self
        while s.first == " " || s.first == "\t" { s = String(s.dropFirst()) }
        while s.last  == " " || s.last  == "\t" { s = String(s.dropLast()) }
        return s
    }
}

// ============================================================================
// Public entry point
// ============================================================================

/// Compile Mosaic source text to a Web Component TypeScript file.
///
/// - Parameter source: `.mosaic` source text.
/// - Returns: `EmitResult` with generated TypeScript.
/// - Throws: LexError, ParseError, AnalysisError, or MosaicVMError.
///
/// Example:
///
///     let result = try emitWebComponent(source: """
///       component Badge {
///         slot label: text;
///         Text { content: @label; }
///       }
///     """)
///     print(result.code) // → "class MosaicBadge extends HTMLElement { ..."
///
public func emitWebComponent(source: String) throws -> EmitResult {
    let component = try analyze(source)
    let renderer = WebComponentRenderer()
    let vm = MosaicVM(component: component)
    return try vm.run(renderer: renderer)
}

// ============================================================================
// Naming utilities
// ============================================================================

/// "ProfileCard" → "mosaic-profile-card"
private func toElementName(_ pascal: String) -> String {
    var result = "mosaic"
    var isFirst = true
    for ch in pascal {
        if ch.isUppercase && !isFirst {
            result += "-"
        }
        if isFirst { result += "-"; isFirst = false }
        result += String(ch).lowercased()
    }
    return result
}

/// "ProfileCard" → "MosaicProfileCard"
private func toClassName(_ pascal: String) -> String {
    "Mosaic\(pascal)"
}

/// "corner-radius" → "_cornerRadius" (private field name)
private func toFieldName(_ hyphenated: String) -> String {
    let parts = hyphenated.split(separator: "-")
    guard !parts.isEmpty else { return "_\(hyphenated)" }
    let first = parts[0].lowercased()
    let rest = parts.dropFirst().map { $0.prefix(1).uppercased() + $0.dropFirst().lowercased() }
    return "_" + ([first] + rest).joined()
}

/// Mosaic type to TypeScript type string
private func mosaicTypeToTS(_ t: MosaicType) -> String {
    switch t {
    case let .primitive(n):
        switch n {
        case "text":   return "string"
        case "number": return "number"
        case "bool":   return "boolean"
        case "image":  return "string"
        case "color":  return "string"
        case "node":   return "Element | null"
        default:       return "unknown"
        }
    case let .list(inner): return "\(mosaicTypeToTS(inner))[]"
    case let .component(n): return "Element | null /* \(n) */"
    }
}

/// Default value for a TS type (for field initialization)
private func defaultValue(for t: MosaicType) -> String {
    switch t {
    case let .primitive(n):
        switch n {
        case "text", "image", "color": return "''"
        case "number": return "0"
        case "bool":   return "false"
        case "node":   return "null"
        default:       return "null"
        }
    case .list: return "[]"
    case .component: return "null"
    }
}

// ============================================================================
// Render fragment type
// ============================================================================

private enum RenderFragment {
    case openTag(html: String)
    case closeTag(tag: String)
    case selfClosing(html: String)
    case slotRef(expr: String)
    case slotProj(slotName: String)
    case whenOpen(field: String)
    case whenClose
    case eachOpen(field: String, itemName: String)
    case eachClose
}

// ============================================================================
// WebComponentRenderer
// ============================================================================

/// Emits a TypeScript Custom Element class.
public class WebComponentRenderer: MosaicRenderer {

    private var componentName: String = ""
    private var slots: [MosaicSlot] = []
    private var fragments: [RenderFragment] = []

    // stack for building nested nodes
    private struct NodeFrame {
        let tag: String
        let jsxTag: String
        let isPrimitive: Bool
        let selfClose: Bool
        let openHtml: String
        var hasContent: Bool = false
    }
    private var nodeStack: [NodeFrame] = []

    public init() {}

    // -------------------------------------------------------------------------
    // MosaicRenderer protocol
    // -------------------------------------------------------------------------

    public func beginComponent(name: String, slots: [MosaicSlot]) {
        self.componentName = name
        self.slots = slots
        self.fragments = []
        self.nodeStack = []
    }

    public func endComponent() {}

    public func beginNode(tag: String, isPrimitive: Bool, properties: [ResolvedProperty], ctx: SlotContext) {
        let htmlTag = primitiveHtmlTag(tag, isPrimitive: isPrimitive)
        let selfClose = isSelfClosing(tag)
        var style: [String] = []
        var extraAttrs: [String] = []
        var textContent: String? = nil

        if let base = baseStyle(for: tag) {
            style.append(contentsOf: base.split(separator: ";").map {
                String($0).trimmingWhitespace()
            }.filter { !$0.isEmpty })
        }

        for p in properties {
            switch p.name {
            case "content":
                textContent = valueToHtmlExpr(p.value)
            case "source":
                let rawSrc = bareValue(p.value)
                let lower = rawSrc.trimmingCharacters(in: .whitespaces).lowercased()
                let safeSrc = lower.hasPrefix("javascript:") ? "about:blank" : rawSrc
                extraAttrs.append("src=\"\(htmlAttrEscape(safeSrc))\"")
            case "a11y-label":
                extraAttrs.append("aria-label=\"\(htmlAttrEscape(bareValue(p.value)))\"")
            case "a11y-hidden":
                extraAttrs.append("aria-hidden=\"true\"")
            default:
                let css = cssProp(p.name)
                style.append("\(css): \(valueToHtmlStyleValue(p.value))")
            }
        }

        var openHtml = "<\(htmlTag)"
        if !style.isEmpty {
            openHtml += " style=\"\(style.joined(separator: "; "))\""
        }
        for a in extraAttrs { openHtml += " \(a)" }

        let frame = NodeFrame(tag: tag, jsxTag: htmlTag, isPrimitive: isPrimitive, selfClose: selfClose, openHtml: openHtml)
        nodeStack.append(frame)

        if selfClose {
            fragments.append(.selfClosing(html: openHtml + " />"))
        } else {
            fragments.append(.openTag(html: openHtml + ">"))
            if let text = textContent {
                fragments.append(.slotRef(expr: text))
            }
        }
    }

    public func endNode(tag: String) {
        if let frame = nodeStack.popLast(), !frame.selfClose {
            fragments.append(.closeTag(tag: frame.jsxTag))
        }
    }

    public func renderSlotChild(slotName: String, slotType: MosaicType, ctx: SlotContext) {
        fragments.append(.slotProj(slotName: slotName))
    }

    public func beginWhen(slotName: String, ctx: SlotContext) {
        fragments.append(.whenOpen(field: toFieldName(slotName)))
    }

    public func endWhen() {
        fragments.append(.whenClose)
    }

    public func beginEach(slotName: String, itemName: String, elementType: MosaicType, ctx: SlotContext) {
        fragments.append(.eachOpen(field: toFieldName(slotName), itemName: itemName))
    }

    public func endEach() {
        fragments.append(.eachClose)
    }

    public func emit() -> EmitResult {
        let className = toClassName(componentName)
        let elementName = toElementName(componentName)

        // Build field declarations
        var fields: [String] = []
        var setters: [String] = []
        var observedAttrs: [String] = []

        for slot in slots {
            let field = toFieldName(slot.name)
            let tsType = mosaicTypeToTS(slot.slotType)
            let defVal = defaultValue(for: slot.slotType)
            fields.append("  private \(field): \(tsType) = \(defVal);")

            // Public setter
            setters.append(contentsOf: [
                "  set \(slot.name)(value: \(tsType)) {",
                "    this.\(field) = value;",
                "    this._render();",
                "  }",
                "  get \(slot.name)(): \(tsType) { return this.\(field); }",
            ])

            // Observe primitive string/number/bool slots as attributes
            if case .primitive(let n) = slot.slotType,
               n == "text" || n == "number" || n == "bool" || n == "image" || n == "color" {
                observedAttrs.append(slot.name)
            }
        }

        // Serialize render method
        let renderBody = serializeFragments()

        // Build the class
        var lines: [String] = []
        lines.append("// Auto-generated by MosaicEmitWebcomponent — do not edit.")
        lines.append("")
        lines.append("class \(className) extends HTMLElement {")
        lines.append(contentsOf: fields)
        lines.append("")
        if !observedAttrs.isEmpty {
            let attrList = observedAttrs.map { "'\($0)'" }.joined(separator: ", ")
            lines.append("  static get observedAttributes() { return [\(attrList)]; }")
            lines.append("")
            lines.append("  attributeChangedCallback(name: string, _: string | null, newValue: string | null) {")
            lines.append("    (this as any)[name] = newValue ?? '';")
            lines.append("    this._render();")
            lines.append("  }")
            lines.append("")
        }
        lines.append("  connectedCallback() {")
        lines.append("    this.attachShadow({ mode: 'open' });")
        lines.append("    this._render();")
        lines.append("  }")
        lines.append("")
        lines.append(contentsOf: setters)
        lines.append("")
        lines.append("  private _render() {")
        lines.append("    if (!this.shadowRoot) return;")
        lines.append("    let html = '';")
        lines.append(contentsOf: renderBody.map { "    \($0)" })
        lines.append("    this.shadowRoot.innerHTML = html;")
        lines.append("  }")
        lines.append("")
        lines.append("  private _escapeHtml(s: string): string {")
        lines.append("    return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')")
        lines.append("             .replace(/\"/g,'&quot;').replace(/'/g,'&#39;');")
        lines.append("  }")
        lines.append("}")
        lines.append("")
        lines.append("customElements.define('\(elementName)', \(className));")

        return EmitResult(code: lines.joined(separator: "\n"), filename: "\(componentName).ts")
    }

    // -------------------------------------------------------------------------
    // Fragment serialization
    // -------------------------------------------------------------------------

    private func serializeFragments() -> [String] {
        var lines: [String] = []
        var indent = ""
        for frag in fragments {
            switch frag {
            case let .openTag(html):
                lines.append("\(indent)html += '\(jsLiteralEscape(html))';")
            case let .closeTag(tag):
                lines.append("\(indent)html += '</\(tag)>';")
            case let .selfClosing(html):
                lines.append("\(indent)html += '\(jsLiteralEscape(html))';")
            case let .slotRef(expr):
                // Use template literal for expressions
                lines.append("\(indent)html += `\(expr)`;")
            case let .slotProj(slotName):
                lines.append("\(indent)html += '<slot name=\"\(slotName)\"></slot>';")
            case let .whenOpen(field):
                lines.append("\(indent)if (this.\(field)) {")
                indent += "  "
            case .whenClose:
                if !indent.isEmpty { indent = String(indent.dropLast(2)) }
                lines.append("\(indent)}")
            case let .eachOpen(field, itemName):
                lines.append("\(indent)this.\(field).forEach((\(itemName): any) => {")
                indent += "  "
            case .eachClose:
                if !indent.isEmpty { indent = String(indent.dropLast(2)) }
                lines.append("\(indent)});")
            }
        }
        return lines
    }

    // -------------------------------------------------------------------------
    // Value helpers
    // -------------------------------------------------------------------------

    /// Escape a Mosaic literal string so it is safe inside a JS single-quoted string
    /// and inside a template literal `${}` expression.  We escape: \ ' ` $
    private func jsLiteralEscape(_ s: String) -> String {
        return s
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "'", with: "\\'")
            .replacingOccurrences(of: "`", with: "\\`")
            .replacingOccurrences(of: "$", with: "\\$")
    }

    private func valueToHtmlExpr(_ v: ResolvedValue) -> String {
        switch v {
        case let .string(s):    return "${this._escapeHtml('\(jsLiteralEscape(s))')}"
        case let .slotRef(name, slotType, _):
            switch slotType {
            case .primitive("text"), .primitive("image"), .primitive("color"):
                return "${this._escapeHtml(this.\(toFieldName(name)))}"
            case .primitive("number"):
                return "${this.\(toFieldName(name))}"
            default:
                return "${this.\(toFieldName(name))}"
            }
        case let .number(n):
            return "\(n)"
        case let .color(c):     return "\(c.cssString)"
        case let .dimension(d): return "\(d.cssString)"
        case let .ident(s):     return s
        }
    }

    private func valueToHtmlStyleValue(_ v: ResolvedValue) -> String {
        switch v {
        case let .string(s):    return s
        case let .number(n):    return "\(n)"
        case let .dimension(d): return d.cssString
        case let .color(c):     return c.cssString
        case let .slotRef(name, _, _): return "' + this.\(toFieldName(name)) + '"
        case let .ident(s):     return s
        }
    }

    private func bareValue(_ v: ResolvedValue) -> String {
        if case let .string(s) = v { return s }
        return ""
    }

    /// HTML-escape a string for use in an HTML attribute value (inside double-quotes).
    private func htmlAttrEscape(_ s: String) -> String {
        return s
            .replacingOccurrences(of: "&", with: "&amp;")
            .replacingOccurrences(of: "<", with: "&lt;")
            .replacingOccurrences(of: ">", with: "&gt;")
            .replacingOccurrences(of: "\"", with: "&quot;")
    }

    // -------------------------------------------------------------------------
    // Tag / style helpers
    // -------------------------------------------------------------------------

    private func primitiveHtmlTag(_ tag: String, isPrimitive: Bool) -> String {
        guard isPrimitive else { return toElementName(tag) }
        switch tag {
        case "Image":   return "img"
        case "Divider": return "hr"
        default:        return "div"
        }
    }

    private func isSelfClosing(_ tag: String) -> Bool {
        tag == "Image" || tag == "Divider" || tag == "Spacer"
    }

    private func baseStyle(for tag: String) -> String? {
        switch tag {
        case "Column":  return "display: flex; flex-direction: column"
        case "Row":     return "display: flex; flex-direction: row"
        case "Spacer":  return "flex: 1"
        case "Scroll":  return "overflow: auto"
        case "Stack":   return "position: relative"
        default:        return nil
        }
    }

    private func cssProp(_ name: String) -> String {
        // Convert hyphen-case to hyphen-case CSS (no change needed)
        name
    }
}
