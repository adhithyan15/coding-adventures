//! # wasm-module-parser
//!
//! Parse raw `.wasm` binary bytes into a structured [`WasmModule`].
//! No execution — pure decoding.
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.
//!
//! ## The WebAssembly Binary Format
//!
//! A `.wasm` file is a compact binary encoding of a WebAssembly module. Every
//! integer uses [LEB128](https://en.wikipedia.org/wiki/LEB128) variable-length
//! encoding to keep the file small. Strings are length-prefixed UTF-8.
//!
//! The overall layout:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  WASM Binary Layout                                                     │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Magic  │  Version  │  Section...  │  Section...  │  ...               │
//! │  4 bytes│  4 bytes  │  id+size+payload             │                   │
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! Magic:   0x00 0x61 0x73 0x6D   (b"\0asm")
//! Version: 0x01 0x00 0x00 0x00   (little-endian 1)
//!
//! Each section:
//!   ┌──────┬──────────────────┬──────────────────────────────────────────┐
//!   │ id   │ size (u32 leb128)│ payload (size bytes)                     │
//!   │ 1 B  │ 1–5 bytes        │ contents vary by id                      │
//!   └──────┴──────────────────┴──────────────────────────────────────────┘
//!
//! Section IDs:
//!   0  = Custom     any position, any number of times
//!   1  = Type       function type signatures
//!   2  = Import     host-provided imports
//!   3  = Function   type index for each local function
//!   4  = Table      indirect-call tables
//!   5  = Memory     linear memory declarations
//!   6  = Global     module-level global variables
//!   7  = Export     names exported to the host
//!   8  = Start      optional auto-called function
//!   9  = Element    table initialisation data
//!   10 = Code       function bodies (locals + bytecode)
//!   11 = Data       memory initialisation data
//!
//! Numbered sections (1–11) must appear in ascending ID order; Custom (0) can
//! appear anywhere.
//! ```
//!
//! ## Section Payload Formats
//!
//! ```text
//! Type (§1):
//!   count: u32leb
//!   each:  0x60 param_count:u32leb param_types:u8[] result_count:u32leb result_types:u8[]
//!
//! Import (§2):
//!   count: u32leb
//!   each:  module:str  name:str  kind:u8  type_info
//!     str = len:u32leb  utf8_bytes
//!     kind 0 = func  → type_index:u32leb
//!     kind 1 = table → element_type:u8  limits
//!     kind 2 = mem   → limits
//!     kind 3 = global→ valtype:u8  mutable:u8
//!     limits = flags:u8  min:u32leb  [max:u32leb if flags bit0 set]
//!
//! Function (§3):  count:u32leb  type_index:u32leb × count
//! Table    (§4):  count:u32leb  element_type:u8  limits × count
//! Memory   (§5):  count:u32leb  limits × count
//!
//! Global (§6):
//!   count: u32leb
//!   each:  valtype:u8  mutable:u8  init_expr (bytes until 0x0B inclusive)
//!
//! Export (§7):
//!   count: u32leb
//!   each:  name:str  kind:u8  index:u32leb
//!
//! Start (§8):  function_index:u32leb
//!
//! Element (§9):
//!   count: u32leb
//!   each:  table_idx:u32leb  offset_expr  func_count:u32leb  func_idx:u32leb × func_count
//!
//! Code (§10):
//!   count: u32leb
//!   each:  body_size:u32leb  local_decl_count:u32leb
//!          (count:u32leb  valtype:u8) × local_decl_count
//!          code_bytes (remainder of body)
//!
//! Data (§11):
//!   count: u32leb
//!   each:  mem_idx:u32leb  offset_expr  byte_count:u32leb  data:u8 × byte_count
//!
//! Custom (§0):  name:str  data:remaining_bytes
//! ```

use wasm_leb128::decode_unsigned;
use wasm_types::{
    CustomSection, DataSegment, Element, Export, ExternalKind, FuncType, FunctionBody, Global,
    GlobalType, Import, ImportTypeInfo, Limits, MemoryType, TableType, ValueType, WasmModule,
};

// ──────────────────────────────────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────────────────────────────────

/// The 4-byte magic number at the start of every `.wasm` file.
/// Spells `\0asm` in ASCII.
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

/// The 4-byte version field, always `1` in WASM 1.0 (little-endian u32).
const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

/// The byte tag that begins every function type entry in the type section.
/// Spells `-0x20` in signed LEB128, chosen to avoid overlap with value-type bytes.
const FUNC_TYPE_TAG: u8 = 0x60;

/// The `end` opcode that terminates constant expressions (init_expr, offset_expr).
const END_OPCODE: u8 = 0x0B;

/// Section IDs from the WASM specification.
const SECTION_CUSTOM: u8 = 0;
const SECTION_TYPE: u8 = 1;
const SECTION_IMPORT: u8 = 2;
const SECTION_FUNCTION: u8 = 3;
const SECTION_TABLE: u8 = 4;
const SECTION_MEMORY: u8 = 5;
const SECTION_GLOBAL: u8 = 6;
const SECTION_EXPORT: u8 = 7;
const SECTION_START: u8 = 8;
const SECTION_ELEMENT: u8 = 9;
const SECTION_CODE: u8 = 10;
const SECTION_DATA: u8 = 11;

// ──────────────────────────────────────────────────────────────────────────────
// Error Type
// ──────────────────────────────────────────────────────────────────────────────

/// An error encountered while parsing a WASM binary.
///
/// The `offset` field indicates the byte position in the input where the error
/// was detected, which helps diagnose malformed binaries.
///
/// # Example
///
/// ```rust
/// use wasm_module_parser::WasmParseError;
///
/// let err = WasmParseError { message: "bad magic".into(), offset: 0 };
/// assert_eq!(err.to_string(), "WASM parse error at offset 0: bad magic");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct WasmParseError {
    /// Human-readable description of what went wrong.
    pub message: String,
    /// The byte offset in the input where the error was detected.
    pub offset: usize,
}

impl std::fmt::Display for WasmParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WASM parse error at offset {}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for WasmParseError {}

// ──────────────────────────────────────────────────────────────────────────────
// Parser state
// ──────────────────────────────────────────────────────────────────────────────

/// Internal parser cursor — a `&[u8]` slice with a tracked position.
///
/// The position is used only for error reporting; all actual reading goes through
/// the cursor methods which advance `pos` in lockstep with `data`.
struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Parser { data, pos: 0 }
    }

    /// Current absolute byte offset in the input.
    fn offset(&self) -> usize {
        self.pos
    }

    /// Remaining unread bytes.
    fn remaining(&self) -> usize {
        self.data.len()
    }

    /// True when all input has been consumed.
    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Create a `WasmParseError` at the current position.
    fn error(&self, msg: impl Into<String>) -> WasmParseError {
        WasmParseError {
            message: msg.into(),
            offset: self.pos,
        }
    }

    /// Read exactly `n` bytes, advancing the cursor.
    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], WasmParseError> {
        if self.data.len() < n {
            return Err(WasmParseError {
                message: format!(
                    "unexpected end of data: need {} bytes, only {} remain",
                    n,
                    self.data.len()
                ),
                offset: self.pos,
            });
        }
        let (head, tail) = self.data.split_at(n);
        self.data = tail;
        self.pos += n;
        Ok(head)
    }

    /// Read a single byte.
    fn read_u8(&mut self) -> Result<u8, WasmParseError> {
        Ok(self.read_bytes(1)?[0])
    }

    /// Decode an unsigned LEB128 u32 from the current position.
    ///
    /// LEB128 encodes integers as a variable number of 7-bit groups. The high
    /// bit of each byte is a continuation flag: 1 = more bytes follow, 0 = last.
    ///
    /// ```text
    /// Byte layout:
    ///   bit 7 (MSB): continuation flag
    ///   bits 0–6:    data
    /// ```
    fn read_u32leb(&mut self) -> Result<u32, WasmParseError> {
        // We pass the full remaining slice and absolute offset 0 (since we are
        // already positioned at the right place), then advance by `consumed`.
        match decode_unsigned(self.data, 0) {
            Ok((val, consumed)) => {
                self.data = &self.data[consumed..];
                self.pos += consumed;
                Ok(val as u32)
            }
            Err(e) => Err(WasmParseError {
                message: e.message,
                offset: self.pos + e.offset,
            }),
        }
    }

    /// Decode a length-prefixed UTF-8 string.
    ///
    /// ```text
    /// str encoding:
    ///   len: u32leb   (byte count, NOT char count)
    ///   data: utf8 bytes × len
    /// ```
    fn read_string(&mut self) -> Result<String, WasmParseError> {
        let len = self.read_u32leb()? as usize;
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| WasmParseError {
            message: "invalid UTF-8 in string".into(),
            offset: self.pos - len,
        })
    }

    /// Consume bytes up to and including the `end` opcode (0x0B).
    ///
    /// Constant expressions (`init_expr`, `offset_expr`) in WASM are just raw
    /// instruction bytes terminated by the `end` opcode. We read them verbatim
    /// so callers can inspect or re-execute them later.
    fn read_expr(&mut self) -> Result<Vec<u8>, WasmParseError> {
        let mut expr = Vec::new();
        loop {
            let b = self.read_u8()?;
            expr.push(b);
            if b == END_OPCODE {
                return Ok(expr);
            }
            // Each instruction may have immediates. We peek at the opcode to
            // read the correct number of following bytes.
            // For the common init_expr instructions (i32.const, i64.const,
            // f32.const, f64.const, global.get) we read the LEB128/raw immediate.
            match b {
                // i32.const <i32 leb128>
                0x41 => {
                    let (val, consumed) = decode_unsigned(self.data, 0).map_err(|e| {
                        WasmParseError {
                            message: e.message,
                            offset: self.pos,
                        }
                    })?;
                    let imm_bytes = &self.data[..consumed];
                    expr.extend_from_slice(imm_bytes);
                    self.data = &self.data[consumed..];
                    self.pos += consumed;
                    let _ = val;
                }
                // i64.const <i64 leb128>
                0x42 => {
                    let (val, consumed) = decode_unsigned(self.data, 0).map_err(|e| {
                        WasmParseError {
                            message: e.message,
                            offset: self.pos,
                        }
                    })?;
                    let imm_bytes = &self.data[..consumed];
                    expr.extend_from_slice(imm_bytes);
                    self.data = &self.data[consumed..];
                    self.pos += consumed;
                    let _ = val;
                }
                // f32.const <4 raw bytes>
                0x43 => {
                    let bytes = self.read_bytes(4)?;
                    expr.extend_from_slice(bytes);
                }
                // f64.const <8 raw bytes>
                0x44 => {
                    let bytes = self.read_bytes(8)?;
                    expr.extend_from_slice(bytes);
                }
                // global.get <u32 leb128>
                0x23 => {
                    let (val, consumed) = decode_unsigned(self.data, 0).map_err(|e| {
                        WasmParseError {
                            message: e.message,
                            offset: self.pos,
                        }
                    })?;
                    let imm_bytes = &self.data[..consumed];
                    expr.extend_from_slice(imm_bytes);
                    self.data = &self.data[consumed..];
                    self.pos += consumed;
                    let _ = val;
                }
                _ => {
                    // Unknown opcode inside an init_expr. The spec restricts
                    // constant expressions to a small fixed set, but for
                    // robustness we just continue scanning for END_OPCODE.
                }
            }
        }
    }

    /// Fork a sub-parser for exactly `len` bytes, advancing the main cursor.
    ///
    /// Used when parsing section payloads: we first read `len` bytes, then
    /// pass a fresh `Parser` over just those bytes to section-specific code.
    fn sub_parser(&mut self, len: usize) -> Result<Parser<'a>, WasmParseError> {
        let bytes = self.read_bytes(len)?;
        Ok(Parser {
            data: bytes,
            pos: self.pos - len, // absolute position of the start of the sub-region
        })
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Value type decoding
// ──────────────────────────────────────────────────────────────────────────────

/// Decode a single value-type byte into a [`ValueType`].
///
/// ```text
/// Byte → ValueType
/// 0x7F → I32
/// 0x7E → I64
/// 0x7D → F32
/// 0x7C → F64
/// ```
fn decode_value_type(byte: u8, offset: usize) -> Result<ValueType, WasmParseError> {
    match byte {
        0x7F => Ok(ValueType::I32),
        0x7E => Ok(ValueType::I64),
        0x7D => Ok(ValueType::F32),
        0x7C => Ok(ValueType::F64),
        _ => Err(WasmParseError {
            message: format!("unknown value type byte: 0x{:02X}", byte),
            offset,
        }),
    }
}

/// Decode a single external-kind byte into an [`ExternalKind`].
fn decode_external_kind(byte: u8, offset: usize) -> Result<ExternalKind, WasmParseError> {
    match byte {
        0x00 => Ok(ExternalKind::Function),
        0x01 => Ok(ExternalKind::Table),
        0x02 => Ok(ExternalKind::Memory),
        0x03 => Ok(ExternalKind::Global),
        _ => Err(WasmParseError {
            message: format!("unknown external kind byte: 0x{:02X}", byte),
            offset,
        }),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Limits (shared by Table and Memory)
// ──────────────────────────────────────────────────────────────────────────────

/// Parse a `limits` entry (used by table and memory sections).
///
/// ```text
/// flags: u8
///   bit 0 = 0  →  { min: u32leb }
///   bit 0 = 1  →  { min: u32leb, max: u32leb }
/// ```
fn parse_limits(p: &mut Parser) -> Result<Limits, WasmParseError> {
    let flags = p.read_u8()?;
    let min = p.read_u32leb()?;
    let max = if flags & 0x01 != 0 {
        Some(p.read_u32leb()?)
    } else {
        None
    };
    Ok(Limits { min, max })
}

// ──────────────────────────────────────────────────────────────────────────────
// Section parsers
// ──────────────────────────────────────────────────────────────────────────────

/// Parse the **type section** (§1): function signatures.
///
/// ```text
/// count: u32leb
/// entry: 0x60  param_count:u32leb  param_types:u8[]  result_count:u32leb  result_types:u8[]
/// ```
///
/// Each entry describes a distinct function signature. The type section acts as
/// a deduplicated pool of signatures that all other sections reference by index.
fn parse_type_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let tag = p.read_u8()?;
        if tag != FUNC_TYPE_TAG {
            return Err(p.error(format!(
                "expected function type tag 0x60, got 0x{:02X}",
                tag
            )));
        }
        let param_count = p.read_u32leb()? as usize;
        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            let b = p.read_u8()?;
            params.push(decode_value_type(b, p.offset() - 1)?);
        }
        let result_count = p.read_u32leb()? as usize;
        let mut results = Vec::with_capacity(result_count);
        for _ in 0..result_count {
            let b = p.read_u8()?;
            results.push(decode_value_type(b, p.offset() - 1)?);
        }
        module.types.push(FuncType { params, results });
    }
    Ok(())
}

/// Parse the **import section** (§2).
///
/// ```text
/// count: u32leb
/// entry: module:str  name:str  kind:u8  type_info
/// ```
///
/// Imports let a WASM module consume functions, tables, memories, or globals
/// that are provided by the host environment at instantiation time.
fn parse_import_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let module_name = p.read_string()?;
        let name = p.read_string()?;
        let kind_byte = p.read_u8()?;
        let kind = decode_external_kind(kind_byte, p.offset() - 1)?;
        let type_info = match kind {
            ExternalKind::Function => {
                let idx = p.read_u32leb()?;
                ImportTypeInfo::Function(idx)
            }
            ExternalKind::Table => {
                let elem_type = p.read_u8()?;
                let limits = parse_limits(p)?;
                ImportTypeInfo::Table(TableType {
                    element_type: elem_type,
                    limits,
                })
            }
            ExternalKind::Memory => {
                let limits = parse_limits(p)?;
                ImportTypeInfo::Memory(MemoryType { limits })
            }
            ExternalKind::Global => {
                let vt_byte = p.read_u8()?;
                let value_type = decode_value_type(vt_byte, p.offset() - 1)?;
                let mut_byte = p.read_u8()?;
                ImportTypeInfo::Global(GlobalType {
                    value_type,
                    mutable: mut_byte != 0,
                })
            }
        };
        module.imports.push(Import {
            module_name,
            name,
            kind,
            type_info,
        });
    }
    Ok(())
}

/// Parse the **function section** (§3): type indices for locally-defined functions.
///
/// ```text
/// count: u32leb
/// type_index: u32leb × count
/// ```
///
/// This section only stores the *type index* for each local function. The actual
/// function body (locals + bytecode) lives in the Code section (§10). The two
/// parallel arrays are matched by position: `functions[i]` → `code[i]`.
fn parse_function_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        module.functions.push(p.read_u32leb()?);
    }
    Ok(())
}

/// Parse the **table section** (§4).
///
/// ```text
/// count: u32leb
/// entry: element_type:u8(0x70)  limits
/// ```
///
/// Tables hold function references used by `call_indirect`. WASM 1.0 has at
/// most one table and its element type is always `funcref` (0x70).
fn parse_table_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let element_type = p.read_u8()?;
        let limits = parse_limits(p)?;
        module.tables.push(TableType {
            element_type,
            limits,
        });
    }
    Ok(())
}

/// Parse the **memory section** (§5).
///
/// ```text
/// count: u32leb
/// entry: limits
/// ```
///
/// Memories are linear byte arrays (starting at 0) that can grow at runtime.
/// Sizes are measured in *pages* where 1 page = 64 KiB = 65,536 bytes.
fn parse_memory_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let limits = parse_limits(p)?;
        module.memories.push(MemoryType { limits });
    }
    Ok(())
}

/// Parse the **global section** (§6).
///
/// ```text
/// count: u32leb
/// entry: valtype:u8  mutable:u8  init_expr
/// ```
///
/// Each global has a type, a mutability flag, and an initializer expression.
/// The initializer is a short byte sequence of WASM instructions (restricted to
/// compile-time-constant operations) ending with the `end` opcode (0x0B).
fn parse_global_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let vt_byte = p.read_u8()?;
        let value_type = decode_value_type(vt_byte, p.offset() - 1)?;
        let mut_byte = p.read_u8()?;
        let init_expr = p.read_expr()?;
        module.globals.push(Global {
            global_type: GlobalType {
                value_type,
                mutable: mut_byte != 0,
            },
            init_expr,
        });
    }
    Ok(())
}

/// Parse the **export section** (§7).
///
/// ```text
/// count: u32leb
/// entry: name:str  kind:u8  index:u32leb
/// ```
///
/// Exports make module-internal things (functions, memories, tables, globals)
/// visible to the host under a human-readable name.
fn parse_export_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let name = p.read_string()?;
        let kind_byte = p.read_u8()?;
        let kind = decode_external_kind(kind_byte, p.offset() - 1)?;
        let index = p.read_u32leb()?;
        module.exports.push(Export { name, kind, index });
    }
    Ok(())
}

/// Parse the **start section** (§8): optional auto-called function.
///
/// ```text
/// function_index: u32leb
/// ```
///
/// If present, the runtime calls this function automatically when the module is
/// instantiated (after memory/table initialisation). Useful for running C/C++
/// global constructors or initialising WASM runtime state.
fn parse_start_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    module.start = Some(p.read_u32leb()?);
    Ok(())
}

/// Parse the **element section** (§9): table initialisation.
///
/// ```text
/// count: u32leb
/// entry: table_idx:u32leb  offset_expr  func_count:u32leb  func_idx:u32leb × func_count
/// ```
///
/// At instantiation, `func_indices[i]` is written into `table[table_idx][offset + i]`.
/// This is how C function-pointer arrays and C++ vtables get populated.
fn parse_element_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let table_index = p.read_u32leb()?;
        let offset_expr = p.read_expr()?;
        let func_count = p.read_u32leb()? as usize;
        let mut function_indices = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            function_indices.push(p.read_u32leb()?);
        }
        module.elements.push(Element {
            table_index,
            offset_expr,
            function_indices,
        });
    }
    Ok(())
}

/// Parse the **code section** (§10): function bodies.
///
/// ```text
/// count: u32leb
/// entry:
///   body_size: u32leb          (byte count for the rest of this entry)
///   local_decl_count: u32leb
///   (count:u32leb  valtype:u8) × local_decl_count   ← run-length encoded locals
///   code_bytes                 (the rest, up to and including 0x0B)
/// ```
///
/// Locals are stored compactly: instead of one byte per local, the binary groups
/// consecutive locals of the same type: "3 i32, 2 f64". We expand these groups
/// into a flat `Vec<ValueType>` for easy indexing.
fn parse_code_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let body_size = p.read_u32leb()? as usize;
        let mut body = p.sub_parser(body_size)?;

        // --- locals (run-length encoded) ---
        let local_decl_count = body.read_u32leb()? as usize;
        let mut locals = Vec::new();
        for _ in 0..local_decl_count {
            let n = body.read_u32leb()? as usize;
            let vt_byte = body.read_u8()?;
            let vt = decode_value_type(vt_byte, body.offset() - 1)?;
            for _ in 0..n {
                locals.push(vt);
            }
        }

        // --- code bytes (everything remaining in the body, includes trailing 0x0B) ---
        let code = body.read_bytes(body.remaining())?.to_vec();

        module.code.push(FunctionBody { locals, code });
    }
    Ok(())
}

/// Parse the **data section** (§11): memory initialisation.
///
/// ```text
/// count: u32leb
/// entry: mem_idx:u32leb  offset_expr  byte_count:u32leb  data:u8 × byte_count
/// ```
///
/// At instantiation, `data[0..byte_count]` is copied into memory at the byte
/// offset computed by `offset_expr`. This is how compiled programs load string
/// literals, lookup tables, and initialised globals into linear memory.
fn parse_data_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let memory_index = p.read_u32leb()?;
        let offset_expr = p.read_expr()?;
        let byte_count = p.read_u32leb()? as usize;
        let bytes = p.read_bytes(byte_count)?.to_vec();
        module.data.push(DataSegment {
            memory_index,
            offset_expr,
            data: bytes,
        });
    }
    Ok(())
}

/// Parse a **custom section** (§0).
///
/// ```text
/// name: str
/// data: remaining bytes in the payload
/// ```
///
/// Custom sections are ignored by the WASM runtime but used by tooling:
/// - `"name"` section maps function indices to debug names
/// - `"sourceMappingURL"` points to a source map
/// - DWARF sections carry full debug info
fn parse_custom_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let name = p.read_string()?;
    let data = p.read_bytes(p.remaining())?.to_vec();
    module.customs.push(CustomSection { name, data });
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────────────────────

/// A stateless WASM module parser.
///
/// Call [`WasmModuleParser::parse`] with the raw bytes of a `.wasm` file to get
/// a fully decoded [`WasmModule`].
///
/// # Example
///
/// ```rust
/// use wasm_module_parser::WasmModuleParser;
///
/// // Minimal valid WASM module — just the 8-byte header.
/// let bytes = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
/// let module = WasmModuleParser::parse(&bytes).unwrap();
/// assert!(module.types.is_empty());
/// ```
pub struct WasmModuleParser;

impl WasmModuleParser {
    /// Parse a WASM binary into a [`WasmModule`].
    ///
    /// The parser:
    /// 1. Validates the 8-byte header (magic + version).
    /// 2. Reads sections in order; dispatches each to the appropriate section parser.
    /// 3. Returns `Err(WasmParseError)` at the first encoding violation encountered.
    ///
    /// Custom sections (ID 0) are allowed anywhere. Numbered sections (1–11) are
    /// accepted in any order for robustness, though the spec requires ascending order.
    ///
    /// # Errors
    ///
    /// Returns [`WasmParseError`] if:
    /// - The data is empty or shorter than 8 bytes.
    /// - The magic bytes do not match `\0asm`.
    /// - The version bytes do not match `\x01\x00\x00\x00`.
    /// - Any section payload is malformed (bad tags, truncated data, invalid UTF-8, etc.).
    pub fn parse(data: &[u8]) -> Result<WasmModule, WasmParseError> {
        let mut p = Parser::new(data);

        // ── Step 1: Validate the 8-byte header ──────────────────────────────
        //
        // Every .wasm file starts with the 4-byte magic `\0asm` followed by the
        // 4-byte little-endian version number 1. This lets tools quickly identify
        // the file type and reject files from incompatible WASM versions.
        //
        //   offset 0: 0x00 0x61 0x73 0x6D  ("asm" with leading null)
        //   offset 4: 0x01 0x00 0x00 0x00  (version = 1 in little-endian u32)

        if data.len() < 8 {
            return Err(WasmParseError {
                message: format!(
                    "input too short: need at least 8 bytes for the WASM header, got {}",
                    data.len()
                ),
                offset: 0,
            });
        }

        let magic = p.read_bytes(4)?;
        if magic != WASM_MAGIC {
            return Err(WasmParseError {
                message: format!(
                    "bad magic bytes: expected {:?}, got {:?}",
                    WASM_MAGIC, magic
                ),
                offset: 0,
            });
        }

        let version = p.read_bytes(4)?;
        if version != WASM_VERSION {
            return Err(WasmParseError {
                message: format!(
                    "unsupported WASM version: expected {:?}, got {:?}",
                    WASM_VERSION, version
                ),
                offset: 4,
            });
        }

        // ── Step 2: Parse sections ───────────────────────────────────────────
        //
        // After the header comes a sequence of sections. Each section starts with:
        //   - A 1-byte section ID
        //   - A u32 LEB128 size (number of bytes in the payload)
        //   - The payload bytes
        //
        // We parse each payload using a sub-parser scoped to exactly those bytes,
        // which gives precise error offsets and prevents one section from reading
        // into the next.

        let mut module = WasmModule::default();

        while !p.is_empty() {
            let section_id = p.read_u8()?;
            let section_size = p.read_u32leb()? as usize;
            let mut section_p = p.sub_parser(section_size)?;

            match section_id {
                SECTION_CUSTOM => parse_custom_section(&mut section_p, &mut module)?,
                SECTION_TYPE => parse_type_section(&mut section_p, &mut module)?,
                SECTION_IMPORT => parse_import_section(&mut section_p, &mut module)?,
                SECTION_FUNCTION => parse_function_section(&mut section_p, &mut module)?,
                SECTION_TABLE => parse_table_section(&mut section_p, &mut module)?,
                SECTION_MEMORY => parse_memory_section(&mut section_p, &mut module)?,
                SECTION_GLOBAL => parse_global_section(&mut section_p, &mut module)?,
                SECTION_EXPORT => parse_export_section(&mut section_p, &mut module)?,
                SECTION_START => parse_start_section(&mut section_p, &mut module)?,
                SECTION_ELEMENT => parse_element_section(&mut section_p, &mut module)?,
                SECTION_CODE => parse_code_section(&mut section_p, &mut module)?,
                SECTION_DATA => parse_data_section(&mut section_p, &mut module)?,
                unknown => {
                    // Unknown section IDs are silently skipped per the WASM spec's
                    // forward-compatibility rule: future proposals may add new sections.
                    // We consumed the payload bytes via sub_parser above, so we just
                    // drop `section_p` here.
                    let _ = section_p;
                    let _ = unknown;
                }
            }
        }

        Ok(module)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Minimal valid WASM module: just the 8-byte header, no sections.
    fn minimal_module() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]
    }

    /// Build a section: id + u32leb(size) + payload
    fn make_section(id: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![id];
        out.extend(encode_u32leb(payload.len() as u32));
        out.extend_from_slice(payload);
        out
    }

    /// Encode a u32 as unsigned LEB128.
    fn encode_u32leb(mut val: u32) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (val & 0x7F) as u8;
            val >>= 7;
            if val == 0 {
                out.push(byte);
                break;
            } else {
                out.push(byte | 0x80);
            }
        }
        out
    }

    /// Encode a length-prefixed UTF-8 string.
    fn encode_str(s: &str) -> Vec<u8> {
        let mut out = encode_u32leb(s.len() as u32);
        out.extend_from_slice(s.as_bytes());
        out
    }

    /// Build a complete WASM module binary: header + sections.
    fn wasm_with_sections(sections: &[Vec<u8>]) -> Vec<u8> {
        let mut out = minimal_module();
        for s in sections {
            out.extend_from_slice(s);
        }
        out
    }

    // ── Test 1: Minimal module (header only) ─────────────────────────────────
    #[test]
    fn test_minimal_module() {
        let m = WasmModuleParser::parse(&minimal_module()).unwrap();
        assert!(m.types.is_empty());
        assert!(m.imports.is_empty());
        assert!(m.functions.is_empty());
        assert!(m.tables.is_empty());
        assert!(m.memories.is_empty());
        assert!(m.globals.is_empty());
        assert!(m.exports.is_empty());
        assert!(m.start.is_none());
        assert!(m.elements.is_empty());
        assert!(m.code.is_empty());
        assert!(m.data.is_empty());
        assert!(m.customs.is_empty());
    }

    // ── Test 2: Type section — (i32, i32) → i32 ──────────────────────────────
    //
    // Binary encoding for one function type (i32,i32) → i32:
    //   01        count = 1
    //   60        func type tag
    //   02        param count = 2
    //   7F 7F     params: i32, i32
    //   01        result count = 1
    //   7F        result: i32
    #[test]
    fn test_type_section() {
        let payload = vec![
            0x01, // count = 1
            0x60, // func type tag
            0x02, 0x7F, 0x7F, // 2 params: i32, i32
            0x01, 0x7F, // 1 result: i32
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.types.len(), 1);
        assert_eq!(
            m.types[0],
            FuncType {
                params: vec![ValueType::I32, ValueType::I32],
                results: vec![ValueType::I32],
            }
        );
    }

    // ── Test 3: Function section — type index list ────────────────────────────
    #[test]
    fn test_function_section() {
        let mut payload = vec![0x02]; // count = 2
        payload.extend(encode_u32leb(0)); // func 0 → type 0
        payload.extend(encode_u32leb(1)); // func 1 → type 1
        let data = wasm_with_sections(&[make_section(3, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.functions, vec![0, 1]);
    }

    // ── Test 4: Export section — function export ──────────────────────────────
    //
    // Export function 0 as "main":
    //   01        count = 1
    //   04 "main" name
    //   00        ExternalKind::Function
    //   00        index = 0
    #[test]
    fn test_export_section() {
        let mut payload = vec![0x01]; // count = 1
        payload.extend(encode_str("main"));
        payload.push(0x00); // Function
        payload.extend(encode_u32leb(0)); // index = 0
        let data = wasm_with_sections(&[make_section(7, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.exports.len(), 1);
        assert_eq!(
            m.exports[0],
            Export {
                name: "main".into(),
                kind: ExternalKind::Function,
                index: 0,
            }
        );
    }

    // ── Test 5: Code section — function with locals ───────────────────────────
    //
    // Function body: 1 local decl (2 × i32), code = [0x0B]
    #[test]
    fn test_code_section() {
        // local decl: 1 group of 2 × i32
        let mut body: Vec<u8> = Vec::new();
        body.extend(encode_u32leb(1)); // 1 local decl
        body.extend(encode_u32leb(2)); // 2 locals
        body.push(0x7F); // type: i32
        body.push(0x0B); // end opcode

        let mut payload = encode_u32leb(1); // count = 1
        payload.extend(encode_u32leb(body.len() as u32)); // body_size
        payload.extend(&body);

        let data = wasm_with_sections(&[make_section(10, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.code.len(), 1);
        assert_eq!(m.code[0].locals, vec![ValueType::I32, ValueType::I32]);
        assert_eq!(m.code[0].code, vec![0x0B]);
    }

    // ── Test 6: Import section — function import ──────────────────────────────
    #[test]
    fn test_import_section_function() {
        let mut payload = vec![0x01]; // count = 1
        payload.extend(encode_str("env"));
        payload.extend(encode_str("abort"));
        payload.push(0x00); // Function
        payload.extend(encode_u32leb(0)); // type index = 0
        let data = wasm_with_sections(&[make_section(2, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.imports.len(), 1);
        assert_eq!(m.imports[0].module_name, "env");
        assert_eq!(m.imports[0].name, "abort");
        assert_eq!(m.imports[0].kind, ExternalKind::Function);
        assert_eq!(m.imports[0].type_info, ImportTypeInfo::Function(0));
    }

    // ── Test 7: Memory section ────────────────────────────────────────────────
    //
    // One memory with min=1 page, no max:
    //   01        count = 1
    //   00        flags = 0 (no max)
    //   01        min = 1
    #[test]
    fn test_memory_section() {
        let payload = vec![0x01, 0x00, 0x01]; // count=1, flags=0, min=1
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.memories.len(), 1);
        assert_eq!(
            m.memories[0],
            MemoryType {
                limits: Limits { min: 1, max: None }
            }
        );
    }

    // ── Test 8: Table section ─────────────────────────────────────────────────
    //
    // One funcref table with min=0, max=100:
    //   01        count = 1
    //   70        funcref element type
    //   01        flags = 1 (has max)
    //   00        min = 0
    //   64        max = 100
    #[test]
    fn test_table_section() {
        let mut payload = vec![0x01]; // count = 1
        payload.push(0x70); // funcref
        payload.push(0x01); // flags: has max
        payload.extend(encode_u32leb(0)); // min = 0
        payload.extend(encode_u32leb(100)); // max = 100
        let data = wasm_with_sections(&[make_section(4, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.tables.len(), 1);
        assert_eq!(m.tables[0].element_type, 0x70);
        assert_eq!(m.tables[0].limits, Limits { min: 0, max: Some(100) });
    }

    // ── Test 9: Global section — immutable i32 const ─────────────────────────
    //
    // global i32 (i32.const 42):
    //   01        count = 1
    //   7F        i32
    //   00        immutable
    //   41 2A 0B  i32.const 42; end
    #[test]
    fn test_global_section() {
        let payload = vec![
            0x01, // count = 1
            0x7F, // i32
            0x00, // immutable
            0x41, 0x2A, 0x0B, // i32.const 42; end
        ];
        let data = wasm_with_sections(&[make_section(6, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.globals.len(), 1);
        assert_eq!(
            m.globals[0].global_type,
            GlobalType {
                value_type: ValueType::I32,
                mutable: false
            }
        );
        assert_eq!(m.globals[0].init_expr, vec![0x41, 0x2A, 0x0B]);
    }

    // ── Test 10: Data section ─────────────────────────────────────────────────
    //
    // Data at memory 0, offset i32.const 0, content = [0xDE, 0xAD]:
    //   01        count = 1
    //   00        mem_idx = 0
    //   41 00 0B  i32.const 0; end  (offset_expr)
    //   02        byte_count = 2
    //   DE AD     data bytes
    #[test]
    fn test_data_section() {
        let payload = vec![
            0x01, // count = 1
            0x00, // mem_idx = 0
            0x41, 0x00, 0x0B, // i32.const 0; end
            0x02, 0xDE, 0xAD, // 2 bytes
        ];
        let data = wasm_with_sections(&[make_section(11, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.data.len(), 1);
        assert_eq!(m.data[0].memory_index, 0);
        assert_eq!(m.data[0].offset_expr, vec![0x41, 0x00, 0x0B]);
        assert_eq!(m.data[0].data, vec![0xDE, 0xAD]);
    }

    // ── Test 11: Element section ──────────────────────────────────────────────
    //
    // Element for table 0, offset i32.const 0, func indices = [0, 1]:
    //   01        count = 1
    //   00        table_idx = 0
    //   41 00 0B  i32.const 0; end  (offset_expr)
    //   02        func_count = 2
    //   00        func idx 0
    //   01        func idx 1
    #[test]
    fn test_element_section() {
        let payload = vec![
            0x01, // count = 1
            0x00, // table_idx = 0
            0x41, 0x00, 0x0B, // i32.const 0; end
            0x02, // func_count = 2
            0x00, // func 0
            0x01, // func 1
        ];
        let data = wasm_with_sections(&[make_section(9, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.elements.len(), 1);
        assert_eq!(m.elements[0].table_index, 0);
        assert_eq!(m.elements[0].offset_expr, vec![0x41, 0x00, 0x0B]);
        assert_eq!(m.elements[0].function_indices, vec![0, 1]);
    }

    // ── Test 12: Start section ────────────────────────────────────────────────
    #[test]
    fn test_start_section() {
        let payload = encode_u32leb(5); // start = func 5
        let data = wasm_with_sections(&[make_section(8, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.start, Some(5));
    }

    // ── Test 13: Custom section ───────────────────────────────────────────────
    #[test]
    fn test_custom_section() {
        let mut payload = encode_str("name");
        payload.extend_from_slice(b"\x01\x02\x03");
        let data = wasm_with_sections(&[make_section(0, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.customs.len(), 1);
        assert_eq!(m.customs[0].name, "name");
        assert_eq!(m.customs[0].data, vec![0x01, 0x02, 0x03]);
    }

    // ── Test 14: Multi-section module ─────────────────────────────────────────
    //
    // Build a module with type + function + export sections.
    #[test]
    fn test_multi_section_module() {
        // Type section: (i32) -> i32
        let type_payload = vec![0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F];

        // Function section: func 0 → type 0
        let func_payload = {
            let mut p = encode_u32leb(1);
            p.extend(encode_u32leb(0));
            p
        };

        // Export section: export func 0 as "add"
        let exp_payload = {
            let mut p = vec![0x01];
            p.extend(encode_str("add"));
            p.push(0x00);
            p.extend(encode_u32leb(0));
            p
        };

        // Code section: empty body (0 locals, just end)
        let code_payload = {
            let body = {
                let mut b = encode_u32leb(0); // 0 local decls
                b.push(0x0B); // end
                b
            };
            let mut p = encode_u32leb(1); // 1 function
            p.extend(encode_u32leb(body.len() as u32));
            p.extend(body);
            p
        };

        let data = wasm_with_sections(&[
            make_section(1, &type_payload),
            make_section(3, &func_payload),
            make_section(7, &exp_payload),
            make_section(10, &code_payload),
        ]);

        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.functions, vec![0]);
        assert_eq!(m.exports[0].name, "add");
        assert_eq!(m.code.len(), 1);
        assert!(m.code[0].locals.is_empty());
    }

    // ── Test 15: Error — bad magic ────────────────────────────────────────────
    #[test]
    fn test_error_bad_magic() {
        let data = vec![0xFF, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert_eq!(err.offset, 0);
        assert!(
            err.message.contains("bad magic"),
            "message was: {}",
            err.message
        );
    }

    // ── Test 16: Error — wrong version ────────────────────────────────────────
    #[test]
    fn test_error_wrong_version() {
        let data = vec![0x00, 0x61, 0x73, 0x6D, 0x02, 0x00, 0x00, 0x00];
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert_eq!(err.offset, 4);
        assert!(
            err.message.contains("version"),
            "message was: {}",
            err.message
        );
    }

    // ── Test 17: Error — empty data / truncated header ────────────────────────
    #[test]
    fn test_error_empty_data() {
        let err = WasmModuleParser::parse(&[]).unwrap_err();
        assert!(
            err.message.contains("too short") || err.message.contains("8 bytes"),
            "message was: {}",
            err.message
        );
    }

    #[test]
    fn test_error_truncated_header() {
        let data = vec![0x00, 0x61, 0x73]; // only 3 bytes
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(
            err.message.contains("too short") || err.message.contains("8 bytes"),
            "message was: {}",
            err.message
        );
    }

    // ── Test 18: Error — truncated section payload ────────────────────────────
    #[test]
    fn test_error_truncated_section_payload() {
        // Type section that claims 10 bytes but only has 1.
        let mut data = minimal_module();
        data.push(0x01); // section id = type
        data.extend(encode_u32leb(10)); // size = 10 bytes
        data.push(0x01); // only 1 byte of payload
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(
            err.message.contains("unexpected end")
                || err.message.contains("need")
                || err.message.contains("remain"),
            "message was: {}",
            err.message
        );
    }

    // ── Test 19: Round-trip — build binary, parse, verify ────────────────────
    //
    // We manually construct a module with:
    //   - Type  section: () -> ()
    //   - Func  section: type 0
    //   - Export section: export func 0 as "nop"
    //   - Code  section: body with 0 locals and `end`
    #[test]
    fn test_round_trip() {
        let type_payload = vec![
            0x01, // 1 type
            0x60, 0x00, 0x00, // () -> ()
        ];

        let func_payload = {
            let mut p = encode_u32leb(1);
            p.extend(encode_u32leb(0));
            p
        };

        let exp_payload = {
            let mut p = vec![0x01];
            p.extend(encode_str("nop"));
            p.push(0x00);
            p.extend(encode_u32leb(0));
            p
        };

        let body = {
            let mut b = encode_u32leb(0); // 0 local decls
            b.push(0x0B); // end
            b
        };
        let code_payload = {
            let mut p = encode_u32leb(1);
            p.extend(encode_u32leb(body.len() as u32));
            p.extend(body);
            p
        };

        let wasm = wasm_with_sections(&[
            make_section(1, &type_payload),
            make_section(3, &func_payload),
            make_section(7, &exp_payload),
            make_section(10, &code_payload),
        ]);

        let m = WasmModuleParser::parse(&wasm).unwrap();

        assert_eq!(m.types[0], FuncType { params: vec![], results: vec![] });
        assert_eq!(m.functions, vec![0]);
        assert_eq!(
            m.exports[0],
            Export {
                name: "nop".into(),
                kind: ExternalKind::Function,
                index: 0,
            }
        );
        assert_eq!(m.code[0].locals, vec![]);
        assert_eq!(m.code[0].code, vec![0x0B]);
    }

    // ── Additional tests for deeper coverage ─────────────────────────────────

    #[test]
    fn test_import_table() {
        let mut payload = vec![0x01]; // count = 1
        payload.extend(encode_str("host"));
        payload.extend(encode_str("tbl"));
        payload.push(0x01); // Table
        payload.push(0x70); // funcref
        payload.push(0x00); // no max
        payload.extend(encode_u32leb(10)); // min = 10
        let data = wasm_with_sections(&[make_section(2, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        if let ImportTypeInfo::Table(tt) = &m.imports[0].type_info {
            assert_eq!(tt.element_type, 0x70);
            assert_eq!(tt.limits.min, 10);
        } else {
            panic!("expected table import");
        }
    }

    #[test]
    fn test_import_memory() {
        let mut payload = vec![0x01]; // count = 1
        payload.extend(encode_str("env"));
        payload.extend(encode_str("memory"));
        payload.push(0x02); // Memory
        payload.push(0x01); // has max
        payload.extend(encode_u32leb(1)); // min = 1
        payload.extend(encode_u32leb(4)); // max = 4
        let data = wasm_with_sections(&[make_section(2, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        if let ImportTypeInfo::Memory(mt) = &m.imports[0].type_info {
            assert_eq!(mt.limits, Limits { min: 1, max: Some(4) });
        } else {
            panic!("expected memory import");
        }
    }

    #[test]
    fn test_import_global() {
        let mut payload = vec![0x01]; // count = 1
        payload.extend(encode_str("env"));
        payload.extend(encode_str("sp"));
        payload.push(0x03); // Global
        payload.push(0x7F); // i32
        payload.push(0x01); // mutable
        let data = wasm_with_sections(&[make_section(2, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        if let ImportTypeInfo::Global(gt) = &m.imports[0].type_info {
            assert_eq!(gt.value_type, ValueType::I32);
            assert!(gt.mutable);
        } else {
            panic!("expected global import");
        }
    }

    #[test]
    fn test_multiple_type_entries() {
        // Two function types: (i32)->() and ()->(f64)
        let payload = vec![
            0x02, // 2 types
            0x60, 0x01, 0x7F, 0x00, // (i32) -> ()
            0x60, 0x00, 0x01, 0x7C, // () -> (f64)
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.types.len(), 2);
        assert_eq!(m.types[0].params, vec![ValueType::I32]);
        assert!(m.types[0].results.is_empty());
        assert!(m.types[1].params.is_empty());
        assert_eq!(m.types[1].results, vec![ValueType::F64]);
    }

    #[test]
    fn test_error_display() {
        let err = WasmParseError {
            message: "test error".into(),
            offset: 42,
        };
        let s = err.to_string();
        assert!(s.contains("42"));
        assert!(s.contains("test error"));
    }

    #[test]
    fn test_custom_section_before_type() {
        // Custom sections may appear anywhere
        let mut custom_payload = encode_str("debug");
        custom_payload.extend_from_slice(b"hello");

        let type_payload = vec![0x01, 0x60, 0x00, 0x00]; // () -> ()

        let data = wasm_with_sections(&[
            make_section(0, &custom_payload),
            make_section(1, &type_payload),
            make_section(0, &{
                let mut p = encode_str("after");
                p.push(0xFF);
                p
            }),
        ]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.customs.len(), 2);
        assert_eq!(m.customs[0].name, "debug");
        assert_eq!(m.customs[1].name, "after");
    }

    #[test]
    fn test_memory_with_max() {
        let mut payload = vec![0x01]; // count = 1
        payload.push(0x01); // flags: has max
        payload.extend(encode_u32leb(2)); // min = 2
        payload.extend(encode_u32leb(8)); // max = 8
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(
            m.memories[0].limits,
            Limits { min: 2, max: Some(8) }
        );
    }

    #[test]
    fn test_code_multiple_local_decls() {
        // body with local decls: (2 × i32) + (1 × f64) + code [0x0B]
        let mut body: Vec<u8> = Vec::new();
        body.extend(encode_u32leb(2)); // 2 local decls
        body.extend(encode_u32leb(2)); // 2 × i32
        body.push(0x7F);
        body.extend(encode_u32leb(1)); // 1 × f64
        body.push(0x7C);
        body.push(0x0B); // end

        let mut payload = encode_u32leb(1); // 1 function
        payload.extend(encode_u32leb(body.len() as u32));
        payload.extend(body);

        let data = wasm_with_sections(&[make_section(10, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(
            m.code[0].locals,
            vec![ValueType::I32, ValueType::I32, ValueType::F64]
        );
    }
}
