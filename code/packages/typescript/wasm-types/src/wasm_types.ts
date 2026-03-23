/**
 * wasm_types.ts — Pure type definitions for the WebAssembly 1.0 type system
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * WHAT IS THE WASM TYPE SYSTEM?
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * WebAssembly (WASM) is a binary instruction format for a stack-based virtual
 * machine. Every value, function, memory region, and import/export in a WASM
 * module is described by a type. This module is the source of truth for all
 * those type definitions.
 *
 * The WASM 1.0 type system is intentionally minimal. It has just four numeric
 * value types, a handful of composite types (function types, table types,
 * memory types, global types), and a handful of structural types (imports,
 * exports, function bodies, etc.). Everything a parser or validator needs to
 * know about *what kinds of things exist* lives here.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * THE WASM BINARY SECTION LAYOUT
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * A valid .wasm file looks like this at the top level:
 *
 *   ┌────────────────────────────────────────────────────────────────────┐
 *   │  Magic bytes: 0x00 0x61 0x73 0x6D  ("\0asm")                      │
 *   │  Version:     0x01 0x00 0x00 0x00  (1)                            │
 *   ├────────────────────────────────────────────────────────────────────┤
 *   │  Section 1:  Type section    → FuncType[]                         │
 *   │  Section 2:  Import section  → Import[]                           │
 *   │  Section 3:  Function section→ type-index[] (one per function)    │
 *   │  Section 4:  Table section   → TableType[]                        │
 *   │  Section 5:  Memory section  → MemoryType[]                       │
 *   │  Section 6:  Global section  → Global[]                           │
 *   │  Section 7:  Export section  → Export[]                           │
 *   │  Section 8:  Start section   → function index (optional)          │
 *   │  Section 9:  Element section → Element[]                          │
 *   │  Section 10: Code section    → FunctionBody[]                     │
 *   │  Section 11: Data section    → DataSegment[]                      │
 *   │  Section 0:  Custom sections → CustomSection[] (any number)       │
 *   └────────────────────────────────────────────────────────────────────┘
 *
 * The WasmModule class in this file mirrors that layout exactly — each field
 * corresponds to one section.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * BINARY ENCODING OVERVIEW
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * Type bytes in the WASM spec are encoded as single-byte "type codes." Because
 * type codes are always negative in the signed 7-bit LEB128 interpretation
 * (their high bit is set), they can be emitted as a single byte.
 *
 * For example, ValueType I32 = 0x7F:
 *   Binary: 0111_1111
 *   As unsigned byte: 127
 *   As signed 7-bit: −1  (but the spec treats it as an unsigned type tag)
 *
 * ─────────────────────────────────────────────────────────────────────────────
 */

// ─────────────────────────────────────────────────────────────────────────────
// ValueType
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ValueType — the four numeric types in WASM 1.0.
 *
 * Every local variable, function parameter, function result, and global
 * variable has exactly one of these four types.
 *
 * Binary encoding (each is a single type-code byte):
 *
 *   ┌──────────┬────────┬────────────────────────────────────────────────┐
 *   │ Name     │ Byte   │ Meaning                                        │
 *   ├──────────┼────────┼────────────────────────────────────────────────┤
 *   │ I32      │ 0x7F   │ 32-bit integer (signed or unsigned by opcode)  │
 *   │ I64      │ 0x7E   │ 64-bit integer                                 │
 *   │ F32      │ 0x7D   │ 32-bit IEEE 754 floating-point                 │
 *   │ F64      │ 0x7C   │ 64-bit IEEE 754 floating-point                 │
 *   └──────────┴────────┴────────────────────────────────────────────────┘
 *
 * Note: WASM 1.0 has no boolean type — booleans are represented as i32
 * (0 = false, any nonzero = true). WASM 2.0 adds v128 (SIMD) and reference
 * types, but those are outside scope here.
 *
 * Usage example:
 *   const t: ValueType = ValueType.I32;  // type-code byte 0x7F
 */
export const ValueType = {
  /** 32-bit integer. Most WASM arithmetic and address computation uses i32. */
  I32: 0x7f,
  /** 64-bit integer. Used for wide arithmetic and 64-bit addresses. */
  I64: 0x7e,
  /** 32-bit IEEE 754 single-precision float. */
  F32: 0x7d,
  /** 64-bit IEEE 754 double-precision float. */
  F64: 0x7c,
} as const;

/** The TypeScript union type for all valid value-type codes. */
export type ValueType = (typeof ValueType)[keyof typeof ValueType];

// ─────────────────────────────────────────────────────────────────────────────
// BlockType
// ─────────────────────────────────────────────────────────────────────────────

/**
 * BlockType — encodes the result type of a structured control block.
 *
 * In WASM 1.0 a block (block/loop/if) can produce either:
 *   - No result:         encoded as the single byte 0x40
 *   - One value result:  encoded as a ValueType byte (0x7F/0x7E/0x7D/0x7C)
 *
 * The 0x40 byte is called the "empty" block type. Conceptually it means the
 * block is a statement (produces no values on the stack) rather than an
 * expression.
 *
 * Binary layout of a block instruction:
 *
 *   block_instr ::= 0x02  bt:blocktype  (instr)*  0x0B
 *
 *   where blocktype is one byte: 0x40 (empty) or a ValueType byte.
 *
 * Example:
 *   [0x02, 0x40, ..., 0x0B]  → block with no result
 *   [0x02, 0x7F, ..., 0x0B]  → block that leaves one i32 on the stack
 */
export const BlockType = {
  /** Block produces no result value (statement, not expression). */
  EMPTY: 0x40,
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// ExternalKind
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ExternalKind — classifies what kind of definition is being imported or
 * exported.
 *
 * WASM modules can import and export four kinds of entities:
 *
 *   ┌──────────┬────────┬───────────────────────────────────────────────────┐
 *   │ Kind     │ Byte   │ What it refers to                                 │
 *   ├──────────┼────────┼───────────────────────────────────────────────────┤
 *   │ FUNCTION │ 0x00   │ A function defined (or imported) in the module    │
 *   │ TABLE    │ 0x01   │ A table (array of function references)            │
 *   │ MEMORY   │ 0x02   │ A linear memory region                           │
 *   │ GLOBAL   │ 0x03   │ A global variable                                │
 *   └──────────┴────────┴───────────────────────────────────────────────────┘
 *
 * Binary encoding example for an import:
 *
 *   [mod_len] [mod_bytes...]   ← module name
 *   [nm_len]  [nm_bytes...]    ← field name
 *   [0x00]                     ← kind = FUNCTION
 *   [type_index uleb128]        ← index into the type section
 *
 * Usage:
 *   const k: ExternalKind = ExternalKind.FUNCTION;
 */
export const ExternalKind = {
  /** A callable function — refers to a type-section index. */
  FUNCTION: 0x00,
  /** A table — an array of opaque references (usually funcref). */
  TABLE: 0x01,
  /** A linear memory — a flat byte array. */
  MEMORY: 0x02,
  /** A global variable — a single typed value, optionally mutable. */
  GLOBAL: 0x03,
} as const;

/** The TypeScript union type for all valid external-kind codes. */
export type ExternalKind = (typeof ExternalKind)[keyof typeof ExternalKind];

// ─────────────────────────────────────────────────────────────────────────────
// FuncType
// ─────────────────────────────────────────────────────────────────────────────

/**
 * FuncType — the type signature of a function.
 *
 * In WASM, every function has a type: a list of parameter types and a list
 * of result types. FuncType stores these lists. WASM 1.0 allows at most one
 * result type (multi-value returns were added in a later proposal), but the
 * type section still encodes results as a vector for future compatibility.
 *
 * Binary encoding:
 *
 *   0x60               ← function type prefix byte
 *   [n uleb128]        ← number of params
 *   [param_type]*n     ← each param ValueType byte
 *   [m uleb128]        ← number of results
 *   [result_type]*m    ← each result ValueType byte
 *
 * Example — function `(i32, i64) → f64`:
 *
 *   0x60 0x02 0x7F 0x7E 0x01 0x7C
 *         ^^^^ ^^^^ ^^^^ ^^^^ ^^^^
 *         2    i32  i64  1    f64
 *
 * The `readonly` annotation on the arrays ensures callers cannot accidentally
 * mutate a type's param/result lists after construction.
 */
export interface FuncType {
  /** Parameter types, in left-to-right declaration order. */
  readonly params: readonly ValueType[];
  /** Result types (WASM 1.0: at most one). */
  readonly results: readonly ValueType[];
}

/**
 * makeFuncType — construct a FuncType with the given params and results.
 *
 * The arrays are frozen via `Object.freeze` so the type is truly immutable
 * at runtime, matching the `readonly` annotation at the type level.
 *
 * Example:
 *   makeFuncType([ValueType.I32, ValueType.I64], [ValueType.F64])
 *   // → { params: [0x7F, 0x7E], results: [0x7C] }
 */
export function makeFuncType(
  params: ValueType[],
  results: ValueType[]
): FuncType {
  return Object.freeze({
    params: Object.freeze([...params]),
    results: Object.freeze([...results]),
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Limits
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Limits — describes the size constraints of a memory or table.
 *
 * WASM memories and tables have a minimum size and an optional maximum size.
 * Sizes are measured in *pages* for memories (1 page = 64 KiB) and in
 * *elements* for tables.
 *
 * Binary encoding:
 *
 *   0x00 [min uleb128]               ← no maximum
 *   0x01 [min uleb128] [max uleb128] ← with maximum
 *
 *   ┌────────┬──────────────────────────────────────────────────────────┐
 *   │ Flags  │ Meaning                                                  │
 *   ├────────┼──────────────────────────────────────────────────────────┤
 *   │  0x00  │ Only a minimum is present; no upper bound.               │
 *   │  0x01  │ Both minimum and maximum are present.                    │
 *   └────────┴──────────────────────────────────────────────────────────┘
 *
 * Example — memory with at least 1 page, at most 4 pages:
 *   0x01 0x01 0x04
 *
 * Example — memory with at least 2 pages, no maximum:
 *   0x00 0x02
 */
export interface Limits {
  /** Minimum number of pages (memory) or elements (table). */
  readonly min: number;
  /**
   * Maximum number of pages/elements, or null if unbounded.
   * When present, a WASM engine may refuse to grow beyond this value.
   */
  readonly max: number | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryType
// ─────────────────────────────────────────────────────────────────────────────

/**
 * MemoryType — the type of a linear memory region.
 *
 * WebAssembly linear memory is a contiguous, mutable byte array. WASM 1.0
 * allows at most one memory per module. Its size is measured in 64-KiB pages.
 *
 * Binary encoding (inside the memory section):
 *
 *   [limits]   ← encoded Limits (see above)
 *
 * The maximum addressable memory in WASM 1.0 is 65536 pages = 4 GiB.
 *
 * Example — one memory starting at 1 page (64 KiB), growing up to 16 pages (1 MiB):
 *
 *   MemoryType { limits: { min: 1, max: 16 } }
 */
export interface MemoryType {
  readonly limits: Limits;
}

// ─────────────────────────────────────────────────────────────────────────────
// TableType
// ─────────────────────────────────────────────────────────────────────────────

/**
 * TableType — the type of a WASM table.
 *
 * A table is an indexed array of opaque references. In WASM 1.0, the only
 * allowed element type is `funcref` (0x70) — a reference to a function.
 * Tables support indirect function calls: `call_indirect` uses a runtime
 * integer index into the table to call an arbitrary function.
 *
 * Binary encoding (inside the table section):
 *
 *   0x70       ← element type: funcref
 *   [limits]   ← encoded Limits
 *
 * The element type is always 0x70 in WASM 1.0. The reference-types proposal
 * (WASM 2.0) allows 0x6F (externref) as well, but that is outside scope.
 *
 * Example:
 *   TableType { elementType: 0x70, limits: { min: 10, max: null } }
 *   → a table of at least 10 function references, no maximum.
 */
export interface TableType {
  /**
   * Element type tag. Always 0x70 (funcref) in WASM 1.0.
   * Stored as a number to remain forward-compatible with reference types.
   */
  readonly elementType: number;
  readonly limits: Limits;
}

/** The funcref element-type byte used in TableType. */
export const FUNCREF = 0x70;

// ─────────────────────────────────────────────────────────────────────────────
// GlobalType
// ─────────────────────────────────────────────────────────────────────────────

/**
 * GlobalType — the type of a global variable.
 *
 * Globals hold a single value of a ValueType. They are either immutable
 * (constants) or mutable (writable from within the module, but not from the
 * host unless exported and the host uses JS API).
 *
 * Binary encoding (inside the global section or an import):
 *
 *   [valueType byte]   ← one of 0x7F/0x7E/0x7D/0x7C
 *   [mutability]       ← 0x00 = immutable, 0x01 = mutable
 *
 * Example — immutable i32 global:
 *   0x7F 0x00
 *
 * Example — mutable f64 global:
 *   0x7C 0x01
 *
 * Mutability truth table:
 *   ┌───────────┬────────┬──────────────────────────────────────────────┐
 *   │ mutable   │ byte   │ Allowed operations                           │
 *   ├───────────┼────────┼──────────────────────────────────────────────┤
 *   │  false    │  0x00  │ global.get only                              │
 *   │  true     │  0x01  │ global.get and global.set                    │
 *   └───────────┴────────┴──────────────────────────────────────────────┘
 */
export interface GlobalType {
  readonly valueType: ValueType;
  /** true → global.set is allowed; false → read-only constant. */
  readonly mutable: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Import
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Import — a single import declaration.
 *
 * A WASM module can import functions, tables, memories, and globals from the
 * host environment. Each import is identified by a two-part name: the module
 * name (e.g. "env") and the field name (e.g. "memory").
 *
 * Binary encoding (inside the import section):
 *
 *   [mod_len uleb128]     ← byte-length of module name
 *   [mod_bytes...] UTF-8  ← module name string
 *   [nm_len uleb128]      ← byte-length of field name
 *   [nm_bytes...] UTF-8   ← field name string
 *   [kind byte]           ← ExternalKind (0x00–0x03)
 *   [kind-specific data]  ← depends on kind (see below)
 *
 * Kind-specific typeInfo:
 *   FUNCTION → number   (index into the type section, a FuncType)
 *   TABLE    → TableType
 *   MEMORY   → MemoryType
 *   GLOBAL   → GlobalType
 *
 * Example — importing a function at type index 2 from "env"."add":
 *   Import {
 *     moduleName: "env",
 *     name: "add",
 *     kind: ExternalKind.FUNCTION,
 *     typeInfo: 2
 *   }
 */
export interface Import {
  /** The module namespace, e.g. "env" or "wasi_snapshot_preview1". */
  readonly moduleName: string;
  /** The field name within the module namespace. */
  readonly name: string;
  /** What kind of entity is being imported. */
  readonly kind: ExternalKind;
  /**
   * Type information, whose shape depends on `kind`:
   *   - FUNCTION: number (type-section index)
   *   - TABLE:    TableType
   *   - MEMORY:   MemoryType
   *   - GLOBAL:   GlobalType
   */
  readonly typeInfo: number | TableType | MemoryType | GlobalType;
}

// ─────────────────────────────────────────────────────────────────────────────
// Export
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Export — a single export declaration.
 *
 * Exports make internal definitions (functions, tables, memories, globals)
 * visible to the host environment under a string name.
 *
 * Binary encoding (inside the export section):
 *
 *   [nm_len uleb128]      ← byte-length of export name
 *   [nm_bytes...] UTF-8   ← export name string
 *   [kind byte]           ← ExternalKind (0x00–0x03)
 *   [index uleb128]       ← index into the appropriate index space
 *
 * Example — exporting function at index 0 as "main":
 *   Export { name: "main", kind: ExternalKind.FUNCTION, index: 0 }
 */
export interface Export {
  /** The name under which this definition is visible to the host. */
  readonly name: string;
  /** What kind of entity is exported. */
  readonly kind: ExternalKind;
  /**
   * Index into the corresponding index space:
   *   - FUNCTION: function index (imports first, then module-defined)
   *   - TABLE:    table index
   *   - MEMORY:   memory index
   *   - GLOBAL:   global index
   */
  readonly index: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Global
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Global — a module-defined global variable with its initializer.
 *
 * Each global in the global section has a type (GlobalType) and an
 * initialization expression. The init expression is a sequence of WASM
 * instructions that produces a single constant value — stored here as raw
 * bytes (including the trailing `end` opcode 0x0B).
 *
 * In WASM 1.0 the init expression must be a *constant expression*:
 *   - `i32.const N` (opcode 0x41 followed by a signed LEB128)
 *   - `i64.const N` (opcode 0x42)
 *   - `f32.const N` (opcode 0x43 + 4 bytes IEEE754 little-endian)
 *   - `f64.const N` (opcode 0x44 + 8 bytes IEEE754 little-endian)
 *   - `global.get N` (opcode 0x23 — only for imported globals)
 *
 * Binary encoding (inside the global section):
 *
 *   [globalType]       ← GlobalType encoding
 *   [init_expr bytes]  ← constant-expression opcodes + 0x0B (end)
 *
 * Example — immutable i32 global with value 42:
 *   globalType: { valueType: 0x7F, mutable: false }
 *   initExpr:   Uint8Array [0x41, 0x2A, 0x0B]
 *                           ^     ^     ^
 *                           i32.const 42 end
 */
export interface Global {
  readonly globalType: GlobalType;
  /**
   * Raw bytes of the constant init expression, including the trailing
   * `end` opcode (0x0B).
   */
  readonly initExpr: Uint8Array;
}

// ─────────────────────────────────────────────────────────────────────────────
// Element
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Element — a table initializer segment.
 *
 * Element segments populate a table with function references at module
 * instantiation time. This is how `call_indirect` can call the right function:
 * the table is pre-filled with function indices by element segments.
 *
 * Binary encoding (inside the element section):
 *
 *   [tableIndex uleb128]     ← which table to fill (always 0 in WASM 1.0)
 *   [offsetExpr bytes]       ← constant-expression giving start index
 *   [count uleb128]          ← number of function-index entries
 *   [funcIndex uleb128]*     ← one entry per function reference
 *
 * Example — fill table 0 starting at index 5 with functions [0, 1, 2]:
 *   tableIndex:      0
 *   offsetExpr:      Uint8Array [0x41, 0x05, 0x0B]  // i32.const 5; end
 *   functionIndices: [0, 1, 2]
 */
export interface Element {
  /** Index of the table to initialize (always 0 in WASM 1.0). */
  readonly tableIndex: number;
  /**
   * Constant-expression bytes yielding the starting slot index in the table.
   * Includes the trailing `end` byte (0x0B).
   */
  readonly offsetExpr: Uint8Array;
  /** Function indices to write into successive table slots. */
  readonly functionIndices: readonly number[];
}

// ─────────────────────────────────────────────────────────────────────────────
// DataSegment
// ─────────────────────────────────────────────────────────────────────────────

/**
 * DataSegment — a linear-memory initializer.
 *
 * Data segments copy bytes into linear memory at instantiation time. They are
 * how compiled programs get read-only string data, lookup tables, and other
 * static data into memory.
 *
 * Binary encoding (inside the data section):
 *
 *   [memoryIndex uleb128]    ← which memory (always 0 in WASM 1.0)
 *   [offsetExpr bytes]       ← constant-expression giving start address
 *   [byteCount uleb128]      ← number of data bytes
 *   [data bytes]*            ← raw byte payload
 *
 * Example — write "Hello" (UTF-8) into memory 0 at address 0x100:
 *   memoryIndex: 0
 *   offsetExpr:  Uint8Array [0x41, 0x80, 0x02, 0x0B]  // i32.const 256; end
 *   data:        Uint8Array [0x48, 0x65, 0x6C, 0x6C, 0x6F]  // "Hello"
 */
export interface DataSegment {
  /** Index of the memory to initialize (always 0 in WASM 1.0). */
  readonly memoryIndex: number;
  /**
   * Constant-expression bytes yielding the byte address in memory
   * where the data will be written. Includes the trailing `end` (0x0B).
   */
  readonly offsetExpr: Uint8Array;
  /** The raw bytes to copy into memory. */
  readonly data: Uint8Array;
}

// ─────────────────────────────────────────────────────────────────────────────
// FunctionBody
// ─────────────────────────────────────────────────────────────────────────────

/**
 * FunctionBody — the body of a module-defined function.
 *
 * The code section contains one FunctionBody per module-defined function
 * (in the same order as the function section's type-index list). Each body
 * has a local-variable declaration list and a raw byte sequence of opcodes.
 *
 * Binary encoding (inside the code section):
 *
 *   [body_size uleb128]              ← total byte length of the body
 *   [local_count uleb128]            ← number of local-decl groups
 *   [local_decl]*                    ← each: [count uleb128] [type byte]
 *   [code bytes...]                  ← instruction sequence
 *   0x0B                             ← end opcode (implicit in body_size)
 *
 * Note: the `locals` field here stores the *expanded* local types (one
 * entry per local), not the compressed run-length encoding in the binary.
 * That decompression is the parser's job.
 *
 * Example — function body with two i32 locals and some code:
 *   locals: [ValueType.I32, ValueType.I32]
 *   code:   Uint8Array [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]
 *                       local.get 0  local.get 1  i32.add  end
 */
export interface FunctionBody {
  /**
   * The types of all local variables declared in this function body
   * (parameters are NOT included — those are in the FuncType).
   */
  readonly locals: readonly ValueType[];
  /**
   * Raw opcode bytes for the function body, including the trailing
   * `end` opcode (0x0B).
   */
  readonly code: Uint8Array;
}

// ─────────────────────────────────────────────────────────────────────────────
// CustomSection
// ─────────────────────────────────────────────────────────────────────────────

/**
 * CustomSection — an arbitrary named byte blob.
 *
 * Custom sections (section id 0) can appear anywhere in a .wasm file and are
 * used for metadata that does not affect module semantics. Common uses:
 *   - "name" section: debug names for functions, locals, globals
 *   - "producers" section: compiler/tool information
 *   - DWARF debug info
 *   - Source maps
 *
 * Binary encoding:
 *
 *   0x00                         ← section id: custom
 *   [section_size uleb128]       ← total byte size of name + data
 *   [name_len uleb128]           ← byte-length of the name string
 *   [name bytes...] UTF-8        ← name of the custom section
 *   [data bytes...]              ← raw payload (section-specific format)
 *
 * Example — a "name" custom section:
 *   CustomSection { name: "name", data: Uint8Array([...name subsections...]) }
 */
export interface CustomSection {
  /** The name of this custom section (e.g. "name", "producers"). */
  readonly name: string;
  /** Raw byte payload (format is specific to each named section). */
  readonly data: Uint8Array;
}

// ─────────────────────────────────────────────────────────────────────────────
// WasmModule
// ─────────────────────────────────────────────────────────────────────────────

/**
 * WasmModule — a mutable in-memory representation of a WASM 1.0 module.
 *
 * This class mirrors the eleven standard sections of the WASM binary format
 * plus zero or more custom sections. A parser reads the binary and populates
 * these arrays; a code generator or validator reads them.
 *
 * The fields are deliberately mutable arrays so a parser can `push()` entries
 * as it scans sections. Immutability of individual entries is enforced by the
 * interface types (e.g. `readonly` FuncType arrays).
 *
 * Section → field mapping:
 *
 *   ┌─────────────────────┬───────────────────────────────────────────────┐
 *   │ WASM Section        │ WasmModule field                              │
 *   ├─────────────────────┼───────────────────────────────────────────────┤
 *   │ 1  Type             │ types:     FuncType[]                         │
 *   │ 2  Import           │ imports:   Import[]                           │
 *   │ 3  Function         │ functions: number[]  (type-section indices)   │
 *   │ 4  Table            │ tables:    TableType[]                        │
 *   │ 5  Memory           │ memories:  MemoryType[]                       │
 *   │ 6  Global           │ globals:   Global[]                           │
 *   │ 7  Export           │ exports:   Export[]                           │
 *   │ 8  Start            │ start:     number | null                      │
 *   │ 9  Element          │ elements:  Element[]                          │
 *   │ 10 Code             │ code:      FunctionBody[]                     │
 *   │ 11 Data             │ data:      DataSegment[]                      │
 *   │ 0  Custom (many)    │ customs:   CustomSection[]                    │
 *   └─────────────────────┴───────────────────────────────────────────────┘
 *
 * Usage:
 *   const mod = new WasmModule();
 *   mod.types.push(makeFuncType([ValueType.I32], [ValueType.I32]));
 *   mod.functions.push(0);  // function 0 has type at index 0
 */
export class WasmModule {
  /** Function type signatures (type section). */
  types: FuncType[] = [];

  /** Imported definitions from the host (import section). */
  imports: Import[] = [];

  /**
   * Type-section indices for each module-defined function (function section).
   * functions[i] is the index into `types` for the i-th function body.
   * Note: the "function index space" includes imported functions first.
   */
  functions: number[] = [];

  /** Table definitions (table section). WASM 1.0 allows at most one. */
  tables: TableType[] = [];

  /** Memory definitions (memory section). WASM 1.0 allows at most one. */
  memories: MemoryType[] = [];

  /** Module-defined globals with their init expressions (global section). */
  globals: Global[] = [];

  /** Exported definitions, visible to the host (export section). */
  exports: Export[] = [];

  /**
   * The index of the start function, or null if absent (start section).
   * When non-null, the WASM engine calls this function automatically at
   * instantiation time, before the module is otherwise usable.
   */
  start: number | null = null;

  /** Table initializer segments (element section). */
  elements: Element[] = [];

  /** Function bodies in the same order as `functions` (code section). */
  code: FunctionBody[] = [];

  /** Memory initializer segments (data section). */
  data: DataSegment[] = [];

  /** Custom (non-standard) sections, e.g. debug names (custom sections). */
  customs: CustomSection[] = [];

  constructor() {
    // All fields initialized to empty arrays and start to null above.
    // A no-arg constructor is provided for explicitness and documentation.
  }
}
