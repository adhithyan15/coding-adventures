//! Encode `wasm-types::WasmModule` into raw WebAssembly bytes.
//!
//! This encoder supports both **WASM 1.0** (pure numeric types) and the
//! **WasmGC** extension (struct types, anyref, i31ref, StructRef).
//!
//! ## WasmGC type section
//!
//! When [`WasmModule::struct_types`] is non-empty, the type section is
//! extended.  Function types are emitted first (tagged `0x60`), then struct
//! types.  Each struct type is wrapped in a *sub-type descriptor*:
//!
//! ```text
//! 0x50        ;; sub-type marker (open recursion group entry)
//! 0x00        ;; no supertypes (0-length vector)
//! 0x5F        ;; struct type marker
//! <n: u32 LEB128>   ;; number of fields
//! for each field:
//!   <val_type bytes>       ;; ValueType::encode()
//!   <mutability: 0x00|0x01>
//! ```
//!
//! This format follows the WasmGC binary encoding spec
//! (https://github.com/WebAssembly/gc/blob/main/proposals/gc/MVP.md#types).
//!
//! ## GC instruction helpers
//!
//! [`GcInstruction`] and [`encode_gc_instruction`] provide typed helpers for
//! emitting WasmGC opcodes into a function body's code buffer.  All GC
//! instructions use the `0xFB` prefix byte.

use std::fmt;

use wasm_leb128::encode_unsigned;
use wasm_types::{
    CustomSection, DataSegment, Element, Export, ExternalKind, FieldType, FuncType, FunctionBody,
    Global, GlobalType, Import, ImportTypeInfo, Limits, MemoryType, StructType, TableType,
    ValueType, WasmModule,
};

pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];
pub const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmEncodeError {
    pub message: String,
}

impl WasmEncodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WasmEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WasmEncodeError {}

pub fn encode_module(module: &WasmModule) -> Result<Vec<u8>, WasmEncodeError> {
    let mut sections = Vec::new();

    for custom in &module.customs {
        sections.extend(encode_section(0, encode_custom(custom)));
    }
    // ── Type section: function types + WasmGC struct types ──────────────────
    //
    // The type section carries both function types and GC struct types.
    // Function types come first (tagged 0x60), then struct types (each
    // wrapped in a sub-type descriptor starting with 0x50).
    //
    // We only emit the section if at least one type is present.
    let has_types = !module.types.is_empty() || !module.struct_types.is_empty();
    if has_types {
        let total_count = module.types.len() + module.struct_types.len();
        let mut type_section = encode_u32(total_count as u32);
        for ft in &module.types {
            type_section.extend(encode_func_type(ft));
        }
        for st in &module.struct_types {
            type_section.extend(encode_struct_type(st));
        }
        sections.extend(encode_section(1, type_section));
    }
    if !module.imports.is_empty() {
        sections.extend(encode_section(2, encode_imports(&module.imports)?));
    }
    if !module.functions.is_empty() {
        sections.extend(encode_section(
            3,
            encode_vector(&module.functions, |index| encode_u32(*index)),
        ));
    }
    if !module.tables.is_empty() {
        sections.extend(encode_section(
            4,
            encode_vector(&module.tables, encode_table_type),
        ));
    }
    if !module.memories.is_empty() {
        sections.extend(encode_section(
            5,
            encode_vector(&module.memories, encode_memory_type),
        ));
    }
    if !module.globals.is_empty() {
        sections.extend(encode_section(
            6,
            encode_vector(&module.globals, encode_global),
        ));
    }
    if !module.exports.is_empty() {
        sections.extend(encode_section(
            7,
            encode_vector(&module.exports, encode_export),
        ));
    }
    if let Some(start) = module.start {
        sections.extend(encode_section(8, encode_u32(start)));
    }
    if !module.elements.is_empty() {
        sections.extend(encode_section(
            9,
            encode_vector(&module.elements, encode_element),
        ));
    }
    if !module.code.is_empty() {
        sections.extend(encode_section(10, encode_function_bodies(&module.code)));
    }
    if !module.data.is_empty() {
        sections.extend(encode_section(
            11,
            encode_vector(&module.data, encode_data_segment),
        ));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);
    bytes.extend(sections);
    Ok(bytes)
}

fn encode_section(section_id: u8, payload: Vec<u8>) -> Vec<u8> {
    let mut bytes = vec![section_id];
    bytes.extend(encode_u32(payload.len() as u32));
    bytes.extend(payload);
    bytes
}

fn encode_u32(value: u32) -> Vec<u8> {
    encode_unsigned(value as u64)
}

fn encode_name(text: &str) -> Vec<u8> {
    let mut bytes = encode_u32(text.len() as u32);
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn encode_vector<T>(values: &[T], mut encode: impl FnMut(&T) -> Vec<u8>) -> Vec<u8> {
    let mut bytes = encode_u32(values.len() as u32);
    for value in values {
        bytes.extend(encode(value));
    }
    bytes
}

/// Encode a vector of `ValueType` values with a LEB128 count prefix.
///
/// In WASM 1.0, every value type was a single byte; the entire params/results
/// vector was just `[count, byte, byte, ...]`.  With WasmGC, reference types
/// like `StructRef(n)` take two or more bytes, so we flatten by extending with
/// each type's `encode()` result rather than casting to `u8`.
///
/// ```text
/// [I32, I32]  →  [0x02, 0x7F, 0x7F]
/// [Anyref]    →  [0x01, 0x6E]
/// [StructRef(1)] → [0x01, 0x63, 0x01]
/// ```
fn encode_value_types(types: &[ValueType]) -> Vec<u8> {
    let mut bytes = encode_u32(types.len() as u32);
    for vt in types {
        bytes.extend(vt.encode());
    }
    bytes
}

fn encode_func_type(func_type: &FuncType) -> Vec<u8> {
    let mut bytes = vec![0x60];
    bytes.extend(encode_value_types(&func_type.params));
    bytes.extend(encode_value_types(&func_type.results));
    bytes
}

fn encode_limits(limits: &Limits) -> Vec<u8> {
    let mut bytes = Vec::new();
    match limits.max {
        Some(max) => {
            bytes.push(0x01);
            bytes.extend(encode_u32(limits.min));
            bytes.extend(encode_u32(max));
        }
        None => {
            bytes.push(0x00);
            bytes.extend(encode_u32(limits.min));
        }
    }
    bytes
}

fn encode_memory_type(memory_type: &MemoryType) -> Vec<u8> {
    encode_limits(&memory_type.limits)
}

fn encode_table_type(table_type: &TableType) -> Vec<u8> {
    let mut bytes = vec![table_type.element_type];
    bytes.extend(encode_limits(&table_type.limits));
    bytes
}

fn encode_global_type(global_type: &GlobalType) -> Vec<u8> {
    // GlobalType holds a ValueType which may now be multi-byte (WasmGC).
    let mut bytes = global_type.value_type.encode();
    bytes.push(if global_type.mutable { 0x01 } else { 0x00 });
    bytes
}

fn encode_imports(imports: &[Import]) -> Result<Vec<u8>, WasmEncodeError> {
    let mut bytes = encode_u32(imports.len() as u32);
    for import in imports {
        bytes.extend(encode_import(import)?);
    }
    Ok(bytes)
}

fn encode_import(import: &Import) -> Result<Vec<u8>, WasmEncodeError> {
    let mut bytes = Vec::new();
    bytes.extend(encode_name(&import.module_name));
    bytes.extend(encode_name(&import.name));
    bytes.push(import.kind as u8);

    match (&import.kind, &import.type_info) {
        (ExternalKind::Function, ImportTypeInfo::Function(type_index)) => {
            bytes.extend(encode_u32(*type_index));
        }
        (ExternalKind::Table, ImportTypeInfo::Table(table_type)) => {
            bytes.extend(encode_table_type(table_type));
        }
        (ExternalKind::Memory, ImportTypeInfo::Memory(memory_type)) => {
            bytes.extend(encode_memory_type(memory_type));
        }
        (ExternalKind::Global, ImportTypeInfo::Global(global_type)) => {
            bytes.extend(encode_global_type(global_type));
        }
        (ExternalKind::Function, _) => {
            return Err(WasmEncodeError::new(
                "function imports require a function type index",
            ));
        }
        (ExternalKind::Table, _) => {
            return Err(WasmEncodeError::new(
                "table imports require TableType metadata",
            ));
        }
        (ExternalKind::Memory, _) => {
            return Err(WasmEncodeError::new(
                "memory imports require MemoryType metadata",
            ));
        }
        (ExternalKind::Global, _) => {
            return Err(WasmEncodeError::new(
                "global imports require GlobalType metadata",
            ));
        }
    }

    Ok(bytes)
}

fn encode_export(export: &Export) -> Vec<u8> {
    let mut bytes = encode_name(&export.name);
    bytes.push(export.kind as u8);
    bytes.extend(encode_u32(export.index));
    bytes
}

fn encode_global(global: &Global) -> Vec<u8> {
    let mut bytes = encode_global_type(&global.global_type);
    bytes.extend_from_slice(&global.init_expr);
    bytes
}

fn encode_element(element: &Element) -> Vec<u8> {
    let mut bytes = encode_u32(element.table_index);
    bytes.extend_from_slice(&element.offset_expr);
    bytes.extend(encode_u32(element.function_indices.len() as u32));
    for func_index in &element.function_indices {
        bytes.extend(encode_u32(*func_index));
    }
    bytes
}

fn encode_data_segment(segment: &DataSegment) -> Vec<u8> {
    let mut bytes = encode_u32(segment.memory_index);
    bytes.extend_from_slice(&segment.offset_expr);
    bytes.extend(encode_u32(segment.data.len() as u32));
    bytes.extend_from_slice(&segment.data);
    bytes
}

fn encode_function_bodies(bodies: &[FunctionBody]) -> Vec<u8> {
    let mut bytes = encode_u32(bodies.len() as u32);
    for body in bodies {
        bytes.extend(encode_function_body(body));
    }
    bytes
}

fn encode_function_body(body: &FunctionBody) -> Vec<u8> {
    let groups = group_locals(&body.locals);
    let mut payload = encode_u32(groups.len() as u32);
    for (count, value_type) in groups {
        payload.extend(encode_u32(count));
        // In WASM 1.0, every ValueType was a single byte and the group just
        // stored that byte.  With WasmGC, reference types may be multi-byte.
        // We still use run-length groups (same count+type encoding) — the
        // type is encoded via ValueType::encode() which may produce 1+ bytes.
        payload.extend(value_type.encode());
    }
    payload.extend_from_slice(&body.code);

    let mut bytes = encode_u32(payload.len() as u32);
    bytes.extend(payload);
    bytes
}

/// Run-length encode a list of local variable types.
///
/// WASM's code section declares locals in *groups*: `(count, type)` pairs.
/// Runs of the same type are merged into a single group, reducing code size.
///
/// With WasmGC, `ValueType` is no longer `Copy` (because `StructRef` holds a
/// `u32` payload), so we clone when recording the current group type.
///
/// ```text
/// [I32, I32, I32, F64, F64]  →  [(3, I32), (2, F64)]
/// [Anyref, Anyref]            →  [(2, Anyref)]
/// ```
fn group_locals(locals: &[ValueType]) -> Vec<(u32, ValueType)> {
    if locals.is_empty() {
        return Vec::new();
    }

    let mut groups = Vec::new();
    let mut current = locals[0].clone();
    let mut count = 1u32;
    for value_type in &locals[1..] {
        if *value_type == current {
            count += 1;
        } else {
            groups.push((count, current));
            current = value_type.clone();
            count = 1;
        }
    }
    groups.push((count, current));
    groups
}

fn encode_custom(custom: &CustomSection) -> Vec<u8> {
    let mut bytes = encode_name(&custom.name);
    bytes.extend_from_slice(&custom.data);
    bytes
}

// ──────────────────────────────────────────────────────────────────────────────
// WasmGC struct type encoding
// ──────────────────────────────────────────────────────────────────────────────

/// Encode a single WasmGC struct type for the type section.
///
/// In the WasmGC binary format, a struct type entry in the type section is
/// wrapped in a *sub-type descriptor*.  The layout is:
///
/// ```text
/// 0x50              ;; sub-type open entry (allows recursive types)
/// 0x00              ;; zero supertypes (no explicit supertype chain)
/// 0x5F              ;; struct marker
/// <n: u32 LEB>      ;; number of fields
/// for each field:
///   <val_type bytes>   ;; ValueType encoding (1 or more bytes)
///   0x00 or 0x01       ;; mutability flag (0 = immutable, 1 = mutable)
/// ```
///
/// For `$LispyPair` with two mutable `anyref` fields:
/// ```text
/// 0x50 0x00 0x5F 0x02       ;; sub-type, 0 supers, struct, 2 fields
/// 0x6E 0x01                 ;; field 0: anyref, mutable
/// 0x6E 0x01                 ;; field 1: anyref, mutable
/// ```
fn encode_struct_type(st: &StructType) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.push(0x50); // sub-type open marker
    bytes.push(0x00); // zero supertypes
    bytes.push(0x5F); // struct marker
    // Number of fields.
    bytes.extend(encode_u32(st.fields.len() as u32));
    for field in &st.fields {
        bytes.extend(encode_field_type(field));
    }
    bytes
}

/// Encode one field of a WasmGC struct type.
///
/// ```text
/// <val_type encoding>   ;; ValueType::encode()
/// <mutability: 0x00 = immutable, 0x01 = mutable>
/// ```
fn encode_field_type(ft: &FieldType) -> Vec<u8> {
    let mut bytes = ft.val_type.encode();
    bytes.push(if ft.mutable { 0x01 } else { 0x00 });
    bytes
}

// ──────────────────────────────────────────────────────────────────────────────
// WasmGC instruction encoding
// ──────────────────────────────────────────────────────────────────────────────

/// A WasmGC instruction that can be emitted into a function body.
///
/// All WasmGC instructions use the **`0xFB` prefix** followed by a secondary
/// opcode byte (and optional LEB128 immediates).  This enum provides a typed
/// view of the instructions our GC backend emits.
///
/// ## Reference
///
/// WasmGC binary encoding:
/// https://github.com/WebAssembly/gc/blob/main/proposals/gc/MVP.md#instructions
///
/// ## Usage
///
/// ```rust
/// use wasm_module_encoder::{GcInstruction, encode_gc_instruction};
///
/// let mut code: Vec<u8> = Vec::new();
/// encode_gc_instruction(&mut code, &GcInstruction::StructNew(0));
/// // code is now [0xFB, 0x00, 0x00]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcInstruction {
    /// `struct.new $type_idx` — allocate a new struct, filling fields from
    /// the stack in declaration order (field 0 is deepest, last field on top).
    ///
    /// Encoding: `0xFB 0x00 <type_idx: LEB128>`
    ///
    /// Stack effect: `[field_0_type, ..., field_n_type] → [(ref $T)]`
    ///
    /// For a two-field `$LispyPair`:
    /// ```text
    /// ;; stack before: [head: anyref, tail: anyref]  (head deepest)
    /// struct.new $LispyPair
    /// ;; stack after: [(ref $LispyPair)]
    /// ```
    StructNew(u32),

    /// `struct.get $type $field_idx` — load the value of field `field_idx`
    /// from a struct reference.
    ///
    /// Encoding: `0xFB 0x02 <type_idx: LEB128> <field_idx: LEB128>`
    ///
    /// Stack effect: `[(ref null $T)] → [field_type]`
    ///
    /// This is how we implement `car` (field 0) and `cdr` (field 1) on a pair.
    StructGet(u32, u32),

    /// `struct.set $type $field_idx` — store a value into field `field_idx`
    /// of a struct reference (field must be mutable).
    ///
    /// Encoding: `0xFB 0x04 <type_idx: LEB128> <field_idx: LEB128>`
    ///
    /// Stack effect: `[(ref null $T), field_type] → []`
    StructSet(u32, u32),

    /// `ref.null none` — push a typed null reference.
    ///
    /// Encoding: `0xD0 0x0F`
    ///
    /// The `none` heap type (`0x0F`) is the bottom type of the GC reference
    /// hierarchy — it is a subtype of all reference types, so `ref.null none`
    /// is assignment-compatible with any nullable reference type including
    /// `anyref`, `(ref null $LispyPair)`, etc.
    ///
    /// This is the canonical way to produce `nil` / `null` in a Lisp runtime
    /// when the target type is a GC reference.
    RefNull,

    /// `ref.is_null` — test whether a reference is null.
    ///
    /// Encoding: `0xD1`
    ///
    /// Stack effect: `[anyref] → [i32]`  (1 if null, 0 if not)
    ///
    /// This is used to implement `(null? x)` in Scheme or `NIL` detection
    /// in Common Lisp.
    RefIsNull,

    /// `i31.new` — box an `i32` value as an `i31ref` (GC integer reference).
    ///
    /// Encoding: `0xFB 0x1C`
    ///
    /// Stack effect: `[i32] → [i31ref]`
    ///
    /// The runtime stores the 31-bit integer directly in the reference word
    /// (no heap allocation).  The top bit of the i32 is dropped (values must
    /// fit in 31 signed bits).
    I31New,

    /// `i31.get_s` — unbox an `i31ref` to a sign-extended `i32`.
    ///
    /// Encoding: `0xFB 0x1D`
    ///
    /// Stack effect: `[i31ref] → [i32]`
    I31GetS,

    /// `any.convert_extern` — convert an `externref` to `anyref`.
    ///
    /// Encoding: `0xFB 0x1A`
    ///
    /// Used to bring JavaScript objects through the `externref`/`anyref`
    /// boundary when interoperating with the host.
    AnyConvertExtern,
}

/// Emit WasmGC instructions as byte sequences into a function body's code buf.
///
/// All GC instructions in the `0xFB` family are emitted here.  The two
/// exceptions — `ref.null` and `ref.is_null` — live in the base opcode
/// space (`0xD0` and `0xD1`) but are included for completeness.
///
/// # Example
///
/// ```rust
/// use wasm_module_encoder::{GcInstruction, encode_gc_instruction};
///
/// let mut code = Vec::new();
/// encode_gc_instruction(&mut code, &GcInstruction::StructNew(0));
/// assert_eq!(code, vec![0xFB, 0x00, 0x00]);
///
/// code.clear();
/// encode_gc_instruction(&mut code, &GcInstruction::RefNull);
/// assert_eq!(code, vec![0xD0, 0x0F]);
///
/// code.clear();
/// encode_gc_instruction(&mut code, &GcInstruction::RefIsNull);
/// assert_eq!(code, vec![0xD1]);
/// ```
pub fn encode_gc_instruction(code: &mut Vec<u8>, instr: &GcInstruction) {
    match instr {
        // ── struct.new $type_idx ─────────────────────────────────────────────
        //
        // 0xFB 0x00 <type_idx: LEB128>
        //
        // Pops all fields from the stack in declaration order (field 0 deepest,
        // last field on top) and constructs a new struct on the GC heap.
        GcInstruction::StructNew(type_idx) => {
            code.push(0xFB);
            code.push(0x00);
            code.extend(encode_unsigned(*type_idx as u64));
        }

        // ── struct.get $type $field ──────────────────────────────────────────
        //
        // 0xFB 0x02 <type_idx: LEB128> <field_idx: LEB128>
        //
        // Pops a struct reference, pushes the value of field `field_idx`.
        GcInstruction::StructGet(type_idx, field_idx) => {
            code.push(0xFB);
            code.push(0x02);
            code.extend(encode_unsigned(*type_idx as u64));
            code.extend(encode_unsigned(*field_idx as u64));
        }

        // ── struct.set $type $field ──────────────────────────────────────────
        //
        // 0xFB 0x04 <type_idx: LEB128> <field_idx: LEB128>
        //
        // Pops a value and a struct reference, stores value at field `field_idx`.
        GcInstruction::StructSet(type_idx, field_idx) => {
            code.push(0xFB);
            code.push(0x04);
            code.extend(encode_unsigned(*type_idx as u64));
            code.extend(encode_unsigned(*field_idx as u64));
        }

        // ── ref.null none ────────────────────────────────────────────────────
        //
        // 0xD0 0x0F
        //
        // Push a typed null.  Heap type `none` (0x0F) is a subtype of all
        // nullable reference types; assigning `ref.null none` to any
        // `(ref null X)` local is always valid.
        GcInstruction::RefNull => {
            code.push(0xD0);
            code.push(0x0F); // heap type: none
        }

        // ── ref.is_null ──────────────────────────────────────────────────────
        //
        // 0xD1
        //
        // Test nullability; pushes i32 (1 = null, 0 = non-null).
        GcInstruction::RefIsNull => {
            code.push(0xD1);
        }

        // ── i31.new ──────────────────────────────────────────────────────────
        //
        // 0xFB 0x1C
        //
        // Box an i32 as an i31ref (no allocation).
        GcInstruction::I31New => {
            code.push(0xFB);
            code.push(0x1C);
        }

        // ── i31.get_s ────────────────────────────────────────────────────────
        //
        // 0xFB 0x1D
        //
        // Unbox an i31ref to a sign-extended i32.
        GcInstruction::I31GetS => {
            code.push(0xFB);
            code.push(0x1D);
        }

        // ── any.convert_extern ───────────────────────────────────────────────
        //
        // 0xFB 0x1A
        //
        // Convert an externref (host object) to anyref (GC-managed).
        GcInstruction::AnyConvertExtern => {
            code.push(0xFB);
            code.push(0x1A);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_module_parser::WasmModuleParser;
    use wasm_types::WasmModule;

    fn minimal_module() -> WasmModule {
        WasmModule {
            types: vec![FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            exports: vec![Export {
                name: "identity".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x20, 0x00, 0x0B],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn encodes_minimal_module_round_trip() {
        let module = minimal_module();
        let encoded = encode_module(&module).unwrap();
        let parsed = WasmModuleParser::parse(&encoded).unwrap();

        assert!(encoded.starts_with(&[WASM_MAGIC, WASM_VERSION].concat()));
        assert_eq!(parsed.types, module.types);
        assert_eq!(parsed.functions, module.functions);
        assert_eq!(parsed.exports, module.exports);
        assert_eq!(parsed.code, module.code);
    }

    #[test]
    fn encodes_memory_data_global_and_start() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            memories: vec![MemoryType {
                limits: Limits {
                    min: 1,
                    max: Some(2),
                },
            }],
            globals: vec![Global {
                global_type: GlobalType {
                    value_type: ValueType::I32,
                    mutable: false,
                },
                init_expr: vec![0x41, 0x2A, 0x0B],
            }],
            exports: vec![
                Export {
                    name: "main".to_string(),
                    kind: ExternalKind::Function,
                    index: 0,
                },
                Export {
                    name: "memory".to_string(),
                    kind: ExternalKind::Memory,
                    index: 0,
                },
            ],
            start: Some(0),
            code: vec![FunctionBody {
                locals: vec![ValueType::I32],
                code: vec![0x41, 0x07, 0x0B],
            }],
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: b"Nib".to_vec(),
            }],
            ..Default::default()
        };

        let parsed = WasmModuleParser::parse(&encode_module(&module).unwrap()).unwrap();
        assert_eq!(parsed.memories, module.memories);
        assert_eq!(parsed.globals, module.globals);
        assert_eq!(parsed.start, module.start);
        assert_eq!(parsed.data, module.data);
    }

    #[test]
    fn encodes_imports_table_and_custom_section() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![],
            }],
            imports: vec![
                Import {
                    module_name: "env".to_string(),
                    name: "f".to_string(),
                    kind: ExternalKind::Function,
                    type_info: ImportTypeInfo::Function(0),
                },
                Import {
                    module_name: "env".to_string(),
                    name: "table".to_string(),
                    kind: ExternalKind::Table,
                    type_info: ImportTypeInfo::Table(TableType {
                        element_type: 0x70,
                        limits: Limits {
                            min: 1,
                            max: Some(4),
                        },
                    }),
                },
                Import {
                    module_name: "env".to_string(),
                    name: "memory".to_string(),
                    kind: ExternalKind::Memory,
                    type_info: ImportTypeInfo::Memory(MemoryType {
                        limits: Limits { min: 1, max: None },
                    }),
                },
                Import {
                    module_name: "env".to_string(),
                    name: "glob".to_string(),
                    kind: ExternalKind::Global,
                    type_info: ImportTypeInfo::Global(GlobalType {
                        value_type: ValueType::I32,
                        mutable: true,
                    }),
                },
            ],
            customs: vec![CustomSection {
                name: "name".to_string(),
                data: vec![0x01, 0x02],
            }],
            ..Default::default()
        };

        let parsed = WasmModuleParser::parse(&encode_module(&module).unwrap()).unwrap();
        assert_eq!(parsed.imports, module.imports);
        assert_eq!(parsed.customs, module.customs);
    }

    #[test]
    fn rejects_invalid_function_import_type() {
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "f".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Memory(MemoryType {
                    limits: Limits { min: 1, max: None },
                }),
            }],
            ..Default::default()
        };

        let err = encode_module(&module).unwrap_err();
        assert!(err.message.contains("function imports require"));
    }

    // ── WasmGC instruction encoding tests ─────────────────────────────────────

    // GC Test 1: struct.new type_idx=0
    #[test]
    fn encode_gc_struct_new_type_0() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::StructNew(0));
        // 0xFB 0x00 + LEB128(0) = [0xFB, 0x00, 0x00]
        assert_eq!(code, vec![0xFB, 0x00, 0x00]);
    }

    // GC Test 2: struct.get type=0, field=0
    #[test]
    fn encode_gc_struct_get_0_0() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::StructGet(0, 0));
        // 0xFB 0x02 + LEB128(0) + LEB128(0) = [0xFB, 0x02, 0x00, 0x00]
        assert_eq!(code, vec![0xFB, 0x02, 0x00, 0x00]);
    }

    // GC Test 3: struct.get type=0, field=1 (cdr / tail)
    #[test]
    fn encode_gc_struct_get_0_1() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::StructGet(0, 1));
        // [0xFB, 0x02, 0x00, 0x01]
        assert_eq!(code, vec![0xFB, 0x02, 0x00, 0x01]);
    }

    // GC Test 4: struct.set type=0, field=0
    #[test]
    fn encode_gc_struct_set_0_0() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::StructSet(0, 0));
        // 0xFB 0x04 + LEB128(0) + LEB128(0) = [0xFB, 0x04, 0x00, 0x00]
        assert_eq!(code, vec![0xFB, 0x04, 0x00, 0x00]);
    }

    // GC Test 5: ref.null none
    #[test]
    fn encode_gc_ref_null() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::RefNull);
        // 0xD0 0x0F
        assert_eq!(code, vec![0xD0, 0x0F]);
    }

    // GC Test 6: ref.is_null
    #[test]
    fn encode_gc_ref_is_null() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::RefIsNull);
        // 0xD1
        assert_eq!(code, vec![0xD1]);
    }

    // GC Test 7: i31.new
    #[test]
    fn encode_gc_i31_new() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::I31New);
        // 0xFB 0x1C
        assert_eq!(code, vec![0xFB, 0x1C]);
    }

    // GC Test 8: i31.get_s
    #[test]
    fn encode_gc_i31_get_s() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::I31GetS);
        // 0xFB 0x1D
        assert_eq!(code, vec![0xFB, 0x1D]);
    }

    // GC Test 9: any.convert_extern
    #[test]
    fn encode_gc_any_convert_extern() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::AnyConvertExtern);
        // 0xFB 0x1A
        assert_eq!(code, vec![0xFB, 0x1A]);
    }

    // GC Test 10: struct.new with larger type index
    #[test]
    fn encode_gc_struct_new_type_5() {
        let mut code = Vec::new();
        encode_gc_instruction(&mut code, &GcInstruction::StructNew(5));
        // 0xFB 0x00 + LEB128(5)
        assert_eq!(code, vec![0xFB, 0x00, 0x05]);
    }

    // GC Test 11: WasmModule with struct_types encodes a type section
    // that contains both func types and struct types.
    #[test]
    fn module_with_struct_types_has_extended_type_section() {
        use wasm_types::{FieldType, StructType};
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![],
            }],
            struct_types: vec![StructType {
                fields: vec![
                    FieldType { val_type: ValueType::Anyref, mutable: true },
                    FieldType { val_type: ValueType::Anyref, mutable: true },
                ],
            }],
            functions: vec![0],
            exports: vec![Export {
                name: "main".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x0B],
            }],
            ..Default::default()
        };

        let bytes = encode_module(&module).expect("encoding failed");
        // Should start with WASM magic.
        assert!(bytes.starts_with(&[0x00, 0x61, 0x73, 0x6D]));

        // Find the type section (section ID = 1).
        // The type section count byte should be 2 (1 func type + 1 struct type).
        // Layout: [magic(4)] [version(4)] [§1: id=0x01, size, count=2, ...]
        // Skip magic+version = 8 bytes, then check section ID.
        let mut pos = 8;
        // Find section 1.
        while pos < bytes.len() {
            let section_id = bytes[pos];
            pos += 1;
            // Decode LEB128 section size.
            let (size, sz_len) = decode_leb128(&bytes[pos..]);
            pos += sz_len;
            if section_id == 1 {
                // First byte of section payload is the type count.
                assert_eq!(bytes[pos], 2, "type section should have count=2 (1 func + 1 struct)");
                break;
            }
            pos += size;
        }
    }

    // GC Test 12: group_locals works with Anyref (non-Copy type)
    #[test]
    fn group_locals_with_anyref() {
        let locals = vec![ValueType::Anyref, ValueType::Anyref, ValueType::I32];
        let groups = group_locals(&locals);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, 2); // 2 x Anyref
        assert_eq!(groups[0].1, ValueType::Anyref);
        assert_eq!(groups[1].0, 1); // 1 x I32
        assert_eq!(groups[1].1, ValueType::I32);
    }

    /// Minimal LEB128 decoder for test assertions — decode one unsigned value.
    fn decode_leb128(bytes: &[u8]) -> (usize, usize) {
        let mut result = 0usize;
        let mut shift = 0;
        let mut len = 0;
        for &b in bytes {
            len += 1;
            result |= ((b & 0x7F) as usize) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                break;
            }
        }
        (result, len)
    }
}
