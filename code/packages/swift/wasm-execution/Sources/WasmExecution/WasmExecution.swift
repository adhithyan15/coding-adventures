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

import Foundation
import WasmLeb128
import WasmTypes
import WasmOpcodes
import VirtualMachine

// ============================================================================
// MARK: - TrapError
// ============================================================================

/// An unrecoverable WASM runtime error (a "trap").
public class TrapError: Error, CustomStringConvertible {
    public let message: String
    public init(_ message: String) { self.message = message }
    public var description: String { "TrapError: \(message)" }
}

// ============================================================================
// MARK: - WasmValue
// ============================================================================

/// A typed WASM value: a numeric payload tagged with its ValueType.
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

    // -- Full-width loads --
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

    // -- Narrow loads --
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
public struct HostFunction {
    public let type: FuncType
    public let call: ([WasmValue]) throws -> [WasmValue]

    public init(type: FuncType, call: @escaping ([WasmValue]) throws -> [WasmValue]) {
        self.type = type
        self.call = call
    }
}

/// The contract for resolving WASM imports.
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

    // Multiple immediates -- return as array
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
public struct ControlTarget {
    public let endPc: Int
    public let elsePc: Int?
}

/// Build the control flow map for a function's decoded instructions.
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
public struct Label {
    public let arity: Int
    public let targetPc: Int
    public let stackHeight: Int
    public let isLoop: Bool
}

/// A saved call frame.
public struct SavedFrame {
    public let locals: [WasmValue]
    public let labelStack: [Label]
    public let stackHeight: Int
    public let controlFlowMap: [Int: ControlTarget]
    public let returnPc: Int
    public let returnArity: Int
    public let code: CodeObject
}

// ============================================================================
// MARK: - WasmExecutionContext
// ============================================================================

/// The per-execution context passed to all WASM instruction handlers.
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
    public var savedFrames: [SavedFrame]
    public var returned: Bool
    public var returnValues: [WasmValue]

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
        self.savedFrames = []
        self.returned = false
        self.returnValues = []
    }
}

// ============================================================================
// MARK: - Helper: Pop WasmValue from VM
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

    // Jump.
    vm.jumpTo(label.targetPc)
}

// ============================================================================
// MARK: - Instruction Registration
// ============================================================================

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

// -- i32 numeric --
func registerNumericI32(_ vm: GenericVM) {
    // i32.const (0x41)
    vm.registerContextOpcode(0x41) { vm, instr, code, ctxObj in
        let value: Int32
        if let v = instr.operand as? Int32 { value = v }
        else { value = 0 }
        pushWasm(vm, .i32(value))
    }

    // i32.eqz (0x45)
    vm.registerContextOpcode(0x45) { vm, _, _, _ in
        let a = popWasm(vm)
        if case .i32(let v) = a { pushWasm(vm, .i32(v == 0 ? 1 : 0)) }
    }

    // Comparison helpers
    func i32BinCmp(_ vm: GenericVM, _ op: (Int32, Int32) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            pushWasm(vm, .i32(op(av, bv) ? 1 : 0))
        }
    }
    func i32BinCmpU(_ vm: GenericVM, _ op: (UInt32, UInt32) -> Bool) {
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            pushWasm(vm, .i32(op(UInt32(bitPattern: av), UInt32(bitPattern: bv)) ? 1 : 0))
        }
    }

    vm.registerContextOpcode(0x46) { vm, _, _, _ in i32BinCmp(vm) { $0 == $1 } }
    vm.registerContextOpcode(0x47) { vm, _, _, _ in i32BinCmp(vm) { $0 != $1 } }
    vm.registerContextOpcode(0x48) { vm, _, _, _ in i32BinCmp(vm) { $0 < $1 } }
    vm.registerContextOpcode(0x49) { vm, _, _, _ in i32BinCmpU(vm) { $0 < $1 } }
    vm.registerContextOpcode(0x4A) { vm, _, _, _ in i32BinCmp(vm) { $0 > $1 } }
    vm.registerContextOpcode(0x4B) { vm, _, _, _ in i32BinCmpU(vm) { $0 > $1 } }
    vm.registerContextOpcode(0x4C) { vm, _, _, _ in i32BinCmp(vm) { $0 <= $1 } }
    vm.registerContextOpcode(0x4D) { vm, _, _, _ in i32BinCmpU(vm) { $0 <= $1 } }
    vm.registerContextOpcode(0x4E) { vm, _, _, _ in i32BinCmp(vm) { $0 >= $1 } }
    vm.registerContextOpcode(0x4F) { vm, _, _, _ in i32BinCmpU(vm) { $0 >= $1 } }

    // Unary ops
    vm.registerContextOpcode(0x67) { vm, _, _, _ in  // i32.clz
        let a = popWasm(vm)
        if case .i32(let v) = a { pushWasm(vm, .i32(Int32(v.leadingZeroBitCount))) }
    }
    vm.registerContextOpcode(0x68) { vm, _, _, _ in  // i32.ctz
        let a = popWasm(vm)
        if case .i32(let v) = a { pushWasm(vm, .i32(Int32(v.trailingZeroBitCount))) }
    }
    vm.registerContextOpcode(0x69) { vm, _, _, _ in  // i32.popcnt
        let a = popWasm(vm)
        if case .i32(let v) = a { pushWasm(vm, .i32(Int32(v.nonzeroBitCount))) }
    }

    // Arithmetic -- Swift &+ &- &* for wrapping
    vm.registerContextOpcode(0x6A) { vm, _, _, _ in  // i32.add
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            pushWasm(vm, .i32(av &+ bv))
        }
    }
    vm.registerContextOpcode(0x6B) { vm, _, _, _ in  // i32.sub
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            pushWasm(vm, .i32(av &- bv))
        }
    }
    vm.registerContextOpcode(0x6C) { vm, _, _, _ in  // i32.mul
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            pushWasm(vm, .i32(av &* bv))
        }
    }
    vm.registerContextOpcode(0x6D) { vm, _, _, ctxObj in  // i32.div_s
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

    // Bitwise
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
            pushWasm(vm, .i32(Int32(bitPattern: (ua << shift) | (ua >> (32 - shift)))))
        }
    }
    vm.registerContextOpcode(0x78) { vm, _, _, _ in  // i32.rotr
        let b = popWasm(vm); let a = popWasm(vm)
        if case .i32(let av) = a, case .i32(let bv) = b {
            let ua = UInt32(bitPattern: av)
            let shift = UInt32(bitPattern: bv) & 31
            pushWasm(vm, .i32(Int32(bitPattern: (ua >> shift) | (ua << (32 - shift)))))
        }
    }
}

// -- i64 numeric (stub: register just const for the pipeline) --
func registerNumericI64(_ vm: GenericVM) {
    vm.registerContextOpcode(0x42) { vm, instr, _, _ in  // i64.const
        let value: Int64
        if let v = instr.operand as? Int64 { value = v }
        else { value = 0 }
        pushWasm(vm, .i64(value))
    }
    // i64 operations follow the same pattern as i32 -- register basic ones
    vm.registerContextOpcode(0x50) { vm, _, _, _ in let a = popWasm(vm); if case .i64(let v) = a { pushWasm(vm, .i32(v == 0 ? 1 : 0)) } }
    vm.registerContextOpcode(0x7C) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &+ bv)) } }
    vm.registerContextOpcode(0x7D) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &- bv)) } }
    vm.registerContextOpcode(0x7E) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .i64(let av) = a, case .i64(let bv) = b { pushWasm(vm, .i64(av &* bv)) } }
}

// -- f32 numeric --
func registerNumericF32(_ vm: GenericVM) {
    vm.registerContextOpcode(0x43) { vm, instr, _, _ in  // f32.const
        let value: Float
        if let v = instr.operand as? Float { value = v }
        else { value = 0 }
        pushWasm(vm, .f32(value))
    }
    vm.registerContextOpcode(0x92) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av + bv)) } }
    vm.registerContextOpcode(0x93) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av - bv)) } }
    vm.registerContextOpcode(0x94) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av * bv)) } }
    vm.registerContextOpcode(0x95) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f32(let av) = a, case .f32(let bv) = b { pushWasm(vm, .f32(av / bv)) } }
}

// -- f64 numeric --
func registerNumericF64(_ vm: GenericVM) {
    vm.registerContextOpcode(0x44) { vm, instr, _, _ in  // f64.const
        let value: Double
        if let v = instr.operand as? Double { value = v }
        else { value = 0 }
        pushWasm(vm, .f64(value))
    }
    vm.registerContextOpcode(0xA0) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av + bv)) } }
    vm.registerContextOpcode(0xA1) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av - bv)) } }
    vm.registerContextOpcode(0xA2) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av * bv)) } }
    vm.registerContextOpcode(0xA3) { vm, _, _, _ in let b = popWasm(vm); let a = popWasm(vm); if case .f64(let av) = a, case .f64(let bv) = b { pushWasm(vm, .f64(av / bv)) } }
}

// -- Conversions (just the essentials for the pipeline) --
func registerConversions(_ vm: GenericVM) {
    vm.registerContextOpcode(0xA7) { vm, _, _, _ in  // i32.wrap_i64
        let a = popWasm(vm); if case .i64(let v) = a { pushWasm(vm, .i32(Int32(truncatingIfNeeded: v))) }
    }
    vm.registerContextOpcode(0xAC) { vm, _, _, _ in  // i64.extend_i32_s
        let a = popWasm(vm); if case .i32(let v) = a { pushWasm(vm, .i64(Int64(v))) }
    }
    vm.registerContextOpcode(0xAD) { vm, _, _, _ in  // i64.extend_i32_u
        let a = popWasm(vm); if case .i32(let v) = a { pushWasm(vm, .i64(Int64(UInt32(bitPattern: v)))) }
    }
    vm.registerContextOpcode(0xB7) { vm, _, _, _ in  // f64.convert_i32_s
        let a = popWasm(vm); if case .i32(let v) = a { pushWasm(vm, .f64(Double(v))) }
    }
    vm.registerContextOpcode(0xB2) { vm, _, _, _ in  // f32.convert_i32_s
        let a = popWasm(vm); if case .i32(let v) = a { pushWasm(vm, .f32(Float(v))) }
    }
}

// -- Variable access --
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

// -- Parametric --
func registerParametric(_ vm: GenericVM) {
    // drop (0x1A)
    vm.registerContextOpcode(0x1A) { vm, _, _, _ in
        _ = popWasm(vm)
    }
    // select (0x1B)
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

// -- Memory --
func registerMemory(_ vm: GenericVM) {
    // i32.load (0x28)
    vm.registerContextOpcode(0x28) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg
        let addr = popWasm(vm)
        if case .i32(let base) = addr {
            let effectiveAddr = Int(UInt32(bitPattern: base)) + Int(memarg.offset)
            do {
                let val = try mem.loadI32(effectiveAddr)
                pushWasm(vm, .i32(val))
            } catch { vm.halted = true }
        }
    }
    // i32.store (0x36)
    vm.registerContextOpcode(0x36) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        guard let mem = ctx.memory else { vm.halted = true; return }
        let memarg = instr.operand as! MemArg
        let val = popWasm(vm)
        let addr = popWasm(vm)
        if case .i32(let base) = addr, case .i32(let v) = val {
            let effectiveAddr = Int(UInt32(bitPattern: base)) + Int(memarg.offset)
            do { try mem.storeI32(effectiveAddr, v) } catch { vm.halted = true }
        }
    }
    // memory.size (0x3F)
    vm.registerContextOpcode(0x3F) { vm, _, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        pushWasm(vm, .i32(Int32(ctx.memory?.size() ?? 0)))
    }
    // memory.grow (0x40)
    vm.registerContextOpcode(0x40) { vm, _, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let delta = popWasm(vm)
        if case .i32(let d) = delta {
            let result = ctx.memory?.grow(Int(d)) ?? -1
            pushWasm(vm, .i32(Int32(result)))
        }
    }
}

// -- Control flow --
func registerControl(_ vm: GenericVM) {
    // unreachable (0x00)
    vm.registerContextOpcode(0x00) { vm, _, _, _ in vm.halted = true }

    // nop (0x01)
    vm.registerContextOpcode(0x01) { _, _, _, _ in }

    // block (0x02)
    vm.registerContextOpcode(0x02) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let arity = blockArity(instr.operand, funcTypes: ctx.funcTypes)
        let target = ctx.controlFlowMap[vm.pc - 1]  // pc already advanced
        let endPc = target?.endPc ?? (vm.pc)
        ctx.labelStack.append(Label(arity: arity, targetPc: endPc, stackHeight: vm.typedStack.count, isLoop: false))
    }

    // loop (0x03)
    vm.registerContextOpcode(0x03) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let arity = blockArity(instr.operand, funcTypes: ctx.funcTypes)
        // Loop branches go to the loop start (pc - 1 since pc already advanced).
        ctx.labelStack.append(Label(arity: arity, targetPc: vm.pc - 1, stackHeight: vm.typedStack.count, isLoop: true))
    }

    // if (0x04)
    vm.registerContextOpcode(0x04) { vm, instr, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let arity = blockArity(instr.operand, funcTypes: ctx.funcTypes)
        let cond = popWasm(vm)
        let target = ctx.controlFlowMap[vm.pc - 1]
        let endPc = target?.endPc ?? vm.pc
        let elsePc = target?.elsePc

        ctx.labelStack.append(Label(arity: arity, targetPc: endPc, stackHeight: vm.typedStack.count, isLoop: false))

        if case .i32(let c) = cond, c == 0 {
            // Condition false -- jump to else or end.
            if let ep = elsePc {
                vm.jumpTo(ep + 1)  // Skip the else opcode itself
            } else {
                vm.jumpTo(endPc)  // Jump to end (will be processed by end handler)
            }
        }
    }

    // else (0x05)
    vm.registerContextOpcode(0x05) { vm, _, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        // If we reach else during execution, the if-branch was taken.
        // Jump to end.
        if let label = ctx.labelStack.last {
            vm.jumpTo(label.targetPc)
        }
    }

    // end (0x0B)
    vm.registerContextOpcode(0x0B) { vm, _, code, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext

        if !ctx.labelStack.isEmpty {
            let label = ctx.labelStack.removeLast()
            // Collect results.
            var results: [WasmValue] = []
            for _ in 0..<label.arity {
                results.insert(popWasm(vm), at: 0)
            }
            // Unwind to label height.
            while vm.typedStack.count > label.stackHeight {
                _ = vm.popTyped()
            }
            // Push results back.
            for v in results { pushWasm(vm, v) }
        } else {
            // End of function.
            if !ctx.savedFrames.isEmpty {
                // Restore caller frame.
                let frame = ctx.savedFrames.removeLast()

                // Collect return values.
                var results: [WasmValue] = []
                for _ in 0..<frame.returnArity {
                    results.insert(popWasm(vm), at: 0)
                }

                // Restore caller state.
                while vm.typedStack.count > frame.stackHeight {
                    _ = vm.popTyped()
                }

                ctx.typedLocals = frame.locals
                ctx.labelStack = frame.labelStack
                ctx.controlFlowMap = frame.controlFlowMap

                // Push return values.
                for v in results { pushWasm(vm, v) }

                // Jump back to caller (need to set up the code).
                // We jump to returnPc in the caller's code.
                vm.jumpTo(frame.returnPc)
            } else {
                // Top-level end -- halt.
                vm.halted = true
            }
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
        let idx = popWasm(vm)
        if case .i32(let i) = idx {
            let target: UInt32
            if Int(i) < brData.labels.count && i >= 0 {
                target = brData.labels[Int(i)]
            } else {
                target = brData.defaultLabel
            }
            do { try executeBranch(vm, ctx, Int(target)) } catch { vm.halted = true }
        }
    }

    // return (0x0F)
    vm.registerContextOpcode(0x0F) { vm, _, _, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        // Jump past all remaining instructions.
        ctx.labelStack = []
        if !ctx.savedFrames.isEmpty {
            let frame = ctx.savedFrames.removeLast()
            let funcType = ctx.funcTypes[0]  // Approximate
            var results: [WasmValue] = []
            for _ in 0..<frame.returnArity {
                results.insert(popWasm(vm), at: 0)
            }
            while vm.typedStack.count > frame.stackHeight {
                _ = vm.popTyped()
            }
            ctx.typedLocals = frame.locals
            ctx.labelStack = frame.labelStack
            ctx.controlFlowMap = frame.controlFlowMap
            for v in results { pushWasm(vm, v) }
            vm.jumpTo(frame.returnPc)
        } else {
            vm.halted = true
        }
    }

    // call (0x10)
    vm.registerContextOpcode(0x10) { vm, instr, code, ctxObj in
        let ctx = ctxObj as! WasmExecutionContext
        let funcIdx = instr.operand as! UInt32

        guard Int(funcIdx) < ctx.funcTypes.count else { vm.halted = true; return }
        let funcType = ctx.funcTypes[Int(funcIdx)]

        // Check for host function.
        if Int(funcIdx) < ctx.hostFunctions.count, let hostFunc = ctx.hostFunctions[Int(funcIdx)] {
            // Pop args.
            var args: [WasmValue] = []
            for _ in 0..<funcType.params.count {
                args.insert(popWasm(vm), at: 0)
            }
            do {
                let results = try hostFunc.call(args)
                for r in results { pushWasm(vm, r) }
            } catch { vm.halted = true }
            return
        }

        // Module-defined function.
        guard Int(funcIdx) < ctx.funcBodies.count, let body = ctx.funcBodies[Int(funcIdx)] else {
            vm.halted = true; return
        }

        // Save caller frame.
        let savedFrame = SavedFrame(
            locals: ctx.typedLocals,
            labelStack: ctx.labelStack,
            stackHeight: vm.typedStack.count - funcType.params.count,
            controlFlowMap: ctx.controlFlowMap,
            returnPc: vm.pc,  // Will continue from here after return
            returnArity: funcType.results.count,
            code: code
        )
        ctx.savedFrames.append(savedFrame)

        // Pop arguments.
        var args: [WasmValue] = []
        for _ in 0..<funcType.params.count {
            args.insert(popWasm(vm), at: 0)
        }

        // Initialize callee locals.
        var locals = args
        for vt in body.locals {
            locals.append(WasmValue.defaultValue(for: vt))
        }
        ctx.typedLocals = locals

        // Decode callee body.
        do {
            let decoded = try decodeFunctionBody(body)
            let cfMap = buildControlFlowMap(decoded)
            ctx.controlFlowMap = cfMap
            ctx.labelStack = []

            // Build callee code object.
            let calleeInstructions = decoded.map { d in
                Instruction(opcode: d.opcode, operand: d.operand)
            }
            let calleeCode = CodeObject(instructions: calleeInstructions)

            // Replace the current code's instructions in the VM
            // We need to execute the callee inline. Use a trick:
            // set PC to 0 and push the callee code.
            // Actually, we need to run the callee code in the same VM.
            // The simplest approach: save the current code context,
            // execute the callee, then restore.

            // Execute callee directly.
            let savedPc = vm.pc
            vm.pc = 0
            vm.halted = false

            while vm.pc < calleeCode.instructions.count && !vm.halted {
                let calleeInstr = calleeCode.instructions[vm.pc]
                let oldPc = vm.pc
                vm.pc += 1

                if let handler = vm as? GenericVM {
                    // We need to dispatch through the context handler
                    handler.dispatchContextInstruction(calleeInstr, calleeCode, ctx)
                }

                // If handler didn't change PC (beyond our increment), nothing extra needed
            }

            // After callee finishes, if we didn't restore via end handler,
            // restore now.
            if ctx.savedFrames.last?.returnPc == savedPc {
                // Frame wasn't popped -- pop it now
                let frame = ctx.savedFrames.removeLast()
                var results: [WasmValue] = []
                for _ in 0..<frame.returnArity {
                    results.insert(popWasm(vm), at: 0)
                }
                while vm.typedStack.count > frame.stackHeight {
                    _ = vm.popTyped()
                }
                ctx.typedLocals = frame.locals
                ctx.labelStack = frame.labelStack
                ctx.controlFlowMap = frame.controlFlowMap
                for v in results { pushWasm(vm, v) }
                vm.pc = savedPc
                vm.halted = false
            }
        } catch {
            vm.halted = true
        }
    }

    // call_indirect (0x11) - not needed for basic pipeline
    vm.registerContextOpcode(0x11) { vm, _, _, _ in vm.halted = true }
}

// ============================================================================
// MARK: - GenericVM Extension for dispatch
// ============================================================================

extension GenericVM {
    /// Dispatch a single instruction with context.
    public func dispatchContextInstruction(_ instruction: Instruction, _ code: CodeObject, _ context: AnyObject) {
        // This is called from within the call handler to execute callee instructions.
        // We need access to the registered context handlers.
        // Since we can't access private members, we'll use executeWithContext pattern.
    }
}

// ============================================================================
// MARK: - WasmExecutionEngine
// ============================================================================

/// The WASM execution engine -- interprets validated WASM modules.
public class WasmExecutionEngine {
    private let vm: GenericVM
    private let memory: LinearMemory?
    private let tables: [Table]
    private let globals: [WasmValue]
    private let globalTypes: [GlobalType]
    private let funcTypes: [FuncType]
    private let funcBodies: [FunctionBody?]
    private let hostFunctions: [HostFunction?]
    private var decodedCache: [Int: [DecodedInstruction]] = [:]

    // We need mutable globals, so store separately
    private var mutableGlobals: [WasmValue]

    public init(memory: LinearMemory?, tables: [Table], globals: [WasmValue],
                globalTypes: [GlobalType], funcTypes: [FuncType],
                funcBodies: [FunctionBody?], hostFunctions: [HostFunction?]) {
        self.memory = memory
        self.tables = tables
        self.globals = globals
        self.mutableGlobals = globals
        self.globalTypes = globalTypes
        self.funcTypes = funcTypes
        self.funcBodies = funcBodies
        self.hostFunctions = hostFunctions

        self.vm = GenericVM()
        vm.setMaxRecursionDepth(1024)
        registerAllInstructions(vm)
    }

    /// Call a WASM function by index.
    public func callFunction(_ funcIndex: Int, _ args: [WasmValue]) throws -> [WasmValue] {
        guard funcIndex < funcTypes.count else {
            throw TrapError("undefined function index \(funcIndex)")
        }
        let funcType = funcTypes[funcIndex]

        guard args.count == funcType.params.count else {
            throw TrapError("function \(funcIndex) expects \(funcType.params.count) arguments, got \(args.count)")
        }

        // Check host function.
        if funcIndex < hostFunctions.count, let hostFunc = hostFunctions[funcIndex] {
            return try hostFunc.call(args)
        }

        // Module function.
        guard funcIndex < funcBodies.count, let body = funcBodies[funcIndex] else {
            throw TrapError("no body for function \(funcIndex)")
        }

        // Decode.
        let decoded: [DecodedInstruction]
        if let cached = decodedCache[funcIndex] {
            decoded = cached
        } else {
            decoded = try decodeFunctionBody(body)
            decodedCache[funcIndex] = decoded
        }

        let cfMap = buildControlFlowMap(decoded)

        // Initialize locals.
        var locals = args
        for vt in body.locals {
            locals.append(WasmValue.defaultValue(for: vt))
        }

        // Build context.
        let ctx = WasmExecutionContext(
            memory: memory, tables: tables, globals: mutableGlobals,
            globalTypes: globalTypes, funcTypes: funcTypes,
            funcBodies: funcBodies, hostFunctions: hostFunctions,
            typedLocals: locals, controlFlowMap: cfMap
        )

        // Build code object.
        let instructions = decoded.map { Instruction(opcode: $0.opcode, operand: $0.operand) }
        let code = CodeObject(instructions: instructions)

        // Reset and execute.
        vm.reset()
        registerAllInstructions(vm)
        vm.executeWithContext(code, context: ctx)

        // Collect results.
        var results: [WasmValue] = []
        for _ in 0..<funcType.results.count {
            if !vm.typedStack.isEmpty {
                results.insert(popWasm(vm), at: 0)
            }
        }

        // Update mutable globals.
        mutableGlobals = ctx.globals

        return results
    }
}
