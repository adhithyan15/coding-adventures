// MosaicVm.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MosaicVm — Generic tree-walking driver for Mosaic compiler backends
// ============================================================================
//
// The MosaicVM is the fourth stage of the Mosaic compiler pipeline:
//
//   Source → Lexer → Parser → Analyzer → MosaicComponent → **VM** → Backend → Code
//
// The VM's responsibilities:
//   1. Traverse the MosaicComponent IR tree depth-first.
//   2. Normalize every MosaicValue into a ResolvedValue (hex → RGBA,
//      dimension → { value, unit }, slot_ref → resolved with type info).
//   3. Track the SlotContext (component slots + active each-loop scopes).
//   4. Call MosaicRenderer methods in strict open-before-close order.
//
// What the VM Does NOT Do
// -----------------------
//
// The VM is agnostic about output format. It has no knowledge of React, Web
// Components, SwiftUI, or any other platform. Backends own the output — the VM
// only drives the traversal and normalizes values.
//
// Traversal Order
// ---------------
//
//   beginComponent(name, slots)
//     beginNode(root, isPrimitive, resolvedProps, ctx)
//       [for each child of root in source order:]
//         beginNode / endNode            ← child nodes
//         renderSlotChild(...)           ← @slotName; children
//         beginWhen / endWhen            ← when blocks
//         beginEach / endEach            ← each blocks
//     endNode(root)
//   endComponent()
//   → EmitResult
//
// ============================================================================

import MosaicAnalyzer

// ============================================================================
// Resolved value types
// ============================================================================

/// A color with components in 0–255 range.
public struct ResolvedColor: Equatable {
    public let r: Int
    public let g: Int
    public let b: Int
    public let a: Int
    public init(r: Int, g: Int, b: Int, a: Int) {
        self.r = r; self.g = g; self.b = b; self.a = a
    }
    /// CSS rgba() string, e.g. "rgba(37, 99, 235, 1.000)"
    public var cssString: String {
        let alpha = Double(a) / 255.0
        // Round to 3 decimal places without Foundation
        let rounded = Double(Int(alpha * 1000 + 0.5)) / 1000.0
        return "rgba(\(r), \(g), \(b), \(rounded))"
    }
}

/// A dimension with numeric value and unit string.
public struct ResolvedDimension: Equatable {
    public let value: Double
    public let unit: String
    public init(value: Double, unit: String) { self.value = value; self.unit = unit }
    /// CSS pixel string for dp/sp: "16px", or percent: "50%"
    public var cssString: String {
        if unit == "%" { return "\(Int(value))%" }
        return "\(Int(value))px"
    }
}

/// A fully resolved property value — no further parsing needed by backends.
///
/// The VM normalizes all raw MosaicValue variants into ResolvedValue before
/// calling MosaicRenderer.beginNode. Backends only deal with resolved values.
public enum ResolvedValue: Equatable {
    /// A plain string (quotes already stripped).
    case string(String)
    /// A bare numeric value (no unit).
    case number(Double)
    /// A dimension value with unit.
    case dimension(ResolvedDimension)
    /// A parsed RGBA color.
    case color(ResolvedColor)
    /// A slot reference with its resolved type and loop-variable flag.
    case slotRef(name: String, slotType: MosaicType, isLoopVar: Bool)
    /// An identifier used as an enum-like value (e.g. `center`, `bold`).
    case ident(String)
}

/// A property with its name and resolved value.
public struct ResolvedProperty: Equatable {
    public let name: String
    public let value: ResolvedValue
    public init(name: String, value: ResolvedValue) {
        self.name = name; self.value = value
    }
}

// ============================================================================
// SlotContext
// ============================================================================

/// Tracks which slots are in scope at any point during traversal.
///
/// When an `each @items as item` block is entered, the loop variable `item`
/// is pushed onto `loopScopes`. It is conceptually popped when the block ends
/// (the Swift scope naturally handles this via a new context copy).
public struct SlotContext {
    public let componentSlots: [String: MosaicSlot]
    public let loopScopes: [(itemName: String, elementType: MosaicType)]

    public init(componentSlots: [String: MosaicSlot], loopScopes: [(String, MosaicType)]) {
        self.componentSlots = componentSlots
        self.loopScopes = loopScopes
    }
}

// ============================================================================
// EmitResult
// ============================================================================

/// The output of a code-generation backend.
public struct EmitResult {
    /// The primary generated file content (e.g., TSX or TypeScript source).
    public let code: String
    /// Optional filename hint (e.g., "ProfileCard.tsx").
    public let filename: String?

    public init(code: String, filename: String? = nil) {
        self.code = code; self.filename = filename
    }
}

// ============================================================================
// MosaicRenderer protocol
// ============================================================================

/// The interface that all Mosaic compiler backends must implement.
///
/// The VM calls these methods in depth-first order. Backends accumulate output
/// in their internal state and return it from `emit()`.
///
/// Each method is called exactly once per structural element:
///   - `beginComponent` once at the start, `endComponent` once at the end.
///   - `beginNode` / `endNode` once per node element (open before children, close after).
///   - `renderSlotChild` once per `@slotName;` child reference.
///   - `beginWhen` / `endWhen` once per `when @flag { ... }` block.
///   - `beginEach` / `endEach` once per `each @items as item { ... }` block.
public protocol MosaicRenderer: AnyObject {
    func beginComponent(name: String, slots: [MosaicSlot])
    func endComponent()
    func beginNode(tag: String, isPrimitive: Bool, properties: [ResolvedProperty], ctx: SlotContext)
    func endNode(tag: String)
    func renderSlotChild(slotName: String, slotType: MosaicType, ctx: SlotContext)
    func beginWhen(slotName: String, ctx: SlotContext)
    func endWhen()
    func beginEach(slotName: String, itemName: String, elementType: MosaicType, ctx: SlotContext)
    func endEach()
    func emit() -> EmitResult
}

// ============================================================================
// MosaicVMError
// ============================================================================

/// Thrown when the VM encounters a runtime invariant violation.
///
/// These indicate a bug in the analyzer (undefined slot reference should have
/// been caught earlier in the pipeline).
public struct MosaicVMError: Error, CustomStringConvertible {
    public let message: String
    public var description: String { "MosaicVMError: \(message)" }
    public init(_ message: String) { self.message = message }
}

// ============================================================================
// MosaicVM
// ============================================================================

/// The generic tree-walking driver for Mosaic compiler backends.
///
/// Construct a VM with a `MosaicComponent` (from the analyzer), then call
/// `run(renderer:)` with any backend that implements `MosaicRenderer`.
///
/// A single `MosaicVM` instance can be run against multiple renderers — the VM
/// is stateless between `run()` calls.
///
/// Example:
///
///     let component = try analyze(source)
///     let vm = MosaicVM(component: component)
///     let result = try vm.run(renderer: ReactRenderer())
///     print(result.code)
///
public class MosaicVM {
    private let component: MosaicComponent

    public init(component: MosaicComponent) {
        self.component = component
    }

    /// Traverse the IR tree, calling renderer methods in depth-first order.
    ///
    /// - Parameter renderer: A backend implementing `MosaicRenderer`.
    /// - Returns: The `EmitResult` produced by `renderer.emit()`.
    /// - Throws: `MosaicVMError` if an undefined slot reference is encountered.
    public func run(renderer: MosaicRenderer) throws -> EmitResult {
        let slotMap = Dictionary(uniqueKeysWithValues: component.slots.map { ($0.name, $0) })
        let ctx = SlotContext(componentSlots: slotMap, loopScopes: [])

        renderer.beginComponent(name: component.name, slots: component.slots)
        try walkNode(component.root, ctx: ctx, renderer: renderer)
        renderer.endComponent()
        return renderer.emit()
    }

    // -------------------------------------------------------------------------
    // Tree traversal
    // -------------------------------------------------------------------------

    private func walkNode(_ node: MosaicNode, ctx: SlotContext, renderer: MosaicRenderer) throws {
        let resolved = try node.properties.map { p in
            ResolvedProperty(name: p.name, value: try resolveValue(p.value, ctx: ctx))
        }
        renderer.beginNode(tag: node.nodeType, isPrimitive: node.isPrimitive, properties: resolved, ctx: ctx)
        for child in node.children {
            try walkChild(child, ctx: ctx, renderer: renderer)
        }
        renderer.endNode(tag: node.nodeType)
    }

    private func walkChild(_ child: MosaicChild, ctx: SlotContext, renderer: MosaicRenderer) throws {
        switch child {
        case let .node(n):
            try walkNode(n, ctx: ctx, renderer: renderer)

        case let .slotRef(name):
            let slot = try resolveSlot(name, ctx: ctx)
            renderer.renderSlotChild(slotName: name, slotType: slot.slotType, ctx: ctx)

        case let .whenBlock(slot, body):
            renderer.beginWhen(slotName: slot, ctx: ctx)
            for n in body { try walkNode(n, ctx: ctx, renderer: renderer) }
            renderer.endWhen()

        case let .eachBlock(slot, item, body):
            let listSlot = ctx.componentSlots[slot]
            guard let listSlot else {
                throw MosaicVMError("Unknown list slot: @\(slot)")
            }
            guard case let .list(elementType) = listSlot.slotType else {
                throw MosaicVMError("@\(slot) is not a list type")
            }
            renderer.beginEach(slotName: slot, itemName: item, elementType: elementType, ctx: ctx)
            let innerCtx = SlotContext(
                componentSlots: ctx.componentSlots,
                loopScopes: ctx.loopScopes + [(itemName: item, elementType: elementType)]
            )
            for n in body { try walkNode(n, ctx: innerCtx, renderer: renderer) }
            renderer.endEach()
        }
    }

    // -------------------------------------------------------------------------
    // Value resolution
    // -------------------------------------------------------------------------

    private func resolveValue(_ v: MosaicValue, ctx: SlotContext) throws -> ResolvedValue {
        switch v {
        case let .literal(s):
            return .string(s)
        case let .number(val, unit):
            if let unit {
                return .dimension(ResolvedDimension(value: val, unit: unit))
            }
            return .number(val)
        case let .slotRef(name):
            return try resolveSlotRef(name, ctx: ctx)
        case let .color(r, g, b, a):
            return .color(ResolvedColor(r: r, g: g, b: b, a: a))
        }
    }

    private func resolveSlotRef(_ name: String, ctx: SlotContext) throws -> ResolvedValue {
        // Check loop scopes innermost-first
        for scope in ctx.loopScopes.reversed() {
            if scope.itemName == name {
                return .slotRef(name: name, slotType: scope.elementType, isLoopVar: true)
            }
        }
        // Fall back to component slots
        guard let slot = ctx.componentSlots[name] else {
            throw MosaicVMError("Unresolved slot reference: @\(name)")
        }
        return .slotRef(name: name, slotType: slot.slotType, isLoopVar: false)
    }

    private func resolveSlot(_ name: String, ctx: SlotContext) throws -> MosaicSlot {
        guard let slot = ctx.componentSlots[name] else {
            throw MosaicVMError("Unknown slot: @\(name)")
        }
        return slot
    }
}
