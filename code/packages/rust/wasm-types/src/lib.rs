//! # wasm-types
//!
//! Pure type definitions for the WebAssembly 1.0 (MVP) type system.
//!
//! This crate contains no parsing logic — it only defines the data structures
//! that represent a decoded WASM module's type information. Higher-level crates
//! like `wasm-opcodes` and `wasm-module-parser` depend on these definitions.
//!
//! ## Where types live in the WASM binary format
//!
//! A `.wasm` file is a sequence of **sections**. Each section has an ID byte,
//! a byte-length, then contents.  The types in this crate mirror the decoded
//! contents of those sections:
//!
//! ```text
//! .wasm file layout
//! ┌──────────────────────────────────────────────────────────┐
//! │ Magic: 0x00 0x61 0x73 0x6D  ("asm")                     │
//! │ Version: 0x01 0x00 0x00 0x00                             │
//! ├──────┬────────────────────────────────────────────────── │
//! │ §  1 │ Type section   → Vec<FuncType>                    │
//! │ §  2 │ Import section → Vec<Import>                      │
//! │ §  3 │ Function section → Vec<u32> (type indices)        │
//! │ §  4 │ Table section  → Vec<TableType>                   │
//! │ §  5 │ Memory section → Vec<MemoryType>                  │
//! │ §  6 │ Global section → Vec<Global>                      │
//! │ §  7 │ Export section → Vec<Export>                      │
//! │ §  8 │ Start section  → Option<u32>                      │
//! │ §  9 │ Element section → Vec<Element>                    │
//! │ § 10 │ Code section   → Vec<FunctionBody>                │
//! │ § 11 │ Data section   → Vec<DataSegment>                 │
//! │ §  0 │ Custom sections (name = "name", debug info, etc.) │
//! └──────┴────────────────────────────────────────────────────
//! ```
//!
//! ## Numeric types and LEB128
//!
//! All integers in WASM binaries are encoded as
//! [LEB128](https://en.wikipedia.org/wiki/LEB128) variable-length integers.
//! The `wasm-leb128` crate handles that encoding; this crate uses plain Rust
//! integers in its structs because we represent the *decoded* form.
//!
//! ## This crate is part of coding-adventures
//!
//! A ground-up implementation of the computing stack from transistors to
//! operating systems, written in multiple languages for learning purposes.

// ──────────────────────────────────────────────────────────────────────────────
// ValueType
// ──────────────────────────────────────────────────────────────────────────────

/// The value types that WASM supports, extended for WasmGC (2023).
///
/// WASM 1.0 has four numeric types.  The WasmGC proposal (standardised 2023,
/// shipping in V8 ≥ 119 / Chrome ≥ 119 and Firefox ≥ 120) adds reference
/// types: `anyref`, `i31ref`, and concrete struct/array references.
///
/// ## Numeric types (WASM 1.0)
///
/// ```text
/// Byte encoding in WASM binary
/// ┌────────┬──────┬────────────────────────────────────────────┐
/// │  Type  │ Byte │ Description                                │
/// ├────────┼──────┼────────────────────────────────────────────┤
/// │  i32   │ 0x7F │ 32-bit integer (signed or unsigned)        │
/// │  i64   │ 0x7E │ 64-bit integer (signed or unsigned)        │
/// │  f32   │ 0x7D │ 32-bit IEEE 754 float                      │
/// │  f64   │ 0x7C │ 64-bit IEEE 754 float                      │
/// └────────┴──────┴────────────────────────────────────────────┘
/// ```
///
/// Note: WASM 1.0 has no boolean type. Boolean results (e.g., from `i32.eq`)
/// are represented as `i32`, where 0 means false and any non-zero means true.
///
/// Note: The byte values count *down* from 0x7F. This is because the WASM
/// binary format uses *signed* LEB128 for type bytes.  0x7F is -1 in signed
/// LEB128, 0x7E is -2, etc.  Newer WASM proposals continue the pattern.
///
/// ## WasmGC reference types
///
/// WasmGC extends the type system with *managed references* — values that
/// point into the GC heap, not into linear memory.  Think of them like
/// Java/JVM object references: the runtime manages their lifetime.
///
/// ```text
/// ┌────────────────────────┬──────────────────────┬──────────────────────────┐
/// │  Type                  │ Byte encoding        │ Description              │
/// ├────────────────────────┼──────────────────────┼──────────────────────────┤
/// │  anyref (null any)     │ 0x6E                 │ Nullable "top" ref type  │
/// │  i31ref (non-null i31) │ 0x6C                 │ Boxed 31-bit integer     │
/// │  (ref null $T)         │ 0x63 + LEB128(idx)   │ Nullable concrete struct │
/// └────────────────────────┴──────────────────────┴──────────────────────────┘
/// ```
///
/// `anyref` (= `(ref null any)`) is the supertype of all GC-managed values.
/// In WasmGC, a `LispyPair` struct field can be typed `anyref` to hold *any*
/// GC value — a cons cell, an integer box, or null.  This mirrors how
/// dynamic Lisp stores atoms and pairs in the same slot.
///
/// `i31ref` is a special type that boxes a 31-bit signed integer *without*
/// allocating heap memory.  The runtime stores the value in the reference
/// pointer bits (like V8's small-integer tagging trick).  Unboxing is done
/// with `i31.get_s`.
///
/// `StructRef(idx)` is a *nullable concrete reference* to a specific named
/// struct type at type-section index `idx`.  The binary encoding is a 2-byte
/// sequence `0x63 <LEB128(idx)>`.
///
/// ## Encoding note
///
/// Because `Anyref`, `I31ref`, and `StructRef` have variable-length binary
/// encodings (unlike the single-byte numeric types), `ValueType` can no
/// longer use `#[repr(u8)]`.  The encoder calls [`ValueType::encode`] to
/// emit the correct byte sequence.
// All variants are trivially Copy: the only data-bearing variant is
// StructRef(u32), and u32: Copy.  Adding Copy back lets the wasm-execution
// interpreter call default_for(*local_type) without a clone().
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// 32-bit integer. Used for booleans, pointers (in linear memory), chars.
    I32,
    /// 64-bit integer. Used for 64-bit arithmetic and 64-bit pointers.
    I64,
    /// 32-bit IEEE 754 single-precision float.
    F32,
    /// 64-bit IEEE 754 double-precision float.
    F64,

    /// `(ref null any)` — nullable any-reference (WasmGC supertype).
    ///
    /// Encoded as a single byte `0x6E`.  Any GC-managed value (struct
    /// reference, i31ref, null) is assignment-compatible with `anyref`.
    /// This is the natural type for a dynamically-typed Lisp value slot.
    Anyref,

    /// `(ref i31)` — boxed 31-bit signed integer (WasmGC).
    ///
    /// Encoded as `0x6C`.  Unlike heap structs, i31ref values do not live
    /// on the GC heap; the runtime encodes the integer directly in the
    /// pointer bits.  This makes integer boxing/unboxing essentially free.
    ///
    /// ```text
    /// i31.new   : [i32] → [i31ref]   — box an i32 (top bit dropped)
    /// i31.get_s : [i31ref] → [i32]   — unbox with sign extension
    /// ```
    I31ref,

    /// `(ref null $T)` — nullable reference to a concrete struct type (WasmGC).
    ///
    /// The `u32` payload is the index into the type section that names struct
    /// type `$T`.  Encoded as `0x63` followed by LEB128(index).
    ///
    /// Example: if `$LispyPair` is defined at type-section index 1, then a
    /// local variable holding a nullable `$LispyPair` reference has type
    /// `StructRef(1)` and is encoded as `[0x63, 0x01]`.
    StructRef(u32),
}

impl ValueType {
    /// Return the single-byte tag for this type, or `None` for multi-byte types.
    ///
    /// This mirrors the WASM 1.0 convention where numeric types are single
    /// bytes.  WasmGC types like `StructRef` need two or more bytes; callers
    /// that need the full encoding should use [`ValueType::encode`] instead.
    ///
    /// ```text
    /// I32 → Some(0x7F),  I64 → Some(0x7E),  F32 → Some(0x7D),  F64 → Some(0x7C)
    /// Anyref → Some(0x6E),  I31ref → Some(0x6C)
    /// StructRef(_) → None   (needs 2+ bytes)
    /// ```
    pub fn byte_tag(&self) -> Option<u8> {
        match self {
            ValueType::I32 => Some(0x7F),
            ValueType::I64 => Some(0x7E),
            ValueType::F32 => Some(0x7D),
            ValueType::F64 => Some(0x7C),
            ValueType::Anyref => Some(0x6E),
            ValueType::I31ref => Some(0x6C),
            ValueType::StructRef(_) => None,
        }
    }

    /// Encode this `ValueType` into its WASM binary representation.
    ///
    /// ```text
    /// I32        → [0x7F]
    /// I64        → [0x7E]
    /// F32        → [0x7D]
    /// F64        → [0x7C]
    /// Anyref     → [0x6E]
    /// I31ref     → [0x6C]
    /// StructRef(n) → [0x63, ...LEB128(n)]
    /// ```
    ///
    /// The LEB128 encoding for small indices fits in one byte, so
    /// `StructRef(0)` → `[0x63, 0x00]` and `StructRef(127)` → `[0x63, 0x7F]`.
    pub fn encode(&self) -> Vec<u8> {
        use wasm_leb128::encode_unsigned;
        match self {
            ValueType::I32 => vec![0x7F],
            ValueType::I64 => vec![0x7E],
            ValueType::F32 => vec![0x7D],
            ValueType::F64 => vec![0x7C],
            ValueType::Anyref => vec![0x6E],
            ValueType::I31ref => vec![0x6C],
            ValueType::StructRef(idx) => {
                // 0x63 = nullable concrete reference tag.
                let mut bytes = vec![0x63u8];
                bytes.extend(encode_unsigned(*idx as u64));
                bytes
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// WasmGC struct types
// ──────────────────────────────────────────────────────────────────────────────

/// One field in a WasmGC struct type.
///
/// In a `.wat` module, this corresponds to:
/// ```wat
/// (field $name (mut <val_type>))   ;; mutable
/// (field $name <val_type>)         ;; immutable
/// ```
///
/// The binary encoding of a field in the type section is:
/// ```text
/// <val_type encoding>   ;; the ValueType bytes
/// <mutability: 0x00 or 0x01>
/// ```
///
/// For a `LispyPair`, both `$head` and `$tail` are mutable `anyref` fields:
/// ```text
/// field $head: [0x6E, 0x01]   ;; anyref, mutable
/// field $tail: [0x6E, 0x01]   ;; anyref, mutable
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldType {
    /// The type stored in this field.
    pub val_type: ValueType,
    /// Whether this field can be modified after the struct is created.
    /// `true` → mutable (heap write is legal via `struct.set`).
    /// `false` → immutable (write-once, set during `struct.new`).
    pub mutable: bool,
}

/// A WasmGC struct type definition — an ordered list of fields.
///
/// In the WAT text format, this looks like:
/// ```wat
/// (type $LispyPair (struct
///   (field $head (mut (ref null any)))
///   (field $tail (mut (ref null any)))))
/// ```
///
/// ## Why structs?
///
/// WasmGC structs are the GC heap analogue of C structs.  They are
/// *heterogeneous* (fields can have different types) and *garbage-collected*
/// (the runtime tracks them; no `free` needed).  For a Lisp runtime, each
/// cons cell is a two-field struct with `$head` (the car) and `$tail` (the
/// cdr), both of type `anyref` so they can hold any Lisp value.
///
/// ## Type-section index
///
/// In the WASM binary, struct types live alongside function types in the
/// type section.  We store them separately in [`WasmModule::struct_types`]
/// and interleave them with function types during encoding.  Struct types
/// appear *after* all function types, so if there are `N` function types,
/// struct type `k` is at type-section index `N + k`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructType {
    /// The fields of this struct, in declaration order.
    /// Field index 0 is the first field, 1 the second, and so on.
    pub fields: Vec<FieldType>,
}

// ──────────────────────────────────────────────────────────────────────────────
// BlockType
// ──────────────────────────────────────────────────────────────────────────────

/// The "result type" of a structured control flow block (`block`, `loop`, `if`).
///
/// In WASM 1.0, a block can either produce no value, produce a single value,
/// or (with the multi-value proposal) produce multiple values by referencing a
/// `FuncType` in the type section.
///
/// ```text
/// block  ;; begins a block
///   i32.const 42
/// end    ;; leaves 42 on the stack if block_type = Value(I32)
/// ```
///
/// The byte encoding in the WASM binary:
/// ```text
///  0x40  →  Empty  (no result)
///  0x7F  →  Value(I32)
///  0x7E  →  Value(I64)
///  0x7D  →  Value(F32)
///  0x7C  →  Value(F64)
///  u32   →  TypeIndex(n)  (non-negative LEB128 integer)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockType {
    /// The block produces no values (the most common case).
    Empty,
    /// The block produces exactly one value of this type.
    Value(ValueType),
    /// The block's signature is a full function type from the type section.
    /// This is the "multi-value" extension to WASM 1.0.
    TypeIndex(u32),
}

/// The byte tag for an empty block type in the WASM binary format.
///
/// ```text
/// 0x40 in hex = 64 in decimal = -64 in signed LEB128
/// ```
pub const BLOCK_TYPE_EMPTY: u8 = 0x40;

// ──────────────────────────────────────────────────────────────────────────────
// ExternalKind
// ──────────────────────────────────────────────────────────────────────────────

/// What kind of entity is imported from or exported to the host environment.
///
/// A WASM module's boundary with the outside world is entirely described by
/// imports and exports. Both use `ExternalKind` to say *what* is being
/// imported/exported.
///
/// ```text
/// Byte encoding
/// ┌──────────┬──────┐
/// │  Kind    │ Byte │
/// ├──────────┼──────┤
/// │ Function │ 0x00 │
/// │ Table    │ 0x01 │
/// │ Memory   │ 0x02 │
/// │ Global   │ 0x03 │
/// └──────────┴──────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExternalKind {
    /// A function — the most commonly imported/exported entity.
    Function = 0x00,
    /// A table — an array of references (function pointers in WASM 1.0).
    Table = 0x01,
    /// A memory — a linear block of bytes shared with the host.
    Memory = 0x02,
    /// A global variable.
    Global = 0x03,
}

// ──────────────────────────────────────────────────────────────────────────────
// FuncType
// ──────────────────────────────────────────────────────────────────────────────

/// A function's type signature: parameter types and result types.
///
/// WASM 1.0 allows at most one result type (the multi-value proposal lifts
/// this restriction, but the *type section* already supports vectors of
/// results even in 1.0). All function signatures are stored in the **type
/// section** and referenced by index elsewhere in the binary.
///
/// ```text
/// Binary encoding (in the type section):
///   0x60                  ;; function type tag
///   <num_params: LEB128>  ;; number of parameters
///   <param_type>*         ;; one byte per param (ValueType encoding)
///   <num_results: LEB128>
///   <result_type>*
///
/// Example: (i32, i64) -> f32
///   0x60  02  7F 7E  01  7D
///   │     │   │  │   │   └── result: F32
///   │     │   │  │   └────── 1 result
///   │     │   │  └────────── param[1]: I64
///   │     │   └──────────── param[0]: I32
///   │     └──────────────── 2 params
///   └────────────────────── func type tag
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncType {
    /// The types of the function's parameters, in order.
    pub params: Vec<ValueType>,
    /// The types of the function's return values, in order.
    pub results: Vec<ValueType>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Limits
// ──────────────────────────────────────────────────────────────────────────────

/// Size constraints (min and optional max) for a memory or table.
///
/// Sizes are in *pages* for memories (1 page = 64 KiB = 65536 bytes) and
/// in *entries* for tables.
///
/// ```text
/// Binary encoding:
///   0x00  <min: LEB128>            ;; no maximum
///   0x01  <min: LEB128>  <max: LEB128>  ;; with maximum
///
/// Example: at least 1 page, at most 4 pages
///   0x01  01  04
/// ```
///
/// The WASM spec requires `min <= max` when a maximum is specified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// Minimum size (must always be present).
    pub min: u32,
    /// Optional maximum size. `None` means unbounded.
    pub max: Option<u32>,
}

// ──────────────────────────────────────────────────────────────────────────────
// MemoryType
// ──────────────────────────────────────────────────────────────────────────────

/// The type of a linear memory — just its size limits.
///
/// WASM 1.0 allows at most one memory per module. A memory is a contiguous
/// array of bytes that the module and host can both read and write. It can
/// grow at runtime via the `memory.grow` instruction (up to `limits.max`).
///
/// ```text
/// Host (JavaScript)           WASM module
/// ┌──────────────────────────────────────┐
/// │ memory = new WebAssembly.Memory(     │
/// │   { initial: 1, maximum: 4 }         │
/// │ )                                    │
/// └──────────────────────────────────────┘
///      limits = Limits { min: 1, max: Some(4) }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryType {
    /// The size constraints on this memory.
    pub limits: Limits,
}

// ──────────────────────────────────────────────────────────────────────────────
// TableType
// ──────────────────────────────────────────────────────────────────────────────

/// The `funcref` element type — the only table element type in WASM 1.0.
///
/// WASM 1.0 tables hold function references. The byte value 0x70 is the tag
/// for `funcref` in the binary format. (The `externref` type was added in a
/// later proposal.)
pub const FUNCREF: u8 = 0x70;

/// The type of a WASM table: an array of references with size limits.
///
/// Tables in WASM 1.0 hold function references (`funcref`). They are used
/// to implement indirect function calls: the `call_indirect` instruction
/// takes an index into the table and calls the function stored there.
///
/// ```text
/// Table layout (conceptually):
///
///   index:    0         1         2         3
///           ┌─────────┬─────────┬─────────┬─────────┐
///           │ func #5 │  null   │ func #2 │ func #7 │  ...
///           └─────────┴─────────┴─────────┴─────────┘
///
/// call_indirect type_idx table_idx
///   → pops an i32 index, looks up function ref, validates type, calls it
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableType {
    /// The reference type stored in this table.
    /// In WASM 1.0, this is always `FUNCREF` (0x70).
    pub element_type: u8,
    /// The size constraints on this table.
    pub limits: Limits,
}

// ──────────────────────────────────────────────────────────────────────────────
// GlobalType
// ──────────────────────────────────────────────────────────────────────────────

/// The type of a global variable: its value type and mutability.
///
/// Immutable globals are constants (e.g., the base address of a data section).
/// Mutable globals hold state that can change across calls (e.g., a stack
/// pointer for a language runtime).
///
/// ```text
/// Binary encoding:
///   <value_type: byte>  <mutability: 0x00 or 0x01>
///
/// Example: mutable i32
///   7F 01
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalType {
    /// The type of value stored in this global.
    pub value_type: ValueType,
    /// Whether this global can be modified after initialization.
    /// `true` → mutable (`var`), `false` → immutable (`const`).
    pub mutable: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// Import / ImportTypeInfo
// ──────────────────────────────────────────────────────────────────────────────

/// Additional type information specific to each import kind.
///
/// An import declaration says "I need *this* from the host environment." The
/// `ImportTypeInfo` says what shape that thing must have.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportTypeInfo {
    /// Import a function; carries the index into the module's type section.
    Function(u32),
    /// Import a table; carries the table's type.
    Table(TableType),
    /// Import a memory; carries the memory's type.
    Memory(MemoryType),
    /// Import a global; carries the global's type.
    Global(GlobalType),
}

/// A single import declaration from the import section.
///
/// Every import names the *module* that provides it (e.g., `"env"`, `"wasi_snapshot_preview1"`)
/// and the *name* within that module (e.g., `"memory"`, `"fd_write"`).
///
/// ```text
/// Binary encoding (import section entry):
///   <module_name: length-prefixed UTF-8>
///   <name: length-prefixed UTF-8>
///   <kind: ExternalKind byte>
///   <type_info: varies by kind>
///
/// Example: import function "env"."abort" with type index 0
///   03 "env"  05 "abort"  00  00
///   │         │           │   └── type index 0
///   │         │           └────── ExternalKind::Function
///   │         └────────────────── name = "abort"
///   └──────────────────────────── module = "env"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    /// The module namespace (e.g., `"env"`, `"wasi_snapshot_preview1"`).
    pub module_name: String,
    /// The name within the module (e.g., `"memory"`, `"fd_write"`).
    pub name: String,
    /// The kind (function, table, memory, or global).
    pub kind: ExternalKind,
    /// Type-specific information about what is being imported.
    pub type_info: ImportTypeInfo,
}

// ──────────────────────────────────────────────────────────────────────────────
// Export
// ──────────────────────────────────────────────────────────────────────────────

/// A single export declaration from the export section.
///
/// Exports make module-internal entities visible to the host. For example,
/// a compiled C program's `main` function and its heap memory would both
/// be exported.
///
/// ```text
/// Binary encoding:
///   <name: length-prefixed UTF-8>
///   <kind: ExternalKind byte>
///   <index: LEB128>  ;; index into the relevant index space
///
/// Example: export function 3 as "main"
///   04 "main"  00  03
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    /// The name visible to the host (e.g., `"main"`, `"memory"`).
    pub name: String,
    /// The kind of entity being exported.
    pub kind: ExternalKind,
    /// Index into the appropriate index space (function, table, memory, or global).
    pub index: u32,
}

// ──────────────────────────────────────────────────────────────────────────────
// Global
// ──────────────────────────────────────────────────────────────────────────────

/// A module-defined global variable with its initialization expression.
///
/// Global variables are initialized by running a *constant expression*
/// (a short sequence of instructions that must produce a compile-time constant).
/// The `init_expr` field stores the raw bytes of that expression, ending with
/// the `end` opcode (0x0B).
///
/// ```text
/// Example: `(global i32 (i32.const 42))`
///   type:     GlobalType { value_type: I32, mutable: false }
///   init_expr: [0x41, 0x2A, 0x0B]
///               │     │     └── end opcode
///               │     └──────── LEB128(42)
///               └────────────── i32.const opcode
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    /// The type of this global.
    pub global_type: GlobalType,
    /// The raw bytes of the constant initializer expression (includes trailing `end` 0x0B).
    pub init_expr: Vec<u8>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Element
// ──────────────────────────────────────────────────────────────────────────────

/// An element segment — initializes a range of table entries with function indices.
///
/// Element segments are the mechanism for populating function tables. At
/// module instantiation time, the runtime copies `function_indices` into
/// the table specified by `table_index`, starting at the position computed
/// by `offset_expr`.
///
/// ```text
/// Conceptually:
///   table[offset_expr()] = [func_0, func_1, func_2, ...]
///
/// Use case: C/C++ function pointer tables, vtables, dynamic dispatch.
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Index of the table to initialize (always 0 in WASM 1.0).
    pub table_index: u32,
    /// Constant expression that computes the starting offset in the table.
    pub offset_expr: Vec<u8>,
    /// The function indices to write into the table.
    pub function_indices: Vec<u32>,
}

// ──────────────────────────────────────────────────────────────────────────────
// DataSegment
// ──────────────────────────────────────────────────────────────────────────────

/// A data segment — initializes a region of linear memory with static bytes.
///
/// Data segments are how compiled programs load their static data (string
/// literals, lookup tables, initialized global variables) into WASM memory
/// at instantiation time.
///
/// ```text
/// Conceptually:
///   memory[offset_expr()] = data[0..data.len()]
///
/// Example: store the string "hello" at byte 1024
///   memory_index: 0
///   offset_expr:  [0x41, 0x80 0x08, 0x0B]  ;; i32.const 1024; end
///   data:         [0x68, 0x65, 0x6C, 0x6C, 0x6F]  ;; "hello"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DataSegment {
    /// Index of the memory to write into (always 0 in WASM 1.0).
    pub memory_index: u32,
    /// Constant expression that computes the byte offset into memory.
    pub offset_expr: Vec<u8>,
    /// The raw bytes to copy into memory.
    pub data: Vec<u8>,
}

// ──────────────────────────────────────────────────────────────────────────────
// FunctionBody
// ──────────────────────────────────────────────────────────────────────────────

/// The body of a locally-defined function: its locals and its bytecode.
///
/// In the WASM binary, locals are declared compactly (e.g., "3 locals of type
/// i32, 2 locals of type f64"). This struct stores them fully expanded —
/// one `ValueType` per local slot — for convenient access.
///
/// ```text
/// Binary structure (code section entry):
///   <body_size: LEB128>
///   <num_local_decls: LEB128>
///   (<count: LEB128>  <type: byte>)*  ;; run-length encoded locals
///   <instructions...>
///   0x0B                              ;; end opcode
///
/// Example: function with 2 i32 locals and code [i32.const 1, end]
///   locals: [I32, I32]
///   code:   [0x41, 0x01, 0x0B]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionBody {
    /// Local variable types (expanded from run-length encoding).
    /// Parameters are NOT included here — they are in the `FuncType`.
    pub locals: Vec<ValueType>,
    /// Raw instruction bytes for the function body (including the trailing `end` 0x0B).
    pub code: Vec<u8>,
}

// ──────────────────────────────────────────────────────────────────────────────
// CustomSection
// ──────────────────────────────────────────────────────────────────────────────

/// A custom section — arbitrary named data that tools can embed in a WASM file.
///
/// Custom sections (section ID 0) are ignored by the WASM runtime but carry
/// valuable metadata for tooling:
///
/// - `"name"` section — maps function indices to human-readable names (for debuggers)
/// - `"sourceMappingURL"` — points to a source map file
/// - DWARF debug info sections (used by `wasm-pack`, Rust's WASM target, etc.)
///
/// ```text
/// Binary encoding:
///   0x00                           ;; section ID = custom
///   <section_size: LEB128>
///   <name: length-prefixed UTF-8>  ;; name of this custom section
///   <data: bytes>                  ;; arbitrary payload
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CustomSection {
    /// The name of this custom section (e.g., `"name"`, `"sourceMappingURL"`).
    pub name: String,
    /// The raw payload bytes of this custom section.
    pub data: Vec<u8>,
}

// ──────────────────────────────────────────────────────────────────────────────
// WasmModule
// ──────────────────────────────────────────────────────────────────────────────

/// A WebAssembly module, extended to support WasmGC struct types.
///
/// This struct holds all data from all sections of a `.wasm` file after
/// parsing (or after lowering from IIR).  It is the intermediate
/// representation between the raw binary and any higher-level analysis
/// (validation, interpretation, compilation, GC type emission).
///
/// ## Relationship between fields
///
/// ```text
/// types[i]      ←── functions[j] (type index)
///               ←── imports with ImportTypeInfo::Function(i)
///               ←── BlockType::TypeIndex(i)
///
/// functions[j]  ←── code[j - num_imported_funcs]  (function body)
///
/// struct_types[k] is encoded at type-section index  types.len() + k
///
/// tables[0]     ←── elements[e].table_index
///
/// memories[0]   ←── data[d].memory_index
/// ```
///
/// ## WasmGC type section layout
///
/// The WasmGC proposal extends the type section to carry both function types
/// and GC types.  In the binary, each entry is tagged:
///
/// - Function type: starts with `0x60` (the WASM 1.0 function-type prefix).
/// - Struct type (open sub-type): starts with `0x50 0x00 0x5F` (sub-type
///   marker, zero supertypes, struct marker).
///
/// We keep `types: Vec<FuncType>` for the function types and add a new
/// `struct_types: Vec<StructType>` for the GC types.  The encoder emits
/// function types first (indices 0..N-1), then struct types (indices N..).
///
/// The `Default` impl produces an empty module (no sections), which is the
/// natural starting point for an incremental builder or parser.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WasmModule {
    /// Type section (§1): all function signatures, deduplicated.
    pub types: Vec<FuncType>,

    /// WasmGC struct type definitions (also in the type section, after `types`).
    ///
    /// Struct type `k` is at type-section index `types.len() + k`.
    /// When this vec is empty, the type section contains only function types
    /// and the encoding is identical to WASM 1.0.
    pub struct_types: Vec<StructType>,

    /// Import section (§2): things the module needs from the host.
    pub imports: Vec<Import>,
    /// Function section (§3): type indices for locally-defined functions.
    /// `functions[i]` is an index into `types`.
    pub functions: Vec<u32>,
    /// Table section (§4): function-reference tables.
    pub tables: Vec<TableType>,
    /// Memory section (§5): linear memory declarations.
    pub memories: Vec<MemoryType>,
    /// Global section (§6): module-defined global variables.
    pub globals: Vec<Global>,
    /// Export section (§7): names the module exposes to the host.
    pub exports: Vec<Export>,
    /// Start section (§8): optional index of a function to call on instantiation.
    pub start: Option<u32>,
    /// Element section (§9): table initialization data.
    pub elements: Vec<Element>,
    /// Code section (§10): function bodies (parallel array with `functions`).
    pub code: Vec<FunctionBody>,
    /// Data section (§11): memory initialization data.
    pub data: Vec<DataSegment>,
    /// Custom sections (§0): tool metadata (debug info, names, etc.).
    pub customs: Vec<CustomSection>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: ValueType byte encoding matches WASM spec ─────────────────────
    //
    // WasmGC dropped #[repr(u8)] because StructRef needs a 2-byte encoding.
    // Use .encode() and .byte_tag() instead of `as u8`.

    #[test]
    fn value_type_byte_values() {
        // Single-byte numeric types.
        assert_eq!(ValueType::I32.encode(), vec![0x7F], "i32 tag");
        assert_eq!(ValueType::I64.encode(), vec![0x7E], "i64 tag");
        assert_eq!(ValueType::F32.encode(), vec![0x7D], "f32 tag");
        assert_eq!(ValueType::F64.encode(), vec![0x7C], "f64 tag");
        // WasmGC reference types.
        assert_eq!(ValueType::Anyref.encode(), vec![0x6E], "anyref tag");
        assert_eq!(ValueType::I31ref.encode(), vec![0x6C], "i31ref tag");
        // StructRef(0) → [0x63, 0x00]
        assert_eq!(ValueType::StructRef(0).encode(), vec![0x63, 0x00], "struct ref tag");
        // StructRef(1) → [0x63, 0x01]
        assert_eq!(ValueType::StructRef(1).encode(), vec![0x63, 0x01]);
    }

    #[test]
    fn value_type_byte_tag_singles() {
        assert_eq!(ValueType::I32.byte_tag(), Some(0x7F));
        assert_eq!(ValueType::I64.byte_tag(), Some(0x7E));
        assert_eq!(ValueType::F32.byte_tag(), Some(0x7D));
        assert_eq!(ValueType::F64.byte_tag(), Some(0x7C));
        assert_eq!(ValueType::Anyref.byte_tag(), Some(0x6E));
        assert_eq!(ValueType::I31ref.byte_tag(), Some(0x6C));
        // StructRef has no single-byte tag.
        assert_eq!(ValueType::StructRef(0).byte_tag(), None);
    }

    // ── Test 2: ExternalKind byte values ─────────────────────────────────────

    #[test]
    fn external_kind_byte_values() {
        assert_eq!(ExternalKind::Function as u8, 0x00);
        assert_eq!(ExternalKind::Table as u8, 0x01);
        assert_eq!(ExternalKind::Memory as u8, 0x02);
        assert_eq!(ExternalKind::Global as u8, 0x03);
    }

    // ── Test 3: BLOCK_TYPE_EMPTY constant ────────────────────────────────────

    #[test]
    fn block_type_empty_constant() {
        assert_eq!(BLOCK_TYPE_EMPTY, 0x40);
    }

    // ── Test 4: FuncType construction and equality ────────────────────────────

    #[test]
    fn func_type_construction_and_equality() {
        let a = FuncType {
            params: vec![ValueType::I32, ValueType::I64],
            results: vec![ValueType::F32],
        };
        let b = FuncType {
            params: vec![ValueType::I32, ValueType::I64],
            results: vec![ValueType::F32],
        };
        assert_eq!(a, b);
    }

    // ── Test 5: FuncType with empty params and results ────────────────────────

    #[test]
    fn func_type_empty() {
        let ft = FuncType {
            params: vec![],
            results: vec![],
        };
        assert!(ft.params.is_empty());
        assert!(ft.results.is_empty());
    }

    // ── Test 6: FuncType with multiple params and results ─────────────────────

    #[test]
    fn func_type_multiple_params_and_results() {
        let ft = FuncType {
            params: vec![ValueType::I32, ValueType::I32, ValueType::F64],
            results: vec![ValueType::I64, ValueType::F32],
        };
        assert_eq!(ft.params.len(), 3);
        assert_eq!(ft.results.len(), 2);
        assert_eq!(ft.params[2], ValueType::F64);
        assert_eq!(ft.results[0], ValueType::I64);
    }

    // ── Test 7: Limits with only min ──────────────────────────────────────────

    #[test]
    fn limits_min_only() {
        let lim = Limits { min: 1, max: None };
        assert_eq!(lim.min, 1);
        assert_eq!(lim.max, None);
    }

    // ── Test 8: Limits with min and max ──────────────────────────────────────

    #[test]
    fn limits_min_and_max() {
        let lim = Limits { min: 1, max: Some(4) };
        assert_eq!(lim.min, 1);
        assert_eq!(lim.max, Some(4));
    }

    // ── Test 9: MemoryType construction ───────────────────────────────────────

    #[test]
    fn memory_type_construction() {
        let mt = MemoryType {
            limits: Limits { min: 2, max: Some(8) },
        };
        assert_eq!(mt.limits.min, 2);
        assert_eq!(mt.limits.max, Some(8));
    }

    // ── Test 10: TableType default element type is FUNCREF ────────────────────

    #[test]
    fn table_type_default_element_type() {
        let tt = TableType {
            element_type: FUNCREF,
            limits: Limits { min: 0, max: None },
        };
        assert_eq!(tt.element_type, 0x70);
        assert_eq!(tt.element_type, FUNCREF);
    }

    // ── Test 11: GlobalType mutable and immutable ─────────────────────────────

    #[test]
    fn global_type_mutability() {
        let mutable_g = GlobalType { value_type: ValueType::I32, mutable: true };
        let const_g = GlobalType { value_type: ValueType::F64, mutable: false };
        assert!(mutable_g.mutable);
        assert!(!const_g.mutable);
        assert_eq!(mutable_g.value_type, ValueType::I32);
        assert_eq!(const_g.value_type, ValueType::F64);
        // Verify encoding still works after WasmGC refactor.
        assert_eq!(mutable_g.value_type.encode(), vec![0x7F]);
        assert_eq!(const_g.value_type.encode(), vec![0x7C]);
    }

    // ── Test 12: Import for each ExternalKind ────────────────────────────────

    #[test]
    fn import_function() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "abort".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(0),
        };
        assert_eq!(imp.kind, ExternalKind::Function);
        assert_eq!(imp.type_info, ImportTypeInfo::Function(0));
    }

    #[test]
    fn import_table() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "table".to_string(),
            kind: ExternalKind::Table,
            type_info: ImportTypeInfo::Table(TableType {
                element_type: FUNCREF,
                limits: Limits { min: 0, max: None },
            }),
        };
        assert_eq!(imp.kind, ExternalKind::Table);
    }

    #[test]
    fn import_memory() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "memory".to_string(),
            kind: ExternalKind::Memory,
            type_info: ImportTypeInfo::Memory(MemoryType {
                limits: Limits { min: 1, max: Some(2) },
            }),
        };
        assert_eq!(imp.kind, ExternalKind::Memory);
    }

    #[test]
    fn import_global() {
        let imp = Import {
            module_name: "env".to_string(),
            name: "stack_ptr".to_string(),
            kind: ExternalKind::Global,
            type_info: ImportTypeInfo::Global(GlobalType {
                value_type: ValueType::I32,
                mutable: true,
            }),
        };
        assert_eq!(imp.kind, ExternalKind::Global);
    }

    // ── Test 13: Export construction ──────────────────────────────────────────

    #[test]
    fn export_construction() {
        let exp = Export {
            name: "main".to_string(),
            kind: ExternalKind::Function,
            index: 3,
        };
        assert_eq!(exp.name, "main");
        assert_eq!(exp.kind, ExternalKind::Function);
        assert_eq!(exp.index, 3);
    }

    // ── Test 14: Global with init_expr ────────────────────────────────────────

    #[test]
    fn global_with_init_expr() {
        // i32.const 42 ; end  →  [0x41, 0x2A, 0x0B]
        let g = Global {
            global_type: GlobalType { value_type: ValueType::I32, mutable: false },
            init_expr: vec![0x41, 0x2A, 0x0B],
        };
        assert_eq!(g.init_expr, vec![0x41, 0x2A, 0x0B]);
        assert_eq!(g.global_type.value_type, ValueType::I32);
    }

    // ── Test 15: Element with function_indices ────────────────────────────────

    #[test]
    fn element_with_function_indices() {
        let elem = Element {
            table_index: 0,
            offset_expr: vec![0x41, 0x00, 0x0B], // i32.const 0; end
            function_indices: vec![1, 3, 5, 7],
        };
        assert_eq!(elem.table_index, 0);
        assert_eq!(elem.function_indices, vec![1, 3, 5, 7]);
        assert_eq!(elem.function_indices.len(), 4);
    }

    // ── Test 16: DataSegment ──────────────────────────────────────────────────

    #[test]
    fn data_segment() {
        let seg = DataSegment {
            memory_index: 0,
            offset_expr: vec![0x41, 0x80, 0x08, 0x0B], // i32.const 1024; end
            data: b"hello".to_vec(),
        };
        assert_eq!(seg.memory_index, 0);
        assert_eq!(seg.data, b"hello");
    }

    // ── Test 17: FunctionBody ─────────────────────────────────────────────────

    #[test]
    fn function_body() {
        let body = FunctionBody {
            locals: vec![ValueType::I32, ValueType::I32],
            code: vec![0x41, 0x01, 0x0B], // i32.const 1; end
        };
        assert_eq!(body.locals.len(), 2);
        assert_eq!(body.locals[0], ValueType::I32);
        assert_eq!(body.code, vec![0x41, 0x01, 0x0B]);
    }

    // ── Test 18: CustomSection ────────────────────────────────────────────────

    #[test]
    fn custom_section() {
        let sec = CustomSection {
            name: "name".to_string(),
            data: vec![0x01, 0x02, 0x03],
        };
        assert_eq!(sec.name, "name");
        assert_eq!(sec.data.len(), 3);
    }

    // ── Test 19: WasmModule has all required fields ───────────────────────────

    #[test]
    fn wasm_module_has_all_fields() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::I32] }],
            struct_types: vec![],
            imports: vec![],
            functions: vec![0],
            tables: vec![],
            memories: vec![MemoryType { limits: Limits { min: 1, max: None } }],
            globals: vec![],
            exports: vec![Export { name: "main".to_string(), kind: ExternalKind::Function, index: 0 }],
            start: Some(0),
            elements: vec![],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            data: vec![],
            customs: vec![],
        };
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.struct_types.len(), 0);
        assert_eq!(m.functions, vec![0]);
        assert_eq!(m.start, Some(0));
        assert_eq!(m.exports[0].name, "main");
    }

    // ── Test 20: WasmModule default is all-empty ──────────────────────────────

    #[test]
    fn wasm_module_default_is_empty() {
        let m = WasmModule::default();
        assert!(m.types.is_empty());
        assert!(m.struct_types.is_empty());
        assert!(m.imports.is_empty());
        assert!(m.functions.is_empty());
        assert!(m.tables.is_empty());
        assert!(m.memories.is_empty());
        assert!(m.globals.is_empty());
        assert!(m.exports.is_empty());
        assert_eq!(m.start, None);
        assert!(m.elements.is_empty());
        assert!(m.code.is_empty());
        assert!(m.data.is_empty());
        assert!(m.customs.is_empty());
    }

    // ── WasmGC type tests ──────────────────────────────────────────────────────

    // Test 21: FieldType construction
    #[test]
    fn field_type_construction() {
        let f = FieldType { val_type: ValueType::Anyref, mutable: true };
        assert_eq!(f.val_type, ValueType::Anyref);
        assert!(f.mutable);

        let g = FieldType { val_type: ValueType::I32, mutable: false };
        assert_eq!(g.val_type, ValueType::I32);
        assert!(!g.mutable);
    }

    // Test 22: StructType for LispyPair has two anyref fields
    #[test]
    fn struct_type_lispy_pair() {
        let lispy_pair = StructType {
            fields: vec![
                FieldType { val_type: ValueType::Anyref, mutable: true }, // $head
                FieldType { val_type: ValueType::Anyref, mutable: true }, // $tail
            ],
        };
        assert_eq!(lispy_pair.fields.len(), 2);
        assert_eq!(lispy_pair.fields[0].val_type, ValueType::Anyref);
        assert!(lispy_pair.fields[0].mutable);
        assert_eq!(lispy_pair.fields[1].val_type, ValueType::Anyref);
        assert!(lispy_pair.fields[1].mutable);
    }

    // Test 23: WasmModule with struct_types carries the GC definition
    #[test]
    fn wasm_module_with_struct_types() {
        let m = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            struct_types: vec![StructType {
                fields: vec![
                    FieldType { val_type: ValueType::Anyref, mutable: true },
                    FieldType { val_type: ValueType::Anyref, mutable: true },
                ],
            }],
            ..Default::default()
        };
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.struct_types.len(), 1);
        // The struct type index in the type section is types.len() + 0 = 1.
        assert_eq!(m.types.len() + 0, 1);
    }

    // Test 24: StructRef(idx) encodes correctly
    #[test]
    fn struct_ref_encoding() {
        // StructRef(0) → [0x63, 0x00]
        assert_eq!(ValueType::StructRef(0).encode(), vec![0x63, 0x00]);
        // StructRef(5) → [0x63, 0x05]
        assert_eq!(ValueType::StructRef(5).encode(), vec![0x63, 0x05]);
        // Large index: 128 needs 2 LEB128 bytes.
        let enc = ValueType::StructRef(128).encode();
        assert_eq!(enc[0], 0x63);
        assert!(enc.len() >= 3); // 0x63 + 2 LEB128 bytes for 128
    }

    // Test 25: Anyref and I31ref encode as single bytes
    #[test]
    fn anyref_i31ref_single_byte() {
        assert_eq!(ValueType::Anyref.encode(), vec![0x6E]);
        assert_eq!(ValueType::I31ref.encode(), vec![0x6C]);
    }

    // ── Additional: BlockType variants ───────────────────────────────────────

    #[test]
    fn block_type_variants() {
        assert_eq!(BlockType::Empty, BlockType::Empty);
        assert_eq!(BlockType::Value(ValueType::I32), BlockType::Value(ValueType::I32));
        assert_ne!(BlockType::Value(ValueType::I32), BlockType::Value(ValueType::I64));
        assert_eq!(BlockType::TypeIndex(5), BlockType::TypeIndex(5));
    }

    // ── Additional: ValueType Clone semantics ────────────────────────────────
    //
    // ValueType lost Copy when we added StructRef(u32) — that variant holds
    // a u32 but the enum is no longer #[repr(u8)].  WasmGC encoding instead
    // uses the ValueType::encode() method.  Use Clone where a copy is needed.

    #[test]
    fn value_type_is_clone() {
        let a = ValueType::I32;
        let b = a.clone();
        assert_eq!(a, b);
        let c = ValueType::StructRef(42);
        let d = c.clone();
        assert_eq!(c, d);
    }

    // ── Additional: FUNCREF constant ─────────────────────────────────────────

    #[test]
    fn funcref_constant() {
        assert_eq!(FUNCREF, 0x70);
    }
}
