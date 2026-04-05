// WasmExecution.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WasmExecution -- WebAssembly 1.0 Execution Engine
// ============================================================================
//
// This is a COMPLETE WASM execution engine built on top of the GenericVM.
// It includes:
//   - WasmValue: typed WASM values (i32, i64, f32, f64)
//   - LinearMemory: byte-addressable heap with page-based growth
//   - Table: function reference table for indirect calls
//   - TrapError: unrecoverable WASM runtime errors
//   - Bytecode decoder: converts variable-length WASM bytecodes
//   - Constant expression evaluator: for global/data/element init
//   - All instruction handlers: numeric, variable, memory, control flow
//   - WasmExecutionEngine: the main interpreter
//
// ============================================================================
// Architecture: Recursive Interpreter
// ============================================================================
//
// Function calls are handled recursively. When a `call` instruction is
// executed, the WasmExecutionEngine decodes the callee's body, builds a
// new execution context, and calls itself recursively. This mirrors how
// real interpreters work and avoids the complexity of inline code switching.
//
// The GenericVM provides the operand stack and PC. The WasmExecutionContext
// carries per-execution state: locals, label stack, control flow map, etc.
// The WasmExecutionEngine ties everything together.
//
// ============================================================================

import Foundation
import WasmLeb128
import WasmTypes
import WasmOpcodes
import VirtualMachine

// ============================================================================
// MARK: - Public Module Type (for scaffold compatibility)
// ============================================================================

/// A namespace type for the WasmExecution module.
public struct WasmExecution {
    public init() {}
}

// ============================================================================
// MARK: - TrapError
// ============================================================================

/// An unrecoverable WASM runtime error (a "trap").
///
/// Traps occur when the program does something illegal at runtime:
/// division by zero, out-of-bounds memory access, integer overflow in
/// division, calling an undefined function, etc. Unlike exceptions in
/// most languages, WASM traps are unrecoverable -- the current
/// execution terminates immediately.
public class TrapError: Error, CustomStringConvertible {
    public let message: String
    public init(_ message: String) { self.message = message }
    public var description: String { "TrapError: \(message)" }
}

// ============================================================================
// MARK: - WasmValue
// ============================================================================

/// A typed WASM value: a numeric payload tagged with its ValueType.
///
/// WASM has exactly four value types. Every value on the operand stack,
/// every local variable, and every global has one of these types.
///
///   +---------+----------------------------+
///   | Variant | Swift Type                 |
///   +---------+----------------------------+
///   | i32     | Int32  (32-bit integer)    |
///   | i64     | Int64  (64-bit integer)    |
///   | f32     | Float  (32-bit float)      |
///   | f64     | Double (64-bit float)      |
///   +---------+----------------------------+
public enum WasmValue: Equatable {
    case i32(Int32)
    case i64(Int64)
    case f32(Float)
    case f64(Double)

    /// The ValueType tag for this value.
    public var type: ValueType {
        switch self {
        case .i32: return .i32
        case .i64: return .i64
        case .f32: return .f32
        case .f64: return .f64
        }
    }

    /// Convert to TypedVMValue for the GenericVM typed stack.
    public var typed: TypedVMValue {
        switch self {
        case .i32(let v): return TypedVMValue(type: ValueType.i32.rawValue, value: .int32(v))
        case .i64(let v): return TypedVMValue(type: ValueType.i64.rawValue, value: .int64(v))
        case .f32(let v): return TypedVMValue(type: ValueType.f32.rawValue, value: .float(v))
        case .f64(let v): return TypedVMValue(type: ValueType.f64.rawValue, value: .double(v))
        }
    }

    /// Create from TypedVMValue.
    public static func fromTyped(_ tv: TypedVMValue) -> WasmValue {
        switch tv.type {
        case ValueType.i32.rawValue:
            if case .int32(let v) = tv.value { return .i32(v) }
            if case .int(let v) = tv.value { return .i32(Int32(v)) }
            return .i32(0)
        case ValueType.i64.rawValue:
            if case .int64(let v) = tv.value { return .i64(v) }
            if case .int(let v) = tv.value { return .i64(Int64(v)) }
            return .i64(0)
        case ValueType.f32.rawValue:
            if case .float(let v) = tv.value { return .f32(v) }
            return .f32(0)
        case ValueType.f64.rawValue:
            if case .double(let v) = tv.value { return .f64(v) }
            return .f64(0)
        default:
            return .i32(0)
        }
    }

    /// Create a zero-initialized value for a given type.
    public static func defaultValue(for type: ValueType) -> WasmValue {
        switch type {
        case .i32: return .i32(0)
        case .i64: return .i64(0)
        case .f32: return .f32(0)
        case .f64: return .f64(0)
        }
    }

    /// Extract as Int32, or trap.
    public func asI32() throws -> Int32 {
        if case .i32(let v) = self { return v }
        throw TrapError("Type mismatch: expected i32")
    }

    /// Extract as Int64, or trap.
    public func asI64() throws -> Int64 {
        if case .i64(let v) = self { return v }
        throw TrapError("Type mismatch: expected i64")
    }

    /// Extract as Float, or trap.
    public func asF32() throws -> Float {
        if case .f32(let v) = self { return v }
        throw TrapError("Type mismatch: expected f32")
    }

    /// Extract as Double, or trap.
    public func asF64() throws -> Double {
        if case .f64(let v) = self { return v }
        throw TrapError("Type mismatch: expected f64")
    }

    /// Get the numeric value as a Double (for convenience).
    public var numericValue: Double {
        switch self {
        case .i32(let v): return Double(v)
        case .i64(let v): return Double(v)
        case .f32(let v): return Double(v)
        case .f64(let v): return v
        }
    }
}

// ============================================================================
// MARK: - LinearMemory
// ============================================================================

/// WASM linear memory -- a contiguous, byte-addressable array.
///
/// Memory is organized in pages of 65536 bytes (64 KiB). A module declares
/// its initial page count and optional maximum. The `memory.grow` instruction
/// can expand memory at runtime.
///
///   Page 0: bytes [0 .. 65535]
///   Page 1: bytes [65536 .. 131071]
///   ...
///
/// All loads and stores are little-endian, matching the WASM spec.
public class LinearMemory {
    public static let PAGE_SIZE = 65536

    private var buffer: [UInt8]
    private var currentPages: Int
    private let maxPages: Int?

    public init(initialPages: Int, maxPages: Int? = nil) {
        self.currentPages = initialPages
        self.maxPages = maxPages
        self.buffer = [UInt8](repeating: 0, count: initialPages * LinearMemory.PAGE_SIZE)
    }

    private func boundsCheck(_ offset: Int, _ width: Int) throws {
        if offset < 0 || offset + width > buffer.count {
            throw TrapError("Out of bounds memory access: offset=\(offset), size=\(width), memory size=\(buffer.count)")
        }
    }

    // -- Full-width loads (little-endian) --

    public func loadI32(_ offset: Int) throws -> Int32 {
        try boundsCheck(offset, 4)
        var val: Int32 = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<4 { ptr[i] = buffer[offset + i] }
        }
        return val
    }

    public func loadI64(_ offset: Int) throws -> Int64 {
        try boundsCheck(offset, 8)
        var val: Int64 = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<8 { ptr[i] = buffer[offset + i] }
        }
        return val
    }

    public func loadF32(_ offset: Int) throws -> Float {
        try boundsCheck(offset, 4)
        var val: Float = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<4 { ptr[i] = buffer[offset + i] }
        }
        return val
    }

    public func loadF64(_ offset: Int) throws -> Double {
        try boundsCheck(offset, 8)
        var val: Double = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<8 { ptr[i] = buffer[offset + i] }
        }
        return val
    }

    // -- Narrow loads (sign- and zero-extending) --

    public func loadI32_8s(_ offset: Int) throws -> Int32 {
        try boundsCheck(offset, 1)
        return Int32(Int8(bitPattern: buffer[offset]))
    }
    public func loadI32_8u(_ offset: Int) throws -> Int32 {
        try boundsCheck(offset, 1)
        return Int32(buffer[offset])
    }
    public func loadI32_16s(_ offset: Int) throws -> Int32 {
        try boundsCheck(offset, 2)
        let lo = UInt16(buffer[offset])
        let hi = UInt16(buffer[offset + 1])
        return Int32(Int16(bitPattern: lo | (hi << 8)))
    }
    public func loadI32_16u(_ offset: Int) throws -> Int32 {
        try boundsCheck(offset, 2)
        let lo = UInt16(buffer[offset])
        let hi = UInt16(buffer[offset + 1])
        return Int32(lo | (hi << 8))
    }
    public func loadI64_8s(_ offset: Int) throws -> Int64 {
        try boundsCheck(offset, 1)
        return Int64(Int8(bitPattern: buffer[offset]))
    }
    public func loadI64_8u(_ offset: Int) throws -> Int64 {
        try boundsCheck(offset, 1)
        return Int64(buffer[offset])
    }
    public func loadI64_16s(_ offset: Int) throws -> Int64 {
        try boundsCheck(offset, 2)
        let lo = UInt16(buffer[offset])
        let hi = UInt16(buffer[offset + 1])
        return Int64(Int16(bitPattern: lo | (hi << 8)))
    }
    public func loadI64_16u(_ offset: Int) throws -> Int64 {
        try boundsCheck(offset, 2)
        let lo = UInt16(buffer[offset])
        let hi = UInt16(buffer[offset + 1])
        return Int64(lo | (hi << 8))
    }
    public func loadI64_32s(_ offset: Int) throws -> Int64 {
        try boundsCheck(offset, 4)
        var val: Int32 = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<4 { ptr[i] = buffer[offset + i] }
        }
        return Int64(val)
    }
    public func loadI64_32u(_ offset: Int) throws -> Int64 {
        try boundsCheck(offset, 4)
        var val: UInt32 = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<4 { ptr[i] = buffer[offset + i] }
        }
        return Int64(val)
    }

    // -- Full-width stores --

    public func storeI32(_ offset: Int, _ value: Int32) throws {
        try boundsCheck(offset, 4)
        withUnsafeBytes(of: value) { ptr in
            for i in 0..<4 { buffer[offset + i] = ptr[i] }
        }
    }

    public func storeI64(_ offset: Int, _ value: Int64) throws {
        try boundsCheck(offset, 8)
        withUnsafeBytes(of: value) { ptr in
            for i in 0..<8 { buffer[offset + i] = ptr[i] }
        }
    }

    public func storeF32(_ offset: Int, _ value: Float) throws {
        try boundsCheck(offset, 4)
        withUnsafeBytes(of: value) { ptr in
            for i in 0..<4 { buffer[offset + i] = ptr[i] }
        }
    }

    public func storeF64(_ offset: Int, _ value: Double) throws {
        try boundsCheck(offset, 8)
        withUnsafeBytes(of: value) { ptr in
            for i in 0..<8 { buffer[offset + i] = ptr[i] }
        }
    }

    // -- Narrow stores --

    public func storeI32_8(_ offset: Int, _ value: Int32) throws {
        try boundsCheck(offset, 1)
        buffer[offset] = UInt8(truncatingIfNeeded: value)
    }
    public func storeI32_16(_ offset: Int, _ value: Int32) throws {
        try boundsCheck(offset, 2)
        let v = UInt16(truncatingIfNeeded: value)
        buffer[offset] = UInt8(v & 0xFF)
        buffer[offset + 1] = UInt8(v >> 8)
    }
    public func storeI64_8(_ offset: Int, _ value: Int64) throws {
        try boundsCheck(offset, 1)
        buffer[offset] = UInt8(truncatingIfNeeded: value)
    }
    public func storeI64_16(_ offset: Int, _ value: Int64) throws {
        try boundsCheck(offset, 2)
        let v = UInt16(truncatingIfNeeded: value)
        buffer[offset] = UInt8(v & 0xFF)
        buffer[offset + 1] = UInt8(v >> 8)
    }
    public func storeI64_32(_ offset: Int, _ value: Int64) throws {
        try boundsCheck(offset, 4)
        let v = Int32(truncatingIfNeeded: value)
        withUnsafeBytes(of: v) { ptr in
            for i in 0..<4 { buffer[offset + i] = ptr[i] }
        }
    }

    // -- Growth --

    public func grow(_ deltaPages: Int) -> Int {
        let oldPages = currentPages
        let newPages = oldPages + deltaPages
        if let max = maxPages, newPages > max { return -1 }
        if newPages > 65536 { return -1 }
        buffer.append(contentsOf: [UInt8](repeating: 0, count: deltaPages * LinearMemory.PAGE_SIZE))
        currentPages = newPages
        return oldPages
    }

    public func size() -> Int { currentPages }
    public func byteLength() -> Int { buffer.count }

    public func writeBytes(_ offset: Int, _ data: [UInt8]) throws {
        try boundsCheck(offset, data.count)
        for i in 0..<data.count {
            buffer[offset + i] = data[i]
        }
    }
}

// ============================================================================
// MARK: - Table
// ============================================================================

/// A WASM table -- resizable array of nullable function indices.
///
/// In WASM 1.0 there is at most one table per module, and it stores
/// function references (funcref). Tables are used by `call_indirect`
/// to implement virtual dispatch and function pointers.
public class Table {
    private var elements: [Int?]
    private let maxSize: Int?

    public init(initialSize: Int, maxSize: Int? = nil) {
        self.elements = [Int?](repeating: nil, count: initialSize)
        self.maxSize = maxSize
    }

    public func get(_ index: Int) throws -> Int? {
        guard index >= 0 && index < elements.count else {
            throw TrapError("Out of bounds table access: index=\(index), size=\(elements.count)")
        }
        return elements[index]
    }

    public func set(_ index: Int, _ funcIndex: Int?) throws {
        guard index >= 0 && index < elements.count else {
            throw TrapError("Out of bounds table access: index=\(index), size=\(elements.count)")
        }
        elements[index] = funcIndex
    }

    public func tableSize() -> Int { elements.count }
}

// ============================================================================
// MARK: - HostFunction
// ============================================================================

/// A callable function provided by the host environment.
///
/// Host functions are the bridge between WASM and the outside world.
/// When a module imports a function, the runtime resolves it to a
/// HostFunction. When the WASM code calls it, the engine invokes
/// the closure with the arguments from the stack.
public struct HostFunction {
    public let type: FuncType
    public let call: ([WasmValue]) throws -> [WasmValue]

    public init(type: FuncType, call: @escaping ([WasmValue]) throws -> [WasmValue]) {
        self.type = type
        self.call = call
    }
}

/// The contract for resolving WASM imports.
///
/// A host interface provides implementations for imported functions,
/// globals, memories, and tables. The WASI stub implements this
/// protocol to provide system-call-like functions.
public protocol HostInterface: AnyObject {
    func resolveFunction(moduleName: String, name: String) -> HostFunction?
    func resolveGlobal(moduleName: String, name: String) -> (type: GlobalType, value: WasmValue)?
    func resolveMemory(moduleName: String, name: String) -> LinearMemory?
    func resolveTable(moduleName: String, name: String) -> Table?
}

// ============================================================================
// MARK: - Constant Expression Evaluator
// ============================================================================

/// Evaluate a WASM constant expression and return its result.
///
/// Constant expressions appear in globals, data segments, and element
/// segments. They are restricted to a tiny subset of instructions:
/// const, global.get, and end.
public func evaluateConstExpr(_ expr: [UInt8], globals: [WasmValue]) throws -> WasmValue {
    var result: WasmValue? = nil
    var pos = 0

    while pos < expr.count {
        let opcode = expr[pos]
        pos += 1

        switch opcode {
        case 0x41: // i32.const
            var decoder = LEB128Decoder(data: expr, offset: pos)
            let value = try decoder.decodeSigned32()
            pos = decoder.position
            result = .i32(value)

        case 0x42: // i64.const
            var decoder = LEB128Decoder(data: expr, offset: pos)
            let value = try decoder.decodeSigned64()
            pos = decoder.position
            result = .i64(value)

        case 0x43: // f32.const
            guard pos + 4 <= expr.count else {
                throw TrapError("f32.const: not enough bytes")
            }
            var val: Float = 0
            withUnsafeMutableBytes(of: &val) { ptr in
                for i in 0..<4 { ptr[i] = expr[pos + i] }
            }
            pos += 4
            result = .f32(val)

        case 0x44: // f64.const
            guard pos + 8 <= expr.count else {
                throw TrapError("f64.const: not enough bytes")
            }
            var val: Double = 0
            withUnsafeMutableBytes(of: &val) { ptr in
                for i in 0..<8 { ptr[i] = expr[pos + i] }
            }
            pos += 8
            result = .f64(val)

        case 0x23: // global.get
            var decoder = LEB128Decoder(data: expr, offset: pos)
            let idx = try decoder.decodeUnsigned32()
            pos = decoder.position
            guard Int(idx) < globals.count else {
                throw TrapError("global.get: index \(idx) out of bounds")
            }
            result = globals[Int(idx)]

        case 0x0B: // end
            guard let r = result else {
                throw TrapError("Constant expression produced no value")
            }
            return r

        default:
            throw TrapError("Illegal opcode 0x\(String(opcode, radix: 16)) in constant expression")
        }
    }

    throw TrapError("Constant expression missing end opcode")
}

// ============================================================================
// MARK: - Decoded Instruction
// ============================================================================

/// A decoded WASM instruction with its operand.
public struct DecodedInstruction {
    public let opcode: UInt8
    public let operand: Any?
    public let offset: Int
    public let size: Int
}

/// Memory argument for load/store instructions.
public struct MemArg {
    public let align: UInt32
    public let offset: UInt32
}

/// Branch table data for br_table.
public struct BrTableData {
    public let labels: [UInt32]
    public let defaultLabel: UInt32
}

// ============================================================================
// MARK: - Bytecode Decoder
// ============================================================================

/// Decode all instructions in a function body's bytecodes.
///
/// The function body is a sequence of variable-length instructions.
/// Each instruction is an opcode byte followed by zero or more
/// immediate operands. This decoder reads all instructions and
/// returns an array of DecodedInstruction objects.
public func decodeFunctionBody(_ body: FunctionBody) throws -> [DecodedInstruction] {
    let code = body.code
    var instructions: [DecodedInstruction] = []
    var offset = 0

    while offset < code.count {
        let startOffset = offset
        let opcodeByte = code[offset]
        offset += 1

        let info = getOpcode(opcodeByte)
        var operand: Any? = nil

        if let info = info {
            let (op, size) = try decodeImmediates(code, offset, info.immediates)
            operand = op
            offset += size
        }

        instructions.append(DecodedInstruction(
            opcode: opcodeByte, operand: operand,
            offset: startOffset, size: offset - startOffset
        ))
    }

    return instructions
}

private func decodeImmediates(_ code: [UInt8], _ offset: Int, _ immediates: [String]) throws -> (Any?, Int) {
    if immediates.isEmpty { return (nil, 0) }

    if immediates.count == 1 {
        return try decodeSingleImmediate(code, offset, immediates[0])
    }

    // Multiple immediates -- return as array.
    var results: [Any] = []
    var pos = offset
    for imm in immediates {
        let (value, size) = try decodeSingleImmediate(code, pos, imm)
        results.append(value as Any)
        pos += size
    }
    return (results, pos - offset)
}

private func decodeSingleImmediate(_ code: [UInt8], _ offset: Int, _ type: String) throws -> (Any?, Int) {
    switch type {
    case "i32":
        var decoder = LEB128Decoder(data: code, offset: offset)
        let value = try decoder.decodeSigned32()
        return (value, decoder.position - offset)

    case "labelidx", "funcidx", "typeidx", "localidx", "globalidx", "tableidx", "memidx":
        var decoder = LEB128Decoder(data: code, offset: offset)
        let value = try decoder.decodeUnsigned32()
        return (value, decoder.position - offset)

    case "i64":
        var decoder = LEB128Decoder(data: code, offset: offset)
        let value = try decoder.decodeSigned64()
        return (value, decoder.position - offset)

    case "f32":
        guard offset + 4 <= code.count else {
            throw TrapError("f32 immediate: not enough bytes")
        }
        var val: Float = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<4 { ptr[i] = code[offset + i] }
        }
        return (val, 4)

    case "f64":
        guard offset + 8 <= code.count else {
            throw TrapError("f64 immediate: not enough bytes")
        }
        var val: Double = 0
        withUnsafeMutableBytes(of: &val) { ptr in
            for i in 0..<8 { ptr[i] = code[offset + i] }
        }
        return (val, 8)

    case "blocktype":
        let byte = code[offset]
        if byte == 0x40 || byte == 0x7F || byte == 0x7E || byte == 0x7D || byte == 0x7C {
            return (byte, 1)
        }
        var decoder = LEB128Decoder(data: code, offset: offset)
        let value = try decoder.decodeSigned32()
        return (value, decoder.position - offset)

    case "memarg":
        var decoder = LEB128Decoder(data: code, offset: offset)
        let align = try decoder.decodeUnsigned32()
        let memOffset = try decoder.decodeUnsigned32()
        return (MemArg(align: align, offset: memOffset), decoder.position - offset)

    case "vec_labelidx":
        var decoder = LEB128Decoder(data: code, offset: offset)
        let count = try decoder.decodeUnsigned32()
        var labels: [UInt32] = []
        for _ in 0..<count {
            labels.append(try decoder.decodeUnsigned32())
        }
        let defaultLabel = try decoder.decodeUnsigned32()
        return (BrTableData(labels: labels, defaultLabel: defaultLabel), decoder.position - offset)

    default:
        return (nil, 0)
    }
}

// ============================================================================
// MARK: - Control Flow Map
// ============================================================================

/// A control flow map entry.
///
/// Every block/loop/if instruction is paired with its `end` instruction.
/// The control flow map records these pairings so that branching and
/// condition-false jumps can quickly find their target PC.
public struct ControlTarget {
    public let endPc: Int
    public let elsePc: Int?
}

/// Build the control flow map for a function's decoded instructions.
///
/// Walks the instruction list once and pairs every block/loop/if with
/// its corresponding end (and else, for if blocks). Returns a dictionary
/// from the opener's PC to its target PCs.
public func buildControlFlowMap(_ instructions: [DecodedInstruction]) -> [Int: ControlTarget] {
    var map = [Int: ControlTarget]()
    var stack: [(index: Int, opcode: UInt8, elsePc: Int?)] = []

    for i in 0..<instructions.count {
        let instr = instructions[i]

        switch instr.opcode {
        case 0x02, 0x03, 0x04: // block, loop, if
            stack.append((index: i, opcode: instr.opcode, elsePc: nil))

        case 0x05: // else
            if !stack.isEmpty {
                stack[stack.count - 1].elsePc = i
            }

        case 0x0B: // end
            if !stack.isEmpty {
                let opener = stack.removeLast()
                map[opener.index] = ControlTarget(endPc: i, elsePc: opener.elsePc)
            }

        default:
            break
        }
    }

    return map
}

// ============================================================================
// MARK: - Label (for structured control flow)
// ============================================================================

/// A label on the label stack.
///
/// Labels are pushed when entering a block/loop/if and popped when
/// reaching the corresponding end. They record:
/// - arity: how many values the block produces
/// - targetPc: where to jump on branch (end for blocks, start for loops)
/// - stackHeight: the operand stack height when the label was pushed
/// - isLoop: whether this is a loop (affects branch semantics)
public struct Label {
    public let arity: Int
    public let targetPc: Int
    public let stackHeight: Int
    public let isLoop: Bool
}

// ============================================================================
// MARK: - WasmExecutionContext
// ============================================================================

/// The per-execution context passed to all WASM instruction handlers.
///
/// This carries the mutable per-function state that instruction handlers
/// need: locals, label stack, control flow map. It also holds references
/// to the shared module state: memory, tables, globals, function types
/// and bodies.
public class WasmExecutionContext {
    public var memory: LinearMemory?
    public var tables: [Table]
    public var globals: [WasmValue]
    public var globalTypes: [GlobalType]
    public var funcTypes: [FuncType]
    public var funcBodies: [FunctionBody?]
    public var hostFunctions: [HostFunction?]
    public var typedLocals: [WasmValue]
    public var labelStack: [Label]
    public var controlFlowMap: [Int: ControlTarget]
    /// Reference to the engine for recursive calls.
    public weak var engine: WasmExecutionEngine?

    public init(memory: LinearMemory?, tables: [Table], globals: [WasmValue],
                globalTypes: [GlobalType], funcTypes: [FuncType],
                funcBodies: [FunctionBody?], hostFunctions: [HostFunction?],
                typedLocals: [WasmValue], controlFlowMap: [Int: ControlTarget]) {
        self.memory = memory
        self.tables = tables
        self.globals = globals
        self.globalTypes = globalTypes
        self.funcTypes = funcTypes
        self.funcBodies = funcBodies
        self.hostFunctions = hostFunctions
        self.typedLocals = typedLocals
        self.labelStack = []
        self.controlFlowMap = controlFlowMap
    }
}

// ============================================================================
// MARK: - Helper: Pop/Push WasmValue from VM
// ============================================================================

func popWasm(_ vm: GenericVM) -> WasmValue {
    return WasmValue.fromTyped(vm.popTyped())
}

func pushWasm(_ vm: GenericVM, _ value: WasmValue) {
    vm.pushTyped(value.typed)
}

// ============================================================================
// MARK: - Block Arity
// ============================================================================

/// Determine how many values a block produces.
///
/// Block types in WASM 1.0:
///   0x40 -> empty (0 results)
///   0x7F -> i32 (1 result)
///   0x7E -> i64 (1 result)
///   0x7D -> f32 (1 result)
///   0x7C -> f64 (1 result)
///   positive integer -> type index (multi-value, for future)
func blockArity(_ blockType: Any?, funcTypes: [FuncType]) -> Int {
    guard let bt = blockType else { return 0 }
    if let b = bt as? UInt8 {
        if b == 0x40 { return 0 }
        if b == 0x7F || b == 0x7E || b == 0x7D || b == 0x7C { return 1 }
    }
    if let idx = bt as? Int32, idx >= 0 && Int(idx) < funcTypes.count {
        return funcTypes[Int(idx)].results.count
    }
    return 0
}

// ============================================================================
// MARK: - Branch Execution
// ============================================================================

/// Execute a branch to a given label depth.
///
/// When branching:
/// 1. Look up the target label on the label stack
/// 2. Save the top N result values (where N = arity, 0 for loops)
/// 3. Unwind the operand stack to the label's saved height
/// 4. Push result values back
/// 5. Pop labels down to the target
/// 6. Jump to the target PC
func executeBranch(_ vm: GenericVM, _ ctx: WasmExecutionContext, _ labelIndex: Int) throws {
    let labelStackIndex = ctx.labelStack.count - 1 - labelIndex
    guard labelStackIndex >= 0 else {
        throw TrapError("branch target \(labelIndex) out of range")
    }

    let label = ctx.labelStack[labelStackIndex]
    let arity = label.isLoop ? 0 : label.arity

    // Save result values.
    var results: [WasmValue] = []
    for _ in 0..<arity {
        results.insert(popWasm(vm), at: 0)
    }

    // Unwind stack to label height.
    while vm.typedStack.count > label.stackHeight {
        _ = vm.popTyped()
    }

    // Push results back.
    for v in results {
        pushWasm(vm, v)
    }

    // Pop labels down to target.
    ctx.labelStack = Array(ctx.labelStack.prefix(labelStackIndex))

    // Jump. For non-loop blocks, jump past the end instruction (endPc + 1)
    // so the end handler doesn't try to pop an already-removed label.
    // For loops, jump to the loop header to re-enter the loop body.
    if label.isLoop {
        vm.jumpTo(label.targetPc)
    } else {
        vm.jumpTo(label.targetPc + 1)
    }
}

// ============================================================================
// MARK: - Instruction Registration
// ============================================================================

/// Register all WASM instruction handlers on a GenericVM.
///
/// This is the heart of the interpreter. Each opcode gets a closure that
/// implements its semantics. The closures pop operands from the stack,
/// perform the operation, and push the result.
func registerAllInstructions(_ vm: GenericVM) {
    registerNumericI32(vm)
    registerNumericI64(vm)
    registerNumericF32(vm)
    registerNumericF64(vm)
    registerConversions(vm)
    registerVariable(vm)
    registerParametric(vm)
    registerMemory(vm)
    registerControl(vm)
}

// ============================================================================
// MARK: - i32 Numeric Instructions
// ============================================================================

func registerNumericI32(_ vm: GenericVM) {
    // -- i32.const (0x41): push an i32 immediate --
    vm.registerContextOpcode(0x41) { vm, instr, code, ctxObj in
        let value: Int32
        if let v = instr.operand as? Int32 { value = v }
        else { value = 0 }
        pushWasm(vm, .i32(value))
    }

    // -- i32.eqz (0x45): push 1 if top == 0, else 0 --
    vm.registerContextOpcode(0x45) { vm, _, _, _ in
        let a = popWasm(vm)
        if case .i32(let v) = a { pushWasm(vm, .i32(v == 0 ? 1 : 0)) }
    }

    // Helper: signed binary comparison
    func i32BinCmp(_ vm: GenericVM, _ op: (Int32, Int32) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            pushWasm(vm, .i32(op(av, bv) ? 1 : 0))
        }
    }
    // Helper: unsigned binary comparison
    func i32BinCmpU(_ vm: GenericVM, _ op: (UInt32, UInt32) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            pushWasm(vm, .i32(op(UInt32(bitPattern: av), UInt32(bitPattern: bv)) ? 1 : 0))
        }
    }

    // Comparison instructions
    vm.registerContextOpcode(0x46) { vm, _, _, _ in i32BinCmp(vm) { $0 == $1 } }  // i32.eq
    vm.registerContextOpcode(0x47) { vm, _, _, _ in i32BinCmp(vm) { $0 != $1 } }  // i32.ne
    vm.registerContextOpcode(0x48) { vm, _, _, _ in i32BinCmp(vm) { $0 < $1 } }   // i32.lt_s
    vm.registerContextOpcode(0x49) { vm, _, _, _ in i32BinCmpU(vm) { $0 < $1 } }  // i32.lt_u
    vm.registerContextOpcode(0x4A) { vm, _, _, _ in i32BinCmp(vm) { $0 > $1 } }   // i32.gt_s
    vm.registerContextOpcode(0x4B) { vm, _, _, _ in i32BinCmpU(vm) { $0 > $1 } }  // i32.gt_u
    vm.registerContextOpcode(0x4C) { vm, _, _, _ in i32BinCmp(vm) { $0 <= $1 } }  // i32.le_s
    vm.registerContextOpcode(0x4D) { vm, _, _, _ in i32BinCmpU(vm) { $0 <= $1 } } // i32.le_u
    vm.registerContextOpcode(0x4E) { vm, _, _, _ in i32BinCmp(vm) { $0 >= $1 } }  // i32.ge_s
    vm.registerContextOpcode(0x4F) { vm, _, _, _ in i32BinCmpU(vm) { $0 >= $1 } } // i32.ge_u

    // Unary bit ops
    vm.registerContextOpcode(0x67) { vm, _, _, _ in  // i32.clz
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .i32(Int32(v.leadingZeroBitCount))) }
    }
    vm.registerContextOpcode(0x68) { vm, _, _, _ in  // i32.ctz
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .i32(Int32(v.trailingZeroBitCount))) }
    }
    vm.registerContextOpcode(0x69) { vm, _, _, _ in  // i32.popcnt
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .i32(Int32(v.nonzeroBitCount))) }
    }

    // Arithmetic -- Swift &+ &- &* for wrapping arithmetic
    vm.registerContextOpcode(0x6A) { vm, _, _, _ in  // i32.add
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av &+ bv)) }
    }
    vm.registerContextOpcode(0x6B) { vm, _, _, _ in  // i32.sub
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av &- bv)) }
    }
    vm.registerContextOpcode(0x6C) { vm, _, _, _ in  // i32.mul
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av &* bv)) }
    }
    vm.registerContextOpcode(0x6D) { vm, _, _, _ in  // i32.div_s
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            if bv == 0 { vm.halted = true; return }
            if av == Int32.min && bv == -1 { vm.halted = true; return }
            pushWasm(vm, .i32(av / bv))
        }
    }
    vm.registerContextOpcode(0x6E) { vm, _, _, _ in  // i32.div_u
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            let ua = UInt32(bitPattern: av); let ub = UInt32(bitPattern: bv)
            if ub == 0 { vm.halted = true; return }
            pushWasm(vm, .i32(Int32(bitPattern: ua / ub)))
        }
    }
    vm.registerContextOpcode(0x6F) { vm, _, _, _ in  // i32.rem_s
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            if bv == 0 { vm.halted = true; return }
            pushWasm(vm, .i32(av % bv))
        }
    }
    vm.registerContextOpcode(0x70) { vm, _, _, _ in  // i32.rem_u
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            let ua = UInt32(bitPattern: av); let ub = UInt32(bitPattern: bv)
            if ub == 0 { vm.halted = true; return }
            pushWasm(vm, .i32(Int32(bitPattern: ua % ub)))
        }
    }

    // Bitwise ops
    vm.registerContextOpcode(0x71) { vm, _, _, _ in  // i32.and
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av & bv)) }
    }
    vm.registerContextOpcode(0x72) { vm, _, _, _ in  // i32.or
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av | bv)) }
    }
    vm.registerContextOpcode(0x73) { vm, _, _, _ in  // i32.xor
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av ^ bv)) }
    }
    vm.registerContextOpcode(0x74) { vm, _, _, _ in  // i32.shl
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av &<< (bv & 31))) }
    }
    vm.registerContextOpcode(0x75) { vm, _, _, _ in  // i32.shr_s
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b { pushWasm(vm, .i32(av &>> (bv & 31))) }
    }
    vm.registerContextOpcode(0x76) { vm, _, _, _ in  // i32.shr_u
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            let ua = UInt32(bitPattern: av)
            pushWasm(vm, .i32(Int32(bitPattern: ua >> (UInt32(bitPattern: bv) & 31))))
        }
    }
    vm.registerContextOpcode(0x77) { vm, _, _, _ in  // i32.rotl
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            let ua = UInt32(bitPattern: av)
            let shift = UInt32(bitPattern: bv) & 31
            pushWasm(vm, .i32(Int32(bitPattern: (ua << shift) | (ua >> (32 &- shift)))))
        }
    }
    vm.registerContextOpcode(0x78) { vm, _, _, _ in  // i32.rotr
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            let ua = UInt32(bitPattern: av)
            let shift = UInt32(bitPattern: bv) & 31
            pushWasm(vm, .i32(Int32(bitPattern: (ua >> shift) | (ua << (32 &- shift)))))
        }
    }
}

// ============================================================================
// MARK: - i64 Numeric Instructions
// ============================================================================

func registerNumericI64(_ vm: GenericVM) {
    // -- i64.const (0x42) --
    vm.registerContextOpcode(0x42) { vm, instr, _, _ in
        let value: Int64
        if let v = instr.operand as? Int64 { value = v }
        else { value = 0 }
        pushWasm(vm, .i64(value))
    }

    // -- i64.eqz (0x50) --
    vm.registerContextOpcode(0x50) { vm, _, _, _ in
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .i32(v == 0 ? 1 : 0)) }
    }

    // Helper: signed binary comparison
    func i64BinCmp(_ vm: GenericVM, _ op: (Int64, Int64) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            pushWasm(vm, .i32(op(av, bv) ? 1 : 0))
        }
    }
    func i64BinCmpU(_ vm: GenericVM, _ op: (UInt64, UInt64) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            pushWasm(vm, .i32(op(UInt64(bitPattern: av), UInt64(bitPattern: bv)) ? 1 : 0))
        }
    }

    vm.registerContextOpcode(0x51) { vm, _, _, _ in i64BinCmp(vm) { $0 == $1 } }   // i64.eq
    vm.registerContextOpcode(0x52) { vm, _, _, _ in i64BinCmp(vm) { $0 != $1 } }   // i64.ne
    vm.registerContextOpcode(0x53) { vm, _, _, _ in i64BinCmp(vm) { $0 < $1 } }    // i64.lt_s
    vm.registerContextOpcode(0x54) { vm, _, _, _ in i64BinCmpU(vm) { $0 < $1 } }   // i64.lt_u
    vm.registerContextOpcode(0x55) { vm, _, _, _ in i64BinCmp(vm) { $0 > $1 } }    // i64.gt_s
    vm.registerContextOpcode(0x56) { vm, _, _, _ in i64BinCmpU(vm) { $0 > $1 } }   // i64.gt_u
    vm.registerContextOpcode(0x57) { vm, _, _, _ in i64BinCmp(vm) { $0 <= $1 } }   // i64.le_s
    vm.registerContextOpcode(0x58) { vm, _, _, _ in i64BinCmpU(vm) { $0 <= $1 } }  // i64.le_u
    vm.registerContextOpcode(0x59) { vm, _, _, _ in i64BinCmp(vm) { $0 >= $1 } }   // i64.ge_s
    vm.registerContextOpcode(0x5A) { vm, _, _, _ in i64BinCmpU(vm) { $0 >= $1 } }  // i64.ge_u

    // Unary bit ops
    vm.registerContextOpcode(0x79) { vm, _, _, _ in  // i64.clz
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .i64(Int64(v.leadingZeroBitCount))) }
    }
    vm.registerContextOpcode(0x7A) { vm, _, _, _ in  // i64.ctz
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .i64(Int64(v.trailingZeroBitCount))) }
    }
    vm.registerContextOpcode(0x7B) { vm, _, _, _ in  // i64.popcnt
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .i64(Int64(v.nonzeroBitCount))) }
    }

    // Arithmetic (wrapping with &+ &- &*)
    vm.registerContextOpcode(0x7C) { vm, _, _, _ in  // i64.add
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &+ bv)) }
    }
    vm.registerContextOpcode(0x7D) { vm, _, _, _ in  // i64.sub
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &- bv)) }
    }
    vm.registerContextOpcode(0x7E) { vm, _, _, _ in  // i64.mul
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &* bv)) }
    }
    vm.registerContextOpcode(0x7F) { vm, _, _, _ in  // i64.div_s
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            if bv == 0 { vm.halted = true; return }
            if av == Int64.min && bv == -1 { vm.halted = true; return }
            pushWasm(vm, .i64(av / bv))
        }
    }
    vm.registerContextOpcode(0x80) { vm, _, _, _ in  // i64.div_u
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            let ua = UInt64(bitPattern: av); let ub = UInt64(bitPattern: bv)
            if ub == 0 { vm.halted = true; return }
            pushWasm(vm, .i64(Int64(bitPattern: ua / ub)))
        }
    }
    vm.registerContextOpcode(0x81) { vm, _, _, _ in  // i64.rem_s
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            if bv == 0 { vm.halted = true; return }
            pushWasm(vm, .i64(av % bv))
        }
    }
    vm.registerContextOpcode(0x82) { vm, _, _, _ in  // i64.rem_u
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            let ua = UInt64(bitPattern: av); let ub = UInt64(bitPattern: bv)
            if ub == 0 { vm.halted = true; return }
            pushWasm(vm, .i64(Int64(bitPattern: ua % ub)))
        }
    }

    // Bitwise
    vm.registerContextOpcode(0x83) { vm, _, _, _ in  // i64.and
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av & bv)) }
    }
    vm.registerContextOpcode(0x84) { vm, _, _, _ in  // i64.or
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av | bv)) }
    }
    vm.registerContextOpcode(0x85) { vm, _, _, _ in  // i64.xor
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av ^ bv)) }
    }
    vm.registerContextOpcode(0x86) { vm, _, _, _ in  // i64.shl
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &<< (bv & 63))) }
    }
    vm.registerContextOpcode(0x87) { vm, _, _, _ in  // i64.shr_s
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &>> (bv & 63))) }
    }
    vm.registerContextOpcode(0x88) { vm, _, _, _ in  // i64.shr_u
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            let ua = UInt64(bitPattern: av)
            pushWasm(vm, .i64(Int64(bitPattern: ua >> (UInt64(bitPattern: bv) & 63))))
        }
    }
    vm.registerContextOpcode(0x89) { vm, _, _, _ in  // i64.rotl
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            let ua = UInt64(bitPattern: av)
            let shift = UInt64(bitPattern: bv) & 63
            pushWasm(vm, .i64(Int64(bitPattern: (ua << shift) | (ua >> (64 &- shift)))))
        }
    }
    vm.registerContextOpcode(0x8A) { vm, _, _, _ in  // i64.rotr
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i64(let av) = a, case .i64(let bv) = b {
            let ua = UInt64(bitPattern: av)
            let shift = UInt64(bitPattern: bv) & 63
            pushWasm(vm, .i64(Int64(bitPattern: (ua >> shift) | (ua << (64 &- shift)))))
        }
    }
}

// ============================================================================
// MARK: - f32 Numeric Instructions
// ============================================================================

func registerNumericF32(_ vm: GenericVM) {
    vm.registerContextOpcode(0x43) { vm, instr, _, _ in  // f32.const
        if let v = instr.operand as? Float { pushWasm(vm, .f32(v)) }
        else { pushWasm(vm, .f32(0)) }
    }

    // Comparisons (f32 comparisons return i32)
    func f32Cmp(_ vm: GenericVM, _ op: (Float, Float) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b {
            pushWasm(vm, .i32(op(av, bv) ? 1 : 0))
        }
    }
    vm.registerContextOpcode(0x5B) { vm, _, _, _ in f32Cmp(vm) { $0 == $1 } }  // f32.eq
    vm.registerContextOpcode(0x5C) { vm, _, _, _ in f32Cmp(vm) { $0 != $1 } }  // f32.ne
    vm.registerContextOpcode(0x5D) { vm, _, _, _ in f32Cmp(vm) { $0 < $1 } }   // f32.lt
    vm.registerContextOpcode(0x5E) { vm, _, _, _ in f32Cmp(vm) { $0 > $1 } }   // f32.gt
    vm.registerContextOpcode(0x5F) { vm, _, _, _ in f32Cmp(vm) { $0 <= $1 } }  // f32.le
    vm.registerContextOpcode(0x60) { vm, _, _, _ in f32Cmp(vm) { $0 >= $1 } }  // f32.ge

    // Unary
    vm.registerContextOpcode(0x8B) { vm, _, _, _ in if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f32(abs(v))) } }   // f32.abs
    vm.registerContextOpcode(0x8C) { vm, _, _, _ in if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f32(-v)) } }        // f32.neg
    vm.registerContextOpcode(0x8D) { vm, _, _, _ in if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f32(ceilf(v))) } }  // f32.ceil
    vm.registerContextOpcode(0x8E) { vm, _, _, _ in if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f32(floorf(v))) } } // f32.floor
    vm.registerContextOpcode(0x8F) { vm, _, _, _ in if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f32(truncf(v))) } } // f32.trunc
    vm.registerContextOpcode(0x90) { vm, _, _, _ in if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f32(nearbyintf(v))) } } // f32.nearest
    vm.registerContextOpcode(0x91) { vm, _, _, _ in if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f32(sqrtf(v))) } }  // f32.sqrt

    // Binary arithmetic
    vm.registerContextOpcode(0x92) { vm, _, _, _ in  // f32.add
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av + bv)) }
    }
    vm.registerContextOpcode(0x93) { vm, _, _, _ in  // f32.sub
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av - bv)) }
    }
    vm.registerContextOpcode(0x94) { vm, _, _, _ in  // f32.mul
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av * bv)) }
    }
    vm.registerContextOpcode(0x95) { vm, _, _, _ in  // f32.div
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av / bv)) }
    }
    vm.registerContextOpcode(0x96) { vm, _, _, _ in  // f32.min
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(Float.minimum(av, bv))) }
    }
    vm.registerContextOpcode(0x97) { vm, _, _, _ in  // f32.max
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(Float.maximum(av, bv))) }
    }
    vm.registerContextOpcode(0x98) { vm, _, _, _ in  // f32.copysign
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(Float(sign: bv.sign, exponent: av.exponent, significand: av.significand))) }
    }
}

// ============================================================================
// MARK: - f64 Numeric Instructions
// ============================================================================

func registerNumericF64(_ vm: GenericVM) {
    vm.registerContextOpcode(0x44) { vm, instr, _, _ in  // f64.const
        if let v = instr.operand as? Double { pushWasm(vm, .f64(v)) }
        else { pushWasm(vm, .f64(0)) }
    }

    func f64Cmp(_ vm: GenericVM, _ op: (Double, Double) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f64(let av) = a, case .f64(let bv) = b {
            pushWasm(vm, .i32(op(av, bv) ? 1 : 0))
        }
    }
    vm.registerContextOpcode(0x61) { vm, _, _, _ in f64Cmp(vm) { $0 == $1 } }  // f64.eq
    vm.registerContextOpcode(0x62) { vm, _, _, _ in f64Cmp(vm) { $0 != $1 } }  // f64.ne
    vm.registerContextOpcode(0x63) { vm, _, _, _ in f64Cmp(vm) { $0 < $1 } }   // f64.lt
    vm.registerContextOpcode(0x64) { vm, _, _, _ in f64Cmp(vm) { $0 > $1 } }   // f64.gt
    vm.registerContextOpcode(0x65) { vm, _, _, _ in f64Cmp(vm) { $0 <= $1 } }  // f64.le
    vm.registerContextOpcode(0x66) { vm, _, _, _ in f64Cmp(vm) { $0 >= $1 } }  // f64.ge

    // Unary
    vm.registerContextOpcode(0x99) { vm, _, _, _ in if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f64(abs(v))) } }       // f64.abs
    vm.registerContextOpcode(0x9A) { vm, _, _, _ in if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f64(-v)) } }            // f64.neg
    vm.registerContextOpcode(0x9B) { vm, _, _, _ in if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f64(ceil(v))) } }       // f64.ceil
    vm.registerContextOpcode(0x9C) { vm, _, _, _ in if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f64(floor(v))) } }      // f64.floor
    vm.registerContextOpcode(0x9D) { vm, _, _, _ in if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f64(trunc(v))) } }      // f64.trunc
    vm.registerContextOpcode(0x9E) { vm, _, _, _ in if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f64(nearbyint(v))) } }  // f64.nearest
    vm.registerContextOpcode(0x9F) { vm, _, _, _ in if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f64(sqrt(v))) } }       // f64.sqrt

    // Binary
    vm.registerContextOpcode(0xA0) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av + bv)) } }  // f64.add
    vm.registerContextOpcode(0xA1) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av - bv)) } }  // f64.sub
    vm.registerContextOpcode(0xA2) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av * bv)) } }  // f64.mul
    vm.registerContextOpcode(0xA3) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av / bv)) } }  // f64.div
    vm.registerContextOpcode(0xA4) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(Double.minimum(av, bv))) } }  // f64.min
    vm.registerContextOpcode(0xA5) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(Double.maximum(av, bv))) } }  // f64.max
    vm.registerContextOpcode(0xA6) { vm, _, _, _ in  // f64.copysign
        let b = popWasm(vm); let a = popWasm(vm)
        if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(Double(sign: bv.sign, exponent: av.exponent, significand: av.significand))) }
    }
}

// ============================================================================
// MARK: - Conversion Instructions
// ============================================================================

func registerConversions(_ vm: GenericVM) {
    // i32.wrap_i64 (0xA7): truncate i64 to i32
    vm.registerContextOpcode(0xA7) { vm, _, _, _ in
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .i32(Int32(truncatingIfNeeded: v))) }
    }
    // i32.trunc_f32_s (0xA8)
    vm.registerContextOpcode(0xA8) { vm, _, _, _ in
        if case .f32(let v) = popWasm(vm) {
            if v.isNaN || v >= Float(Int32.max) + 1 || v < Float(Int32.min) { vm.halted = true; return }
            pushWasm(vm, .i32(Int32(v)))
        }
    }
    // i32.trunc_f32_u (0xA9)
    vm.registerContextOpcode(0xA9) { vm, _, _, _ in
        if case .f32(let v) = popWasm(vm) {
            if v.isNaN || v >= Float(UInt32.max) + 1 || v < 0 { vm.halted = true; return }
            pushWasm(vm, .i32(Int32(bitPattern: UInt32(v))))
        }
    }
    // i32.trunc_f64_s (0xAA)
    vm.registerContextOpcode(0xAA) { vm, _, _, _ in
        if case .f64(let v) = popWasm(vm) {
            if v.isNaN || v >= Double(Int32.max) + 1 || v < Double(Int32.min) { vm.halted = true; return }
            pushWasm(vm, .i32(Int32(v)))
        }
    }
    // i32.trunc_f64_u (0xAB)
    vm.registerContextOpcode(0xAB) { vm, _, _, _ in
        if case .f64(let v) = popWasm(vm) {
            if v.isNaN || v >= Double(UInt32.max) + 1 || v < 0 { vm.halted = true; return }
            pushWasm(vm, .i32(Int32(bitPattern: UInt32(v))))
        }
    }
    // i64.extend_i32_s (0xAC)
    vm.registerContextOpcode(0xAC) { vm, _, _, _ in
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .i64(Int64(v))) }
    }
    // i64.extend_i32_u (0xAD)
    vm.registerContextOpcode(0xAD) { vm, _, _, _ in
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .i64(Int64(UInt32(bitPattern: v)))) }
    }
    // i64.trunc_f32_s (0xAE)
    vm.registerContextOpcode(0xAE) { vm, _, _, _ in
        if case .f32(let v) = popWasm(vm) {
            if v.isNaN { vm.halted = true; return }
            pushWasm(vm, .i64(Int64(v)))
        }
    }
    // i64.trunc_f32_u (0xAF)
    vm.registerContextOpcode(0xAF) { vm, _, _, _ in
        if case .f32(let v) = popWasm(vm) {
            if v.isNaN || v < 0 { vm.halted = true; return }
            pushWasm(vm, .i64(Int64(bitPattern: UInt64(v))))
        }
    }
    // i64.trunc_f64_s (0xB0)
    vm.registerContextOpcode(0xB0) { vm, _, _, _ in
        if case .f64(let v) = popWasm(vm) {
            if v.isNaN { vm.halted = true; return }
            pushWasm(vm, .i64(Int64(v)))
        }
    }
    // i64.trunc_f64_u (0xB1)
    vm.registerContextOpcode(0xB1) { vm, _, _, _ in
        if case .f64(let v) = popWasm(vm) {
            if v.isNaN || v < 0 { vm.halted = true; return }
            pushWasm(vm, .i64(Int64(bitPattern: UInt64(v))))
        }
    }
    // f32.convert_i32_s (0xB2)
    vm.registerContextOpcode(0xB2) { vm, _, _, _ in
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .f32(Float(v))) }
    }
    // f32.convert_i32_u (0xB3)
    vm.registerContextOpcode(0xB3) { vm, _, _, _ in
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .f32(Float(UInt32(bitPattern: v)))) }
    }
    // f32.convert_i64_s (0xB4)
    vm.registerContextOpcode(0xB4) { vm, _, _, _ in
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .f32(Float(v))) }
    }
    // f32.convert_i64_u (0xB5)
    vm.registerContextOpcode(0xB5) { vm, _, _, _ in
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .f32(Float(UInt64(bitPattern: v)))) }
    }
    // f32.demote_f64 (0xB6)
    vm.registerContextOpcode(0xB6) { vm, _, _, _ in
        if case .f64(let v) = popWasm(vm) { pushWasm(vm, .f32(Float(v))) }
    }
    // f64.convert_i32_s (0xB7)
    vm.registerContextOpcode(0xB7) { vm, _, _, _ in
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .f64(Double(v))) }
    }
    // f64.convert_i32_u (0xB8)
    vm.registerContextOpcode(0xB8) { vm, _, _, _ in
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .f64(Double(UInt32(bitPattern: v)))) }
    }
    // f64.convert_i64_s (0xB9)
    vm.registerContextOpcode(0xB9) { vm, _, _, _ in
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .f64(Double(v))) }
    }
    // f64.convert_i64_u (0xBA)
    vm.registerContextOpcode(0xBA) { vm, _, _, _ in
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .f64(Double(UInt64(bitPattern: v)))) }
    }
    // f64.promote_f32 (0xBB)
    vm.registerContextOpcode(0xBB) { vm, _, _, _ in
        if case .f32(let v) = popWasm(vm) { pushWasm(vm, .f64(Double(v))) }
    }
    // i32.reinterpret_f32 (0xBC)
    vm.registerContextOpcode(0xBC) { vm, _, _, _ in
        if case .f32(let v) = popWasm(vm) { pushWasm(vm, .i32(Int32(bitPattern: v.bitPattern))) }
    }
    // i64.reinterpret_f64 (0xBD)
    vm.registerContextOpcode(0xBD) { vm, _, _, _ in
        if case .f64(let v) = popWasm(vm) { pushWasm(vm, .i64(Int64(bitPattern: v.bitPattern))) }
    }
    // f32.reinterpret_i32 (0xBE)
    vm.registerContextOpcode(0xBE) { vm, _, _, _ in
        if case .i32(let v) = popWasm(vm) { pushWasm(vm, .f32(Float(bitPattern: UInt32(bitPattern: v)))) }
    }
    // f64.reinterpret_i64 (0xBF)
    vm.registerContextOpcode(0xBF) { vm, _, _, _ in
        if case .i64(let v) = popWasm(vm) { pushWasm(vm, .f64(Double(bitPattern: UInt64(bitPattern: v)))) }
    }
}

// ============================================================================
// MARK: - Variable Access Instructions
// ============================================================================

func registerVariable(_ vm: GenericVM) {
    // local.get (0x20)
    vm.registerContextOpcode(0x20) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let idx = instr.operand as! UInt32
        pushWasm(vm, ctx.typedLocals[Int(idx)])
    }
    // local.set (0x21)
    vm.registerContextOpcode(0x21) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let idx = instr.operand as! UInt32
        ctx.typedLocals[Int(idx)] = popWasm(vm)
    }
    // local.tee (0x22)
    vm.registerContextOpcode(0x22) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let idx = instr.operand as! UInt32
        let val = popWasm(vm)
        ctx.typedLocals[Int(idx)] = val
        pushWasm(vm, val)
    }
    // global.get (0x23)
    vm.registerContextOpcode(0x23) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let idx = instr.operand as! UInt32
        pushWasm(vm, ctx.globals[Int(idx)])
    }
    // global.set (0x24)
    vm.registerContextOpcode(0x24) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let idx = instr.operand as! UInt32
        ctx.globals[Int(idx)] = popWasm(vm)
    }
}

// ============================================================================
// MARK: - Parametric Instructions
// ============================================================================

func registerParametric(_ vm: GenericVM) {
    // drop (0x1A): discard the top stack value
    vm.registerContextOpcode(0x1A) { vm, _, _, _ in
        _ = popWasm(vm)
    }
    // select (0x1B): ternary selection
    vm.registerContextOpcode(0x1B) { vm, _, _, _ in
        let cond = popWasm(vm)
        let val2 = popWasm(vm)
        let val1 = popWasm(vm)
        if case .i32(let c) = cond {
            pushWasm(vm, c != 0 ? val1 : val2)
        } else {
            pushWasm(vm, val1)
        }
    }
}

// ============================================================================
// MARK: - Memory Instructions
// ============================================================================

func registerMemory(_ vm: GenericVM) {
    // Helper to compute effective address from base + memarg offset
    func effectiveAddr(_ base: WasmValue, _ memarg: MemArg) -> Int {
        if case .i32(let b) = base {
            return Int(UInt32(bitPattern: b)) + Int(memarg.offset)
        }
        return 0
    }

    // -- Loads --
    vm.registerContextOpcode(0x28) { vm, instr, _, ctxObj in  // i32.load
        let ctx = ctxObj as! WasmExecutionContext
        guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg
        let addr = effectiveAddr(popWasm(vm), memarg)
        do { pushWasm(vm, .i32(try mem.loadI32(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x29) { vm, instr, _, ctxObj in  // i64.load
        let ctx = ctxObj as! WasmExecutionContext
        guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg
        let addr = effectiveAddr(popWasm(vm), memarg)
        do { pushWasm(vm, .i64(try mem.loadI64(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x2A) { vm, instr, _, ctxObj in  // f32.load
        let ctx = ctxObj as! WasmExecutionContext
        guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg
        let addr = effectiveAddr(popWasm(vm), memarg)
        do { pushWasm(vm, .f32(try mem.loadF32(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x2B) { vm, instr, _, ctxObj in  // f64.load
        let ctx = ctxObj as! WasmExecutionContext
        guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg
        let addr = effectiveAddr(popWasm(vm), memarg)
        do { pushWasm(vm, .f64(try mem.loadF64(addr))) } catch { vm.halted = true }
    }
    // Narrow loads
    vm.registerContextOpcode(0x2C) { vm, instr, _, ctxObj in  // i32.load8_s
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i32(try mem.loadI32_8s(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x2D) { vm, instr, _, ctxObj in  // i32.load8_u
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i32(try mem.loadI32_8u(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x2E) { vm, instr, _, ctxObj in  // i32.load16_s
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i32(try mem.loadI32_16s(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x2F) { vm, instr, _, ctxObj in  // i32.load16_u
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i32(try mem.loadI32_16u(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x30) { vm, instr, _, ctxObj in  // i64.load8_s
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i64(try mem.loadI64_8s(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x31) { vm, instr, _, ctxObj in  // i64.load8_u
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i64(try mem.loadI64_8u(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x32) { vm, instr, _, ctxObj in  // i64.load16_s
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i64(try mem.loadI64_16s(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x33) { vm, instr, _, ctxObj in  // i64.load16_u
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i64(try mem.loadI64_16u(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x34) { vm, instr, _, ctxObj in  // i64.load32_s
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i64(try mem.loadI64_32s(addr))) } catch { vm.halted = true }
    }
    vm.registerContextOpcode(0x35) { vm, instr, _, ctxObj in  // i64.load32_u
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let addr = effectiveAddr(popWasm(vm), instr.operand as! MemArg)
        do { pushWasm(vm, .i64(try mem.loadI64_32u(addr))) } catch { vm.halted = true }
    }

    // -- Stores --
    vm.registerContextOpcode(0x36) { vm, instr, _, ctxObj in  // i32.store
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        let addr = effectiveAddr(base, memarg)
        if case .i32(let v) = val { do { try mem.storeI32(addr, v) } catch { vm.halted = true } }
    }
    vm.registerContextOpcode(0x37) { vm, instr, _, ctxObj in  // i64.store
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        let addr = effectiveAddr(base, memarg)
        if case .i64(let v) = val { do { try mem.storeI64(addr, v) } catch { vm.halted = true } }
    }
    vm.registerContextOpcode(0x38) { vm, instr, _, ctxObj in  // f32.store
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        let addr = effectiveAddr(base, memarg)
        if case .f32(let v) = val { do { try mem.storeF32(addr, v) } catch { vm.halted = true } }
    }
    vm.registerContextOpcode(0x39) { vm, instr, _, ctxObj in  // f64.store
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        let addr = effectiveAddr(base, memarg)
        if case .f64(let v) = val { do { try mem.storeF64(addr, v) } catch { vm.halted = true } }
    }
    // Narrow stores
    vm.registerContextOpcode(0x3A) { vm, instr, _, ctxObj in  // i32.store8
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        if case .i32(let v) = val { do { try mem.storeI32_8(effectiveAddr(base, memarg), v) } catch { vm.halted = true } }
    }
    vm.registerContextOpcode(0x3B) { vm, instr, _, ctxObj in  // i32.store16
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        if case .i32(let v) = val { do { try mem.storeI32_16(effectiveAddr(base, memarg), v) } catch { vm.halted = true } }
    }
    vm.registerContextOpcode(0x3C) { vm, instr, _, ctxObj in  // i64.store8
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        if case .i64(let v) = val { do { try mem.storeI64_8(effectiveAddr(base, memarg), v) } catch { vm.halted = true } }
    }
    vm.registerContextOpcode(0x3D) { vm, instr, _, ctxObj in  // i64.store16
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        if case .i64(let v) = val { do { try mem.storeI64_16(effectiveAddr(base, memarg), v) } catch { vm.halted = true } }
    }
    vm.registerContextOpcode(0x3E) { vm, instr, _, ctxObj in  // i64.store32
        let ctx = ctxObj as! WasmExecutionContext; guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg; let val = popWasm(vm); let base = popWasm(vm)
        if case .i64(let v) = val { do { try mem.storeI64_32(effectiveAddr(base, memarg), v) } catch { vm.halted = true } }
    }

    // memory.size (0x3F)
    vm.registerContextOpcode(0x3F) { vm, _, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        pushWasm(vm, .i32(Int32(ctx.memory?.size() ?? 0)))
    }
    // memory.grow (0x40)
    vm.registerContextOpcode(0x40) { vm, _, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        if case .i32(let d) = popWasm(vm) {
            pushWasm(vm, .i32(Int32(ctx.memory?.grow(Int(d)) ?? -1)))
        }
    }
}

// ============================================================================
// MARK: - Control Flow Instructions
// ============================================================================

func registerControl(_ vm: GenericVM) {
    // unreachable (0x00)
    vm.registerContextOpcode(0x00) { vm, _, _, _ in vm.halted = true }

    // nop (0x01)
    vm.registerContextOpcode(0x01) { _, _, _, _ in }

    // block (0x02)
    vm.registerContextOpcode(0x02) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let arity = blockArity(instr.operand, funcTypes: ctx.funcTypes)
        // vm.pc is the current instruction index (auto-advanced after handler).
        let target = ctx.controlFlowMap[vm.pc]
        let endPc = target?.endPc ?? (vm.pc + 1)
        ctx.labelStack.append(Label(arity: arity, targetPc: endPc, stackHeight: vm.typedStack.count, isLoop: false))
    }

    // loop (0x03)
    vm.registerContextOpcode(0x03) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let arity = blockArity(instr.operand, funcTypes: ctx.funcTypes)
        // Loop branches jump back to the loop instruction itself.
        ctx.labelStack.append(Label(arity: arity, targetPc: vm.pc, stackHeight: vm.typedStack.count, isLoop: true))
    }

    // if (0x04)
    vm.registerContextOpcode(0x04) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let arity = blockArity(instr.operand, funcTypes: ctx.funcTypes)
        let cond = popWasm(vm)
        let target = ctx.controlFlowMap[vm.pc]
        let endPc = target?.endPc ?? (vm.pc + 1)
        let elsePc = target?.elsePc

        ctx.labelStack.append(Label(arity: arity, targetPc: endPc, stackHeight: vm.typedStack.count, isLoop: false))

        if case .i32(let c) = cond, c == 0 {
            if let ep = elsePc {
                vm.jumpTo(ep + 1)
            } else {
                vm.jumpTo(endPc)
            }
        }
    }

    // else (0x05)
    vm.registerContextOpcode(0x05) { vm, _, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        if let label = ctx.labelStack.last {
            vm.jumpTo(label.targetPc)
        }
    }

    // end (0x0B)
    vm.registerContextOpcode(0x0B) { vm, _, code, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext

        if !ctx.labelStack.isEmpty {
            let label = ctx.labelStack.removeLast()
            var results: [WasmValue] = []
            for _ in 0..<label.arity {
                results.insert(popWasm(vm), at: 0)
            }
            while vm.typedStack.count > label.stackHeight {
                _ = vm.popTyped()
            }
            for v in results { pushWasm(vm, v) }
        } else {
            // End of function -- halt the VM.
            vm.halted = true
        }
    }

    // br (0x0C)
    vm.registerContextOpcode(0x0C) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let labelIdx = instr.operand as! UInt32
        do { try executeBranch(vm, ctx, Int(labelIdx)) } catch { vm.halted = true }
    }

    // br_if (0x0D)
    vm.registerContextOpcode(0x0D) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let labelIdx = instr.operand as! UInt32
        let cond = popWasm(vm)
        if case .i32(let c) = cond, c != 0 {
            do { try executeBranch(vm, ctx, Int(labelIdx)) } catch { vm.halted = true }
        }
    }

    // br_table (0x0E)
    vm.registerContextOpcode(0x0E) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let brData = instr.operand as! BrTableData
        if case .i32(let i) = popWasm(vm) {
            let target: UInt32
            if i >= 0 && Int(i) < brData.labels.count {
                target = brData.labels[Int(i)]
            } else {
                target = brData.defaultLabel
            }
            do { try executeBranch(vm, ctx, Int(target)) } catch { vm.halted = true }
        }
    }

    // return (0x0F)
    vm.registerContextOpcode(0x0F) { vm, _, _, ctxObj in
        // Return halts the current function's execution.
        // The engine collects return values from the stack.
        vm.halted = true
    }

    // call (0x10) -- handled by the engine, not inline
    // The handler pops args, calls engine.callFunction recursively,
    // then pushes results back.
    vm.registerContextOpcode(0x10) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let funcIdx = instr.operand as! UInt32
        guard let engine = ctx.engine else { vm.halted = true; return }
        guard Int(funcIdx) < ctx.funcTypes.count else { vm.halted = true; return }
        let funcType = ctx.funcTypes[Int(funcIdx)]

        // Pop arguments from the stack (right to left).
        var args: [WasmValue] = []
        for _ in 0..<funcType.params.count {
            args.insert(popWasm(vm), at: 0)
        }

        // Call recursively through the engine.
        do {
            let results = try engine.callFunctionInternal(Int(funcIdx), args, globals: &ctx.globals)
            for r in results { pushWasm(vm, r) }
        } catch {
            vm.halted = true
        }
    }

    // call_indirect (0x11)
    vm.registerContextOpcode(0x11) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        guard let engine = ctx.engine else { vm.halted = true; return }

        // Operand is [typeidx, tableidx] as array.
        var typeIdx: UInt32 = 0
        if let arr = instr.operand as? [Any], let ti = arr[0] as? UInt32 {
            typeIdx = ti
        } else if let ti = instr.operand as? UInt32 {
            typeIdx = ti
        }

        // Pop the table index from the stack.
        let idxVal = popWasm(vm)
        guard case .i32(let tableIdx) = idxVal else { vm.halted = true; return }
        guard !ctx.tables.isEmpty else { vm.halted = true; return }

        do {
            guard let funcIdx = try ctx.tables[0].get(Int(tableIdx)) else {
                vm.halted = true; return
            }
            guard Int(typeIdx) < ctx.funcTypes.count else { vm.halted = true; return }
            let expectedType = ctx.funcTypes[Int(typeIdx)]
            let funcType = ctx.funcTypes[funcIdx]

            // Type check.
            guard expectedType.params == funcType.params && expectedType.results == funcType.results else {
                vm.halted = true; return
            }

            var args: [WasmValue] = []
            for _ in 0..<funcType.params.count {
                args.insert(popWasm(vm), at: 0)
            }

            let results = try engine.callFunctionInternal(funcIdx, args, globals: &ctx.globals)
            for r in results { pushWasm(vm, r) }
        } catch {
            vm.halted = true
        }
    }
}

// ============================================================================
// MARK: - WasmExecutionEngine
// ============================================================================

/// The WASM execution engine -- interprets validated WASM modules.
///
/// The engine owns a GenericVM and manages the recursive call mechanism.
/// Function calls are handled by decoding the callee's body, building
/// a new execution context, and running it on a fresh VM. Results flow
/// back through the return value of callFunction.
public class WasmExecutionEngine {
    private let memory: LinearMemory?
    private let tables: [Table]
    private var mutableGlobals: [WasmValue]
    private let globalTypes: [GlobalType]
    private let funcTypes: [FuncType]
    private let funcBodies: [FunctionBody?]
    private let hostFunctions: [HostFunction?]
    private var decodedCache: [Int: [DecodedInstruction]] = [:]
    private var callDepth: Int = 0
    private let maxCallDepth: Int = 1024

    public init(memory: LinearMemory?, tables: [Table], globals: [WasmValue],
                globalTypes: [GlobalType], funcTypes: [FuncType],
                funcBodies: [FunctionBody?], hostFunctions: [HostFunction?]) {
        self.memory = memory
        self.tables = tables
        self.mutableGlobals = globals
        self.globalTypes = globalTypes
        self.funcTypes = funcTypes
        self.funcBodies = funcBodies
        self.hostFunctions = hostFunctions
    }

    /// Call a WASM function by index (public entry point).
    public func callFunction(_ funcIndex: Int, _ args: [WasmValue]) throws -> [WasmValue] {
        callDepth = 0
        return try callFunctionInternal(funcIndex, args, globals: &mutableGlobals)
    }

    /// Internal recursive function call.
    ///
    /// This is called both from the public entry point and from the `call`
    /// instruction handler. It creates a new VM, registers all handlers,
    /// decodes the function body, and executes it.
    func callFunctionInternal(_ funcIndex: Int, _ args: [WasmValue], globals: inout [WasmValue]) throws -> [WasmValue] {
        guard funcIndex < funcTypes.count else {
            throw TrapError("undefined function index \(funcIndex)")
        }
        let funcType = funcTypes[funcIndex]

        guard args.count == funcType.params.count else {
            throw TrapError("function \(funcIndex) expects \(funcType.params.count) arguments, got \(args.count)")
        }

        callDepth += 1
        defer { callDepth -= 1 }
        if callDepth > maxCallDepth {
            throw TrapError("call stack depth exceeded")
        }

        // Check host function.
        if funcIndex < hostFunctions.count, let hostFunc = hostFunctions[funcIndex] {
            return try hostFunc.call(args)
        }

        // Module function.
        guard funcIndex < funcBodies.count, let body = funcBodies[funcIndex] else {
            throw TrapError("no body for function \(funcIndex)")
        }

        // Decode (cached).
        let decoded: [DecodedInstruction]
        if let cached = decodedCache[funcIndex] {
            decoded = cached
        } else {
            decoded = try decodeFunctionBody(body)
            decodedCache[funcIndex] = decoded
        }

        let cfMap = buildControlFlowMap(decoded)

        // Initialize locals: parameters + zero-initialized declared locals.
        var locals = args
        for vt in body.locals {
            locals.append(WasmValue.defaultValue(for: vt))
        }

        // Build context.
        let ctx = WasmExecutionContext(
            memory: memory, tables: tables, globals: globals,
            globalTypes: globalTypes, funcTypes: funcTypes,
            funcBodies: funcBodies, hostFunctions: hostFunctions,
            typedLocals: locals, controlFlowMap: cfMap
        )
        ctx.engine = self

        // Build code object from decoded instructions.
        let instructions = decoded.map { Instruction(opcode: $0.opcode, operand: $0.operand) }
        let code = CodeObject(instructions: instructions)

        // Create a fresh VM and register all handlers.
        let vm = GenericVM()
        registerAllInstructions(vm)

        // Execute.
        vm.executeWithContext(code, context: ctx)

        // Collect results from the VM's typed stack.
        var results: [WasmValue] = []
        for _ in 0..<funcType.results.count {
            if !vm.typedStack.isEmpty {
                results.insert(popWasm(vm), at: 0)
            }
        }

        // Propagate global mutations back.
        globals = ctx.globals

        return results
    }

    /// Get the current global values (for the runtime to read after execution).
    public var globals: [WasmValue] {
        return mutableGlobals
    }
}
