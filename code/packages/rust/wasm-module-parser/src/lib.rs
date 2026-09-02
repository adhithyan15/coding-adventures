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

use wasm_leb128::{decode_signed_bounded, decode_unsigned, decode_unsigned_bounded};
use wasm_types::{
    CustomSection, DataSegment, Element, Export, ExternalKind, FieldType, FuncType, FunctionBody,
    Global, GlobalType, Import, ImportTypeInfo, Limits, MemoryType, StructType, TableType,
    ValueType, WasmModule,
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

/// The byte tag that begins a **WasmGC sub-type** entry in the type section
/// (the form used for struct types). It is followed by a vector of supertype
/// indices (we require it to be empty) and then the composite-type body.
const SUBTYPE_TAG: u8 = 0x50;

/// The composite-type marker for a **struct** (inside a sub-type entry).
const STRUCT_TYPE_MARKER: u8 = 0x5F;

/// The leading byte of a nullable concrete reference type (`(ref null $n)`),
/// e.g. `structref` to a named struct type: `0x63 <typeidx: u32 LEB>`.
const REF_NULL_CONCRETE_TAG: u8 = 0x63;

/// The leading byte of a NON-NULL concrete reference type (`(ref $n)`, no
/// `null` keyword) -- `0x64 <typeidx: u32 LEB>`, one more than
/// [`REF_NULL_CONCRETE_TAG`] (W32 second slice: `code/specs/
/// W32-wasm-non-null-concrete-reference-types.md`). Independently verified
/// against the real reference interpreter's `interpreter/binary/decode.ml`
/// (`ref_type`'s `-0x1c -> (NoNull, heap_type s)` arm: `-28 mod 128 =
/// 0x64`), same discipline `wasm_types::ValueType::NonNullStructRef`'s own
/// doc comment uses.
const REF_NON_NULL_CONCRETE_TAG: u8 = 0x64;

/// An upper bound on how much a length-prefixed vector may **pre-allocate**.
/// The byte stream is untrusted, so a crafted count (e.g. `0xFFFFFFFF`) must not
/// trigger a multi-gigabyte allocation before the (failing) element reads. We
/// reserve at most this many slots up front and let the `Vec` grow as elements
/// actually arrive; a truncated module then errors on a missing byte instead.
const MAX_PREALLOC: usize = 1024;

/// Upper bound on the TOTAL number of locals (summed across every
/// run-length-encoded group) a single function body may declare. See
/// `parse_code_section`'s own doc comment at the call site for the
/// DoS shape this guards against -- the short version: a group's count
/// field is legitimately as large as `u32::MAX`, and there can be many
/// groups, so without a cap a few dozen attacker-controlled bytes could
/// ask this crate to allocate billions of `ValueType` clones.
const MAX_LOCALS: u64 = 1_000_000;

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
/// Data Count section (bulk-memory-operations proposal) -- a single
/// `u32leb` declaring how many segments the data section will contain,
/// placed BEFORE the code section so validators can type-check
/// `memory.init`/`data.drop` (which reference a data segment index) without
/// a forward-reference to the data section itself. Purely a cross-check
/// value at the structural-parse layer this crate operates at: real corpus
/// evidence (`custom.wast`'s "data count and data section have
/// inconsistent lengths" `assert_malformed` case, task #84) that a mismatch
/// between the declared count and the data section's actual segment count
/// must be rejected, not silently accepted -- this section ID used to fall
/// into the generic "unknown section, skip it" arm below, so its value was
/// never even read, let alone checked.
const SECTION_DATA_COUNT: u8 = 12;

/// Maps a numbered section's byte `id` to its position in the WASM binary
/// format's CANONICAL section order -- the order every module's numbered
/// sections must appear in, at most once each. Returns `None` for
/// `SECTION_CUSTOM` (no fixed position -- allowed any number of times,
/// anywhere) and for any id the format doesn't define at all.
///
/// ## Why this can't just compare `section_id` values directly
///
/// The canonical order is **Type, Import, Function, Table, Memory, Global,
/// Export, Start, Element, DataCount, Code, Data** -- but `DataCount`'s own
/// byte id is `12`, numerically LARGER than `Code`'s (`10`) and `Data`'s
/// (`11`), even though it must appear BEFORE both of them. `DataCount` was
/// added later, by the bulk-memory proposal (a validator needs to know how
/// many data segments exist before it can type-check a `memory.init`/
/// `data.drop` inside the code section, which is why it's placed just
/// ahead of Code rather than appended at the end where its id number would
/// suggest) -- so naively rejecting any section whose numeric id isn't
/// greater than the previous one would REJECT perfectly valid modules
/// (`DataCount` then `Code`, `12` then `10`) while failing to reject some
/// invalid ones. This table's return value is the section's rank in the
/// real required sequence; callers compare THAT, not the raw id byte.
fn canonical_section_order(section_id: u8) -> Option<u8> {
    match section_id {
        SECTION_TYPE => Some(1),
        SECTION_IMPORT => Some(2),
        SECTION_FUNCTION => Some(3),
        SECTION_TABLE => Some(4),
        SECTION_MEMORY => Some(5),
        SECTION_GLOBAL => Some(6),
        SECTION_EXPORT => Some(7),
        SECTION_START => Some(8),
        SECTION_ELEMENT => Some(9),
        SECTION_DATA_COUNT => Some(10),
        SECTION_CODE => Some(11),
        SECTION_DATA => Some(12),
        _ => None,
    }
}

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
    ///
    /// Bounded to 32 significant bits via [`decode_unsigned_bounded`] --
    /// every `u32` field in the binary format (section sizes, vector counts,
    /// every index space, string/name lengths, …) goes through this one
    /// method, so bounding it here is the single fix for all of them at
    /// once. Two real, corpus-confirmed (`binary-leb128.wast`) bugs this
    /// closes that the old unbounded `decode_unsigned` + `as u32` cast
    /// could not:
    /// - **Overlong**: a `u32` field is only allowed up to `ceil(32/7) = 5`
    ///   LEB128 bytes. The old code had no cap at all beyond the shared
    ///   decoder's own generic 10-byte/`u64` limit, so a 6-, 7-, …,
    ///   10-byte encoding of a small value like `2` parsed successfully.
    /// - **Out of range**: `val as u32` silently truncates -- a value like
    ///   `2^32` (one bit too many for `u32`, but still comfortably within
    ///   `decode_unsigned`'s own `u64` range) wrapped to `0` instead of
    ///   being rejected.
    fn read_u32leb(&mut self) -> Result<u32, WasmParseError> {
        // We pass the full remaining slice and absolute offset 0 (since we are
        // already positioned at the right place), then advance by `consumed`.
        match decode_unsigned_bounded(self.data, 0, 32) {
            Ok((val, consumed)) => {
                self.data = &self.data[consumed..];
                self.pos += consumed;
                // Safe: `decode_unsigned_bounded(.., 32)` guarantees `val`
                // fits in 32 bits, or returns `Err` -- never silently wraps.
                Ok(val as u32)
            }
            Err(e) => Err(WasmParseError {
                message: e.message,
                offset: self.pos + e.offset,
            }),
        }
    }

    /// Decode an unsigned LEB128 u64 from the current position (W25 /
    /// memory64 proposal: a 64-bit memory's `min`/`max` limits, up to
    /// `2^48`, don't fit `u32`). Same underlying `decode_unsigned` shared
    /// LEB128 decoder as [`read_u32leb`](Self::read_u32leb) -- that
    /// helper already decodes into a full `u64` internally and only
    /// narrows at the very end; this is the same decode with the
    /// narrowing removed.
    fn read_u64leb(&mut self) -> Result<u64, WasmParseError> {
        match decode_unsigned(self.data, 0) {
            Ok((val, consumed)) => {
                self.data = &self.data[consumed..];
                self.pos += consumed;
                Ok(val)
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
                // i32.const <i32 leb128> -- SIGNED (s32), bounded to 32 bits:
                // the same overlong/out-of-range LEB128 rules as
                // `read_u32leb` apply here too, just with sign-extension
                // padding instead of zero padding (real corpus cases:
                // `binary-leb128.wast`'s "Signed LEB128 must not be
                // overlong"/"Signed LEB128s sign-extend" `assert_malformed`
                // groups, both specifically targeting `i32.const`/global
                // init-expr immediates). Previously used the UNSIGNED,
                // width-less `decode_unsigned` -- byte-consumption happens
                // to match for well-formed input (the two decoders share
                // the same continuation-bit loop), but it could never
                // reject an overlong or badly-padded encoding since it
                // imposed neither a byte cap nor a sign-consistency check.
                0x41 => {
                    let (val, consumed) = decode_signed_bounded(self.data, 0, 32).map_err(|e| {
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
                // i64.const <i64 leb128> -- SIGNED (s64). `decode_signed`
                // (not `decode_unsigned`) for the same reason as `i32.const`
                // above: a value like `i64.const -1` encoded with
                // deliberately-unset (instead of sign-extended) high
                // padding bits is well-formed under an unsigned reading but
                // not under the signed one the spec actually requires here
                // -- exactly `binary-leb128.wast`'s "i64.const -1 with
                // unused bits unset" case. No explicit 64-bit bound needed:
                // `decode_signed` (== `decode_signed_bounded(.., 64)`)
                // already enforces the native 10-byte/64-bit cap.
                0x42 => {
                    let (val, consumed) = wasm_leb128::decode_signed(self.data, 0).map_err(|e| {
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
                // global.get <u32 leb128> -- a global INDEX, unsigned and
                // bounded to 32 bits like every other index space in the
                // format (see `read_u32leb`'s own doc comment for why the
                // bound matters).
                0x23 => {
                    let (val, consumed) = decode_unsigned_bounded(self.data, 0, 32).map_err(|e| {
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
                // ref.func <funcidx: u32 leb128> -- a function INDEX, same
                // shape/bound as `global.get`'s immediate above. Added
                // alongside `read_value_type` reuse for `ref.null` (0xD0)
                // below as part of the W32 security-review follow-up: this
                // catch-all previously did NOT know `ref.func` has an
                // immediate at all, so its funcidx byte(s) were treated as
                // the START of the next "instruction" -- a genuine
                // byte-stream desync on any real `(global funcref (ref.func
                // $f))`/`(elem ... (item (ref.func $f)))`-shaped constant
                // expression, confirmed by direct execution: a funcidx
                // whose LEB128 encoding happens to end in `0x0B` (e.g. 11)
                // was misread as an early `end`, and the genuinely-leftover
                // byte then silently became the FIRST byte of whatever data
                // followed the expression in the section (demonstrated to
                // corrupt a data-segment's payload without any parse
                // error). `read_u32leb` (not the unbounded `decode_unsigned`)
                // for the same overlong/out-of-range rejection every other
                // index-space read in this crate already gets.
                0xD2 => {
                    let before = self.data;
                    self.read_u32leb()?;
                    let consumed = before.len() - self.data.len();
                    expr.extend_from_slice(&before[..consumed]);
                }
                // ref.null <heaptype> -- either a single abstract-heap-type
                // byte (`funcref`=0x70, `externref`=0x6F, the four W32
                // bottom types, etc.) or a 2-byte concrete-type-index form
                // (`0x63`/`0x64 <typeidx: u32 leb128>`), exactly the same
                // shapes `read_value_type` already knows how to decode for
                // struct field types -- reused here rather than
                // reimplementing the tag-byte dispatch a second time. Same
                // desync bug class as `ref.func` above: the catch-all
                // previously consumed only `ref.null`'s own opcode byte and
                // left its heap-type immediate to be misread as the next
                // instruction.
                0xD0 => {
                    let before = self.data;
                    read_value_type(self)?;
                    let consumed = before.len() - self.data.len();
                    expr.extend_from_slice(&before[..consumed]);
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
/// 0x7B → V128   (SIMD proposal, see below)
/// 0x6E → Anyref (WasmGC)
/// 0x6C → I31ref (WasmGC)
/// ```
fn decode_value_type(byte: u8, offset: usize) -> Result<ValueType, WasmParseError> {
    match byte {
        0x7F => Ok(ValueType::I32),
        0x7E => Ok(ValueType::I64),
        0x7D => Ok(ValueType::F32),
        0x7C => Ok(ValueType::F64),
        // `v128` (SIMD proposal, first slice added in SIMD PR1a) -- a
        // single byte like the 4 MVP scalars above, not a handle or any
        // other indirection at THIS layer; this crate only decodes the
        // module's declared SHAPE (what type a param/result/local IS),
        // never a runtime value, so there's nothing SIMD-specific to do
        // here beyond recognizing the byte. Needed for any `(module
        // binary ...)` whose type section declares a `(result v128)` (or
        // a v128 param/local) -- confirmed against the real, pinned-
        // commit `simd_const.wast`, whose `(module binary ...)` directives
        // do exactly this (e.g. its `parse_i32x4`/`parse_f64x2` etc. test
        // functions). Was previously a real gap: this crate's own binary
        // decoder had no arm for 0x7B at all, even though `wasm-types`
        // (0.1.3) and `wasm-wast-parser`'s TEXT-format decoder (SIMD
        // PR1b-2) already recognized it -- the code SECTION itself never
        // needed a SIMD-aware fix (it's read as raw, undecoded bytes; see
        // `parse_code_section`), only the TYPE-section byte did.
        0x7B => Ok(ValueType::V128),
        // WasmGC single-byte reference types (LANG77 / McCarthy L3b-3a-3c).
        // `structref` (`0x63 <typeidx>`) is multi-byte and is handled by
        // [`read_value_type`], which is why it is absent here.
        0x6E => Ok(ValueType::Anyref),
        0x6C => Ok(ValueType::I31ref),
        // WASM1.0/reference-types-proposal single-byte reference types.
        // `funcref`/`externref` were previously only recognized in
        // table-element-type/heap-type-immediate contexts elsewhere in
        // this crate, never as a plain value-type byte here -- adding
        // them alongside the four W32 bottom types below (rather than
        // leaving that pre-existing gap for this addendum to silently
        // paper over) keeps this decoder able to round-trip every
        // `ValueType::encode()` output byte-for-byte, matching
        // `wasm-module-encoder`'s universal `.encode()` call sites.
        0x70 => Ok(ValueType::Funcref),
        0x6F => Ok(ValueType::Externref),
        0x69 => Ok(ValueType::Exnref),
        // The four W32-first-slice BOTTOM reference types (`code/specs/
        // W32-wasm-non-null-concrete-reference-types.md`): single-byte,
        // independently verified against the real reference interpreter's
        // `interpreter/binary/decode.ml` (see `ValueType::NullFuncref`'s
        // own doc comment for the derivation), matching `ValueType::
        // encode()`'s output for these variants exactly.
        0x73 => Ok(ValueType::NullFuncref),
        0x72 => Ok(ValueType::NullExternref),
        0x74 => Ok(ValueType::NullExnref),
        0x71 => Ok(ValueType::NullRef),
        _ => Err(WasmParseError {
            message: format!("unknown value type byte: 0x{:02X}", byte),
            offset,
        }),
    }
}

/// Read one `ValueType` from the stream, handling the **multi-byte** WasmGC
/// reference encodings that [`decode_value_type`] cannot (it only sees a single
/// byte).  Specifically, a nullable concrete struct reference is
/// `0x63 <typeidx: u32 LEB>`; every other value type is a single byte.
///
/// Used when parsing WasmGC struct field types, whose field value types may be
/// `anyref`, `i31ref`, or a concrete `structref` (LANG77 / McCarthy L3b-3a-3c).
fn read_value_type(p: &mut Parser) -> Result<ValueType, WasmParseError> {
    let byte = p.read_u8()?;
    if byte == REF_NULL_CONCRETE_TAG {
        let idx = p.read_u32leb()?;
        return Ok(ValueType::StructRef(idx));
    }
    // W32 second slice: a NON-NULL concrete reference in a struct field
    // (`(field (ref $t))`, no `null` keyword) -- same 2-byte shape, one
    // more than `REF_NULL_CONCRETE_TAG`. This function is only ever called
    // for STRUCT FIELD types (see its own doc comment), so -- exactly like
    // `REF_NULL_CONCRETE_TAG` immediately above -- the index always names
    // a struct type here, never a function type: `wasm-wast-parser` has no
    // struct-type TEXT-format declarations at all, so no real `.wast`
    // source can produce `NonNullConcreteFuncRef` via THIS path either.
    if byte == REF_NON_NULL_CONCRETE_TAG {
        let idx = p.read_u32leb()?;
        return Ok(ValueType::NonNullStructRef(idx));
    }
    // `offset()` now points one past `byte`, so the error offset is `- 1`.
    decode_value_type(byte, p.offset().saturating_sub(1))
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

/// Decode a mutability byte (used by global imports, global definitions, and
/// WasmGC struct fields) into `bool`. Per spec this is a one-bit flag with
/// exactly two legal encodings, `0x00` (immutable) and `0x01` (mutable) --
/// NOT "any nonzero byte means mutable". A real corpus gap (found via a
/// `wasm-conformance` prioritization scan after task #80): every call site
/// here used to do `byte != 0`, silently accepting a value like `0x04` as
/// "mutable" instead of rejecting it, so the official testsuite's
/// `global.wast` `assert_malformed` cases (binary global encodings that
/// deliberately use `0x04` to test this exact rule, expecting the message
/// "malformed mutability") were wrongly parsing as valid modules.
fn decode_mutability(byte: u8, offset: usize) -> Result<bool, WasmParseError> {
    match byte {
        0x00 => Ok(false),
        0x01 => Ok(true),
        _ => Err(WasmParseError {
            message: format!("malformed mutability: 0x{:02X}", byte),
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
///   bit 1 = 1  →  shared (threads proposal, memory only -- WASM18)
///   bit 2 = 1  →  64-bit index (memory64/table64 proposals, W25/W26):
///                 min/max are u64leb instead of u32leb
/// ```
///
/// Verified live against the real spec's binary grammar (`https://
/// webassembly.github.io/spec/core/binary/types.html`) while implementing
/// W25: `0x00`/`0x01` = 32-bit index, `0x04`/`0x05` = 64-bit index --
/// `min`/`max` are `u64leb` in the 64-bit case (needed for real, spec-valid
/// values up to `2^48`, past `u32`'s range).
///
/// Returns `(Limits, shared, is64)`. `shared` is only ever meaningful for a
/// memory (a real encoder never sets bit 1 on a table's limits -- WASM18's
/// threads proposal has no shared-table concept); `is64` is meaningful for
/// BOTH kinds as of W26 (table64) -- both call sites (table and memory) now
/// wire it into their own `TableType`/`MemoryType.is64` field, reading it
/// unconditionally here rather than threading a "which kind is this" flag
/// through keeps this one shared helper simple.
fn parse_limits(p: &mut Parser) -> Result<(Limits, bool, bool), WasmParseError> {
    const IS64_FLAG: u8 = 0x04;
    // Only bits 0 (has-max), 1 (shared), 2 (64-bit index) are defined by any
    // proposal this crate supports -- anything else set is a genuinely
    // unrecognized flags encoding, not a value to tolerate. Real corpus bug
    // (`binary.wast`'s "malformed limits flags" cases: `0x08`, `0x10`,
    // `0x81`, all with exactly one bit outside this mask set): the old code
    // only ever tested individual bits it cared about (`flags & 0x01`,
    // `flags & IS64_FLAG`) and never rejected a stray bit elsewhere in the
    // byte, so e.g. `0x08` silently parsed identically to `0x00` (no max,
    // 32-bit, not shared) instead of being rejected.
    const VALID_FLAGS_MASK: u8 = 0x01 | 0x02 | IS64_FLAG;
    let flags_offset = p.offset();
    let flags = p.read_u8()?;
    if flags & !VALID_FLAGS_MASK != 0 {
        return Err(WasmParseError {
            message: format!("malformed limits flags: 0x{flags:02X}"),
            offset: flags_offset,
        });
    }
    let is64 = flags & IS64_FLAG != 0;
    let (min, max) = if is64 {
        let min = p.read_u64leb()?;
        let max = if flags & 0x01 != 0 { Some(p.read_u64leb()?) } else { None };
        (min, max)
    } else {
        let min = p.read_u32leb()? as u64;
        let max = if flags & 0x01 != 0 { Some(p.read_u32leb()? as u64) } else { None };
        (min, max)
    };
    let shared = flags & 0x02 != 0;
    Ok((Limits { min, max }, shared, is64))
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
        match tag {
            FUNC_TYPE_TAG => {
                let func_type = parse_func_type(p)?;
                module.types.push(func_type);
            }
            // WasmGC struct type (LANG77 / McCarthy L3b-3a-3c). The `$LispyPair`
            // cons cell is emitted as a sub-type entry; recover it so the
            // runtime can learn its field count (e.g. for `struct.new`).
            //
            // NOTE: function and struct types share one type-index space. The
            // encoder emits all function types first, then struct types, so a
            // function's `module.types` index still equals its wasm type index;
            // a producer that interleaved them would break that alignment, which
            // we neither emit nor (currently) need to consume.
            SUBTYPE_TAG => {
                let struct_type = parse_struct_type(p)?;
                module.struct_types.push(struct_type);
            }
            other => {
                return Err(p.error(format!(
                    "expected a function type (0x60) or struct sub-type (0x50), got 0x{:02X}",
                    other
                )));
            }
        }
    }
    Ok(())
}

/// Parse a function type body — everything **after** the `0x60` tag.
///
/// ```text
/// param_count: u32leb   params: valtype × param_count
/// result_count: u32leb  results: valtype × result_count
/// ```
fn parse_func_type(p: &mut Parser) -> Result<FuncType, WasmParseError> {
    let param_count = p.read_u32leb()? as usize;
    let mut params = Vec::with_capacity(param_count.min(MAX_PREALLOC));
    for _ in 0..param_count {
        let b = p.read_u8()?;
        params.push(decode_value_type(b, p.offset() - 1)?);
    }
    let result_count = p.read_u32leb()? as usize;
    let mut results = Vec::with_capacity(result_count.min(MAX_PREALLOC));
    for _ in 0..result_count {
        let b = p.read_u8()?;
        results.push(decode_value_type(b, p.offset() - 1)?);
    }
    Ok(FuncType { params, results })
}

/// Parse a WasmGC **struct type** body — everything **after** the `0x50`
/// sub-type tag.  Mirrors `wasm-module-encoder`'s `encode_struct_type`:
///
/// ```text
/// 0x50                      ;; sub-type open tag (already consumed by caller)
/// supertype_count: u32leb   ;; must be 0 (we don't support explicit supertypes)
/// 0x5F                      ;; struct composite-type marker
/// field_count: u32leb
/// for each field:
///   val_type bytes          ;; ValueType (anyref/i31ref/structref/numeric)
///   mutability: u8          ;; 0 = immutable, 1 = mutable
/// ```
///
/// For `$LispyPair` (two mutable `anyref` fields) the bytes after `0x50` are
/// `0x00 0x5F 0x02  0x6E 0x01  0x6E 0x01`.
fn parse_struct_type(p: &mut Parser) -> Result<StructType, WasmParseError> {
    let supertype_count = p.read_u32leb()?;
    if supertype_count != 0 {
        return Err(p.error(format!(
            "WasmGC struct sub-types with explicit supertypes are not supported \
             (supertype_count = {supertype_count})"
        )));
    }
    let marker = p.read_u8()?;
    if marker != STRUCT_TYPE_MARKER {
        return Err(p.error(format!(
            "expected struct composite-type marker 0x5F, got 0x{marker:02X}"
        )));
    }
    let field_count = p.read_u32leb()? as usize;
    // Do NOT pre-allocate with the attacker-controlled `field_count`: a crafted
    // module could claim a huge count and force a giant allocation before the
    // (failing) reads. Grow on demand instead — a truncated module then errors
    // on the first missing byte without over-allocating.
    let mut fields = Vec::new();
    for _ in 0..field_count {
        let val_type = read_value_type(p)?;
        let mutability = p.read_u8()?;
        fields.push(FieldType::plain(val_type, decode_mutability(mutability, p.offset() - 1)?));
    }
    Ok(StructType { fields })
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
                // W26 (table64 proposal): `is64` (flags bit 0x04) is now
                // wired into `TableType.is64`, mirroring `Memory` below --
                // previously rejected outright (see git history/W25's own
                // "table64 out of scope" note).
                let (limits, _shared, is64) = parse_limits(p)?;
                ImportTypeInfo::Table(TableType {
                    element_type: elem_type,
                    limits,
                    is64,
                })
            }
            ExternalKind::Memory => {
                let (limits, shared, is64) = parse_limits(p)?;
                ImportTypeInfo::Memory(MemoryType { limits, shared, is64 })
            }
            ExternalKind::Global => {
                let vt_byte = p.read_u8()?;
                let value_type = decode_value_type(vt_byte, p.offset() - 1)?;
                let mut_byte = p.read_u8()?;
                ImportTypeInfo::Global(GlobalType {
                    value_type,
                    mutable: decode_mutability(mut_byte, p.offset() - 1)?,
                })
            }
            // W21 (exceptions proposal): `decode_external_kind` above
            // never actually produces `Tag` (still only recognizes bytes
            // 0x00-0x03 -- real binary tag-section/tag-import decoding
            // stays out of scope for this slice, matching W20's own
            // precedent of not touching this crate; `wasm-wast-parser`'s
            // text pipeline, this repo's real corpus entry point, never
            // round-trips through this binary parser at all). This arm
            // exists only so the match stays exhaustive now that
            // `ExternalKind` has a 5th variant -- unreachable in practice.
            ExternalKind::Tag => {
                return Err(WasmParseError {
                    message: "tag imports are not supported by the binary module parser".to_string(),
                    offset: p.offset(),
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

/// The leading byte of a table entry that carries an explicit INIT
/// EXPRESSION (function-references proposal): `0x40 0x00 et:reftype
/// lim:limits init:expr`, used when `et` isn't defaultable to `ref.null`
/// (e.g. a non-nullable `(ref func)` table) so the table's initial contents
/// need a real initializer instead of every slot defaulting to null. Not
/// itself a valid `element_type` byte in the plain `et:reftype lim:limits`
/// form this crate otherwise parses.
const TABLE_WITH_INIT_EXPR_TAG: u8 = 0x40;

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
        let element_type_offset = p.offset();
        let element_type = p.read_u8()?;
        // Real corpus bug, surfaced (not introduced) by this same pass's
        // section-size-mismatch check (`elem.wast`'s own function-
        // references-proposal table entries): `0x40` isn't a valid
        // `element_type` byte in the plain two-field form this function
        // otherwise parses -- it's the leading byte of an entirely
        // DIFFERENT, longer entry shape (`0x40 0x00 et:reftype
        // lim:limits init:expr`) that needs a stored initializer
        // expression `TableType` has no field for yet. Before the
        // size-mismatch check existed, this branch didn't exist either:
        // `0x40` was silently read AS IF it were a normal element_type,
        // `parse_limits` then misread several bytes of the reftype/init
        // expression AS IF they were a flags+min/max limits encoding, and
        // the resulting garbage `TableType` plus leftover unconsumed
        // bytes were both silently accepted -- a real, if invisible
        // (nothing downstream happened to reject `element_type: 0x40`),
        // decode-time bug. Failing loudly and immediately here, with a
        // message that names the real gap, is strictly better than that
        // silent corruption -- even though it means these specific
        // modules now correctly grade `NotYetSupported` (an honest
        // capability gap) rather than the false `Pass` the silent
        // misparse used to produce.
        if element_type == TABLE_WITH_INIT_EXPR_TAG {
            return Err(WasmParseError {
                message: "table with an explicit init expression (function-references proposal) is not yet supported".to_string(),
                offset: element_type_offset,
            });
        }
        // W26 (table64 proposal): `is64` (flags bit 0x04) is now wired
        // into `TableType.is64`, mirroring `Memory`'s own arm above --
        // previously rejected outright (see git history/W25's own
        // "table64 out of scope" note).
        let (limits, _shared, is64) = parse_limits(p)?;
        module.tables.push(TableType {
            element_type,
            limits,
            is64,
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
        let (limits, shared, is64) = parse_limits(p)?;
        module.memories.push(MemoryType { limits, shared, is64 });
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
        // Validate BEFORE `read_expr()` advances past it -- `p.offset()` must
        // still point one-past the mutability byte itself here, not one-past
        // the (variable-length) init expression that follows it.
        let mutable = decode_mutability(mut_byte, p.offset() - 1)?;
        let init_expr = p.read_expr()?;
        module.globals.push(Global {
            global_type: GlobalType { value_type, mutable },
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
/// The real encoding has **eight** binary segment-mode variants (bulk-
/// table + reference-types proposals); this repo decodes the four
/// (task #97, `code/specs/W17-wasm-bulk-table-ops.md`) confirmed by
/// direct census of the two real corpus files that need them:
///
/// | `flags` | Mode                              | Fields between `flags` and the entry list |
/// |---------|------------------------------------|--------------------------------------------|
/// | `0x00`  | active, implicit table 0, funcidx  | `offset_expr` only                        |
/// | `0x01`  | passive, funcidx-list              | `elemkind:u8` (must be `0x00`)            |
/// | `0x02`  | active, explicit table, funcidx    | `table_idx:u32leb` then `offset_expr`     |
/// | `0x05`  | passive, exprs-list (funcref only) | `reftype:u8` (must be `0x70`)             |
///
/// Modes 3/7 (declarative) and 4/6 (an ACTIVE segment carrying exprs) are
/// each a clean, explicit `WasmParseError` -- not a silent misparse --
/// same "structurally earlier than data segments were pre-task-#95"
/// posture the old unconditional-mode-0-only version of this function
/// had (it never even read a `flags` byte at all, so it couldn't
/// distinguish ANY mode).
///
/// A funcidx-list entry (modes 0-2) is a bare `funcidx:u32leb`, always
/// `Some`. An exprs-list entry (mode 5) is a real encoded constant
/// expression -- this repo only decodes the two shapes its own corpus
/// actually uses, `ref.func funcidx` (→ `Some(funcidx)`) and `ref.null
/// func` (→ `None`, a real null table slot, not merely absent) -- any
/// other expression opcode is a clean parse error, matching this
/// function's own scoped-modes discipline.
fn parse_element_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let flags = p.read_u32leb()?;
        let (table_index, offset_expr, is_passive, use_exprs) = match flags {
            0 => (0, p.read_expr()?, false, false),
            1 => {
                let elemkind = p.read_u8()?;
                if elemkind != 0x00 {
                    return Err(p.error(format!("unsupported element segment elemkind {elemkind:#04x} (only funcref/0x00 supported)")));
                }
                (0, Vec::new(), true, false)
            }
            2 => {
                // Real, pre-existing parsing bug surfaced (not introduced)
                // by this pass's section-size-mismatch check
                // (`elem.wast`'s own explicit-table-index active segments):
                // per spec, mode 2's entry shape is `tableidx:u32leb
                // offset:expr elemkind:u8 vec(funcidx)` -- the SAME
                // `elemkind` byte mode 1 (passive) already reads just
                // below, just after the offset expression instead of
                // instead of it. This arm skipped straight from the
                // offset expression to `entry_count`, so it silently read
                // the elemkind byte (always `0x00`/funcref in every
                // encoder this crate or the real corpus produces) AS IF
                // it were the low byte of `entry_count`'s LEB128 -- for a
                // real 1-or-more-element segment this desynchronized
                // every subsequent read by exactly one byte, silently
                // producing a garbage (usually too-small, sometimes
                // outright wrong) element list instead of erroring.
                let table_index = p.read_u32leb()?;
                let offset_expr = p.read_expr()?;
                let elemkind = p.read_u8()?;
                if elemkind != 0x00 {
                    return Err(p.error(format!("unsupported element segment elemkind {elemkind:#04x} (only funcref/0x00 supported)")));
                }
                (table_index, offset_expr, false, false)
            }
            5 => {
                let reftype = p.read_u8()?;
                if reftype != 0x70 {
                    return Err(p.error(format!("unsupported element segment reftype {reftype:#04x} (only funcref/0x70 supported)")));
                }
                (0, Vec::new(), true, true)
            }
            other => {
                return Err(p.error(format!("unsupported element segment mode flags {other} (only 0/1/2/5 supported)")));
            }
        };
        let entry_count = p.read_u32leb()? as usize;
        let mut function_indices = Vec::with_capacity(entry_count.min(MAX_PREALLOC));
        for _ in 0..entry_count {
            if use_exprs {
                function_indices.push(read_elem_expr_entry(p)?);
            } else {
                function_indices.push(Some(p.read_u32leb()?));
            }
        }
        // W38 slice 4 (`code/specs/W38-wasm-gc-array-bulk-ops.md`,
        // Correction 2): binary-format encoding of `array.init_elem`/
        // `array.new_elem` themselves is still explicitly out of scope for
        // this decoder (confirmed by corpus grep: no file in the cluster
        // uses `(module binary ...)` for these two instructions), but
        // `item_exprs` is populated for real anyway, purely so this
        // decoder's own segments satisfy the field's documented "always a
        // real byte sequence, same length as `function_indices`"
        // invariant identically to the text parser's -- this decoder only
        // ever accepts funcref-family entries (modes 0/1/2/5, the `reftype
        // != 0x70` check above already rejects anything else), so every
        // entry re-encodes as a plain `ref.func`/`ref.null func` constant
        // expression, exactly what `function_indices` already says it is.
        let item_exprs: Vec<Vec<u8>> = function_indices
            .iter()
            .map(|entry| match entry {
                Some(idx) => {
                    let mut bytes = vec![0xD2];
                    bytes.extend(wasm_leb128::encode_unsigned(*idx as u64));
                    bytes.push(0x0B);
                    bytes
                }
                None => vec![0xD0, 0x70, 0x0B], // ref.null func; end
            })
            .collect();
        module.elements.push(Element {
            table_index,
            offset_expr,
            function_indices,
            is_passive,
            // This decoder only ever accepts modes 0/1/2/5 (see this
            // function's own doc comment) -- modes 3/7 (declarative) are a
            // clean parse error, never reach this push. Always `false`.
            is_declarative: false,
            item_exprs,
            // This decoder only ever accepts funcref-family entries (see
            // this block's own doc comment above) -- always the implicit
            // funcref elemkind, matching the real spec's own binary-format
            // default and `wasm-wast-parser`'s identical convention for a
            // plain funcidx-list segment.
            declared_type: ValueType::Funcref,
        });
    }
    Ok(())
}

/// Decode one exprs-list element-segment entry: a real encoded constant
/// expression, restricted to the two shapes this repo's own corpus uses
/// (see `parse_element_section`'s own doc comment) -- `ref.func funcidx`
/// (`0xD2 <funcidx:u32leb> 0x0B`) or `ref.null func` (`0xD0 0x70 0x0B`).
/// Deliberately NOT implemented via the generic `read_expr` (which reads
/// arbitrary constant-expression bytes verbatim for `offset_expr`/
/// `init_expr` use sites): `read_expr`'s own catch-all arm for an
/// unrecognized opcode does not know to skip `ref.func`'s funcidx
/// immediate or `ref.null`'s heaptype immediate, so reusing it here
/// would misparse the very shapes this function exists to decode.
fn read_elem_expr_entry(p: &mut Parser) -> Result<Option<u32>, WasmParseError> {
    let opcode = p.read_u8()?;
    let value = match opcode {
        0xD2 => Some(p.read_u32leb()?), // ref.func <funcidx>
        0xD0 => {
            let _heaptype = p.read_u8()?; // ref.null <heaptype> -- byte consumed, value unused
            None
        }
        other => {
            return Err(p.error(format!(
                "unsupported element exprs-list entry opcode {other:#04x} (only ref.func/0xD2 and ref.null/0xD0 supported)"
            )));
        }
    };
    let end = p.read_u8()?;
    if end != END_OPCODE {
        return Err(p.error("element exprs-list entry missing terminating end opcode"));
    }
    Ok(value)
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
        // Real corpus bug (`binary.wast`'s "too many locals" cases): each
        // group's own count `n` is a plain u32leb -- legitimately as large
        // as `u32::MAX` per this crate's own (correct) `read_u32leb`, and
        // there can be many groups. Before this running total + cap, `for
        // _ in 0..n { locals.push(vt) }` would try to push that many clones
        // from a handful of attacker-controlled bytes -- the same
        // tiny-file-claims-enormous-allocation shape `MAX_PREALLOC` already
        // guards against for the element section's `func_count` (see that
        // constant's own doc comment), just summed across groups instead
        // of a single field.
        let mut total_locals: u64 = 0;
        for _ in 0..local_decl_count {
            let n = body.read_u32leb()?;
            let vt_byte = body.read_u8()?;
            let vt = decode_value_type(vt_byte, body.offset() - 1)?;
            total_locals += n as u64;
            if total_locals > MAX_LOCALS {
                return Err(WasmParseError {
                    message: format!(
                        "too many locals: total of {total_locals} exceeds the {MAX_LOCALS} limit"
                    ),
                    offset: body.offset(),
                });
            }
            for _ in 0..n {
                // ValueType is no longer Copy (WasmGC added StructRef(u32));
                // clone for each local in the run.
                locals.push(vt);
            }
        }

        // --- code bytes (everything remaining in the body, includes trailing 0x0B) ---
        let code = body.read_bytes(body.remaining())?.to_vec();

        // Real corpus bug (`binary.wast`'s "END opcode expected"/
        // "unexpected end of section or function" cases): a function
        // body's instruction stream is required to end with the `end`
        // (0x0B) opcode. Nothing here ever checked that -- `body_size`
        // taken at face value let a body whose last declared byte is, say,
        // `drop` (0x1A) parse as a perfectly normal (truncated) function.
        if code.last() != Some(&END_OPCODE) {
            return Err(WasmParseError {
                message: "END opcode expected: function body does not end with 0x0B".to_string(),
                offset: body.offset(),
            });
        }

        module.code.push(FunctionBody { locals, code });
    }
    Ok(())
}

/// Parse the **data section** (§11): memory initialisation.
///
/// ```text
/// count: u32leb
/// entry: flags:u32leb  (mode-dependent fields)  byte_count:u32leb  data:u8 × byte_count
/// ```
///
/// The real encoding has three segment-mode variants (bulk-memory
/// proposal, task #95), distinguished by the leading `flags` LEB128:
///
/// | `flags` | Mode                    | Fields between `flags` and `byte_count`   |
/// |---------|--------------------------|--------------------------------------------|
/// | `0x00`  | active, implicit mem 0  | `offset_expr` only                        |
/// | `0x01`  | passive                 | (none)                                    |
/// | `0x02`  | active, explicit memory | `mem_idx:u32leb` then `offset_expr`       |
///
/// Mode 0 happens to be byte-identical to reading a bare `memory_index:
/// u32leb` of `0` followed by an offset expression -- which is exactly
/// what an earlier version of this function did unconditionally, so it
/// "worked" only because every real-world module used mode 0. It could
/// not have decoded mode 1 (no offset expr present at all -- the
/// following data bytes would have been misread as one) or mode 2
/// (`flags` isn't the memory index; a real explicit index follows it)
/// correctly. Any `flags` value other than 0/1/2 is a real WASM spec
/// violation (`InvalidDataSegment`), not silently accepted.
///
/// A **passive** segment (`is_passive: true`) is never applied
/// automatically at instantiation -- it stays resident so `memory.init`
/// can copy from it (any number of times, into any memory) until
/// `data.drop` frees it. An **active** segment is applied once, at
/// instantiation time, to the memory `memory_index` names at the byte
/// offset `offset_expr` computes.
fn parse_data_section(p: &mut Parser, module: &mut WasmModule) -> Result<(), WasmParseError> {
    let count = p.read_u32leb()? as usize;
    for _ in 0..count {
        let flags = p.read_u32leb()?;
        let (memory_index, offset_expr, is_passive) = match flags {
            0 => (0, p.read_expr()?, false),
            1 => (0, Vec::new(), true),
            2 => {
                let memory_index = p.read_u32leb()?;
                (memory_index, p.read_expr()?, false)
            }
            other => {
                return Err(p.error(format!("unsupported data segment mode flags {other}")));
            }
        };
        let byte_count = p.read_u32leb()? as usize;
        let bytes = p.read_bytes(byte_count)?.to_vec();
        module.data.push(DataSegment {
            memory_index,
            offset_expr,
            data: bytes,
            is_passive,
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
        let mut data_count: Option<u32> = None;
        // Tracks how far along the CANONICAL section sequence we've gotten
        // (see `canonical_section_order`'s own doc comment) -- `0` means
        // "no numbered section seen yet". Custom sections (id 0) never
        // touch this: they may appear any number of times, anywhere.
        let mut last_section_order: u8 = 0;

        while !p.is_empty() {
            let section_id_offset = p.offset();
            let section_id = p.read_u8()?;

            // Real corpus bug (`binary.wast`'s "malformed section id" cases,
            // ids 13/127/128/129/255): the OLD code's doc comment claimed
            // "future proposals may add new sections" and silently skipped
            // any id it didn't recognize. That reasoning doesn't hold up --
            // the spec defines exactly ids 0-12 and requires anything else
            // to be REJECTED, not tolerated. A section whose id isn't
            // Custom(0) and isn't in the canonical sequence below is
            // malformed.
            if section_id != SECTION_CUSTOM && canonical_section_order(section_id).is_none() {
                return Err(WasmParseError {
                    message: format!("malformed section id: 0x{section_id:02X}"),
                    offset: section_id_offset,
                });
            }

            let section_size = p.read_u32leb()? as usize;
            let mut section_p = p.sub_parser(section_size)?;

            // Real corpus bug (`binary.wast`'s "unexpected content after
            // last section" cases): every numbered section must appear at
            // most once, in ascending CANONICAL order (see that function's
            // own doc comment for why this is "canonical position", not
            // raw numeric id) -- a repeated section, or one that appears
            // out of sequence, is malformed. The old code never checked
            // this at all: it happily accepted `Global, Global` or
            // `Export, Import` back to back.
            if let Some(order) = canonical_section_order(section_id) {
                if order <= last_section_order {
                    return Err(WasmParseError {
                        message: format!(
                            "unexpected content after last section: section id 0x{section_id:02X} is repeated or out of the required order"
                        ),
                        offset: section_id_offset,
                    });
                }
                last_section_order = order;
            }

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
                SECTION_DATA_COUNT => {
                    data_count = Some(section_p.read_u32leb()?);
                }
                _ => {
                    // Unreachable in practice -- every id that reaches this
                    // match already passed the `malformed section id` check
                    // above, which only lets Custom(0) or a
                    // `canonical_section_order`-recognized id through. Kept
                    // as a real (non-`unreachable!()`) error rather than a
                    // silent skip anyway: a future edit that adds a new
                    // section id to ONE of these two places but not the
                    // other should fail loudly on the very next malformed
                    // input it sees, not corrupt-but-not-crash.
                    return Err(WasmParseError {
                        message: format!("malformed section id: 0x{section_id:02X}"),
                        offset: section_id_offset,
                    });
                }
            }

            // Real corpus bug (`binary.wast`'s "section size mismatch"
            // cases): the declared section `size` is a promise about
            // exactly how many bytes the payload occupies. The old code
            // took that promise on faith -- it sliced `section_size` bytes
            // into `section_p` and handed it to the section-specific
            // parser, but never checked whether that parser actually
            // consumed all of them. A section that finishes parsing with
            // bytes still unread inside its own declared boundary (e.g. a
            // type section's declared size is bigger than the single
            // functype it actually contains) is just as malformed as one
            // that runs out of bytes too EARLY (which already errors
            // naturally, via `sub_parser`/`read_bytes`'s own bounds checks).
            if !section_p.is_empty() {
                return Err(WasmParseError {
                    message: format!(
                        "section size mismatch: {} unconsumed byte(s) left in section id 0x{section_id:02X}'s declared {section_size}-byte payload",
                        section_p.remaining()
                    ),
                    offset: section_p.offset(),
                });
            }
        }

        // Real corpus bug (`binary.wast`'s "function and code section have
        // inconsistent lengths" cases): the function section declares one
        // type index per function, and the code section declares one body
        // per function -- the SAME index space, so they must have the same
        // length. Same shape as the data-count-vs-data-section cross-check
        // just below (added first, for the bulk-memory proposal's own data
        // count section) -- this is the MVP-core version of that same
        // "two sections both claim to enumerate the same index space, so
        // they'd better agree" rule.
        if module.functions.len() != module.code.len() {
            return Err(WasmParseError {
                message: format!(
                    "function and code section have inconsistent lengths: {} function(s), {} code entrie(s)",
                    module.functions.len(),
                    module.code.len()
                ),
                offset: p.offset(),
            });
        }

        // A data count section, when present, must agree with the data
        // section's actual segment count -- a real, corpus-confirmed
        // `assert_malformed` case (task #84), not a hypothetical.
        if let Some(count) = data_count {
            if count as usize != module.data.len() {
                return Err(WasmParseError {
                    message: format!(
                        "data count and data section have inconsistent lengths: declared {count}, found {}",
                        module.data.len()
                    ),
                    offset: p.offset(),
                });
            }
        }

        // Real corpus bug (`binary.wast`'s "memory.init requires a data
        // count section" / "data.drop requires a data count section"
        // `assert_malformed` cases -- the W-addendum 2026-09-01 LEB128
        // prioritization pass): the spec requires a data count section
        // (§12) whenever the code section uses `memory.init`/`data.drop`
        // (`0xFC 0x08`/`0xFC 0x09`), REGARDLESS of whether the data
        // section's own segment count would otherwise agree. This crate
        // doesn't walk function-body instructions byte-by-byte (`code` is
        // stored raw, see `parse_code_section`'s own doc comment) -- and
        // deliberately does NOT start here, since a byte-pattern scan for
        // `0xFC 0x08`/`0xFC 0x09` without real instruction-boundary
        // tracking risks a false positive on some OTHER instruction's raw
        // immediate bytes (e.g. an `f64.const`'s 8 literal bytes)
        // coincidentally containing that pair. `wasm-validator`'s
        // type-checker already walks every instruction precisely (it has
        // to, to type-check them) and already has dedicated `0x08`/`0x09`
        // arms -- this flag just hands it the one piece of binary-only
        // context it has no other way to recover once section parsing is
        // done: whether §12 was present at all. See `WasmModule::
        // missing_data_count_section`'s own doc comment for why this is
        // phrased as "missing" (default `false`) rather than "has".
        module.missing_data_count_section = data_count.is_none();

        Ok(module)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_types::StorageType;

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

    // ── WasmGC struct types in the type section (LANG77 / McCarthy L3b-3a-3c) ──

    /// The `$LispyPair` cons cell: two mutable `anyref` fields. Bytes after the
    /// `0x50` sub-type tag are `0x00 0x5F 0x02  0x6E 0x01  0x6E 0x01`.
    fn lispy_pair_struct_bytes() -> Vec<u8> {
        vec![
            0x50, // sub-type open tag
            0x00, // zero supertypes
            0x5F, // struct composite-type marker
            0x02, // 2 fields
            0x6E, 0x01, // field 0: anyref, mutable
            0x6E, 0x01, // field 1: anyref, mutable
        ]
    }

    #[test]
    fn test_struct_type_section() {
        // A type section containing only the $LispyPair struct type.
        let mut payload = vec![0x01]; // count = 1
        payload.extend(lispy_pair_struct_bytes());
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();

        assert!(m.types.is_empty(), "no function types in this module");
        assert_eq!(m.struct_types.len(), 1, "the $LispyPair struct is recovered");
        let st = &m.struct_types[0];
        assert_eq!(st.fields.len(), 2);
        assert_eq!(st.fields[0].storage, StorageType::Val(ValueType::Anyref));
        assert!(st.fields[0].mutable);
        assert_eq!(st.fields[1].storage, StorageType::Val(ValueType::Anyref));
        assert!(st.fields[1].mutable);
    }

    #[test]
    fn test_func_and_struct_types_share_section_with_aligned_indices() {
        // Two function types FIRST, then the struct type — the layout the
        // encoder emits. Function type indices must remain 0 and 1 (unaffected
        // by the trailing struct type), and the struct lands in struct_types.
        let mut payload = vec![0x03]; // count = 3 (2 func + 1 struct)
        payload.extend([0x60, 0x00, 0x00]); // type 0: () -> ()
        payload.extend([0x60, 0x01, 0x7F, 0x01, 0x7E]); // type 1: (i32) -> (i64)
        payload.extend(lispy_pair_struct_bytes()); // type 2: $LispyPair
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();

        assert_eq!(m.types.len(), 2, "two function types");
        assert_eq!(m.types[0], FuncType { params: vec![], results: vec![] });
        assert_eq!(
            m.types[1],
            FuncType { params: vec![ValueType::I32], results: vec![ValueType::I64] }
        );
        assert_eq!(m.struct_types.len(), 1, "one struct type");
        assert_eq!(m.struct_types[0].fields.len(), 2);
    }

    #[test]
    fn test_struct_field_immutable_and_ref_types_roundtrip() {
        // A struct with an immutable i31ref field and a concrete structref
        // field (`0x63 <typeidx>`) — exercises the multi-byte ref decoding.
        let payload = vec![
            0x01, // count = 1
            0x50, 0x00, 0x5F, // sub-type, 0 supers, struct
            0x02, // 2 fields
            0x6C, 0x00, // field 0: i31ref, immutable
            0x63, 0x00, 0x01, // field 1: (ref null $0), mutable
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();

        let st = &m.struct_types[0];
        assert_eq!(st.fields[0].storage, StorageType::Val(ValueType::I31ref));
        assert!(!st.fields[0].mutable, "field 0 is immutable");
        assert_eq!(st.fields[1].storage, StorageType::Val(ValueType::StructRef(0)));
        assert!(st.fields[1].mutable, "field 1 is mutable");
    }

    #[test]
    fn test_struct_field_non_null_concrete_ref_roundtrips() {
        // W32 second slice: `0x64 <typeidx>` -- the NON-NULL counterpart to
        // `0x63`'s nullable `(ref null $t)` immediately above, verified
        // against the real reference interpreter's `interpreter/binary/
        // decode.ml` (see `REF_NON_NULL_CONCRETE_TAG`'s own doc comment).
        let payload = vec![
            0x01, // count = 1
            0x50, 0x00, 0x5F, // sub-type, 0 supers, struct
            0x02, // 2 fields
            0x64, 0x00, 0x01, // field 0: (ref $0), mutable
            0x64, 0x01, 0x00, // field 1: (ref $1), immutable
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();

        let st = &m.struct_types[0];
        assert_eq!(st.fields[0].storage, StorageType::Val(ValueType::NonNullStructRef(0)));
        assert!(st.fields[0].mutable);
        assert_eq!(st.fields[1].storage, StorageType::Val(ValueType::NonNullStructRef(1)));
        assert!(!st.fields[1].mutable);
    }

    #[test]
    fn test_struct_type_bad_marker_is_clean_error() {
        // 0x50 followed by a non-0x5F composite marker must be a clean Err.
        let payload = vec![
            0x01, // count = 1
            0x50, 0x00, 0x99, // sub-type, 0 supers, BOGUS marker
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        assert!(WasmModuleParser::parse(&data).is_err());
    }

    #[test]
    fn test_struct_supertypes_unsupported_is_clean_error() {
        // A sub-type that declares a supertype (count != 0) is unsupported —
        // a clean Err, not a panic or a misparse.
        let payload = vec![
            0x01, // count = 1
            0x50, 0x01, 0x00, // sub-type, ONE supertype (idx 0)
            0x5F, 0x00, // struct, 0 fields
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        assert!(WasmModuleParser::parse(&data).is_err());
    }

    #[test]
    fn test_truncated_struct_type_is_clean_error() {
        // Claims 2 fields but the bytes end after the first — must error
        // cleanly (no panic, no over-allocation).
        let payload = vec![
            0x01, // count = 1
            0x50, 0x00, 0x5F, // sub-type, 0 supers, struct
            0x02, // 2 fields...
            0x6E, 0x01, // ...but only one field's bytes are present
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        assert!(WasmModuleParser::parse(&data).is_err());
    }

    #[test]
    fn test_unknown_type_tag_is_clean_error() {
        // A type entry that is neither 0x60 nor 0x50 must be a clean Err.
        let payload = vec![0x01, 0x42]; // count = 1, bogus tag 0x42
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        assert!(WasmModuleParser::parse(&data).is_err());
    }

    // ── Test 3: Function section — type index list ────────────────────────────
    #[test]
    fn test_function_section() {
        let mut payload = vec![0x02]; // count = 2
        payload.extend(encode_u32leb(0)); // func 0 → type 0
        payload.extend(encode_u32leb(1)); // func 1 → type 1

        // The function and code sections must declare the same COUNT (see
        // `test_function_code_length_mismatch_is_rejected`) -- two trivial
        // matching bodies here.
        let body = vec![0x00, 0x0B]; // 0 local decls, end
        let mut code_payload = encode_u32leb(2); // 2 functions
        code_payload.extend(encode_u32leb(body.len() as u32));
        code_payload.extend(&body);
        code_payload.extend(encode_u32leb(body.len() as u32));
        code_payload.extend(&body);

        let data = wasm_with_sections(&[make_section(3, &payload), make_section(10, &code_payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.functions, vec![0, 1]);
    }

    // ── Section-structure hardening (LEB128/malformed-binary pass) ───────────

    #[test]
    fn test_function_code_length_mismatch_is_rejected() {
        // Function section declares 1 function; code section declares 0.
        let func_payload = encode_u32leb(1)
            .into_iter()
            .chain(encode_u32leb(0))
            .collect::<Vec<u8>>();
        let data = wasm_with_sections(&[make_section(3, &func_payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(
            err.message.contains("function and code section have inconsistent lengths"),
            "unexpected error: {}",
            err.message
        );
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

        // A function section entry is required to match: the function and
        // code sections must declare the same COUNT (see
        // `test_function_code_length_mismatch_is_rejected` below).
        let func_payload = encode_u32leb(1)
            .into_iter()
            .chain(encode_u32leb(0)) // 1 function, type index 0
            .collect::<Vec<u8>>();
        let data = wasm_with_sections(&[make_section(3, &func_payload), make_section(10, &payload)]);
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
                limits: Limits { min: 1, max: None },
                shared: false,
                is64: false,
            }
        );
    }

    // ── Test 7b: Memory section, memory64 (W25) ───────────────────────────────
    //
    // One 64-bit memory with min=1 page, no max:
    //   01        count = 1
    //   04        flags = 0x04 (64-bit index, no max)
    //   01        min = 1 (u64leb)
    #[test]
    fn test_memory_section_is64() {
        let payload = vec![0x01, 0x04, 0x01]; // count=1, flags=0x04, min=1
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.memories.len(), 1);
        assert_eq!(
            m.memories[0],
            MemoryType {
                limits: Limits { min: 1, max: None },
                shared: false,
                is64: true,
            }
        );
    }

    // ── Test 7c: Memory section, memory64 with a value past u32::MAX ──────────
    #[test]
    fn test_memory_section_is64_wide_limits() {
        let big: u64 = (u32::MAX as u64) + 42;
        let mut payload = vec![0x01, 0x05]; // count=1, flags=0x05 (64-bit, has max)
        payload.extend(wasm_leb128::encode_unsigned(big));
        payload.extend(wasm_leb128::encode_unsigned(big + 8));
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.memories.len(), 1);
        assert!(m.memories[0].is64);
        assert_eq!(m.memories[0].limits.min, big);
        assert_eq!(m.memories[0].limits.max, Some(big + 8));
    }

    // ── Test 7d: table64 (W26) round-trips instead of being rejected ─────────
    #[test]
    fn test_table_section_is64() {
        let mut payload = vec![0x01]; // count = 1
        payload.push(0x70); // funcref
        payload.push(0x04); // flags = 0x04 (64-bit index) -- table64
        payload.extend(wasm_leb128::encode_unsigned(1)); // min = 1 (u64leb)
        let data = wasm_with_sections(&[make_section(4, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.tables.len(), 1);
        assert_eq!(
            m.tables[0],
            TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: true }
        );
    }

    // ── Test 7e: table64 with a value past u32::MAX ───────────────────────────
    #[test]
    fn test_table_section_is64_wide_limits() {
        let big: u64 = (u32::MAX as u64) + 42;
        let mut payload = vec![0x01]; // count = 1
        payload.push(0x70); // funcref
        payload.push(0x05); // flags = 0x05 (64-bit index, has max)
        payload.extend(wasm_leb128::encode_unsigned(big));
        payload.extend(wasm_leb128::encode_unsigned(big + 8));
        let data = wasm_with_sections(&[make_section(4, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.tables.len(), 1);
        assert!(m.tables[0].is64);
        assert_eq!(m.tables[0].limits.min, big);
        assert_eq!(m.tables[0].limits.max, Some(big + 8));
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

    /// Security-review regression (W32 second slice follow-up): `read_expr`
    /// didn't know `ref.func` (`0xD2`) has a `funcidx` immediate, so a
    /// funcidx whose LEB128 encoding equals `0x0B` (i.e. 11) was misread as
    /// an early `end`, leaving the TRUE `end` byte unconsumed in the
    /// section stream. Demonstrated exactly this way against the pre-fix
    /// code: `(global funcref (ref.func 11))` failed with a bogus "section
    /// size mismatch" instead of parsing.
    #[test]
    fn test_global_ref_func_funcidx_immediate_is_not_misparsed_as_end() {
        let payload = vec![
            0x01, // count = 1
            0x70, // funcref
            0x00, // immutable
            0xD2, 0x0B, 0x0B, // ref.func 11; end
        ];
        let data = wasm_with_sections(&[make_section(6, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.globals.len(), 1);
        assert_eq!(m.globals[0].init_expr, vec![0xD2, 0x0B, 0x0B]);
    }

    /// Same bug class as above, for `ref.null`'s heap-type immediate
    /// (`0xD0`) in its 2-byte CONCRETE-type-index form (`0x63 <typeidx
    /// leb128>`) rather than the 1-byte abstract-heap-type form: a type
    /// index whose LEB128 byte equals `0x0B` (11) was misread as `end`
    /// one byte early, leaving the true `end` unconsumed.
    #[test]
    fn test_global_ref_null_concrete_typeidx_immediate_is_not_misparsed_as_end() {
        let payload = vec![
            0x01, // count = 1
            0x6E, // anyref (the global's OWN declared type -- single-byte,
            // independent of the concrete type index named inside its
            // init expression)
            0x00, // immutable
            0xD0, 0x63, 0x0B, 0x0B, // ref.null (struct-type index 11); end
        ];
        let data = wasm_with_sections(&[make_section(6, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.globals.len(), 1);
        assert_eq!(m.globals[0].init_expr, vec![0xD0, 0x63, 0x0B, 0x0B]);
    }

    /// Task #82 (prioritization scan after task #80, PR #11844): a global's
    /// mutability byte is a spec-mandated one-bit flag -- `0x00` or `0x01`
    /// only, NOT "any nonzero byte means mutable". Real corpus case
    /// (`global.wast`'s `assert_malformed` "malformed mutability" cases,
    /// which use `0x04`) must be rejected, not silently accepted as
    /// mutable=true.
    #[test]
    fn test_global_section_rejects_malformed_mutability_byte() {
        let payload = vec![
            0x01, // count = 1
            0x7F, // i32
            0x04, // malformed mutability (not 0 or 1)
            0x41, 0x2A, 0x0B, // i32.const 42; end
        ];
        let data = wasm_with_sections(&[make_section(6, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("malformed mutability"), "unexpected error: {}", err.message);
    }

    /// As above, for a global IMPORT's mutability byte (a separate call
    /// site, same bug).
    #[test]
    fn test_import_global_rejects_malformed_mutability_byte() {
        let mut payload = vec![0x01]; // import count = 1
        payload.extend(encode_str("m"));
        payload.extend(encode_str("g"));
        payload.push(0x03); // ExternalKind::Global
        payload.push(0x7F); // i32
        payload.push(0x04); // malformed mutability
        let data = wasm_with_sections(&[make_section(2, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("malformed mutability"), "unexpected error: {}", err.message);
    }

    /// As above, for a WasmGC struct field's mutability byte -- the same
    /// spec rule (one-bit flag, `0x00`/`0x01` only) applies here too, even
    /// though no vendored corpus case currently exercises it; fixed for
    /// consistency with the two sites above since it's the identical bug
    /// pattern.
    #[test]
    fn test_struct_field_rejects_malformed_mutability_byte() {
        let mut payload = vec![0x01]; // type count = 1
        payload.push(0x50); // sub-type open tag
        payload.push(0x00); // zero supertypes
        payload.push(0x5F); // struct composite-type marker
        payload.push(0x01); // field count = 1
        payload.push(0x7F); // i32
        payload.push(0x04); // malformed mutability
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("malformed mutability"), "unexpected error: {}", err.message);
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

    /// Task #84 (prioritization scan after task #80, PR #11844): a data
    /// count section that DOES agree with the data section's actual
    /// segment count must parse fine.
    #[test]
    fn test_data_count_section_matching_data_section_parses_fine() {
        let data_count_payload = vec![0x01]; // declares 1 segment
        let data_payload = vec![
            0x01, // count = 1
            0x00, // mem_idx = 0
            0x41, 0x00, 0x0B, // i32.const 0; end
            0x02, 0xDE, 0xAD, // 2 bytes
        ];
        let data = wasm_with_sections(&[make_section(12, &data_count_payload), make_section(11, &data_payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.data.len(), 1);
    }

    /// Real corpus case (`custom.wast`'s `assert_malformed` "data count and
    /// data section have inconsistent lengths"): a data count section
    /// silently fell into the "unknown section, skip it" arm before this
    /// fix, so its declared count (2) was never checked against the data
    /// section's real segment count (1) -- the module parsed successfully
    /// when it should have been rejected as malformed.
    #[test]
    fn test_data_count_section_mismatch_is_rejected() {
        let data_count_payload = vec![0x02]; // declares 2 segments
        let data_payload = vec![
            0x01, // count = 1 (mismatch!)
            0x00, // mem_idx = 0
            0x41, 0x00, 0x0B, // i32.const 0; end
            0x02, 0xDE, 0xAD, // 2 bytes
        ];
        let data = wasm_with_sections(&[make_section(12, &data_count_payload), make_section(11, &data_payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(
            err.message.contains("data count and data section have inconsistent lengths"),
            "unexpected error: {}",
            err.message
        );
    }

    /// W-addendum 2026-09-01 pass (`binary.wast`'s "memory.init/data.drop
    /// requires a data count section" `assert_malformed` cases): a binary
    /// module with no data count section at all must come out of this
    /// crate flagged `missing_data_count_section: true` -- the actual
    /// "memory.init/data.drop without one is malformed" enforcement lives
    /// in `wasm-validator` (this crate never walks function-body
    /// instructions), but it can only do that if this crate hands the one
    /// piece of binary-only context it needs forward instead of silently
    /// dropping it once the data-count/data-section length cross-check
    /// above is done.
    #[test]
    fn test_missing_data_count_section_flag_set_when_section_absent() {
        let data = wasm_with_sections(&[]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert!(m.missing_data_count_section);
    }

    /// The mirror case: a data count section IS present (and agrees with
    /// the data section, so it parses at all) -- the flag must be `false`.
    #[test]
    fn test_missing_data_count_section_flag_false_when_section_present() {
        let data_count_payload = vec![0x00]; // declares 0 segments
        let data = wasm_with_sections(&[make_section(12, &data_count_payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert!(!m.missing_data_count_section);
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
            0x00, // flags = 0 (active, implicit table 0)
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
        assert_eq!(m.elements[0].function_indices, vec![Some(0), Some(1)]);
    }

    #[test]
    fn element_section_func_count_does_not_preallocate_beyond_max_prealloc() {
        // A crafted func_count of u32::MAX, with no actual func indices
        // following (a truncated/adversarial stream) -- the old behavior,
        // `Vec::with_capacity(func_count)` with no cap, would have requested
        // a ~16 GiB up-front allocation from four attacker-controlled
        // bytes, before the loop ever reaches the first (failing) read. The
        // fix caps pre-allocation at MAX_PREALLOC, so this must error
        // cleanly (a missing byte on the first func-index read) rather than
        // hang or abort the process trying to allocate.
        let mut payload = vec![0x01]; // count = 1
        payload.push(0x00); // table_idx = 0
        payload.extend_from_slice(&[0x41, 0x00, 0x0B]); // offset_expr
        payload.extend_from_slice(&encode_u32leb(u32::MAX)); // func_count
        // No func indices follow -- the stream is truncated.
        let data = wasm_with_sections(&[make_section(9, &payload)]);
        let result = WasmModuleParser::parse(&data);
        assert!(result.is_err(), "a truncated stream with a huge func_count must error, not allocate/panic");
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
    fn v128_result_type_decodes_from_the_type_section() {
        // (SIMD PR1b-3 follow-up, task #75) `0x7B` was previously not in
        // `decode_value_type`'s match at all -- a `(module binary ...)`
        // whose type section declared `(result v128)` failed to decode
        // with "unknown value type byte: 0x7B", even though `wasm-types`
        // and `wasm-wast-parser`'s TEXT-format decoder already recognized
        // `v128`. Confirmed needed by the real, pinned-commit
        // `simd_const.wast`'s own `(module binary ...)` directives.
        let payload = vec![
            0x01, // 1 type
            0x60, 0x00, 0x01, 0x7B, // () -> (v128)
        ];
        let data = wasm_with_sections(&[make_section(1, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.types.len(), 1);
        assert!(m.types[0].params.is_empty());
        assert_eq!(m.types[0].results, vec![ValueType::V128]);
    }

    #[test]
    fn v128_param_and_local_type_decode_from_a_real_function_body() {
        // A function `(v128) -> (v128)` with a single declared v128 local
        // -- exercises `decode_value_type` at all three call sites that
        // feed it (type-section params/results AND a code-section local
        // declaration), not just the narrower results-only case above.
        let type_payload = vec![
            0x01, // 1 type
            0x60, 0x01, 0x7B, 0x01, 0x7B, // (v128) -> (v128)
        ];
        let func_payload = vec![0x01, 0x00]; // 1 function, type index 0
        let mut body = vec![
            0x01, // 1 local-decl run
            0x01, 0x7B, // 1 local of type v128
        ];
        body.push(0x20); // local.get
        body.push(0x00); // local index 0
        body.push(0x0B); // end
        let code_payload = {
            let mut p = vec![0x01]; // 1 function body
            p.push(body.len() as u8);
            p.extend(body);
            p
        };
        let data = wasm_with_sections(&[
            make_section(1, &type_payload),
            make_section(3, &func_payload),
            make_section(10, &code_payload),
        ]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.types[0].params, vec![ValueType::V128]);
        assert_eq!(m.types[0].results, vec![ValueType::V128]);
        assert_eq!(m.code[0].locals, vec![ValueType::V128]);
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

        // Matching function section entry -- see
        // `test_function_code_length_mismatch_is_rejected`.
        let func_payload = encode_u32leb(1)
            .into_iter()
            .chain(encode_u32leb(0))
            .collect::<Vec<u8>>();
        let data = wasm_with_sections(&[make_section(3, &func_payload), make_section(10, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(
            m.code[0].locals,
            vec![ValueType::I32, ValueType::I32, ValueType::F64]
        );
    }

    // ── LEB128 malformed-encoding classes (W?? -- binary.wast /
    //    binary-leb128.wast / binary_leb128_64.wast vendoring pass) ─────────
    //
    // These mirror the real corpus's own `assert_malformed` cases directly,
    // one per malformed-encoding CLASS, so the rule stays covered even if a
    // future corpus change ever drops one of those files' specific byte
    // sequences.

    /// Non-minimal (padded) LEB128 is legal as long as the byte count stays
    /// within budget -- NOT itself a malformed encoding. `[0x82, 0x80,
    /// 0x80, 0x80, 0x00]` is 5 bytes (the max for a `u32` field) encoding
    /// the value 2.
    #[test]
    fn u32_field_non_minimal_but_in_budget_is_accepted() {
        let payload = vec![0x01, 0x00, 0x82, 0x80, 0x80, 0x80, 0x00]; // count=1, flags=0, min=2 (5-byte LEB)
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.memories[0].limits.min, 2);
    }

    /// Class 1: overlong -- one byte more than a `u32` field's 5-byte
    /// budget, even though the extra byte pads a small, otherwise-valid
    /// value. Uses the memory section's `min` field, same shape as the real
    /// corpus's own "Unsigned LEB128 must not be overlong" cases.
    #[test]
    fn u32_field_overlong_is_rejected() {
        let payload = vec![0x01, 0x00, 0x82, 0x80, 0x80, 0x80, 0x80, 0x00]; // 6-byte LEB, one too many
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("too long"), "unexpected error: {}", err.message);
    }

    /// Class 2: out of range -- byte count is within the 5-byte budget, but
    /// the value doesn't fit in 32 bits (`2^32`, one bit too many). Before
    /// this crate's LEB128 hardening, `read_u32leb`'s `as u32` cast
    /// silently wrapped this to `0` instead of rejecting it.
    #[test]
    fn u32_field_out_of_range_is_rejected() {
        let payload = vec![0x01, 0x00, 0x80, 0x80, 0x80, 0x80, 0x10]; // count=1, flags=0, min=2^32
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected error: {}", err.message);
    }

    /// Same two classes, but for `i32.const`'s SIGNED immediate inside a
    /// global's init expr -- confirms `read_expr`'s 0x41 arm is wired
    /// through `decode_signed_bounded`, not the unsigned/unbounded decoder.
    #[test]
    fn i32_const_overlong_is_rejected() {
        let mut payload = vec![0x01, 0x7F, 0x00]; // 1 global, i32, immutable
        payload.push(0x41); // i32.const
        payload.extend([0x80, 0x80, 0x80, 0x80, 0x80, 0x00]); // 6-byte signed LEB for 0 -- one too many
        payload.push(0x0B); // end
        let data = wasm_with_sections(&[make_section(6, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("too long"), "unexpected error: {}", err.message);
    }

    #[test]
    fn i32_const_out_of_range_padding_is_rejected() {
        let mut payload = vec![0x01, 0x7F, 0x00]; // 1 global, i32, immutable
        payload.push(0x41); // i32.const
        payload.extend([0x80, 0x80, 0x80, 0x80, 0x70]); // 5 bytes, padding bits inconsistent with sign
        payload.push(0x0B); // end
        let data = wasm_with_sections(&[make_section(6, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected error: {}", err.message);
    }

    /// Class 3: truncated stream -- a section that ends mid-LEB128 (already
    /// covered structurally by `test_error_truncated_section_payload`;
    /// this variant confirms the SAME failure mode for a `u32` field
    /// specifically, not just a raw byte read).
    #[test]
    fn u32_field_truncated_mid_leb128_is_rejected() {
        let payload = vec![0x01, 0x00, 0x82, 0x80]; // count=1, flags=0, then a 2-byte partial LEB (still wants more)
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        assert!(WasmModuleParser::parse(&data).is_err());
    }

    /// A 64-bit field (memory64 limits) gets the SAME two checks, at the
    /// wider 10-byte/64-bit boundary -- confirms `read_u64leb` benefits
    /// from `wasm-leb128`'s own core fix (it calls the now-fixed
    /// `decode_unsigned` directly) without needing its own code change.
    #[test]
    fn u64_field_out_of_range_is_rejected() {
        let mut payload = vec![0x01]; // count = 1
        payload.push(0x05); // flags: is64 (0x04) + has-max (0x01)
        payload.extend([0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02]); // min = one bit past u64::MAX
        payload.extend(encode_u32leb(1)); // max (won't be reached)
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected error: {}", err.message);
    }

    // ── Section-structure malformed-binary classes ────────────────────────

    #[test]
    fn unrecognized_section_id_is_rejected() {
        let mut data = minimal_module();
        data.push(0x0E); // id 14 -- not Custom(0), not in 1..=12
        data.extend(encode_u32leb(1));
        data.push(0x00);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("malformed section id"), "unexpected error: {}", err.message);
    }

    #[test]
    fn repeated_section_is_rejected() {
        // Two Global sections back to back -- same id twice.
        let payload = vec![0x00]; // count = 0
        let data = wasm_with_sections(&[make_section(6, &payload), make_section(6, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(
            err.message.contains("unexpected content after last section"),
            "unexpected error: {}",
            err.message
        );
    }

    #[test]
    fn out_of_order_sections_are_rejected() {
        // Export(7) before Import(2) -- valid ids, wrong order.
        let export_payload = vec![0x00]; // count = 0
        let import_payload = vec![0x00]; // count = 0
        let data = wasm_with_sections(&[make_section(7, &export_payload), make_section(2, &import_payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(
            err.message.contains("unexpected content after last section"),
            "unexpected error: {}",
            err.message
        );
    }

    /// The one canonical-order case that ISN'T also numeric order: DataCount
    /// (byte id 12) must come before Code (byte id 10) despite having a
    /// numerically LARGER id -- see `canonical_section_order`'s own doc
    /// comment. A correct implementation must accept this, not reject it.
    #[test]
    fn data_count_before_code_is_accepted_despite_higher_numeric_id() {
        let data_count_payload = encode_u32leb(0); // 0 data segments
        let body = vec![0x00, 0x0B];
        let mut code_payload = encode_u32leb(1);
        code_payload.extend(encode_u32leb(body.len() as u32));
        code_payload.extend(&body);
        let func_payload = encode_u32leb(1).into_iter().chain(encode_u32leb(0)).collect::<Vec<u8>>();
        let data = wasm_with_sections(&[
            make_section(3, &func_payload),
            make_section(12, &data_count_payload), // DataCount, id=12
            make_section(10, &code_payload),        // Code, id=10 -- numerically smaller, but canonically AFTER
        ]);
        assert!(WasmModuleParser::parse(&data).is_ok());
    }

    #[test]
    fn custom_sections_may_repeat_and_appear_anywhere() {
        let custom = |name: &str| {
            let mut p = encode_str(name);
            p.extend([1, 2, 3]);
            p
        };
        let data = wasm_with_sections(&[
            make_section(0, &custom("a")),
            make_section(1, &[0x00]), // empty type section
            make_section(0, &custom("b")),
            make_section(0, &custom("c")),
        ]);
        let m = WasmModuleParser::parse(&data).unwrap();
        assert_eq!(m.customs.len(), 3);
    }

    #[test]
    fn section_size_mismatch_with_leftover_bytes_is_rejected() {
        // Type section declares size 5 but the single functype inside it
        // (`60 00 00`, 3 bytes after the count byte) only accounts for 4 of
        // those 5 bytes -- one stray trailing byte.
        let mut data = minimal_module();
        data.push(0x01); // type section
        data.extend(encode_u32leb(5)); // declared size
        data.push(0x01); // count = 1
        data.extend([0x60, 0x00, 0x00]); // () -> ()
        data.push(0xFF); // stray byte within the declared section size
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("section size mismatch"), "unexpected error: {}", err.message);
    }

    #[test]
    fn malformed_limits_flags_is_rejected() {
        let payload = vec![0x01, 0x08, 0x00]; // count=1, flags=0x08 (no defined bit), min=0
        let data = wasm_with_sections(&[make_section(5, &payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("malformed limits flags"), "unexpected error: {}", err.message);
    }

    #[test]
    fn too_many_locals_is_rejected() {
        let mut body: Vec<u8> = Vec::new();
        body.extend(encode_u32leb(1)); // 1 local decl group
        body.extend(encode_u32leb(u32::MAX)); // an absurd count
        body.push(0x7F); // i32
        body.push(0x0B); // end (never reached -- rejected before this)

        let mut code_payload = encode_u32leb(1);
        code_payload.extend(encode_u32leb(body.len() as u32));
        code_payload.extend(&body);
        let func_payload = encode_u32leb(1).into_iter().chain(encode_u32leb(0)).collect::<Vec<u8>>();
        let data = wasm_with_sections(&[make_section(3, &func_payload), make_section(10, &code_payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("too many locals"), "unexpected error: {}", err.message);
    }

    #[test]
    fn function_body_not_ending_in_end_opcode_is_rejected() {
        let body = vec![0x00, 0x41, 0x01, 0x1A]; // 0 locals, i32.const 1, drop -- NO end
        let mut code_payload = encode_u32leb(1);
        code_payload.extend(encode_u32leb(body.len() as u32));
        code_payload.extend(&body);
        let func_payload = encode_u32leb(1).into_iter().chain(encode_u32leb(0)).collect::<Vec<u8>>();
        let data = wasm_with_sections(&[make_section(3, &func_payload), make_section(10, &code_payload)]);
        let err = WasmModuleParser::parse(&data).unwrap_err();
        assert!(err.message.contains("END opcode expected"), "unexpected error: {}", err.message);
    }
}
