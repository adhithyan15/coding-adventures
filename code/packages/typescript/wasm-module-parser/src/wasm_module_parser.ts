/**
 * wasm_module_parser.ts — Parse a raw .wasm binary into a structured WasmModule
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * WHAT IS A .wasm FILE?
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * A .wasm file is the compiled binary format of a WebAssembly (WASM) module.
 * It's what compilers like Clang/LLVM, Emscripten, or wasm-pack produce.
 * The format is designed to be:
 *   - Compact: smaller than equivalent JavaScript
 *   - Fast to validate: linear structure, one pass
 *   - Safe: strongly typed, no arbitrary jumps
 *
 * This parser reads those bytes and builds an in-memory WasmModule object —
 * structured data you can inspect, transform, or generate code from.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * BINARY LAYOUT — ASCII DIAGRAM
 * ─────────────────────────────────────────────────────────────────────────────
 *
 *  Byte offset  Content
 *  ──────────── ───────────────────────────────────────────────────────────────
 *  0x00         Magic: 0x00 0x61 0x73 0x6D  ("\0asm")
 *  0x04         Version: 0x01 0x00 0x00 0x00  (= 1, little-endian)
 *  0x08         [Section]*  (zero or more sections follow)
 *
 *  Each section has the format:
 *
 *    ┌─────────────────────────────────────────────────────────────┐
 *    │  id      : 1 byte       — section type (0–11)               │
 *    │  size    : u32 ULEB128  — byte count of payload that follows │
 *    │  payload : size bytes   — section-specific content          │
 *    └─────────────────────────────────────────────────────────────┘
 *
 *  Section IDs:
 *    0  = Custom    (can appear anywhere, zero or more)
 *    1  = Type      (function signatures)
 *    2  = Import    (host imports)
 *    3  = Function  (type indices for local functions)
 *    4  = Table     (indirect call tables)
 *    5  = Memory    (linear memory declarations)
 *    6  = Global    (global variables)
 *    7  = Export    (host-visible exports)
 *    8  = Start     (entry-point function index)
 *    9  = Element   (table initializers)
 *    10 = Code      (function bodies / bytecode)
 *    11 = Data      (memory initializers)
 *
 *  Numbered sections (1–11) must appear in ascending order.
 *  Custom sections (0) may appear between any two sections.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * DESIGN PHILOSOPHY
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * This parser is intentionally a single-pass, cursor-based reader. A cursor
 * is just an integer index into the Uint8Array. We advance it forward as we
 * consume bytes. When something is wrong (wrong magic, truncated data), we
 * throw a WasmParseError that records the offset so the caller knows exactly
 * where the problem is.
 *
 * Think of parsing like reading a book:
 *   - We start at page 1 (offset 0).
 *   - We read chapters in order (sections must be in order).
 *   - If a page is missing, we know exactly which page is missing.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 */

import { decodeUnsigned } from "@coding-adventures/wasm-leb128";
import {
  WasmModule,
  ValueType,
  ExternalKind,
  FUNCREF,
  makeFuncType,
} from "@coding-adventures/wasm-types";
import type {
  FuncType,
  Limits,
  MemoryType,
  TableType,
  GlobalType,
  Import,
  Export,
  Global,
  Element,
  DataSegment,
  FunctionBody,
  CustomSection,
} from "@coding-adventures/wasm-types";

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The magic four-byte prefix of every valid .wasm file.
 *
 * Chosen to be:
 *   - "\0asm" — human-readable as a magic tag
 *   - 0x00 as the first byte — tools that read text files will detect it as
 *     binary, preventing accidental text-editor corruption
 *
 *   Hex: 00 61 73 6D
 *   ASCII: '\0' 'a' 's' 'm'
 */
const WASM_MAGIC = new Uint8Array([0x00, 0x61, 0x73, 0x6d]);

/**
 * The version field is a 4-byte little-endian uint32 = 1.
 * All WASM 1.0 modules have this exact value. A value of 2 would mean WASM 2.0,
 * which this parser does not support.
 */
const WASM_VERSION = new Uint8Array([0x01, 0x00, 0x00, 0x00]);

/**
 * WASM section IDs 0–11.
 *
 * Think of these as the "chapters" of the .wasm book. Custom sections (0)
 * are "appendices" that can appear anywhere.
 */
const SECTION_CUSTOM = 0;
const SECTION_TYPE = 1;
const SECTION_IMPORT = 2;
const SECTION_FUNCTION = 3;
const SECTION_TABLE = 4;
const SECTION_MEMORY = 5;
const SECTION_GLOBAL = 6;
const SECTION_EXPORT = 7;
const SECTION_START = 8;
const SECTION_ELEMENT = 9;
const SECTION_CODE = 10;
const SECTION_DATA = 11;

/**
 * The function type prefix byte (0x60).
 *
 * In the type section, each function type begins with 0x60 to distinguish it
 * from other potential type encodings (reserved for future WASM versions).
 */
const FUNC_TYPE_PREFIX = 0x60;

/**
 * The "end of expression" opcode (0x0B).
 *
 * Constant initialization expressions (for globals, offsets in element and
 * data segments) are terminated by this byte. It signals "stop evaluating."
 */
const END_OPCODE = 0x0b;

// ─────────────────────────────────────────────────────────────────────────────
// WasmParseError
// ─────────────────────────────────────────────────────────────────────────────

/**
 * WasmParseError — thrown when the binary data is malformed.
 *
 * In addition to the standard Error message, it records the byte offset at
 * which the problem was detected. This helps callers point users at the
 * exact problematic byte in a hex dump or disassembly.
 *
 * Example:
 *   try {
 *     parser.parse(badData);
 *   } catch (e) {
 *     if (e instanceof WasmParseError) {
 *       console.error(`Parse failed at offset 0x${e.offset.toString(16)}: ${e.message}`);
 *     }
 *   }
 */
export class WasmParseError extends Error {
  /**
   * @param message - Human-readable description of what went wrong.
   * @param offset  - Byte offset in the input where the error was detected.
   */
  constructor(
    message: string,
    public readonly offset: number
  ) {
    super(message);
    this.name = "WasmParseError";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// WasmModuleParser
// ─────────────────────────────────────────────────────────────────────────────

/**
 * WasmModuleParser — the main entry point for parsing .wasm binary data.
 *
 * Usage:
 *   const parser = new WasmModuleParser();
 *   const module = parser.parse(wasmBytes);   // throws WasmParseError on bad data
 *
 * The parser is stateless between calls — you can reuse the same instance
 * to parse many different modules. All state lives in the local Parser class
 * created per call.
 *
 * Supported sections: Type, Import, Function, Table, Memory, Global, Export,
 *                     Start, Element, Code, Data, Custom.
 * Section ordering: validated (numbered sections must appear in order 1–11).
 */
export class WasmModuleParser {
  /**
   * Parse a .wasm binary into a WasmModule.
   *
   * @param data - The raw bytes of the .wasm file.
   * @returns    A populated WasmModule.
   * @throws     WasmParseError if the data is malformed.
   */
  parse(data: Uint8Array): WasmModule {
    const reader = new BinaryReader(data);
    return reader.parseModule();
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// BinaryReader — internal cursor-based parser
// ─────────────────────────────────────────────────────────────────────────────

/**
 * BinaryReader — a stateful cursor over a Uint8Array.
 *
 * This is an internal helper class. Every read method advances `pos` forward
 * by the number of bytes consumed. Think of it as a tape head reading a tape.
 *
 * ┌──────────────────────────────────────────────────────────┐
 * │  data:  [00][61][73][6D][01][00][00][00][01][05][...] .. │
 * │          ^                                               │
 * │          pos=0 (start)                                   │
 * └──────────────────────────────────────────────────────────┘
 *
 * After reading the 8-byte header:
 *
 * ┌──────────────────────────────────────────────────────────┐
 * │  data:  [00][61][73][6D][01][00][00][00][01][05][...] .. │
 * │                                          ^               │
 * │                                          pos=8           │
 * └──────────────────────────────────────────────────────────┘
 */
class BinaryReader {
  private pos = 0;
  private readonly data: Uint8Array;

  constructor(data: Uint8Array) {
    this.data = data;
  }

  // ── Primitive reads ────────────────────────────────────────────────────────

  /**
   * Read a single byte.
   *
   * @throws WasmParseError if at end of data.
   */
  readByte(): number {
    if (this.pos >= this.data.length) {
      throw new WasmParseError(
        `Unexpected end of data: expected 1 byte at offset ${this.pos}`,
        this.pos
      );
    }
    return this.data[this.pos++];
  }

  /**
   * Peek at the current byte without advancing the cursor.
   *
   * @throws WasmParseError if at end of data.
   */
  peekByte(): number {
    if (this.pos >= this.data.length) {
      throw new WasmParseError(
        `Unexpected end of data: expected 1 byte at offset ${this.pos}`,
        this.pos
      );
    }
    return this.data[this.pos];
  }

  /**
   * Read exactly `n` bytes and return them as a new Uint8Array.
   *
   * @throws WasmParseError if fewer than n bytes remain.
   */
  readBytes(n: number): Uint8Array {
    if (this.pos + n > this.data.length) {
      throw new WasmParseError(
        `Unexpected end of data: expected ${n} bytes at offset ${this.pos}, but only ${this.data.length - this.pos} remain`,
        this.pos
      );
    }
    const slice = this.data.slice(this.pos, this.pos + n);
    this.pos += n;
    return slice;
  }

  /**
   * Read a ULEB128-encoded unsigned integer.
   *
   * ULEB128 (Unsigned LEB128) uses variable-length encoding:
   *   - Each byte contributes 7 bits of data.
   *   - The high bit (0x80) signals "more bytes follow."
   *   - This lets small numbers (< 128) fit in a single byte.
   *
   * Example: 300 = 0b1_0010_1100
   *   Byte 0: 0b1010_1100 = 0xAC (low 7 bits = 0101100, continuation set)
   *   Byte 1: 0b0000_0010 = 0x02 (high bits = 0000010, no continuation)
   *
   * @throws WasmParseError (via LEB128Error forwarded) on malformed encoding.
   */
  readU32(): number {
    const offset = this.pos;
    try {
      const [value, consumed] = decodeUnsigned(this.data, this.pos);
      this.pos += consumed;
      return value;
    } catch (e) {
      throw new WasmParseError(
        `Invalid LEB128 at offset ${offset}: ${(e as Error).message}`,
        offset
      );
    }
  }

  /**
   * Read a UTF-8 string encoded as: u32leb length prefix + raw bytes.
   *
   * WASM strings are always UTF-8. Names (import module names, export names,
   * custom section names) use this encoding.
   *
   * Example — encoding "env" (3 bytes):
   *   [0x03, 0x65, 0x6E, 0x76]
   *    ^^^^ length   e    n    v
   */
  readString(): string {
    const length = this.readU32();
    const bytes = this.readBytes(length);
    return new TextDecoder("utf-8").decode(bytes);
  }

  /** Check if we've consumed all bytes. */
  atEnd(): boolean {
    return this.pos >= this.data.length;
  }

  /** Current cursor position (byte offset). */
  get offset(): number {
    return this.pos;
  }

  // ── Structured reads ───────────────────────────────────────────────────────

  /**
   * Read a Limits structure.
   *
   * Limits appear in memory and table types. They describe a minimum (required)
   * size and an optional maximum (upper bound) size.
   *
   * Binary encoding:
   *   flags:u8 = 0x00 → min:u32leb only (no maximum)
   *   flags:u8 = 0x01 → min:u32leb, max:u32leb
   *
   * The "flag" byte is a bitmask. Bit 0 signals whether a maximum is present.
   *
   *   ┌───────┬─────────────────────────────┐
   *   │ flags │ Meaning                     │
   *   ├───────┼─────────────────────────────┤
   *   │  0x00 │ min only (unbounded growth) │
   *   │  0x01 │ min + max (bounded growth)  │
   *   └───────┴─────────────────────────────┘
   */
  readLimits(): Limits {
    const flagsOffset = this.pos;
    const flags = this.readByte();
    const min = this.readU32();
    let max: number | null = null;
    if (flags & 1) {
      max = this.readU32();
    } else if (flags !== 0) {
      throw new WasmParseError(
        `Unknown limits flags byte 0x${flags.toString(16)} at offset ${flagsOffset}`,
        flagsOffset
      );
    }
    return { min, max };
  }

  /**
   * Read a GlobalType.
   *
   * GlobalType = value_type:u8 + mutability:u8
   *
   * Examples:
   *   [0x7F, 0x00] → immutable i32 global
   *   [0x7C, 0x01] → mutable f64 global
   */
  readGlobalType(): GlobalType {
    const vtOffset = this.pos;
    const valueTypeByte = this.readByte();
    if (!isValidValueType(valueTypeByte)) {
      throw new WasmParseError(
        `Unknown value type byte 0x${valueTypeByte.toString(16)} at offset ${vtOffset}`,
        vtOffset
      );
    }
    const mutableByte = this.readByte();
    return {
      valueType: valueTypeByte as ValueType,
      mutable: mutableByte !== 0,
    };
  }

  /**
   * Read a constant init expression: bytes up to and including 0x0B (end).
   *
   * In WASM 1.0, init expressions are used to initialize:
   *   - Global variables (global section)
   *   - Table offsets (element section)
   *   - Memory offsets (data section)
   *
   * They are sequences of one or more opcodes that must evaluate to a constant.
   * The expression ends with the `end` opcode (0x0B). We collect all bytes
   * including the 0x0B.
   *
   * Common init expressions:
   *   [0x41, 0x00, 0x0B]  → i32.const 0; end
   *   [0x41, 0x2A, 0x0B]  → i32.const 42; end
   *   [0x42, 0x00, 0x0B]  → i64.const 0; end
   */
  readInitExpr(): Uint8Array {
    const start = this.pos;
    // Scan forward until we find 0x0B (end opcode).
    // We don't attempt to decode the opcodes — we just slice the raw bytes.
    while (this.pos < this.data.length) {
      const b = this.data[this.pos++];
      if (b === END_OPCODE) {
        return this.data.slice(start, this.pos);
      }
    }
    throw new WasmParseError(
      `Init expression at offset ${start} never terminated with 0x0B (end opcode)`,
      start
    );
  }

  // ── Section parsers ────────────────────────────────────────────────────────

  /**
   * Parse the type section (ID 1): function type signatures.
   *
   * The type section lists all unique function signatures used in the module.
   * Both imported and local functions reference these by index.
   *
   * Structure:
   *   count:u32leb
   *   for each type:
   *     0x60  (function type prefix)
   *     param_count:u32leb
   *     param_types:u8[param_count]
   *     result_count:u32leb
   *     result_types:u8[result_count]
   *
   * Example — `(i32, i32) → i32`:
   *   [0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F]
   *           ^^^^ ^^^^ ^^^^ ^^^^ ^^^^ ^^^^
   *           2    i32  i32  1    i32
   */
  parseTypeSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const prefixOffset = this.pos;
      const prefix = this.readByte();
      if (prefix !== FUNC_TYPE_PREFIX) {
        throw new WasmParseError(
          `Expected function type prefix 0x60 at offset ${prefixOffset}, got 0x${prefix.toString(16)}`,
          prefixOffset
        );
      }
      const params = this.readValueTypeVec();
      const results = this.readValueTypeVec();
      module.types.push(makeFuncType(params as ValueType[], results as ValueType[]));
    }
  }

  /**
   * Read a vector of value types: count:u32leb + type_byte[count].
   *
   * Used for function parameters and results in the type section.
   *
   * Value type bytes:
   *   0x7F = i32  (32-bit integer)
   *   0x7E = i64  (64-bit integer)
   *   0x7D = f32  (32-bit float)
   *   0x7C = f64  (64-bit float)
   */
  readValueTypeVec(): number[] {
    const count = this.readU32();
    const types: number[] = [];
    for (let i = 0; i < count; i++) {
      const vtOffset = this.pos;
      const b = this.readByte();
      if (!isValidValueType(b)) {
        throw new WasmParseError(
          `Unknown value type byte 0x${b.toString(16)} at offset ${vtOffset}`,
          vtOffset
        );
      }
      types.push(b);
    }
    return types;
  }

  /**
   * Parse the import section (ID 2).
   *
   * Imports let a module request functions, tables, memories, and globals from
   * the host environment. Imports must be satisfied before a module can run.
   *
   * Structure per import:
   *   module_name:str  (e.g. "env")
   *   name:str         (e.g. "memory")
   *   kind:u8          (0=func, 1=table, 2=memory, 3=global)
   *   type_desc        (depends on kind)
   *
   * Kind 0 (function):
   *   type_index:u32leb  → index into the type section
   *
   * Kind 1 (table):
   *   element_type:u8 (always 0x70 = funcref in WASM 1.0)
   *   limits
   *
   * Kind 2 (memory):
   *   limits
   *
   * Kind 3 (global):
   *   globaltype
   */
  parseImportSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const moduleName = this.readString();
      const name = this.readString();
      const kindOffset = this.pos;
      const kindByte = this.readByte();

      let typeInfo: number | TableType | MemoryType | GlobalType;
      let kind: number;

      switch (kindByte) {
        case ExternalKind.FUNCTION: {
          kind = ExternalKind.FUNCTION;
          typeInfo = this.readU32();
          break;
        }
        case ExternalKind.TABLE: {
          kind = ExternalKind.TABLE;
          const etOffset = this.pos;
          const elementType = this.readByte();
          if (elementType !== FUNCREF) {
            throw new WasmParseError(
              `Unknown table element type 0x${elementType.toString(16)} at offset ${etOffset}`,
              etOffset
            );
          }
          const limits = this.readLimits();
          typeInfo = { elementType, limits };
          break;
        }
        case ExternalKind.MEMORY: {
          kind = ExternalKind.MEMORY;
          const limits = this.readLimits();
          typeInfo = { limits };
          break;
        }
        case ExternalKind.GLOBAL: {
          kind = ExternalKind.GLOBAL;
          typeInfo = this.readGlobalType();
          break;
        }
        default:
          throw new WasmParseError(
            `Unknown import kind 0x${kindByte.toString(16)} at offset ${kindOffset}`,
            kindOffset
          );
      }

      module.imports.push({ moduleName, name, kind, typeInfo } as Import);
    }
  }

  /**
   * Parse the function section (ID 3).
   *
   * The function section contains one type-section index per locally-defined
   * function. It does NOT contain code — that's in the code section.
   *
   * Think of it as a registry: "function 0 has signature type[2]."
   *
   * Structure:
   *   count:u32leb
   *   [type_index:u32leb × count]
   *
   * Note: function index space starts AFTER all imported functions. If the
   * module imports 3 functions, local function 0 has overall function index 3.
   */
  parseFunctionSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      module.functions.push(this.readU32());
    }
  }

  /**
   * Parse the table section (ID 4).
   *
   * Tables are indexed arrays of opaque references (in WASM 1.0: function
   * references only). They enable indirect function calls via call_indirect.
   *
   * Structure per table:
   *   element_type:u8 (always 0x70 = funcref in WASM 1.0)
   *   limits
   */
  parseTableSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const etOffset = this.pos;
      const elementType = this.readByte();
      if (elementType !== FUNCREF) {
        throw new WasmParseError(
          `Unknown table element type 0x${elementType.toString(16)} at offset ${etOffset}`,
          etOffset
        );
      }
      const limits = this.readLimits();
      module.tables.push({ elementType, limits });
    }
  }

  /**
   * Parse the memory section (ID 5).
   *
   * Linear memory is the WASM heap — a flat, resizable byte array. WASM 1.0
   * allows at most one memory per module. Memory is measured in 64 KiB pages.
   *
   * Structure per memory:
   *   limits  (min pages, optional max pages)
   *
   * Analogy: like declaring how much RAM a process needs at startup.
   */
  parseMemorySection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const limits = this.readLimits();
      module.memories.push({ limits });
    }
  }

  /**
   * Parse the global section (ID 6).
   *
   * Globals are module-level variables. Each has a type (value type +
   * mutability) and a constant init expression that sets its initial value.
   *
   * Structure per global:
   *   globaltype
   *   init_expr (bytes until 0x0B inclusive)
   *
   * Examples of init expressions:
   *   i32.const 42   → [0x41, 0x2A, 0x0B]
   *   f64.const 0.0  → [0x44, 0,0,0,0,0,0,0,0, 0x0B]  (8-byte IEEE754 + end)
   */
  parseGlobalSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const globalType = this.readGlobalType();
      const initExpr = this.readInitExpr();
      module.globals.push({ globalType, initExpr } as Global);
    }
  }

  /**
   * Parse the export section (ID 7).
   *
   * Exports make module internals visible to the host environment. Without
   * exports, a module is a sealed box that can run but not be called.
   *
   * Structure per export:
   *   name:str
   *   kind:u8  (0=func, 1=table, 2=memory, 3=global)
   *   index:u32leb  (index into the appropriate index space)
   *
   * Example: exporting function at index 0 as "main":
   *   [0x04, 'm', 'a', 'i', 'n', 0x00, 0x00]
   */
  parseExportSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const name = this.readString();
      const kindOffset = this.pos;
      const kindByte = this.readByte();
      if (
        kindByte !== ExternalKind.FUNCTION &&
        kindByte !== ExternalKind.TABLE &&
        kindByte !== ExternalKind.MEMORY &&
        kindByte !== ExternalKind.GLOBAL
      ) {
        throw new WasmParseError(
          `Unknown export kind 0x${kindByte.toString(16)} at offset ${kindOffset}`,
          kindOffset
        );
      }
      const index = this.readU32();
      module.exports.push({ name, kind: kindByte as ExternalKind, index } as Export);
    }
  }

  /**
   * Parse the start section (ID 8).
   *
   * The start section contains a single function index. If present, the WASM
   * runtime calls that function automatically after instantiation, before any
   * exports are callable. It's like a C `main()` or a constructor.
   *
   * Structure:
   *   function_index:u32leb
   */
  parseStartSection(module: WasmModule): void {
    module.start = this.readU32();
  }

  /**
   * Parse the element section (ID 9).
   *
   * Element segments initialize tables with function indices at instantiation.
   * This is how indirect calls work: the table is pre-populated so that
   * `call_indirect` can find the right function at runtime.
   *
   * Structure per element:
   *   table_index:u32leb  (which table; always 0 in WASM 1.0)
   *   offset_expr         (constant init expression = starting slot)
   *   func_count:u32leb
   *   func_indices:u32leb[func_count]
   *
   * Example: place functions [0, 1] into table 0 starting at slot 5.
   *   tableIndex=0, offsetExpr=[0x41,0x05,0x0B], functionIndices=[0,1]
   */
  parseElementSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const tableIndex = this.readU32();
      const offsetExpr = this.readInitExpr();
      const funcCount = this.readU32();
      const functionIndices: number[] = [];
      for (let j = 0; j < funcCount; j++) {
        functionIndices.push(this.readU32());
      }
      module.elements.push({ tableIndex, offsetExpr, functionIndices } as Element);
    }
  }

  /**
   * Parse the code section (ID 10).
   *
   * The code section contains one function body per locally-defined function.
   * Function bodies are paired 1:1 with the function section's type indices:
   * function[i] has type types[functions[i]] and body code[i].
   *
   * Structure per body:
   *   body_size:u32leb             (total byte length of everything below)
   *   local_decl_count:u32leb      (number of local groups, NOT total locals)
   *   for each local group:
   *     group_count:u32leb         (how many locals of this type)
   *     type:u8                    (the ValueType of those locals)
   *   code:remaining_bytes_in_body (raw opcodes, ends with 0x0B)
   *
   * Local groups are run-length encoded for compactness. We expand them:
   * "(3, i32)" means three i32 locals, stored as [i32, i32, i32].
   *
   * Example body: two i32 locals, then `local.get 0; local.get 1; i32.add; end`
   *   body_size=9
   *   [0x01]          ← 1 local group
   *   [0x02, 0x7F]    ← 2 × i32
   *   [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]   ← code
   */
  parseCodeSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const bodySize = this.readU32();
      const bodyStart = this.pos;
      const bodyEnd = bodyStart + bodySize;

      if (bodyEnd > this.data.length) {
        throw new WasmParseError(
          `Code body ${i} extends beyond end of data (offset ${bodyStart}, size ${bodySize})`,
          bodyStart
        );
      }

      // Read local declarations (run-length encoded groups)
      const localDeclCount = this.readU32();
      const locals: ValueType[] = [];
      for (let j = 0; j < localDeclCount; j++) {
        const groupCount = this.readU32();
        const vtOffset = this.pos;
        const typeByte = this.readByte();
        if (!isValidValueType(typeByte)) {
          throw new WasmParseError(
            `Unknown local type byte 0x${typeByte.toString(16)} at offset ${vtOffset}`,
            vtOffset
          );
        }
        // Expand the run-length group: (groupCount, type) → [type, type, ...]
        for (let k = 0; k < groupCount; k++) {
          locals.push(typeByte as ValueType);
        }
      }

      // The remaining bytes (up to bodyEnd) are the raw code
      const codeLength = bodyEnd - this.pos;
      if (codeLength < 0) {
        throw new WasmParseError(
          `Code body ${i} local declarations exceeded body size at offset ${this.pos}`,
          this.pos
        );
      }
      const code = this.readBytes(codeLength);
      module.code.push({ locals, code } as FunctionBody);
    }
  }

  /**
   * Parse the data section (ID 11).
   *
   * Data segments copy raw bytes into linear memory at instantiation.
   * This is how static data (strings, lookup tables, global variables)
   * ends up in WASM memory.
   *
   * Think of it like the .data and .rodata sections in an ELF binary.
   *
   * Structure per segment:
   *   mem_index:u32leb   (which memory; always 0 in WASM 1.0)
   *   offset_expr        (constant expression = base address in memory)
   *   byte_count:u32leb
   *   data:u8[byte_count]
   *
   * Example: write [0x48, 0x65, 0x6C, 0x6C, 0x6F] ("Hello") at address 0x100:
   *   mem_index=0, offsetExpr=[0x41, 0x80, 0x02, 0x0B], data=[0x48...0x6F]
   */
  parseDataSection(module: WasmModule): void {
    const count = this.readU32();
    for (let i = 0; i < count; i++) {
      const memoryIndex = this.readU32();
      const offsetExpr = this.readInitExpr();
      const byteCount = this.readU32();
      const data = this.readBytes(byteCount);
      module.data.push({ memoryIndex, offsetExpr, data } as DataSegment);
    }
  }

  /**
   * Parse a custom section (ID 0).
   *
   * Custom sections are the extension mechanism of WASM. They carry metadata
   * that doesn't affect execution. Well-known custom sections include:
   *   - "name":      debug names for functions, locals, globals
   *   - "producers": compiler/toolchain metadata
   *   - DWARF sections for source-level debugging
   *
   * Structure:
   *   name:str          (section name, e.g. "name")
   *   data:remaining    (raw bytes up to the section boundary)
   *
   * We receive the payload slice (already size-bounded) from the section
   * dispatch loop, so we create a sub-reader over it.
   */
  parseCustomSection(module: WasmModule, payload: Uint8Array): void {
    // Parse name + remaining bytes from the payload slice
    const sub = new BinaryReader(payload);
    const name = sub.readString();
    const data = sub.readBytes(payload.length - sub.offset);
    module.customs.push({ name, data } as CustomSection);
  }

  // ── Top-level module parsing ───────────────────────────────────────────────

  /**
   * parseModule — read and validate the header, then dispatch sections.
   *
   * The header is always 8 bytes:
   *   Bytes 0–3: magic "\0asm"
   *   Bytes 4–7: version (1 as little-endian u32)
   *
   * After the header, we loop reading sections until end-of-file.
   * Each section is: id:u8 + size:u32leb + payload:size_bytes.
   *
   * We track the last seen numbered section ID to enforce ordering.
   * Custom sections (id=0) are exempt from the ordering requirement.
   */
  parseModule(): WasmModule {
    this.validateHeader();
    const module = new WasmModule();
    let lastSectionId = 0; // for ordering enforcement

    while (!this.atEnd()) {
      const sectionIdOffset = this.pos;
      const sectionId = this.readByte();
      const payloadSize = this.readU32();
      const payloadStart = this.pos;
      const payloadEnd = payloadStart + payloadSize;

      if (payloadEnd > this.data.length) {
        throw new WasmParseError(
          `Section ${sectionId} payload extends beyond end of data (offset ${payloadStart}, size ${payloadSize})`,
          payloadStart
        );
      }

      // Enforce ordering for numbered sections (1–11).
      // Custom sections (0) may appear anywhere and don't update lastSectionId.
      if (sectionId !== SECTION_CUSTOM) {
        if (sectionId < lastSectionId) {
          throw new WasmParseError(
            `Section ${sectionId} appears out of order: already saw section ${lastSectionId}`,
            sectionIdOffset
          );
        }
        lastSectionId = sectionId;
      }

      // Extract payload slice so section parsers can't accidentally
      // read past the section boundary.
      const payload = this.data.slice(payloadStart, payloadEnd);

      switch (sectionId) {
        case SECTION_TYPE:
          this.parseTypeSection(module);
          break;
        case SECTION_IMPORT:
          this.parseImportSection(module);
          break;
        case SECTION_FUNCTION:
          this.parseFunctionSection(module);
          break;
        case SECTION_TABLE:
          this.parseTableSection(module);
          break;
        case SECTION_MEMORY:
          this.parseMemorySection(module);
          break;
        case SECTION_GLOBAL:
          this.parseGlobalSection(module);
          break;
        case SECTION_EXPORT:
          this.parseExportSection(module);
          break;
        case SECTION_START:
          this.parseStartSection(module);
          break;
        case SECTION_ELEMENT:
          this.parseElementSection(module);
          break;
        case SECTION_CODE:
          this.parseCodeSection(module);
          break;
        case SECTION_DATA:
          this.parseDataSection(module);
          break;
        case SECTION_CUSTOM:
          this.parseCustomSection(module, payload);
          break;
        default:
          // Unknown section — skip it. Forward-compatibility: future WASM
          // versions may add new section IDs. Skipping is safe.
          break;
      }

      // Always advance to the next section boundary, even if the section
      // parser consumed fewer bytes (e.g., unknown section we skipped).
      this.pos = payloadEnd;
    }

    return module;
  }

  /**
   * validateHeader — check the magic bytes and version.
   *
   * If these 8 bytes are wrong, the data is not a WASM module at all.
   * Common causes of failure:
   *   - Passing a .wat (text-format) file instead of a .wasm binary
   *   - Truncated download (file cut short)
   *   - Byte-order error (version read as big-endian)
   */
  private validateHeader(): void {
    if (this.data.length < 8) {
      throw new WasmParseError(
        `File too short: ${this.data.length} bytes (need at least 8 for the header)`,
        0
      );
    }

    // Check magic bytes: \0asm
    for (let i = 0; i < 4; i++) {
      if (this.data[i] !== WASM_MAGIC[i]) {
        throw new WasmParseError(
          `Invalid magic bytes at offset ${i}: expected 0x${WASM_MAGIC[i].toString(16).padStart(2, "0")}, got 0x${this.data[i].toString(16).padStart(2, "0")}`,
          i
        );
      }
    }
    this.pos = 4;

    // Check version: [0x01, 0x00, 0x00, 0x00]
    for (let i = 0; i < 4; i++) {
      if (this.data[4 + i] !== WASM_VERSION[i]) {
        throw new WasmParseError(
          `Unsupported WASM version at offset ${4 + i}: expected 0x${WASM_VERSION[i].toString(16).padStart(2, "0")}, got 0x${this.data[4 + i].toString(16).padStart(2, "0")}`,
          4 + i
        );
      }
    }
    this.pos = 8;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * isValidValueType — check if a byte is a valid WASM 1.0 value type.
 *
 * Value types are the four numeric types: i32, i64, f32, f64.
 * Their byte codes occupy the range 0x7C–0x7F.
 *
 *   ┌──────┬────────┐
 *   │ Type │ Byte   │
 *   ├──────┼────────┤
 *   │ f64  │ 0x7C   │
 *   │ f32  │ 0x7D   │
 *   │ i64  │ 0x7E   │
 *   │ i32  │ 0x7F   │
 *   └──────┴────────┘
 */
function isValidValueType(b: number): boolean {
  return (
    b === ValueType.I32 ||
    b === ValueType.I64 ||
    b === ValueType.F32 ||
    b === ValueType.F64
  );
}
