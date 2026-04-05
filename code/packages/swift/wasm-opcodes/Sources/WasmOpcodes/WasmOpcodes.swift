// WasmOpcodes.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WASM 1.0 Complete Opcode Table
// ============================================================================
//
// WebAssembly is a stack machine. Instructions operate on an implicit operand
// stack. Each instruction has metadata about its name, opcode byte, category,
// immediate operand types, and stack effects (how many values are consumed
// and produced).
//
// ============================================================================

import Foundation

// ============================================================================
// MARK: - OpcodeInfo
// ============================================================================

/// Metadata for a single WASM instruction.
public struct OpcodeInfo: Equatable {
    /// The canonical WASM text-format name, e.g. "i32.add".
    public let name: String
    /// The single-byte opcode value, e.g. 0x6A for i32.add.
    public let opcode: UInt8
    /// Broad instruction category.
    public let category: String
    /// List of immediate operand types that follow the opcode.
    public let immediates: [String]
    /// Number of values popped from the operand stack.
    public let stackPop: Int
    /// Number of values pushed onto the operand stack.
    public let stackPush: Int

    public init(name: String, opcode: UInt8, category: String,
                immediates: [String], stackPop: Int, stackPush: Int) {
        self.name = name
        self.opcode = opcode
        self.category = category
        self.immediates = immediates
        self.stackPop = stackPop
        self.stackPush = stackPush
    }
}

// ============================================================================
// MARK: - Raw Opcode Table
// ============================================================================
//
// Each entry: (opcode, name, category, immediates, stackPop, stackPush)
// Formatted in groups matching the WASM spec section order.

private let rawOpcodes: [(UInt8, String, String, [String], Int, Int)] = [
    // -- Control flow --
    (0x00, "unreachable",    "control", [],                          0, 0),
    (0x01, "nop",            "control", [],                          0, 0),
    (0x02, "block",          "control", ["blocktype"],               0, 0),
    (0x03, "loop",           "control", ["blocktype"],               0, 0),
    (0x04, "if",             "control", ["blocktype"],               1, 0),
    (0x05, "else",           "control", [],                          0, 0),
    (0x0B, "end",            "control", [],                          0, 0),
    (0x0C, "br",             "control", ["labelidx"],                0, 0),
    (0x0D, "br_if",          "control", ["labelidx"],                1, 0),
    (0x0E, "br_table",       "control", ["vec_labelidx"],            1, 0),
    (0x0F, "return",         "control", [],                          0, 0),
    (0x10, "call",           "control", ["funcidx"],                 0, 0),
    (0x11, "call_indirect",  "control", ["typeidx", "tableidx"],     1, 0),

    // -- Parametric --
    (0x1A, "drop",   "parametric", [], 1, 0),
    (0x1B, "select", "parametric", [], 3, 1),

    // -- Variable access --
    (0x20, "local.get",  "variable", ["localidx"],  0, 1),
    (0x21, "local.set",  "variable", ["localidx"],  1, 0),
    (0x22, "local.tee",  "variable", ["localidx"],  1, 1),
    (0x23, "global.get", "variable", ["globalidx"], 0, 1),
    (0x24, "global.set", "variable", ["globalidx"], 1, 0),

    // -- Memory loads --
    (0x28, "i32.load",     "memory", ["memarg"], 1, 1),
    (0x29, "i64.load",     "memory", ["memarg"], 1, 1),
    (0x2A, "f32.load",     "memory", ["memarg"], 1, 1),
    (0x2B, "f64.load",     "memory", ["memarg"], 1, 1),
    (0x2C, "i32.load8_s",  "memory", ["memarg"], 1, 1),
    (0x2D, "i32.load8_u",  "memory", ["memarg"], 1, 1),
    (0x2E, "i32.load16_s", "memory", ["memarg"], 1, 1),
    (0x2F, "i32.load16_u", "memory", ["memarg"], 1, 1),
    (0x30, "i64.load8_s",  "memory", ["memarg"], 1, 1),
    (0x31, "i64.load8_u",  "memory", ["memarg"], 1, 1),
    (0x32, "i64.load16_s", "memory", ["memarg"], 1, 1),
    (0x33, "i64.load16_u", "memory", ["memarg"], 1, 1),
    (0x34, "i64.load32_s", "memory", ["memarg"], 1, 1),
    (0x35, "i64.load32_u", "memory", ["memarg"], 1, 1),

    // -- Memory stores --
    (0x36, "i32.store",   "memory", ["memarg"], 2, 0),
    (0x37, "i64.store",   "memory", ["memarg"], 2, 0),
    (0x38, "f32.store",   "memory", ["memarg"], 2, 0),
    (0x39, "f64.store",   "memory", ["memarg"], 2, 0),
    (0x3A, "i32.store8",  "memory", ["memarg"], 2, 0),
    (0x3B, "i32.store16", "memory", ["memarg"], 2, 0),
    (0x3C, "i64.store8",  "memory", ["memarg"], 2, 0),
    (0x3D, "i64.store16", "memory", ["memarg"], 2, 0),
    (0x3E, "i64.store32", "memory", ["memarg"], 2, 0),

    // -- Memory management --
    (0x3F, "memory.size", "memory", ["memidx"], 0, 1),
    (0x40, "memory.grow", "memory", ["memidx"], 1, 1),

    // -- i32 numeric --
    (0x41, "i32.const",  "numeric_i32", ["i32"], 0, 1),
    (0x45, "i32.eqz",   "numeric_i32", [],      1, 1),
    (0x46, "i32.eq",    "numeric_i32", [],      2, 1),
    (0x47, "i32.ne",    "numeric_i32", [],      2, 1),
    (0x48, "i32.lt_s",  "numeric_i32", [],      2, 1),
    (0x49, "i32.lt_u",  "numeric_i32", [],      2, 1),
    (0x4A, "i32.gt_s",  "numeric_i32", [],      2, 1),
    (0x4B, "i32.gt_u",  "numeric_i32", [],      2, 1),
    (0x4C, "i32.le_s",  "numeric_i32", [],      2, 1),
    (0x4D, "i32.le_u",  "numeric_i32", [],      2, 1),
    (0x4E, "i32.ge_s",  "numeric_i32", [],      2, 1),
    (0x4F, "i32.ge_u",  "numeric_i32", [],      2, 1),
    (0x67, "i32.clz",   "numeric_i32", [],      1, 1),
    (0x68, "i32.ctz",   "numeric_i32", [],      1, 1),
    (0x69, "i32.popcnt","numeric_i32", [],      1, 1),
    (0x6A, "i32.add",   "numeric_i32", [],      2, 1),
    (0x6B, "i32.sub",   "numeric_i32", [],      2, 1),
    (0x6C, "i32.mul",   "numeric_i32", [],      2, 1),
    (0x6D, "i32.div_s", "numeric_i32", [],      2, 1),
    (0x6E, "i32.div_u", "numeric_i32", [],      2, 1),
    (0x6F, "i32.rem_s", "numeric_i32", [],      2, 1),
    (0x70, "i32.rem_u", "numeric_i32", [],      2, 1),
    (0x71, "i32.and",   "numeric_i32", [],      2, 1),
    (0x72, "i32.or",    "numeric_i32", [],      2, 1),
    (0x73, "i32.xor",   "numeric_i32", [],      2, 1),
    (0x74, "i32.shl",   "numeric_i32", [],      2, 1),
    (0x75, "i32.shr_s", "numeric_i32", [],      2, 1),
    (0x76, "i32.shr_u", "numeric_i32", [],      2, 1),
    (0x77, "i32.rotl",  "numeric_i32", [],      2, 1),
    (0x78, "i32.rotr",  "numeric_i32", [],      2, 1),

    // -- i64 numeric --
    (0x42, "i64.const",  "numeric_i64", ["i64"], 0, 1),
    (0x50, "i64.eqz",   "numeric_i64", [],      1, 1),
    (0x51, "i64.eq",    "numeric_i64", [],      2, 1),
    (0x52, "i64.ne",    "numeric_i64", [],      2, 1),
    (0x53, "i64.lt_s",  "numeric_i64", [],      2, 1),
    (0x54, "i64.lt_u",  "numeric_i64", [],      2, 1),
    (0x55, "i64.gt_s",  "numeric_i64", [],      2, 1),
    (0x56, "i64.gt_u",  "numeric_i64", [],      2, 1),
    (0x57, "i64.le_s",  "numeric_i64", [],      2, 1),
    (0x58, "i64.le_u",  "numeric_i64", [],      2, 1),
    (0x59, "i64.ge_s",  "numeric_i64", [],      2, 1),
    (0x5A, "i64.ge_u",  "numeric_i64", [],      2, 1),
    (0x79, "i64.clz",   "numeric_i64", [],      1, 1),
    (0x7A, "i64.ctz",   "numeric_i64", [],      1, 1),
    (0x7B, "i64.popcnt","numeric_i64", [],      1, 1),
    (0x7C, "i64.add",   "numeric_i64", [],      2, 1),
    (0x7D, "i64.sub",   "numeric_i64", [],      2, 1),
    (0x7E, "i64.mul",   "numeric_i64", [],      2, 1),
    (0x7F, "i64.div_s", "numeric_i64", [],      2, 1),
    (0x80, "i64.div_u", "numeric_i64", [],      2, 1),
    (0x81, "i64.rem_s", "numeric_i64", [],      2, 1),
    (0x82, "i64.rem_u", "numeric_i64", [],      2, 1),
    (0x83, "i64.and",   "numeric_i64", [],      2, 1),
    (0x84, "i64.or",    "numeric_i64", [],      2, 1),
    (0x85, "i64.xor",   "numeric_i64", [],      2, 1),
    (0x86, "i64.shl",   "numeric_i64", [],      2, 1),
    (0x87, "i64.shr_s", "numeric_i64", [],      2, 1),
    (0x88, "i64.shr_u", "numeric_i64", [],      2, 1),
    (0x89, "i64.rotl",  "numeric_i64", [],      2, 1),
    (0x8A, "i64.rotr",  "numeric_i64", [],      2, 1),

    // -- f32 numeric --
    (0x43, "f32.const",    "numeric_f32", ["f32"], 0, 1),
    (0x5B, "f32.eq",       "numeric_f32", [],      2, 1),
    (0x5C, "f32.ne",       "numeric_f32", [],      2, 1),
    (0x5D, "f32.lt",       "numeric_f32", [],      2, 1),
    (0x5E, "f32.gt",       "numeric_f32", [],      2, 1),
    (0x5F, "f32.le",       "numeric_f32", [],      2, 1),
    (0x60, "f32.ge",       "numeric_f32", [],      2, 1),
    (0x8B, "f32.abs",      "numeric_f32", [],      1, 1),
    (0x8C, "f32.neg",      "numeric_f32", [],      1, 1),
    (0x8D, "f32.ceil",     "numeric_f32", [],      1, 1),
    (0x8E, "f32.floor",    "numeric_f32", [],      1, 1),
    (0x8F, "f32.trunc",    "numeric_f32", [],      1, 1),
    (0x90, "f32.nearest",  "numeric_f32", [],      1, 1),
    (0x91, "f32.sqrt",     "numeric_f32", [],      1, 1),
    (0x92, "f32.add",      "numeric_f32", [],      2, 1),
    (0x93, "f32.sub",      "numeric_f32", [],      2, 1),
    (0x94, "f32.mul",      "numeric_f32", [],      2, 1),
    (0x95, "f32.div",      "numeric_f32", [],      2, 1),
    (0x96, "f32.min",      "numeric_f32", [],      2, 1),
    (0x97, "f32.max",      "numeric_f32", [],      2, 1),
    (0x98, "f32.copysign", "numeric_f32", [],      2, 1),

    // -- f64 numeric --
    (0x44, "f64.const",    "numeric_f64", ["f64"], 0, 1),
    (0x61, "f64.eq",       "numeric_f64", [],      2, 1),
    (0x62, "f64.ne",       "numeric_f64", [],      2, 1),
    (0x63, "f64.lt",       "numeric_f64", [],      2, 1),
    (0x64, "f64.gt",       "numeric_f64", [],      2, 1),
    (0x65, "f64.le",       "numeric_f64", [],      2, 1),
    (0x66, "f64.ge",       "numeric_f64", [],      2, 1),
    (0x99, "f64.abs",      "numeric_f64", [],      1, 1),
    (0x9A, "f64.neg",      "numeric_f64", [],      1, 1),
    (0x9B, "f64.ceil",     "numeric_f64", [],      1, 1),
    (0x9C, "f64.floor",    "numeric_f64", [],      1, 1),
    (0x9D, "f64.trunc",    "numeric_f64", [],      1, 1),
    (0x9E, "f64.nearest",  "numeric_f64", [],      1, 1),
    (0x9F, "f64.sqrt",     "numeric_f64", [],      1, 1),
    (0xA0, "f64.add",      "numeric_f64", [],      2, 1),
    (0xA1, "f64.sub",      "numeric_f64", [],      2, 1),
    (0xA2, "f64.mul",      "numeric_f64", [],      2, 1),
    (0xA3, "f64.div",      "numeric_f64", [],      2, 1),
    (0xA4, "f64.min",      "numeric_f64", [],      2, 1),
    (0xA5, "f64.max",      "numeric_f64", [],      2, 1),
    (0xA6, "f64.copysign", "numeric_f64", [],      2, 1),

    // -- Conversions --
    (0xA7, "i32.wrap_i64",       "conversion", [], 1, 1),
    (0xA8, "i32.trunc_f32_s",    "conversion", [], 1, 1),
    (0xA9, "i32.trunc_f32_u",    "conversion", [], 1, 1),
    (0xAA, "i32.trunc_f64_s",    "conversion", [], 1, 1),
    (0xAB, "i32.trunc_f64_u",    "conversion", [], 1, 1),
    (0xAC, "i64.extend_i32_s",   "conversion", [], 1, 1),
    (0xAD, "i64.extend_i32_u",   "conversion", [], 1, 1),
    (0xAE, "i64.trunc_f32_s",    "conversion", [], 1, 1),
    (0xAF, "i64.trunc_f32_u",    "conversion", [], 1, 1),
    (0xB0, "i64.trunc_f64_s",    "conversion", [], 1, 1),
    (0xB1, "i64.trunc_f64_u",    "conversion", [], 1, 1),
    (0xB2, "f32.convert_i32_s",  "conversion", [], 1, 1),
    (0xB3, "f32.convert_i32_u",  "conversion", [], 1, 1),
    (0xB4, "f32.convert_i64_s",  "conversion", [], 1, 1),
    (0xB5, "f32.convert_i64_u",  "conversion", [], 1, 1),
    (0xB6, "f32.demote_f64",     "conversion", [], 1, 1),
    (0xB7, "f64.convert_i32_s",  "conversion", [], 1, 1),
    (0xB8, "f64.convert_i32_u",  "conversion", [], 1, 1),
    (0xB9, "f64.convert_i64_s",  "conversion", [], 1, 1),
    (0xBA, "f64.convert_i64_u",  "conversion", [], 1, 1),
    (0xBB, "f64.promote_f32",    "conversion", [], 1, 1),
    (0xBC, "i32.reinterpret_f32","conversion", [], 1, 1),
    (0xBD, "i64.reinterpret_f64","conversion", [], 1, 1),
    (0xBE, "f32.reinterpret_i32","conversion", [], 1, 1),
    (0xBF, "f64.reinterpret_i64","conversion", [], 1, 1),
]

// ============================================================================
// MARK: - Lookup Tables (built once at load time)
// ============================================================================

/// Primary lookup table: opcode byte -> OpcodeInfo.
public let OPCODES: [UInt8: OpcodeInfo] = {
    var map = [UInt8: OpcodeInfo]()
    for (opcode, name, category, immediates, stackPop, stackPush) in rawOpcodes {
        map[opcode] = OpcodeInfo(
            name: name, opcode: opcode, category: category,
            immediates: immediates, stackPop: stackPop, stackPush: stackPush
        )
    }
    return map
}()

/// Secondary lookup table: instruction name -> OpcodeInfo.
public let OPCODES_BY_NAME: [String: OpcodeInfo] = {
    var map = [String: OpcodeInfo]()
    for (_, info) in OPCODES {
        map[info.name] = info
    }
    return map
}()

// ============================================================================
// MARK: - Public API
// ============================================================================

/// Look up an instruction by its opcode byte.
public func getOpcode(_ byte: UInt8) -> OpcodeInfo? {
    return OPCODES[byte]
}

/// Look up an instruction by its text-format name.
public func getOpcodeByName(_ name: String) -> OpcodeInfo? {
    return OPCODES_BY_NAME[name]
}
