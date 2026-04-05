// MosaicEmitReact.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MosaicEmitReact — React backend for the Mosaic compiler
// ============================================================================
//
// This module implements `MosaicRenderer` and emits a TypeScript React
// functional component (.tsx file) from a MosaicComponent IR.
//
// Architecture: String Stack
// --------------------------
//
// The renderer maintains a stack of string buffers, one per open node.
// When `beginNode` is called, a new buffer is pushed. When `endNode` is
// called, the buffer is popped, wrapped in a JSX element, and appended to
// the parent buffer.
//
// Primitive Node → JSX Element Mapping
// -------------------------------------
//
//   Box     → <div>
//   Column  → <div style={{ display: 'flex', flexDirection: 'column' }}>
//   Row     → <div style={{ display: 'flex', flexDirection: 'row' }}>
//   Text    → <span>
//   Image   → <img />
//   Spacer  → <div style={{ flex: 1 }} />
//   Scroll  → <div style={{ overflow: 'auto' }}>
//   Divider → <hr />
//   Stack   → <div style={{ position: 'relative' }}>
//   Icon    → <span>
//
// ============================================================================

import MosaicVm
import MosaicAnalyzer

// ============================================================================
// String utility (no Foundation dependency)
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

/// Compile Mosaic source text to a React TSX file.
///
/// - Parameter source: `.mosaic` source text.
/// - Returns: `EmitResult` with the generated `.tsx` code.
/// - Throws: LexError, ParseError, AnalysisError, or MosaicVMError.
///
/// Example:
///
///     let result = try emitReact(source: """
///       component Label {
///         slot text: text;
///         Text { content: @text; }
///       }
///     """)
///     print(result.code) // → "import React from 'react';\n..."
///
public func emitReact(source: String) throws -> EmitResult {
    let component = try analyze(source)
    let renderer = ReactRenderer()
    let vm = MosaicVM(component: component)
    return try vm.run(renderer: renderer)
}

// ============================================================================
// Tag mapping
// ============================================================================

/// Map a Mosaic primitive node tag to the base HTML element name.
private func htmlTag(for mosaicTag: String) -> String {
    switch mosaicTag {
    case "Image":   return "img"
    case "Divider": return "hr"
    default:        return "div"
    }
}

/// Whether a tag generates a self-closing element (no children).
private func isSelfClosing(_ tag: String) -> Bool {
    tag == "Image" || tag == "Divider" || tag == "Spacer"
}

/// Intrinsic styles for primitive layout nodes.
private func baseStyle(for mosaicTag: String) -> String? {
    switch mosaicTag {
    case "Column":  return "display: 'flex', flexDirection: 'column'"
    case "Row":     return "display: 'flex', flexDirection: 'row'"
    case "Spacer":  return "flex: 1"
    case "Scroll":  return "overflow: 'auto'"
    case "Stack":   return "position: 'relative'"
    default:        return nil
    }
}

// ============================================================================
// Property → inline style helpers
// ============================================================================

/// Convert a Mosaic property to its JSX attribute string.
///
/// Most Mosaic properties become inline style entries. Special cases:
///   - `content` on Text → children text / expression
///   - `source` on Image → `src` attribute
///   - `a11y-label` → `aria-label`
///   - `a11y-role`  → `role`
///   - `a11y-hidden`→ `aria-hidden`
///
private func resolvedValueToStyleExpr(_ v: ResolvedValue) -> String {
    switch v {
    case let .string(s):    return "'\(s)'"
    case let .number(n):    return "\(Int(n) == Int(n.rounded()) ? String(Int(n)) : String(n))"
    case let .dimension(d): return "'\(d.cssString)'"
    case let .color(c):     return "'\(c.cssString)'"
    case let .slotRef(name, _, _): return "{\(name)}"
    case let .ident(s):     return "'\(s)'"
    }
}

// ============================================================================
// ReactRenderer — MosaicRenderer implementation
// ============================================================================

/// Emits a TypeScript React functional component (.tsx).
public class ReactRenderer: MosaicRenderer {

    // Stack of JSX buffers — one per open node.
    private struct Frame {
        var lines: [String]
        let tag: String
        let jsxTag: String
        let isPrimitive: Bool
        var styleEntries: [String]
        var textContent: String?     // for Text nodes: the content expression
        var isSelfClosing: Bool
    }

    private var stack: [Frame] = []
    private var componentFrame: [String] = []
    private var componentName: String = ""
    private var slots: [MosaicSlot] = []
    private var indent: Int = 2

    public init() {}

    // -------------------------------------------------------------------------
    // MosaicRenderer protocol
    // -------------------------------------------------------------------------

    public func beginComponent(name: String, slots: [MosaicSlot]) {
        self.componentName = name
        self.slots = slots
        componentFrame = []
    }

    public func endComponent() {
        // nothing — the root node's endNode populates componentFrame
    }

    public func beginNode(tag: String, isPrimitive: Bool, properties: [ResolvedProperty], ctx: SlotContext) {
        let jsxTag = isPrimitive ? htmlTag(for: tag) : kebabToJsx(tag)
        var styleEntries: [String] = []
        var textContent: String? = nil
        var extraAttrs: [String] = []

        // Intrinsic base styles
        if let base = baseStyle(for: tag) {
            for part in base.split(separator: ",").map({ String($0).trimmingWhitespace() }) {
                styleEntries.append(part)
            }
        }

        // Map properties
        for p in properties {
            switch p.name {
            case "content":
                // Text content — becomes JSX children
                textContent = valueToJsxExpr(p.value)
            case "source":
                extraAttrs.append("src={\(valueToJsxExpr(p.value, bare: true))}")
            case "a11y-label":
                extraAttrs.append("aria-label=\"\(bareString(p.value))\"")
            case "a11y-role":
                extraAttrs.append("role=\"\(bareString(p.value))\"")
            case "a11y-hidden":
                extraAttrs.append("aria-hidden=\"true\"")
            default:
                // Map to camelCase style property
                let cssKey = camelCase(p.name)
                styleEntries.append("\(cssKey): \(resolvedValueToStyleExpr(p.value))")
            }
        }

        // Build open-tag string
        var openTag = "<\(jsxTag)"
        if !styleEntries.isEmpty {
            let styleObj = styleEntries.joined(separator: ", ")
            openTag += " style={{ \(styleObj) }}"
        }
        for attr in extraAttrs { openTag += " \(attr)" }
        if isSelfClosing(tag) { openTag += " />" }
        else { openTag += ">" }

        let frame = Frame(
            lines: [],
            tag: tag,
            jsxTag: jsxTag,
            isPrimitive: isPrimitive,
            styleEntries: styleEntries,
            textContent: textContent,
            isSelfClosing: isSelfClosing(tag)
        )
        stack.append(frame)
        // Push the open tag as the first line of this frame's content
        pushLine(openTag)
    }

    public func endNode(tag: String) {
        guard let frame = stack.popLast() else { return }
        // Finalize the frame into a JSX element
        if !frame.isSelfClosing {
            // Insert text content if present
            if let text = frame.textContent {
                pushLine("  \(text)")
            }
            pushLine("</\(frame.jsxTag)>")
        }
        // Collect all lines from frame.lines
        let content = stack.isEmpty ? frame.lines : frame.lines
        if stack.isEmpty {
            componentFrame = content
        } else {
            stack[stack.count - 1].lines.append(contentsOf: content)
        }
    }

    public func renderSlotChild(slotName: String, slotType: MosaicType, ctx: SlotContext) {
        pushLine("{\(slotName)}")
    }

    public func beginWhen(slotName: String, ctx: SlotContext) {
        pushLine("{\(slotName) && (")
    }

    public func endWhen() {
        pushLine(")}")
    }

    public func beginEach(slotName: String, itemName: String, elementType: MosaicType, ctx: SlotContext) {
        pushLine("{\(slotName).map((\(itemName)) => (")
    }

    public func endEach() {
        pushLine("))}")
    }

    public func emit() -> EmitResult {
        let propsType = buildPropsInterface()
        let body = componentFrame.map { "  \($0)" }.joined(separator: "\n")
        let code = """
        // Auto-generated by MosaicEmitReact — do not edit.
        import React from 'react';

        \(propsType)

        export function \(componentName)({ \(slotNames()) }: \(componentName)Props): React.ReactElement {
          return (
        \(body)
          );
        }
        """
        return EmitResult(code: code, filename: "\(componentName).tsx")
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    private func pushLine(_ line: String) {
        if stack.isEmpty {
            componentFrame.append(line)
        } else {
            stack[stack.count - 1].lines.append(line)
        }
    }

    private func buildPropsInterface() -> String {
        var lines = ["interface \(componentName)Props {"]
        for slot in slots {
            lines.append("  \(slot.name): \(mosaicTypeToTS(slot.slotType));")
        }
        lines.append("}")
        return lines.joined(separator: "\n")
    }

    private func slotNames() -> String {
        slots.map(\.name).joined(separator: ", ")
    }

    private func mosaicTypeToTS(_ t: MosaicType) -> String {
        switch t {
        case let .primitive(n):
            switch n {
            case "text":   return "string"
            case "number": return "number"
            case "bool":   return "boolean"
            case "image":  return "string"
            case "color":  return "string"
            case "node":   return "React.ReactNode"
            default:       return "unknown"
            }
        case let .list(inner): return "\(mosaicTypeToTS(inner))[]"
        case let .component(n): return n
        }
    }

    private func valueToJsxExpr(_ v: ResolvedValue, bare: Bool = false) -> String {
        switch v {
        case let .string(s):    return bare ? s : s
        case let .slotRef(name, _, _): return name
        case let .dimension(d): return d.cssString
        case let .color(c):     return c.cssString
        case let .number(n):    return "\(n)"
        case let .ident(s):     return s
        }
    }

    private func bareString(_ v: ResolvedValue) -> String {
        if case let .string(s) = v { return s }
        return ""
    }

    /// Convert hyphen-case to camelCase for CSS property names.
    /// e.g. "corner-radius" → "cornerRadius"
    private func camelCase(_ s: String) -> String {
        let parts = s.split(separator: "-")
        guard !parts.isEmpty else { return s }
        let first = parts[0].lowercased()
        let rest = parts.dropFirst().map { $0.prefix(1).uppercased() + $0.dropFirst().lowercased() }
        return ([first] + rest).joined()
    }

    /// Convert PascalCase component name to a valid JSX tag (pass-through for imported components).
    private func kebabToJsx(_ s: String) -> String { s }
}
