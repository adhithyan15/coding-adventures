// WasmModuleParser.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WasmModuleParser -- Parse a raw .wasm binary into a structured WasmModule
// ============================================================================
//
// A .wasm file is the compiled binary format of a WebAssembly module.
// This parser reads those bytes and builds an in-memory WasmModule object.
//
// Binary layout:
//   0x00: Magic: 0x00 0x61 0x73 0x6D  ("\0asm")
//   0x04: Version: 0x01 0x00 0x00 0x00  (= 1)
//   0x08: [Section]*  (zero or more sections)
//
// Each section: [id:u8] [size:u32leb] [payload:size bytes]
//
// ============================================================================

import Foundation
import WasmLeb128
import WasmTypes

// ============================================================================
// MARK: - Constants
// ============================================================================

private let WASM_MAGIC: [UInt8] = [0x00, 0x61, 0x73, 0x6D]
private let WASM_VERSION: [UInt8] = [0x01, 0x00, 0x00, 0x00]

private let SECTION_CUSTOM: UInt8 = 0
private let SECTION_TYPE: UInt8 = 1
private let SECTION_IMPORT: UInt8 = 2
private let SECTION_FUNCTION: UInt8 = 3
private let SECTION_TABLE: UInt8 = 4
private let SECTION_MEMORY: UInt8 = 5
private let SECTION_GLOBAL: UInt8 = 6
private let SECTION_EXPORT: UInt8 = 7
private let SECTION_START: UInt8 = 8
private let SECTION_ELEMENT: UInt8 = 9
private let SECTION_CODE: UInt8 = 10
private let SECTION_DATA: UInt8 = 11

private let FUNC_TYPE_PREFIX: UInt8 = 0x60
private let END_OPCODE: UInt8 = 0x0B

// ============================================================================
// MARK: - WasmParseError
// ============================================================================

/// Thrown when the binary data is malformed.
public enum WasmParseError: Error, Equatable {
    case invalidMagic
    case invalidVersion
    case unexpectedEnd(offset: Int)
    case invalidSectionOrder(offset: Int)
    case invalidFuncTypePrefix(offset: Int, got: UInt8)
    case unknownValueType(offset: Int, got: UInt8)
    case unknownImportKind(offset: Int, got: UInt8)
    case unknownExportKind(offset: Int, got: UInt8)
    case unknownLimitsFlags(offset: Int, got: UInt8)
    case unknownTableElementType(offset: Int, got: UInt8)
    case unterminatedInitExpr(offset: Int)
    case invalidLEB128(offset: Int)
}

// ============================================================================
// MARK: - WasmModuleParser
// ============================================================================

/// The main entry point for parsing .wasm binary data.
public struct WasmModuleParser {

    public init() {}

    /// Parse a .wasm binary into a WasmModule.
    public func parse(_ data: [UInt8]) throws -> WasmModule {
        var reader = BinaryReader(data: data)
        return try reader.parseModule()
    }
}

// ============================================================================
// MARK: - BinaryReader
// ============================================================================

/// A stateful cursor over a byte array.
private struct BinaryReader {
    private var pos: Int = 0
    private let data: [UInt8]

    init(data: [UInt8]) {
        self.data = data
    }

    // -- Primitive reads --

    mutating func readByte() throws -> UInt8 {
        guard pos < data.count else {
            throw WasmParseError.unexpectedEnd(offset: pos)
        }
        let b = data[pos]
        pos += 1
        return b
    }

    func peekByte() throws -> UInt8 {
        guard pos < data.count else {
            throw WasmParseError.unexpectedEnd(offset: pos)
        }
        return data[pos]
    }

    mutating func readBytes(_ n: Int) throws -> [UInt8] {
        guard pos + n <= data.count else {
            throw WasmParseError.unexpectedEnd(offset: pos)
        }
        let slice = Array(data[pos..<(pos + n)])
        pos += n
        return slice
    }

    mutating func readU32() throws -> UInt32 {
        let offset = pos
        do {
            var decoder = LEB128Decoder(data: data, offset: pos)
            let value = try decoder.decodeUnsigned32()
            pos = decoder.position
            return value
        } catch {
            throw WasmParseError.invalidLEB128(offset: offset)
        }
    }

    mutating func readString() throws -> String {
        let length = try readU32()
        let bytes = try readBytes(Int(length))
        guard let str = String(bytes: bytes, encoding: .utf8) else {
            return String(bytes.map { Character(UnicodeScalar($0)) })
        }
        return str
    }

    var atEnd: Bool { pos >= data.count }
    var offset: Int { pos }

    // -- Structured reads --

    mutating func readLimits() throws -> Limits {
        let flags = try readByte()
        let min = try readU32()
        var max: UInt32? = nil
        if flags & 1 != 0 {
            max = try readU32()
        } else if flags != 0 {
            throw WasmParseError.unknownLimitsFlags(offset: pos - 1, got: flags)
        }
        return Limits(min: min, max: max)
    }

    mutating func readGlobalType() throws -> GlobalType {
        let vtByte = try readByte()
        guard let vt = ValueType(rawValue: vtByte) else {
            throw WasmParseError.unknownValueType(offset: pos - 1, got: vtByte)
        }
        let mutByte = try readByte()
        return GlobalType(valueType: vt, mutable: mutByte != 0)
    }

    mutating func readInitExpr() throws -> [UInt8] {
        let start = pos
        while pos < data.count {
            let b = data[pos]
            pos += 1
            if b == END_OPCODE {
                return Array(data[start..<pos])
            }
            // Skip over immediate operands by scanning for 0x0B.
            // For const instructions, the LEB128 bytes have high bit patterns
            // that won't equal 0x0B, so simple byte scanning works.
        }
        throw WasmParseError.unterminatedInitExpr(offset: start)
    }

    mutating func readValueTypeVec() throws -> [ValueType] {
        let count = try readU32()
        var types: [ValueType] = []
        for _ in 0..<count {
            let b = try readByte()
            guard let vt = ValueType(rawValue: b) else {
                throw WasmParseError.unknownValueType(offset: pos - 1, got: b)
            }
            types.append(vt)
        }
        return types
    }

    // -- Section parsers --

    mutating func parseTypeSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let prefix = try readByte()
            guard prefix == FUNC_TYPE_PREFIX else {
                throw WasmParseError.invalidFuncTypePrefix(offset: pos - 1, got: prefix)
            }
            let params = try readValueTypeVec()
            let results = try readValueTypeVec()
            module.types.append(FuncType(params: params, results: results))
        }
    }

    mutating func parseImportSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let moduleName = try readString()
            let name = try readString()
            let kindByte = try readByte()

            let kind: ExternalKind
            let typeInfo: ImportTypeInfo

            switch kindByte {
            case ExternalKind.function.rawValue:
                kind = .function
                let typeIndex = try readU32()
                typeInfo = .function(typeIndex: typeIndex)

            case ExternalKind.table.rawValue:
                kind = .table
                let et = try readByte()
                guard et == FUNCREF else {
                    throw WasmParseError.unknownTableElementType(offset: pos - 1, got: et)
                }
                let limits = try readLimits()
                typeInfo = .table(TableType(elementType: et, limits: limits))

            case ExternalKind.memory.rawValue:
                kind = .memory
                let limits = try readLimits()
                typeInfo = .memory(MemoryType(limits: limits))

            case ExternalKind.global.rawValue:
                kind = .global
                let gt = try readGlobalType()
                typeInfo = .global(gt)

            default:
                throw WasmParseError.unknownImportKind(offset: pos - 1, got: kindByte)
            }

            module.imports.append(Import(moduleName: moduleName, name: name,
                                         kind: kind, typeInfo: typeInfo))
        }
    }

    mutating func parseFunctionSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let typeIndex = try readU32()
            module.functions.append(typeIndex)
        }
    }

    mutating func parseTableSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let et = try readByte()
            guard et == FUNCREF else {
                throw WasmParseError.unknownTableElementType(offset: pos - 1, got: et)
            }
            let limits = try readLimits()
            module.tables.append(TableType(elementType: et, limits: limits))
        }
    }

    mutating func parseMemorySection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let limits = try readLimits()
            module.memories.append(MemoryType(limits: limits))
        }
    }

    mutating func parseGlobalSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let gt = try readGlobalType()
            let initExpr = try readInitExpr()
            module.globals.append(Global(globalType: gt, initExpr: initExpr))
        }
    }

    mutating func parseExportSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let name = try readString()
            let kindByte = try readByte()
            guard let kind = ExternalKind(rawValue: kindByte) else {
                throw WasmParseError.unknownExportKind(offset: pos - 1, got: kindByte)
            }
            let index = try readU32()
            module.exports.append(Export(name: name, kind: kind, index: index))
        }
    }

    mutating func parseStartSection(_ module: WasmModule) throws {
        let funcIndex = try readU32()
        module.start = funcIndex
    }

    mutating func parseElementSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let tableIndex = try readU32()
            let offsetExpr = try readInitExpr()
            let numFuncs = try readU32()
            var funcIndices: [UInt32] = []
            for _ in 0..<numFuncs {
                funcIndices.append(try readU32())
            }
            module.elements.append(Element(tableIndex: tableIndex,
                                            offsetExpr: offsetExpr,
                                            functionIndices: funcIndices))
        }
    }

    mutating func parseCodeSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let bodySize = try readU32()
            let bodyEnd = pos + Int(bodySize)

            // Parse local declarations.
            let localDeclCount = try readU32()
            var locals: [ValueType] = []
            for _ in 0..<localDeclCount {
                let localCount = try readU32()
                let localByte = try readByte()
                guard let vt = ValueType(rawValue: localByte) else {
                    throw WasmParseError.unknownValueType(offset: pos - 1, got: localByte)
                }
                for _ in 0..<localCount {
                    locals.append(vt)
                }
            }

            // The remaining bytes are the instruction sequence.
            let codeLen = bodyEnd - pos
            let code = try readBytes(codeLen)

            module.code.append(FunctionBody(locals: locals, code: code))
        }
    }

    mutating func parseDataSection(_ module: WasmModule) throws {
        let count = try readU32()
        for _ in 0..<count {
            let memIndex = try readU32()
            let offsetExpr = try readInitExpr()
            let dataLen = try readU32()
            let bytes = try readBytes(Int(dataLen))
            module.data.append(DataSegment(memoryIndex: memIndex,
                                            offsetExpr: offsetExpr,
                                            data: bytes))
        }
    }

    mutating func parseCustomSection(_ module: WasmModule, sectionSize: Int) throws {
        let sectionEnd = pos + sectionSize
        let name = try readString()
        let dataLen = sectionEnd - pos
        let bytes = dataLen > 0 ? try readBytes(dataLen) : []
        module.customs.append(CustomSection(name: name, data: bytes))
    }

    // -- Top-level parser --

    mutating func parseModule() throws -> WasmModule {
        // Validate magic bytes.
        let magic = try readBytes(4)
        guard magic == WASM_MAGIC else {
            throw WasmParseError.invalidMagic
        }

        // Validate version.
        let version = try readBytes(4)
        guard version == WASM_VERSION else {
            throw WasmParseError.invalidVersion
        }

        let module = WasmModule()
        var lastSectionId: UInt8 = 0

        while !atEnd {
            let sectionId = try readByte()
            let sectionSize = try readU32()

            // Validate section ordering (custom sections can appear anywhere).
            if sectionId != SECTION_CUSTOM {
                if sectionId <= lastSectionId && lastSectionId != 0 {
                    throw WasmParseError.invalidSectionOrder(offset: pos)
                }
                lastSectionId = sectionId
            }

            switch sectionId {
            case SECTION_CUSTOM:
                try parseCustomSection(module, sectionSize: Int(sectionSize))
            case SECTION_TYPE:
                try parseTypeSection(module)
            case SECTION_IMPORT:
                try parseImportSection(module)
            case SECTION_FUNCTION:
                try parseFunctionSection(module)
            case SECTION_TABLE:
                try parseTableSection(module)
            case SECTION_MEMORY:
                try parseMemorySection(module)
            case SECTION_GLOBAL:
                try parseGlobalSection(module)
            case SECTION_EXPORT:
                try parseExportSection(module)
            case SECTION_START:
                try parseStartSection(module)
            case SECTION_ELEMENT:
                try parseElementSection(module)
            case SECTION_CODE:
                try parseCodeSection(module)
            case SECTION_DATA:
                try parseDataSection(module)
            default:
                // Skip unknown sections.
                _ = try readBytes(Int(sectionSize))
            }
        }

        return module
    }
}
