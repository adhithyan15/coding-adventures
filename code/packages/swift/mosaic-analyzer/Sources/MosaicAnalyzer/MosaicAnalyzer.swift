// MosaicAnalyzer.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MosaicAnalyzer — Walks a Mosaic ASTNode tree and produces a typed IR
// ============================================================================
//
// The analyzer is the third stage of the Mosaic compiler pipeline:
//
//   Source → Lexer → Tokens → Parser → ASTNode → **Analyzer** → MosaicComponent
//
// What the Analyzer Does
// ----------------------
//
// 1. Strip syntax noise — the AST from the parser keeps structural info
//    (slot names, type names, property names, values) and discards the
//    parse-tree wrapper nodes.
// 2. Resolve types — convert ASTNode.slotType("text") → MosaicType.primitive("text")
//    and ASTNode.listType → MosaicType.list(...)
// 3. Normalize values — parse color AST nodes into (r,g,b,a), number nodes
//    into (Double, unit?), strings into bare strings.
// 4. Classify nodes — "Row", "Column", etc. are primitives; all other names
//    are component references.
// 5. Produce a flat, strongly-typed MosaicComponent struct.
//
// ============================================================================

import MosaicParser
import MosaicLexer

// ============================================================================
// IR Types — the strongly-typed output of the analyzer
// ============================================================================

/// A fully analyzed Mosaic component, ready for code generation.
///
/// This is the primary output type of this module. It contains all semantic
/// information extracted from the source, with syntax noise removed and types
/// resolved.
public struct MosaicComponent: Equatable {
    /// The component's PascalCase name (e.g. "ProfileCard").
    public let name: String
    /// Typed slot declarations in declaration order.
    public let slots: [MosaicSlot]
    /// File-level imports (other components used as slot types).
    public let imports: [MosaicImport]
    /// The root visual node of the component.
    public let root: MosaicNode

    public init(name: String, slots: [MosaicSlot], imports: [MosaicImport], root: MosaicNode) {
        self.name = name; self.slots = slots; self.imports = imports; self.root = root
    }
}

/// A slot declaration: `slot title: text;`
public struct MosaicSlot: Equatable {
    public let name: String
    public let slotType: MosaicType
    public init(name: String, slotType: MosaicType) {
        self.name = name; self.slotType = slotType
    }
}

/// The type of a slot.
///
/// Mosaic has three classes of type:
///   - Primitive: text, number, bool, image, color, node
///   - List<T>: a homogeneous ordered collection
///   - Component: a named imported component (e.g. Button, Badge)
public indirect enum MosaicType: Equatable {
    case primitive(String)
    case list(MosaicType)
    case component(String)
}

/// A visual node in the component tree.
///
/// Primitive nodes (Row, Column, Text, etc.) map to built-in elements.
/// Non-primitive nodes are component references.
public struct MosaicNode: Equatable {
    public let nodeType: String
    public let isPrimitive: Bool
    public let properties: [MosaicProperty]
    public let children: [MosaicChild]

    public init(nodeType: String, isPrimitive: Bool, properties: [MosaicProperty], children: [MosaicChild]) {
        self.nodeType = nodeType; self.isPrimitive = isPrimitive
        self.properties = properties; self.children = children
    }
}

/// A property assignment: `padding: 16dp;`
public struct MosaicProperty: Equatable {
    public let name: String
    public let value: MosaicValue
    public init(name: String, value: MosaicValue) {
        self.name = name; self.value = value
    }
}

/// A resolved property value.
public enum MosaicValue: Equatable {
    /// A string literal (quotes stripped): `"hello"` → `"hello"`
    case literal(String)
    /// A number with optional unit: `16dp` → `.number(16, "dp")`, `42` → `.number(42, nil)`
    case number(Double, String?)
    /// A slot reference: `@title` → `.slotRef("title")`
    case slotRef(String)
    /// A parsed RGBA color (each 0–255).
    case color(Int, Int, Int, Int)
}

/// A child of a node — either another node, a slot ref, a when, or an each.
public indirect enum MosaicChild: Equatable {
    case node(MosaicNode)
    case whenBlock(slot: String, body: [MosaicNode])
    case eachBlock(slot: String, item: String, body: [MosaicNode])
    case slotRef(String)
}

/// A file-level import: `import Button from "./button.mosaic";`
public struct MosaicImport: Equatable {
    public let name: String
    public init(name: String) { self.name = name }
}

// ============================================================================
// Primitive node registry
// ============================================================================

/// The built-in layout and display elements recognized by the Mosaic VM.
///
/// All other node names are treated as component references (imported or
/// self-referencing). This mirrors the TypeScript analyzer's `PRIMITIVE_NODES`.
private let primitiveNodes: Set<String> = [
    "Row", "Column", "Box", "Stack",
    "Text", "Image", "Icon",
    "Spacer", "Divider", "Scroll",
]

// ============================================================================
// AnalysisError
// ============================================================================

/// Thrown when the analyzer encounters a structural problem in the AST.
///
/// These are "should not happen" errors if the parser is correct — they guard
/// against unexpected AST shapes.
public struct AnalysisError: Error, CustomStringConvertible {
    public let message: String
    public var description: String { "AnalysisError: \(message)" }
    public init(_ message: String) { self.message = message }
}

// ============================================================================
// Public API
// ============================================================================

/// Analyze Mosaic source text and return a typed MosaicComponent.
///
/// This is the primary entry point. It parses the source, then analyzes
/// the resulting AST to produce a validated IR.
///
/// - Parameter source: The `.mosaic` source text.
/// - Returns: A `MosaicComponent` ready for code generation.
/// - Throws: `LexError`, `ParseError`, or `AnalysisError`.
///
/// Example:
///
///     let comp = try analyze("""
///       component Label {
///         slot text: text;
///         Text { content: @text; }
///       }
///     """)
///     print(comp.name)          // "Label"
///     print(comp.slots[0].name) // "text"
///
public func analyze(_ source: String) throws -> MosaicComponent {
    let ast = try parse(source)
    return try analyzeAST(ast)
}

/// Analyze a pre-parsed ASTNode.
///
/// Use this when you already hold an AST from the parser.
public func analyzeAST(_ ast: ASTNode) throws -> MosaicComponent {
    guard case let .component(name, slots, body) = ast else {
        throw AnalysisError("Expected .component at top level")
    }

    let mSlots = try slots.map { try analyzeSlot($0) }
    let mImports: [MosaicImport] = []
    let root = try analyzeNode(body)

    return MosaicComponent(name: name, slots: mSlots, imports: mImports, root: root)
}

// ============================================================================
// Slot analysis
// ============================================================================

private func analyzeSlot(_ node: ASTNode) throws -> MosaicSlot {
    guard case let .slot(name, typeNode) = node else {
        throw AnalysisError("Expected .slot node")
    }
    let t = try analyzeType(typeNode)
    return MosaicSlot(name: name, slotType: t)
}

private func analyzeType(_ node: ASTNode) throws -> MosaicType {
    switch node {
    case let .slotType(name):
        let primitiveTypes: Set<String> = ["text", "number", "bool", "image", "color", "node"]
        if primitiveTypes.contains(name) {
            return .primitive(name)
        }
        return .component(name)
    case let .listType(inner):
        return .list(try analyzeType(inner))
    default:
        throw AnalysisError("Expected slot type node, got \(node)")
    }
}

// ============================================================================
// Node analysis
// ============================================================================

private func analyzeNode(_ node: ASTNode) throws -> MosaicNode {
    guard case let .node(type, properties, children) = node else {
        throw AnalysisError("Expected .node, got \(node)")
    }
    let isPrimitive = primitiveNodes.contains(type)
    let mProps = try properties.map { try analyzeProperty($0) }
    let mChildren = try children.compactMap { try analyzeChild($0) }
    return MosaicNode(nodeType: type, isPrimitive: isPrimitive, properties: mProps, children: mChildren)
}

private func analyzeProperty(_ node: ASTNode) throws -> MosaicProperty {
    guard case let .property(name, value) = node else {
        throw AnalysisError("Expected .property node")
    }
    let mValue = try analyzeValue(value)
    return MosaicProperty(name: name, value: mValue)
}

private func analyzeValue(_ node: ASTNode) throws -> MosaicValue {
    switch node {
    case let .literal(s):
        // Strip surrounding quotes if present
        if s.hasPrefix("\"") && s.hasSuffix("\"") && s.count >= 2 {
            return .literal(String(s.dropFirst().dropLast()))
        }
        return .literal(s)
    case let .number(v, unit):
        return .number(v, unit)
    case let .slotRef(name):
        return .slotRef(name)
    case let .color(r, g, b, a):
        return .color(r, g, b, a)
    default:
        throw AnalysisError("Unexpected value node: \(node)")
    }
}

private func analyzeChild(_ node: ASTNode) throws -> MosaicChild? {
    switch node {
    case .node:
        return .node(try analyzeNode(node))
    case let .whenBlock(slot, body):
        let mBody = try body.compactMap { n -> MosaicNode? in
            if case .node = n { return try analyzeNode(n) }
            return nil
        }
        return .whenBlock(slot: slot, body: mBody)
    case let .eachBlock(slot, item, body):
        let mBody = try body.compactMap { n -> MosaicNode? in
            if case .node = n { return try analyzeNode(n) }
            return nil
        }
        return .eachBlock(slot: slot, item: item, body: mBody)
    case let .slotRef(name):
        return .slotRef(name)
    default:
        return nil
    }
}
