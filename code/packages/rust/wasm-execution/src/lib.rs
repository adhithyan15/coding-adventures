//! # wasm-execution
//!
//! WebAssembly 1.0 execution engine — interprets validated WASM modules.
//!
//! This crate provides the complete WASM instruction set implementation,
//! linear memory, tables, a bytecode decoder, control flow map builder,
//! constant expression evaluator, and the [`WasmExecutionEngine`] that
//! ties it all together.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  WasmExecutionEngine                                            │
//! │                                                                  │
//! │  ┌─────────────┐  ┌───────────────┐  ┌───────────────────────┐  │
//! │  │ GenericVM    │  │ LinearMemory  │  │ WasmExecutionContext  │  │
//! │  │ (typed stack)│  │ (byte heap)   │  │ (locals, labels, etc.)│  │
//! │  └─────────────┘  └───────────────┘  └───────────────────────┘  │
//! │                                                                  │
//! │  ┌──────────────────────────────────────────────────────────┐    │
//! │  │  ~182 instruction handlers (registered on GenericVM)     │    │
//! │  │  numeric_i32, numeric_i64, numeric_f32, numeric_f64,     │    │
//! │  │  conversion, variable, parametric, memory, control       │    │
//! │  └──────────────────────────────────────────────────────────┘    │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use wasm_execution::*;
//!
//! let engine = WasmExecutionEngine::new(WasmEngineConfig {
//!     memories: vec![],
//!     tables: vec![],
//!     globals: vec![],
//!     global_types: vec![],
//!     func_types: vec![func_type],
//!     func_bodies: vec![Some(body)],
//!     host_functions: vec![None],
//! });
//!
//! let result = engine.call_function(0, &[WasmValue::I32(5)])?;
//! // result = [WasmValue::I32(25)]
//! ```
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.

mod gc;

use std::any::Any;
use std::collections::HashMap;

use virtual_machine::{
    CodeObject, GenericVM, Instruction, Operand, TypedVMValue, VMError, VMResult, Value,
};
use wasm_leb128::{decode_signed, decode_unsigned};
use wasm_opcodes::get_opcode;
use wasm_types::{FuncType, FunctionBody, GlobalType, ValueType};

// ══════════════════════════════════════════════════════════════════════════════
// Section 1: WasmValue — Typed WASM values
// ══════════════════════════════════════════════════════════════════════════════

/// A typed WASM value: one of the four numeric types in WASM 1.0.
///
/// Unlike the GenericVM's untyped `Value`, WASM values carry their type
/// explicitly. The execution engine must maintain type safety at all times.
///
/// ```text
/// ┌────────────────────┬─────────────────────────────────────────────────┐
/// │ Variant            │ Description                                      │
/// ├────────────────────┼─────────────────────────────────────────────────┤
/// │ I32(i32)           │ 32-bit signed integer (also used for bools and   │
/// │                    │ for `i31ref` payloads — see L3b-3a-3a)           │
/// │ I64(i64)           │ 64-bit signed integer                            │
/// │ F32(f32)           │ 32-bit IEEE 754 float                            │
/// │ F64(f64)           │ 64-bit IEEE 754 float                            │
/// │ Ref(Option<u32>)   │ A WasmGC reference: `None` = null, `Some(h)` = a │
/// │                    │ handle into the engine's GC object heap          │
/// │                    │ (L3b-3a-3b — the `$LispyPair` cons cell)         │
/// └────────────────────┴─────────────────────────────────────────────────┘
/// ```
///
/// ## Why an `i31ref` is an `I32`, but a struct ref is a `Ref`
///
/// In the uniform-anyref lisp value model, *every* lisp value is a WasmGC
/// `anyref`.  Small integers are boxed as `i31ref` — but an `i31ref` is just a
/// tagged 31-bit payload with no heap identity, so we carry it as its plain
/// `I32` payload (the box/unbox ops are stack-identity no-ops, L3b-3a-3a).  A
/// **cons cell**, by contrast, is a heap object with identity and mutable
/// fields, so it needs a real reference: `Ref(Some(handle))` points at a
/// `GcStruct` in the engine's `gc_heap`.  `Ref(None)` is the null reference
/// (`ref.null`), which in the lisp model is how `nil` is represented.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmValue {
    /// 32-bit integer. Wrapping arithmetic via `i32::wrapping_*` methods.
    /// Also carries `i31ref` payloads (an `i31ref` ≡ its `i32` payload).
    I32(i32),
    /// 64-bit integer. Wrapping arithmetic via `i64::wrapping_*` methods.
    I64(i64),
    /// 32-bit IEEE 754 single-precision float.
    F32(f32),
    /// 64-bit IEEE 754 double-precision float.
    F64(f64),
    /// A WasmGC reference value. `None` is the null reference (`ref.null`);
    /// `Some(handle)` is a non-null reference into the engine's GC object heap
    /// (`gc_heap[handle]`). Used for `$LispyPair` cons cells (L3b-3a-3b).
    Ref(Option<u32>),
    /// A 128-bit SIMD lane vector (see `code/specs/
    /// W13-wasm-simd-v128-first-slice.md`). The 16 raw bytes don't fit in
    /// `TypedVMValue`'s shared 64-bit `Value` slot, so -- mirroring `Ref`'s
    /// own GC-heap-handle shape exactly -- this carries a handle into
    /// `WasmExecutionContext::v128_heap[handle]`, never the bytes directly.
    /// Handle `0` is a permanently-reserved all-zero vector (see
    /// `v128_heap`'s own doc comment), so `V128` needs no `Option` wrapper
    /// the way `Ref` does for its null case.
    V128(u32),
}

/// The real 16 bytes behind a `WasmValue::V128(handle)` RESULT, resolved
/// from `ctx.v128_heap` before that heap drops at the end of
/// `WasmExecutionEngine::call_function_with_v128` — see that method's own
/// doc comment for why a bare post-return handle can't be used directly
/// (`WasmExecutionEngine` itself has no `v128_heap` field; only that one
/// call's `ctx` ever holds it). Deliberately a separate type, not a second
/// meaning layered onto `WasmValue::V128` itself — internally (on the
/// stack, in locals, as an opcode operand) `V128` always means "a handle
/// into the CURRENTLY LIVE `ctx.v128_heap`"; conflating that with "16
/// bytes that already escaped the engine" in the same variant would make
/// every existing internal use ambiguous about which one it's holding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct V128Bytes(pub [u8; 16]);

/// The typed-stack tag we use for [`WasmValue::Ref`] when round-tripping through
/// the GenericVM's `TypedVMValue`.  `0x6E` is the WASM binary type byte for
/// `anyref`, which is exactly what a lisp reference value *is* in the
/// uniform-anyref model.  The handle (or the null sentinel) rides along in the
/// `Value::Int` payload.
const REF_TAG: u8 = 0x6E;

/// The `Value::Int` payload we use to mark a *null* reference on the typed
/// stack.  A real heap handle is a `u32` (always `>= 0` when widened to `i64`),
/// so `-1` is an unambiguous "this is `ref.null`" marker.
const REF_NULL_SENTINEL: i64 = -1;

/// The typed-stack tag for [`WasmValue::V128`] -- `0x7B`, WASM's real binary
/// type byte for `v128` (verified against the SIMD proposal's own encoding
/// table, see `code/specs/W13-wasm-simd-v128-first-slice.md`). Same shape as
/// `REF_TAG`: the handle into `v128_heap` rides the `Value::Int` payload.
const V128_TAG: u8 = 0x7B;

impl WasmValue {
    /// Convert to a [`TypedVMValue`] for the GenericVM's typed stack.
    pub fn to_typed(self) -> TypedVMValue {
        // ValueType no longer has #[repr(u8)], so we cannot write
        // `ValueType::I32 as u8`.  Use the WASM 1.0 single-byte tags directly:
        //   I32 = 0x7F, I64 = 0x7E, F32 = 0x7D, F64 = 0x7C.
        match self {
            WasmValue::I32(v) => TypedVMValue {
                value_type: 0x7F, // I32
                value: Value::Int(v as i64),
            },
            WasmValue::I64(v) => TypedVMValue {
                value_type: 0x7E, // I64
                value: Value::Int(v),
            },
            WasmValue::F32(v) => TypedVMValue {
                value_type: 0x7D, // F32
                // Reinterpret the f32's raw BITS into an f64 (zero-extend the
                // u32 pattern, then read it back as f64 bits) -- NOT an
                // arithmetic `as f64` widen. `GenericVM`'s typed stack only
                // has one float slot (`Value::Float(f64)`), shared by both
                // WASM float widths, so every f32 that's merely pushed/popped
                // (which is EVERY f32 in a running program -- locals, params,
                // results, operands) round-trips through this f64 box. Rust's
                // `as` cast between float widths does not guarantee NaN
                // payload preservation on the narrowing leg back to f32 --
                // confirmed empirically: `f32::from_bits(0x7fa00000) as f64
                // as f32` produces `0x7fc00000` (LLVM's fpext/fptrunc
                // canonicalize the payload to the target's generic quiet
                // NaN) -- so the old arithmetic cast silently destroyed the
                // exact NaN bit pattern of ANY f32 value merely passing
                // through the stack, not just ones an opcode computed on.
                // Reinterpreting the bits directly is lossless and exactly
                // reversible by `from_typed` below for every case (NaN,
                // normal, ±0.0, ±inf) since it does no rounding at all.
                value: Value::Float(f64::from_bits(v.to_bits() as u64)),
            },
            WasmValue::F64(v) => TypedVMValue {
                value_type: 0x7C, // F64
                value: Value::Float(v),
            },
            // A GC reference: tag as anyref (0x6E), carry the handle (or the
            // null sentinel) in the integer payload.
            WasmValue::Ref(handle) => TypedVMValue {
                value_type: REF_TAG,
                value: Value::Int(match handle {
                    Some(h) => h as i64,
                    None => REF_NULL_SENTINEL,
                }),
            },
            // A v128 handle: tag as v128 (0x7B), carry the v128_heap index
            // in the integer payload, exactly like `Ref` above.
            WasmValue::V128(handle) => TypedVMValue {
                value_type: V128_TAG,
                value: Value::Int(handle as i64),
            },
        }
    }

    /// Convert from a [`TypedVMValue`] back to a [`WasmValue`].
    pub fn from_typed(tv: &TypedVMValue) -> Result<Self, TrapError> {
        match tv.value_type {
            0x7F => match &tv.value { // I32
                Value::Int(v) => Ok(WasmValue::I32(*v as i32)),
                _ => Err(TrapError::new("type mismatch: expected i32")),
            },
            0x7E => match &tv.value { // I64
                Value::Int(v) => Ok(WasmValue::I64(*v)),
                _ => Err(TrapError::new("type mismatch: expected i64")),
            },
            0x7D => match &tv.value { // F32
                // Reverse of `to_typed`'s bit-reinterpret above -- truncate
                // back to the low 32 bits and read them as f32 bits, not an
                // arithmetic narrowing cast. See that match arm's doc
                // comment for why this must be a bit reinterpretation.
                Value::Float(v) => Ok(WasmValue::F32(f32::from_bits(v.to_bits() as u32))),
                _ => Err(TrapError::new("type mismatch: expected f32")),
            },
            0x7C => match &tv.value { // F64
                Value::Float(v) => Ok(WasmValue::F64(*v)),
                _ => Err(TrapError::new("type mismatch: expected f64")),
            },
            REF_TAG => match &tv.value { // anyref / GC reference
                Value::Int(v) if *v == REF_NULL_SENTINEL => Ok(WasmValue::Ref(None)),
                Value::Int(v) => Ok(WasmValue::Ref(Some(*v as u32))),
                _ => Err(TrapError::new("type mismatch: expected anyref")),
            },
            V128_TAG => match &tv.value { // v128 -- handle into v128_heap
                Value::Int(v) => Ok(WasmValue::V128(*v as u32)),
                _ => Err(TrapError::new("type mismatch: expected v128")),
            },
            other => Err(TrapError::new(format!(
                "unknown value type: 0x{:02X}",
                other
            ))),
        }
    }

    /// Create the zero/default value for a given WASM type.
    ///
    /// For the reference types (`Anyref`, `StructRef`) the default is the
    /// **null reference** `Ref(None)` — the correct WasmGC zero value for a
    /// nullable reference, and (in the lisp model) the representation of `nil`.
    /// `I31ref` is the exception: an `i31ref` is carried as its `i32` payload
    /// (L3b-3a-3a), so its zero value is `I32(0)`, not a null reference.
    pub fn default_for(vt: ValueType) -> Self {
        match vt {
            ValueType::I32 => WasmValue::I32(0),
            ValueType::I64 => WasmValue::I64(0),
            ValueType::F32 => WasmValue::F32(0.0),
            ValueType::F64 => WasmValue::F64(0.0),
            // An `i31ref` is its `i32` payload, so its zero value is I32(0).
            ValueType::I31ref => WasmValue::I32(0),
            // Nullable reference types (GC and funcref/externref alike)
            // default to the null reference.
            ValueType::Anyref
            | ValueType::StructRef(_)
            | ValueType::Funcref
            | ValueType::Externref => WasmValue::Ref(None),
            // Handle 0 is the permanently-reserved all-zero v128 (see
            // `v128_heap`'s own doc comment) -- no allocation needed here,
            // unlike a GC struct's default, since `default_for` has no
            // `WasmExecutionContext` to allocate into.
            ValueType::V128 => WasmValue::V128(0),
        }
    }

    /// Extract as i32, trapping on type mismatch.
    pub fn as_i32(&self) -> Result<i32, TrapError> {
        match self {
            WasmValue::I32(v) => Ok(*v),
            _ => Err(TrapError::new(format!(
                "type mismatch: expected i32, got {:?}",
                self
            ))),
        }
    }

    /// Extract as i64, trapping on type mismatch.
    pub fn as_i64(&self) -> Result<i64, TrapError> {
        match self {
            WasmValue::I64(v) => Ok(*v),
            _ => Err(TrapError::new(format!(
                "type mismatch: expected i64, got {:?}",
                self
            ))),
        }
    }

    /// Extract as f32, trapping on type mismatch.
    pub fn as_f32(&self) -> Result<f32, TrapError> {
        match self {
            WasmValue::F32(v) => Ok(*v),
            _ => Err(TrapError::new(format!(
                "type mismatch: expected f32, got {:?}",
                self
            ))),
        }
    }

    /// Extract as f64, trapping on type mismatch.
    pub fn as_f64(&self) -> Result<f64, TrapError> {
        match self {
            WasmValue::F64(v) => Ok(*v),
            _ => Err(TrapError::new(format!(
                "type mismatch: expected f64, got {:?}",
                self
            ))),
        }
    }

    /// Extract as a v128 heap handle, trapping on type mismatch.
    pub fn as_v128_handle(&self) -> Result<u32, TrapError> {
        match self {
            WasmValue::V128(handle) => Ok(*handle),
            _ => Err(TrapError::new(format!(
                "type mismatch: expected v128, got {:?}",
                self
            ))),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 2: TrapError — WASM runtime traps
// ══════════════════════════════════════════════════════════════════════════════

/// A WASM trap — an unrecoverable runtime error.
///
/// Traps occur when execution hits an illegal operation:
/// out-of-bounds memory access, integer division by zero, unreachable
/// instruction, type mismatch in `call_indirect`, etc.
///
/// The WASM spec defines traps as immediately halting execution with
/// no recovery. We model them as a dedicated error type so host code
/// can distinguish traps from other errors.
#[derive(Debug, Clone, PartialEq)]
pub struct TrapError {
    /// Human-readable description of what caused the trap.
    pub message: String,
}

impl TrapError {
    /// Create a new TrapError with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        TrapError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TrapError: {}", self.message)
    }
}

impl std::error::Error for TrapError {}

impl From<TrapError> for VMError {
    fn from(e: TrapError) -> Self {
        VMError::GenericError(e.message)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 3: LinearMemory — byte-addressable WASM heap
// ══════════════════════════════════════════════════════════════════════════════

/// Bytes per WASM memory page: 64 KiB.
pub const PAGE_SIZE: usize = 65536;

/// Linear memory — a contiguous, byte-addressable array of bytes.
///
/// This is WASM's heap. Memory is measured in "pages" (64 KiB each).
/// All multi-byte accesses use little-endian byte ordering.
///
/// ```text
/// ┌─────────────────────────────┬─────────────────────────────┐
/// │  Page 0 (0x0000 - 0xFFFF)  │  Page 1 (0x10000 - 0x1FFFF) │ ...
/// └─────────────────────────────┴─────────────────────────────┘
/// ```
#[derive(Clone)]
pub struct LinearMemory {
    /// The raw byte storage.
    data: Vec<u8>,
    /// Current page count.
    current_pages: u32,
    /// Maximum page count (None = no limit other than spec max 65536).
    max_pages: Option<u32>,
}

impl LinearMemory {
    /// Create a new LinearMemory with the given initial page count.
    pub fn new(initial_pages: u32, max_pages: Option<u32>) -> Self {
        let size = initial_pages as usize * PAGE_SIZE;
        LinearMemory {
            data: vec![0u8; size],
            current_pages: initial_pages,
            max_pages,
        }
    }

    /// Bounds-check: ensures `offset + width` is within the memory.
    fn bounds_check(&self, offset: usize, width: usize) -> Result<(), TrapError> {
        if offset + width > self.data.len() {
            return Err(TrapError::new(format!(
                "out of bounds memory access: offset={}, size={}, memory_size={}",
                offset,
                width,
                self.data.len()
            )));
        }
        Ok(())
    }

    // ── Full-width loads ──────────────────────────────────────────────

    /// Load a 32-bit signed integer (little-endian).
    pub fn load_i32(&self, offset: usize) -> Result<i32, TrapError> {
        self.bounds_check(offset, 4)?;
        Ok(i32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ]))
    }

    /// Load a 64-bit signed integer (little-endian).
    pub fn load_i64(&self, offset: usize) -> Result<i64, TrapError> {
        self.bounds_check(offset, 8)?;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[offset..offset + 8]);
        Ok(i64::from_le_bytes(bytes))
    }

    /// Load a 32-bit float (little-endian).
    pub fn load_f32(&self, offset: usize) -> Result<f32, TrapError> {
        self.bounds_check(offset, 4)?;
        Ok(f32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ]))
    }

    /// Load a 64-bit float (little-endian).
    pub fn load_f64(&self, offset: usize) -> Result<f64, TrapError> {
        self.bounds_check(offset, 8)?;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[offset..offset + 8]);
        Ok(f64::from_le_bytes(bytes))
    }

    // ── Narrow loads for i32 ──────────────────────────────────────────

    /// Load 1 byte, sign-extend to i32.
    pub fn load_i32_8s(&self, offset: usize) -> Result<i32, TrapError> {
        self.bounds_check(offset, 1)?;
        Ok(self.data[offset] as i8 as i32)
    }

    /// Load 1 byte, zero-extend to i32.
    pub fn load_i32_8u(&self, offset: usize) -> Result<i32, TrapError> {
        self.bounds_check(offset, 1)?;
        Ok(self.data[offset] as i32)
    }

    /// Load 2 bytes (LE), sign-extend to i32.
    pub fn load_i32_16s(&self, offset: usize) -> Result<i32, TrapError> {
        self.bounds_check(offset, 2)?;
        Ok(i16::from_le_bytes([self.data[offset], self.data[offset + 1]]) as i32)
    }

    /// Load 2 bytes (LE), zero-extend to i32.
    pub fn load_i32_16u(&self, offset: usize) -> Result<i32, TrapError> {
        self.bounds_check(offset, 2)?;
        Ok(u16::from_le_bytes([self.data[offset], self.data[offset + 1]]) as i32)
    }

    // ── Narrow loads for i64 ──────────────────────────────────────────

    /// Load 1 byte, sign-extend to i64.
    pub fn load_i64_8s(&self, offset: usize) -> Result<i64, TrapError> {
        self.bounds_check(offset, 1)?;
        Ok(self.data[offset] as i8 as i64)
    }

    /// Load 1 byte, zero-extend to i64.
    pub fn load_i64_8u(&self, offset: usize) -> Result<i64, TrapError> {
        self.bounds_check(offset, 1)?;
        Ok(self.data[offset] as i64)
    }

    /// Load 2 bytes (LE), sign-extend to i64.
    pub fn load_i64_16s(&self, offset: usize) -> Result<i64, TrapError> {
        self.bounds_check(offset, 2)?;
        Ok(i16::from_le_bytes([self.data[offset], self.data[offset + 1]]) as i64)
    }

    /// Load 2 bytes (LE), zero-extend to i64.
    pub fn load_i64_16u(&self, offset: usize) -> Result<i64, TrapError> {
        self.bounds_check(offset, 2)?;
        Ok(u16::from_le_bytes([self.data[offset], self.data[offset + 1]]) as i64)
    }

    /// Load 4 bytes (LE), sign-extend to i64.
    pub fn load_i64_32s(&self, offset: usize) -> Result<i64, TrapError> {
        self.bounds_check(offset, 4)?;
        Ok(i32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ]) as i64)
    }

    /// Load 4 bytes (LE), zero-extend to i64.
    pub fn load_i64_32u(&self, offset: usize) -> Result<i64, TrapError> {
        self.bounds_check(offset, 4)?;
        Ok(u32::from_le_bytes([
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ]) as i64)
    }

    // ── Full-width stores ─────────────────────────────────────────────

    /// Store a 32-bit integer (little-endian).
    pub fn store_i32(&mut self, offset: usize, value: i32) -> Result<(), TrapError> {
        self.bounds_check(offset, 4)?;
        let bytes = value.to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
        Ok(())
    }

    /// Store a 64-bit integer (little-endian).
    pub fn store_i64(&mut self, offset: usize, value: i64) -> Result<(), TrapError> {
        self.bounds_check(offset, 8)?;
        let bytes = value.to_le_bytes();
        self.data[offset..offset + 8].copy_from_slice(&bytes);
        Ok(())
    }

    /// Store a 32-bit float (little-endian).
    pub fn store_f32(&mut self, offset: usize, value: f32) -> Result<(), TrapError> {
        self.bounds_check(offset, 4)?;
        let bytes = value.to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
        Ok(())
    }

    /// Store a 64-bit float (little-endian).
    pub fn store_f64(&mut self, offset: usize, value: f64) -> Result<(), TrapError> {
        self.bounds_check(offset, 8)?;
        let bytes = value.to_le_bytes();
        self.data[offset..offset + 8].copy_from_slice(&bytes);
        Ok(())
    }

    // ── Narrow stores ─────────────────────────────────────────────────

    /// Store the low 8 bits of an i32.
    pub fn store_i32_8(&mut self, offset: usize, value: i32) -> Result<(), TrapError> {
        self.bounds_check(offset, 1)?;
        self.data[offset] = value as u8;
        Ok(())
    }

    /// Store the low 16 bits of an i32 (little-endian).
    pub fn store_i32_16(&mut self, offset: usize, value: i32) -> Result<(), TrapError> {
        self.bounds_check(offset, 2)?;
        let bytes = (value as i16).to_le_bytes();
        self.data[offset..offset + 2].copy_from_slice(&bytes);
        Ok(())
    }

    /// Store the low 8 bits of an i64.
    pub fn store_i64_8(&mut self, offset: usize, value: i64) -> Result<(), TrapError> {
        self.bounds_check(offset, 1)?;
        self.data[offset] = value as u8;
        Ok(())
    }

    /// Store the low 16 bits of an i64 (little-endian).
    pub fn store_i64_16(&mut self, offset: usize, value: i64) -> Result<(), TrapError> {
        self.bounds_check(offset, 2)?;
        let bytes = (value as i16).to_le_bytes();
        self.data[offset..offset + 2].copy_from_slice(&bytes);
        Ok(())
    }

    /// Store the low 32 bits of an i64 (little-endian).
    pub fn store_i64_32(&mut self, offset: usize, value: i64) -> Result<(), TrapError> {
        self.bounds_check(offset, 4)?;
        let bytes = (value as i32).to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
        Ok(())
    }

    // ── Memory management ─────────────────────────────────────────────

    /// Grow memory by `delta_pages`. Returns old page count on success, -1 on failure.
    pub fn grow(&mut self, delta_pages: u32) -> i32 {
        let old_pages = self.current_pages;
        let new_pages = old_pages as u64 + delta_pages as u64;

        if let Some(max) = self.max_pages {
            if new_pages > max as u64 {
                return -1;
            }
        }
        if new_pages > 65536 {
            return -1;
        }

        let new_size = new_pages as usize * PAGE_SIZE;
        self.data.resize(new_size, 0);
        self.current_pages = new_pages as u32;
        old_pages as i32
    }

    /// Current size in pages.
    pub fn size(&self) -> u32 {
        self.current_pages
    }

    /// Declared maximum size in pages, if any (link-time limits
    /// compatibility checking needs this in addition to `size()`).
    pub fn max_pages(&self) -> Option<u32> {
        self.max_pages
    }

    /// Write raw bytes into memory at offset.
    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<(), TrapError> {
        self.bounds_check(offset, data.len())?;
        self.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Copy `len` bytes within linear memory — the `memory.copy` bulk-memory
    /// primitive.  Both the source and destination ranges are bounds-checked
    /// (either out of range traps, never panics), and `copy_within` gives the
    /// correct overlap-safe (memmove) semantics the spec requires.
    ///
    /// A zero-length copy does NOT skip the bounds check (task #94, found
    /// vendoring `memory_copy.wast`): the real spec only allows `dest`/`src`
    /// to sit at exactly `data.len()` (one-past-the-end, the same convention
    /// as a Rust slice's exclusive upper bound) when `len` is 0 -- anything
    /// PAST that must still trap. An earlier version special-cased `len ==
    /// 0` to always return `Ok(())` before the bounds check ran at all,
    /// silently accepting a wildly out-of-range `dest`/`src` as long as
    /// nothing was actually copied. The `checked_add`/`<=` check below
    /// already handles `len == 0` correctly on its own (`x.checked_add(0)
    /// == Some(x)`, and `data[x..x]` is a valid empty slice for any `x <=
    /// data.len()`), so no separate zero-length branch is needed at all.
    pub fn copy(&mut self, dest: usize, src: usize, len: usize) -> Result<(), TrapError> {
        // Overflow-proof bounds check: compute each range end with `checked_add` so a
        // huge `src`/`dest`/`len` can't wrap `offset + width` past `data.len()` and
        // slip through (the ordinary `bounds_check` adds without checking). Defense in
        // depth — the caller already truncates operands to 32 bits — so this method is
        // safe against out-of-bounds indexing regardless of how it is invoked.
        let src_end = src.checked_add(len);
        let dest_end = dest.checked_add(len);
        match (src_end, dest_end) {
            (Some(se), Some(de)) if se <= self.data.len() && de <= self.data.len() => {
                self.data.copy_within(src..se, dest);
                Ok(())
            }
            _ => Err(TrapError::new(format!(
                "out of bounds memory.copy: dest={dest}, src={src}, len={len}, memory_size={}",
                self.data.len()
            ))),
        }
    }

    /// Copy `len` bytes from `src_mem[src..]` into `dst_mem[dest..]` --
    /// the CROSS-MEMORY `memory.copy` primitive (task #92/W18, multi-
    /// memory proposal). Takes RAW POINTERS, not `&mut LinearMemory`/
    /// `&LinearMemory` references, for the identical reason `Table::
    /// copy_between` does (task #97, `/security-review` finding): `memory.
    /// copy`'s two memory operands can name either the SAME memory or two
    /// DIFFERENT ones, and forming a `&mut`/`&` pair that both alias the
    /// same memory -- even briefly, even with no actual read/write hazard
    /// -- is undefined behavior under Rust's aliasing model regardless of
    /// what the two references do with it. Every access below goes
    /// through the raw pointers directly, inside `unsafe`, with each
    /// reference scoped to a single statement (explicit, not autoref'd,
    /// same `dangerous_implicit_autorefs`-satisfying shape `Table::
    /// copy_between` uses) so no aliased reference is ever constructed.
    ///
    /// # Safety
    /// `dst_mem`/`src_mem` must each be valid, live, properly aligned
    /// pointers to a `LinearMemory` for the whole call (satisfied by every
    /// call site, which resolves them from `ctx.memories: Vec<*mut
    /// LinearMemory>` -- pointers into a `Vec` that outlives the call).
    /// Correct even when they point at the SAME `LinearMemory` (a self-
    /// copy with potentially overlapping ranges): the source range is
    /// read into a temporary `Vec` BEFORE any destination write happens,
    /// so it never observes a partially-overwritten source -- the same
    /// overlap-safe (memmove) semantics `copy`'s own `copy_within` gives
    /// for the single-buffer case. Same zero-length-still-bounds-checked
    /// discipline `copy`/`fill` above established.
    pub unsafe fn copy_between(dst_mem: *mut LinearMemory, src_mem: *const LinearMemory, dest: usize, src: usize, len: usize) -> Result<(), TrapError> {
        let src_end = src.checked_add(len);
        let dest_end = dest.checked_add(len);
        let src_len = unsafe { (*src_mem).data.len() };
        let dst_len = unsafe { (*dst_mem).data.len() };
        match (src_end, dest_end) {
            (Some(se), Some(de)) if se <= src_len && de <= dst_len => {
                let bytes: Vec<u8> = unsafe { (&(*src_mem).data)[src..se].to_vec() };
                unsafe { (&mut (*dst_mem).data)[dest..de].copy_from_slice(&bytes) };
                Ok(())
            }
            _ => Err(TrapError::new(format!(
                "out of bounds memory.copy: dest={dest}, src={src}, len={len}, dst_memory_size={dst_len}, src_memory_size={src_len}"
            ))),
        }
    }

    /// Fill `len` bytes of linear memory starting at `dest` with `value` —
    /// the `memory.fill` bulk-memory primitive (task #94). Same overflow-
    /// proof bounds-check shape as `copy` above, including the same
    /// zero-length-still-bounds-checked fix explained in `copy`'s own doc
    /// comment (`dest` must be `<= data.len()` even when `len == 0`).
    pub fn fill(&mut self, dest: usize, value: u8, len: usize) -> Result<(), TrapError> {
        match dest.checked_add(len) {
            Some(dest_end) if dest_end <= self.data.len() => {
                self.data[dest..dest_end].fill(value);
                Ok(())
            }
            _ => Err(TrapError::new(format!(
                "out of bounds memory.fill: dest={dest}, len={len}, memory_size={}",
                self.data.len()
            ))),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 4: Table — function reference array
// ══════════════════════════════════════════════════════════════════════════════

/// A table of function references for indirect calls.
///
/// WASM 1.0 tables hold nullable function indices. The `call_indirect`
/// instruction looks up a function reference by table index, then calls it.
#[derive(Clone)]
pub struct Table {
    /// Elements: `Some(func_index)` or `None` (uninitialized).
    elements: Vec<Option<u32>>,
    /// Maximum table size, enforced by `grow` (task #98).
    max_size: Option<u32>,
}

impl Table {
    /// Create a new table with `initial_size` null entries.
    pub fn new(initial_size: u32, max_size: Option<u32>) -> Self {
        Table {
            elements: vec![None; initial_size as usize],
            max_size,
        }
    }

    /// Get the function index at the given table index.
    pub fn get(&self, index: u32) -> Result<Option<u32>, TrapError> {
        if index as usize >= self.elements.len() {
            return Err(TrapError::new(format!(
                "out of bounds table access: index={}, table size={}",
                index,
                self.elements.len()
            )));
        }
        Ok(self.elements[index as usize])
    }

    /// Set the function index at the given table index.
    pub fn set(&mut self, index: u32, func_index: Option<u32>) -> Result<(), TrapError> {
        if index as usize >= self.elements.len() {
            return Err(TrapError::new(format!(
                "out of bounds table access: index={}, table size={}",
                index,
                self.elements.len()
            )));
        }
        self.elements[index as usize] = func_index;
        Ok(())
    }

    /// Current table size.
    pub fn size(&self) -> u32 {
        self.elements.len() as u32
    }

    /// Declared maximum size, if any (link-time limits compatibility
    /// checking needs this in addition to `size()`).
    pub fn max_size(&self) -> Option<u32> {
        self.max_size
    }

    /// Grow the table by `delta` entries, filling new slots with `init`.
    /// Returns the OLD size on success, `-1` on failure -- same contract
    /// as `LinearMemory::grow` (task #98). Failure cases, checked in this
    /// order: growing past the table's own declared `max_size` (if any);
    /// growing past `MAX_TABLE_ELEMENTS` -- a HARD, implementation-defined
    /// ceiling independent of any module-declared max, the same two-tier
    /// shape `LinearMemory::grow`'s own 65536-page cap already gives
    /// memory (that check exists precisely because the *spec's* memory
    /// max is real and enforced; a table's declared `max_size` is
    /// optional, so without this second tier a module with NO declared
    /// max could `table.grow` this `Vec<Option<u32>>` (8 bytes/entry) all
    /// the way to just under `i32::MAX` entries -- ~17GB in one call, a
    /// real memory-exhaustion DoS this crate's own `MAX_TABLE_ELEMENTS`
    /// already exists to prevent for a table's DECLARED `min` (task #96,
    /// security review) but this runtime growth path never applied to
    /// growth itself until this check was added (task #98, security
    /// review round 1); or growing past what fits in `table.size`'s own
    /// `i32` result type (moot in practice now that `MAX_TABLE_ELEMENTS`
    /// is far below `i32::MAX`, kept as an explicit, self-documenting
    /// invariant rather than relying on the constant never changing).
    /// `u64` arithmetic throughout so a huge `delta` can't wrap `usize`/
    /// `u32` addition and slip past any of the three checks.
    pub fn grow(&mut self, delta: u32, init: Option<u32>) -> i32 {
        let old_size = self.elements.len() as u32;
        let new_size = old_size as u64 + delta as u64;
        if let Some(max) = self.max_size {
            if new_size > max as u64 {
                return -1;
            }
        }
        if new_size > MAX_TABLE_ELEMENTS as u64 {
            return -1;
        }
        if new_size > i32::MAX as u64 {
            return -1;
        }
        self.elements.resize(new_size as usize, init);
        old_size as i32
    }

    /// Fill `len` entries starting at `dest` with `value` -- the
    /// `table.fill` bulk-table primitive (task #98). Same overflow-proof,
    /// zero-length-still-bounds-checked discipline as `LinearMemory::
    /// fill` (established in task #94 after a real bug there): `dest`
    /// must be `<= size()` even when `len == 0`, so `checked_add` runs
    /// before any indexing rather than being skipped for a zero-length
    /// call.
    pub fn fill(&mut self, dest: u32, value: Option<u32>, len: u32) -> Result<(), TrapError> {
        let dest = dest as usize;
        let len = len as usize;
        match dest.checked_add(len) {
            Some(dest_end) if dest_end <= self.elements.len() => {
                self.elements[dest..dest_end].fill(value);
                Ok(())
            }
            _ => Err(TrapError::new(format!(
                "out of bounds table access: dest={dest}, len={len}, table size={}",
                self.elements.len()
            ))),
        }
    }

    /// Copies `len` entries from `src_table[src..]` into `dst_table[dest..]`
    /// -- the `table.copy` bulk-table primitive (task #97). Takes RAW
    /// POINTERS, not `&mut Table`/`&Table` references, because
    /// `table.copy`'s two table operands can name either the SAME table
    /// or two DIFFERENT ones: `table.copy $t $t ...` is a legal,
    /// attacker-reachable self-copy, and forming a `&mut Table` and a
    /// `&Table` that both alias the same `Table` object -- even briefly,
    /// even with no observable read/write hazard -- is undefined behavior
    /// under Rust's aliasing model regardless of what the two references
    /// actually do with it (a `/security-review` finding: an earlier
    /// version of this function took `&mut`/`&` and relied on the caller
    /// dereferencing two raw pointers into the arguments, which still
    /// aliases at the reference level the instant both parameters are
    /// live). Every access below goes through the raw pointers directly,
    /// inside `unsafe`, so no aliased reference is ever constructed, self-
    /// copy or not.
    ///
    /// # Safety
    /// `dst_table`/`src_table` must each be valid, live, properly aligned
    /// pointers to a `Table` for the whole call (satisfied by every call
    /// site, which resolves them from `ctx.tables: Vec<*mut Table>` --
    /// pointers into a `Vec` that outlives the call). Correct even when
    /// they point at the SAME `Table` (a self-copy with potentially
    /// overlapping ranges): the source range is read into a temporary
    /// `Vec` BEFORE any destination write happens, so it never observes a
    /// partially-overwritten source -- the same overlap-safe (memmove)
    /// semantics `LinearMemory::copy`'s `copy_within` gives for the
    /// single-buffer case. Same zero-length-still-bounds-checked
    /// discipline task #94 established (`dest`/`src` must be `<= size()`
    /// even when `len == 0`), checking BOTH tables' bounds before any
    /// write, so a trap leaves both tables completely untouched.
    pub unsafe fn copy_between(dst_table: *mut Table, src_table: *const Table, dest: u32, src: u32, len: u32) -> Result<(), TrapError> {
        let dest = dest as usize;
        let src = src as usize;
        let len = len as usize;
        let src_end = src.checked_add(len);
        let dest_end = dest.checked_add(len);
        // Explicit (not autoref'd) `&`/`&mut` -- each scoped to the
        // narrowest block that needs it, so the two never overlap even
        // when `dst_table`/`src_table` point at the SAME `Table` (a
        // self-copy): the shared borrow that reads `entries` ends before
        // the mutable borrow that writes it is ever formed.
        let src_len = unsafe { (*src_table).elements.len() };
        let dst_len = unsafe { (*dst_table).elements.len() };
        match (src_end, dest_end) {
            (Some(se), Some(de)) if se <= src_len && de <= dst_len => {
                let entries: Vec<Option<u32>> = unsafe { (&(*src_table).elements)[src..se].to_vec() };
                unsafe { (&mut (*dst_table).elements)[dest..de].clone_from_slice(&entries) };
                Ok(())
            }
            _ => Err(TrapError::new(format!(
                "out of bounds table.copy: dest={dest}, src={src}, len={len}, dst_table_size={dst_len}, src_table_size={src_len}"
            ))),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 5: HostFunction trait
// ══════════════════════════════════════════════════════════════════════════════

/// A host function — callable from WASM via imports.
///
/// Host functions are the bridge between the WASM sandbox and the outside
/// world. They receive typed arguments and return typed results.
pub trait HostFunction {
    /// The function's type signature.
    fn func_type(&self) -> &FuncType;
    /// Invoke the function with the given arguments.
    fn call(
        &self,
        args: &[WasmValue],
        memory: Option<&mut LinearMemory>,
    ) -> Result<Vec<WasmValue>, TrapError>;
}

/// A host interface — resolves WASM imports.
pub trait HostInterface {
    /// Resolve an imported function.
    fn resolve_function(&self, module_name: &str, name: &str) -> Option<Box<dyn HostFunction>>;

    /// Resolve an imported global.
    fn resolve_global(&self, module_name: &str, name: &str) -> Option<(GlobalType, WasmValue)>;

    /// Resolve an imported memory.
    fn resolve_memory(&self, module_name: &str, name: &str) -> Option<LinearMemory>;

    /// Resolve an imported table.
    fn resolve_table(&self, module_name: &str, name: &str) -> Option<Table>;

    /// Bind the current instance memory into the host before a call executes.
    fn set_memory(&self, _memory: LinearMemory) {}

    /// Retrieve any host-owned memory after a call completes.
    fn take_memory(&self) -> Option<LinearMemory> {
        None
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 6: Constant expression evaluator
// ══════════════════════════════════════════════════════════════════════════════

/// Evaluate a WASM constant expression (used in global initializers,
/// data segment offsets, and element segment offsets).
///
/// Allowed opcodes: i32.const (0x41), i64.const (0x42), f32.const (0x43),
/// f64.const (0x44), global.get (0x23), v128.const (0xFD 0x0C), end (0x0B).
///
/// `v128_heap` is the instance's own persistent v128 heap (see
/// `code/specs/W15-wasm-v128-persistent-storage.md`) -- a `v128.const` here
/// allocates directly into it, so the resulting handle stays valid for the
/// instance's entire lifetime, not just this one evaluation. Every other
/// arm ignores it.
pub fn evaluate_const_expr(
    expr: &[u8],
    globals: &[WasmValue],
    v128_heap: &mut Vec<[u8; 16]>,
) -> Result<WasmValue, TrapError> {
    let mut result: Option<WasmValue> = None;
    let mut pos: usize = 0;

    while pos < expr.len() {
        let opcode = expr[pos];
        pos += 1;

        match opcode {
            // i32.const
            0x41 => {
                let (value, consumed) =
                    decode_signed(expr, pos).map_err(|e| TrapError::new(e.message))?;
                pos += consumed;
                result = Some(WasmValue::I32(value as i32));
            }
            // i64.const
            0x42 => {
                let (value, consumed) = decode_signed_64(expr, pos)?;
                pos += consumed;
                result = Some(WasmValue::I64(value));
            }
            // f32.const
            0x43 => {
                if pos + 4 > expr.len() {
                    return Err(TrapError::new("f32.const: not enough bytes"));
                }
                let value =
                    f32::from_le_bytes([expr[pos], expr[pos + 1], expr[pos + 2], expr[pos + 3]]);
                pos += 4;
                result = Some(WasmValue::F32(value));
            }
            // f64.const
            0x44 => {
                if pos + 8 > expr.len() {
                    return Err(TrapError::new("f64.const: not enough bytes"));
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&expr[pos..pos + 8]);
                let value = f64::from_le_bytes(bytes);
                pos += 8;
                result = Some(WasmValue::F64(value));
            }
            // global.get
            0x23 => {
                let (idx, consumed) =
                    decode_unsigned(expr, pos).map_err(|e| TrapError::new(e.message))?;
                pos += consumed;
                if idx as usize >= globals.len() {
                    return Err(TrapError::new(format!(
                        "global.get: index {} out of bounds",
                        idx
                    )));
                }
                result = Some(globals[idx as usize]);
            }
            // v128.const (SIMD, 0xFD-prefixed) -- the sub-opcode is a
            // LEB128 u32 (verified against the real binary encoding, see
            // `decode_leb_u32`'s own call site in `decode_immediates`),
            // `0x0C` is the only SIMD sub-opcode legal in a constant
            // expression, and its own operand is 16 RAW bytes (not
            // LEB128) -- the literal lane bytes themselves. Any other
            // `0xFD`-prefixed sub-opcode here is itself illegal and falls
            // through to the catch-all below.
            0xFD => {
                let (sub_opcode, consumed) = decode_leb_u32(expr, pos);
                pos += consumed;
                if sub_opcode != 0x0C {
                    return Err(TrapError::new(format!(
                        "illegal SIMD sub-opcode 0x{:02X} in constant expression",
                        sub_opcode
                    )));
                }
                if pos + 16 > expr.len() {
                    return Err(TrapError::new("v128.const: not enough bytes"));
                }
                if v128_heap.len() >= MAX_V128_HEAP_LEN {
                    return Err(TrapError::new(
                        "v128 heap limit exceeded (too many SIMD values created)",
                    ));
                }
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&expr[pos..pos + 16]);
                pos += 16;
                let handle = v128_heap.len() as u32;
                v128_heap.push(bytes);
                result = Some(WasmValue::V128(handle));
            }
            // end
            0x0B => {
                return result.ok_or_else(|| TrapError::new("empty constant expression"));
            }
            _ => {
                return Err(TrapError::new(format!(
                    "illegal opcode 0x{:02X} in constant expression",
                    opcode
                )));
            }
        }
    }

    Err(TrapError::new("constant expression missing end opcode"))
}

/// Decode a signed 64-bit LEB128 value.
fn decode_signed_64(data: &[u8], offset: usize) -> Result<(i64, usize), TrapError> {
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    let mut bytes_consumed: usize = 0;

    loop {
        if offset + bytes_consumed >= data.len() {
            return Err(TrapError::new("unterminated LEB128 sequence"));
        }
        let byte = data[offset + bytes_consumed];
        bytes_consumed += 1;

        result |= ((byte & 0x7F) as i64) << shift;
        shift += 7;

        if (byte & 0x80) == 0 {
            // Sign extension
            if shift < 64 && (byte & 0x40) != 0 {
                result |= !0i64 << shift;
            }
            return Ok((result, bytes_consumed));
        }

        if bytes_consumed >= 10 {
            return Err(TrapError::new("LEB128 sequence too long for i64"));
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 7: Decoder — bytecodes to instructions
// ══════════════════════════════════════════════════════════════════════════════

/// A decoded WASM instruction.
#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    /// The opcode byte.
    pub opcode: u8,
    /// Decoded operand data, serialized for the GenericVM.
    pub operand: DecodedOperand,
}

/// Decoded operand data for a WASM instruction.
#[derive(Debug, Clone)]
pub enum DecodedOperand {
    /// No operand.
    None,
    /// A single integer value (label index, local index, const, etc.).
    Int(i64),
    /// A memory argument: (alignment_log2, offset, memidx). `memidx` is 0
    /// unless the multi-memory proposal's flags-bit `0x40` was set on the
    /// encoded `align` byte (task #92/W18), in which case a real memory
    /// index followed the offset in the binary.
    MemArg { _align: u32, offset: u32, memidx: u32 },
    /// A branch table: labels + default.
    BrTable {
        labels: Vec<u32>,
        default_label: u32,
    },
    /// call_indirect: (type_idx, table_idx).
    CallIndirect { type_idx: u32, table_idx: u32 },
    /// f32 constant.
    F32(f32),
    /// f64 constant.
    F64(f64),
    /// A WasmGC instruction's decoded immediates: the `0xFB` sub-opcode plus the
    /// (up to two) index immediates it carries.  Unused indices are `0`.
    ///
    /// | sub  | instruction  | type_idx | field_idx |
    /// |------|--------------|----------|-----------|
    /// | 0x00 | struct.new   | ✓        | —         |
    /// | 0x02 | struct.get   | ✓        | ✓         |
    /// | 0x04 | struct.set   | ✓        | ✓         |
    /// | 0x1C | i31.new      | —        | —         |
    /// | 0x1D | i31.get_s    | —        | —         |
    ///
    /// Carrying all three together (rather than a bare `Int(sub)`) lets the
    /// single `0xFB` handler dispatch *and* read its indices from one place.
    Gc {
        sub: u8,
        type_idx: u32,
        field_idx: u32,
    },
    /// An atomic memory operation's (WASM18) decoded immediates: the
    /// `0xFE` sub-opcode plus the memarg offset. Unlike `Gc` (which needs
    /// a side-table since two full `u32` indices plus a sub-opcode don't
    /// fit in one `usize` on every target), a `u8` sub-opcode and a
    /// `u32` offset together need at most 40 bits, well inside `usize`
    /// on every platform this repo targets -- so `convert_operand` packs
    /// them directly into one `Operand::Index` instead of spilling into
    /// a side-table the way `Gc` does.
    Atomic {
        sub: u8,
        offset: u32,
    },
    /// `v128.const`'s 16-byte literal immediate (SIMD), decoded from the
    /// bytecode's raw bytes.
    V128Const([u8; 16]),
    /// Any other `0xFD`-prefixed SIMD instruction in this first slice
    /// (`i32x4.splat`/`.add`/`.eq`/`.extract_lane`) -- the LEB128 sub-
    /// opcode, plus `aux`: the single-byte lane-index immediate for
    /// `extract_lane` (0-3), unused (`0`) for every other op here.
    Simd { sub_opcode: u32, aux: u32 },
    /// A `0xFC`-prefixed bulk-memory/bulk-table/non-trapping-conversion
    /// instruction's decoded immediates: the sub-opcode, plus `data_idx`
    /// -- a generic index slot reused across sub-opcodes for whichever
    /// index space that sub-opcode actually indexes: the real
    /// data-segment index for `memory.init`/`data.drop` (0x08/0x09, task
    /// #95); the real table index for `table.grow`/`table.size`/
    /// `table.fill` (0x0F/0x10/0x11, task #98); the real elem-segment
    /// index for `elem.drop` (0x0D, task #97) and (paired with `aux`
    /// below) `table.init` (0x0C, task #97); the destination table index
    /// for `table.copy` (0x0E, task #97, paired with `aux` as the source
    /// table index); unused (`0`) for every other 0xFC sub-opcode
    /// (trunc_sat, memory.copy, memory.fill). `aux` is a SECOND index
    /// slot, needed only by the two sub-opcodes that carry two real
    /// indices at once (`table.init`'s elem-then-table pair, `table.
    /// copy`'s dst-then-src table pair) -- `MAX_TABLES` (64) means a
    /// table index always fits in a `u8`, so this stays a single extra
    /// byte in the packed representation rather than widening `data_idx`
    /// itself. Packed uniformly the same way `Atomic`/`Simd` above pack
    /// their own sub-opcode + aux value(s), so the single `0xFC` handler
    /// can dispatch *and* read both indices from one place.
    BulkMemory { sub: u8, data_idx: u32, aux: u8 },
}

/// Decode all instructions in a function body.
pub fn decode_function_body(body: &FunctionBody) -> Vec<DecodedInstruction> {
    let code = &body.code;
    let mut instructions = Vec::new();
    let mut offset: usize = 0;

    while offset < code.len() {
        let opcode_byte = code[offset];
        offset += 1;

        // ── WasmGC two-byte opcodes: `0xFB <sub-opcode> [immediates]` ──────────
        //
        // The MVP opcode table (`get_opcode`) is single-byte and doesn't know
        // the `0xFB` GC prefix, so we decode it explicitly: read the sub-opcode
        // byte, then any index immediates it carries, and bundle them in a
        // `Gc { sub, type_idx, field_idx }` operand. The engine's `0xFB` handler
        // dispatches on `sub` and reads the indices from there.
        //
        //   sub  instruction   immediates
        //   0x00 struct.new    <type_idx>
        //   0x02 struct.get    <type_idx> <field_idx>
        //   0x04 struct.set    <type_idx> <field_idx>
        //   0x1C i31.new       (none)
        //   0x1D i31.get_s     (none)
        if opcode_byte == 0xFB {
            let sub = if offset < code.len() {
                let s = code[offset];
                offset += 1;
                s
            } else {
                0
            };
            let (type_idx, field_idx) = match sub {
                // struct.new: one index immediate (the struct type).
                0x00 => {
                    let (t, sz) = decode_leb_u32(code, offset);
                    offset += sz;
                    (t, 0)
                }
                // ref.test (0x14) / ref.test null (0x15): one heap-type immediate
                // (LANG77 L3b-3a-4). For McCarthy `pair?` the heap type is the
                // concrete `$LispyPair` struct type, whose small typeidx encodes
                // identically as a signed or unsigned LEB, so we read it as a
                // typeidx. (Abstract heap types — negative sLEB — are not used.)
                0x14 | 0x15 => {
                    let (t, sz) = decode_leb_u32(code, offset);
                    offset += sz;
                    (t, 0)
                }
                // struct.get / struct.set: two index immediates (type, field).
                0x02 | 0x04 => {
                    let (t, sz1) = decode_leb_u32(code, offset);
                    let (f, sz2) = decode_leb_u32(code, offset + sz1);
                    offset += sz1 + sz2;
                    (t, f)
                }
                // i31.new / i31.get_s (and any unknown sub-opcode): no immediates.
                _ => (0, 0),
            };
            instructions.push(DecodedInstruction {
                opcode: 0xFB,
                operand: DecodedOperand::Gc { sub, type_idx, field_idx },
            });
            continue;
        }

        // ── `0xFC`-prefixed two-byte opcodes: `0xFC <sub-opcode> [immediates]` ─
        //
        // The MVP `get_opcode` table doesn't know the `0xFC` prefix byte (used
        // by three unrelated proposals sharing the same prefix), so we decode
        // it explicitly, carrying `(sub, data_idx)` in a `BulkMemory` operand
        // (see its own doc comment); the `0xFC` handler unpacks and dispatches
        // on `sub`.
        //
        //   sub          instruction         immediates
        //   0x00-0x07    *.trunc_sat_*_s/u   (none -- WASM03)
        //   0x08         memory.init         <data_idx:u32leb> <memidx:u32leb> (task #95, #92/W18)
        //   0x09         data.drop           <data_idx:u32leb> (task #95)
        //   0x0A         memory.copy         <dst_memidx:u32leb> <src_memidx:u32leb> (task #92/W18)
        //   0x0B         memory.fill         <memidx:u32leb> (task #92/W18)
        //   0x0C         table.init          <elem_idx:u32leb> <table_idx:u32leb> (task #97)
        //   0x0D         elem.drop           <elem_idx:u32leb> (task #97)
        //   0x0E         table.copy          <dst_table_idx:u32leb> <src_table_idx:u32leb> (task #97)
        //   0x0F         table.grow          <table_idx:u32leb> (task #98)
        //   0x10         table.size          <table_idx:u32leb> (task #98)
        //   0x11         table.fill          <table_idx:u32leb> (task #98)
        //
        // `trunc_sat`'s 8 sub-opcodes need no extra immediate bytes consumed
        // here (the default `_ => (0, 0)` arm already handles that correctly).
        // The memory-index immediate(s) memory.copy/fill/init carry are
        // real LEB128 `u32`s (task #92/W18, multi-memory proposal) --
        // previously assumed to be a fixed single byte and skipped/
        // discarded entirely, which happened to work only because every
        // MVP-only (memidx-always-0) encoding is coincidentally one byte.
        // `memory.init`/`data.drop`'s `data_idx` is a data-segment index;
        // `table.grow`/`table.size`/`table.fill` reuse the same `data_idx`
        // slot to carry a real table index instead (see `BulkMemory`'s own
        // doc comment) -- this repo supports up to `MAX_TABLES` (64) real
        // tables, so a table.get/table.set-style hardcoded-to-0 shortcut
        // would be wrong. `table.init`/`table.copy`/`memory.init`/`memory.
        // copy` each carry a SECOND real index in `aux` too (`memory.fill`
        // needs only one, carried in `data_idx` like `data.drop`), reusing
        // the same repurposed-slot pattern.
        if opcode_byte == 0xFC {
            let sub = if offset < code.len() {
                let s = code[offset];
                offset += 1;
                s
            } else {
                0
            };
            let (data_idx, aux) = match sub {
                0x08 => {
                    // `memory.init` (task #92/W18 fix): the trailing
                    // memory-index immediate is a REAL LEB128 `u32`, not
                    // always a single `0x00` byte -- a prior version just
                    // skipped exactly one byte, which happened to work
                    // for every MVP-only (memidx-always-0) module but
                    // would misalign every subsequent decode the moment
                    // a real multi-memory-encoded non-zero index needed
                    // more than one LEB128 byte. `aux` carries the real
                    // memidx (same repurposed-slot pattern `table.grow`'s
                    // own doc comment above already establishes).
                    let (idx, sz1) = decode_leb_u32(code, offset);
                    let (memidx, sz2) = decode_leb_u32(code, offset + sz1);
                    offset += sz1 + sz2;
                    (idx, memidx as u8)
                }
                0x09 => {
                    let (idx, sz) = decode_leb_u32(code, offset);
                    offset += sz;
                    (idx, 0)
                }
                0x0A => {
                    // `memory.copy` (task #92/W18 fix): dst/src memidx
                    // immediates are both real LEB128 `u32`s, same fix as
                    // `memory.init` above. `data_idx` carries dst,
                    // `aux` carries src -- same shape `table.copy`'s own
                    // `(dst_table_idx, src_table_idx as u8)` uses.
                    let (dst_memidx, sz1) = decode_leb_u32(code, offset);
                    let (src_memidx, sz2) = decode_leb_u32(code, offset + sz1);
                    offset += sz1 + sz2;
                    (dst_memidx, src_memidx as u8)
                }
                0x0B => {
                    // `memory.fill` (task #92/W18 fix): same real-LEB128
                    // fix as above; `data_idx` carries the real memidx.
                    let (memidx, sz) = decode_leb_u32(code, offset);
                    offset += sz;
                    (memidx, 0)
                }
                0x0C => {
                    let (elem_idx, sz1) = decode_leb_u32(code, offset);
                    let (table_idx, sz2) = decode_leb_u32(code, offset + sz1);
                    offset += sz1 + sz2;
                    (elem_idx, table_idx as u8)
                }
                0x0D => {
                    let (elem_idx, sz) = decode_leb_u32(code, offset);
                    offset += sz;
                    (elem_idx, 0)
                }
                0x0E => {
                    let (dst_table_idx, sz1) = decode_leb_u32(code, offset);
                    let (src_table_idx, sz2) = decode_leb_u32(code, offset + sz1);
                    offset += sz1 + sz2;
                    (dst_table_idx, src_table_idx as u8)
                }
                0x0F..=0x11 => {
                    let (idx, sz) = decode_leb_u32(code, offset);
                    offset += sz;
                    (idx, 0)
                }
                _ => (0, 0),
            };
            instructions.push(DecodedInstruction {
                opcode: 0xFC,
                operand: DecodedOperand::BulkMemory { sub, data_idx, aux },
            });
            continue;
        }

        // ── `0xFE`-prefixed atomic memory operations (threads proposal,
        // WASM18): `0xFE <sub-opcode> [<align:u32leb> <offset:u32leb>]` --
        // same two-byte-prefix shape as `0xFB`/`0xFC` above.
        // `atomic.fence` (sub-opcode `0x03`) carries no memarg at all;
        // every other atomic op does, identical to the plain
        // load/store family's own memarg encoding. `align` is a
        // validation-time-only constraint (this repo's execution layer
        // never reads it, same as `DecodedOperand::MemArg`'s own unused
        // `_align` field) -- only `offset` needs to survive into the
        // decoded instruction. See `code/specs/W09-wasm-atomics-plain.md`.
        if opcode_byte == 0xFE {
            let sub = if offset < code.len() {
                let s = code[offset];
                offset += 1;
                s
            } else {
                0
            };
            let decoded_offset = if sub == 0x03 {
                0
            } else {
                let (_align, sz1) = decode_leb_u32(code, offset);
                let (mem_offset, sz2) = decode_leb_u32(code, offset + sz1);
                offset += sz1 + sz2;
                mem_offset
            };
            instructions.push(DecodedInstruction {
                opcode: 0xFE,
                operand: DecodedOperand::Atomic { sub, offset: decoded_offset },
            });
            continue;
        }

        // ── `0xFD`-prefixed SIMD (v128) instructions -- see code/specs/
        // W13-wasm-simd-v128-first-slice.md ─────────────────────────────
        //
        // Structurally DIFFERENT from `0xFB`/`0xFC`/`0xFE` above: those
        // three read their sub-opcode as a single raw byte (safe only
        // because every value they use happens to be < 128). The real
        // SIMD encoding's sub-opcode is a LEB128-encoded `u32` --
        // verified against two independent sources (the SIMD proposal's
        // own `BinarySIMD.md` and the W3C core spec), and genuinely
        // needed: `i32x4.add`'s real sub-opcode is 174 (`0xAE`), which
        // does not fit in a single LEB128 byte.
        if opcode_byte == 0xFD {
            let (sub_opcode, sz) = decode_leb_u32(code, offset);
            offset += sz;
            if sub_opcode == 0x0C {
                // v128.const: a 16-byte raw (not LEB128) immediate --
                // the literal lane bytes themselves.
                let mut bytes = [0u8; 16];
                let available = (code.len().saturating_sub(offset)).min(16);
                bytes[..available].copy_from_slice(&code[offset..offset + available]);
                offset += 16;
                instructions.push(DecodedInstruction {
                    opcode: 0xFD,
                    operand: DecodedOperand::V128Const(bytes),
                });
            } else if sub_opcode == 0x1B {
                // i32x4.extract_lane: a single raw byte lane-index
                // immediate (0-3), NOT LEB128 -- verified against the
                // SIMD proposal's own BinarySIMD.md ("These immediate
                // operands are encoded as individual bytes").
                let lane_idx = if offset < code.len() { code[offset] } else { 0 };
                offset += 1;
                instructions.push(DecodedInstruction {
                    opcode: 0xFD,
                    operand: DecodedOperand::Simd { sub_opcode, aux: lane_idx as u32 },
                });
            } else {
                // Every other SIMD op in this first slice (splat/eq/add)
                // carries no immediate beyond the sub-opcode itself.
                instructions.push(DecodedInstruction {
                    opcode: 0xFD,
                    operand: DecodedOperand::Simd { sub_opcode, aux: 0 },
                });
            }
            continue;
        }

        // ── `ref.null` (`0xD0 <heap_type>`) ────────────────────────────────────
        //
        // `ref.null` is a *single-byte* primary opcode (not `0xFB`-prefixed)
        // followed by a one-byte heap-type immediate (the encoder emits `none` =
        // `0x0F`).  We must consume that immediate explicitly, otherwise the
        // `0x0F` would be mis-decoded as a separate instruction.  The heap type
        // doesn't change runtime behaviour (every null is the same null in our
        // model), so we drop it and carry no operand.
        if opcode_byte == 0xD0 {
            if offset < code.len() {
                offset += 1; // skip the heap-type byte
            }
            instructions.push(DecodedInstruction {
                opcode: 0xD0,
                operand: DecodedOperand::None,
            });
            continue;
        }

        let info = get_opcode(opcode_byte);
        let operand = if let Some(info) = info {
            let (op, size) = decode_immediates(code, offset, info.immediates);
            offset += size;
            op
        } else {
            DecodedOperand::None
        };

        instructions.push(DecodedInstruction {
            opcode: opcode_byte,
            operand,
        });
    }

    instructions
}

/// Decode immediate operands based on the opcode's metadata.
fn decode_immediates(code: &[u8], offset: usize, immediates: &[&str]) -> (DecodedOperand, usize) {
    if immediates.is_empty() {
        return (DecodedOperand::None, 0);
    }

    // Handle multi-immediate opcodes: call_indirect (typeidx + tableidx).
    if immediates.len() == 2 && immediates[0] == "typeidx" && immediates[1] == "tableidx" {
        let (type_idx, sz1) = decode_leb_u32(code, offset);
        let (table_idx, sz2) = decode_leb_u32(code, offset + sz1);
        return (
            DecodedOperand::CallIndirect {
                type_idx,
                table_idx,
            },
            sz1 + sz2,
        );
    }

    // Single immediate
    let imm_name = immediates[0];
    match imm_name {
        "i32" => {
            let (value, consumed) = decode_signed(code, offset).unwrap_or((0, 1));
            (DecodedOperand::Int(value), consumed)
        }
        "i64" => {
            let (value, consumed) = decode_signed_64(code, offset).unwrap_or((0, 1));
            (DecodedOperand::Int(value), consumed)
        }
        "f32" => {
            if offset + 4 <= code.len() {
                let val = f32::from_le_bytes([
                    code[offset],
                    code[offset + 1],
                    code[offset + 2],
                    code[offset + 3],
                ]);
                (DecodedOperand::F32(val), 4)
            } else {
                (DecodedOperand::F32(0.0), 0)
            }
        }
        "f64" => {
            if offset + 8 <= code.len() {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&code[offset..offset + 8]);
                let val = f64::from_le_bytes(bytes);
                (DecodedOperand::F64(val), 8)
            } else {
                (DecodedOperand::F64(0.0), 0)
            }
        }
        "blocktype" => {
            // Security review (task #81's PR): `code[offset]` here was an
            // unchecked index -- unlike `f32`/`f64` above, which already
            // guard truncated input with a length check and a safe
            // default. A module reaching this decoder without first going
            // through `wasm-validator::validate()` (a real, reachable path:
            // `wasm-runtime::instantiate()`/`call()` don't call `validate()`
            // themselves -- only the separate `load_and_run()` convenience
            // wrapper does) could panic the whole process on a function
            // body truncated right after a `block`/`loop`/`if` opcode, with
            // no blocktype byte at all. `0x40` (empty) is a safe, non-
            // crashing default for the same reason `f32`/`f64` default to
            // `0.0` on truncation -- the actually-malformed body still gets
            // caught downstream (by validation, or by a real trap once
            // execution runs off the end of a too-short instruction
            // stream), just not via a raw Rust panic here.
            let byte = code.get(offset).copied().unwrap_or(0x40);
            match byte {
                // Single-byte value-type blocktypes -- carried as the RAW
                // byte (not signed-LEB128-decoded), matching `block_arity`'s
                // own raw-byte range check below. Originally just the 4 MVP
                // scalars; `0x7B` (v128, SIMD) and `0x70`/`0x6F` (funcref/
                // externref, WASM17) were a real, previously-undetected gap
                // here -- both fell through to the signed-LEB128 type-index
                // branch below instead, producing a bogus negative "type
                // index" (e.g. `0x7B` decodes to signed value -5) that a
                // real module using `(block (result v128) ...)` or
                // `(block (result funcref) ...)` would then fail structural
                // validation against (confirmed via the real, pinned-commit
                // `simd_const.wast` corpus -- see wasm-validator's matching
                // fix in `decode_blocktype`).
                0x40 | 0x7F | 0x7E | 0x7D | 0x7C | 0x7B | 0x70 | 0x6F => (DecodedOperand::Int(byte as i64), 1),
                _ => {
                    // Type index (signed LEB128)
                    let (value, consumed) = decode_signed(code, offset).unwrap_or((0, 1));
                    (DecodedOperand::Int(value), consumed)
                }
            }
        }
        "labelidx" | "funcidx" | "typeidx" | "localidx" | "globalidx" | "tableidx" | "memidx" => {
            let (value, consumed) = decode_leb_u32(code, offset);
            (DecodedOperand::Int(value as i64), consumed)
        }
        "memarg" => {
            // Multi-memory proposal (task #92/W18): bit 6 (0x40) of the
            // encoded `align` byte, previously required to be zero, is a
            // sentinel -- when set, a third LEB128 memidx immediately
            // follows the offset. Masked back out of `align` itself so
            // `_align`'s own low 6 bits (the real alignment log2) stay
            // correct either way.
            const MULTI_MEMORY_FLAG: u32 = 0x40;
            let (raw_align, sz1) = decode_leb_u32(code, offset);
            let (mem_offset, sz2) = decode_leb_u32(code, offset + sz1);
            let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
            let align = raw_align & !MULTI_MEMORY_FLAG;
            let (memidx, sz3) = if has_memidx {
                decode_leb_u32(code, offset + sz1 + sz2)
            } else {
                (0, 0)
            };
            (
                DecodedOperand::MemArg {
                    _align: align,
                    offset: mem_offset,
                    memidx,
                },
                sz1 + sz2 + sz3,
            )
        }
        "vec_labelidx" => {
            let (count, sz0) = decode_leb_u32(code, offset);
            let mut pos = offset + sz0;
            let mut labels = Vec::new();
            for _ in 0..count {
                let (label, sz) = decode_leb_u32(code, pos);
                labels.push(label);
                pos += sz;
            }
            let (default_label, sz) = decode_leb_u32(code, pos);
            pos += sz;
            (
                DecodedOperand::BrTable {
                    labels,
                    default_label,
                },
                pos - offset,
            )
        }
        _ => (DecodedOperand::None, 0),
    }
}

/// Convenience: decode an unsigned LEB128 u32.
fn decode_leb_u32(data: &[u8], offset: usize) -> (u32, usize) {
    match decode_unsigned(data, offset) {
        Ok((val, consumed)) => (val as u32, consumed),
        Err(_) => (0, 1),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 8: Control flow map
// ══════════════════════════════════════════════════════════════════════════════

/// A control flow target: where a block/loop/if ends (and optionally, where else is).
#[derive(Debug, Clone)]
pub struct ControlTarget {
    /// Instruction index of the matching `end`.
    pub end_pc: usize,
    /// Instruction index of `else`, or None.
    pub else_pc: Option<usize>,
}

/// Build the control flow map for decoded instructions.
pub fn build_control_flow_map(
    instructions: &[DecodedInstruction],
) -> HashMap<usize, ControlTarget> {
    let mut map = HashMap::new();
    let mut stack: Vec<(usize, u8, Option<usize>)> = Vec::new(); // (index, opcode, else_pc)

    for (i, instr) in instructions.iter().enumerate() {
        match instr.opcode {
            0x02..=0x04 => {
                // block, loop, if
                stack.push((i, instr.opcode, None));
            }
            0x05 => {
                // else
                if let Some(entry) = stack.last_mut() {
                    entry.2 = Some(i);
                }
            }
            0x0B => {
                // end
                if let Some((start_idx, _opcode, else_pc)) = stack.pop() {
                    map.insert(start_idx, ControlTarget { end_pc: i, else_pc });
                }
            }
            _ => {}
        }
    }

    map
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 9: Execution context
// ══════════════════════════════════════════════════════════════════════════════

/// A label on the label stack — tracks one level of structured control flow.
///
/// Carries BOTH of the blocktype's two arities (WASM06/WASM04), because a
/// branch targeting this label needs a different one depending on which
/// direction it goes:
/// - A branch to a `block`/`if`'s END (falls out) needs `arity` (its
///   RESULT values) on the stack.
/// - A branch to a `loop`'s START (re-enters it) needs `param_arity` (its
///   PARAM values, since re-entering the loop re-consumes them) on the
///   stack instead.
///
/// In WASM 1.0's MVP (single-byte blocktype only), a loop could never
/// declare params, so this distinction never mattered — `param_arity` is
/// always 0 outside a multi-value blocktype. See `execute_branch`.
#[derive(Debug, Clone)]
pub struct Label {
    /// How many result values this block produces (what a branch to a
    /// block/if's END, or a loop's own fall-through, needs on the stack).
    pub arity: usize,
    /// How many param values this block's blocktype declares (what a
    /// branch to a LOOP's START needs on the stack, re-consumed on
    /// re-entry). Always 0 for `block`/`if` labels and for any label
    /// whose blocktype isn't a multi-value type-section index.
    pub param_arity: usize,
    /// Where to jump when branching to this label.
    pub target_pc: usize,
    /// The typed stack height when this block started.
    pub stack_height: usize,
    /// Whether this is a loop label (branches backward).
    pub is_loop: bool,
}

/// A saved call frame for function calls.
#[derive(Debug, Clone)]
pub struct SavedFrame {
    pub locals: Vec<WasmValue>,
    pub label_stack: Vec<Label>,
    pub stack_height: usize,
    pub control_flow_map: HashMap<usize, ControlTarget>,
    pub return_pc: usize,
    pub return_arity: usize,
    /// The br_table targets table for the caller, saved so the callee can
    /// overwrite `ctx.br_table_targets` and restore it on return (LANG34).
    pub br_table_targets: Vec<Vec<u32>>,
    /// The caller's WasmGC operand table, saved so the callee can overwrite
    /// `ctx.gc_ops` and restore it on return (LANG77 L3b-3a-3b).
    pub gc_ops: Vec<GcOp>,
    /// The caller's `v128.const` literal table, saved so the callee can
    /// overwrite `ctx.simd_consts` and restore it on return (SIMD).
    pub simd_consts: Vec<[u8; 16]>,
}

/// A heap-allocated WasmGC struct object — the engine's representation of one
/// `struct.new` allocation.  For McCarthy Lisp this is a `$LispyPair` cons cell:
/// `type_idx` identifies the struct type, and `fields` holds the field values
/// (`fields[0]` = car, `fields[1]` = cdr).  Each field is itself an `anyref`
/// (an `I32` i31 payload, or a nested `Ref` to another cons / null).
///
/// The heap (`WasmExecutionContext::gc_heap`) is a real, collected mark-sweep
/// arena (W04) — see the [`gc`](crate::gc) module for the collector itself.
/// It is a **tombstone + free-list slot arena**, not a plain growable `Vec`:
/// a `WasmValue::Ref(Some(handle))` is a `Vec` index (a WASM-spec-mandated
/// representation), so a dead object's slot is tombstoned to `None` and its
/// index reused by a later `struct.new`, rather than the `Vec` being
/// shrunk — shrinking it would silently invalidate every other live handle
/// pointing past the removed index.
#[derive(Debug, Clone, PartialEq)]
pub struct GcStruct {
    /// The struct type index this object was allocated with (`struct.new N`).
    pub type_idx: u32,
    /// The field values, in declaration order.
    pub fields: Vec<WasmValue>,
}

/// The WASM execution context — all runtime state for WASM instructions.
pub struct WasmExecutionContext {
    /// Every memory this instance declared/imported, in index order
    /// (multi-memory proposal, W16, task #85). Index 0 is "the" default
    /// memory every pre-existing load/store/bulk-memory instruction still
    /// implicitly targets -- see `get_memory()` below.
    pub memories: Vec<*mut LinearMemory>,
    pub tables: Vec<*mut Table>,
    pub globals: Vec<WasmValue>,
    pub global_types: Vec<GlobalType>,
    pub func_types: Vec<FuncType>,
    /// The module's raw type section, indexed by type index — see
    /// [`WasmExecutionEngine::set_type_section`]'s doc comment. Empty
    /// unless the embedder set it; `call_indirect` treats empty as
    /// "no type info available", not "the type section is empty".
    pub types: Vec<FuncType>,
    pub func_bodies: Vec<Option<FunctionBody>>,
    pub host_functions: Vec<Option<Box<dyn HostFunction>>>,
    pub typed_locals: Vec<WasmValue>,
    pub label_stack: Vec<Label>,
    pub control_flow_map: HashMap<usize, ControlTarget>,
    pub saved_frames: Vec<SavedFrame>,
    pub returned: bool,
    /// Per-function br_table label arrays.  Each `br_table` instruction
    /// stores an index into this Vec as its operand; the Vec entry holds
    /// `[l0, l1, ..., l_{n-1}, default]`.  Saved/restored on every call so
    /// that the callee's table doesn't collide with the caller's (LANG34).
    pub br_table_targets: Vec<Vec<u32>>,
    /// Per-function WasmGC operand table (LANG77 L3b-3a-3b).  Each `0xFB`
    /// instruction stores an index into this Vec as its operand; the entry
    /// holds the decoded `(sub, type_idx, field_idx)`.  Like `br_table_targets`
    /// this is per-function and saved/restored across calls.
    pub gc_ops: Vec<GcOp>,
    /// Per-function `v128.const` literal table (SIMD, see `code/specs/
    /// W13-wasm-simd-v128-first-slice.md`). Each `v128.const` instruction
    /// stores an index into this Vec as its operand (the 16-byte literal
    /// doesn't fit in `Operand::Index(usize)` directly). Exactly like
    /// `br_table_targets`/`gc_ops`, this is per-function and saved/
    /// restored across nested calls -- NOT the same thing as `v128_heap`
    /// below, which holds live runtime *values*, not decoded bytecode
    /// literals.
    pub simd_consts: Vec<[u8; 16]>,
    /// The WasmGC object heap (LANG77 L3b-3a-3b) — a real, collected
    /// mark-sweep arena (W04): `Some` is a live object, `None` is a
    /// tombstoned/reclaimed slot available for reuse (see the
    /// [`gc`](crate::gc) module). A `WasmValue::Ref(Some(h))` indexes into
    /// this Vec.  **Module-global**: it persists across calls within one run
    /// (a cons built in a callee and returned stays live), so it is *not*
    /// saved/restored per call.
    pub gc_heap: Vec<Option<GcStruct>>,
    /// The v128 lane-vector heap (SIMD, see `code/specs/
    /// W13-wasm-simd-v128-first-slice.md`). A `WasmValue::V128(h)` indexes
    /// into this Vec. Unlike `gc_heap`, v128 values need no mark-sweep
    /// collection -- they're plain, immutable-once-created `Copy` 16-byte
    /// arrays, so this simply grows for the duration of one top-level
    /// `call_function` invocation (reset fresh per top-level call, exactly
    /// like `gc_heap`, but persisting across the NESTED calls within one
    /// such invocation). **Index `0` is permanently reserved as the
    /// all-zero vector** (`[0u8; 16]`), seeded once at `ctx` construction --
    /// this is the value `WasmValue::default_for(ValueType::V128)` returns,
    /// letting a `(local $x v128)` default-initialize without needing heap
    /// access from that (context-free) function.
    ///
    /// Security review (SIMD PR1a): every SIMD op that produces a new
    /// v128 (`v128.const`/`splat`/`add`/`eq`) unconditionally pushes a
    /// new entry here, with no reclamation -- this crate's own threat
    /// model treats WASM bytecode as untrusted (this exact interpreter
    /// runs adversarially-crafted modules; see its own `MAX_CALL_DEPTH`
    /// guard's doc comment), so a `loop` executing e.g. `i32x4.splat` on
    /// every iteration -- no recursion needed, so `MAX_CALL_DEPTH` never
    /// engages -- would grow this Vec without bound and exhaust memory.
    /// `MAX_V128_HEAP_LEN` below caps it, the same shape of guard
    /// `MAX_CALL_DEPTH` already provides for unbounded recursion. A
    /// fuller fix (real reclamation, mirroring `gc_heap`'s own W04
    /// mark-sweep collector) is deferred -- see `code/specs/
    /// W13-wasm-simd-v128-first-slice.md`'s follow-up scope.
    pub v128_heap: Vec<[u8; 16]>,
    /// Field counts per struct type index (LANG77 L3b-3a-3b).  `struct.new N`
    /// pops `struct_field_counts[N]` field values.  Supplied by the embedder via
    /// [`WasmExecutionEngine::set_struct_field_counts`] (the parser does not yet
    /// surface struct type definitions to the engine); empty by default, in
    /// which case any `struct.*` op is a clean trap.  Module-global / constant.
    pub struct_field_counts: Vec<u32>,
    /// GC bookkeeping (free list, live count, adaptive threshold, profile)
    /// alongside `gc_heap` — see [`gc::GcState`](crate::gc::GcState).
    /// Module-global for the same reason `gc_heap` is.
    pub gc_state: gc::GcState,
    /// Current WASM call-nesting depth — see [`MAX_CALL_DEPTH`].
    pub call_depth: usize,
    /// Set by a `return_call`/`return_call_indirect` handler (WASM16) to
    /// signal `call_function_inner`'s outer loop that the CURRENT frame
    /// should be replaced with a new function, not that a new frame
    /// should be pushed. `(func_index, args)` — args already popped from
    /// the VM stack by the handler, in the same shape
    /// `call_function_inner` would otherwise pop itself for an ordinary
    /// call. Checked once per outer-loop iteration, right after the
    /// inner instruction-dispatch loop halts; `None` in every other case
    /// (an ordinary `return`, or falling off the end of the function).
    pub pending_tail_call: Option<(usize, Vec<WasmValue>)>,
    /// The module's data segments' raw bytes, indexed by data-segment index
    /// (task #95) -- `memory.init`'s source. Populated once from the parsed
    /// module (immutable content; unlike `dropped_data_segments` below,
    /// nothing ever mutates a segment's own bytes), via
    /// [`WasmExecutionEngine::set_data_segments`], same optional-setter
    /// pattern as `struct_field_counts`. Active segments' bytes are ALSO
    /// applied directly to memory at instantiation time
    /// (`wasm-runtime::instantiate()`) -- this Vec exists so `memory.init`
    /// can copy from ANY segment (active or passive) on demand, any number
    /// of times, independent of that one-time instantiation-time copy.
    pub data_segments: Vec<Vec<u8>>,
    /// Per-data-segment "has `data.drop` already run" flag (task #95),
    /// same index space as `data_segments` above. `data.drop` sets an
    /// entry `true`; `memory.init` on a dropped segment traps ("out of
    /// bounds memory access", the spec's own wording -- deliberately NOT
    /// a distinct error, since a real WASM program can't tell the
    /// difference between "this data segment never had these bytes" and
    /// "it did, but they're gone now"). Module-global and persistent
    /// across calls (like `gc_heap`'s live-object bookkeeping, `v128_heap`
    /// -- NOT reset per call): once dropped, a segment stays dropped for
    /// the rest of the instance's lifetime.
    pub dropped_data_segments: Vec<bool>,
    /// The module's element segments' entries, indexed by elem-segment
    /// index (task #97) -- `table.init`'s source. Same shape as
    /// `data_segments` above, `Vec<Option<u32>>` per segment (`Some(idx)`
    /// for a real funcref index, `None` for a `ref.null` entry, matching
    /// `Table::elements`'s own representation). Populated once via
    /// [`WasmExecutionEngine::set_elements`]. Active segments' entries are
    /// ALSO applied directly to their table at instantiation time
    /// (`wasm-runtime::instantiate()`) -- this Vec exists so `table.init`
    /// can copy from ANY segment (active or passive) on demand, any
    /// number of times, independent of that one-time instantiation-time
    /// copy.
    pub elements: Vec<Vec<Option<u32>>>,
    /// Per-elem-segment "has `elem.drop` already run" flag (task #97),
    /// same index space as `elements` above and same semantics as
    /// `dropped_data_segments`: `elem.drop` sets an entry `true`;
    /// `table.init` on a dropped segment traps. Module-global and
    /// persistent across calls, never reset per call.
    pub dropped_elements: Vec<bool>,
}

/// `call_function`'s own call-stack recursion ceiling. WASM's `call`/
/// `call_indirect` recurse through this crate's *Rust* call stack one
/// level per nested WASM call (see `call_function`'s own doc comment) —
/// with no guard at all, a WASM program that recurses without bound (the
/// official testsuite's own `call.wast`/`call_indirect.wast`/`fac.wast`
/// deliberately test exactly this, expecting a clean "call stack
/// exhausted" trap) would overflow the REAL host thread stack: an
/// uncatchable process abort, not a WASM-level trap a caller could ever
/// observe or recover from.
///
/// A security review of the first version of this constant (200) found
/// its justification was wrong: it reasoned from a *different* crate's
/// (`wasm-wast-parser`'s) measured overflow floor on a *different*,
/// lighter-weight recursive path, rather than measuring THIS crate's own
/// (heavier — `call_function` clones and re-decodes the callee's full
/// instruction list on every call) recursion directly. 200 reliably
/// overflowed the real host stack in a **debug build** (the profile
/// `cargo test` uses by default) on any thread stack at or below ~1 MiB —
/// not a contrived scenario for a WASM interpreter specifically, which is
/// commonly embedded in worker-thread-pool contexts with a reduced stack.
///
/// Corrected the same way this repo's other recursive-descent crates
/// (e.g. `mccarthy-lisp-parser`) document their own limits: measured this
/// crate's OWN actual debug-build crash floor directly, via a real
/// recursive WASM module built through `wasm-wast-parser` and run on a
/// thread with an explicit, deliberately-small stack size
/// (`std::thread::Builder::stack_size`/`RUST_MIN_STACK`, bisected) —
/// **512 KiB** was chosen as the assumed minimum caller-provided stack
/// (well under Rust's own 2 MiB default spawned-thread stack, so any
/// caller using ordinary defaults has headroom to spare; a caller running
/// on a materially smaller stack than this is out of scope). At 512 KiB,
/// unbounded recursion overflows the real stack at depth 130 and is still
/// safe at 120 — `MAX_CALL_DEPTH` is **80**, a real ~33% margin below that
/// measured floor (this repo's other crates document a 25-45% convention
/// for the same kind of guard). Confirmed safe at 512 KiB, 768 KiB, and
/// 1 MiB stacks in a debug build.
///
/// **Known, deliberate trade-off**: this is NOT "far above any real
/// intentionally-bounded recursion" — the official testsuite's own
/// `call.wast` has two genuinely bounded (terminating) mutual-recursion
/// cases, `even(100)`/`odd(200)`, that need more than 80 levels and now
/// correctly-but-unfortunately trap "call stack exhausted" instead of
/// completing (before this fix, they only "passed" by relying on an
/// unguarded, unsafe recursion depth). A materially higher ceiling isn't
/// safe without also controlling the ACTUAL stack size WASM execution
/// runs on — see the tracked follow-up (WASM10 in this session's backlog)
/// for running `call_function` on a dedicated thread with a guaranteed
/// larger stack, which would let this constant rise well past 200 safely.
/// Shipping the conservative, safe value now and taking the small,
/// honestly-documented regression in those 2 cases was judged better than
/// leaving the unguarded host-crash risk in place while that larger,
/// separate architectural change (blocked on this crate's `*mut
/// LinearMemory`/`*mut Table` raw pointers not being `Send`) is pending.
///
/// **WASM10 update**: the blocker above is resolved (see
/// [`DEDICATED_STACK_SIZE`] and `call_function`'s own doc comment) —
/// `call_function` now always runs its decode/dispatch loop on a
/// dedicated thread with an explicit, generous stack, so this ceiling no
/// longer depends on what stack the CALLER happens to provide. Re-bisected
/// directly against that new stack size (see `code/specs/
/// W12-wasm-dedicated-thread-call-depth.md` for the full methodology) —
/// a bounded, terminating countdown-recursion WASM module, run at
/// increasing depths
/// via `call_function` (so through the real `DEDICATED_STACK_SIZE`
/// dedicated thread, not a simulation of one), each depth its own
/// subprocess (a stack overflow aborts the whole process, so bisection
/// can't share one). Measured, real, reproducible (3 repeats, identical
/// result each time) debug-build floor: **safe at depth 1820, overflows
/// at depth 1830** on an 8 MiB stack. Applying the same ~33%-margin-below-
/// the-safe-floor convention the original 80 used (`120 * 0.67 ≈ 80`):
/// `1820 * 0.67 ≈ 1214`, rounded down to a clean **1200**. This clears
/// `call.wast`'s `even(100)`/`odd(200)` cases (previously the only 2
/// `assert_return` failures in that file, now both pass) with over 10x
/// margin to spare, not just barely.
const MAX_CALL_DEPTH: usize = 1200;

/// The stack size `call_function` gives its internally-spawned dedicated
/// execution thread (WASM10). 8 MiB — 16x the 512 KiB floor the original
/// (caller-stack-dependent) `MAX_CALL_DEPTH` was bisected against — chosen
/// as a generous, round starting point, then [`MAX_CALL_DEPTH`] was
/// re-measured directly against THIS value rather than assumed via linear
/// scaling. If this ever changes, `MAX_CALL_DEPTH` must be re-bisected
/// again the same way, not just recomputed by ratio.
const DEDICATED_STACK_SIZE: usize = 8 * 1024 * 1024;

/// Caps how many WASM10 dedicated threads may be nested inside one
/// another via cross-module host calls (e.g. `wasm-conformance`'s
/// `CrossModuleFunction`, which re-enters a DIFFERENT engine's own
/// `call_function` from inside a host-function call reached through
/// `HostFunction::call`). Each nested `call_function` invocation spawns
/// its OWN dedicated thread and `DEDICATED_STACK_SIZE` stack — unlike
/// ordinary same-instance recursion (bounded by `MAX_CALL_DEPTH`/
/// `ctx.call_depth`, which resets to 0 per top-level call and therefore
/// does NOT see across this boundary at all). Security review (WASM10):
/// without a separate bound, an ordinary, non-circular chain of N linked
/// module instances calling into each other spawns N nested OS threads,
/// each reserving `DEDICATED_STACK_SIZE` — a materially larger and more
/// exhaustible resource than the old unbounded-Rust-stack-recursion
/// version of this same reentrancy pattern. `64` bounds worst-case
/// address-space use to `64 * DEDICATED_STACK_SIZE` (512 MiB at the
/// current stack size) while comfortably covering legitimate multi-
/// module linking chains.
const MAX_DEDICATED_THREAD_DEPTH: usize = 64;

/// Caps `WasmExecutionContext::v128_heap`'s growth (SIMD, security
/// review). `v128_heap` has no reclamation yet (see its own doc comment
/// for why, and the deferred-to-a-follow-up mark-sweep plan) -- without
/// SOME bound, a WASM `loop` executing e.g. `i32x4.splat` on every
/// iteration (no recursion needed, so `MAX_CALL_DEPTH` never engages)
/// would grow it without limit and exhaust memory, given this
/// interpreter's own threat model treats WASM bytecode as untrusted.
/// 1,000,000 entries (16 MiB of v128 data) is comfortably past what any
/// real `simd_*.wast` conformance case or legitimate program needs
/// (typically a few hundred SIMD operations at most), while still
/// bounding worst-case memory to a small, safe amount.
///
/// `pub` (task #86, W15 follow-up): an embedder allocating directly into
/// `WasmInstance.v128_heap` OUTSIDE this crate's own execution loop --
/// e.g. `wasm-conformance` converting a `v128.const` `invoke` ARGUMENT
/// into a real handle before a call even starts -- needs the identical
/// cap, not a separately-chosen or unbounded one.
pub const MAX_V128_HEAP_LEN: usize = 1_000_000;

/// Caps how many linear memories a single module may declare + import
/// (multi-memory proposal, W16, task #85). Real WASM has no spec-mandated
/// numeric ceiling here (unlike MVP's hardcoded 1) -- concrete limits are
/// implementation-defined. 64 is comfortably past `memory_grow.wast`'s
/// real count of 4 and any plausible legitimate module, while still
/// bounding a maliciously-crafted module's memory-section entry count
/// before any allocation runs, matching `MAX_V128_HEAP_LEN`'s own
/// reasoning above. `pub` so `wasm-validator` enforces the identical cap
/// rather than a separately-chosen one -- same cross-crate reuse pattern
/// `MAX_V128_HEAP_LEN` already established.
pub const MAX_MEMORIES: usize = 64;

/// Caps how many tables a single module may declare + import (task #96).
/// Same reasoning as `MAX_MEMORIES` -- WASM 1.0's own hardcoded "at most
/// 1" is a real historical MVP restriction, not a load-bearing invariant
/// this interpreter's execution layer actually needs: `Table` storage
/// (`WasmInstance.tables`/`WasmExecutionContext.tables`) has been a `Vec`
/// all along (unlike memory, which needed W16 to become one), and both
/// `table.get`/`table.set` (wasm-execution) and element-segment
/// application (`wasm-runtime::instantiate()`) already index by a real,
/// decoded table index rather than assuming table 0 -- so relaxing this
/// cap needs no companion storage-layer work, just this bound.
pub const MAX_TABLES: usize = 64;

/// Caps a SINGLE table's declared minimum element count (task #96,
/// security review). Unlike `MAX_TABLES`/`MAX_MEMORIES` (a count of
/// tables/memories), and unlike memory's own real spec-mandated 65536-
/// page ceiling (`grow()`'s existing check, above), WASM's real spec
/// allows a table's `min` up to `2^32 - 1` -- so this is NOT a spec
/// requirement, it's this interpreter's own implementation-defined
/// resource limit: `Table::new` allocates `min` elements EAGERLY
/// (`vec![None; initial_size]`), so an unvalidated, attacker-controlled
/// `min` near `u32::MAX` would attempt an ~34GB allocation on a single
/// `(table 4294967295 funcref)` declaration -- and combined with raising
/// `MAX_TABLES` from 1 to 64 (task #96), a small `.wat` file could now
/// request up to 64x that in one `instantiate()` call. 10,000,000
/// elements (`Table::elements: Vec<Option<u32>>`, ~80MB worst case) is
/// comfortably past any real corpus file's actual usage while still
/// bounding worst-case memory to a sane amount -- matching the same
/// "generous but bounded" reasoning as `MAX_V128_HEAP_LEN`. Enforced at
/// VALIDATION time (`wasm-validator`), not silently at allocation time,
/// so an over-cap module fails loudly and predictably rather than OOMing
/// the process.
pub const MAX_TABLE_ELEMENTS: u32 = 10_000_000;

/// Caps the SUM of every memory's page count -- declared minimum plus
/// any `memory.grow` growth -- at RUNTIME (task #101). `wasm-validator`'s
/// own "Check 1b" already caps the sum of every memory's DECLARED
/// minimum at this same bound (65536 pages = 4GB, the real single-memory
/// spec max, generalized across however many memories a module
/// declares) -- but that check only runs at declare time. Without a
/// matching runtime check, a module could declare `MAX_MEMORIES` (64)
/// memories each at `min = 0`, then `memory.grow` each independently up
/// to its own per-memory 65536-page cap, reaching ~256GB from one small
/// module -- reintroducing at runtime exactly what Check 1b closes at
/// declare-time. Reuses the SAME bound Check 1b already uses, rather
/// than a new arbitrary constant, so "total pages across every memory,
/// declared or grown, never exceeds 65536" is one consistent invariant
/// enforced at both points in a module's lifecycle.
pub const MAX_TOTAL_MEMORY_PAGES: u32 = 65536;

thread_local! {
    /// How many WASM10 dedicated threads deep the CURRENT thread is
    /// nested, relative to the original (non-WASM-spawned) caller — see
    /// `MAX_DEDICATED_THREAD_DEPTH`'s own doc comment. Deliberately a
    /// `thread_local!`, NOT a single process-global counter: thread-
    /// locals do not inherit across `std::thread::spawn`/`spawn_scoped`
    /// on their own, so each `call_function` invocation explicitly reads
    /// its OWN thread's current depth, passes `depth + 1` into the
    /// spawned closure's payload, and that closure sets its own (fresh)
    /// thread's local value at the top before doing anything else. A
    /// global counter would incorrectly conflate two unrelated,
    /// genuinely-concurrent top-level `call_function` chains (e.g. a
    /// multi-threaded host serving independent requests), tripping a
    /// false trap on one because of unrelated depth building up
    /// elsewhere — this per-thread, explicitly-propagated design avoids
    /// that.
    static DEDICATED_THREAD_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Crosses `call_function`'s internal dedicated-thread boundary (WASM10)
/// for data this crate cannot make genuinely `Send` without a breaking
/// change to public API: `WasmExecutionContext`'s raw `*mut
/// LinearMemory`/`*mut Table` pointers, and its `Box<dyn HostFunction>`
/// entries (`HostFunction`/`HostInterface` deliberately do NOT require
/// `Send` — see `code/specs/W12-wasm-dedicated-thread-call-depth.md` for
/// why: adding it would force a breaking `Rc<RefCell<..>>` ->
/// `Arc<Mutex<..>>` rewrite of `wasm-conformance`'s real cross-module
/// linking, WASM05, for no benefit to this goal).
///
/// SAFETY (must hold at every call site that constructs one): the wrapped
/// value is moved into a thread spawned via `Builder::spawn_scoped`, and
/// the spawning thread calls `.join()` on the returned handle immediately,
/// with no other work happening on the spawning thread in between. So the
/// wrapped data is accessed from exactly one thread at a time — the
/// spawned thread, for the whole duration of the call, then nothing, since
/// the spawning thread is blocked in `.join()` for the spawned thread's
/// entire lifetime. This is never true "shared across threads" access,
/// only "which one OS thread's stack this synchronous call happens to run
/// on" — the same "logically sequential, never actually concurrent"
/// shape `wasm-conformance`'s own `RefCell`-based cross-module registry
/// already relies on (different mechanism, same argument), made explicit
/// here because `unsafe impl Send` needs its own standalone justification.
struct AssertSend<T>(T);
// SAFETY: see `AssertSend`'s own doc comment above.
unsafe impl<T> Send for AssertSend<T> {}

impl<T> AssertSend<T> {
    /// Unwraps back to `T`. Deliberately a *method call*, not a `let
    /// AssertSend(inner) = x;` destructure or `.0` field access, at every
    /// call site inside a spawned closure: Rust's 2021 disjoint-capture
    /// analysis reaches straight through direct field-projection syntax,
    /// capturing `x`'s INNER (non-`Send`) fields individually rather than
    /// `x` itself — silently defeating this wrapper's whole purpose (the
    /// closure would then require `Send` on the unwrapped raw pointers
    /// directly, the exact compile error this type exists to avoid). A
    /// method call with a by-value `self` is not a field-projection path,
    /// so it forces whole-value capture of the wrapper instead.
    fn into_inner(self) -> T {
        self.0
    }
}

/// One decoded WasmGC instruction's immediates — see [`DecodedOperand::Gc`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcOp {
    /// The `0xFB` sub-opcode (e.g. `0x00` struct.new, `0x02` struct.get).
    pub sub: u8,
    /// The struct type index immediate (0 when the op carries none).
    pub type_idx: u32,
    /// The field index immediate (0 when the op carries none).
    pub field_idx: u32,
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 10: Instruction handlers
// ══════════════════════════════════════════════════════════════════════════════

// ── Helper: convert a decoded operand into a VM operand ───────────────────
//
// Most operands collapse to a single `Operand::Index`.  The two complex
// variants — `BrTable` (variable-length label vector) and `Gc` (the WasmGC
// sub-opcode + its index immediates) — don't fit in one `usize`, so they are
// spilled into per-function side-tables (`br_table_targets`, `gc_ops`) and the
// operand becomes an index into the relevant table.  Both instruction-build
// sites (top-level entry and nested callee) call this so the bookkeeping lives
// in exactly one place.
fn convert_operand(
    operand: &DecodedOperand,
    br_table_targets: &mut Vec<Vec<u32>>,
    gc_ops: &mut Vec<GcOp>,
    simd_consts: &mut Vec<[u8; 16]>,
) -> Option<Operand> {
    match operand {
        DecodedOperand::None => None,
        DecodedOperand::Int(v) => Some(Operand::Index(*v as usize)),
        // Packed exactly like `CallIndirect` above: `memidx` in the high
        // 32 bits, `offset` in the low 32 (task #92/W18). `offset` needs
        // the full low 32 bits (no memory64 support here); `MAX_MEMORIES`
        // (64) comfortably fits `memidx` in the high bits either way.
        DecodedOperand::MemArg { offset, memidx, .. } => {
            Some(Operand::Index(((*memidx as usize) << 32) | (*offset as usize)))
        }
        DecodedOperand::F32(v) => Some(Operand::Index(v.to_bits() as usize)),
        DecodedOperand::F64(v) => Some(Operand::Index(v.to_bits() as usize)),
        // Packed exactly like `Atomic`/`Simd` above: `table_idx` in the
        // high 32 bits, `type_idx` in the low 32 -- task #107. Previously
        // `table_idx` (already correctly decoded, per `decode_immediates`'
        // `["typeidx","tableidx"]` metadata) was silently dropped here,
        // the real reason `call_indirect`/`return_call_indirect` always
        // ran against table 0 regardless of what a real WASM module's
        // explicit-table-index encoding named.
        DecodedOperand::CallIndirect { type_idx, table_idx } => {
            Some(Operand::Index(((*table_idx as usize) << 32) | (*type_idx as usize)))
        }
        DecodedOperand::BrTable { labels, default_label } => {
            let idx = br_table_targets.len();
            let mut table: Vec<u32> = labels.clone();
            table.push(*default_label);
            br_table_targets.push(table);
            Some(Operand::Index(idx))
        }
        DecodedOperand::Gc { sub, type_idx, field_idx } => {
            let idx = gc_ops.len();
            gc_ops.push(GcOp {
                sub: *sub,
                type_idx: *type_idx,
                field_idx: *field_idx,
            });
            Some(Operand::Index(idx))
        }
        DecodedOperand::Atomic { sub, offset } => Some(Operand::Index(((*sub as usize) << 32) | (*offset as usize))),
        // Packed exactly like `Atomic` above: sub-opcode in the high 32
        // bits, an auxiliary index/value in the low 32 -- see
        // `unpack_simd_operand`. `v128.const`'s sub-opcode (0x0C) is
        // packed alongside the const-pool index it was just given; every
        // other SIMD op packs its sub-opcode with a `0` aux (unused).
        DecodedOperand::V128Const(bytes) => {
            let idx = simd_consts.len();
            simd_consts.push(*bytes);
            Some(Operand::Index((0x0Cusize << 32) | idx))
        }
        DecodedOperand::Simd { sub_opcode, aux } => Some(Operand::Index(((*sub_opcode as usize) << 32) | (*aux as usize))),
        DecodedOperand::BulkMemory { sub, data_idx, aux } => {
            Some(Operand::Index(((*sub as usize) << 32) | ((*aux as usize) << 40) | (*data_idx as usize)))
        }
    }
}

/// Unpack a `0xFD` SIMD instruction's `(sub_opcode, aux)` from the packed
/// `Operand::Index` `convert_operand` built above. `aux` is the
/// `simd_consts` index for `v128.const` (sub-opcode `0x0C`), unused (`0`)
/// for every other SIMD op in this first slice.
fn unpack_simd_operand(instr: &Instruction) -> (u32, usize) {
    let packed = match &instr.operand {
        Some(Operand::Index(i)) => *i,
        _ => 0,
    };
    ((packed >> 32) as u32, packed & 0xFFFF_FFFF)
}

/// Unpack a `0xFE` atomic instruction's `(sub_opcode, offset)` from the
/// packed `Operand::Index` `convert_operand` built above.
fn unpack_atomic_operand(instr: &Instruction) -> (u8, u32) {
    let packed = match &instr.operand {
        Some(Operand::Index(i)) => *i,
        _ => 0,
    };
    ((packed >> 32) as u8, (packed & 0xFFFF_FFFF) as u32)
}

/// Unpack a `0xFC` bulk-memory/conversion instruction's `(sub, data_idx,
/// aux)` from the packed `Operand::Index` `convert_operand` built above --
/// same shape as `unpack_atomic_operand`, plus the extra `aux` byte
/// `table.init`/`table.copy` need for their second index (task #97).
fn unpack_bulk_memory_operand(instr: &Instruction) -> (u8, u32, u8) {
    let packed = match &instr.operand {
        Some(Operand::Index(i)) => *i,
        _ => 0,
    };
    ((packed >> 32) as u8, (packed & 0xFFFF_FFFF) as u32, (packed >> 40) as u8)
}

/// Unpack `call_indirect`/`return_call_indirect`'s `(type_idx, table_idx)`
/// from the packed `Operand::Index` `convert_operand` built above -- same
/// shape as `unpack_atomic_operand` (task #107).
fn unpack_call_indirect_operand(instr: &Instruction) -> (u32, u32) {
    let packed = match &instr.operand {
        Some(Operand::Index(i)) => *i,
        _ => 0,
    };
    ((packed & 0xFFFF_FFFF) as u32, (packed >> 32) as u32)
}

/// Unpack a `memarg`-carrying load/store's `(offset, memidx)` from the
/// packed `Operand::Index` `convert_operand` built above -- same shape as
/// `unpack_call_indirect_operand` (task #92/W18).
fn unpack_memarg_operand(instr: &Instruction) -> (u32, u32) {
    let packed = match &instr.operand {
        Some(Operand::Index(i)) => *i,
        _ => 0,
    };
    ((packed & 0xFFFF_FFFF) as u32, (packed >> 32) as u32)
}

// ── Helper: pop a WasmValue from the VM's typed stack ─────────────────────
fn pop_wasm(vm: &mut GenericVM) -> Result<WasmValue, VMError> {
    let tv = vm.pop_typed()?;
    WasmValue::from_typed(&tv).map_err(|e| VMError::GenericError(e.message))
}

// ── Helper: push a WasmValue onto the VM's typed stack ────────────────────
fn push_wasm(vm: &mut GenericVM, val: WasmValue) {
    vm.push_typed(val.to_typed());
}

// ── Helper: peek at top WasmValue ─────────────────────────────────────────
fn peek_wasm(vm: &GenericVM) -> Result<WasmValue, VMError> {
    let tv = vm.peek_typed()?;
    WasmValue::from_typed(&tv).map_err(|e| VMError::GenericError(e.message))
}

// ── Helper: pop a *non-null* struct reference handle ──────────────────────
//
// Used by `struct.get` / `struct.set`.  A null reference is a clean trap (the
// WasmGC spec traps on a null struct access), and any non-reference value
// (e.g. a bare i31 payload) is a type mismatch — also a clean trap, never a
// panic.  `op` names the operation for the error message.
fn pop_struct_ref(vm: &mut GenericVM, op: &str) -> Result<u32, VMError> {
    match pop_wasm(vm)? {
        WasmValue::Ref(Some(h)) => Ok(h),
        WasmValue::Ref(None) => Err(VMError::GenericError(format!(
            "{op}: null reference (cannot access a field of nil)"
        ))),
        other => Err(VMError::GenericError(format!(
            "{op}: expected a struct reference, got {other:?}"
        ))),
    }
}

// ── Helper: get operand as integer ────────────────────────────────────────
fn operand_int(instr: &Instruction) -> i64 {
    match &instr.operand {
        Some(Operand::Index(i)) => *i as i64,
        _ => 0,
    }
}

// ── Helper: downcast context ──────────────────────────────────────────────
fn get_ctx(ctx: &mut dyn Any) -> &mut WasmExecutionContext {
    ctx.downcast_mut::<WasmExecutionContext>()
        .expect("context must be WasmExecutionContext")
}

// ── Helper: get the default (index 0) memory from context ─────────────────
//
// Every load/store/bulk-memory instruction except `memory.size`/
// `memory.grow` still only ever targets memory 0 (W16 scopes multi-memory
// support to exactly those two instructions -- see
// `code/specs/W16-wasm-multi-memory-first-slice.md`'s "What does NOT
// change"), so this stays the common-case helper; `get_memory_at` below is
// for the two instructions that need a real, non-default index.
fn get_memory<'a>(ctx: &WasmExecutionContext) -> Result<&'a mut LinearMemory, VMError> {
    get_memory_at(ctx, 0)
}

// ── Helper: get a specific memory from context by index ───────────────────
fn get_memory_at<'a>(ctx: &WasmExecutionContext, memidx: usize) -> Result<&'a mut LinearMemory, VMError> {
    match ctx.memories.get(memidx) {
        Some(&ptr) => Ok(unsafe { &mut *ptr }),
        None => Err(VMError::GenericError(format!(
            "memory index {} out of range ({} memories)",
            memidx,
            ctx.memories.len()
        ))),
    }
}

/// Pure aggregate-cap check for `memory.grow` (task #101): given every
/// memory's CURRENT page count, which one is being grown, and by how
/// much, returns whether the resulting CROSS-MEMORY total would exceed
/// `MAX_TOTAL_MEMORY_PAGES`. `u64` arithmetic throughout so a huge
/// `delta` (up to `u32::MAX`) can't wrap the sum and slip past the
/// check. Kept as a free function, separate from the `0x40` interpreter
/// handler, purely so it's cheaply unit-testable with small synthetic
/// page counts -- `MAX_TOTAL_MEMORY_PAGES` (65536 pages = 4GB) is far
/// too large to actually allocate in a unit test just to exercise the
/// threshold.
fn memory_grow_would_exceed_aggregate_cap(current_pages: &[u32], target_idx: usize, delta: u32) -> bool {
    let mut aggregate: u64 = 0;
    for (i, &pages) in current_pages.iter().enumerate() {
        aggregate += if i == target_idx { pages as u64 + delta as u64 } else { pages as u64 };
    }
    aggregate > MAX_TOTAL_MEMORY_PAGES as u64
}

// ── Helper: get table from context ────────────────────────────────────────
fn get_table<'a>(ctx: &mut WasmExecutionContext, idx: usize) -> Result<&'a mut Table, VMError> {
    if idx >= ctx.tables.len() {
        return Err(VMError::GenericError("undefined table".to_string()));
    }
    Ok(unsafe { &mut *ctx.tables[idx] })
}

// ── Helper: block arity resolution ────────────────────────────────────────
//
// Returns `(param_arity, result_arity)` for a `block`/`loop`/`if` header's
// blocktype (WASM06/WASM04): how many values a branch to this label's
// START needs (loop re-entry) and how many its END/fall-through needs
// (block/if exit, and a loop's own fall-through) -- see `Label`'s own doc
// comment for how the two get used differently by `execute_branch`.
//
// `types` MUST be the module's real TYPE SECTION (`ctx.types`), never
// `ctx.func_types` (indexed by FUNCTION index -- one entry per function,
// resolved to whichever type THAT function happens to declare, a
// completely different index space). `call_indirect`'s handler already
// had this exact wrong-table bug fixed once; `block_arity` had the same
// bug, just never reachable before this crate could parse a multi-value
// blocktype (a type-index blocktype) at all.
fn block_arity(block_type: i64, types: &[FuncType]) -> (usize, usize) {
    match block_type {
        0x40 => (0, 0),                        // empty
        // Single value type, no params -- the 4 MVP scalars (0x7C..=0x7F)
        // plus v128 (0x7B, SIMD) and funcref/externref (0x70/0x6F, WASM17),
        // matching the raw-byte blocktype cases `decode_function_body`'s
        // own "blocktype" operand decoder now carries for all 7 (see that
        // match's own doc comment for why this was a real, previously-
        // undetected gap for the 3 non-MVP-scalar types).
        0x7C..=0x7F | 0x7B | 0x70 | 0x6F => (0, 1),
        n if n >= 0 && (n as usize) < types.len() => {
            let t = &types[n as usize];
            (t.params.len(), t.results.len())
        }
        _ => (0, 0),
    }
}

// ── Helper: execute a branch ──────────────────────────────────────────────
fn execute_branch(
    vm: &mut GenericVM,
    ctx: &mut WasmExecutionContext,
    label_index: usize,
) -> VMResult<()> {
    let label_stack_index = ctx
        .label_stack
        .len()
        .checked_sub(1 + label_index)
        .ok_or_else(|| {
            VMError::GenericError(format!("branch target {} out of range", label_index))
        })?;

    let label = ctx.label_stack[label_stack_index].clone();

    // A branch to a LOOP's label re-enters its START, so it needs the
    // loop's declared PARAM arity preserved (re-entry re-consumes them,
    // same as the initial entry did). A branch to a block/`if` label
    // exits at its END, so it needs the block's RESULT arity instead. See
    // `Label`'s own doc comment for the full asymmetry.
    let arity = if label.is_loop {
        label.param_arity
    } else {
        label.arity
    };

    // Save result values.
    let mut results = Vec::new();
    for _ in 0..arity {
        results.push(pop_wasm(vm)?);
    }
    results.reverse();

    // Unwind stack to label height.
    while vm.typed_stack.len() > label.stack_height {
        let _ = vm.pop_typed();
    }

    // Push results back.
    for v in results {
        push_wasm(vm, v);
    }

    // Pop labels down to (and including, or not, depending on kind — see
    // below) the target. A WASM11 security-review-shaped bug: a BLOCK's
    // own `target_pc` IS the literal position of that block's own `end`
    // opcode (see the `block` handler and `build_control_flow_map`), and
    // the `end` handler unconditionally pops one label whenever it runs,
    // whether reached by falling through normally or landed on by a
    // branch. The original `ctx.label_stack.truncate(label_stack_index)`
    // (no `+ 1`) removed the target block's label a second time before
    // that same `end` byte ever ran — a genuine double-pop that silently
    // corrupted `label_stack` for any branch NOT targeting the innermost
    // currently-open label (i.e. any branch that unwinds past one or more
    // already-open outer blocks), popping ONE EXTRA label belonging to
    // whatever the *next* enclosing block happened to be. This was
    // invisible for the extremely common "the branched-into block is
    // effectively the last thing in the function" shape (the accidental
    // extra pop just triggered the function-end path a little early, with
    // no observable difference), but produced a real `StackUnderflow` (or
    // a silently wrong later branch) for anything with real code still to
    // run after the target block closes — e.g. the official testsuite's
    // own `switch.wast` dispatch pattern (10 levels of nested named
    // blocks, `br_table` jumping straight from the innermost out to a
    // middle level), where this was found. For a BLOCK, `+ 1` (keep the
    // target label in place) fixes this: landing on its own `end` byte
    // then pops it EXACTLY ONCE, identical to ordinary fall-through.
    //
    // A LOOP needs the OPPOSITE fix, not `+ 1` too — its `target_pc` is
    // the position of the `loop` OPCODE ITSELF (`loop_pc = vm.pc`,
    // captured BEFORE `vm.advance_pc()` in the `loop` handler), not an
    // `end` byte. Branching back to a loop re-executes that `loop` opcode,
    // which unconditionally PUSHES A FRESH LABEL. Keeping the old label
    // in place (`+ 1`) here as well — the first version of this fix,
    // caught by manually reproducing a simple `loop`+`br_if`-break before
    // pushing, not by the testsuite (no vendored `.wast` file with a
    // simple bounded loop currently parses) — left BOTH the retained old
    // label and the freshly re-pushed one on the stack every iteration:
    // an unbounded per-iteration duplicate, corrupting every later depth
    // calculation and hanging (an effectively infinite loop, not a clean
    // trap) rather than terminating. Loops keep the ORIGINAL (no `+ 1`)
    // behavior instead: remove the old instance now, so the re-push on
    // the next iteration nets back to exactly one.
    let keep_target = !label.is_loop;
    ctx.label_stack.truncate(label_stack_index + usize::from(keep_target));

    // Jump.
    vm.jump_to(label.target_pc);

    // GC checkpoint (W04): every taken branch is a candidate loop back-edge
    // (a loop iterates by branching to its own loop label), so this is the
    // natural "safepoint at back-edges" chokepoint — see gc::maybe_collect.
    gc::maybe_collect(vm, ctx);

    Ok(())
}

/// Register all WASM instruction handlers on a GenericVM.
pub fn register_all_handlers(vm: &mut GenericVM) {
    register_numeric_i32(vm);
    register_numeric_i64(vm);
    register_numeric_f32(vm);
    register_numeric_f64(vm);
    register_conversion(vm);
    register_variable(vm);
    register_parametric(vm);
    register_memory(vm);
    register_atomics(vm);
    register_simd(vm);
    register_control(vm);
}

// ── Numeric i32 (0x41, 0x45-0x4F, 0x67-0x78) ────────────────────────────

fn register_numeric_i32(vm: &mut GenericVM) {
    // i32.const (0x41)
    vm.register_context_opcode(0x41, |vm, instr, _code, _ctx| {
        let val = operand_int(instr) as i32;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });

    // i32.eqz (0x45)
    vm.register_context_opcode(0x45, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(if a == 0 { 1 } else { 0 }));
        vm.advance_pc();
        Ok(None)
    });

    // i32 comparison/arithmetic: macro for binary ops
    macro_rules! i32_binop {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let b = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let result = $op(a, b)?;
                push_wasm(vm, WasmValue::I32(result));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    i32_binop!(vm, 0x46, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if a == b { 1 } else { 0 })
    }); // eq
    i32_binop!(vm, 0x47, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if a != b { 1 } else { 0 })
    }); // ne
    i32_binop!(vm, 0x48, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if a < b { 1 } else { 0 })
    }); // lt_s
    i32_binop!(vm, 0x49, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if (a as u32) < (b as u32) { 1 } else { 0 })
    }); // lt_u
    i32_binop!(vm, 0x4A, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if a > b { 1 } else { 0 })
    }); // gt_s
    i32_binop!(vm, 0x4B, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if (a as u32) > (b as u32) { 1 } else { 0 })
    }); // gt_u
    i32_binop!(vm, 0x4C, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if a <= b { 1 } else { 0 })
    }); // le_s
    i32_binop!(vm, 0x4D, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if (a as u32) <= (b as u32) { 1 } else { 0 })
    }); // le_u
    i32_binop!(vm, 0x4E, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if a >= b { 1 } else { 0 })
    }); // ge_s
    i32_binop!(vm, 0x4F, |a: i32, b: i32| -> VMResult<i32> {
        Ok(if (a as u32) >= (b as u32) { 1 } else { 0 })
    }); // ge_u

    // i32 unary ops
    macro_rules! i32_unop {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32($op(a)));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    i32_unop!(vm, 0x67, |a: i32| a.leading_zeros() as i32); // clz
    i32_unop!(vm, 0x68, |a: i32| a.trailing_zeros() as i32); // ctz
    i32_unop!(vm, 0x69, |a: i32| a.count_ones() as i32); // popcnt

    // Arithmetic
    i32_binop!(vm, 0x6A, |a: i32, b: i32| -> VMResult<i32> {
        Ok(a.wrapping_add(b))
    }); // add
    i32_binop!(vm, 0x6B, |a: i32, b: i32| -> VMResult<i32> {
        Ok(a.wrapping_sub(b))
    }); // sub
    i32_binop!(vm, 0x6C, |a: i32, b: i32| -> VMResult<i32> {
        Ok(a.wrapping_mul(b))
    }); // mul

    // div_s (0x6D): traps on div by zero or overflow
    i32_binop!(vm, 0x6D, |a: i32, b: i32| -> VMResult<i32> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        if a == i32::MIN && b == -1 {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        Ok(a.wrapping_div(b))
    });

    // div_u (0x6E)
    i32_binop!(vm, 0x6E, |a: i32, b: i32| -> VMResult<i32> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        Ok(((a as u32).wrapping_div(b as u32)) as i32)
    });

    // rem_s (0x6F)
    i32_binop!(vm, 0x6F, |a: i32, b: i32| -> VMResult<i32> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        if a == i32::MIN && b == -1 {
            return Ok(0);
        }
        Ok(a.wrapping_rem(b))
    });

    // rem_u (0x70)
    i32_binop!(vm, 0x70, |a: i32, b: i32| -> VMResult<i32> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        Ok(((a as u32).wrapping_rem(b as u32)) as i32)
    });

    // Bitwise
    i32_binop!(vm, 0x71, |a: i32, b: i32| -> VMResult<i32> { Ok(a & b) }); // and
    i32_binop!(vm, 0x72, |a: i32, b: i32| -> VMResult<i32> { Ok(a | b) }); // or
    i32_binop!(vm, 0x73, |a: i32, b: i32| -> VMResult<i32> { Ok(a ^ b) }); // xor
    i32_binop!(vm, 0x74, |a: i32, b: i32| -> VMResult<i32> {
        Ok(a.wrapping_shl(b as u32 & 31))
    }); // shl
    i32_binop!(vm, 0x75, |a: i32, b: i32| -> VMResult<i32> {
        Ok(a.wrapping_shr(b as u32 & 31))
    }); // shr_s
    i32_binop!(vm, 0x76, |a: i32, b: i32| -> VMResult<i32> {
        Ok(((a as u32).wrapping_shr(b as u32 & 31)) as i32)
    }); // shr_u
    i32_binop!(vm, 0x77, |a: i32, b: i32| -> VMResult<i32> {
        Ok(a.rotate_left(b as u32 & 31))
    }); // rotl
    i32_binop!(vm, 0x78, |a: i32, b: i32| -> VMResult<i32> {
        Ok(a.rotate_right(b as u32 & 31))
    }); // rotr
}

// ── Numeric i64 ──────────────────────────────────────────────────────────

fn register_numeric_i64(vm: &mut GenericVM) {
    // i64.const (0x42)
    vm.register_context_opcode(0x42, |vm, instr, _code, _ctx| {
        let val = operand_int(instr);
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });

    // WasmGC prefix (0xFB) — LANG77 / McCarthy L3b-3a-3a + L3b-3a-3b.
    //
    // The decoder bundles the sub-opcode and its index immediates into the
    // `ctx.gc_ops` side-table; the instruction's operand is the index into it.
    // We dispatch on the sub-opcode:
    //
    //   - `i31.new`   (0x1C) / `i31.get_s` (0x1D): an `i31ref` ≡ its plain `i32`
    //     payload on the value stack, so both are stack-identity **no-ops** —
    //     the i32 passes straight through (L3b-3a-3a).
    //   - `struct.new` (0x00): pop `struct_field_counts[type_idx]` field values
    //     (last field on top), allocate a `GcStruct` on the heap, push a
    //     `Ref(Some(handle))` (L3b-3a-3b).
    //   - `struct.get` (0x02): pop a non-null struct ref, push `fields[field_idx]`.
    //   - `struct.set` (0x04): pop the new value then a non-null struct ref,
    //     write `fields[field_idx]`.
    //
    // Every failure mode (unknown type/field index, null deref, missing arity,
    // type mismatch, unknown sub-opcode) is a clean trap, never a panic.
    vm.register_context_opcode(0xFB, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let op_idx = operand_int(instr) as usize;
        let op = *ctx.gc_ops.get(op_idx).ok_or_else(|| {
            VMError::GenericError(format!("WasmGC operand index {op_idx} out of range"))
        })?;
        match op.sub {
            // i31.new / i31.get_s: i31ref ≡ its i32 payload — stack-identity.
            0x1C | 0x1D => {}

            // struct.new <type_idx>: allocate a cons-like object.
            0x00 => {
                let n = *ctx
                    .struct_field_counts
                    .get(op.type_idx as usize)
                    .ok_or_else(|| {
                        VMError::GenericError(format!(
                            "struct.new: no field count registered for struct type {} \
                             (call set_struct_field_counts)",
                            op.type_idx
                        ))
                    })? as usize;
                // Pop the fields. WasmGC pushes field 0 first, so field (n-1) is
                // on top: pop into the tail of the Vec, then it reads correctly.
                let mut fields = vec![WasmValue::I32(0); n];
                for slot in fields.iter_mut().rev() {
                    *slot = pop_wasm(vm)?;
                }
                let obj = GcStruct { type_idx: op.type_idx, fields };
                let handle = gc::alloc(ctx, obj)?;
                push_wasm(vm, WasmValue::Ref(Some(handle)));
            }

            // struct.get <type_idx> <field_idx>: read a field. A `None` slot
            // (out-of-range *or* tombstoned by a collection) is the same
            // clean-trap case — both mean "not a live object".
            0x02 => {
                let handle = pop_struct_ref(vm, "struct.get")?;
                let obj = ctx
                    .gc_heap
                    .get(handle as usize)
                    .and_then(|slot| slot.as_ref())
                    .ok_or_else(|| {
                        VMError::GenericError(format!("struct.get: dangling handle {handle}"))
                    })?;
                let val = *obj.fields.get(op.field_idx as usize).ok_or_else(|| {
                    VMError::GenericError(format!(
                        "struct.get: field {} out of range for struct with {} field(s)",
                        op.field_idx,
                        obj.fields.len()
                    ))
                })?;
                push_wasm(vm, val);
            }

            // struct.set <type_idx> <field_idx>: write a field. Same
            // None-slot clean-trap reasoning as struct.get above.
            0x04 => {
                // Stack order: ... ref, value (value on top).
                let val = pop_wasm(vm)?;
                let handle = pop_struct_ref(vm, "struct.set")?;
                let obj = ctx
                    .gc_heap
                    .get_mut(handle as usize)
                    .and_then(|slot| slot.as_mut())
                    .ok_or_else(|| {
                        VMError::GenericError(format!("struct.set: dangling handle {handle}"))
                    })?;
                let slot = obj.fields.get_mut(op.field_idx as usize).ok_or_else(|| {
                    VMError::GenericError(format!(
                        "struct.set: field {} out of range",
                        op.field_idx
                    ))
                })?;
                *slot = val;
            }

            // ref.test (0x14) / ref.test null (0x15): pop a reference, push i32
            // 1 if it is an instance of the heap type, else 0 (LANG77 L3b-3a-4).
            // This is what McCarthy `pair?` lowers to: "is this value a cons
            // cell?" — i.e. a (non-null) `$LispyPair` struct reference.
            //
            // Our value model has exactly one struct type ($LispyPair), so a
            // `struct.new` result — the only way to make a `Ref(Some(_))` — is
            // always that type. A test against any concrete struct type is
            // therefore "is it a struct ref": `Ref(Some(_))` → 1; an `i31`
            // payload (`I32`) or the null reference → 0. The `0x15` (nullable)
            // variant additionally accepts the null reference.
            0x14 | 0x15 => {
                let nullable = op.sub == 0x15;
                let matches = match pop_wasm(vm)? {
                    WasmValue::Ref(Some(_)) => true,
                    WasmValue::Ref(None) => nullable,
                    _ => false, // an i31 payload / numeric value is not a struct ref
                };
                push_wasm(vm, WasmValue::I32(if matches { 1 } else { 0 }));
            }

            other => {
                return Err(VMError::GenericError(format!(
                    "unsupported WasmGC opcode 0xFB 0x{other:02X}"
                )));
            }
        }
        vm.advance_pc();
        Ok(None)
    });

    // Bulk-memory prefix (0xFC) — `memory.copy` (E4-dyn runtime string concat)
    // and `memory.fill` (task #94).
    //
    // `memory.copy` takes three i32 operands pushed in order dest, src, size, so
    // the value stack (bottom → top) is `[dest, src, size]`: pop size, then src,
    // then dest.  It copies `size` bytes from `src` to `dest` within the single
    // linear memory, overlap-safe.  Out-of-range (either endpoint) is a clean trap.
    // `memory.fill` takes three i32 operands pushed in order dest, value, size
    // (`[dest, value, size]`): pop size, then value (truncated to a byte), then
    // dest, and fills `size` bytes starting at `dest` with that byte.
    // Sub-opcodes 0x00-0x07 (WASM03), 0x08/0x09 (memory.init/data.drop, task
    // #95), 0x0A/0x0B (memory.copy/memory.fill, task #94), and 0x0F/0x10/
    // 0x11 (table.grow/table.size/table.fill, task #98) are implemented;
    // any other 0xFC sub-opcode traps rather than silently misbehaving.
    //
    // 0x00-0x07: the "non-trapping float-to-int conversions" proposal's 8
    // `trunc_sat` instructions -- like `trunc_f32_s`/etc. but never traps:
    // NaN becomes 0, and an out-of-range magnitude SATURATES to the target
    // type's MIN/MAX instead of trapping. Rust's own `as` cast from float to
    // int has used exactly this saturating behavior (NaN -> 0, out-of-range
    // -> the nearest bound, truncate-toward-zero otherwise) since Rust 1.45
    // — a direct match for the spec's definition, so no hand-rolled bounds
    // checking is needed here (contrast the TRAPPING `trunc_f32_s`/etc.
    // handlers just above, at 0xA8-0xB1, which explicitly reject NaN/
    // overflow because THEY must trap, not saturate).
    vm.register_context_opcode(0xFC, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let (sub, data_idx, aux) = unpack_bulk_memory_operand(instr);
        match sub {
            0x00 => {
                let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(a as i32));
            }
            0x01 => {
                let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(a as u32 as i32));
            }
            0x02 => {
                let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(a as i32));
            }
            0x03 => {
                let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(a as u32 as i32));
            }
            0x04 => {
                let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I64(a as i64));
            }
            0x05 => {
                let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I64(a as u64 as i64));
            }
            0x06 => {
                let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I64(a as i64));
            }
            0x07 => {
                let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I64(a as u64 as i64));
            }
            0x08 => {
                // `memory.init` (task #95): stack (bottom -> top) is
                // `[dest, src, size]`, same pop order as memory.copy --
                // but here `src`/`size` index into the DATA SEGMENT named
                // by `data_idx` (this instruction's own decoded
                // immediate, not a stack operand), while `dest`/`size`
                // still index into memory. A dropped segment (`data.drop`
                // already ran) behaves as length-0 for bounds-checking:
                // `memory.init` with `size=0` still succeeds (any offset
                // `<= 0` does), but any `size>0` traps -- matching the
                // real spec's "a dropped segment can never be initialized
                // from again" rule, not a distinct error path.
                let n = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let src = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let dest = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let idx = data_idx as usize;
                // Security review (task #95): an out-of-range `idx` is
                // ALWAYS a hard error, checked and rejected BEFORE any
                // indexing at all -- shouldn't happen in a validated
                // module (`wasm-validator` bounds-checks `data_idx`
                // against `module.data.len()`), but this interpreter
                // never trusts a decoded index at runtime regardless of
                // what validation should have caught, matching `data.
                // drop`'s own defensive check just below. Kept separate
                // from "segment is dropped" (below), which is a real,
                // spec-defined, IN-range state that degrades to a
                // length-0 segment rather than erroring outright. A
                // prior version conflated the two by defaulting an
                // out-of-range `idx` to `segment_len = 0` via `.get(idx)
                // .unwrap_or(0)`, which a zero-length `memory.init`
                // trivially satisfied (`0 <= 0`) and then indexed
                // `ctx.data_segments[idx]` directly in the copy step --
                // panicking on the very index the bounds check had just
                // "passed," instead of hitting this check at all.
                if idx >= ctx.data_segments.len() || idx >= ctx.dropped_data_segments.len() {
                    return Err(VMError::GenericError(format!("memory.init: data segment index {idx} out of bounds")));
                }
                // Task #92/W18: `aux` carries the real, decoded memory
                // index -- previously discarded entirely (`memory.init`
                // always wrote to memory 0 regardless of what the
                // trailing immediate actually named).
                let memidx = aux as usize;
                let dropped = ctx.dropped_data_segments[idx];
                let segment_bytes: &[u8] = if dropped { &[] } else { ctx.data_segments[idx].as_slice() };
                match src.checked_add(n) {
                    Some(src_end) if src_end <= segment_bytes.len() => {
                        let bytes = segment_bytes[src..src_end].to_vec();
                        get_memory_at(ctx, memidx)?.write_bytes(dest, &bytes).map_err(VMError::from)?;
                    }
                    _ => {
                        return Err(VMError::from(TrapError::new(format!(
                            "out of bounds memory access: memory.init data_idx={idx}, src={src}, len={n}, segment_size={}",
                            segment_bytes.len()
                        ))));
                    }
                }
            }
            0x09 => {
                // `data.drop` (task #95): no stack operands at all -- just
                // marks the segment permanently dropped. Bounds-checking
                // `data_idx` here is defensive only (a validated module's
                // own `wasm-validator` check already guarantees it's in
                // range); this is the same "never trust the raw index at
                // runtime regardless of what validation should have
                // caught" posture every other indexed operand in this
                // interpreter takes.
                let idx = data_idx as usize;
                if idx >= ctx.dropped_data_segments.len() {
                    return Err(VMError::GenericError(format!("data.drop: data segment index {idx} out of bounds")));
                }
                ctx.dropped_data_segments[idx] = true;
            }
            0x0A => {
                // `as u32 as usize` (never a bare `as usize`): a negative i32 must
                // truncate to its 32-bit value, NOT sign-extend to a huge usize — the
                // same guard every other memory op applies via `effective_addr`.
                // Sign-extending here would let `offset + width` wrap past the bounds
                // check and index out of bounds.
                let n = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let src = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let dest = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                // Task #92/W18: `data_idx`/`aux` carry the real, decoded
                // dst/src memory indices -- previously both discarded
                // entirely (`memory.copy` always operated on memory 0
                // regardless of what the two trailing immediates
                // actually named). Same `Table::copy_between` two-
                // pointer, no-aliasing pattern task #97's own
                // `/security-review` finding established.
                let dst_memidx = data_idx as usize;
                let src_memidx = aux as usize;
                if dst_memidx >= ctx.memories.len() {
                    return Err(VMError::GenericError(format!("memory.copy: destination memory index {dst_memidx} out of bounds")));
                }
                if src_memidx >= ctx.memories.len() {
                    return Err(VMError::GenericError(format!("memory.copy: source memory index {src_memidx} out of bounds")));
                }
                let dst_ptr = ctx.memories[dst_memidx];
                let src_ptr: *const LinearMemory = ctx.memories[src_memidx];
                // SAFETY: both pointers were bounds-checked above and come
                // from `ctx.memories`, which outlives this call.
                // `LinearMemory::copy_between` takes raw pointers, not
                // `&mut`/`&` references -- see its own doc comment for
                // why: forming both over the same `LinearMemory` when
                // `dst_memidx == src_memidx` would be aliasing UB even
                // though no actual read/write hazard exists.
                unsafe {
                    LinearMemory::copy_between(dst_ptr, src_ptr, dest, src, n).map_err(VMError::from)?;
                }
            }
            0x0B => {
                // `memory.fill` (task #94): stack (bottom -> top) is
                // `[dest, value, size]` -- pop size, then value, then
                // dest, matching `wasm-validator`'s `type_check.rs` pop
                // order for this same sub-opcode. `value` is a byte
                // (`i32 as u8`, truncating): the spec defines memory.fill's
                // fill byte as the low 8 bits of the i32 operand.
                let n = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let value = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as u8;
                let dest = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                // Task #92/W18: `data_idx` carries the real, decoded
                // memory index -- previously discarded entirely.
                let memidx = data_idx as usize;
                get_memory_at(ctx, memidx)?.fill(dest, value, n).map_err(VMError::from)?;
            }
            0x0C => {
                // `table.init` (task #97): stack (bottom -> top) is
                // `[dest, src, len]`, same pop order as `memory.init`
                // (task #95). `dest` indexes into the TARGET table
                // (named by `aux`, this instruction's own decoded
                // table-index immediate); `src`/`len` index into the
                // ELEMENT SEGMENT named by `data_idx` (the elem-segment
                // index immediate). A dropped segment behaves as
                // length-0 for bounds-checking -- `table.init` with
                // `len=0` still succeeds, but any `len>0` traps --
                // matching `memory.init`'s own "a dropped segment can
                // never be initialized from again" rule exactly.
                let len = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let src = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let dest = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32 as usize;
                let elem_idx = data_idx as usize;
                // Security review pattern (task #95/#97): an out-of-range
                // `elem_idx` is ALWAYS a hard error, checked before any
                // indexing at all -- kept separate from "segment is
                // dropped" (below), which is a real, spec-defined,
                // IN-range state that degrades to a length-0 segment
                // rather than erroring outright.
                if elem_idx >= ctx.elements.len() || elem_idx >= ctx.dropped_elements.len() {
                    return Err(VMError::GenericError(format!("table.init: elem segment index {elem_idx} out of bounds")));
                }
                let dropped = ctx.dropped_elements[elem_idx];
                // Cloned to an owned `Vec` (not borrowed as `&[Option<u32>]`
                // from `ctx.elements`) so this doesn't hold an immutable
                // borrow of `ctx` across the `get_table(ctx, ..)` call just
                // below, which needs `ctx` mutably.
                let segment: Vec<Option<u32>> = if dropped { Vec::new() } else { ctx.elements[elem_idx].clone() };
                // Both bounds are checked BEFORE any write happens, so a
                // trap (either side out of range) leaves the table
                // completely untouched -- same atomicity discipline
                // `Table::copy_between` below uses.
                let src_end = src.checked_add(len);
                let table = get_table(ctx, aux as usize)?;
                let dest_end = dest.checked_add(len);
                match (src_end, dest_end) {
                    (Some(se), Some(de)) if se <= segment.len() && de <= table.size() as usize => {
                        for i in 0..len {
                            table.set((dest + i) as u32, segment[src + i]).map_err(VMError::from)?;
                        }
                    }
                    _ => {
                        return Err(VMError::from(TrapError::new(format!(
                            "out of bounds table access: table.init elem_idx={elem_idx}, dest={dest}, src={src}, len={len}, segment_size={}",
                            segment.len()
                        ))));
                    }
                }
            }
            0x0D => {
                // `elem.drop` (task #97): no stack operands at all --
                // just marks the segment permanently dropped, same shape
                // `data.drop`'s own handler uses (task #95).
                let elem_idx = data_idx as usize;
                if elem_idx >= ctx.dropped_elements.len() {
                    return Err(VMError::GenericError(format!("elem.drop: elem segment index {elem_idx} out of bounds")));
                }
                ctx.dropped_elements[elem_idx] = true;
            }
            0x0E => {
                // `table.copy` (task #97): stack (bottom -> top) is
                // `[dest, src, len]`, same pop order as `memory.copy`/
                // `table.fill`. `dest` indexes into the DESTINATION
                // table (`data_idx`, the dst table-index immediate);
                // `src` indexes into the SOURCE table (`aux`, the src
                // table-index immediate) -- unlike `memory.copy`'s own
                // discarded-to-0 memory operands (W16 deferred scope),
                // this repo already supports `MAX_TABLES` real tables,
                // so both operands are real decoded indices, never
                // hardcoded to 0.
                let len = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32;
                let src = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32;
                let dest = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32;
                let dst_table_idx = data_idx as usize;
                let src_table_idx = aux as usize;
                if dst_table_idx >= ctx.tables.len() {
                    return Err(VMError::GenericError(format!("table.copy: destination table index {dst_table_idx} out of bounds")));
                }
                if src_table_idx >= ctx.tables.len() {
                    return Err(VMError::GenericError(format!("table.copy: source table index {src_table_idx} out of bounds")));
                }
                let dst_ptr = ctx.tables[dst_table_idx];
                let src_ptr: *const Table = ctx.tables[src_table_idx];
                // SAFETY: both pointers were bounds-checked above and come
                // from `ctx.tables`, which outlives this call.
                // `Table::copy_between` takes raw pointers (not `&mut`/`&`
                // references -- see its own doc comment for why: forming
                // both over the same `Table` when `dst_table_idx ==
                // src_table_idx` would be aliasing UB even though no
                // actual read/write hazard exists) and reads the whole
                // source range into a temporary `Vec` before writing any
                // destination slot, so a same-table self-copy is sound.
                unsafe {
                    Table::copy_between(dst_ptr, src_ptr, dest, src, len).map_err(VMError::from)?;
                }
            }
            0x0F => {
                // `table.grow` (task #98): stack (bottom -> top) is
                // `[init, delta]` -- pop delta (i32), then init (a
                // reference value, funcref or externref depending on the
                // target table's own element type -- this interpreter
                // doesn't distinguish the two at runtime, see
                // `WasmValue::Ref`'s own doc comment). Pushes the OLD
                // size (i32) on success, or -1 on failure -- growth
                // failure is a normal return value per spec, never a
                // trap, same contract as `memory.grow`.
                let delta = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32;
                let init = match pop_wasm(vm)? {
                    WasmValue::Ref(v) => v,
                    other => return Err(VMError::GenericError(format!("type mismatch: table.grow expected a reference, got {other:?}"))),
                };
                let target_idx = data_idx as usize;
                // Security review (task #98, round 2): `Table::grow`'s own
                // `MAX_TABLE_ELEMENTS` cap bounds a SINGLE table, but
                // `MAX_TABLES` (64) tables each individually grown to that
                // cap would still total ~4.77GB from one small module --
                // reintroducing at RUNTIME exactly the aggregate DoS gap
                // `wasm-validator`'s "Check 2b" already closes at
                // DECLARE-time for a table's declared `min`. Sum every
                // OTHER table's CURRENT size plus this table's PROSPECTIVE
                // new size (`sz + delta`, not yet applied) and reject
                // BEFORE ever calling `Table::grow` -- so a rejected
                // growth leaves every table, including the target,
                // completely untouched, same "no partial mutation on
                // failure" discipline `memory.init`'s out-of-range check
                // (task #95) established.
                let mut aggregate: u64 = 0;
                for (i, &ptr) in ctx.tables.iter().enumerate() {
                    let sz = unsafe { (*ptr).size() as u64 };
                    aggregate += if i == target_idx { sz + delta as u64 } else { sz };
                }
                if aggregate > MAX_TABLE_ELEMENTS as u64 {
                    push_wasm(vm, WasmValue::I32(-1));
                } else {
                    let table = get_table(ctx, target_idx)?;
                    let old_size = table.grow(delta, init);
                    push_wasm(vm, WasmValue::I32(old_size));
                }
            }
            0x10 => {
                // `table.size` (task #98): no stack operands, pushes the
                // table's current size as i32.
                let table = get_table(ctx, data_idx as usize)?;
                push_wasm(vm, WasmValue::I32(table.size() as i32));
            }
            0x11 => {
                // `table.fill` (task #98): stack (bottom -> top) is
                // `[dest, value, len]` -- pop len, then value, then dest,
                // matching `wasm-validator`'s pop order for this
                // sub-opcode (mirrors `memory.fill`'s own `[dest, value,
                // size]` shape just above).
                let len = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32;
                let value = match pop_wasm(vm)? {
                    WasmValue::Ref(v) => v,
                    other => return Err(VMError::GenericError(format!("type mismatch: table.fill expected a reference, got {other:?}"))),
                };
                let dest = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32;
                let table = get_table(ctx, data_idx as usize)?;
                table.fill(dest, value, len).map_err(VMError::from)?;
            }
            other => {
                return Err(VMError::GenericError(format!(
                    "unsupported bulk-memory opcode 0xFC 0x{other:02X}"
                )));
            }
        }
        vm.advance_pc();
        Ok(None)
    });

    // ref.null (0xD0 <heap_type>) — push the null reference (LANG77 L3b-3a-3b).
    // In the lisp value model this is how `nil` is materialised.
    vm.register_context_opcode(0xD0, |vm, _instr, _code, _ctx| {
        push_wasm(vm, WasmValue::Ref(None));
        vm.advance_pc();
        Ok(None)
    });

    // ref.is_null (0xD1) — pop an anyref, push i32 1 if it is the null
    // reference, else 0 (LANG77 L3b-3a-3b).  A boxed integer (i31 payload, an
    // `I32`) is a non-null reference, so it yields 0.
    vm.register_context_opcode(0xD1, |vm, _instr, _code, _ctx| {
        let is_null = matches!(pop_wasm(vm)?, WasmValue::Ref(None));
        push_wasm(vm, WasmValue::I32(if is_null { 1 } else { 0 }));
        vm.advance_pc();
        Ok(None)
    });

    // ref.func (0xD2 <funcidx>) — push a non-null funcref referring to a
    // function by index (WASM17). The wrapped `u32` is a function index
    // into `ctx.func_types`/`ctx.func_bodies`, reusing the same uniform
    // `Ref(Option<u32>)` handle `ref.null`/`ref.is_null` already carry —
    // only the *static* type (tracked by `wasm-validator`) distinguishes a
    // funcref from any other reference kind, see `code/specs/
    // W08-wasm-funcref-externref.md`. Bounds-checked against the same
    // `func_types` table `call_function_inner`'s own bounds check uses.
    vm.register_context_opcode(0xD2, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let func_index = operand_int(instr) as usize;
        if func_index >= ctx.func_types.len() {
            return Err(VMError::GenericError(format!("undefined function {func_index}")));
        }
        push_wasm(vm, WasmValue::Ref(Some(func_index as u32)));
        vm.advance_pc();
        Ok(None)
    });

    // i64.eqz (0x50)
    vm.register_context_opcode(0x50, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(if a == 0 { 1 } else { 0 }));
        vm.advance_pc();
        Ok(None)
    });

    macro_rules! i64_cmp {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let b = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
                let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(if $op(a, b) { 1 } else { 0 }));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    i64_cmp!(vm, 0x51, |a: i64, b: i64| a == b); // eq
    i64_cmp!(vm, 0x52, |a: i64, b: i64| a != b); // ne
    i64_cmp!(vm, 0x53, |a: i64, b: i64| a < b); // lt_s
    i64_cmp!(vm, 0x54, |a: i64, b: i64| (a as u64) < (b as u64)); // lt_u
    i64_cmp!(vm, 0x55, |a: i64, b: i64| a > b); // gt_s
    i64_cmp!(vm, 0x56, |a: i64, b: i64| (a as u64) > (b as u64)); // gt_u
    i64_cmp!(vm, 0x57, |a: i64, b: i64| a <= b); // le_s
    i64_cmp!(vm, 0x58, |a: i64, b: i64| (a as u64) <= (b as u64)); // le_u
    i64_cmp!(vm, 0x59, |a: i64, b: i64| a >= b); // ge_s
    i64_cmp!(vm, 0x5A, |a: i64, b: i64| (a as u64) >= (b as u64)); // ge_u

    macro_rules! i64_binop {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let b = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
                let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
                let result = $op(a, b)?;
                push_wasm(vm, WasmValue::I64(result));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    // Unary
    vm.register_context_opcode(0x79, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a.leading_zeros() as i64));
        vm.advance_pc();
        Ok(None)
    }); // clz
    vm.register_context_opcode(0x7A, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a.trailing_zeros() as i64));
        vm.advance_pc();
        Ok(None)
    }); // ctz
    vm.register_context_opcode(0x7B, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a.count_ones() as i64));
        vm.advance_pc();
        Ok(None)
    }); // popcnt

    i64_binop!(vm, 0x7C, |a: i64, b: i64| -> VMResult<i64> {
        Ok(a.wrapping_add(b))
    }); // add
    i64_binop!(vm, 0x7D, |a: i64, b: i64| -> VMResult<i64> {
        Ok(a.wrapping_sub(b))
    }); // sub
    i64_binop!(vm, 0x7E, |a: i64, b: i64| -> VMResult<i64> {
        Ok(a.wrapping_mul(b))
    }); // mul
    i64_binop!(vm, 0x7F, |a: i64, b: i64| -> VMResult<i64> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        if a == i64::MIN && b == -1 {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        Ok(a.wrapping_div(b))
    }); // div_s
    i64_binop!(vm, 0x80, |a: i64, b: i64| -> VMResult<i64> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        Ok(((a as u64).wrapping_div(b as u64)) as i64)
    }); // div_u
    i64_binop!(vm, 0x81, |a: i64, b: i64| -> VMResult<i64> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        if a == i64::MIN && b == -1 {
            return Ok(0);
        }
        Ok(a.wrapping_rem(b))
    }); // rem_s
    i64_binop!(vm, 0x82, |a: i64, b: i64| -> VMResult<i64> {
        if b == 0 {
            return Err(VMError::GenericError("integer divide by zero".into()));
        }
        Ok(((a as u64).wrapping_rem(b as u64)) as i64)
    }); // rem_u
    i64_binop!(vm, 0x83, |a: i64, b: i64| -> VMResult<i64> { Ok(a & b) }); // and
    i64_binop!(vm, 0x84, |a: i64, b: i64| -> VMResult<i64> { Ok(a | b) }); // or
    i64_binop!(vm, 0x85, |a: i64, b: i64| -> VMResult<i64> { Ok(a ^ b) }); // xor
    i64_binop!(vm, 0x86, |a: i64, b: i64| -> VMResult<i64> {
        Ok(a.wrapping_shl((b & 63) as u32))
    }); // shl
    i64_binop!(vm, 0x87, |a: i64, b: i64| -> VMResult<i64> {
        Ok(a.wrapping_shr((b & 63) as u32))
    }); // shr_s
    i64_binop!(vm, 0x88, |a: i64, b: i64| -> VMResult<i64> {
        Ok(((a as u64).wrapping_shr((b & 63) as u32)) as i64)
    }); // shr_u
    i64_binop!(vm, 0x89, |a: i64, b: i64| -> VMResult<i64> {
        Ok(a.rotate_left((b & 63) as u32))
    }); // rotl
    i64_binop!(vm, 0x8A, |a: i64, b: i64| -> VMResult<i64> {
        Ok(a.rotate_right((b & 63) as u32))
    }); // rotr
}

// ── Numeric f32 ──────────────────────────────────────────────────────────

fn register_numeric_f32(vm: &mut GenericVM) {
    // f32.const (0x43)
    vm.register_context_opcode(0x43, |vm, instr, _code, _ctx| {
        let val = match &instr.operand {
            Some(Operand::Index(i)) => f32::from_bits(*i as u32),
            _ => 0.0,
        };
        push_wasm(vm, WasmValue::F32(val));
        vm.advance_pc();
        Ok(None)
    });

    macro_rules! f32_cmp {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let b = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(if $op(a, b) { 1 } else { 0 }));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    f32_cmp!(vm, 0x5B, |a: f32, b: f32| a == b); // eq
    f32_cmp!(vm, 0x5C, |a: f32, b: f32| a != b); // ne
    f32_cmp!(vm, 0x5D, |a: f32, b: f32| a < b); // lt
    f32_cmp!(vm, 0x5E, |a: f32, b: f32| a > b); // gt
    f32_cmp!(vm, 0x5F, |a: f32, b: f32| a <= b); // le
    f32_cmp!(vm, 0x60, |a: f32, b: f32| a >= b); // ge

    macro_rules! f32_unop {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::F32($op(a)));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    f32_unop!(vm, 0x8B, |a: f32| a.abs()); // abs
    f32_unop!(vm, 0x8C, |a: f32| -a); // neg
    // WASM requires any NaN propagated through ceil/floor/trunc to have its
    // quiet bit set (the spec's `nans()` function always quiets). The
    // platform libm `a.ceil()`/`.floor()`/`.trunc()` call on a NaN input
    // sometimes returns it with the ORIGINAL bit pattern unchanged --
    // whether that preserves a signaling NaN's clear quiet bit is platform-
    // and-libm-dependent, found as a genuine cross-platform (macOS vs
    // Linux) discrepancy running the real WebAssembly/testsuite corpus
    // (`f64.wast`'s `nan:arithmetic` cases) via `wasm-conformance`. Forcing
    // the canonical NaN on any NaN input sidesteps the platform dependency
    // entirely rather than trying to track it down further.
    f32_unop!(vm, 0x8D, |a: f32| if a.is_nan() { f32::NAN } else { a.ceil() }); // ceil
    f32_unop!(vm, 0x8E, |a: f32| if a.is_nan() { f32::NAN } else { a.floor() }); // floor
    f32_unop!(vm, 0x8F, |a: f32| if a.is_nan() { f32::NAN } else { a.trunc() }); // trunc
    f32_unop!(
        vm,
        0x90,
        |a: f32| {
            // WASM's `nearest` (round-ties-to-even) must preserve the sign
            // of a result that rounds to zero -- `nearest(-0.25)` is
            // `-0.0`, not `0.0` -- per IEEE 754's roundTiesToEven. Rust's
            // `f32::round()` doesn't guarantee that for magnitudes that
            // round down to zero; found running the real
            // WebAssembly/testsuite corpus (`f32.wast`) via
            // `wasm-conformance`.
            let rounded = if a.fract() == 0.5 || a.fract() == -0.5 {
                // nearest even
                let r = a.round();
                if r as i32 % 2 != 0 {
                    r - a.signum()
                } else {
                    r
                }
            } else {
                a.round()
            };
            if rounded == 0.0 {
                rounded.copysign(a)
            } else {
                rounded
            }
        }
    ); // nearest
    f32_unop!(vm, 0x91, |a: f32| a.sqrt()); // sqrt

    macro_rules! f32_binop {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let b = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::F32($op(a, b)));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    f32_binop!(vm, 0x92, |a: f32, b: f32| a + b); // add
    f32_binop!(vm, 0x93, |a: f32, b: f32| a - b); // sub
    f32_binop!(vm, 0x94, |a: f32, b: f32| a * b); // mul
    f32_binop!(vm, 0x95, |a: f32, b: f32| a / b); // div
    // WASM's `min`/`max` MUST propagate NaN unconditionally (if either
    // operand is NaN, the result is NaN) and treat -0.0 < +0.0 for the
    // purposes of picking a result -- neither matches Rust's native
    // `f32::min`/`max`, which follow IEEE 754-2008 minNum/maxNum semantics
    // instead: "if one of the arguments is NaN, then the OTHER argument is
    // returned." Using `.min()`/`.max()` directly silently turned every
    // `min(NaN, x)`/`max(NaN, x)` into a normal float, found running the
    // real WebAssembly/testsuite corpus (`f32.wast`) via `wasm-conformance`.
    f32_binop!(vm, 0x96, |a: f32, b: f32| {
        if a.is_nan() || b.is_nan() {
            f32::NAN
        } else if a == 0.0 && b == 0.0 {
            if a.is_sign_negative() || b.is_sign_negative() { -0.0 } else { 0.0 }
        } else {
            a.min(b)
        }
    }); // min
    f32_binop!(vm, 0x97, |a: f32, b: f32| {
        if a.is_nan() || b.is_nan() {
            f32::NAN
        } else if a == 0.0 && b == 0.0 {
            if a.is_sign_positive() || b.is_sign_positive() { 0.0 } else { -0.0 }
        } else {
            a.max(b)
        }
    }); // max
    f32_binop!(vm, 0x98, |a: f32, b: f32| f32::from_bits(
        (a.to_bits() & 0x7FFF_FFFF) | (b.to_bits() & 0x8000_0000)
    )); // copysign
}

// ── Numeric f64 ──────────────────────────────────────────────────────────

fn register_numeric_f64(vm: &mut GenericVM) {
    // f64.const (0x44)
    vm.register_context_opcode(0x44, |vm, instr, _code, _ctx| {
        let val = match &instr.operand {
            Some(Operand::Index(i)) => f64::from_bits(*i as u64),
            _ => 0.0,
        };
        push_wasm(vm, WasmValue::F64(val));
        vm.advance_pc();
        Ok(None)
    });

    macro_rules! f64_cmp {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let b = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(if $op(a, b) { 1 } else { 0 }));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    f64_cmp!(vm, 0x61, |a: f64, b: f64| a == b); // eq
    f64_cmp!(vm, 0x62, |a: f64, b: f64| a != b); // ne
    f64_cmp!(vm, 0x63, |a: f64, b: f64| a < b); // lt
    f64_cmp!(vm, 0x64, |a: f64, b: f64| a > b); // gt
    f64_cmp!(vm, 0x65, |a: f64, b: f64| a <= b); // le
    f64_cmp!(vm, 0x66, |a: f64, b: f64| a >= b); // ge

    macro_rules! f64_unop {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::F64($op(a)));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    f64_unop!(vm, 0x99, |a: f64| a.abs()); // abs
    f64_unop!(vm, 0x9A, |a: f64| -a); // neg
    // See the f32 ceil/floor/trunc registrations above for why NaN needs
    // explicit quieting here.
    f64_unop!(vm, 0x9B, |a: f64| if a.is_nan() { f64::NAN } else { a.ceil() }); // ceil
    f64_unop!(vm, 0x9C, |a: f64| if a.is_nan() { f64::NAN } else { a.floor() }); // floor
    f64_unop!(vm, 0x9D, |a: f64| if a.is_nan() { f64::NAN } else { a.trunc() }); // trunc
    f64_unop!(
        vm,
        0x9E,
        // See the f32 `nearest` registration above for why the zero-result
        // sign fixup is needed.
        |a: f64| {
            let rounded = if a.fract() == 0.5 || a.fract() == -0.5 {
                let r = a.round();
                if r as i64 % 2 != 0 {
                    r - a.signum()
                } else {
                    r
                }
            } else {
                a.round()
            };
            if rounded == 0.0 {
                rounded.copysign(a)
            } else {
                rounded
            }
        }
    ); // nearest
    f64_unop!(vm, 0x9F, |a: f64| a.sqrt()); // sqrt

    macro_rules! f64_binop {
        ($vm:expr, $opcode:expr, $op:expr) => {
            $vm.register_context_opcode($opcode, |vm, _instr, _code, _ctx| {
                let b = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
                push_wasm(vm, WasmValue::F64($op(a, b)));
                vm.advance_pc();
                Ok(None)
            });
        };
    }

    f64_binop!(vm, 0xA0, |a: f64, b: f64| a + b); // add
    f64_binop!(vm, 0xA1, |a: f64, b: f64| a - b); // sub
    f64_binop!(vm, 0xA2, |a: f64, b: f64| a * b); // mul
    f64_binop!(vm, 0xA3, |a: f64, b: f64| a / b); // div
    // See the f32 `min`/`max` registration above for why this can't be
    // Rust's native `.min()`/`.max()` -- same NaN-propagation mismatch.
    f64_binop!(vm, 0xA4, |a: f64, b: f64| {
        if a.is_nan() || b.is_nan() {
            f64::NAN
        } else if a == 0.0 && b == 0.0 {
            if a.is_sign_negative() || b.is_sign_negative() { -0.0 } else { 0.0 }
        } else {
            a.min(b)
        }
    }); // min
    f64_binop!(vm, 0xA5, |a: f64, b: f64| {
        if a.is_nan() || b.is_nan() {
            f64::NAN
        } else if a == 0.0 && b == 0.0 {
            if a.is_sign_positive() || b.is_sign_positive() { 0.0 } else { -0.0 }
        } else {
            a.max(b)
        }
    }); // max
    f64_binop!(vm, 0xA6, |a: f64, b: f64| f64::from_bits(
        (a.to_bits() & 0x7FFF_FFFF_FFFF_FFFF) | (b.to_bits() & 0x8000_0000_0000_0000)
    )); // copysign
}

// ── Conversion instructions (0xA7-0xBF) ──────────────────────────────────

/// Whether `z` (the SOURCE float, already widened to `f64` -- lossless for
/// an `f32` input, since every `f32` value is exactly representable in
/// `f64`) is in i32.trunc_s's valid domain: `-2^31 - 1 < z < 2^31`, per the
/// WASM spec's `trunc_sN` definition, checked against the REAL value, not a
/// rounded one. `-2147483649.0` is exactly representable in `f64` (its
/// magnitude is well under `f64`'s 2^53 exact-integer limit), so this strict
/// `>` comparison is exact, not an approximation.
fn trunc_s_i32_in_range(z: f64) -> bool {
    z > -2147483649.0 && z < 2147483648.0
}

/// As [`trunc_s_i32_in_range`], for `trunc_u`: valid iff `-1 < z < 2^32`.
fn trunc_u_i32_in_range(z: f64) -> bool {
    z > -1.0 && z < 4294967296.0
}

/// As [`trunc_s_i32_in_range`], for i64.trunc_s: valid iff
/// `-2^63 - 1 < z < 2^63`. Unlike the i32 case, `-9223372036854775809.0`
/// (2^63 + 1) is NOT exactly representable in `f64` (its magnitude exceeds
/// `f64`'s 2^53 exact-integer limit), so the lower bound is written as `z >=
/// -9223372036854775808.0` (`i64::MIN` as `f64`, exactly representable —
/// it's a power of two) instead of a strict `>` against an inexact
/// constant. This is still an EXACT check, not an approximation: `f64`'s
/// representable-value spacing near `-2^63` is 2^11 = 2048 (`2^(63-52)`),
/// so no `f64` value exists strictly between `-2^63 - 1` and `-2^63` for
/// the strict/non-strict distinction to ever matter.
fn trunc_s_i64_in_range(z: f64) -> bool {
    (-9223372036854775808.0..9223372036854775808.0).contains(&z)
}

/// As [`trunc_s_i64_in_range`], for i64.trunc_u: valid iff `-1 < z < 2^64`.
/// `-1.0` and `18446744073709551616.0` (2^64) are both exactly
/// representable in `f64`.
fn trunc_u_i64_in_range(z: f64) -> bool {
    z > -1.0 && z < 18446744073709551616.0
}

fn register_conversion(vm: &mut GenericVM) {
    // i32.wrap_i64 (0xA7)
    vm.register_context_opcode(0xA7, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(a as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i32.trunc_f32_s (0xA8)
    vm.register_context_opcode(0xA8, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_s_i32_in_range(a as f64) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I32(a as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i32.trunc_f32_u (0xA9)
    vm.register_context_opcode(0xA9, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_u_i32_in_range(a as f64) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I32(a as u32 as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i32.trunc_f64_s (0xAA)
    vm.register_context_opcode(0xAA, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_s_i32_in_range(a) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I32(a as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i32.trunc_f64_u (0xAB)
    vm.register_context_opcode(0xAB, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_u_i32_in_range(a) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I32(a as u32 as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i64.extend_i32_s (0xAC)
    vm.register_context_opcode(0xAC, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a as i64));
        vm.advance_pc();
        Ok(None)
    });

    // i64.extend_i32_u (0xAD)
    vm.register_context_opcode(0xAD, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a as u32 as i64));
        vm.advance_pc();
        Ok(None)
    });

    // i64.trunc_f32_s (0xAE), i64.trunc_f32_u (0xAF), i64.trunc_f64_s (0xB0), i64.trunc_f64_u (0xB1)
    //
    // A security/conformance investigation for WASM03 (sign-extension/
    // trunc_sat) found these 4 handlers -- pre-existing, untouched by that
    // work otherwise -- had NO overflow range check at all beyond NaN,
    // because they'd never been reachable: `conversions.wast`, the only
    // vendored file exercising them, failed to even PARSE until WASM03's
    // opcode-table additions landed. `a as i64`/`a as u64 as i64` alone is
    // Rust's SATURATING float-to-int cast (see the sign-extension section
    // above and 0xFC's trunc_sat handler for the same fact used correctly
    // there) -- these TRAPPING opcodes were silently behaving like their
    // non-trapping `trunc_sat` counterparts instead, never trapping on
    // overflow. Fixed with the same `trunc_*_i64_in_range` bounds check the
    // i32-destination handlers above already used (once corrected -- see
    // those functions' own doc comments for the exact boundary math).
    vm.register_context_opcode(0xAE, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_s_i64_in_range(a as f64) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I64(a as i64));
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0xAF, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_u_i64_in_range(a as f64) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I64(a as u64 as i64));
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0xB0, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_s_i64_in_range(a) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I64(a as i64));
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0xB1, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
        }
        if !trunc_u_i64_in_range(a) {
            return Err(VMError::GenericError("integer overflow".into()));
        }
        push_wasm(vm, WasmValue::I64(a as u64 as i64));
        vm.advance_pc();
        Ok(None)
    });

    // f32.convert_i32_s (0xB2), f32.convert_i32_u (0xB3)
    vm.register_context_opcode(0xB2, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F32(a as f32));
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0xB3, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F32(a as u32 as f32));
        vm.advance_pc();
        Ok(None)
    });

    // f32.convert_i64_s (0xB4), f32.convert_i64_u (0xB5)
    vm.register_context_opcode(0xB4, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F32(a as f32));
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0xB5, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F32(a as u64 as f32));
        vm.advance_pc();
        Ok(None)
    });

    // f32.demote_f64 (0xB6)
    vm.register_context_opcode(0xB6, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F32(a as f32));
        vm.advance_pc();
        Ok(None)
    });

    // f64.convert_i32_s (0xB7), f64.convert_i32_u (0xB8)
    vm.register_context_opcode(0xB7, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F64(a as f64));
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0xB8, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F64(a as u32 as f64));
        vm.advance_pc();
        Ok(None)
    });

    // f64.convert_i64_s (0xB9), f64.convert_i64_u (0xBA)
    vm.register_context_opcode(0xB9, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F64(a as f64));
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0xBA, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F64(a as u64 as f64));
        vm.advance_pc();
        Ok(None)
    });

    // f64.promote_f32 (0xBB)
    vm.register_context_opcode(0xBB, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F64(a as f64));
        vm.advance_pc();
        Ok(None)
    });

    // i32.reinterpret_f32 (0xBC)
    vm.register_context_opcode(0xBC, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(a.to_bits() as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i64.reinterpret_f64 (0xBD)
    vm.register_context_opcode(0xBD, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a.to_bits() as i64));
        vm.advance_pc();
        Ok(None)
    });

    // f32.reinterpret_i32 (0xBE)
    vm.register_context_opcode(0xBE, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F32(f32::from_bits(a as u32)));
        vm.advance_pc();
        Ok(None)
    });

    // f64.reinterpret_i64 (0xBF)
    vm.register_context_opcode(0xBF, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F64(f64::from_bits(a as u64)));
        vm.advance_pc();
        Ok(None)
    });

    // ── Sign-extension instructions (0xC0-0xC4, WASM03) ────────────────────
    //
    // Each takes the operand's LOW N bits, reinterprets them as a two's-
    // complement SIGNED N-bit integer, and sign-extends that value to fill
    // the full i32/i64 width -- e.g. `i32.extend8_s` on the i32 bit pattern
    // 0x000000FF (byte 0xFF, the other 3 bytes irrelevant) produces
    // 0xFFFFFFFF (-1), the same result `i32.load8_s` would produce loading
    // that byte from memory. Rust's `as i8`/`as i16` truncate-then-sign-
    // extend on the subsequent `as i32`/`as i64` cast does exactly this in
    // one step, matching the spec's `signed_N(x mod 2^N)` definition.

    // i32.extend8_s (0xC0)
    vm.register_context_opcode(0xC0, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(a as i8 as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i32.extend16_s (0xC1)
    vm.register_context_opcode(0xC1, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(a as i16 as i32));
        vm.advance_pc();
        Ok(None)
    });

    // i64.extend8_s (0xC2)
    vm.register_context_opcode(0xC2, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a as i8 as i64));
        vm.advance_pc();
        Ok(None)
    });

    // i64.extend16_s (0xC3)
    vm.register_context_opcode(0xC3, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a as i16 as i64));
        vm.advance_pc();
        Ok(None)
    });

    // i64.extend32_s (0xC4)
    vm.register_context_opcode(0xC4, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(a as i32 as i64));
        vm.advance_pc();
        Ok(None)
    });
}

// ── Variable instructions ────────────────────────────────────────────────

fn register_variable(vm: &mut GenericVM) {
    // local.get (0x20)
    vm.register_context_opcode(0x20, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let index = operand_int(instr) as usize;
        push_wasm(vm, ctx.typed_locals[index]);
        vm.advance_pc();
        Ok(None)
    });

    // local.set (0x21)
    vm.register_context_opcode(0x21, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let index = operand_int(instr) as usize;
        ctx.typed_locals[index] = pop_wasm(vm)?;
        vm.advance_pc();
        Ok(None)
    });

    // local.tee (0x22)
    vm.register_context_opcode(0x22, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let index = operand_int(instr) as usize;
        ctx.typed_locals[index] = peek_wasm(vm)?;
        vm.advance_pc();
        Ok(None)
    });

    // global.get (0x23)
    vm.register_context_opcode(0x23, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let index = operand_int(instr) as usize;
        push_wasm(vm, ctx.globals[index]);
        vm.advance_pc();
        Ok(None)
    });

    // global.set (0x24)
    vm.register_context_opcode(0x24, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let index = operand_int(instr) as usize;
        ctx.globals[index] = pop_wasm(vm)?;
        vm.advance_pc();
        Ok(None)
    });

    // table.get (0x25 <tableidx>) — push table[tableidx][index] as a
    // funcref (WASM17). Thin wrapper around `Table::get`, same pattern
    // `call_indirect`'s table lookup already uses, but honoring the real
    // decoded table index instead of `call_indirect`'s hardcoded 0 (WASM
    // 1.0 has exactly one table either way, but the text form can name it
    // explicitly, so this resolves whichever index `wasm-wast-parser`
    // actually emitted).
    vm.register_context_opcode(0x25, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let table_idx = operand_int(instr) as usize;
        let elem_index = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let table = get_table(ctx, table_idx)?;
        let func_index = table.get(elem_index as u32).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::Ref(func_index));
        vm.advance_pc();
        Ok(None)
    });

    // table.set (0x26 <tableidx>) — pop a funcref and an i32 index, store
    // into table[tableidx][index] (WASM17). Thin wrapper around `Table::set`.
    vm.register_context_opcode(0x26, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let table_idx = operand_int(instr) as usize;
        let value = match pop_wasm(vm)? {
            WasmValue::Ref(v) => v,
            other => return Err(VMError::GenericError(format!("type mismatch: expected funcref, got {other:?}"))),
        };
        let elem_index = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let table = get_table(ctx, table_idx)?;
        table.set(elem_index as u32, value).map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });
}

// ── Parametric instructions ──────────────────────────────────────────────

fn register_parametric(vm: &mut GenericVM) {
    // drop (0x1A)
    vm.register_context_opcode(0x1A, |vm, _instr, _code, _ctx| {
        let _ = pop_wasm(vm)?;
        vm.advance_pc();
        Ok(None)
    });

    // select (0x1B)
    vm.register_context_opcode(0x1B, |vm, _instr, _code, _ctx| {
        let cond = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let val2 = pop_wasm(vm)?;
        let val1 = pop_wasm(vm)?;
        push_wasm(vm, if cond != 0 { val1 } else { val2 });
        vm.advance_pc();
        Ok(None)
    });
}

// ── Memory instructions ──────────────────────────────────────────────────

fn register_memory(vm: &mut GenericVM) {
    // Helper to compute effective address from memarg operand
    fn effective_addr(instr: &Instruction, base: i32) -> usize {
        let (mem_offset, _memidx) = unpack_memarg_operand(instr);
        (base as u32 as usize).wrapping_add(mem_offset as usize)
    }

    // Helper: the memarg's real memory index (task #92/W18) -- 0 unless
    // the multi-memory proposal's flags-bit 0x40 was set in the binary.
    fn memarg_memidx(instr: &Instruction) -> usize {
        unpack_memarg_operand(instr).1 as usize
    }

    // i32.load (0x28)
    vm.register_context_opcode(0x28, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let mem = get_memory_at(ctx, memarg_memidx(instr))?;
        let val = mem.load_i32(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });

    // i64.load (0x29)
    vm.register_context_opcode(0x29, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let mem = get_memory_at(ctx, memarg_memidx(instr))?;
        let val = mem.load_i64(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });

    // f32.load (0x2A)
    vm.register_context_opcode(0x2A, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let mem = get_memory_at(ctx, memarg_memidx(instr))?;
        let val = mem.load_f32(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F32(val));
        vm.advance_pc();
        Ok(None)
    });

    // f64.load (0x2B)
    vm.register_context_opcode(0x2B, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let mem = get_memory_at(ctx, memarg_memidx(instr))?;
        let val = mem.load_f64(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::F64(val));
        vm.advance_pc();
        Ok(None)
    });

    // i32.load8_s (0x2C)
    vm.register_context_opcode(0x2C, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i32_8s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });
    // i32.load8_u (0x2D)
    vm.register_context_opcode(0x2D, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i32_8u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });
    // i32.load16_s (0x2E)
    vm.register_context_opcode(0x2E, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i32_16s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });
    // i32.load16_u (0x2F)
    vm.register_context_opcode(0x2F, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i32_16u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });

    // i64.load8_s (0x30)
    vm.register_context_opcode(0x30, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i64_8s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load8_u (0x31)
    vm.register_context_opcode(0x31, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i64_8u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load16_s (0x32)
    vm.register_context_opcode(0x32, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i64_16s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load16_u (0x33)
    vm.register_context_opcode(0x33, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i64_16u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load32_s (0x34)
    vm.register_context_opcode(0x34, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i64_32s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load32_u (0x35)
    vm.register_context_opcode(0x35, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory_at(ctx, memarg_memidx(instr))?.load_i64_32u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });

    // Full-width stores: 0x36-0x39
    vm.register_context_opcode(0x36, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_i32(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0x37, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_i64(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0x38, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_f32(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0x39, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_f64().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_f64(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });

    // Narrow stores for i32: 0x3A (8-bit), 0x3B (16-bit)
    vm.register_context_opcode(0x3A, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_i32_8(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0x3B, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_i32_16(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });

    // Narrow stores for i64: 0x3C (8-bit), 0x3D (16-bit), 0x3E (32-bit)
    vm.register_context_opcode(0x3C, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_i64_8(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0x3D, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_i64_16(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });
    vm.register_context_opcode(0x3E, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let val = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        get_memory_at(ctx, memarg_memidx(instr))?
            .store_i64_32(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });

    // memory.size (0x3F) -- `instr.operand` carries the real memory index
    // (multi-memory, W16, task #85); `convert_operand` already maps the
    // decoded `memidx` LEB128 straight through to `Operand::Index`, so
    // this only needed to start reading it instead of ignoring it.
    vm.register_context_opcode(0x3F, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let memidx = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => 0,
        };
        let size = match ctx.memories.get(memidx) {
            Some(&ptr) => unsafe { (*ptr).size() as i32 },
            None => 0,
        };
        push_wasm(vm, WasmValue::I32(size));
        vm.advance_pc();
        Ok(None)
    });

    // memory.grow (0x40) -- same memidx plumbing as memory.size above.
    vm.register_context_opcode(0x40, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let memidx = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => 0,
        };
        let delta = pop_wasm(vm)?.as_i32().map_err(VMError::from)? as u32;
        // Security review follow-up (task #101, mirroring `table.grow`'s
        // own task #98 round-2 fix): `LinearMemory::grow`'s per-memory
        // 65536-page cap alone still permits an AGGREGATE DoS across
        // `MAX_MEMORIES` (64) memories -- each individually grown to
        // 65536 pages (4GB) would total ~256GB from one small module.
        // `wasm-validator`'s own "Check 1b" already caps the SUM of every
        // memory's DECLARED minimum at 65536 pages total (the real
        // single-memory spec max, generalized across however many
        // memories a module declares); this reuses that exact same
        // `MAX_TOTAL_MEMORY_PAGES` bound at RUNTIME, so growth can't
        // reintroduce what Check 1b already closes at declare-time.
        // `memory_grow_would_exceed_aggregate_cap` (see its own doc
        // comment) does the actual arithmetic, kept as a pure function so
        // it's cheaply unit-testable without allocating anywhere near
        // 4GB of real backing memory just to exercise the threshold.
        let result = if memidx >= ctx.memories.len() {
            -1
        } else {
            let current_pages: Vec<u32> = ctx.memories.iter().map(|&ptr| unsafe { (*ptr).size() }).collect();
            if memory_grow_would_exceed_aggregate_cap(&current_pages, memidx, delta) {
                -1
            } else {
                unsafe { (*ctx.memories[memidx]).grow(delta) }
            }
        };
        push_wasm(vm, WasmValue::I32(result));
        vm.advance_pc();
        Ok(None)
    });
}

// ── Atomic memory operations (0xFE prefix, threads proposal — WASM18) ────
//
// Every atomic op here is a plain, unsynchronized memory access -- with
// one native thread of execution, nothing else can ever observe a
// location mid-operation, so a read-modify-write done as an ordinary
// Rust read, then compute, then write, in one uninterrupted native call
// already satisfies the spec's atomicity guarantee. See `code/specs/
// W09-wasm-atomics-plain.md`'s own "Why every atomic op is trivially
// atomic here" section for the full reasoning. `memory.atomic.notify`/
// `wait32`/`wait64` are not registered at all -- they need a second real
// thread to make sense, which `wasm_opcodes::ATOMIC_OPS` doesn't even
// list (see that table's own doc comment).

/// Read a value of the shape `op` describes (its `value_type` +
/// `natural_align` byte width) from `mem` at `addr` -- the same set of
/// per-width `LinearMemory` methods the plain MVP load family already
/// uses, just selected generically by width instead of one opcode
/// handler per width.
fn atomic_mem_load(mem: &LinearMemory, op: &wasm_opcodes::AtomicOpInfo, addr: usize) -> Result<WasmValue, TrapError> {
    use wasm_types::ValueType;
    match (op.value_type, op.natural_align) {
        (Some(ValueType::I32), 4) => Ok(WasmValue::I32(mem.load_i32(addr)?)),
        (Some(ValueType::I32), 2) => Ok(WasmValue::I32(mem.load_i32_16u(addr)?)),
        (Some(ValueType::I32), 1) => Ok(WasmValue::I32(mem.load_i32_8u(addr)?)),
        (Some(ValueType::I64), 8) => Ok(WasmValue::I64(mem.load_i64(addr)?)),
        (Some(ValueType::I64), 4) => Ok(WasmValue::I64(mem.load_i64_32u(addr)?)),
        (Some(ValueType::I64), 2) => Ok(WasmValue::I64(mem.load_i64_16u(addr)?)),
        (Some(ValueType::I64), 1) => Ok(WasmValue::I64(mem.load_i64_8u(addr)?)),
        _ => Err(TrapError::new(format!("unsupported atomic load shape for {}", op.name))),
    }
}

/// Write `value` at `addr`, using the width `op` describes -- the store
/// counterpart to `atomic_mem_load`.
fn atomic_mem_store(mem: &mut LinearMemory, op: &wasm_opcodes::AtomicOpInfo, addr: usize, value: WasmValue) -> Result<(), TrapError> {
    use wasm_types::ValueType;
    match (op.value_type, op.natural_align) {
        (Some(ValueType::I32), 4) => mem.store_i32(addr, value.as_i32()?),
        (Some(ValueType::I32), 2) => mem.store_i32_16(addr, value.as_i32()?),
        (Some(ValueType::I32), 1) => mem.store_i32_8(addr, value.as_i32()?),
        (Some(ValueType::I64), 8) => mem.store_i64(addr, value.as_i64()?),
        (Some(ValueType::I64), 4) => mem.store_i64_32(addr, value.as_i64()?),
        (Some(ValueType::I64), 2) => mem.store_i64_16(addr, value.as_i64()?),
        (Some(ValueType::I64), 1) => mem.store_i64_8(addr, value.as_i64()?),
        _ => Err(TrapError::new(format!("unsupported atomic store shape for {}", op.name))),
    }
}

/// Apply an RMW operator to `(old, operand)`, both already the same
/// width/signedness this op's `atomic_mem_load`/`atomic_mem_store` calls
/// use. The specific operator (add/sub/and/or/xor/xchg) is the LAST
/// dot-segment of the op's name with any `_u` suffix trimmed --
/// `"i32.atomic.rmw.add"` -> `"add"`, `"i32.atomic.rmw8.add_u"` ->
/// `"add"` -- since `wasm_opcodes::ATOMIC_OPS` already spells this out in
/// the name and duplicating it as a second enum would just be the same
/// information twice.
fn apply_rmw_op(op_name: &str, old: WasmValue, operand: WasmValue) -> Result<WasmValue, TrapError> {
    let operator = op_name.rsplit('.').next().unwrap_or("").trim_end_matches("_u");
    match (old, operand) {
        (WasmValue::I32(a), WasmValue::I32(b)) => {
            let result = match operator {
                "add" => a.wrapping_add(b),
                "sub" => a.wrapping_sub(b),
                "and" => a & b,
                "or" => a | b,
                "xor" => a ^ b,
                "xchg" => b,
                other => return Err(TrapError::new(format!("unknown atomic RMW operator: {other}"))),
            };
            Ok(WasmValue::I32(result))
        }
        (WasmValue::I64(a), WasmValue::I64(b)) => {
            let result = match operator {
                "add" => a.wrapping_add(b),
                "sub" => a.wrapping_sub(b),
                "and" => a & b,
                "or" => a | b,
                "xor" => a ^ b,
                "xchg" => b,
                other => return Err(TrapError::new(format!("unknown atomic RMW operator: {other}"))),
            };
            Ok(WasmValue::I64(result))
        }
        _ => Err(TrapError::new("atomic RMW type mismatch")),
    }
}

fn register_atomics(vm: &mut GenericVM) {
    // Effective address = i32 base (popped from the stack) + the memarg
    // offset decoded onto the instruction -- identical to `register_memory`'s
    // own `effective_addr`, just reading the packed `(sub, offset)` this
    // prefix's decoder built instead of a bare `MemArg`.
    fn effective_addr(offset_imm: u32, base: i32) -> usize {
        (base as u32 as usize).wrapping_add(offset_imm as usize)
    }

    // The real spec requires atomic instructions to trap at RUNTIME if
    // the EFFECTIVE address isn't a multiple of the operation's natural
    // alignment -- distinct from (and in addition to) `wasm-validator`'s
    // check that the DECLARED `align=` immediate matches natural
    // alignment exactly. The declared immediate is a static property of
    // the bytecode; the effective address is `base + offset`, a runtime
    // value the validator can't know in advance (e.g. `base` might come
    // from a `local.get`). Confirmed against the real, pinned-commit
    // `atomic.wast` testsuite file's own `"unaligned atomic"` trap
    // assertions (e.g. `i32.atomic.load` at address 1, not a multiple of
    // its natural 4-byte alignment).
    fn check_atomic_alignment(align: u32, addr: usize) -> Result<(), TrapError> {
        if align > 0 && !addr.is_multiple_of(align as usize) {
            return Err(TrapError::new("unaligned atomic"));
        }
        Ok(())
    }

    vm.register_context_opcode(0xFE, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let (sub, offset_imm) = unpack_atomic_operand(instr);
        let op = wasm_opcodes::get_atomic_op(sub)
            .ok_or_else(|| VMError::GenericError(format!("unknown atomic sub-opcode {sub:#04x}")))?;

        use wasm_opcodes::AtomicOpKind;
        match op.kind {
            AtomicOpKind::Fence => {
                // A true no-op with one native thread -- nothing to
                // order, no memory or stack effect at all.
            }
            AtomicOpKind::Load => {
                let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let addr = effective_addr(offset_imm, base);
                check_atomic_alignment(op.natural_align, addr).map_err(VMError::from)?;
                let mem = get_memory(ctx)?;
                let val = atomic_mem_load(mem, op, addr).map_err(VMError::from)?;
                push_wasm(vm, val);
            }
            AtomicOpKind::Store => {
                let value = pop_wasm(vm)?;
                let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let addr = effective_addr(offset_imm, base);
                check_atomic_alignment(op.natural_align, addr).map_err(VMError::from)?;
                let mem = get_memory(ctx)?;
                atomic_mem_store(mem, op, addr, value).map_err(VMError::from)?;
            }
            AtomicOpKind::Rmw => {
                let operand = pop_wasm(vm)?;
                let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let addr = effective_addr(offset_imm, base);
                check_atomic_alignment(op.natural_align, addr).map_err(VMError::from)?;
                let mem = get_memory(ctx)?;
                let old = atomic_mem_load(mem, op, addr).map_err(VMError::from)?;
                let new_val = apply_rmw_op(op.name, old, operand).map_err(VMError::from)?;
                atomic_mem_store(mem, op, addr, new_val).map_err(VMError::from)?;
                push_wasm(vm, old);
            }
            AtomicOpKind::Cmpxchg => {
                let replacement = pop_wasm(vm)?;
                let expected = pop_wasm(vm)?;
                let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let addr = effective_addr(offset_imm, base);
                check_atomic_alignment(op.natural_align, addr).map_err(VMError::from)?;
                let mem = get_memory(ctx)?;
                let old = atomic_mem_load(mem, op, addr).map_err(VMError::from)?;
                if old == expected {
                    atomic_mem_store(mem, op, addr, replacement).map_err(VMError::from)?;
                }
                push_wasm(vm, old);
            }
            AtomicOpKind::Notify => {
                // memory.atomic.notify: with one native thread, no other
                // agent is EVER blocked in `wait` on this address, so the
                // real, deterministic answer is always "0 woken" -- not a
                // stand-in for unimplemented behavior (see
                // `wasm_opcodes::AtomicOpKind::Notify`'s own doc
                // comment). The address is still bounds-checked (a real
                // `i32.atomic.load`-shaped access), even though its value
                // is unused, since the spec still requires the address be
                // valid for `notify` to succeed.
                let _count = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let addr = effective_addr(offset_imm, base);
                check_atomic_alignment(op.natural_align, addr).map_err(VMError::from)?;
                get_memory(ctx)?.load_i32(addr).map_err(VMError::from)?;
                push_wasm(vm, WasmValue::I32(0));
            }
            AtomicOpKind::Wait => {
                // memory.atomic.wait32/wait64: with one native thread, no
                // other agent can ever `notify` this wait, so the only
                // two real outcomes are "not-equal" (1, when the current
                // value already differs from `expected` -- no wait
                // needed at all) and "timed-out" (2, when it matches --
                // nothing will ever wake it). "ok" (0, woken by a real
                // notify) can never happen here. See `wasm_opcodes::
                // AtomicOpKind::Wait`'s own doc comment.
                let _timeout = pop_wasm(vm)?.as_i64().map_err(VMError::from)?;
                let expected = pop_wasm(vm)?;
                let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let addr = effective_addr(offset_imm, base);
                check_atomic_alignment(op.natural_align, addr).map_err(VMError::from)?;
                let mem = get_memory(ctx)?;
                let current = atomic_mem_load(mem, op, addr).map_err(VMError::from)?;
                let result = if current == expected { 2 } else { 1 };
                push_wasm(vm, WasmValue::I32(result));
            }
        }
        vm.advance_pc();
        Ok(None)
    });
}

// ── SIMD (v128) instructions, 0xFD prefix -- see code/specs/
// W13-wasm-simd-v128-first-slice.md for the design, this first slice's
// exact scope, and where each opcode's sub-opcode value was verified ────

/// Push a new v128 value onto `ctx.v128_heap`, enforcing
/// `MAX_V128_HEAP_LEN` (security review) -- every SIMD opcode handler
/// that produces a new v128 must go through this, not `ctx.v128_heap
/// .push(...)` directly, so the bound is enforced uniformly.
fn push_v128(ctx: &mut WasmExecutionContext, bytes: [u8; 16]) -> Result<u32, VMError> {
    if ctx.v128_heap.len() >= MAX_V128_HEAP_LEN {
        return Err(VMError::GenericError(
            "v128 heap limit exceeded (too many SIMD values created in one call)".into(),
        ));
    }
    let handle = ctx.v128_heap.len() as u32;
    ctx.v128_heap.push(bytes);
    Ok(handle)
}

fn register_simd(vm: &mut GenericVM) {
    vm.register_context_opcode(0xFD, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let (sub_opcode, aux) = unpack_simd_operand(instr);

        if sub_opcode == 0x0C {
            // v128.const: push the const-pool literal as a brand-new
            // v128_heap entry. (Not deduplicated against an identical
            // earlier literal -- a cheap future optimization, not a
            // correctness requirement for this first slice.)
            let bytes = *ctx
                .simd_consts
                .get(aux)
                .ok_or_else(|| VMError::GenericError("v128.const: const-pool index out of range".into()))?;
            let handle = push_v128(ctx, bytes)?;
            push_wasm(vm, WasmValue::V128(handle));
            vm.advance_pc();
            return Ok(None);
        }

        let op = wasm_opcodes::get_simd_op(sub_opcode)
            .ok_or_else(|| VMError::GenericError(format!("unknown SIMD sub-opcode {sub_opcode:#x}")))?;

        use wasm_opcodes::SimdOpKind;
        match op.kind {
            SimdOpKind::Const => unreachable!("v128.const handled above, before this lookup"),
            SimdOpKind::ExtractLane => {
                // i32x4.extract_lane: pop a v128, read the `aux`-selected
                // lane back out as a plain i32 -- the only opcode in this
                // slice that produces a scalar, not another v128.
                let handle = pop_wasm(vm)?.as_v128_handle().map_err(VMError::from)?;
                let bytes = *ctx
                    .v128_heap
                    .get(handle as usize)
                    .ok_or_else(|| VMError::GenericError("v128 operand: heap handle out of range".into()))?;
                let lane_idx = aux;
                if lane_idx >= 4 {
                    return Err(VMError::GenericError(format!(
                        "i32x4.extract_lane: lane index {lane_idx} out of range (must be 0-3)"
                    )));
                }
                let value = i32::from_le_bytes(bytes[lane_idx * 4..lane_idx * 4 + 4].try_into().unwrap());
                push_wasm(vm, WasmValue::I32(value));
            }
            SimdOpKind::Splat => {
                // i32x4.splat: pop one i32, broadcast its 4 little-endian
                // bytes into all 4 lanes of a new v128.
                let scalar = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
                let lane = scalar.to_le_bytes();
                let mut bytes = [0u8; 16];
                for i in 0..4 {
                    bytes[i * 4..i * 4 + 4].copy_from_slice(&lane);
                }
                let handle = push_v128(ctx, bytes)?;
                push_wasm(vm, WasmValue::V128(handle));
            }
            SimdOpKind::Add
            | SimdOpKind::Sub
            | SimdOpKind::Mul
            | SimdOpKind::Eq
            | SimdOpKind::Ne
            | SimdOpKind::LtS
            | SimdOpKind::LtU
            | SimdOpKind::GtS
            | SimdOpKind::GtU
            | SimdOpKind::LeS
            | SimdOpKind::LeU
            | SimdOpKind::GeS
            | SimdOpKind::GeU
            | SimdOpKind::MinS
            | SimdOpKind::MinU
            | SimdOpKind::MaxS
            | SimdOpKind::MaxU => {
                // All lane-wise BINARY ops over 4 i32 lanes, popped in
                // WASM's usual (lhs pushed first, rhs second) order --
                // rhs is on top of the stack.
                let rhs_handle = pop_wasm(vm)?.as_v128_handle().map_err(VMError::from)?;
                let lhs_handle = pop_wasm(vm)?.as_v128_handle().map_err(VMError::from)?;
                let rhs = *ctx
                    .v128_heap
                    .get(rhs_handle as usize)
                    .ok_or_else(|| VMError::GenericError("v128 operand: heap handle out of range".into()))?;
                let lhs = *ctx
                    .v128_heap
                    .get(lhs_handle as usize)
                    .ok_or_else(|| VMError::GenericError("v128 operand: heap handle out of range".into()))?;

                // WASM SIMD comparisons produce a boolean MASK per lane:
                // all-1s (-1) if true, all-0s (0) if false -- not a plain
                // 0/1 i32 the way a scalar comparison does.
                let mask = |b: bool| if b { -1i32 } else { 0i32 };

                let mut result = [0u8; 16];
                for i in 0..4 {
                    let l = i32::from_le_bytes(lhs[i * 4..i * 4 + 4].try_into().unwrap());
                    let r = i32::from_le_bytes(rhs[i * 4..i * 4 + 4].try_into().unwrap());
                    let out = match op.kind {
                        SimdOpKind::Add => l.wrapping_add(r),
                        SimdOpKind::Sub => l.wrapping_sub(r),
                        SimdOpKind::Mul => l.wrapping_mul(r),
                        SimdOpKind::Eq => mask(l == r),
                        SimdOpKind::Ne => mask(l != r),
                        SimdOpKind::LtS => mask(l < r),
                        SimdOpKind::LtU => mask((l as u32) < (r as u32)),
                        SimdOpKind::GtS => mask(l > r),
                        SimdOpKind::GtU => mask((l as u32) > (r as u32)),
                        SimdOpKind::LeS => mask(l <= r),
                        SimdOpKind::LeU => mask((l as u32) <= (r as u32)),
                        SimdOpKind::GeS => mask(l >= r),
                        SimdOpKind::GeU => mask((l as u32) >= (r as u32)),
                        SimdOpKind::MinS => l.min(r),
                        SimdOpKind::MinU => ((l as u32).min(r as u32)) as i32,
                        SimdOpKind::MaxS => l.max(r),
                        SimdOpKind::MaxU => ((l as u32).max(r as u32)) as i32,
                        _ => unreachable!("only the binary lane-wise kinds listed in this arm's pattern reach here"),
                    };
                    result[i * 4..i * 4 + 4].copy_from_slice(&out.to_le_bytes());
                }

                let handle = push_v128(ctx, result)?;
                push_wasm(vm, WasmValue::V128(handle));
            }
            SimdOpKind::Neg | SimdOpKind::Abs => {
                // i32x4.neg/i32x4.abs: UNARY, unlike every kind in the arm
                // above -- pops exactly ONE v128, transforms each lane,
                // pushes one.
                let handle = pop_wasm(vm)?.as_v128_handle().map_err(VMError::from)?;
                let bytes = *ctx
                    .v128_heap
                    .get(handle as usize)
                    .ok_or_else(|| VMError::GenericError("v128 operand: heap handle out of range".into()))?;
                let mut result = [0u8; 16];
                for i in 0..4 {
                    let v = i32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
                    let out = match op.kind {
                        SimdOpKind::Neg => v.wrapping_neg(),
                        SimdOpKind::Abs => v.wrapping_abs(),
                        _ => unreachable!("only Neg/Abs reach this arm"),
                    };
                    result[i * 4..i * 4 + 4].copy_from_slice(&out.to_le_bytes());
                }
                let handle = push_v128(ctx, result)?;
                push_wasm(vm, WasmValue::V128(handle));
            }
        }

        vm.advance_pc();
        Ok(None)
    });
}

// ── Control flow instructions ────────────────────────────────────────────

fn register_control(vm: &mut GenericVM) {
    // unreachable (0x00)
    vm.register_context_opcode(0x00, |_vm, _instr, _code, _ctx| {
        Err(VMError::GenericError(
            "unreachable instruction executed".into(),
        ))
    });

    // nop (0x01)
    vm.register_context_opcode(0x01, |vm, _instr, _code, _ctx| {
        vm.advance_pc();
        Ok(None)
    });

    // block (0x02)
    vm.register_context_opcode(0x02, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let block_type = operand_int(instr);
        let (param_arity, arity) = block_arity(block_type, &ctx.types);
        let end_pc = ctx
            .control_flow_map
            .get(&vm.pc)
            .map(|t| t.end_pc)
            .unwrap_or(vm.pc + 1);
        ctx.label_stack.push(Label {
            arity,
            param_arity,
            target_pc: end_pc,
            stack_height: vm.typed_stack.len(),
            is_loop: false,
        });
        vm.advance_pc();
        Ok(None)
    });

    // loop (0x03)
    vm.register_context_opcode(0x03, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let block_type = operand_int(instr);
        let (param_arity, arity) = block_arity(block_type, &ctx.types);
        let loop_pc = vm.pc;
        ctx.label_stack.push(Label {
            arity,
            param_arity,
            target_pc: loop_pc, // loops branch backward
            stack_height: vm.typed_stack.len(),
            is_loop: true,
        });
        vm.advance_pc();
        Ok(None)
    });

    // if (0x04)
    vm.register_context_opcode(0x04, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let block_type = operand_int(instr);
        let (param_arity, arity) = block_arity(block_type, &ctx.types);
        let condition = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let target = ctx.control_flow_map.get(&vm.pc).cloned();
        let end_pc = target.as_ref().map(|t| t.end_pc).unwrap_or(vm.pc + 1);
        let else_pc = target.as_ref().and_then(|t| t.else_pc);

        ctx.label_stack.push(Label {
            arity,
            param_arity,
            target_pc: end_pc,
            stack_height: vm.typed_stack.len(),
            is_loop: false,
        });
        if condition != 0 {
            vm.advance_pc();
        } else {
            match else_pc {
                Some(ep) => vm.jump_to(ep + 1),
                None => vm.jump_to(end_pc),
            }
        }
        Ok(None)
    });

    // else (0x05)
    vm.register_context_opcode(0x05, |vm, _instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let label = ctx.label_stack.last().expect("else without label");
        let target = label.target_pc;
        vm.jump_to(target);
        Ok(None)
    });

    // end (0x0B)
    vm.register_context_opcode(0x0B, |vm, _instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        if !ctx.label_stack.is_empty() {
            ctx.label_stack.pop();
            vm.advance_pc();
        } else {
            // End of function.
            ctx.returned = true;
            vm.halted = true;
        }
        Ok(None)
    });

    // br (0x0C)
    vm.register_context_opcode(0x0C, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let label_index = operand_int(instr) as usize;
        execute_branch(vm, ctx, label_index)?;
        Ok(None)
    });

    // br_if (0x0D)
    vm.register_context_opcode(0x0D, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let label_index = operand_int(instr) as usize;
        let condition = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        if condition != 0 {
            execute_branch(vm, ctx, label_index)?;
        } else {
            vm.advance_pc();
        }
        Ok(None)
    });

    // br_table (0x0E)
    //
    // The dispatch-loop pattern emitted by iir-to-wasm looks like:
    //
    //   loop                          ← outer loop label (depth = N from inside)
    //     block ... (N blocks)
    //       local.get $dispatch
    //       br_table 0 1 … N-1  N    ← labels[i] → break out of block i
    //                                   default  N → jump back to loop (infinite)
    //     end ... end end
    //
    // The operand stored in the `Instruction` is the *index* into
    // `ctx.br_table_targets`, a per-function Vec populated when the callee's
    // instructions are built.  Each entry is `[l0, l1, ..., l_{n-1}, default]`.
    vm.register_context_opcode(0x0E, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        // Pop the selector from the stack.
        let index = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        // Look up the full label table for this specific br_table instruction.
        let table_idx = operand_int(instr) as usize;
        let targets = ctx
            .br_table_targets
            .get(table_idx)
            .ok_or_else(|| VMError::GenericError(format!("br_table targets[{}] missing", table_idx)))?;
        // `targets` = [l0, l1, ..., l_{n-1}, default_label]
        // If index is in-bounds for the non-default entries, use targets[index];
        // otherwise fall through to the default (last entry).
        let n = targets.len().saturating_sub(1); // number of non-default labels
        let depth = if index >= 0 && (index as usize) < n {
            targets[index as usize] as usize
        } else {
            *targets.last().unwrap_or(&0) as usize
        };
        execute_branch(vm, ctx, depth)?;
        Ok(None)
    });

    // return (0x0F)
    vm.register_context_opcode(0x0F, |vm, _instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        ctx.returned = true;
        vm.halted = true;
        Ok(None)
    });

    // call (0x10)
    vm.register_context_opcode(0x10, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let func_index = operand_int(instr) as usize;
        call_function(vm, ctx, func_index)?;
        Ok(None)
    });

    // call_indirect (0x11)
    vm.register_context_opcode(0x11, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let (type_idx, table_idx) = unpack_call_indirect_operand(instr);
        let type_idx = type_idx as usize;
        let elem_index = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let table = get_table(ctx, table_idx as usize)?;
        let func_index = table
            .get(elem_index as u32)
            .map_err(VMError::from)?
            .ok_or_else(|| VMError::GenericError("uninitialized table element".into()))?;

        // Type check: `type_idx` indexes the module's TYPE SECTION (what the
        // call site declared), which is a different index space from
        // `func_types` (indexed by FUNCTION index — one entry per function,
        // resolved to whichever type that function happens to declare).
        // Comparing against `func_types[type_idx]` would check the callee
        // against an unrelated function's type instead of the declared one,
        // so this needs `ctx.types` (the real type section) specifically —
        // see `WasmExecutionEngine::set_type_section`'s doc comment. If the
        // embedder never set it, there's nothing to check against; skip
        // rather than fail closed on missing type info the caller never
        // promised to provide.
        //
        // Security review (WASM16, follow-up): `func_index` comes from
        // `table.get(elem_index)` -- DATA, not a static part of the
        // bytecode a validator necessarily already checked (this crate's
        // own tests construct engines that skip `wasm-validator`
        // entirely). A direct `ctx.func_types[func_index]` index here
        // panicked on an out-of-range table entry whenever a type section
        // was set (the common case); `.get()` makes it a clean trap
        // instead, matching `return_call_indirect` (0x13)'s identical fix.
        if let Some(expected) = ctx.types.get(type_idx) {
            let actual = ctx.func_types.get(func_index as usize).ok_or_else(|| {
                VMError::GenericError(
                    "indirect call: table entry references an undefined function".into(),
                )
            })?;
            if expected.params != actual.params || expected.results != actual.results {
                return Err(VMError::GenericError("indirect call type mismatch".into()));
            }
        }

        call_function(vm, ctx, func_index as usize)?;
        Ok(None)
    });

    // return_call (0x12, WASM16): tail call. Pops the same args an
    // ordinary `call` would, but does NOT recurse -- signals
    // `call_function_inner`'s outer loop to replace the CURRENT frame
    // instead of pushing a new one (no `SavedFrame`, no `call_depth`
    // increment: this is what makes an unbounded tail-call chain run in
    // genuinely constant Rust-stack space). See `pending_tail_call`'s
    // own doc comment.
    vm.register_context_opcode(0x12, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let func_index = operand_int(instr) as usize;
        let func_type = ctx
            .func_types
            .get(func_index)
            .ok_or_else(|| VMError::GenericError(format!("undefined function {func_index}")))?
            .clone();
        let mut args = Vec::with_capacity(func_type.params.len());
        for _ in 0..func_type.params.len() {
            args.push(pop_wasm(vm)?);
        }
        args.reverse();
        ctx.pending_tail_call = Some((func_index, args));
        vm.halted = true;
        Ok(None)
    });

    // return_call_indirect (0x13, WASM16): same table-lookup + type
    // check as call_indirect, then the same tail-call signal as
    // return_call above.
    vm.register_context_opcode(0x13, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let (type_idx, table_idx) = unpack_call_indirect_operand(instr);
        let type_idx = type_idx as usize;
        let elem_index = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let table = get_table(ctx, table_idx as usize)?;
        let func_index = table
            .get(elem_index as u32)
            .map_err(VMError::from)?
            .ok_or_else(|| VMError::GenericError("uninitialized table element".into()))?;

        let func_index = func_index as usize;
        // Security review (WASM16): a table entry -- unlike the
        // instruction's own `funcidx` immediate `return_call`'s (0x12)
        // handler bounds-checks via `.get()` -- is DATA, not a static
        // part of the bytecode a validator necessarily already checked
        // (this crate's own tests construct engines that skip
        // `wasm-validator` entirely, e.g. `wasm16_tail_calls.rs`'s
        // `engine_from_wat`). `.get()` here -- this call site is
        // unconditional, so it must never panic on an out-of-range table
        // entry. `call_indirect` (0x11) has the identical `.get()` fix on
        // its own equivalent lookup.
        let func_type = ctx
            .func_types
            .get(func_index)
            .ok_or_else(|| VMError::GenericError("indirect call: table entry references an undefined function".into()))?
            .clone();

        if let Some(expected) = ctx.types.get(type_idx) {
            if expected.params != func_type.params || expected.results != func_type.results {
                return Err(VMError::GenericError("indirect call type mismatch".into()));
            }
        }

        let mut args = Vec::with_capacity(func_type.params.len());
        for _ in 0..func_type.params.len() {
            args.push(pop_wasm(vm)?);
        }
        args.reverse();
        ctx.pending_tail_call = Some((func_index, args));
        vm.halted = true;
        Ok(None)
    });
}

/// Execute a function call within the WASM execution context.
///
/// Thin wrapper around [`call_function_inner`] enforcing [`MAX_CALL_DEPTH`]
/// — see that constant's doc comment for why this can't be left unguarded.
/// `ctx.call_depth` is incremented/decremented symmetrically around the
/// inner call regardless of whether it succeeds or traps: an `Err` here
/// unwinds the ENTIRE top-level `WasmExecutionEngine::call_function`
/// invocation (nothing catches it and resumes execution partway through),
/// which always starts the next top-level call with a freshly constructed
/// `WasmExecutionContext` (`call_depth: 0`) — so a stale count from an
/// aborted call can never leak into a later, unrelated one.
fn call_function(
    vm: &mut GenericVM,
    ctx: &mut WasmExecutionContext,
    func_index: usize,
) -> VMResult<()> {
    if ctx.call_depth >= MAX_CALL_DEPTH {
        return Err(VMError::GenericError("call stack exhausted".to_string()));
    }
    ctx.call_depth += 1;
    let result = call_function_inner(vm, ctx, func_index);
    ctx.call_depth -= 1;
    result
}

fn call_function_inner(
    vm: &mut GenericVM,
    ctx: &mut WasmExecutionContext,
    func_index: usize,
) -> VMResult<()> {
    // Captured ONCE, before any tail-call transition: where this ENTIRE
    // `call_function_inner` invocation should eventually hand control
    // back to, once it's completely done (whether that's after zero,
    // one, or an unbounded chain of `return_call`/`return_call_indirect`
    // transitions -- see the WASM16 loop below). A tail call replaces
    // WHICH function is currently executing; it never changes where the
    // whole invocation ultimately returns to, so this must not be
    // recomputed from `vm.pc` on a later iteration (by then `vm.pc` has
    // been overwritten to point into whichever callee is currently
    // running).
    let saved_pc = vm.pc;

    // Whether THIS invocation pushed a `SavedFrame` -- distinct from
    // whether `ctx.saved_frames` is non-empty, which reflects every
    // OTHER still-in-flight nested call too. Only ever set once, on the
    // first iteration that turns out to be a module-defined function
    // (never for a pure host-function call, matching the pre-WASM16
    // behavior exactly): guards the "restore caller state" step below
    // so a host function reached via a tail call doesn't wrongly pop an
    // ENCLOSING caller's frame that doesn't belong to this invocation.
    let mut frame_pushed = false;

    let mut current_func_index = func_index;
    // `None` on the first iteration (pop args from the VM stack, same
    // as always); `Some` after a `return_call`/`return_call_indirect`
    // transition -- that handler already popped and prepared them (see
    // `WasmExecutionContext::pending_tail_call`'s own doc comment).
    let mut pending_args: Option<Vec<WasmValue>> = None;

    // WASM16: this loop is what makes an unbounded `return_call` chain
    // run in genuinely constant Rust-stack space -- a tail call
    // `continue`s the SAME loop iteration inside the SAME Rust stack
    // frame, never recursing through `call_function`/`call_function_inner`
    // again (that's `call`/`call_indirect`'s job, and `MAX_CALL_DEPTH`
    // still guards exactly that path, untouched by this change).
    let return_arity = loop {
        // GC checkpoint (W04): every call/call_indirect/return_call
        // (nested, recursive, or a tail-call loop iteration) routes
        // through this one shared helper, so this single call site is
        // the "safepoint at calls" chokepoint for both of this crate's
        // independent dispatch loops — see gc::maybe_collect.
        gc::maybe_collect(vm, ctx);

        let func_type = ctx
            .func_types
            .get(current_func_index)
            .ok_or_else(|| VMError::GenericError(format!("undefined function {}", current_func_index)))?
            .clone();

        // Pop arguments.
        let args = match pending_args.take() {
            Some(args) => args,
            None => {
                let mut args = Vec::new();
                for _ in 0..func_type.params.len() {
                    args.push(pop_wasm(vm)?);
                }
                args.reverse();
                args
            }
        };

        // Check for host function. A tail call landing on a host import
        // is still a leaf call (no further WASM frames to run), so it's
        // handled the same way an ordinary call to a host function
        // always has been: invoke it, push its results, and fall
        // through to the shared "restore caller state" tail below --
        // `frame_pushed` (false unless an EARLIER iteration already
        // pushed one) makes that tail behave exactly like the original
        // single-pass `vm.advance_pc()` shortcut when this is reached
        // on the very first iteration.
        if let Some(Some(host_func)) = ctx.host_functions.get(current_func_index) {
            let results = host_func
                .call(&args, ctx.memories.first().map(|&ptr| unsafe { &mut *ptr }))
                .map_err(VMError::from)?;
            for r in results {
                push_wasm(vm, r);
            }
            break func_type.results.len();
        }

        // Module-defined function.
        let body = ctx
            .func_bodies
            .get(current_func_index)
            .and_then(|b| b.as_ref())
            .ok_or_else(|| VMError::GenericError(format!("no body for function {}", current_func_index)))?
            .clone();

        // Save caller state (including br_table targets so the callee can
        // install its own without clobbering the caller's dispatch
        // table) -- but only ONCE per invocation: a tail-call transition
        // reuses the CURRENT frame, it doesn't grow the logical call
        // stack, so nothing new gets pushed here on iteration 2+.
        if !frame_pushed {
            ctx.saved_frames.push(SavedFrame {
                locals: ctx.typed_locals.clone(),
                label_stack: ctx.label_stack.clone(),
                stack_height: vm.typed_stack.len(),
                control_flow_map: ctx.control_flow_map.clone(),
                return_pc: saved_pc + 1,
                return_arity: func_type.results.len(),
                br_table_targets: std::mem::take(&mut ctx.br_table_targets),
                gc_ops: std::mem::take(&mut ctx.gc_ops),
                simd_consts: std::mem::take(&mut ctx.simd_consts),
            });
            frame_pushed = true;
        }

        // Initialize callee locals.
        let mut locals: Vec<WasmValue> = args;
        for t in &body.locals {
            locals.push(WasmValue::default_for(*t));
        }
        ctx.typed_locals = locals;
        ctx.label_stack = Vec::new();
        ctx.returned = false;

        // Decode and build control flow map for callee.
        let decoded = decode_function_body(&body);
        ctx.control_flow_map = build_control_flow_map(&decoded);

        // Convert to VM instructions, simultaneously building the callee's
        // per-function side-tables (br_table targets + WasmGC ops).  Each complex
        // instruction stores its index into the relevant Vec as its Operand::Index.
        let mut callee_br_table_targets: Vec<Vec<u32>> = Vec::new();
        let mut callee_gc_ops: Vec<GcOp> = Vec::new();
        let mut callee_simd_consts: Vec<[u8; 16]> = Vec::new();
        let mut vm_instructions: Vec<Instruction> = Vec::new();
        for d in &decoded {
            let operand =
                convert_operand(&d.operand, &mut callee_br_table_targets, &mut callee_gc_ops, &mut callee_simd_consts);
            vm_instructions.push(Instruction {
                opcode: d.opcode,
                operand,
            });
        }
        // Install the callee's side-tables; the caller's were saved above.  The GC
        // *heap* and struct field counts are module-global, so they are left alone.
        ctx.br_table_targets = callee_br_table_targets;
        ctx.gc_ops = callee_gc_ops;
        ctx.simd_consts = callee_simd_consts;

        // A WASM function body is itself an implicit outer `block` whose label is
        // the function's own end (spec: "execution of an instruction sequence
        // behaves as if it was wrapped in a block"). Without this, `br N`/`br_if
        // N`/`br_table` at a depth that walks all the way out of every *explicit*
        // block has nothing left on `label_stack` to resolve against and
        // `execute_branch` reports a spurious "branch target out of range" —
        // even though a bare `(br 0)` at function-top-level (no enclosing block
        // at all) is completely ordinary, spec-legal WASM, equivalent to
        // `return`. Pushing this label first, with `target_pc` one past the
        // callee's last instruction, makes that walk-all-the-way-out branch land
        // exactly where the function naturally ends: `execute_branch` pops the
        // function's own result arity, truncates the stack to the height it had
        // on entry, and jumps past the last instruction, so the `while` loop
        // below exits the same way it would on an ordinary fall-through return.
        ctx.label_stack.push(Label {
            arity: func_type.results.len(),
            param_arity: func_type.params.len(),
            target_pc: vm_instructions.len(),
            stack_height: vm.typed_stack.len(),
            is_loop: false,
        });

        // Set up callee code and jump to start.
        // We need to use a recursive execution approach. Execute the callee inline.
        vm.halted = false;

        let callee_code = CodeObject {
            instructions: vm_instructions,
            constants: vec![],
            names: vec![],
        };

        vm.pc = 0;

        // Execute callee with the same context.
        while !vm.halted && vm.pc < callee_code.instructions.len() {
            let instr = callee_code.instructions[vm.pc].clone();
            let pc_before = vm.pc;

            if let Some(handler) = vm.context_handlers.get(&instr.opcode).copied() {
                handler(vm, &instr, &callee_code, ctx)?;
            } else {
                return Err(VMError::InvalidOpcode(format!(
                    "no handler for opcode 0x{:02X}",
                    instr.opcode
                )));
            }

            // Check for nested calls that might have changed things
            let _ = pc_before;
        }

        // WASM16: did the inner loop halt because of a real
        // `return_call`/`return_call_indirect`, not an ordinary
        // `return`/fall-through? If so, don't collect return values or
        // restore caller state yet -- swap in the new function and loop
        // again, still inside this same Rust stack frame.
        if let Some((next_func_index, next_args)) = ctx.pending_tail_call.take() {
            current_func_index = next_func_index;
            pending_args = Some(next_args);
            continue;
        }

        break func_type.results.len();
    };

    // Collect return values (from whichever function FINALLY completed
    // -- the last one in the tail-call chain, or the only one if there
    // was no tail call at all).
    let mut return_values = Vec::new();
    for _ in 0..return_arity {
        return_values.push(pop_wasm(vm)?);
    }
    return_values.reverse();

    // Restore caller state.
    if frame_pushed {
        if let Some(frame) = ctx.saved_frames.pop() {
            ctx.typed_locals = frame.locals;
            ctx.label_stack = frame.label_stack;
            ctx.control_flow_map = frame.control_flow_map;
            ctx.br_table_targets = frame.br_table_targets;
            ctx.gc_ops = frame.gc_ops;
            ctx.simd_consts = frame.simd_consts;

            // Truncate stack to caller's height.
            while vm.typed_stack.len() > frame.stack_height {
                let _ = vm.pop_typed();
            }

            vm.pc = frame.return_pc;
        } else {
            vm.pc = saved_pc + 1;
        }
        vm.halted = false;
    } else {
        // Never pushed a frame -- this invocation never ran a
        // module-defined function at all (a pure host-function call,
        // whether reached on the first iteration or via a tail call
        // straight into an import). Matches the pre-WASM16 code's own
        // `vm.advance_pc()` shortcut exactly (`saved_pc` is `vm.pc` as
        // captured at entry, before any mutation).
        vm.pc = saved_pc + 1;
        vm.halted = false;
    }

    // Push return values.
    for v in return_values {
        push_wasm(vm, v);
    }

    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 11: WasmExecutionEngine
// ══════════════════════════════════════════════════════════════════════════════

/// Configuration for the execution engine.
pub struct WasmEngineConfig {
    /// Every memory this instance declared/imported, in index order
    /// (multi-memory proposal, W16, task #85).
    pub memories: Vec<LinearMemory>,
    pub tables: Vec<Table>,
    pub globals: Vec<WasmValue>,
    pub global_types: Vec<GlobalType>,
    pub func_types: Vec<FuncType>,
    pub func_bodies: Vec<Option<FunctionBody>>,
    pub host_functions: Vec<Option<Box<dyn HostFunction>>>,
}

/// Mutable engine state that should be written back to a long-lived instance.
pub struct WasmEngineState {
    pub memories: Vec<LinearMemory>,
    pub tables: Vec<Table>,
    pub globals: Vec<WasmValue>,
    pub host_functions: Vec<Option<Box<dyn HostFunction>>>,
    /// The instance's persistent v128 heap after this call (see
    /// `code/specs/W15-wasm-v128-persistent-storage.md`) -- set via
    /// [`WasmExecutionEngine::set_v128_heap`] before the call (mirroring
    /// `struct_field_counts`/`type_section`'s optional-setter pattern
    /// rather than a mandatory `WasmEngineConfig` field, so the ~50
    /// existing test construction sites that don't care about v128 don't
    /// all need updating), written back here so the caller can restore it
    /// onto the owning `WasmInstance`.
    pub v128_heap: Vec<[u8; 16]>,
    /// Each data segment's "already dropped" state after this call (task
    /// #95) -- set via [`WasmExecutionEngine::set_dropped_data_segments`]
    /// before the call, written back here so the caller can restore it
    /// onto the owning `WasmInstance`, same round-trip shape as
    /// `v128_heap` above.
    pub dropped_data_segments: Vec<bool>,
    /// Each element segment's "already dropped" state after this call
    /// (task #97) -- same round-trip shape as `dropped_data_segments`
    /// above, set via [`WasmExecutionEngine::set_dropped_elements`].
    pub dropped_elements: Vec<bool>,
}

/// The WASM execution engine — interprets validated WASM modules.
pub struct WasmExecutionEngine {
    vm: GenericVM,
    // Boxing is intentional, NOT redundant: `memory.size $mem1`/etc. and
    // `call_indirect` both store `*mut _` raw pointers into the execution
    // context (see `memory_ptrs`/`table_ptrs` below). Boxing gives each
    // entry a stable heap address so those pointers stay valid even if
    // this Vec reallocates; a plain `Vec<LinearMemory>`/`Vec<Table>` would
    // invalidate them.
    #[allow(clippy::vec_box)]
    memories: Vec<Box<LinearMemory>>,
    #[allow(clippy::vec_box)]
    tables: Vec<Box<Table>>,
    globals: Vec<WasmValue>,
    global_types: Vec<GlobalType>,
    func_types: Vec<FuncType>,
    func_bodies: Vec<Option<FunctionBody>>,
    host_functions: Vec<Option<Box<dyn HostFunction>>>,
    /// Field counts per struct type index (LANG77 L3b-3a-3b).  Empty by default;
    /// set with [`WasmExecutionEngine::set_struct_field_counts`].  Flows into the
    /// execution context so `struct.new N` knows how many fields to pop.
    struct_field_counts: Vec<u32>,
    /// The module's raw type section, indexed by **type index** — distinct
    /// from `func_types`, which is indexed by *function* index (one entry
    /// per function, resolved to whichever type that function declares).
    /// `call_indirect $type`'s immediate is a type-section index, so
    /// checking it against `func_types` would compare against an unrelated
    /// function's type. Empty by default (permissive: see
    /// `call_indirect`'s handler); set with
    /// [`WasmExecutionEngine::set_type_section`].
    type_section: Vec<FuncType>,
    /// GC bookkeeping from the most recently completed [`Self::call_function`]
    /// (W04). `gc_heap` itself isn't persisted here — it's rebuilt fresh every
    /// call, same as `struct_field_counts` isn't — but the *counters* (live
    /// object count, collection/freed/survived history) are written back
    /// after execution so a caller (or a test) can inspect them via
    /// [`Self::gc_live_object_count`] / [`Self::gc_profile`] without needing
    /// access to the ephemeral `WasmExecutionContext` itself.
    last_gc_state: gc::GcState,
    /// The instance's persistent v128 heap (see `code/specs/
    /// W15-wasm-v128-persistent-storage.md`) -- defaults to just the
    /// reserved all-zero entry (index 0), matching every `v128_heap`
    /// elsewhere in this crate; set for real via [`Self::set_v128_heap`]
    /// (same optional-setter pattern as `struct_field_counts`/
    /// `type_section`) when the embedder has a persistent instance to
    /// thread it from.
    v128_heap: Vec<[u8; 16]>,
    /// The module's data segments' raw bytes (task #95) -- see
    /// `WasmExecutionContext::data_segments`'s own doc comment. Immutable
    /// content, so unlike `dropped_data_segments` below there is no
    /// writeback after a call; set once via [`Self::set_data_segments`].
    data_segments: Vec<Vec<u8>>,
    /// Per-data-segment dropped flags (task #95) -- see
    /// `WasmExecutionContext::dropped_data_segments`'s own doc comment.
    /// Persistent across calls, same `set`-before/writeback-after pattern
    /// as `v128_heap`.
    dropped_data_segments: Vec<bool>,
    /// The module's element segments' entries (task #97) -- see
    /// `WasmExecutionContext::elements`'s own doc comment. Immutable
    /// content, same "no writeback needed" reasoning as `data_segments`
    /// above; set once via [`Self::set_elements`].
    elements: Vec<Vec<Option<u32>>>,
    /// Per-elem-segment dropped flags (task #97) -- see
    /// `WasmExecutionContext::dropped_elements`'s own doc comment.
    /// Persistent across calls, same `set`-before/writeback-after pattern
    /// as `dropped_data_segments`.
    dropped_elements: Vec<bool>,
}

impl WasmExecutionEngine {
    /// Create a new execution engine.
    pub fn new(config: WasmEngineConfig) -> Self {
        let mut vm = GenericVM::new();
        vm.set_max_recursion_depth(Some(1024));
        register_all_handlers(&mut vm);

        WasmExecutionEngine {
            vm,
            memories: config.memories.into_iter().map(Box::new).collect(),
            tables: config.tables.into_iter().map(Box::new).collect(),
            globals: config.globals,
            global_types: config.global_types,
            func_types: config.func_types,
            func_bodies: config.func_bodies,
            host_functions: config.host_functions,
            struct_field_counts: Vec::new(),
            type_section: Vec::new(),
            last_gc_state: gc::GcState::default(),
            v128_heap: vec![[0u8; 16]],
            data_segments: Vec::new(),
            dropped_data_segments: Vec::new(),
            elements: Vec::new(),
            dropped_elements: Vec::new(),
        }
    }

    /// Register the field counts of the module's WasmGC struct types, indexed by
    /// type index: `counts[N]` is the number of fields of struct type `N`, which
    /// is how many values `struct.new N` pops.  The wasm parser does not yet
    /// surface struct type definitions to the engine, so the embedder supplies
    /// them here (LANG77 L3b-3a-3b; populated automatically from the parsed
    /// module once that lands, L3b-3a-3c).  Returns `&mut self` for chaining.
    pub fn set_struct_field_counts(&mut self, counts: Vec<u32>) -> &mut Self {
        self.struct_field_counts = counts;
        self
    }

    /// Register the module's raw type section (`module.types`), indexed by
    /// type index — needed for `call_indirect $type` to check the callee's
    /// *actual* type against the type the call site *declared*, which are
    /// two different index spaces (see `type_section`'s own doc comment).
    /// `WasmEngineConfig` doesn't carry this (it would otherwise force
    /// every existing construction site — the many hand-built single/few
    /// function modules in this crate's own unit tests among them — to
    /// supply it), so it's set the same way `struct_field_counts` is: an
    /// optional embedder call between `new` and `call_function`. Left
    /// unset, `call_indirect`'s type check is permissive rather than wrong
    /// (see its handler). Returns `&mut self` for chaining.
    pub fn set_type_section(&mut self, types: Vec<FuncType>) -> &mut Self {
        self.type_section = types;
        self
    }

    /// Register the instance's persistent v128 heap (see `code/specs/
    /// W15-wasm-v128-persistent-storage.md`) -- same optional-setter
    /// pattern as [`Self::set_struct_field_counts`]/[`Self::set_type_section`]
    /// for the identical reason: a mandatory `WasmEngineConfig` field would
    /// force every existing construction site (including the many
    /// v128-agnostic hand-built modules in this crate's own unit tests) to
    /// supply it. Left unset, the engine keeps its `new()`-time default
    /// (just the reserved all-zero entry at index 0), which is correct for
    /// any module that never sets a v128 global -- there's nothing to
    /// persist. Returns `&mut self` for chaining.
    pub fn set_v128_heap(&mut self, heap: Vec<[u8; 16]>) -> &mut Self {
        self.v128_heap = heap;
        self
    }

    /// Register the module's data segments' raw bytes, indexed by data-
    /// segment index (task #95) -- `memory.init`'s source. Same optional-
    /// setter pattern as `set_struct_field_counts`/`set_type_section`:
    /// left unset, `memory.init`/`data.drop` see an empty `data_segments`,
    /// so any real data-segment index is cleanly out-of-bounds (a trap,
    /// not a panic) rather than a false "here are some bytes that don't
    /// belong to any real segment."
    pub fn set_data_segments(&mut self, segments: Vec<Vec<u8>>) -> &mut Self {
        self.data_segments = segments;
        self
    }

    /// Register each data segment's "already dropped" state (task #95),
    /// same index space as `set_data_segments` above. Same optional-setter
    /// pattern as [`Self::set_v128_heap`]: an embedder with a persistent
    /// `WasmInstance` threads this in before a call and reads the (now
    /// possibly further-dropped) result back out after, via
    /// [`WasmEngineState::dropped_data_segments`], so `data.drop`'s effect
    /// from one call is still visible in a later one.
    pub fn set_dropped_data_segments(&mut self, dropped: Vec<bool>) -> &mut Self {
        self.dropped_data_segments = dropped;
        self
    }

    /// Register the module's element segments' entries, indexed by
    /// elem-segment index (task #97) -- `table.init`'s source. Same
    /// optional-setter pattern as [`Self::set_data_segments`]: left
    /// unset, `table.init`/`elem.drop` see an empty `elements`, so any
    /// real elem-segment index is cleanly out-of-bounds (a trap, not a
    /// panic).
    pub fn set_elements(&mut self, elements: Vec<Vec<Option<u32>>>) -> &mut Self {
        self.elements = elements;
        self
    }

    /// Register each element segment's "already dropped" state (task
    /// #97), same index space as [`Self::set_elements`] above. Same
    /// optional-setter pattern as [`Self::set_dropped_data_segments`].
    pub fn set_dropped_elements(&mut self, dropped: Vec<bool>) -> &mut Self {
        self.dropped_elements = dropped;
        self
    }

    /// Live `gc_heap` object count as of the most recently completed
    /// [`Self::call_function`] (W04). `gc_heap` itself resets every call, so
    /// this reflects only that one call's allocation/collection activity,
    /// not a running total across calls.
    pub fn gc_live_object_count(&self) -> usize {
        self.last_gc_state.live_count
    }

    /// Diagnostic history (collections run, objects freed/survived,
    /// fragmentation/survival-ratio estimates) from the most recently
    /// completed [`Self::call_function`] — the same `gc_core::GcProfile`
    /// type the native-AOT and `vm-core` GC paths use, for consistency.
    pub fn gc_profile(&self) -> &gc_core::GcProfile {
        &self.last_gc_state.profile
    }

    /// Consume the engine and return the mutated runtime state.
    pub fn into_state(self) -> WasmEngineState {
        WasmEngineState {
            memories: self.memories.into_iter().map(|memory| *memory).collect(),
            tables: self.tables.into_iter().map(|table| *table).collect(),
            globals: self.globals,
            host_functions: self.host_functions,
            v128_heap: self.v128_heap,
            dropped_data_segments: self.dropped_data_segments,
            dropped_elements: self.dropped_elements,
        }
    }

    /// Call a WASM function by index.
    ///
    /// **WASM10**: nested WASM `call`/`call_indirect` recurse through a
    /// Rust call stack one level per nested call, up to [`MAX_CALL_DEPTH`]
    /// before this returns a "call stack exhausted" trap — but that
    /// recursion now runs entirely on a dedicated OS thread this call
    /// spawns internally with an explicit [`DEDICATED_STACK_SIZE`], not on
    /// whatever stack the CALLER of `call_function` happens to have. The
    /// calling thread only spawns and immediately `.join()`s; it does none
    /// of the recursive work itself. This means, unlike before WASM10,
    /// there is no caller-stack-size requirement at all — `call_function`
    /// is safe to invoke from a thread with a small or unusual stack (a
    /// constrained worker-thread pool, a reactor/executor with small
    /// per-task stacks) without that affecting how deep WASM recursion can
    /// safely go.
    /// See [`Self::call_function`] and [`Self::call_function_with_v128`] --
    /// both are thin wrappers around this shared implementation, which
    /// does the real work and additionally resolves any `WasmValue::
    /// V128(handle)` RESULT into its real 16 bytes (via `ctx.v128_heap`,
    /// still alive at the point results are collected, right before this
    /// function returns and `ctx` -- along with the heap the handle
    /// indexes into -- drops) before that handle becomes meaningless to
    /// the caller. See `code/specs/W13-wasm-simd-v128-first-slice.md`'s
    /// follow-up scope for why this exists: `wasm-conformance` needs REAL
    /// byte-exact v128 comparison, which a bare, post-return handle can't
    /// provide (`WasmExecutionEngine` itself has no `v128_heap` field --
    /// only this one call's now-dropped `ctx` ever held it).
    fn call_function_impl(
        &mut self,
        func_index: usize,
        args: &[WasmValue],
    ) -> Result<(Vec<WasmValue>, Vec<Option<V128Bytes>>), TrapError> {
        let initial_func_type = self
            .func_types
            .get(func_index)
            .ok_or_else(|| TrapError::new(format!("undefined function index {}", func_index)))?;

        if args.len() != initial_func_type.params.len() {
            return Err(TrapError::new(format!(
                "function {} expects {} arguments, got {}",
                func_index,
                initial_func_type.params.len(),
                args.len()
            )));
        }

        // Check for host function -- unchanged fast path for calling a
        // host-imported function directly; no WASM frame or tail-call
        // machinery is needed at all here. A host function has no engine
        // `ctx`/`v128_heap` of its own to have produced a real v128
        // handle from, so its results (whatever their `WasmValue` shape)
        // carry no resolved v128 bytes -- `None` for each, not a
        // meaningful absence.
        if let Some(Some(host_func)) = self.host_functions.get(func_index) {
            let memory = self.memories.first_mut().map(|m| &mut **m);
            return host_func.call(args, memory).map(|results| {
                let v128_bytes = vec![None; results.len()];
                (results, v128_bytes)
            });
        }

        // WASM10 (security review): about to spawn a dedicated OS thread
        // for this call. If a `HostFunction::call` reached from inside
        // that thread re-enters a DIFFERENT engine's own `call_function`
        // (as `wasm-conformance`'s real cross-module linking does), THAT
        // nested call spawns its OWN dedicated thread, nested inside this
        // one -- unlike same-instance recursion, this isn't bounded by
        // `MAX_CALL_DEPTH`/`ctx.call_depth` at all (that resets to 0 per
        // top-level call). Reject before spawning rather than let an
        // ordinary, non-circular multi-module chain exhaust OS threads.
        // See `MAX_DEDICATED_THREAD_DEPTH`'s own doc comment.
        let dedicated_thread_depth = DEDICATED_THREAD_DEPTH.with(|d| d.get());
        if dedicated_thread_depth >= MAX_DEDICATED_THREAD_DEPTH {
            return Err(TrapError::new("cross-module call nesting exhausted".to_string()));
        }

        // Build raw pointers for the context -- shared VM-level state,
        // computed once regardless of how many WASM16 tail-call
        // transitions follow below.
        let memory_ptrs: Vec<*mut LinearMemory> = self
            .memories
            .iter_mut()
            .map(|m| &mut **m as *mut LinearMemory)
            .collect();
        let table_ptrs: Vec<*mut Table> = self
            .tables
            .iter_mut()
            .map(|t| &mut **t as *mut Table)
            .collect();
        let host_functions = std::mem::take(&mut self.host_functions);

        let mut ctx = WasmExecutionContext {
            memories: memory_ptrs,
            tables: table_ptrs,
            globals: self.globals.clone(),
            global_types: self.global_types.clone(),
            func_types: self.func_types.clone(),
            types: self.type_section.clone(),
            func_bodies: self.func_bodies.clone(),
            host_functions,
            typed_locals: Vec::new(),
            label_stack: Vec::new(),
            control_flow_map: HashMap::new(),
            saved_frames: Vec::new(),
            returned: false,
            br_table_targets: Vec::new(),
            gc_ops: Vec::new(),
            simd_consts: Vec::new(),
            // The GC heap starts empty and grows as `struct.new` allocates; it
            // lives for the whole call so a cons built in a callee survives.
            // Real mark-sweep collection now runs against it (W04) at loop
            // back-edges and calls, so a long call no longer grows it
            // without bound.
            gc_heap: Vec::new(),
            // Cloned from `self.v128_heap`, NOT reseeded to
            // `vec![[0u8; 16]]` -- see `code/specs/
            // W15-wasm-v128-persistent-storage.md`. Reseeding here was
            // the original bug this spec fixes: any v128 handle a global
            // held across calls became garbage once the heap it indexed
            // into was thrown away and rebuilt from scratch every call.
            v128_heap: self.v128_heap.clone(),
            struct_field_counts: self.struct_field_counts.clone(),
            gc_state: gc::GcState::default(),
            call_depth: 0,
            pending_tail_call: None,
            data_segments: self.data_segments.clone(),
            dropped_data_segments: self.dropped_data_segments.clone(),
            elements: self.elements.clone(),
            dropped_elements: self.dropped_elements.clone(),
        };

        // Reset and execute.
        self.vm.reset();
        register_all_handlers(&mut self.vm);

        // WASM10: everything below that actually recurses (this loop,
        // plus every NESTED `call`/`call_indirect` it triggers through
        // `call_function_inner`) runs on a dedicated OS thread with an
        // explicit `DEDICATED_STACK_SIZE`, decoupling `MAX_CALL_DEPTH`
        // from whatever stack the CALLER of `call_function` happens to
        // have. Security review (WASM10, round 2): `ctx` stays OWNED
        // here, in this (spawning) stack frame -- only a raw pointer to
        // it (`ctx_ptr`, alongside `vm_ptr`) crosses the thread boundary,
        // the same treatment `vm_ptr` already gets. This is deliberate:
        // an earlier version moved `ctx` BY VALUE into the spawned
        // closure, which meant a `Builder::spawn_scoped` failure (a real
        // possibility under OS thread/resource exhaustion -- this
        // feature can spawn up to `MAX_DEDICATED_THREAD_DEPTH` nested
        // threads per call chain) would drop the closure -- and `ctx`
        // along with it -- before `self.host_functions` (moved out via
        // `mem::take` above) was ever restored, permanently corrupting
        // engine state for later, unrelated calls (the exact WASM07 bug
        // class, via a THIRD trigger beyond the trap/panic cases already
        // fixed). Keeping `ctx` owned here means it's always available to
        // restore from, regardless of whether the thread spawns, panics,
        // or completes normally. See `AssertSend`'s own doc comment for
        // the full cross-thread-access safety argument (identical to
        // `vm_ptr`'s).
        let vm_ptr: *mut GenericVM = &mut self.vm;
        let ctx_ptr: *mut WasmExecutionContext = &mut ctx;
        let payload = AssertSend((vm_ptr, ctx_ptr, func_index, args.to_vec(), dedicated_thread_depth));

        /// The three ways the dedicated thread can fail to produce a
        /// normal `VMResult<usize>` -- kept distinct from `VMError`
        /// (an ordinary WASM-level trap, handled via `Ok(Err(..))`
        /// below) because a spawn failure and a panic both need
        /// `ctx`'s state restored before propagating, but must be
        /// propagated differently (a spawn failure becomes a clean
        /// `TrapError`; a panic must keep unwinding via `resume_unwind`).
        enum DedicatedThreadFailure {
            SpawnFailed(std::io::Error),
            Panicked(Box<dyn std::any::Any + Send>),
        }

        let outcome: Result<VMResult<usize>, DedicatedThreadFailure> = std::thread::scope(|scope| {
            match std::thread::Builder::new().stack_size(DEDICATED_STACK_SIZE).spawn_scoped(scope, move || {
                    let (vm_ptr, ctx_ptr, func_index, initial_args, parent_depth) = payload.into_inner();
                    // Propagate this THREAD's own nesting depth before
                    // doing anything else -- see `DEDICATED_THREAD_DEPTH`'s
                    // own doc comment. A fresh OS thread starts with a
                    // fresh (zeroed) thread-local, so this must be set
                    // explicitly from the parent's depth, not inherited.
                    DEDICATED_THREAD_DEPTH.with(|d| d.set(parent_depth + 1));
                    // SAFETY: see `AssertSend`'s doc comment -- for the
                    // whole lifetime of this closure, the spawning thread
                    // is blocked in `.join()` below (or, if spawning
                    // itself failed, this closure never ran at all) and
                    // touches nothing reachable through `vm_ptr`/
                    // `ctx_ptr`, so this is the only thread ever
                    // dereferencing them at a time.
                    let vm = unsafe { &mut *vm_ptr };
                    let ctx = unsafe { &mut *ctx_ptr };

                    // WASM10 (security review): the loop below can call
                    // arbitrary embedder-supplied `Box<dyn HostFunction>`
                    // code, including (via `wasm-conformance`'s real
                    // `CrossModuleFunction`) a re-entrant call into a
                    // DIFFERENT engine's own `call_function`, which might
                    // panic. `catch_unwind` here, rather than letting the
                    // panic unwind straight out of this thread past
                    // `handle.join()` below, lets the calling thread
                    // restore `self.globals`/`self.host_functions` from
                    // `ctx` BEFORE the panic is re-raised -- the same
                    // "restore engine state before propagating ANY
                    // failure" rule the WASM07 security review already
                    // established for traps, extended here to panics
                    // (a bare `resume_unwind` before that restoration
                    // would permanently leave `self.host_functions` empty
                    // -- moved out via `mem::take` above -- for every
                    // LATER, unrelated call on this same engine).
                    // `AssertUnwindSafe` is sound here specifically
                    // because a caught panic means this call is being
                    // abandoned regardless (about to be re-raised) --
                    // `ctx`'s post-panic contents are only ever used for
                    // that restoration, never assumed logically
                    // consistent for anything else.
                    let panic_result: std::thread::Result<VMResult<usize>> =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let mut current_func_index = func_index;
                            let mut pending_args: Option<Vec<WasmValue>> = Some(initial_args);

                            // WASM16: this top-level entry point has its own
                            // separate instruction-decode-and-dispatch path (it
                            // doesn't go through `call_function_inner`, which
                            // only handles NESTED calls) — so a
                            // `return_call`/`return_call_indirect` chain that
                            // starts at the very top level needs the SAME "swap
                            // the current frame instead of recursing" handling
                            // duplicated here. See `call_function_inner`'s
                            // matching loop and
                            // `WasmExecutionContext::pending_tail_call`'s own
                            // doc comment.
                            //
                            // The `Ok` variant carries the result arity
                            // alongside `()` (as `usize`) so it can be read
                            // back out after `catch_unwind` returns without
                            // needing a separate `&mut final_result_count`
                            // capture into this closure.
                            let exec_result: VMResult<usize> = loop {
                                let func_type = match ctx
                                    .func_types
                                    .get(current_func_index)
                                    .ok_or_else(|| VMError::GenericError(format!("undefined function {current_func_index}")))
                                {
                                    Ok(t) => t.clone(),
                                    Err(e) => break Err(e),
                                };
                                let current_args = pending_args.take().expect("pending_args is always Some at the top of each loop iteration");

                                // A tail call landing on a host import is still a leaf call
                                // (no further WASM frames) -- call it, push its results
                                // exactly like `call_function_inner`'s own host branch
                                // does, and stop.
                                if let Some(Some(host_func)) = ctx.host_functions.get(current_func_index) {
                                    match host_func.call(&current_args, ctx.memories.first().map(|&ptr| unsafe { &mut *ptr })) {
                                        Ok(results) => {
                                            for r in results {
                                                push_wasm(vm, r);
                                            }
                                            break Ok(func_type.results.len());
                                        }
                                        Err(e) => break Err(VMError::from(e)),
                                    }
                                }

                                // Module-defined function.
                                let body = match ctx
                                    .func_bodies
                                    .get(current_func_index)
                                    .and_then(|b| b.as_ref())
                                    .ok_or_else(|| VMError::GenericError(format!("no body for function {current_func_index}")))
                                {
                                    Ok(b) => b.clone(),
                                    Err(e) => break Err(e),
                                };

                                // Decode the function body.
                                let decoded = decode_function_body(&body);
                                ctx.control_flow_map = build_control_flow_map(&decoded);

                                // Convert to VM instructions, building this function's
                                // side-tables (br_table targets + WasmGC ops) in lockstep.
                                // Each complex instruction stores its index into the
                                // relevant Vec as its Operand::Index. Nested calls save/
                                // restore these on the saved-frame stack so callee and
                                // caller don't collide; a tail-call transition here simply
                                // overwrites them, matching the fact that no new logical
                                // call frame is being pushed.
                                let mut br_table_targets: Vec<Vec<u32>> = Vec::new();
                                let mut gc_ops: Vec<GcOp> = Vec::new();
                                let mut simd_consts: Vec<[u8; 16]> = Vec::new();
                                let mut vm_instructions: Vec<Instruction> = Vec::new();
                                for d in &decoded {
                                    let operand = convert_operand(&d.operand, &mut br_table_targets, &mut gc_ops, &mut simd_consts);
                                    vm_instructions.push(Instruction {
                                        opcode: d.opcode,
                                        operand,
                                    });
                                }
                                ctx.br_table_targets = br_table_targets;
                                ctx.gc_ops = gc_ops;
                                ctx.simd_consts = simd_consts;

                                // Initialize locals.
                                let mut typed_locals: Vec<WasmValue> = current_args;
                                for t in &body.locals {
                                    typed_locals.push(WasmValue::default_for(*t));
                                }
                                ctx.typed_locals = typed_locals;

                                // See `call_function_inner`'s matching comment: a WASM
                                // function body is itself an implicit outer `block` whose
                                // label is the function's own end, so `br`/`br_if`/
                                // `br_table` at a depth that walks out of every *explicit*
                                // block (including a bare top-level `(br 0)`, which is
                                // ordinary, spec-legal WASM meaning "return") needs a label
                                // on `label_stack` to resolve against. `stack_height: 0` is
                                // correct on every iteration, not just the first: the
                                // validator requires a `return_call`/`return_call_indirect`
                                // site to leave the operand stack at exactly this
                                // function's entry height once the callee's args are
                                // popped (the same "stack-polymorphic, like `return`" rule
                                // that lets `return_call` type-check at all), so control
                                // never reaches a later iteration with anything extra left
                                // on `vm`'s stack.
                                ctx.label_stack = vec![Label {
                                    arity: func_type.results.len(),
                                    param_arity: func_type.params.len(),
                                    target_pc: vm_instructions.len(),
                                    stack_height: 0,
                                    is_loop: false,
                                }];

                                let code = CodeObject {
                                    instructions: vm_instructions,
                                    constants: vec![],
                                    names: vec![],
                                };

                                vm.pc = 0;
                                vm.halted = false;
                                // `ctx` is already `&mut WasmExecutionContext` here (obtained
                                // via `ctx_ptr`) -- pass it directly, relying on Rust's
                                // implicit reborrow, NOT `&mut ctx` (which would build a
                                // `&mut &mut WasmExecutionContext` and break
                                // `execute_with_context`'s downcast).
                                if let Err(e) = vm.execute_with_context(&code, ctx) {
                                    break Err(e);
                                }

                                if let Some((next_func_index, next_args)) = ctx.pending_tail_call.take() {
                                    current_func_index = next_func_index;
                                    pending_args = Some(next_args);
                                    continue;
                                }

                                break Ok(func_type.results.len());
                            };

                            exec_result
                        }));

                    panic_result
                }) {
                Ok(handle) => match handle.join() {
                    // `panic_result` is itself `std::thread::Result<
                    // VMResult<usize>>` -- flatten its `Err` (a caught
                    // panic, from the `catch_unwind` inside the closure)
                    // into `DedicatedThreadFailure::Panicked` too, so
                    // both this and the `join()`-level `Err` arm below
                    // are handled uniformly by the caller.
                    Ok(panic_result) => panic_result.map_err(DedicatedThreadFailure::Panicked),
                    // Defensive fallback only: `catch_unwind` above
                    // already catches every panic reachable from
                    // ordinary WASM execution or host-function calls, so
                    // this thread should always return normally via the
                    // `Ok` arm above. This arm exists only for a panic
                    // reached OUTSIDE that `catch_unwind` (e.g. in
                    // `payload.into_inner()` or the raw-pointer derefs
                    // immediately above it) -- effectively unreachable
                    // in practice, but handled the same way as a caught
                    // panic below rather than silently ignored.
                    Err(join_panic_payload) => Err(DedicatedThreadFailure::Panicked(join_panic_payload)),
                },
                Err(spawn_err) => Err(DedicatedThreadFailure::SpawnFailed(spawn_err)),
            }
        });

        // Update engine state back UNCONDITIONALLY, before propagating
        // ANY failure -- a trap, a panic, OR a thread-spawn failure
        // (WASM07 security review, extended twice over by WASM10's own
        // security review: once to cover the panic case, and again to
        // cover the spawn-failure case, which an earlier version of this
        // fix still missed -- `ctx` moving BY VALUE into the spawned
        // closure meant a `spawn_scoped` failure dropped it, along with
        // `self.host_functions`, before this restoration ever ran).
        // `self.host_functions` was moved out via `mem::take` above, so
        // the ONLY way it's ever seen again is `ctx.host_functions` here.
        // Propagating a failure before this line permanently leaves
        // `self.host_functions` empty for every LATER, unrelated call on
        // this engine. `wasm-runtime`'s real embedding path wires WASI
        // imports (fd_write, random_get, clock_time_get, ...) through
        // exactly this field, so this was a real, reachable bug, not a
        // theoretical one: `wasm-runtime`'s own `call_engine` had the
        // identical bug for `instance.memory`/`instance.tables` (fixed in
        // this same PR, WASM07) one layer further out. Keeping `ctx`
        // OWNED by this (spawning) stack frame the whole time -- see the
        // comment where `ctx_ptr` is built above -- is what makes this
        // restoration reachable regardless of which of the three ways
        // the dedicated thread could fail to produce a normal result.
        self.globals = ctx.globals;
        self.host_functions = ctx.host_functions;
        // Cloned here (NOT moved), unconditionally alongside globals/
        // host_functions above -- security review (task #79): a version
        // of this that only wrote back AFTER the `outcome` trap-check
        // below (see `final_result_count`) silently lost any `v128.const`
        // growth from a call that trapped, the exact class of bug
        // `wasm-runtime::call_engine`'s own doc comment already warns
        // about for memory/tables ("skip this restoration on any trap ...
        // masking whatever the test was actually checking"). Cloning
        // (rather than moving) here is required because the per-result
        // `V128Bytes` resolution loop below still needs to BORROW
        // `ctx.v128_heap` after this point -- moving it out now would be
        // a use-after-move compile error.
        self.v128_heap = ctx.v128_heap.clone();
        // Same unconditional-writeback reasoning as `v128_heap` just above
        // (task #95): a `data.drop` from a call that later traps must
        // still stick -- the drop already happened before the trap, and a
        // real WASM engine can't un-happen it.
        self.dropped_data_segments = ctx.dropped_data_segments.clone();
        // Same unconditional-writeback reasoning as `dropped_data_segments`
        // just above (task #97): an `elem.drop` from a call that later
        // traps must still stick.
        self.dropped_elements = ctx.dropped_elements.clone();
        // gc_heap itself is not persisted (see its own doc comment above);
        // the counters are, so a caller can inspect this call's GC activity
        // via gc_live_object_count()/gc_profile() (W04).
        self.last_gc_state = ctx.gc_state;

        let final_result_count = match outcome {
            Ok(exec_result) => exec_result.map_err(|e| TrapError::new(format!("{}", e)))?,
            Err(DedicatedThreadFailure::SpawnFailed(spawn_err)) => {
                return Err(TrapError::new(format!("failed to spawn dedicated execution thread: {spawn_err}")));
            }
            Err(DedicatedThreadFailure::Panicked(panic_payload)) => std::panic::resume_unwind(panic_payload),
        };

        // Collect return values, resolving any V128 handle into its real
        // bytes via `ctx.v128_heap` -- still alive here, one statement
        // before this function returns and `ctx` (and the heap the
        // handle indexes into) drops for good.
        let mut results = Vec::new();
        let mut v128_bytes = Vec::new();
        for _ in 0..final_result_count {
            let tv = self
                .vm
                .pop_typed()
                .map_err(|e| TrapError::new(format!("{}", e)))?;
            let wv = WasmValue::from_typed(&tv)?;
            v128_bytes.push(match wv {
                WasmValue::V128(handle) => ctx.v128_heap.get(handle as usize).copied().map(V128Bytes),
                _ => None,
            });
            results.push(wv);
        }
        results.reverse();
        v128_bytes.reverse();

        Ok((results, v128_bytes))
    }

    /// Call a WASM function by index. See [`Self::call_function_impl`]'s
    /// own doc comment for the full design; this is the pre-existing,
    /// unchanged-signature entry point every current caller in this
    /// workspace already uses -- it simply discards the resolved v128
    /// bytes [`Self::call_function_with_v128`] also provides.
    pub fn call_function(&mut self, func_index: usize, args: &[WasmValue]) -> Result<Vec<WasmValue>, TrapError> {
        self.call_function_impl(func_index, args).map(|(results, _)| results)
    }

    /// Like [`Self::call_function`], but also returns each result's real
    /// v128 bytes (`Some` for a `WasmValue::V128` result, `None` for
    /// every other result shape), resolved before they'd otherwise become
    /// meaningless — see `code/specs/
    /// W13-wasm-simd-v128-first-slice.md`'s follow-up scope. The two
    /// returned `Vec`s are always the same length and index-aligned with
    /// each other.
    pub fn call_function_with_v128(
        &mut self,
        func_index: usize,
        args: &[WasmValue],
    ) -> Result<(Vec<WasmValue>, Vec<Option<V128Bytes>>), TrapError> {
        self.call_function_impl(func_index, args)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 12: Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    // Tests use 3.14 / 2.718 as arbitrary float sample values (checking f32/f64
    // store/load/const/convert behaviour), not as approximations of PI or E.
    #![allow(clippy::approx_constant)]
    use super::*;
    use wasm_types::{FuncType, FunctionBody, ValueType};

    #[test]
    fn test_wasm_value_constructors() {
        assert_eq!(WasmValue::I32(42).as_i32().unwrap(), 42);
        assert_eq!(WasmValue::I64(100).as_i64().unwrap(), 100);
        assert_eq!(WasmValue::F32(3.14).as_f32().unwrap(), 3.14);
        assert_eq!(WasmValue::F64(2.718).as_f64().unwrap(), 2.718);
    }

    #[test]
    fn test_wasm_value_type_mismatch() {
        assert!(WasmValue::I32(42).as_i64().is_err());
        assert!(WasmValue::F32(1.0).as_i32().is_err());
    }

    #[test]
    fn test_wasm_value_round_trip() {
        let values = [
            WasmValue::I32(-1),
            WasmValue::I64(i64::MAX),
            WasmValue::F32(1.5),
            WasmValue::F64(std::f64::consts::PI),
        ];
        for v in &values {
            let typed = v.to_typed();
            let back = WasmValue::from_typed(&typed).unwrap();
            assert_eq!(*v, back);
        }
    }

    #[test]
    fn test_f32_nan_bit_pattern_survives_the_typed_stack_round_trip() {
        // WASM13: `to_typed`/`from_typed` box every f32 through the
        // GenericVM's single f64 float slot -- EVERY f32 that's merely
        // pushed/popped (locals, params, results, operands) round-trips
        // through here, not just ones an opcode computed on. `assert_eq!`
        // can't check this (NaN != NaN under IEEE754, the derived
        // `PartialEq`), so this compares raw bits directly. Covers a range
        // of real testsuite bit patterns this bug actually lost: distinct
        // payloads, both signs, both quiet and would-be-signaling patterns
        // (the top mantissa bit set vs. clear).
        let patterns: [u32; 6] = [
            0x7fa00000, // quiet NaN, one payload
            0xffa00000, // quiet NaN, negative, same payload magnitude
            0x7fc00000, // the canonical quiet NaN itself (must still round-trip exactly)
            0x7f800001, // signaling NaN, minimal payload
            0xff800001, // signaling NaN, negative, minimal payload
            0x7fffffff, // quiet NaN, all payload bits set
        ];
        for bits in patterns {
            let original = f32::from_bits(bits);
            assert!(original.is_nan(), "test fixture bug: {bits:#010x} is not actually a NaN");
            let typed = WasmValue::F32(original).to_typed();
            let back = WasmValue::from_typed(&typed).unwrap();
            match back {
                WasmValue::F32(v) => assert_eq!(
                    v.to_bits(),
                    bits,
                    "NaN payload lost round-tripping {bits:#010x}: got {:#010x}",
                    v.to_bits()
                ),
                other => panic!("expected F32 back, got {other:?}"),
            }
        }
    }

    #[test]
    fn test_linear_memory_basic() {
        let mut mem = LinearMemory::new(1, None);
        assert_eq!(mem.size(), 1);

        mem.store_i32(0, 42).unwrap();
        assert_eq!(mem.load_i32(0).unwrap(), 42);

        // Little-endian check
        mem.store_i32(0, 0x01020304).unwrap();
        assert_eq!(mem.load_i32_8u(0).unwrap(), 0x04);
        assert_eq!(mem.load_i32_16u(0).unwrap(), 0x0304);
    }

    #[test]
    fn test_linear_memory_out_of_bounds() {
        let mem = LinearMemory::new(1, None);
        assert!(mem.load_i32(65536).is_err());
    }

    #[test]
    fn test_linear_memory_grow() {
        let mut mem = LinearMemory::new(1, Some(3));
        assert_eq!(mem.grow(1), 1); // old size was 1
        assert_eq!(mem.size(), 2);
        assert_eq!(mem.grow(2), -1); // would exceed max of 3
    }

    #[test]
    fn test_table_basic() {
        let mut table = Table::new(5, None);
        assert_eq!(table.size(), 5);
        assert_eq!(table.get(0).unwrap(), None);

        table.set(2, Some(42)).unwrap();
        assert_eq!(table.get(2).unwrap(), Some(42));
    }

    #[test]
    fn test_table_grow() {
        let mut table = Table::new(1, Some(3));
        assert_eq!(table.grow(1, Some(7)), 1); // old size was 1
        assert_eq!(table.size(), 2);
        assert_eq!(table.get(1).unwrap(), Some(7));
        assert_eq!(table.grow(2, None), -1); // would exceed max of 3
        assert_eq!(table.size(), 2); // unchanged on failure
    }

    #[test]
    fn test_table_grow_zero_delta_is_a_no_op_success() {
        let mut table = Table::new(4, None);
        assert_eq!(table.grow(0, None), 4);
        assert_eq!(table.size(), 4);
    }

    #[test]
    fn test_table_grow_rejects_growth_past_i32_max_even_with_no_declared_max() {
        // No `max_size` at all, but growing this far would make `table.size`'s
        // own i32 result type unable to represent the new size.
        let mut table = Table::new(16, None);
        assert_eq!(table.grow(0xFFFF_FFF0, None), -1);
        assert_eq!(table.size(), 16); // unchanged on failure
    }

    /// Security review (task #98): a table with NO declared `max_size`
    /// (entirely legal WASM) must still reject growth well before
    /// `i32::MAX` entries -- without this, `Table::elements` (`Vec<
    /// Option<u32>>`, 8 bytes/entry with no niche optimization for
    /// `u32`) could be resized to ~17GB in a single `table.grow` call
    /// driven by nothing more than one attacker-controlled `.wasm`
    /// module. `MAX_TABLE_ELEMENTS` already exists (task #96, security
    /// review) to bound a table's DECLARED `min` for exactly this
    /// resource-exhaustion reason; this test pins that the same ceiling
    /// also applies to RUNTIME growth, not just declaration.
    #[test]
    fn test_table_grow_rejects_growth_past_max_table_elements_even_with_no_declared_max() {
        let mut table = Table::new(1, None);
        assert_eq!(table.grow(MAX_TABLE_ELEMENTS, None), -1);
        assert_eq!(table.size(), 1); // unchanged on failure

        // One past the cap, from a starting size that's already AT the cap:
        // proves the check compares the NEW total, not just `delta` in isolation.
        let mut at_cap = Table::new(MAX_TABLE_ELEMENTS, None);
        assert_eq!(at_cap.grow(1, None), -1);
        assert_eq!(at_cap.size(), MAX_TABLE_ELEMENTS);
    }

    #[test]
    fn test_table_fill() {
        let mut table = Table::new(10, None);
        table.fill(2, Some(1), 3).unwrap();
        assert_eq!(table.get(1).unwrap(), None);
        assert_eq!(table.get(2).unwrap(), Some(1));
        assert_eq!(table.get(3).unwrap(), Some(1));
        assert_eq!(table.get(4).unwrap(), Some(1));
        assert_eq!(table.get(5).unwrap(), None);
    }

    #[test]
    fn test_table_fill_zero_length_at_the_exact_end_still_bounds_checks_but_succeeds() {
        // Same discipline as `LinearMemory::fill` (task #94): `dest ==
        // size()` with `len == 0` is the one boundary case that's valid,
        // not an off-by-one trap.
        let mut table = Table::new(5, None);
        table.fill(5, Some(1), 0).unwrap();
    }

    #[test]
    fn test_table_fill_out_of_bounds_traps_cleanly_not_a_panic() {
        let mut table = Table::new(5, None);
        assert!(table.fill(3, Some(1), 3).is_err());
        assert!(table.fill(6, None, 0).is_err());
    }

    #[test]
    fn test_table_out_of_bounds() {
        let table = Table::new(2, None);
        assert!(table.get(5).is_err());
    }

    #[test]
    fn test_evaluate_const_expr_i32() {
        // i32.const 42; end
        let expr = vec![0x41, 0x2A, 0x0B];
        let result = evaluate_const_expr(&expr, &[], &mut Vec::new()).unwrap();
        assert_eq!(result, WasmValue::I32(42));
    }

    #[test]
    fn test_evaluate_const_expr_global_get() {
        // global.get 0; end
        let expr = vec![0x23, 0x00, 0x0B];
        let globals = vec![WasmValue::I32(100)];
        let result = evaluate_const_expr(&expr, &globals, &mut Vec::new()).unwrap();
        assert_eq!(result, WasmValue::I32(100));
    }

    #[test]
    fn test_decode_function_body() {
        // local.get 0; local.get 0; i32.mul; end
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B],
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded.len(), 4);
        assert_eq!(decoded[0].opcode, 0x20); // local.get
        assert_eq!(decoded[2].opcode, 0x6C); // i32.mul
        assert_eq!(decoded[3].opcode, 0x0B); // end
    }

    #[test]
    fn test_square_function() {
        // square(x) = x * x
        // Bytecodes: local.get 0; local.get 0; i32.mul; end
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B],
        };

        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });

        let result = engine.call_function(0, &[WasmValue::I32(5)]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(25)]);
    }

    #[test]
    fn test_add_function() {
        // add(a, b) = a + b
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B],
        };

        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });

        let result = engine
            .call_function(0, &[WasmValue::I32(3), WasmValue::I32(7)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(10)]);
    }

    // ── WasmGC i31 execution (LANG77 / McCarthy L3b-3a-3a) ────────────────────

    #[test]
    fn test_decode_gc_i31_two_byte_opcodes() {
        // The decoder must group `0xFB <sub>` into ONE instruction carrying the
        // sub-opcode, not two single-byte instructions.
        let body = FunctionBody {
            locals: vec![],
            code: vec![0xFB, 0x1C, 0xFB, 0x1D, 0x0B], // i31.new, i31.get_s, end
        };
        let instrs = decode_function_body(&body);
        assert_eq!(instrs.len(), 3, "0xFB-prefixed pairs must collapse to one instr each");
        assert_eq!(instrs[0].opcode, 0xFB);
        assert!(matches!(instrs[0].operand, DecodedOperand::Gc { sub: 0x1C, .. }));
        assert_eq!(instrs[1].opcode, 0xFB);
        assert!(matches!(instrs[1].operand, DecodedOperand::Gc { sub: 0x1D, .. }));
        assert_eq!(instrs[2].opcode, 0x0B, "end");
    }

    #[test]
    fn test_i31_box_unbox_round_trip() {
        // fn() -> i32 { i31.get_s(i31.new(i32.const 42)) }  → 42
        // bytes: i32.const 42 (0x41 0x2A), i31.new (0xFB 0x1C), i31.get_s (0xFB 0x1D), end (0x0B)
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x41, 0x2A, 0xFB, 0x1C, 0xFB, 0x1D, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        let result = engine.call_function(0, &[]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(42)], "i31 box/unbox must round-trip the integer");
    }

    #[test]
    fn test_unsupported_gc_opcode_is_clean_error() {
        // An unimplemented GC sub-opcode (here 0x77, not a real WasmGC op) must
        // be a clean Err, not a panic.
        let func_type = FuncType { params: vec![], results: vec![] };
        let body = FunctionBody { locals: vec![], code: vec![0xFB, 0x77, 0x0B] };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert!(engine.call_function(0, &[]).is_err());
    }

    // ── WasmGC struct heap + references (LANG77 / McCarthy L3b-3a-3b) ──────────

    /// Helper: build a single-function engine from a raw code body and result
    /// type, with the given struct field-count table registered.
    fn gc_engine(
        code: Vec<u8>,
        results: Vec<ValueType>,
        struct_field_counts: Vec<u32>,
    ) -> WasmExecutionEngine {
        let func_type = FuncType { params: vec![], results };
        let body = FunctionBody { locals: vec![], code };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_struct_field_counts(struct_field_counts);
        engine
    }

    #[test]
    fn test_decode_struct_ops_carry_indices() {
        // struct.new 0 ; struct.get 0 1 ; struct.set 0 1 — the decoder must read
        // the type/field index immediates, not mis-decode them as opcodes.
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0xFB, 0x00, 0x00, // struct.new 0
                0xFB, 0x02, 0x00, 0x01, // struct.get 0 1
                0xFB, 0x04, 0x00, 0x01, // struct.set 0 1
                0x0B, // end
            ],
        };
        let instrs = decode_function_body(&body);
        assert_eq!(instrs.len(), 4, "three GC ops + end");
        assert!(matches!(
            instrs[0].operand,
            DecodedOperand::Gc { sub: 0x00, type_idx: 0, .. }
        ));
        assert!(matches!(
            instrs[1].operand,
            DecodedOperand::Gc { sub: 0x02, type_idx: 0, field_idx: 1 }
        ));
        assert!(matches!(
            instrs[2].operand,
            DecodedOperand::Gc { sub: 0x04, type_idx: 0, field_idx: 1 }
        ));
        assert_eq!(instrs[3].opcode, 0x0B);
    }

    #[test]
    fn test_decode_ref_null_consumes_heap_type_byte() {
        // ref.null none (0xD0 0x0F) must be ONE instruction — the 0x0F heap-type
        // byte must be consumed, not decoded as a separate instruction.
        let body = FunctionBody { locals: vec![], code: vec![0xD0, 0x0F, 0xD1, 0x0B] };
        let instrs = decode_function_body(&body);
        assert_eq!(instrs.len(), 3, "ref.null, ref.is_null, end");
        assert_eq!(instrs[0].opcode, 0xD0);
        assert_eq!(instrs[1].opcode, 0xD1, "ref.is_null");
        assert_eq!(instrs[2].opcode, 0x0B);
    }

    #[test]
    fn test_cons_car_round_trip_on_heap() {
        // The flagship slice goal: (CAR (CONS 7 9)) → 7, run on the engine.
        //   i32.const 7, i31.new      ; car = box(7)
        //   i32.const 9, i31.new      ; cdr = box(9)
        //   struct.new 0              ; $LispyPair{car, cdr}  → ref
        //   struct.get 0 0            ; read car  → i31ref
        //   i31.get_s                 ; unbox     → 7
        let code = vec![
            0x41, 0x07, 0xFB, 0x1C, // i32.const 7 ; i31.new
            0x41, 0x09, 0xFB, 0x1C, // i32.const 9 ; i31.new
            0xFB, 0x00, 0x00, // struct.new 0
            0xFB, 0x02, 0x00, 0x00, // struct.get 0 0  (car)
            0xFB, 0x1D, // i31.get_s
            0x0B, // end
        ];
        let mut engine = gc_engine(code, vec![ValueType::I32], vec![2]);
        let result = engine.call_function(0, &[]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(7)], "(CAR (CONS 7 9)) must be 7");
    }

    #[test]
    fn test_cdr_reads_second_field() {
        // (CDR (CONS 7 9)) → 9 — confirms field ordering (field 1 = cdr).
        let code = vec![
            0x41, 0x07, 0xFB, 0x1C, // box 7
            0x41, 0x09, 0xFB, 0x1C, // box 9
            0xFB, 0x00, 0x00, // struct.new 0
            0xFB, 0x02, 0x00, 0x01, // struct.get 0 1  (cdr)
            0xFB, 0x1D, // i31.get_s
            0x0B,
        ];
        let mut engine = gc_engine(code, vec![ValueType::I32], vec![2]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(9)]);
    }

    #[test]
    fn test_struct_set_mutates_field() {
        // Build (CONS 7 9), RPLACA-style overwrite car with 42, read it back.
        //   ... struct.new 0  (ref on stack)
        //   ref is consumed by struct.get, so we need two refs; instead use a
        //   local. Simpler: dup via building, set, then get on the same ref by
        //   keeping it through local 0.
        // Code: box7, box9, struct.new0 -> local.set 0 ; local.get0, box42,
        //       struct.set 0 0 ; local.get0, struct.get 0 0, i31.get_s.
        let code = vec![
            0x41, 0x07, 0xFB, 0x1C, // box 7
            0x41, 0x09, 0xFB, 0x1C, // box 9
            0xFB, 0x00, 0x00, // struct.new 0
            0x21, 0x00, // local.set 0 (the ref)
            0x20, 0x00, // local.get 0
            0x41, 0x2A, 0xFB, 0x1C, // box 42
            0xFB, 0x04, 0x00, 0x00, // struct.set 0 0  (car := 42)
            0x20, 0x00, // local.get 0
            0xFB, 0x02, 0x00, 0x00, // struct.get 0 0  (car)
            0xFB, 0x1D, // i31.get_s
            0x0B,
        ];
        // One anyref local to hold the cons ref.
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![ValueType::Anyref], code };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_struct_field_counts(vec![2]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_ref_null_is_null_true_and_false() {
        // ref.is_null(ref.null)            → 1
        let code_null = vec![0xD0, 0x0F, 0xD1, 0x0B];
        let mut e1 = gc_engine(code_null, vec![ValueType::I32], vec![]);
        assert_eq!(e1.call_function(0, &[]).unwrap(), vec![WasmValue::I32(1)]);

        // ref.is_null(struct.new 0)        → 0  (a cons is not null)
        let code_cons = vec![
            0x41, 0x01, 0xFB, 0x1C, // box 1
            0x41, 0x02, 0xFB, 0x1C, // box 2
            0xFB, 0x00, 0x00, // struct.new 0
            0xD1, // ref.is_null
            0x0B,
        ];
        let mut e2 = gc_engine(code_cons, vec![ValueType::I32], vec![2]);
        assert_eq!(e2.call_function(0, &[]).unwrap(), vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_ref_test_distinguishes_cons_from_atom_and_nil() {
        // McCarthy `pair?` lowers to `ref.test $LispyPair`. Here $LispyPair is
        // type 0 (the only struct type), so the op is `0xFB 0x14 0x00`.

        // pair?(atom 5) → 0  — a boxed integer is not a cons.
        let atom = vec![
            0x41, 0x05, 0xFB, 0x1C, // i32.const 5 ; i31.new
            0xFB, 0x14, 0x00, // ref.test $LispyPair
            0x0B,
        ];
        let mut e = gc_engine(atom, vec![ValueType::I32], vec![2]);
        assert_eq!(e.call_function(0, &[]).unwrap(), vec![WasmValue::I32(0)]);

        // pair?(cons 1 2) → 1  — a struct ref IS a cons.
        let cons = vec![
            0x41, 0x01, 0xFB, 0x1C, // box 1
            0x41, 0x02, 0xFB, 0x1C, // box 2
            0xFB, 0x00, 0x00, // struct.new $LispyPair
            0xFB, 0x14, 0x00, // ref.test $LispyPair
            0x0B,
        ];
        let mut e = gc_engine(cons, vec![ValueType::I32], vec![2]);
        assert_eq!(e.call_function(0, &[]).unwrap(), vec![WasmValue::I32(1)]);

        // pair?(nil) → 0  — the null reference is not a cons.
        let nil = vec![
            0xD0, 0x0F, // ref.null
            0xFB, 0x14, 0x00, // ref.test $LispyPair (non-null)
            0x0B,
        ];
        let mut e = gc_engine(nil, vec![ValueType::I32], vec![2]);
        assert_eq!(e.call_function(0, &[]).unwrap(), vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_ref_test_null_variant_accepts_null() {
        // The nullable `ref.test null` (0x15) additionally matches the null ref.
        let nil = vec![
            0xD0, 0x0F, // ref.null
            0xFB, 0x15, 0x00, // ref.test null $LispyPair
            0x0B,
        ];
        let mut e = gc_engine(nil, vec![ValueType::I32], vec![2]);
        assert_eq!(e.call_function(0, &[]).unwrap(), vec![WasmValue::I32(1)]);
    }

    #[test]
    fn test_struct_get_on_null_traps_cleanly() {
        // (CAR nil) — struct.get on ref.null must be a clean Err, not a panic.
        let code = vec![
            0xD0, 0x0F, // ref.null
            0xFB, 0x02, 0x00, 0x00, // struct.get 0 0
            0x0B,
        ];
        let mut engine = gc_engine(code, vec![ValueType::I32], vec![2]);
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_struct_new_without_registered_arity_traps() {
        // struct.new with no field-count table registered must trap cleanly
        // (no panic, no silent 0-field object).
        let code = vec![0xFB, 0x00, 0x00, 0x0B];
        let mut engine = gc_engine(code, vec![], vec![]); // empty arity table
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_struct_get_out_of_range_field_traps() {
        // struct.get with a field index past the end traps cleanly.
        let code = vec![
            0x41, 0x07, 0xFB, 0x1C, // box 7
            0x41, 0x09, 0xFB, 0x1C, // box 9
            0xFB, 0x00, 0x00, // struct.new 0  (2 fields)
            0xFB, 0x02, 0x00, 0x05, // struct.get 0 5  (out of range)
            0x0B,
        ];
        let mut engine = gc_engine(code, vec![ValueType::I32], vec![2]);
        assert!(engine.call_function(0, &[]).is_err());
    }

    // ── W04: real GC — end-to-end reclamation through real dispatch ────────
    //
    // These drive an actual loop through the real `execute_branch`/`br_if`
    // dispatch path (not gc.rs's own direct-call unit tests), proving the
    // *wiring* — that a real, long-running WASM loop crossing the adaptive
    // threshold is actually collected, not just that the mark/sweep
    // algorithm is correct in isolation.

    /// A loop allocates 2000 "garbage" objects (each iteration's `struct.new`
    /// overwrites the previous one's only reference) while a single `kept`
    /// object, allocated once before the loop, survives throughout. Proves
    /// both halves at once: `kept`'s field reads back correctly (nothing
    /// live was wrongly collected) and `gc_live_object_count()` stays far
    /// below 2001 (the garbage was actually reclaimed mid-run, not just
    /// leaked into an ever-growing arena — the exact gap W04 closes).
    #[test]
    fn end_to_end_loop_reclaims_garbage_and_preserves_kept_object() {
        const LIMIT: i64 = 2000; // well past gc::INITIAL_THRESHOLD (1024)

        let mut code: Vec<u8> = Vec::new();
        // kept (local 1) = struct.new(box(999))
        code.push(0x41);
        code.extend(wasm_leb128::encode_signed(999));
        code.extend([0xFB, 0x00, 0x00]); // struct.new 0
        code.extend([0x21, 0x01]); // local.set 1

        // i (local 0) = 0
        code.extend([0x41, 0x00, 0x21, 0x00]);

        // loop (empty block type)
        code.extend([0x03, 0x40]);
        {
            // garbage (local 2) = struct.new(box(i)) -- overwritten every
            // iteration, so the previous garbage object loses its only root.
            code.extend([0x20, 0x00]); // local.get 0
            code.extend([0xFB, 0x00, 0x00]); // struct.new 0
            code.extend([0x21, 0x02]); // local.set 2

            // i = i + 1
            code.extend([0x20, 0x00, 0x41, 0x01, 0x6A, 0x21, 0x00]);

            // if i < LIMIT: br_if 0 (back to the loop label)
            code.push(0x20);
            code.push(0x00); // local.get 0
            code.push(0x41);
            code.extend(wasm_leb128::encode_signed(LIMIT));
            code.push(0x48); // i32.lt_s
            code.extend([0x0D, 0x00]); // br_if 0
        }
        code.push(0x0B); // end (loop)

        // return kept.field0
        code.extend([0x20, 0x01]); // local.get 1
        code.extend([0xFB, 0x02, 0x00, 0x00]); // struct.get 0 0
        code.push(0x0B); // end (function)

        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody {
            locals: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            code,
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_struct_field_counts(vec![1]); // one struct type, one field

        let result = engine.call_function(0, &[]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(999)], "kept object's field survives intact");
        // The adaptive threshold floors at gc::INITIAL_THRESHOLD (1024, mirroring
        // FlatHeap's own heuristic verbatim), so with LIMIT=2000 only one full
        // collection cycle completes before the loop ends — the live count
        // settles well under half of everything ever allocated, not anywhere
        // near LIMIT + 1, which is the actual, meaningful proof of reclamation
        // (an uncollected arena would report exactly 2001 here).
        assert!(
            engine.gc_live_object_count() < (LIMIT as usize) / 2,
            "garbage was reclaimed mid-loop, not left to accumulate to ~{}: got {}",
            LIMIT + 1,
            engine.gc_live_object_count()
        );
        assert!(engine.gc_profile().total_collections >= 1, "at least one real collection ran");
        assert!(
            engine.gc_profile().total_freed as i64 >= LIMIT / 2,
            "a substantial number of garbage objects were actually freed, not just some: got {}",
            engine.gc_profile().total_freed
        );
    }

    #[test]
    fn test_ref_value_round_trips_through_typed_stack() {
        // A Ref must survive to_typed/from_typed unchanged (handle + null).
        for v in [WasmValue::Ref(None), WasmValue::Ref(Some(0)), WasmValue::Ref(Some(7))] {
            let tv = v.to_typed();
            assert_eq!(WasmValue::from_typed(&tv).unwrap(), v);
        }
    }

    #[test]
    fn test_wrapping_arithmetic() {
        // Test i32 overflow wraps
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B], // add
        };

        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });

        let result = engine
            .call_function(0, &[WasmValue::I32(i32::MAX), WasmValue::I32(1)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(i32::MIN)]); // wraps
    }

    // ══════════════════════════════════════════════════════════════════════
    // Value constructors and conversions
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_wasm_value_default_for_all_types() {
        assert_eq!(WasmValue::default_for(ValueType::I32), WasmValue::I32(0));
        assert_eq!(WasmValue::default_for(ValueType::I64), WasmValue::I64(0));
        assert_eq!(WasmValue::default_for(ValueType::F32), WasmValue::F32(0.0));
        assert_eq!(WasmValue::default_for(ValueType::F64), WasmValue::F64(0.0));
        // GC reference defaults (LANG77 L3b-3a-3b): an i31ref defaults to its
        // i32 payload, the nullable reference types default to the null ref.
        assert_eq!(WasmValue::default_for(ValueType::I31ref), WasmValue::I32(0));
        assert_eq!(WasmValue::default_for(ValueType::Anyref), WasmValue::Ref(None));
        assert_eq!(
            WasmValue::default_for(ValueType::StructRef(0)),
            WasmValue::Ref(None)
        );
    }

    #[test]
    fn test_wasm_value_all_type_mismatches() {
        // I32 cannot extract as other types
        assert!(WasmValue::I32(0).as_i64().is_err());
        assert!(WasmValue::I32(0).as_f32().is_err());
        assert!(WasmValue::I32(0).as_f64().is_err());

        // I64 cannot extract as other types
        assert!(WasmValue::I64(0).as_i32().is_err());
        assert!(WasmValue::I64(0).as_f32().is_err());
        assert!(WasmValue::I64(0).as_f64().is_err());

        // F32 cannot extract as other types
        assert!(WasmValue::F32(0.0).as_i32().is_err());
        assert!(WasmValue::F32(0.0).as_i64().is_err());
        assert!(WasmValue::F32(0.0).as_f64().is_err());

        // F64 cannot extract as other types
        assert!(WasmValue::F64(0.0).as_i32().is_err());
        assert!(WasmValue::F64(0.0).as_i64().is_err());
        assert!(WasmValue::F64(0.0).as_f32().is_err());
    }

    #[test]
    fn test_wasm_value_edge_values() {
        assert_eq!(WasmValue::I32(i32::MIN).as_i32().unwrap(), i32::MIN);
        assert_eq!(WasmValue::I32(i32::MAX).as_i32().unwrap(), i32::MAX);
        assert_eq!(WasmValue::I64(i64::MIN).as_i64().unwrap(), i64::MIN);
        assert_eq!(WasmValue::I64(i64::MAX).as_i64().unwrap(), i64::MAX);
        assert!(WasmValue::F32(f32::NAN).as_f32().unwrap().is_nan());
        assert_eq!(
            WasmValue::F32(f32::INFINITY).as_f32().unwrap(),
            f32::INFINITY
        );
        assert_eq!(
            WasmValue::F64(f64::NEG_INFINITY).as_f64().unwrap(),
            f64::NEG_INFINITY
        );
    }

    #[test]
    fn test_wasm_value_from_typed_bad_type() {
        use virtual_machine::{TypedVMValue, Value};
        // Unknown value type byte
        let tv = TypedVMValue {
            value_type: 0xFF,
            value: Value::Int(0),
        };
        assert!(WasmValue::from_typed(&tv).is_err());

        // Wrong value variant for i32 type (0x7F = I32 tag)
        let tv_bad = TypedVMValue {
            value_type: 0x7F,
            value: Value::Float(1.0),
        };
        assert!(WasmValue::from_typed(&tv_bad).is_err());

        // Wrong value variant for f64 type (0x7C = F64 tag)
        let tv_bad2 = TypedVMValue {
            value_type: 0x7C,
            value: Value::Int(1),
        };
        assert!(WasmValue::from_typed(&tv_bad2).is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // TrapError
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_trap_error_display() {
        let err = TrapError::new("divide by zero");
        assert_eq!(format!("{}", err), "TrapError: divide by zero");
    }

    #[test]
    fn test_trap_error_into_vm_error() {
        let trap = TrapError::new("test trap");
        let vm_err: VMError = trap.into();
        match vm_err {
            VMError::GenericError(msg) => assert_eq!(msg, "test trap"),
            _ => panic!("expected GenericError"),
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // LinearMemory: all load/store widths, grow, OOB
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_memory_i64_store_load() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i64(0, 0x0102030405060708).unwrap();
        assert_eq!(mem.load_i64(0).unwrap(), 0x0102030405060708);
    }

    #[test]
    fn test_memory_f32_store_load() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_f32(0, 3.14).unwrap();
        assert!((mem.load_f32(0).unwrap() - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_memory_f64_store_load() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_f64(0, std::f64::consts::PI).unwrap();
        assert!((mem.load_f64(0).unwrap() - std::f64::consts::PI).abs() < 1e-15);
    }

    #[test]
    fn test_memory_i32_8s_sign_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i32_8(0, 0xFF).unwrap(); // -1 as i8
        assert_eq!(mem.load_i32_8s(0).unwrap(), -1);
        mem.store_i32_8(0, 0x7F).unwrap(); // 127
        assert_eq!(mem.load_i32_8s(0).unwrap(), 127);
    }

    #[test]
    fn test_memory_i32_8u_zero_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i32_8(0, 0xFF).unwrap();
        assert_eq!(mem.load_i32_8u(0).unwrap(), 255);
    }

    #[test]
    fn test_memory_i32_16s_sign_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i32_16(0, -1).unwrap();
        assert_eq!(mem.load_i32_16s(0).unwrap(), -1);
        mem.store_i32_16(0, 0x7FFF).unwrap();
        assert_eq!(mem.load_i32_16s(0).unwrap(), 32767);
    }

    #[test]
    fn test_memory_i32_16u_zero_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i32_16(0, -1).unwrap(); // 0xFFFF as u16
        assert_eq!(mem.load_i32_16u(0).unwrap(), 65535);
    }

    #[test]
    fn test_memory_i64_8s_sign_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i64_8(0, 0xFF).unwrap();
        assert_eq!(mem.load_i64_8s(0).unwrap(), -1i64);
    }

    #[test]
    fn test_memory_i64_8u_zero_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i64_8(0, 0xFF).unwrap();
        assert_eq!(mem.load_i64_8u(0).unwrap(), 255i64);
    }

    #[test]
    fn test_memory_i64_16s_sign_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i64_16(0, -1).unwrap();
        assert_eq!(mem.load_i64_16s(0).unwrap(), -1i64);
    }

    #[test]
    fn test_memory_i64_16u_zero_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i64_16(0, -1).unwrap();
        assert_eq!(mem.load_i64_16u(0).unwrap(), 65535i64);
    }

    #[test]
    fn test_memory_i64_32s_sign_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i64_32(0, -1).unwrap();
        assert_eq!(mem.load_i64_32s(0).unwrap(), -1i64);
    }

    #[test]
    fn test_memory_i64_32u_zero_extension() {
        let mut mem = LinearMemory::new(1, None);
        mem.store_i64_32(0, -1).unwrap();
        assert_eq!(mem.load_i64_32u(0).unwrap(), 0xFFFFFFFFi64);
    }

    #[test]
    fn test_memory_oob_all_widths() {
        let mem = LinearMemory::new(1, None);
        let end = 65536;
        // i32 OOB (needs 4 bytes at boundary)
        assert!(mem.load_i32(end - 3).is_err());
        // i64 OOB
        assert!(mem.load_i64(end - 7).is_err());
        // f32 OOB
        assert!(mem.load_f32(end - 3).is_err());
        // f64 OOB
        assert!(mem.load_f64(end - 7).is_err());
        // narrow loads OOB
        assert!(mem.load_i32_8s(end).is_err());
        assert!(mem.load_i32_8u(end).is_err());
        assert!(mem.load_i32_16s(end - 1).is_err());
        assert!(mem.load_i32_16u(end - 1).is_err());
        assert!(mem.load_i64_8s(end).is_err());
        assert!(mem.load_i64_8u(end).is_err());
        assert!(mem.load_i64_16s(end - 1).is_err());
        assert!(mem.load_i64_16u(end - 1).is_err());
        assert!(mem.load_i64_32s(end - 3).is_err());
        assert!(mem.load_i64_32u(end - 3).is_err());
    }

    #[test]
    fn test_memory_store_oob() {
        let mut mem = LinearMemory::new(1, None);
        let end = 65536;
        assert!(mem.store_i32(end - 3, 0).is_err());
        assert!(mem.store_i64(end - 7, 0).is_err());
        assert!(mem.store_f32(end - 3, 0.0).is_err());
        assert!(mem.store_f64(end - 7, 0.0).is_err());
        assert!(mem.store_i32_8(end, 0).is_err());
        assert!(mem.store_i32_16(end - 1, 0).is_err());
        assert!(mem.store_i64_8(end, 0).is_err());
        assert!(mem.store_i64_16(end - 1, 0).is_err());
        assert!(mem.store_i64_32(end - 3, 0).is_err());
    }

    #[test]
    fn test_memory_grow_no_max() {
        let mut mem = LinearMemory::new(1, None);
        assert_eq!(mem.grow(2), 1); // old pages = 1
        assert_eq!(mem.size(), 3);
        assert_eq!(mem.data.len(), 3 * PAGE_SIZE);
    }

    #[test]
    fn test_memory_grow_exceeds_spec_max() {
        let mut mem = LinearMemory::new(1, None);
        // Spec max is 65536 pages
        assert_eq!(mem.grow(65536), -1); // 1 + 65536 > 65536
    }

    #[test]
    fn test_memory_grow_zero() {
        let mut mem = LinearMemory::new(2, Some(4));
        assert_eq!(mem.grow(0), 2); // returns old pages, no change
        assert_eq!(mem.size(), 2);
    }

    /// End-to-end proof `memory.size`/`memory.grow` actually target the
    /// memory index they decode (multi-memory, W16, task #85), not always
    /// memory 0 -- growing memory 1 must leave memory 0 completely
    /// unaffected, and `memory.size 0` read AFTER that grow must still
    /// report memory 0's own, untouched page count.
    #[test]
    fn memory_size_and_grow_target_the_correct_memory_by_index() {
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32, ValueType::I32] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x02, // i32.const 2 (delta)
                0x40, 0x01, // memory.grow memidx=1 -- pushes memory 1's OLD page count
                0x3F, 0x00, // memory.size memidx=0 -- pushes memory 0's CURRENT page count
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![LinearMemory::new(1, None), LinearMemory::new(3, None)],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        let results = engine.call_function(0, &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(3), WasmValue::I32(1)], "memory 1's old size was 3; memory 0's size must stay 1, unaffected by growing memory 1");

        let state = engine.into_state();
        assert_eq!(state.memories[0].size(), 1, "memory 0 must not have grown");
        assert_eq!(state.memories[1].size(), 5, "memory 1 must have grown by 2 (3 -> 5)");
    }

    /// Security review (task #101): a per-memory 65536-page cap alone
    /// still permits an AGGREGATE resource-exhaustion DoS across many
    /// memories -- `memory_grow_would_exceed_aggregate_cap` (extracted as
    /// a pure function specifically so it's cheaply testable with small
    /// synthetic page counts, since `MAX_TOTAL_MEMORY_PAGES` itself is
    /// 65536 pages = 4GB, far too large to actually allocate in a unit
    /// test) is the runtime counterpart to `wasm-validator`'s declare-time
    /// "Check 1b". These tests pin the exact threshold arithmetic the
    /// `0x40` (`memory.grow`) handler relies on.
    #[test]
    fn memory_grow_aggregate_cap_rejects_a_small_target_growth_when_another_memory_is_already_at_the_cap() {
        // Memory 0 is (synthetically) already at the full aggregate cap;
        // growing memory 1 by just 1 page must still be rejected, because
        // the CROSS-MEMORY total would exceed the cap -- even though
        // memory 1's own per-memory check would trivially pass on its
        // own (1 page is nowhere near the per-memory 65536-page cap in
        // isolation).
        let current_pages = [MAX_TOTAL_MEMORY_PAGES, 0];
        assert!(memory_grow_would_exceed_aggregate_cap(&current_pages, 1, 1));
    }

    #[test]
    fn memory_grow_aggregate_cap_allows_growth_exactly_up_to_the_cap() {
        // Two memories whose pages, plus the delta, sum to EXACTLY
        // `MAX_TOTAL_MEMORY_PAGES` -- must be allowed (the check is `>`,
        // not `>=`).
        let current_pages = [100u32, 200];
        let delta = MAX_TOTAL_MEMORY_PAGES - 300;
        assert!(!memory_grow_would_exceed_aggregate_cap(&current_pages, 0, delta));
    }

    #[test]
    fn memory_grow_aggregate_cap_rejects_one_page_past_the_cap() {
        let current_pages = [100u32, 200];
        let delta = MAX_TOTAL_MEMORY_PAGES - 300 + 1;
        assert!(memory_grow_would_exceed_aggregate_cap(&current_pages, 0, delta));
    }

    #[test]
    fn memory_grow_aggregate_cap_arithmetic_does_not_overflow_with_a_huge_delta() {
        // `delta` can be as large as `u32::MAX` (a WASM module's i32
        // operand, reinterpreted). `u64` arithmetic throughout must
        // correctly reject this without wrapping.
        let current_pages = [0u32];
        assert!(memory_grow_would_exceed_aggregate_cap(&current_pages, 0, u32::MAX));
    }

    /// End-to-end wiring proof: growth that stays well under the
    /// aggregate cap across multiple memories must still succeed and
    /// leave every other memory unaffected -- confirms the aggregate
    /// check doesn't wrongly reject ordinary, small multi-memory growth
    /// (the happy path this whole feature must not break).
    #[test]
    fn memory_grow_aggregate_cap_does_not_block_ordinary_small_multi_memory_growth() {
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x02, // i32.const 2 (delta)
                0x40, 0x01, // memory.grow memidx=1
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![LinearMemory::new(1, None), LinearMemory::new(3, None)],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(3)]); // old size

        let state = engine.into_state();
        assert_eq!(state.memories[0].size(), 1, "memory 0 must be untouched");
        assert_eq!(state.memories[1].size(), 5, "memory 1 must have grown by 2 (3 -> 5)");
    }

    #[test]
    fn test_memory_write_bytes() {
        let mut mem = LinearMemory::new(1, None);
        mem.write_bytes(10, &[1, 2, 3, 4]).unwrap();
        assert_eq!(mem.load_i32_8u(10).unwrap(), 1);
        assert_eq!(mem.load_i32_8u(13).unwrap(), 4);
    }

    #[test]
    fn test_memory_write_bytes_oob() {
        let mut mem = LinearMemory::new(1, None);
        assert!(mem.write_bytes(65534, &[1, 2, 3]).is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // Table: get/set, OOB
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_table_set_and_get() {
        let mut table = Table::new(10, Some(20));
        table.set(0, Some(5)).unwrap();
        table.set(9, Some(99)).unwrap();
        assert_eq!(table.get(0).unwrap(), Some(5));
        assert_eq!(table.get(9).unwrap(), Some(99));
        assert_eq!(table.get(1).unwrap(), None);
    }

    #[test]
    fn test_table_set_oob() {
        let mut table = Table::new(3, None);
        assert!(table.set(3, Some(1)).is_err());
        assert!(table.set(100, Some(1)).is_err());
    }

    #[test]
    fn test_table_get_oob() {
        let table = Table::new(3, None);
        assert!(table.get(3).is_err());
        assert!(table.get(100).is_err());
    }

    #[test]
    fn test_table_set_none() {
        let mut table = Table::new(5, None);
        table.set(2, Some(42)).unwrap();
        assert_eq!(table.get(2).unwrap(), Some(42));
        table.set(2, None).unwrap();
        assert_eq!(table.get(2).unwrap(), None);
    }

    // ══════════════════════════════════════════════════════════════════════
    // WASM17: ref.func, table.get, table.set opcode handlers
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_ref_func_pushes_non_null_funcref_for_valid_index() {
        // ref.func 0 -- the function's own index, always valid for a
        // single-function module.
        let code = vec![0xD2, 0x00, 0x0B];
        let func_type = FuncType { params: vec![], results: vec![ValueType::Funcref] };
        let body = FunctionBody { locals: vec![], code };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::Ref(Some(0))]);
    }

    #[test]
    fn test_ref_func_out_of_range_index_is_a_clean_error_not_a_panic() {
        let code = vec![0xD2, 0x63, 0x0B]; // ref.func 99 -- no function 99 exists
        let func_type = FuncType { params: vec![], results: vec![ValueType::Funcref] };
        let body = FunctionBody { locals: vec![], code };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_table_get_set_opcodes_round_trip_through_a_real_table() {
        // table.set 0 (i32.const 3) (ref.func 0); table.get 0 (i32.const 3)
        let code = vec![
            0x41, 0x03, // i32.const 3
            0xD2, 0x00, // ref.func 0
            0x26, 0x00, // table.set 0
            0x41, 0x03, // i32.const 3
            0x25, 0x00, // table.get 0
            0x0B,
        ];
        let func_type = FuncType { params: vec![], results: vec![ValueType::Funcref] };
        let body = FunctionBody { locals: vec![], code };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(10, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::Ref(Some(0))]);
    }

    #[test]
    fn test_table_get_uninitialized_slot_returns_null_ref() {
        let code = vec![0x41, 0x00, 0x25, 0x00, 0x0B]; // i32.const 0; table.get 0
        let func_type = FuncType { params: vec![], results: vec![ValueType::Funcref] };
        let body = FunctionBody { locals: vec![], code };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::Ref(None)]);
    }

    #[test]
    fn test_table_get_out_of_bounds_index_is_a_clean_error_not_a_panic() {
        let code = vec![0x41, 0x63, 0x25, 0x00, 0x0B]; // i32.const 99; table.get 0
        let func_type = FuncType { params: vec![], results: vec![ValueType::Funcref] };
        let body = FunctionBody { locals: vec![], code };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_return_call_indirect_through_a_table_entry_referencing_an_undefined_function_is_a_clean_error_not_a_panic() {
        // WASM16 security review: a table entry is DATA, not a static
        // part of the bytecode a validator necessarily already checked
        // (this engine can be, and in real usage sometimes is, driven
        // without going through wasm-validator first) -- a crafted or
        // corrupt table slot pointing past `func_types.len()` must trap
        // cleanly, the same discipline `test_table_get_out_of_bounds_
        // index_is_a_clean_error_not_a_panic` above already established
        // for `table.get`.
        let code = vec![0x41, 0x00, 0x13, 0x00, 0x00, 0x0B]; // i32.const 0; return_call_indirect type=0 table=0; end
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![], code };
        let mut table = Table::new(1, None);
        table.set(0, Some(99)).unwrap(); // function index 99 doesn't exist -- only index 0 (this function itself) does
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![table],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_call_indirect_through_a_table_entry_referencing_an_undefined_function_is_a_clean_error_not_a_panic() {
        // Same bug class as `return_call_indirect`'s regression test above,
        // on `call_indirect` (0x11) itself. The vulnerable branch only
        // runs when `ctx.types.get(type_idx)` is `Some` (a type section is
        // set -- the common real-world case, e.g. via `wasm-runtime`), so
        // this test must call `set_type_section` to actually exercise it,
        // unlike `return_call_indirect`'s fix which is unconditional. A
        // hand-built engine with a corrupt table entry used to panic on
        // the direct `ctx.func_types[func_index]` index inside that block.
        let code = vec![0x41, 0x00, 0x11, 0x00, 0x00, 0x0B]; // i32.const 0; call_indirect type=0 table=0; end
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![], code };
        let mut table = Table::new(1, None);
        table.set(0, Some(99)).unwrap(); // function index 99 doesn't exist -- only index 0 (this function itself) does
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![table],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type.clone()],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_type_section(vec![func_type]);
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_call_indirect_with_explicit_nonzero_table_index_calls_through_the_named_table() {
        // Task #107: `call_indirect`'s `table_idx` immediate used to be
        // decoded correctly but silently dropped in `convert_operand`,
        // so every `call_indirect` ran against table 0 regardless of
        // what the bytecode actually named. Two tables here hold
        // DIFFERENT functions at the same slot (index 0); a
        // `call_indirect type=0 table=1` must reach table 1's function
        // (returning 22), not table 0's (11) -- proving the real table
        // index is read and used, not just that decoding doesn't crash.
        // (11/22 are single-byte signed-LEB128-safe, -64..=63; 111/222
        // are not and would silently sign-extend to the wrong value.)
        let code = vec![0x41, 0x00, 0x11, 0x00, 0x01, 0x0B]; // i32.const 0; call_indirect type=0 table=1; end
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let caller_body = FunctionBody { locals: vec![], code };
        let table0_target = FunctionBody { locals: vec![], code: vec![0x41, 11, 0x0B] }; // func 1: i32.const 11
        let table1_target = FunctionBody { locals: vec![], code: vec![0x41, 22, 0x0B] }; // func 2: i32.const 22
        let mut table0 = Table::new(1, None);
        table0.set(0, Some(1)).unwrap();
        let mut table1 = Table::new(1, None);
        table1.set(0, Some(2)).unwrap();
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![table0, table1],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type.clone(), func_type.clone(), func_type.clone()],
            func_bodies: vec![Some(caller_body), Some(table0_target), Some(table1_target)],
            host_functions: vec![None, None, None],
        });
        engine.set_type_section(vec![func_type]);
        let result = engine.call_function(0, &[]).expect("call_indirect through table 1 should succeed");
        assert_eq!(result, vec![WasmValue::I32(22)]);
    }

    // ══════════════════════════════════════════════════════════════════════
    // Task #92/W18: real multi-memory memarg
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_i32_load_with_explicit_nonzero_memidx_reads_the_named_memory() {
        // The multi-memory proposal's memarg flags-bit 0x40 + trailing
        // memidx immediate used to be entirely unread (the decoder never
        // even checked bit 6), so every load/store always ran against
        // memory 0 regardless of what the bytecode named. Two memories
        // here hold DIFFERENT values at the same offset; an
        // `i32.load align=2|0x40 offset=0 memidx=1` must read memory 1's
        // value (99), not memory 0's (0, its untouched default) --
        // proving the real memory index is decoded and used, not just
        // that decoding doesn't crash.
        let code = vec![
            0x41, 0x00, // i32.const 0 (address)
            0x28, 0x42, 0x00, 0x01, // i32.load align=(0x40|2) offset=0 memidx=1
            0x0B, // end
        ];
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![], code };
        let mem0 = LinearMemory::new(1, None);
        let mut mem1 = LinearMemory::new(1, None);
        mem1.store_i32(0, 99).unwrap();
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![mem0, mem1],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        let result = engine.call_function(0, &[]).expect("i32.load through memory 1 should succeed");
        assert_eq!(result, vec![WasmValue::I32(99)]);
    }

    #[test]
    fn test_memory_fill_with_explicit_nonzero_memidx_fills_the_named_memory() {
        // `memory.fill`'s memidx immediate used to be assumed a fixed
        // single MVP-only byte and discarded entirely -- every fill
        // always targeted memory 0. `memory.fill memidx=1` must leave
        // memory 0 untouched (still 0) and fill memory 1 with 7s.
        let code = vec![
            0x41, 0x00, // i32.const 0 (dest)
            0x41, 0x07, // i32.const 7 (value)
            0x41, 0x04, // i32.const 4 (len)
            0xFC, 0x0B, 0x01, // memory.fill memidx=1
            0x0B, // end
        ];
        let func_type = FuncType { params: vec![], results: vec![] };
        let body = FunctionBody { locals: vec![], code };
        let mem0 = LinearMemory::new(1, None);
        let mem1 = LinearMemory::new(1, None);
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![mem0, mem1],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.call_function(0, &[]).expect("memory.fill through memory 1 should succeed");
        let state = engine.into_state();
        assert_eq!(&state.memories[0].data[0..4], &[0, 0, 0, 0], "memory 0 must be untouched");
        assert_eq!(&state.memories[1].data[0..4], &[7, 7, 7, 7], "memory 1 must be filled");
    }

    #[test]
    fn test_memory_copy_between_two_different_memories() {
        // `memory.copy`'s dst/src memidx immediates used to be assumed
        // fixed single MVP-only bytes and discarded entirely -- every
        // copy always operated within memory 0. `memory.copy dst_mem=1
        // src_mem=0` must read from memory 0 and write into memory 1,
        // leaving memory 0 itself unchanged (proving neither side is
        // hardcoded, and that this isn't a same-memory self-copy).
        let code = vec![
            0x41, 0x00, // i32.const 0 (dest, in memory 1)
            0x41, 0x00, // i32.const 0 (src, in memory 0)
            0x41, 0x04, // i32.const 4 (len)
            0xFC, 0x0A, 0x01, 0x00, // memory.copy dst_memidx=1 src_memidx=0
            0x0B, // end
        ];
        let func_type = FuncType { params: vec![], results: vec![] };
        let body = FunctionBody { locals: vec![], code };
        let mut mem0 = LinearMemory::new(1, None);
        mem0.store_i32(0, 0x11223344).unwrap();
        let mem1 = LinearMemory::new(1, None);
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![mem0, mem1],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.call_function(0, &[]).expect("cross-memory memory.copy should succeed");
        let state = engine.into_state();
        assert_eq!(state.memories[0].load_i32(0).unwrap(), 0x11223344, "memory 0 (source) must be unchanged");
        assert_eq!(state.memories[1].load_i32(0).unwrap(), 0x11223344, "memory 1 (destination) must have the copied value");
    }

    #[test]
    fn test_memory_init_with_explicit_nonzero_memidx_writes_the_named_memory() {
        // `memory.init`'s trailing memidx immediate used to be skipped
        // as a fixed single byte and discarded -- every init always
        // wrote into memory 0. `memory.init $d memidx=1` must leave
        // memory 0 untouched and write the segment's bytes into memory 1.
        let code = vec![
            0x41, 0x00, // i32.const 0 (dest)
            0x41, 0x00, // i32.const 0 (src)
            0x41, 0x02, // i32.const 2 (len)
            0xFC, 0x08, 0x00, 0x01, // memory.init data_idx=0 memidx=1
            0x0B, // end
        ];
        let func_type = FuncType { params: vec![], results: vec![] };
        let body = FunctionBody { locals: vec![], code };
        let mem0 = LinearMemory::new(1, None);
        let mem1 = LinearMemory::new(1, None);
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![mem0, mem1],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_data_segments(vec![vec![0xAA, 0xBB]]);
        engine.set_dropped_data_segments(vec![false]);
        engine.call_function(0, &[]).expect("memory.init through memory 1 should succeed");
        let state = engine.into_state();
        assert_eq!(&state.memories[0].data[0..2], &[0, 0], "memory 0 must be untouched");
        assert_eq!(&state.memories[1].data[0..2], &[0xAA, 0xBB], "memory 1 must hold the segment's bytes");
    }

    // ══════════════════════════════════════════════════════════════════════
    // WASM10: dedicated-thread call_function, cross-module nesting guard
    // ══════════════════════════════════════════════════════════════════════

    /// Security review (WASM10): white-box test of `MAX_DEDICATED_THREAD_
    /// DEPTH`'s guard itself -- directly sets this thread's own depth
    /// counter to the maximum (simulating what it would read if a real
    /// chain of that many cross-module `HostFunction` re-entries into
    /// `call_function` got this far -- see `wasm10_dedicated_thread.rs`'s
    /// own integration test for the end-to-end propagation proof) and
    /// confirms `call_function` rejects cleanly -- crucially, WITHOUT
    /// spawning another OS thread at all -- rather than only catching the
    /// problem after actually exhausting real threads.
    #[test]
    fn dedicated_thread_depth_guard_traps_without_spawning_when_already_at_max() {
        DEDICATED_THREAD_DEPTH.with(|d| d.set(MAX_DEDICATED_THREAD_DEPTH));
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![], code: vec![0x41, 0x2a, 0x0B] }; // i32.const 42; end
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        let result = engine.call_function(0, &[]);
        DEDICATED_THREAD_DEPTH.with(|d| d.set(0)); // don't leak into other tests on this thread
        assert!(result.is_err(), "call_function must reject once already at MAX_DEDICATED_THREAD_DEPTH");
        assert!(result.unwrap_err().to_string().contains("cross-module call nesting exhausted"));
    }

    /// Companion: confirms the guard does NOT false-trip on ordinary,
    /// shallow calls -- depth 0 (the default for any thread that never
    /// nested through a nested `call_function`) must succeed normally.
    #[test]
    fn dedicated_thread_depth_guard_does_not_trip_at_the_default_depth() {
        DEDICATED_THREAD_DEPTH.with(|d| d.set(0));
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![], code: vec![0x41, 0x2a, 0x0B] }; // i32.const 42; end
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(42)]);
    }

    // ══════════════════════════════════════════════════════════════════════
    // SIMD (v128) first slice -- see code/specs/
    // W13-wasm-simd-v128-first-slice.md
    // ══════════════════════════════════════════════════════════════════════

    fn simd_engine(code: Vec<u8>) -> WasmExecutionEngine {
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![], code };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    fn v128_const_bytes(lanes: [i32; 4]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        for lane in lanes {
            bytes.extend_from_slice(&lane.to_le_bytes());
        }
        bytes
    }

    /// `v128.const` + `i32x4.extract_lane` round-trip: proves the const
    /// pool (`ctx.simd_consts`) and the v128 heap handle mechanism both
    /// work end to end, not just that the code compiles.
    #[test]
    fn v128_const_then_extract_lane_round_trips_every_lane() {
        for (lane_idx, expected) in [(0, 10), (1, 20), (2, 30), (3, 40)] {
            let mut code = vec![0xFD, 0x0C]; // v128.const
            code.extend(v128_const_bytes([10, 20, 30, 40]));
            code.extend([0xFD, 0x1B, lane_idx]); // i32x4.extract_lane <lane_idx>
            code.push(0x0B); // end
            let mut engine = simd_engine(code);
            assert_eq!(
                engine.call_function(0, &[]).unwrap(),
                vec![WasmValue::I32(expected)],
                "lane {lane_idx}"
            );
        }
    }

    /// `i32x4.splat`: one scalar broadcast into all 4 lanes.
    #[test]
    fn i32x4_splat_broadcasts_into_every_lane() {
        let code = vec![
            0x41, 0x07, // i32.const 7
            0xFD, 0x11, // i32x4.splat
            0xFD, 0x1B, 0x02, // i32x4.extract_lane 2 (any lane should be 7)
            0x0B,
        ];
        let mut engine = simd_engine(code);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(7)]);
    }

    /// `i32x4.add`: real lane-wise addition, not just "returns some v128" --
    /// verifies each lane's SPECIFIC computed value via `extract_lane`,
    /// including wrapping overflow (the same semantics scalar `i32.add`
    /// already has).
    #[test]
    fn i32x4_add_computes_real_lane_wise_wrapping_sums() {
        let mut code = vec![0xFD, 0x0C];
        code.extend(v128_const_bytes([1, 2, 3, i32::MAX]));
        code.extend([0xFD, 0x0C]);
        code.extend(v128_const_bytes([10, 20, 30, 1]));
        code.extend([0xFD, 0xAE, 0x01]); // i32x4.add (LEB128: [0xAE, 0x01] for sub-opcode 174)
        code.extend([0xFD, 0x1B, 0x03]); // extract_lane 3 -- checks the wrapping case specifically
        code.push(0x0B);
        let mut engine = simd_engine(code);
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(i32::MAX.wrapping_add(1))],
            "lane 3 must wrap exactly like scalar i32.add does"
        );
    }

    /// `i32x4.eq`: WASM's boolean-mask convention (-1 for equal, 0 for
    /// not), not a plain 0/1 the way scalar `i32.eq` works.
    #[test]
    fn i32x4_eq_produces_the_all_ones_all_zeros_mask_convention() {
        let mut code = vec![0xFD, 0x0C];
        code.extend(v128_const_bytes([5, 6, 7, 8]));
        code.extend([0xFD, 0x0C]);
        code.extend(v128_const_bytes([5, 0, 7, 0]));
        code.extend([0xFD, 0x37]); // i32x4.eq
        code.extend([0xFD, 0x1B, 0x00]); // extract_lane 0 -- equal
        code.push(0x0B);
        let mut engine = simd_engine(code.clone());
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(-1)], "equal lane must be all-1s (-1)");

        let mut code2 = vec![0xFD, 0x0C];
        code2.extend(v128_const_bytes([5, 6, 7, 8]));
        code2.extend([0xFD, 0x0C]);
        code2.extend(v128_const_bytes([5, 0, 7, 0]));
        code2.extend([0xFD, 0x37]);
        code2.extend([0xFD, 0x1B, 0x01]); // extract_lane 1 -- not equal
        code2.push(0x0B);
        let mut engine2 = simd_engine(code2);
        assert_eq!(engine2.call_function(0, &[]).unwrap(), vec![WasmValue::I32(0)], "unequal lane must be all-0s (0)");
    }

    /// `i32x4.sub`/`i32x4.mul`: real lane-wise arithmetic (SIMD widening,
    /// task #113-117), verified via `extract_lane` the same way `i32x4.add`
    /// already is above, including wrapping.
    #[test]
    fn i32x4_sub_and_mul_compute_real_lane_wise_wrapping_results() {
        let mut sub_code = vec![0xFD, 0x0C];
        sub_code.extend(v128_const_bytes([10, 20, 30, i32::MIN]));
        sub_code.extend([0xFD, 0x0C]);
        sub_code.extend(v128_const_bytes([1, 2, 3, 1]));
        sub_code.extend([0xFD, 0xB1, 0x01]); // i32x4.sub (LEB128 for sub-opcode 0xB1 = 177)
        sub_code.extend([0xFD, 0x1B, 0x03]); // extract_lane 3 -- checks the wrapping case
        sub_code.push(0x0B);
        let mut sub_engine = simd_engine(sub_code);
        assert_eq!(
            sub_engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(i32::MIN.wrapping_sub(1))],
            "lane 3 must wrap exactly like scalar i32.sub does"
        );

        let mut mul_code = vec![0xFD, 0x0C];
        mul_code.extend(v128_const_bytes([2, 3, 4, i32::MAX]));
        mul_code.extend([0xFD, 0x0C]);
        mul_code.extend(v128_const_bytes([5, 6, 7, 2]));
        mul_code.extend([0xFD, 0xB5, 0x01]); // i32x4.mul (LEB128 for sub-opcode 0xB5 = 181)
        mul_code.extend([0xFD, 0x1B, 0x03]); // extract_lane 3 -- checks the wrapping case
        mul_code.push(0x0B);
        let mut mul_engine = simd_engine(mul_code);
        assert_eq!(
            mul_engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(i32::MAX.wrapping_mul(2))],
            "lane 3 must wrap exactly like scalar i32.mul does"
        );
    }

    /// `i32x4.neg`: UNARY, unlike every other arithmetic op tested above
    /// (pops exactly one v128, not two) -- also verifies wrapping negation
    /// (`i32::MIN.wrapping_neg() == i32::MIN`, the one value that doesn't
    /// negate to its "intuitive" positive counterpart).
    #[test]
    fn i32x4_neg_computes_real_lane_wise_wrapping_negation() {
        let mut code = vec![0xFD, 0x0C];
        code.extend(v128_const_bytes([5, -5, 0, i32::MIN]));
        code.extend([0xFD, 0xA1, 0x01]); // i32x4.neg (LEB128 for sub-opcode 0xA1 = 161)
        code.extend([0xFD, 0x1B, 0x03]); // extract_lane 3 -- the wrapping edge case
        code.push(0x0B);
        let mut engine = simd_engine(code);
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(i32::MIN)],
            "negating i32::MIN must wrap back to i32::MIN, matching scalar wrapping_neg"
        );
    }

    /// `i32x4.ne`/signed-vs-unsigned comparison family (SIMD widening,
    /// task #113-117): the boolean-mask convention extends to every new
    /// predicate, and -- the one place a signed/unsigned bug could
    /// actually hide -- `lt_u` on a negative i32 lane (which is a LARGE
    /// value when reinterpreted as u32) must disagree with `lt_s` on the
    /// exact same bit pattern.
    #[test]
    fn i32x4_comparison_family_uses_the_mask_convention_and_distinguishes_signed_from_unsigned() {
        let mut ne_code = vec![0xFD, 0x0C];
        ne_code.extend(v128_const_bytes([5, 6, 7, 8]));
        ne_code.extend([0xFD, 0x0C]);
        ne_code.extend(v128_const_bytes([5, 0, 7, 0]));
        ne_code.extend([0xFD, 0x38]); // i32x4.ne
        ne_code.extend([0xFD, 0x1B, 0x01]); // extract_lane 1 -- 6 != 0
        ne_code.push(0x0B);
        let mut ne_engine = simd_engine(ne_code);
        assert_eq!(ne_engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(-1)], "unequal lane must be all-1s (-1)");

        // -1i32 reinterpreted as u32 is 0xFFFFFFFF, the largest possible
        // u32 -- so `-1 <_s 1` is true (signed: -1 < 1) but `-1 <_u 1` is
        // false (unsigned: 0xFFFFFFFF is NOT < 1). If the u32 cast in the
        // interpreter were missing or wrong, these two would silently
        // agree instead of disagreeing.
        let lt_s_code = |sub_opcode: u8| {
            let mut code = vec![0xFD, 0x0C];
            code.extend(v128_const_bytes([-1, 0, 0, 0]));
            code.extend([0xFD, 0x0C]);
            code.extend(v128_const_bytes([1, 0, 0, 0]));
            code.extend([0xFD, sub_opcode]);
            code.extend([0xFD, 0x1B, 0x00]); // extract_lane 0
            code.push(0x0B);
            code
        };
        let mut lt_s_engine = simd_engine(lt_s_code(0x39)); // i32x4.lt_s
        assert_eq!(lt_s_engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(-1)], "-1 <_s 1 must be true");
        let mut lt_u_engine = simd_engine(lt_s_code(0x3A)); // i32x4.lt_u
        assert_eq!(lt_u_engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(0)], "0xFFFFFFFF <_u 1 must be false");
    }

    /// `i32x4.abs`: UNARY, same shape as `neg` -- including the wrapping
    /// edge case (`i32::MIN.wrapping_abs() == i32::MIN`, since `-i32::MIN`
    /// doesn't fit in an `i32`).
    #[test]
    fn i32x4_abs_computes_real_lane_wise_wrapping_absolute_value() {
        let mut code = vec![0xFD, 0x0C];
        code.extend(v128_const_bytes([5, -5, 0, i32::MIN]));
        code.extend([0xFD, 0xA0, 0x01]); // i32x4.abs (LEB128 for sub-opcode 0xA0 = 160)
        code.extend([0xFD, 0x1B, 0x03]); // extract_lane 3 -- the wrapping edge case
        code.push(0x0B);
        let mut engine = simd_engine(code);
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(i32::MIN)],
            "abs(i32::MIN) must wrap back to i32::MIN, matching scalar wrapping_abs"
        );
    }

    /// `i32x4.min_s`/`min_u`/`max_s`/`max_u`: the same signed-vs-unsigned
    /// distinguishing pattern as the comparison family above -- `-1`
    /// (0xFFFFFFFF unsigned) vs `1` must give opposite answers for the
    /// signed and unsigned variants of both min and max.
    #[test]
    fn i32x4_min_max_family_distinguishes_signed_from_unsigned() {
        let minmax_code = |sub_opcode: u8| {
            let mut code = vec![0xFD, 0x0C];
            code.extend(v128_const_bytes([-1, 0, 0, 0]));
            code.extend([0xFD, 0x0C]);
            code.extend(v128_const_bytes([1, 0, 0, 0]));
            code.extend([0xFD, sub_opcode, 0x01]); // LEB128: sub-opcodes 0xB6-0xB9 all need the continuation byte
            code.extend([0xFD, 0x1B, 0x00]); // extract_lane 0
            code.push(0x0B);
            code
        };
        let mut min_s_engine = simd_engine(minmax_code(0xB6)); // i32x4.min_s
        assert_eq!(min_s_engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(-1)], "min_s(-1, 1) must be -1 (signed: -1 < 1)");
        let mut min_u_engine = simd_engine(minmax_code(0xB7)); // i32x4.min_u
        assert_eq!(min_u_engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(1)], "min_u(0xFFFFFFFF, 1) must be 1 (unsigned: 1 < 0xFFFFFFFF)");
        let mut max_s_engine = simd_engine(minmax_code(0xB8)); // i32x4.max_s
        assert_eq!(max_s_engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(1)], "max_s(-1, 1) must be 1 (signed: 1 > -1)");
        let mut max_u_engine = simd_engine(minmax_code(0xB9)); // i32x4.max_u
        assert_eq!(max_u_engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(-1)], "max_u(0xFFFFFFFF, 1) must be -1's bit pattern (unsigned: 0xFFFFFFFF > 1)");
    }

    /// Multiple `v128.const`s inside ONE function body must each resolve
    /// to their OWN distinct literal, not accidentally alias -- a real
    /// regression risk given the const pool is a per-function side-table
    /// (`ctx.simd_consts`) indexed by decode-order position.
    #[test]
    fn multiple_v128_consts_in_one_function_stay_distinct() {
        let mut code = vec![0xFD, 0x0C];
        code.extend(v128_const_bytes([100, 200, 300, 400]));
        code.extend([0xFD, 0x1B, 0x02]); // first const, lane 2 -> 300
        code.push(0x0B);
        let mut engine = simd_engine(code);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(300)]);
    }

    /// Extracting a lane out of range must trap cleanly, not panic --
    /// this can only be reached with a hand-crafted/adversarial module,
    /// since `wasm-wast-parser`'s own literal syntax and `wasm-validator`
    /// (once it type-checks this op) would reject an out-of-range lane
    /// index at a different layer.
    #[test]
    fn extract_lane_out_of_range_index_is_a_clean_error_not_a_panic() {
        let mut code = vec![0xFD, 0x0C];
        code.extend(v128_const_bytes([1, 2, 3, 4]));
        code.extend([0xFD, 0x1B, 0x09]); // lane index 9 -- out of the valid 0-3 range
        code.push(0x0B);
        let mut engine = simd_engine(code);
        assert!(engine.call_function(0, &[]).is_err());
    }

    /// Security review (SIMD PR1a): `v128_heap` has no reclamation yet
    /// (see its own doc comment), so a WASM `loop` that creates a new
    /// v128 on every iteration -- via a backward `br`, needing NO
    /// recursion at all, so `MAX_CALL_DEPTH` never engages -- must still
    /// be bounded. This runs the exact adversarial shape
    /// `MAX_V128_HEAP_LEN` exists to stop: an infinite loop pushing a new
    /// `i32x4.splat` result every iteration. A clean `Err` (not an OOM
    /// process abort, and not a hang) proves the guard trips.
    #[test]
    fn an_unbounded_loop_creating_v128s_every_iteration_traps_cleanly_instead_of_exhausting_memory() {
        let code = vec![
            0x03, 0x40, // loop (blocktype: empty)
            0x41, 0x01, // i32.const 1
            0xFD, 0x11, // i32x4.splat -- pushes a NEW v128_heap entry every iteration
            0x1A, // drop
            0x0C, 0x00, // br 0 -- back to the top of the loop, unconditionally
            0x0B, // end (loop)
            0x0B, // end (function)
        ];
        let mut engine = simd_engine(code);
        let result = engine.call_function(0, &[]);
        assert!(result.is_err(), "an unbounded v128-creating loop must trap, not hang or exhaust memory");
        assert!(result.unwrap_err().to_string().contains("v128 heap limit exceeded"));
    }

    fn simd_engine_returning_v128(code: Vec<u8>) -> WasmExecutionEngine {
        let func_type = FuncType { params: vec![], results: vec![ValueType::V128] };
        let body = FunctionBody { locals: vec![], code };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    /// `call_function_with_v128` must resolve the REAL bytes of a `v128`
    /// RESULT — not just prove a handle came back, but that the handle
    /// resolves to the exact bytes the WASM code actually computed, both
    /// for a bare `v128.const` (a direct literal) and for `i32x4.add` (a
    /// genuinely computed value) — this is what unblocks real byte-exact
    /// `wasm-conformance` grading (see `code/specs/
    /// W13-wasm-simd-v128-first-slice.md`'s follow-up scope).
    #[test]
    fn call_function_with_v128_resolves_real_bytes_for_a_const_and_a_computed_value() {
        let mut const_code = vec![0xFD, 0x0C];
        const_code.extend(v128_const_bytes([11, 22, 33, 44]));
        const_code.push(0x0B);
        let mut engine = simd_engine_returning_v128(const_code);
        let (results, v128_bytes) = engine.call_function_with_v128(0, &[]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], WasmValue::V128(_)));
        assert_eq!(v128_bytes.len(), 1);
        assert_eq!(v128_bytes[0], Some(V128Bytes(v128_const_bytes([11, 22, 33, 44]).try_into().unwrap())));

        let mut add_code = vec![0xFD, 0x0C];
        add_code.extend(v128_const_bytes([1, 2, 3, 4]));
        add_code.extend([0xFD, 0x0C]);
        add_code.extend(v128_const_bytes([10, 20, 30, 40]));
        add_code.extend([0xFD, 0xAE, 0x01]); // i32x4.add
        add_code.push(0x0B);
        let mut engine2 = simd_engine_returning_v128(add_code);
        let (_, v128_bytes2) = engine2.call_function_with_v128(0, &[]).unwrap();
        assert_eq!(
            v128_bytes2[0],
            Some(V128Bytes(v128_const_bytes([11, 22, 33, 44]).try_into().unwrap())),
            "the resolved bytes must reflect the ACTUAL computation (1+10, 2+20, 3+30, 4+40), not just any v128"
        );
    }

    /// `call_function` (the pre-existing, unchanged entry point) must
    /// keep working exactly as before for a v128-returning function too
    /// -- it just discards the resolved bytes `call_function_with_v128`
    /// also provides, it doesn't behave differently or error out.
    #[test]
    fn call_function_still_works_unchanged_for_a_v128_returning_function() {
        let mut code = vec![0xFD, 0x0C];
        code.extend(v128_const_bytes([1, 2, 3, 4]));
        code.push(0x0B);
        let mut engine = simd_engine_returning_v128(code);
        let results = engine.call_function(0, &[]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], WasmValue::V128(_)));
    }

    // ══════════════════════════════════════════════════════════════════════
    // WASM18: atomic memory operations (0xFE prefix)
    // ══════════════════════════════════════════════════════════════════════

    fn atomic_engine(code: Vec<u8>, results: Vec<ValueType>) -> WasmExecutionEngine {
        let func_type = FuncType { params: vec![], results };
        let body = FunctionBody { locals: vec![], code };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![LinearMemory::new(1, None)],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    #[test]
    fn test_atomic_store_then_load_round_trips() {
        let code = vec![
            0x41, 0x00, // i32.const 0 (addr)
            0x41, 0x2A, // i32.const 42 (value)
            0xFE, 0x17, 0x02, 0x00, // i32.atomic.store align=4 offset=0
            0x41, 0x00, // i32.const 0 (addr)
            0xFE, 0x10, 0x02, 0x00, // i32.atomic.load align=4 offset=0
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_atomic_rmw_add_returns_the_old_value_and_updates_memory() {
        // All constants kept in -64..63 so their signed-LEB128 encoding is
        // exactly one byte equal to the raw value -- values >= 64 need a
        // 2-byte encoding (bit 6 set looks like a sign bit otherwise).
        let code = vec![
            0x41, 0x00, 0x41, 0x32, 0xFE, 0x17, 0x02, 0x00, // store 50 at addr 0
            0x41, 0x00, 0x41, 0x05, 0xFE, 0x1E, 0x02, 0x00, // rmw.add 5 -> pushes OLD (50)
            0x41, 0x00, 0xFE, 0x10, 0x02, 0x00, // load -> pushes NEW (55)
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32, ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(50), WasmValue::I32(55)]);
    }

    #[test]
    fn test_atomic_cmpxchg_success_replaces_and_returns_old_value() {
        let code = vec![
            0x41, 0x00, 0x41, 0x07, 0xFE, 0x17, 0x02, 0x00, // store 7 at addr 0
            0x41, 0x00, 0x41, 0x07, 0x41, 0x32, 0xFE, 0x48, 0x02, 0x00, // cmpxchg(expected=7, replacement=50) -> pushes OLD (7)
            0x41, 0x00, 0xFE, 0x10, 0x02, 0x00, // load -> pushes NEW (50, exchange happened)
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32, ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(7), WasmValue::I32(50)]);
    }

    #[test]
    fn test_atomic_cmpxchg_failure_leaves_memory_unchanged() {
        let code = vec![
            0x41, 0x00, 0x41, 0x07, 0xFE, 0x17, 0x02, 0x00, // store 7 at addr 0
            0x41, 0x00, 0x41, 0x32, 0x41, 0x2A, 0xFE, 0x48, 0x02, 0x00, // cmpxchg(expected=50, replacement=42) -- mismatch
            0x41, 0x00, 0xFE, 0x10, 0x02, 0x00, // load -> still 7, no exchange
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32, ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(7), WasmValue::I32(7)]);
    }

    #[test]
    fn test_atomic_fence_is_a_true_no_op() {
        let code = vec![0xFE, 0x03, 0x0B]; // atomic.fence; end
        let mut engine = atomic_engine(code, vec![]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), Vec::<WasmValue>::new());
    }

    #[test]
    fn test_atomic_narrow_i64_load_store_round_trip() {
        // i64.atomic.store8 (0x1B) then i64.atomic.load8_u (0x14):
        // narrow-width RMW/load/store dispatch (natural_align=1, not the
        // full 8-byte width) exercises a DIFFERENT arm of
        // atomic_mem_load/store than the other tests here.
        let code = vec![
            0x41, 0x00, 0x42, 0xFF, 0x01, // i32.const 0 (addr); i64.const 255
            0xFE, 0x1B, 0x00, 0x00, // i64.atomic.store8 align=1 offset=0
            0x41, 0x00, 0xFE, 0x14, 0x00, 0x00, // i64.atomic.load8_u align=1 offset=0
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I64]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I64(255)]);
    }

    #[test]
    fn test_atomic_op_on_out_of_bounds_address_is_a_clean_error_not_a_panic() {
        let code = vec![
            0x41, 0x7F, // i32.const -1 -- as u32, far past the 1-page (65536-byte) memory
            0xFE, 0x10, 0x02, 0x00, // i32.atomic.load
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_atomic_op_on_a_misaligned_effective_address_traps() {
        // `i32.atomic.load`'s natural alignment is 4 bytes. Address 1 is
        // in-bounds (so this isn't the OOB case above) but not a
        // multiple of 4 -- the real, pinned-commit `atomic.wast`
        // testsuite asserts this traps with message "unaligned atomic".
        // This is a RUNTIME check distinct from wasm-validator's static
        // check of the declared `align=` immediate: the effective
        // address here is `base + offset` where `base` is a runtime
        // value (from `i32.const 1`), not knowable at validation time.
        let code = vec![
            0x41, 0x01, // i32.const 1 (addr -- not a multiple of 4)
            0xFE, 0x10, 0x02, 0x00, // i32.atomic.load align=4 offset=0
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_atomic_op_on_a_naturally_aligned_effective_address_still_succeeds() {
        // Companion to the misalignment trap above: address 4 IS a
        // multiple of i32's natural 4-byte alignment, so this must
        // still succeed -- proves the alignment check doesn't
        // over-trigger on valid accesses.
        let code = vec![
            0x41, 0x04, 0x41, 0x2A, 0xFE, 0x17, 0x02, 0x00, // store 42 at addr 4
            0x41, 0x04, 0xFE, 0x10, 0x02, 0x00, // load from addr 4
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_atomic_notify_always_returns_zero_woken() {
        // With one native thread, no other agent is ever blocked in
        // `wait`, so the only real, deterministic answer is 0 -- not a
        // stand-in for unimplemented behavior.
        let code = vec![
            0x41, 0x00, // i32.const 0 (addr)
            0x41, 0x05, // i32.const 5 (count -- ignored either way)
            0xFE, 0x00, 0x02, 0x00, // memory.atomic.notify align=4 offset=0
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_atomic_wait32_returns_not_equal_when_current_differs_from_expected() {
        // Fresh memory is all-zero, so `expected = 1` never matches --
        // the deterministic "not-equal" outcome (1), reachable with zero
        // real threads.
        let code = vec![
            0x41, 0x00, // i32.const 0 (addr)
            0x41, 0x01, // i32.const 1 (expected, mismatched)
            0x42, 0x00, // i64.const 0 (timeout)
            0xFE, 0x01, 0x02, 0x00, // memory.atomic.wait32 align=4 offset=0
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(1)]);
    }

    #[test]
    fn test_atomic_wait64_returns_timed_out_when_current_equals_expected() {
        // Fresh memory is all-zero, so `expected = 0` DOES match --
        // nothing will ever notify this wait, so the only sound outcome
        // is "timed-out" (2).
        let code = vec![
            0x41, 0x00, // i32.const 0 (addr)
            0x42, 0x00, // i64.const 0 (expected, matches current mem)
            0x42, 0x00, // i64.const 0 (timeout)
            0xFE, 0x02, 0x03, 0x00, // memory.atomic.wait64 align=8 offset=0
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(2)]);
    }

    #[test]
    fn test_atomic_wait32_on_a_misaligned_effective_address_traps() {
        // `Wait` shares `effective_addr`/`atomic_mem_load` with every
        // other atomic kind but is handled in its own match arm, so the
        // alignment check needs its own call site -- this guards against
        // that call site being forgotten (it was, initially: caught in
        // security review, not by the vendored testsuite, since
        // atomic.wast's own "unaligned atomic" assert_trap cases don't
        // happen to cover wait32/wait64).
        let code = vec![
            0x41, 0x01, // i32.const 1 (addr -- not a multiple of 4)
            0x41, 0x00, // i32.const 0 (expected)
            0x42, 0x00, // i64.const 0 (timeout)
            0xFE, 0x01, 0x02, 0x00, // memory.atomic.wait32 align=4 offset=0
            0x0B,
        ];
        let mut engine = atomic_engine(code, vec![ValueType::I32]);
        assert!(engine.call_function(0, &[]).is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // Const expression evaluator
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_const_expr_i64() {
        // i64.const 42; end (42 in signed LEB128 = 0x2A)
        let expr = vec![0x42, 0x2A, 0x0B];
        let result = evaluate_const_expr(&expr, &[], &mut Vec::new()).unwrap();
        assert_eq!(result, WasmValue::I64(42));
    }

    #[test]
    fn test_const_expr_f32() {
        let val: f32 = 3.14;
        let bytes = val.to_le_bytes();
        let expr = vec![0x43, bytes[0], bytes[1], bytes[2], bytes[3], 0x0B];
        let result = evaluate_const_expr(&expr, &[], &mut Vec::new()).unwrap();
        assert_eq!(result, WasmValue::F32(3.14));
    }

    #[test]
    fn test_const_expr_f64() {
        let val: f64 = 2.718281828;
        let bytes = val.to_le_bytes();
        let mut expr = vec![0x44];
        expr.extend_from_slice(&bytes);
        expr.push(0x0B);
        let result = evaluate_const_expr(&expr, &[], &mut Vec::new()).unwrap();
        assert_eq!(result, WasmValue::F64(2.718281828));
    }

    #[test]
    fn test_const_expr_global_get_oob() {
        let expr = vec![0x23, 0x05, 0x0B]; // global.get 5
        assert!(evaluate_const_expr(&expr, &[], &mut Vec::new()).is_err());
    }

    #[test]
    fn test_const_expr_empty() {
        // Just end opcode
        let expr = vec![0x0B];
        assert!(evaluate_const_expr(&expr, &[], &mut Vec::new()).is_err()); // "empty constant expression"
    }

    #[test]
    fn test_const_expr_illegal_opcode() {
        let expr = vec![0x6A, 0x0B]; // i32.add is not allowed in const expr
        assert!(evaluate_const_expr(&expr, &[], &mut Vec::new()).is_err());
    }

    #[test]
    fn test_const_expr_missing_end() {
        let expr = vec![0x41, 0x2A]; // i32.const 42 without end
        assert!(evaluate_const_expr(&expr, &[], &mut Vec::new()).is_err());
    }

    #[test]
    fn test_const_expr_f32_truncated() {
        let expr = vec![0x43, 0x00, 0x00]; // f32.const but only 2 bytes
        assert!(evaluate_const_expr(&expr, &[], &mut Vec::new()).is_err());
    }

    #[test]
    fn test_const_expr_f64_truncated() {
        let expr = vec![0x44, 0x00, 0x00, 0x00]; // f64.const but only 3 bytes
        assert!(evaluate_const_expr(&expr, &[], &mut Vec::new()).is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // Decoder tests
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_i32_const() {
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x41, 0x2A, 0x0B], // i32.const 42; end
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].opcode, 0x41);
        match &decoded[0].operand {
            DecodedOperand::Int(v) => assert_eq!(*v, 42),
            _ => panic!("expected Int operand"),
        }
    }

    #[test]
    fn test_decode_i64_const() {
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x42, 0x2A, 0x0B], // i64.const 42; end
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded[0].opcode, 0x42);
        match &decoded[0].operand {
            DecodedOperand::Int(v) => assert_eq!(*v, 42),
            _ => panic!("expected Int operand"),
        }
    }

    #[test]
    fn test_decode_f32_const() {
        let val: f32 = 1.5;
        let bytes = val.to_le_bytes();
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x43, bytes[0], bytes[1], bytes[2], bytes[3], 0x0B],
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded[0].opcode, 0x43);
        match &decoded[0].operand {
            DecodedOperand::F32(v) => assert_eq!(*v, 1.5),
            _ => panic!("expected F32 operand"),
        }
    }

    #[test]
    fn test_decode_f64_const() {
        let val: f64 = 2.5;
        let bytes = val.to_le_bytes();
        let mut code = vec![0x44];
        code.extend_from_slice(&bytes);
        code.push(0x0B);
        let body = FunctionBody {
            locals: vec![],
            code,
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded[0].opcode, 0x44);
        match &decoded[0].operand {
            DecodedOperand::F64(v) => assert_eq!(*v, 2.5),
            _ => panic!("expected F64 operand"),
        }
    }

    #[test]
    fn test_decode_block_type() {
        // block with empty type (0x40), then end
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x02, 0x40, 0x0B, 0x0B], // block (empty); end; end
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded[0].opcode, 0x02);
        match &decoded[0].operand {
            DecodedOperand::Int(v) => assert_eq!(*v, 0x40),
            _ => panic!("expected Int operand for blocktype"),
        }
    }

    #[test]
    fn test_decode_block_type_truncated_body_does_not_panic() {
        // Security review regression: a function body truncated right
        // after a `block`/`loop`/`if` opcode (no blocktype byte at all)
        // must not panic via an unchecked `code[offset]` index -- this is
        // reachable without going through `wasm-validator::validate()`
        // first (`wasm-runtime::instantiate()`/`call()` don't call it
        // themselves), so a real embedder could hit this with a crafted
        // module. Defaults to the same "empty" blocktype (0x40) truncated
        // f32/f64 immediates already default to on short input.
        let body = FunctionBody { locals: vec![], code: vec![0x02] }; // block, then nothing
        let decoded = decode_function_body(&body); // must not panic
        assert_eq!(decoded[0].opcode, 0x02);
        match &decoded[0].operand {
            DecodedOperand::Int(v) => assert_eq!(*v, 0x40),
            _ => panic!("expected Int operand for blocktype"),
        }
    }

    #[test]
    fn test_decode_block_type_v128_funcref_externref_are_raw_bytes_not_signed_leb128() {
        // Real bug (task #81, found vendoring simd_const.wast): these 3
        // single-byte blocktypes fell through to the signed-LEB128
        // type-index branch instead of being carried as their raw byte
        // (like the 4 MVP scalars already were) -- 0x7B/0x70/0x6F decode
        // to -5/-16/-17 as signed LEB128, which is NOT what `block_arity`
        // (or wasm-validator's `decode_blocktype`) expects.
        for byte in [0x7Bu8, 0x70, 0x6F] {
            let body = FunctionBody { locals: vec![], code: vec![0x02, byte, 0x0B, 0x0B] };
            let decoded = decode_function_body(&body);
            match &decoded[0].operand {
                DecodedOperand::Int(v) => assert_eq!(*v, byte as i64, "blocktype byte {byte:#x}"),
                _ => panic!("expected Int operand for blocktype {byte:#x}"),
            }
        }
    }

    /// End-to-end proof `block_arity` (not just the decoder) is fixed: a
    /// `br` OUT OF a `(block (result v128) ...)` must actually carry its
    /// v128 value across the branch -- before the fix, `block_arity`
    /// silently returned `(0, 0)` for this blocktype (the `_ => (0, 0)`
    /// fallback), which would have dropped the branched value instead of
    /// keeping it on the stack.
    #[test]
    fn br_out_of_a_v128_result_block_carries_the_value() {
        let mut code = vec![0x02, 0x7B]; // block (result v128)
        code.extend([0xFD, 0x0C]);
        code.extend(v128_const_bytes([11, 22, 33, 44]));
        code.extend([0x0C, 0x00]); // br 0
        code.push(0x0B); // end (block)
        code.push(0x0B); // end (func)
        let mut engine = simd_engine_returning_v128(code);
        let (results, v128_bytes) = engine.call_function_with_v128(0, &[]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(v128_bytes[0], Some(V128Bytes(v128_const_bytes([11, 22, 33, 44]).try_into().unwrap())));
    }

    /// Security review regression (task #79): `self.v128_heap` must be
    /// written back from `ctx.v128_heap` UNCONDITIONALLY, not only on a
    /// successful call -- a call that pushes a new `v128.const` entry and
    /// then traps must not silently lose that heap growth. Before this
    /// fix, the write-back sat after the `outcome` trap-check (`?` on a
    /// `TrapError` returns early, skipping it entirely), the exact class
    /// of bug `wasm-runtime::call_engine`'s own doc comment already warns
    /// about for memory/tables. Verified via `into_state()` after a
    /// trapped call, since `WasmExecutionEngine` doesn't expose
    /// `v128_heap` any other way.
    #[test]
    fn v128_heap_growth_survives_a_call_that_traps() {
        let mut code = vec![0xFD, 0x0C]; // v128.const -- pushes handle 1
        code.extend(v128_const_bytes([1, 2, 3, 4]));
        code.push(0x1A); // drop (discard the v128 -- we only care about heap growth)
        code.push(0x00); // unreachable -- traps
        code.push(0x0B); // end (unreachable code, never executed)
        let mut engine = simd_engine(code);
        let result = engine.call_function(0, &[]);
        assert!(result.is_err(), "unreachable must trap");
        let state = engine.into_state();
        assert_eq!(
            state.v128_heap.len(),
            2,
            "the v128.const pushed before the trap must still be in the restored heap, not discarded"
        );
        assert_eq!(&state.v128_heap[1][..], v128_const_bytes([1, 2, 3, 4]).as_slice());
    }

    #[test]
    fn test_decode_local_get() {
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x03, 0x0B], // local.get 3; end
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded[0].opcode, 0x20);
        match &decoded[0].operand {
            DecodedOperand::Int(v) => assert_eq!(*v, 3),
            _ => panic!("expected Int operand"),
        }
    }

    #[test]
    fn test_decode_memory_load() {
        // i32.load align=2 offset=8
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x28, 0x02, 0x08, 0x0B],
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded[0].opcode, 0x28);
        match &decoded[0].operand {
            DecodedOperand::MemArg { _align, offset, memidx } => {
                assert_eq!(*_align, 2);
                assert_eq!(*offset, 8);
                assert_eq!(*memidx, 0);
            }
            _ => panic!("expected MemArg operand"),
        }
    }

    #[test]
    fn test_decode_nop_and_unreachable() {
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x01, 0x00, 0x0B], // nop; unreachable; end
        };
        let decoded = decode_function_body(&body);
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].opcode, 0x01); // nop
        assert_eq!(decoded[1].opcode, 0x00); // unreachable
    }

    // ══════════════════════════════════════════════════════════════════════
    // Control flow map
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_control_flow_map_block() {
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x02, 0x40, 0x01, 0x0B, 0x0B], // block; nop; end; end
        };
        let decoded = decode_function_body(&body);
        let map = build_control_flow_map(&decoded);
        // block at index 0, nop at 1, end at 2, end at 3
        // block at index 0 should map to end at index 2
        assert!(map.contains_key(&0));
        assert_eq!(map[&0].end_pc, 2);
        assert_eq!(map[&0].else_pc, None);
    }

    #[test]
    fn test_control_flow_map_if_else() {
        // if (empty); nop; else; nop; end; end
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x04, 0x40, // if (empty)
                0x01, // nop
                0x05, // else
                0x01, // nop
                0x0B, // end
                0x0B, // end (function)
            ],
        };
        let decoded = decode_function_body(&body);
        let map = build_control_flow_map(&decoded);
        // Verify the decoded instruction count and positions
        assert!(map.contains_key(&0));
        let target = &map[&0];
        assert!(target.else_pc.is_some());
        assert_eq!(target.end_pc, decoded.len() - 2); // end of if-else block
    }

    // ══════════════════════════════════════════════════════════════════════
    // i32 arithmetic via engine
    // ══════════════════════════════════════════════════════════════════════

    /// Helper: build a 2-arg i32 function using given opcode for the operation.
    fn make_i32_binop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    /// Helper: build a 1-arg i32 function using given opcode.
    fn make_i32_unop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    #[test]
    fn test_i32_sub() {
        let mut engine = make_i32_binop_engine(0x6B);
        let result = engine
            .call_function(0, &[WasmValue::I32(10), WasmValue::I32(3)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(7)]);
    }

    #[test]
    fn test_i32_mul() {
        let mut engine = make_i32_binop_engine(0x6C);
        let result = engine
            .call_function(0, &[WasmValue::I32(6), WasmValue::I32(7)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_i32_div_s() {
        let mut engine = make_i32_binop_engine(0x6D);
        let result = engine
            .call_function(0, &[WasmValue::I32(-10), WasmValue::I32(3)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(-3)]);
    }

    #[test]
    fn test_i32_div_s_by_zero() {
        let mut engine = make_i32_binop_engine(0x6D);
        assert!(engine
            .call_function(0, &[WasmValue::I32(1), WasmValue::I32(0)])
            .is_err());
    }

    #[test]
    fn test_i32_div_s_overflow() {
        let mut engine = make_i32_binop_engine(0x6D);
        assert!(engine
            .call_function(0, &[WasmValue::I32(i32::MIN), WasmValue::I32(-1)])
            .is_err());
    }

    #[test]
    fn test_i32_div_u() {
        let mut engine = make_i32_binop_engine(0x6E);
        // -1 as u32 = 0xFFFFFFFF, divided by 2 = 0x7FFFFFFF
        let result = engine
            .call_function(0, &[WasmValue::I32(-1), WasmValue::I32(2)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(0x7FFFFFFFi32)]);
    }

    #[test]
    fn test_i32_rem_s() {
        let mut engine = make_i32_binop_engine(0x6F);
        let result = engine
            .call_function(0, &[WasmValue::I32(-7), WasmValue::I32(2)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(-1)]);
    }

    #[test]
    fn test_i32_rem_s_overflow() {
        let mut engine = make_i32_binop_engine(0x6F);
        // i32::MIN % -1 should be 0 (not trap)
        let result = engine
            .call_function(0, &[WasmValue::I32(i32::MIN), WasmValue::I32(-1)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_i32_rem_u() {
        let mut engine = make_i32_binop_engine(0x70);
        let result = engine
            .call_function(0, &[WasmValue::I32(7), WasmValue::I32(3)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(1)]);
    }

    #[test]
    fn test_i32_and_or_xor() {
        let mut eng_and = make_i32_binop_engine(0x71);
        let mut eng_or = make_i32_binop_engine(0x72);
        let mut eng_xor = make_i32_binop_engine(0x73);

        assert_eq!(
            eng_and
                .call_function(0, &[WasmValue::I32(0xFF), WasmValue::I32(0x0F)])
                .unwrap(),
            vec![WasmValue::I32(0x0F)]
        );
        assert_eq!(
            eng_or
                .call_function(0, &[WasmValue::I32(0xF0), WasmValue::I32(0x0F)])
                .unwrap(),
            vec![WasmValue::I32(0xFF)]
        );
        assert_eq!(
            eng_xor
                .call_function(0, &[WasmValue::I32(0xFF), WasmValue::I32(0x0F)])
                .unwrap(),
            vec![WasmValue::I32(0xF0)]
        );
    }

    #[test]
    fn test_i32_shl_shr() {
        let mut eng_shl = make_i32_binop_engine(0x74);
        let mut eng_shr_s = make_i32_binop_engine(0x75);
        let mut eng_shr_u = make_i32_binop_engine(0x76);

        assert_eq!(
            eng_shl
                .call_function(0, &[WasmValue::I32(1), WasmValue::I32(4)])
                .unwrap(),
            vec![WasmValue::I32(16)]
        );
        assert_eq!(
            eng_shr_s
                .call_function(0, &[WasmValue::I32(-16), WasmValue::I32(2)])
                .unwrap(),
            vec![WasmValue::I32(-4)]
        );
        assert_eq!(
            eng_shr_u
                .call_function(0, &[WasmValue::I32(-1), WasmValue::I32(1)])
                .unwrap(),
            vec![WasmValue::I32(0x7FFFFFFF)]
        );
    }

    #[test]
    fn test_i32_rotl_rotr() {
        let mut eng_rotl = make_i32_binop_engine(0x77);
        let mut eng_rotr = make_i32_binop_engine(0x78);

        assert_eq!(
            eng_rotl
                .call_function(0, &[WasmValue::I32(1), WasmValue::I32(1)])
                .unwrap(),
            vec![WasmValue::I32(2)]
        );
        assert_eq!(
            eng_rotr
                .call_function(0, &[WasmValue::I32(1), WasmValue::I32(1)])
                .unwrap(),
            vec![WasmValue::I32(i32::MIN)]
        ); // 0x80000000
    }

    #[test]
    fn test_i32_clz_ctz_popcnt() {
        let mut eng_clz = make_i32_unop_engine(0x67);
        let mut eng_ctz = make_i32_unop_engine(0x68);
        let mut eng_popcnt = make_i32_unop_engine(0x69);

        assert_eq!(
            eng_clz.call_function(0, &[WasmValue::I32(1)]).unwrap(),
            vec![WasmValue::I32(31)]
        );
        assert_eq!(
            eng_ctz.call_function(0, &[WasmValue::I32(0x80)]).unwrap(),
            vec![WasmValue::I32(7)]
        );
        assert_eq!(
            eng_popcnt
                .call_function(0, &[WasmValue::I32(0xFF)])
                .unwrap(),
            vec![WasmValue::I32(8)]
        );
    }

    #[test]
    fn test_i32_eqz() {
        let mut engine = make_i32_unop_engine(0x45);
        assert_eq!(
            engine.call_function(0, &[WasmValue::I32(0)]).unwrap(),
            vec![WasmValue::I32(1)]
        );

        let mut engine2 = make_i32_unop_engine(0x45);
        assert_eq!(
            engine2.call_function(0, &[WasmValue::I32(42)]).unwrap(),
            vec![WasmValue::I32(0)]
        );
    }

    #[test]
    fn test_i32_comparisons() {
        // eq
        let mut eng = make_i32_binop_engine(0x46);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(5), WasmValue::I32(5)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );
        let mut eng = make_i32_binop_engine(0x46);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(5), WasmValue::I32(6)])
                .unwrap(),
            vec![WasmValue::I32(0)]
        );

        // ne
        let mut eng = make_i32_binop_engine(0x47);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(5), WasmValue::I32(6)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );

        // lt_s
        let mut eng = make_i32_binop_engine(0x48);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(-1), WasmValue::I32(0)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );

        // lt_u
        let mut eng = make_i32_binop_engine(0x49);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(-1), WasmValue::I32(0)])
                .unwrap(),
            vec![WasmValue::I32(0)]
        ); // -1 as u32 > 0

        // gt_s
        let mut eng = make_i32_binop_engine(0x4A);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(1), WasmValue::I32(-1)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );

        // ge_s
        let mut eng = make_i32_binop_engine(0x4E);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(5), WasmValue::I32(5)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );

        // le_u
        let mut eng = make_i32_binop_engine(0x4D);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(0), WasmValue::I32(-1)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        ); // 0 <= 0xFFFFFFFF
    }

    // ══════════════════════════════════════════════════════════════════════
    // i64 arithmetic via engine
    // ══════════════════════════════════════════════════════════════════════

    fn make_i64_binop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::I64, ValueType::I64],
            results: vec![ValueType::I64],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    fn make_i64_unop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::I64],
            results: vec![ValueType::I64],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    #[test]
    fn test_i64_add_sub_mul() {
        let mut eng = make_i64_binop_engine(0x7C);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(10), WasmValue::I64(20)])
                .unwrap(),
            vec![WasmValue::I64(30)]
        );

        let mut eng = make_i64_binop_engine(0x7D);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(20), WasmValue::I64(7)])
                .unwrap(),
            vec![WasmValue::I64(13)]
        );

        let mut eng = make_i64_binop_engine(0x7E);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(6), WasmValue::I64(7)])
                .unwrap(),
            vec![WasmValue::I64(42)]
        );
    }

    #[test]
    fn test_i64_div_s_by_zero() {
        let mut eng = make_i64_binop_engine(0x7F);
        assert!(eng
            .call_function(0, &[WasmValue::I64(1), WasmValue::I64(0)])
            .is_err());
    }

    #[test]
    fn test_i64_div_s_overflow() {
        let mut eng = make_i64_binop_engine(0x7F);
        assert!(eng
            .call_function(0, &[WasmValue::I64(i64::MIN), WasmValue::I64(-1)])
            .is_err());
    }

    #[test]
    fn test_i64_rem_s() {
        let mut eng = make_i64_binop_engine(0x81);
        let result = eng
            .call_function(0, &[WasmValue::I64(-7), WasmValue::I64(2)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I64(-1)]);
    }

    #[test]
    fn test_i64_rem_s_min_neg1() {
        let mut eng = make_i64_binop_engine(0x81);
        let result = eng
            .call_function(0, &[WasmValue::I64(i64::MIN), WasmValue::I64(-1)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I64(0)]);
    }

    #[test]
    fn test_i64_clz_ctz_popcnt() {
        let mut eng = make_i64_unop_engine(0x79);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(1)]).unwrap(),
            vec![WasmValue::I64(63)]
        );

        let mut eng = make_i64_unop_engine(0x7A);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(0x100)]).unwrap(),
            vec![WasmValue::I64(8)]
        );

        let mut eng = make_i64_unop_engine(0x7B);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(0xFF)]).unwrap(),
            vec![WasmValue::I64(8)]
        );
    }

    #[test]
    fn test_i64_eqz() {
        // i64.eqz returns i32
        let func_type = FuncType {
            params: vec![ValueType::I64],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x50, 0x0B], // local.get 0; i64.eqz; end
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[WasmValue::I64(0)]).unwrap(),
            vec![WasmValue::I32(1)]
        );
    }

    #[test]
    fn test_i64_comparisons() {
        // i64.eq returns i32
        let func_type = FuncType {
            params: vec![ValueType::I64, ValueType::I64],
            results: vec![ValueType::I32],
        };

        // eq (0x51)
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x51, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type.clone()],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine
                .call_function(0, &[WasmValue::I64(42), WasmValue::I64(42)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );

        // lt_s (0x53)
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x53, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine
                .call_function(0, &[WasmValue::I64(-1), WasmValue::I64(0)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // f32 arithmetic
    // ══════════════════════════════════════════════════════════════════════

    fn make_f32_binop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::F32, ValueType::F32],
            results: vec![ValueType::F32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    fn make_f32_unop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::F32],
            results: vec![ValueType::F32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    #[test]
    fn test_f32_add_sub_mul_div() {
        let mut eng = make_f32_binop_engine(0x92);
        let r = eng
            .call_function(0, &[WasmValue::F32(1.5), WasmValue::F32(2.5)])
            .unwrap();
        assert_eq!(r, vec![WasmValue::F32(4.0)]);

        let mut eng = make_f32_binop_engine(0x93);
        let r = eng
            .call_function(0, &[WasmValue::F32(5.0), WasmValue::F32(2.0)])
            .unwrap();
        assert_eq!(r, vec![WasmValue::F32(3.0)]);

        let mut eng = make_f32_binop_engine(0x94);
        let r = eng
            .call_function(0, &[WasmValue::F32(3.0), WasmValue::F32(4.0)])
            .unwrap();
        assert_eq!(r, vec![WasmValue::F32(12.0)]);

        let mut eng = make_f32_binop_engine(0x95);
        let r = eng
            .call_function(0, &[WasmValue::F32(10.0), WasmValue::F32(4.0)])
            .unwrap();
        assert_eq!(r, vec![WasmValue::F32(2.5)]);
    }

    #[test]
    fn test_f32_min_max() {
        let mut eng = make_f32_binop_engine(0x96);
        let r = eng
            .call_function(0, &[WasmValue::F32(3.0), WasmValue::F32(5.0)])
            .unwrap();
        assert_eq!(r, vec![WasmValue::F32(3.0)]);

        let mut eng = make_f32_binop_engine(0x97);
        let r = eng
            .call_function(0, &[WasmValue::F32(3.0), WasmValue::F32(5.0)])
            .unwrap();
        assert_eq!(r, vec![WasmValue::F32(5.0)]);
    }

    /// Found running the real WebAssembly/testsuite corpus (`f32.wast`) via
    /// `wasm-conformance`: WASM's `min`/`max` MUST propagate NaN
    /// unconditionally, unlike Rust's native `f32::min`/`max`, which return
    /// the OTHER (non-NaN) operand when one input is NaN -- `min(NaN, -0.0)`
    /// was silently returning `-0.0` instead of NaN.
    #[test]
    fn test_f32_min_max_propagates_nan() {
        let mut eng = make_f32_binop_engine(0x96);
        let r = eng.call_function(0, &[WasmValue::F32(f32::NAN), WasmValue::F32(1.0)]).unwrap();
        assert!(matches!(r[0], WasmValue::F32(v) if v.is_nan()), "min(NaN, 1.0) should be NaN, got {r:?}");
        let r = eng.call_function(0, &[WasmValue::F32(1.0), WasmValue::F32(f32::NAN)]).unwrap();
        assert!(matches!(r[0], WasmValue::F32(v) if v.is_nan()), "min(1.0, NaN) should be NaN, got {r:?}");

        let mut eng = make_f32_binop_engine(0x97);
        let r = eng.call_function(0, &[WasmValue::F32(f32::NAN), WasmValue::F32(1.0)]).unwrap();
        assert!(matches!(r[0], WasmValue::F32(v) if v.is_nan()), "max(NaN, 1.0) should be NaN, got {r:?}");
    }

    /// `min`/`max` must also treat `-0.0` as strictly less than `+0.0` --
    /// `min(+0.0, -0.0) == -0.0`, `max(+0.0, -0.0) == +0.0` -- per the WASM
    /// spec's own signed-zero tie-breaking rule.
    #[test]
    fn test_f32_min_max_signed_zero() {
        let mut eng = make_f32_binop_engine(0x96);
        let r = eng.call_function(0, &[WasmValue::F32(0.0), WasmValue::F32(-0.0)]).unwrap();
        assert!(matches!(r[0], WasmValue::F32(v) if v.is_sign_negative()), "min(+0,-0) should be -0.0, got {r:?}");

        let mut eng = make_f32_binop_engine(0x97);
        let r = eng.call_function(0, &[WasmValue::F32(0.0), WasmValue::F32(-0.0)]).unwrap();
        assert!(matches!(r[0], WasmValue::F32(v) if v.is_sign_positive()), "max(+0,-0) should be +0.0, got {r:?}");
    }

    #[test]
    fn test_f32_abs_neg_sqrt() {
        let mut eng = make_f32_unop_engine(0x8B);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(-5.0)]).unwrap(),
            vec![WasmValue::F32(5.0)]
        );

        let mut eng = make_f32_unop_engine(0x8C);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(5.0)]).unwrap(),
            vec![WasmValue::F32(-5.0)]
        );

        let mut eng = make_f32_unop_engine(0x91);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(9.0)]).unwrap(),
            vec![WasmValue::F32(3.0)]
        );
    }

    #[test]
    fn test_f32_ceil_floor_trunc() {
        let mut eng = make_f32_unop_engine(0x8D);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(1.3)]).unwrap(),
            vec![WasmValue::F32(2.0)]
        );

        let mut eng = make_f32_unop_engine(0x8E);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(1.7)]).unwrap(),
            vec![WasmValue::F32(1.0)]
        );

        let mut eng = make_f32_unop_engine(0x8F);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(-1.7)]).unwrap(),
            vec![WasmValue::F32(-1.0)]
        );
    }

    /// Found running the real corpus (`f64.wast`'s `nan:arithmetic`
    /// cases) via `wasm-conformance`: a SIGNALING NaN input (quiet bit
    /// clear) passed through `ceil`/`floor`/`trunc` must come out with the
    /// quiet bit SET -- the platform libm's own behavior for this was
    /// found to differ between macOS and Linux, so this can't rely on
    /// `f32::ceil`/`floor`/`trunc`'s native NaN handling.
    #[test]
    fn test_f32_ceil_floor_trunc_quiets_signaling_nan() {
        let signaling_nan = f32::from_bits(0x7FA0_0000); // exponent all-1, quiet bit clear, payload nonzero
        assert!(signaling_nan.is_nan());
        assert_eq!(signaling_nan.to_bits() & 0x0040_0000, 0, "test input must actually be signaling");

        for opcode in [0x8D, 0x8E, 0x8F] {
            let mut eng = make_f32_unop_engine(opcode);
            let r = eng.call_function(0, &[WasmValue::F32(signaling_nan)]).unwrap();
            let WasmValue::F32(v) = r[0] else { panic!("expected F32 result") };
            assert!(v.is_nan(), "opcode {opcode:#x}: expected a NaN result");
            assert_ne!(v.to_bits() & 0x0040_0000, 0, "opcode {opcode:#x}: result NaN must have the quiet bit set, got {:#010x}", v.to_bits());
        }
    }

    /// Found running the real WebAssembly/testsuite corpus (`f32.wast`)
    /// via `wasm-conformance`: `nearest` (round-ties-to-even, opcode
    /// `0x90`) must preserve the sign of a result that rounds to zero --
    /// `nearest(-0.25)` is `-0.0`, not `0.0` -- per IEEE 754's own
    /// roundTiesToEven rule. Rust's `f32::round()` doesn't guarantee that
    /// for magnitudes that round down to zero.
    #[test]
    fn test_f32_nearest() {
        let mut eng = make_f32_unop_engine(0x90);
        // Ordinary cases, unaffected by the sign-of-zero fix.
        assert_eq!(eng.call_function(0, &[WasmValue::F32(2.3)]).unwrap(), vec![WasmValue::F32(2.0)]);
        assert_eq!(eng.call_function(0, &[WasmValue::F32(2.5)]).unwrap(), vec![WasmValue::F32(2.0)], "ties to even");
        assert_eq!(eng.call_function(0, &[WasmValue::F32(3.5)]).unwrap(), vec![WasmValue::F32(4.0)], "ties to even");

        // The sign-of-zero case itself.
        let r = eng.call_function(0, &[WasmValue::F32(-0.25)]).unwrap();
        assert!(matches!(r[0], WasmValue::F32(v) if v == 0.0 && v.is_sign_negative()), "nearest(-0.25) should be -0.0, got {r:?}");
        let r = eng.call_function(0, &[WasmValue::F32(0.25)]).unwrap();
        assert!(matches!(r[0], WasmValue::F32(v) if v == 0.0 && v.is_sign_positive()), "nearest(0.25) should be +0.0, got {r:?}");
    }

    #[test]
    fn test_f32_comparisons() {
        let func_type = FuncType {
            params: vec![ValueType::F32, ValueType::F32],
            results: vec![ValueType::I32],
        };

        // f32.eq (0x5B)
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x5B, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type.clone()],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine
                .call_function(0, &[WasmValue::F32(1.0), WasmValue::F32(1.0)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );

        // f32.lt (0x5D)
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x5D, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine
                .call_function(0, &[WasmValue::F32(1.0), WasmValue::F32(2.0)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // f64 arithmetic
    // ══════════════════════════════════════════════════════════════════════

    fn make_f64_binop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::F64, ValueType::F64],
            results: vec![ValueType::F64],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    fn make_f64_unop_engine(opcode: u8) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![ValueType::F64],
            results: vec![ValueType::F64],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    #[test]
    fn test_f64_add_sub_mul_div() {
        let mut eng = make_f64_binop_engine(0xA0);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(1.5), WasmValue::F64(2.5)])
                .unwrap(),
            vec![WasmValue::F64(4.0)]
        );

        let mut eng = make_f64_binop_engine(0xA1);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(5.0), WasmValue::F64(2.0)])
                .unwrap(),
            vec![WasmValue::F64(3.0)]
        );

        let mut eng = make_f64_binop_engine(0xA2);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(3.0), WasmValue::F64(4.0)])
                .unwrap(),
            vec![WasmValue::F64(12.0)]
        );

        let mut eng = make_f64_binop_engine(0xA3);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(10.0), WasmValue::F64(4.0)])
                .unwrap(),
            vec![WasmValue::F64(2.5)]
        );
    }

    #[test]
    fn test_f64_min_max() {
        let mut eng = make_f64_binop_engine(0xA4);
        let r = eng.call_function(0, &[WasmValue::F64(3.0), WasmValue::F64(5.0)]).unwrap();
        assert_eq!(r, vec![WasmValue::F64(3.0)]);

        let mut eng = make_f64_binop_engine(0xA5);
        let r = eng.call_function(0, &[WasmValue::F64(3.0), WasmValue::F64(5.0)]).unwrap();
        assert_eq!(r, vec![WasmValue::F64(5.0)]);
    }

    /// As `test_f32_min_max_propagates_nan` -- same bug, same fix, f64.
    #[test]
    fn test_f64_min_max_propagates_nan() {
        let mut eng = make_f64_binop_engine(0xA4);
        let r = eng.call_function(0, &[WasmValue::F64(f64::NAN), WasmValue::F64(1.0)]).unwrap();
        assert!(matches!(r[0], WasmValue::F64(v) if v.is_nan()), "min(NaN, 1.0) should be NaN, got {r:?}");

        let mut eng = make_f64_binop_engine(0xA5);
        let r = eng.call_function(0, &[WasmValue::F64(1.0), WasmValue::F64(f64::NAN)]).unwrap();
        assert!(matches!(r[0], WasmValue::F64(v) if v.is_nan()), "max(1.0, NaN) should be NaN, got {r:?}");
    }

    /// As `test_f32_min_max_signed_zero` -- same rule, f64.
    #[test]
    fn test_f64_min_max_signed_zero() {
        let mut eng = make_f64_binop_engine(0xA4);
        let r = eng.call_function(0, &[WasmValue::F64(0.0), WasmValue::F64(-0.0)]).unwrap();
        assert!(matches!(r[0], WasmValue::F64(v) if v.is_sign_negative()), "min(+0,-0) should be -0.0, got {r:?}");

        let mut eng = make_f64_binop_engine(0xA5);
        let r = eng.call_function(0, &[WasmValue::F64(0.0), WasmValue::F64(-0.0)]).unwrap();
        assert!(matches!(r[0], WasmValue::F64(v) if v.is_sign_positive()), "max(+0,-0) should be +0.0, got {r:?}");
    }

    #[test]
    fn test_f64_abs_neg_sqrt_ceil_floor() {
        let mut eng = make_f64_unop_engine(0x99);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(-5.0)]).unwrap(),
            vec![WasmValue::F64(5.0)]
        );

        let mut eng = make_f64_unop_engine(0x9A);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(5.0)]).unwrap(),
            vec![WasmValue::F64(-5.0)]
        );

        let mut eng = make_f64_unop_engine(0x9F);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(16.0)]).unwrap(),
            vec![WasmValue::F64(4.0)]
        );

        let mut eng = make_f64_unop_engine(0x9B);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(1.3)]).unwrap(),
            vec![WasmValue::F64(2.0)]
        );

        let mut eng = make_f64_unop_engine(0x9C);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(1.7)]).unwrap(),
            vec![WasmValue::F64(1.0)]
        );
    }

    /// As `test_f32_ceil_floor_trunc_quiets_signaling_nan` -- same
    /// cross-platform bug, same fix, f64.
    #[test]
    fn test_f64_ceil_floor_trunc_quiets_signaling_nan() {
        let signaling_nan = f64::from_bits(0x7FF4_0000_0000_0000); // exponent all-1, quiet bit clear, payload nonzero
        assert!(signaling_nan.is_nan());
        assert_eq!(signaling_nan.to_bits() & 0x0008_0000_0000_0000, 0, "test input must actually be signaling");

        for opcode in [0x9B, 0x9C, 0x9D] {
            let mut eng = make_f64_unop_engine(opcode);
            let r = eng.call_function(0, &[WasmValue::F64(signaling_nan)]).unwrap();
            let WasmValue::F64(v) = r[0] else { panic!("expected F64 result") };
            assert!(v.is_nan(), "opcode {opcode:#x}: expected a NaN result");
            assert_ne!(v.to_bits() & 0x0008_0000_0000_0000, 0, "opcode {opcode:#x}: result NaN must have the quiet bit set, got {:#018x}", v.to_bits());
        }
    }

    /// As `test_f32_nearest` -- same sign-of-zero bug, same fix, f64
    /// (opcode `0x9E`).
    #[test]
    fn test_f64_nearest() {
        let mut eng = make_f64_unop_engine(0x9E);
        assert_eq!(eng.call_function(0, &[WasmValue::F64(2.3)]).unwrap(), vec![WasmValue::F64(2.0)]);
        assert_eq!(eng.call_function(0, &[WasmValue::F64(2.5)]).unwrap(), vec![WasmValue::F64(2.0)], "ties to even");
        assert_eq!(eng.call_function(0, &[WasmValue::F64(3.5)]).unwrap(), vec![WasmValue::F64(4.0)], "ties to even");

        let r = eng.call_function(0, &[WasmValue::F64(-0.25)]).unwrap();
        assert!(matches!(r[0], WasmValue::F64(v) if v == 0.0 && v.is_sign_negative()), "nearest(-0.25) should be -0.0, got {r:?}");
        let r = eng.call_function(0, &[WasmValue::F64(0.25)]).unwrap();
        assert!(matches!(r[0], WasmValue::F64(v) if v == 0.0 && v.is_sign_positive()), "nearest(0.25) should be +0.0, got {r:?}");
    }

    #[test]
    fn test_f64_comparisons() {
        let func_type = FuncType {
            params: vec![ValueType::F64, ValueType::F64],
            results: vec![ValueType::I32],
        };

        // f64.eq
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x61, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type.clone()],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine
                .call_function(0, &[WasmValue::F64(1.0), WasmValue::F64(1.0)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );

        // NaN != NaN
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x61, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine
                .call_function(0, &[WasmValue::F64(f64::NAN), WasmValue::F64(f64::NAN)])
                .unwrap(),
            vec![WasmValue::I32(0)]
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // Conversion instructions
    // ══════════════════════════════════════════════════════════════════════

    /// Helper to build a single-opcode conversion engine (one input type, one output type).
    fn make_conversion_engine(
        opcode: u8,
        param: ValueType,
        result: ValueType,
    ) -> WasmExecutionEngine {
        let func_type = FuncType {
            params: vec![param],
            results: vec![result],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, opcode, 0x0B],
        };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    #[test]
    fn test_i32_wrap_i64() {
        let mut eng = make_conversion_engine(0xA7, ValueType::I64, ValueType::I32);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(0x1_0000_0001)])
                .unwrap(),
            vec![WasmValue::I32(1)]
        );
    }

    #[test]
    fn test_i64_extend_i32_s() {
        let mut eng = make_conversion_engine(0xAC, ValueType::I32, ValueType::I64);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(-1)]).unwrap(),
            vec![WasmValue::I64(-1)]
        );
    }

    #[test]
    fn test_i64_extend_i32_u() {
        let mut eng = make_conversion_engine(0xAD, ValueType::I32, ValueType::I64);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(-1)]).unwrap(),
            vec![WasmValue::I64(0xFFFFFFFF)]
        );
    }

    #[test]
    fn test_i32_trunc_f32_s() {
        let mut eng = make_conversion_engine(0xA8, ValueType::F32, ValueType::I32);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(-2.9)]).unwrap(),
            vec![WasmValue::I32(-2)]
        );
    }

    #[test]
    fn test_i32_trunc_f32_s_nan_traps() {
        let mut eng = make_conversion_engine(0xA8, ValueType::F32, ValueType::I32);
        assert!(eng.call_function(0, &[WasmValue::F32(f32::NAN)]).is_err());
    }

    #[test]
    fn test_i32_trunc_f32_u() {
        let mut eng = make_conversion_engine(0xA9, ValueType::F32, ValueType::I32);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F32(3.7)]).unwrap(),
            vec![WasmValue::I32(3)]
        );
    }

    #[test]
    fn test_i32_trunc_f64_s() {
        let mut eng = make_conversion_engine(0xAA, ValueType::F64, ValueType::I32);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(-2.9)]).unwrap(),
            vec![WasmValue::I32(-2)]
        );
    }

    #[test]
    fn test_i32_trunc_f64_u() {
        let mut eng = make_conversion_engine(0xAB, ValueType::F64, ValueType::I32);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(3.7)]).unwrap(),
            vec![WasmValue::I32(3)]
        );
    }

    #[test]
    fn test_f32_convert_i32_s() {
        let mut eng = make_conversion_engine(0xB2, ValueType::I32, ValueType::F32);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(-5)]).unwrap(),
            vec![WasmValue::F32(-5.0)]
        );
    }

    #[test]
    fn test_f32_convert_i32_u() {
        let mut eng = make_conversion_engine(0xB3, ValueType::I32, ValueType::F32);
        // -1 as u32 = 4294967295
        let r = eng.call_function(0, &[WasmValue::I32(-1)]).unwrap();
        assert_eq!(r, vec![WasmValue::F32(4294967296.0)]); // f32 rounds
    }

    #[test]
    fn test_f64_convert_i32_s() {
        let mut eng = make_conversion_engine(0xB7, ValueType::I32, ValueType::F64);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I32(-5)]).unwrap(),
            vec![WasmValue::F64(-5.0)]
        );
    }

    #[test]
    fn test_f64_convert_i64_s() {
        let mut eng = make_conversion_engine(0xB9, ValueType::I64, ValueType::F64);
        assert_eq!(
            eng.call_function(0, &[WasmValue::I64(42)]).unwrap(),
            vec![WasmValue::F64(42.0)]
        );
    }

    #[test]
    fn test_f32_demote_f64() {
        let mut eng = make_conversion_engine(0xB6, ValueType::F64, ValueType::F32);
        let r = eng.call_function(0, &[WasmValue::F64(3.14)]).unwrap();
        // f32 loses precision
        assert!((r[0].as_f32().unwrap() - 3.14f32).abs() < 0.001);
    }

    #[test]
    fn test_f64_promote_f32() {
        let mut eng = make_conversion_engine(0xBB, ValueType::F32, ValueType::F64);
        let r = eng.call_function(0, &[WasmValue::F32(1.5)]).unwrap();
        assert_eq!(r, vec![WasmValue::F64(1.5)]);
    }

    // ── Reinterpret ──────────────────────────────────────────────────────

    #[test]
    fn test_i32_reinterpret_f32() {
        let mut eng = make_conversion_engine(0xBC, ValueType::F32, ValueType::I32);
        let r = eng.call_function(0, &[WasmValue::F32(1.0)]).unwrap();
        assert_eq!(r, vec![WasmValue::I32(1.0f32.to_bits() as i32)]);
    }

    #[test]
    fn test_i64_reinterpret_f64() {
        let mut eng = make_conversion_engine(0xBD, ValueType::F64, ValueType::I64);
        let r = eng.call_function(0, &[WasmValue::F64(1.0)]).unwrap();
        assert_eq!(r, vec![WasmValue::I64(1.0f64.to_bits() as i64)]);
    }

    #[test]
    fn test_f32_reinterpret_i32() {
        let mut eng = make_conversion_engine(0xBE, ValueType::I32, ValueType::F32);
        let bits = 1.0f32.to_bits() as i32;
        let r = eng.call_function(0, &[WasmValue::I32(bits)]).unwrap();
        assert_eq!(r, vec![WasmValue::F32(1.0)]);
    }

    #[test]
    fn test_f64_reinterpret_i64() {
        let mut eng = make_conversion_engine(0xBF, ValueType::I64, ValueType::F64);
        let bits = 1.0f64.to_bits() as i64;
        let r = eng.call_function(0, &[WasmValue::I64(bits)]).unwrap();
        assert_eq!(r, vec![WasmValue::F64(1.0)]);
    }

    // ── Sign-extension instructions (WASM03) ─────────────────────────────

    #[test]
    fn test_i32_extend8_s() {
        let mut eng = make_conversion_engine(0xC0, ValueType::I32, ValueType::I32);
        // 0xFF as an i32's low byte: byte 0xFF sign-extends to -1.
        assert_eq!(eng.call_function(0, &[WasmValue::I32(0xFF)]).unwrap(), vec![WasmValue::I32(-1)]);
        // The high bytes must be ignored entirely, not just the low byte's sign.
        assert_eq!(eng.call_function(0, &[WasmValue::I32(0x7F00_007F)]).unwrap(), vec![WasmValue::I32(127)]);
    }

    #[test]
    fn test_i32_extend16_s() {
        let mut eng = make_conversion_engine(0xC1, ValueType::I32, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::I32(0xFFFF)]).unwrap(), vec![WasmValue::I32(-1)]);
        assert_eq!(eng.call_function(0, &[WasmValue::I32(0x7FFF)]).unwrap(), vec![WasmValue::I32(32767)]);
    }

    #[test]
    fn test_i64_extend8_s() {
        let mut eng = make_conversion_engine(0xC2, ValueType::I64, ValueType::I64);
        assert_eq!(eng.call_function(0, &[WasmValue::I64(0xFF)]).unwrap(), vec![WasmValue::I64(-1)]);
    }

    #[test]
    fn test_i64_extend16_s() {
        let mut eng = make_conversion_engine(0xC3, ValueType::I64, ValueType::I64);
        assert_eq!(eng.call_function(0, &[WasmValue::I64(0xFFFF)]).unwrap(), vec![WasmValue::I64(-1)]);
    }

    #[test]
    fn test_i64_extend32_s() {
        let mut eng = make_conversion_engine(0xC4, ValueType::I64, ValueType::I64);
        assert_eq!(eng.call_function(0, &[WasmValue::I64(0xFFFF_FFFF)]).unwrap(), vec![WasmValue::I64(-1)]);
        assert_eq!(eng.call_function(0, &[WasmValue::I64(0x7FFF_FFFF)]).unwrap(), vec![WasmValue::I64(2147483647)]);
    }

    // ── Saturating truncation instructions (0xFC 0x00-0x07, WASM03) ──────

    /// Helper mirroring [`make_conversion_engine`], for the two-byte `0xFC
    /// <sub>` prefix encoding `trunc_sat` uses.
    fn make_trunc_sat_engine(sub: u8, param: ValueType, result: ValueType) -> WasmExecutionEngine {
        let func_type = FuncType { params: vec![param], results: vec![result] };
        let body = FunctionBody { locals: vec![], code: vec![0x20, 0x00, 0xFC, sub, 0x0B] };
        WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        })
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_ordinary_value() {
        let mut eng = make_trunc_sat_engine(0x00, ValueType::F32, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::F32(-2.9)]).unwrap(), vec![WasmValue::I32(-2)]);
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_nan_saturates_to_zero_not_trap() {
        // The whole point of trunc_sat vs. trunc: NaN must NOT trap.
        let mut eng = make_trunc_sat_engine(0x00, ValueType::F32, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::F32(f32::NAN)]).unwrap(), vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_overflow_saturates_to_i32_max() {
        let mut eng = make_trunc_sat_engine(0x00, ValueType::F32, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::F32(1e10)]).unwrap(), vec![WasmValue::I32(i32::MAX)]);
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_underflow_saturates_to_i32_min() {
        let mut eng = make_trunc_sat_engine(0x00, ValueType::F32, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::F32(-1e10)]).unwrap(), vec![WasmValue::I32(i32::MIN)]);
    }

    #[test]
    fn test_i32_trunc_sat_f32_u_negative_saturates_to_zero() {
        let mut eng = make_trunc_sat_engine(0x01, ValueType::F32, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::F32(-5.0)]).unwrap(), vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_i32_trunc_sat_f64_u_overflow_saturates_to_u32_max() {
        let mut eng = make_trunc_sat_engine(0x03, ValueType::F64, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::F64(1e20)]).unwrap(), vec![WasmValue::I32(-1)]); // 0xFFFFFFFF
    }

    #[test]
    fn test_i64_trunc_sat_f64_s_nan_saturates_to_zero() {
        let mut eng = make_trunc_sat_engine(0x06, ValueType::F64, ValueType::I64);
        assert_eq!(eng.call_function(0, &[WasmValue::F64(f64::NAN)]).unwrap(), vec![WasmValue::I64(0)]);
    }

    #[test]
    fn test_i64_trunc_sat_f64_u_overflow_saturates_to_u64_max() {
        let mut eng = make_trunc_sat_engine(0x07, ValueType::F64, ValueType::I64);
        assert_eq!(eng.call_function(0, &[WasmValue::F64(1e30)]).unwrap(), vec![WasmValue::I64(-1)]); // 0xFFFFFFFFFFFFFFFF
    }

    // ── Regression: TRAPPING trunc must still trap on overflow, unlike
    //    trunc_sat (WASM03 -- caught by conversions.wast newly parsing) ──

    #[test]
    fn test_i32_trunc_f32_u_accepts_tiny_negative_near_zero() {
        // A tiny negative value (trunc toward zero -> 0) is IN RANGE for
        // trunc_u -- the old `0.0..` lower bound wrongly rejected any
        // negative input, even ones truncating to a valid 0.
        let mut eng = make_conversion_engine(0xA9, ValueType::F32, ValueType::I32);
        assert_eq!(eng.call_function(0, &[WasmValue::F32(-1e-10)]).unwrap(), vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_i32_trunc_f64_s_traps_exactly_at_the_lower_boundary() {
        // -2147483649.0 is exactly representable in f64 and exactly ONE
        // past the valid lower bound (-2^31 - 1) -- must trap, not wrap.
        let mut eng = make_conversion_engine(0xAA, ValueType::F64, ValueType::I32);
        assert!(eng.call_function(0, &[WasmValue::F64(-2147483649.0)]).is_err());
    }

    #[test]
    fn test_i64_trunc_f32_s_traps_on_overflow() {
        // Before this fix, 0xAE/0xAF/0xB0 had NO overflow check at all
        // (only NaN), so `a as i64` silently SATURATED instead of trapping
        // -- unreachable until conversions.wast could parse.
        let mut eng = make_conversion_engine(0xAE, ValueType::F32, ValueType::I64);
        assert!(eng.call_function(0, &[WasmValue::F32(1e20)]).is_err());
    }

    #[test]
    fn test_i64_trunc_f32_u_traps_on_overflow() {
        let mut eng = make_conversion_engine(0xAF, ValueType::F32, ValueType::I64);
        assert!(eng.call_function(0, &[WasmValue::F32(1e20)]).is_err());
    }

    #[test]
    fn test_i64_trunc_f64_s_traps_on_overflow() {
        let mut eng = make_conversion_engine(0xB0, ValueType::F64, ValueType::I64);
        assert!(eng.call_function(0, &[WasmValue::F64(1e20)]).is_err());
    }

    #[test]
    fn test_i64_trunc_f64_u_traps_on_overflow() {
        let mut eng = make_conversion_engine(0xB1, ValueType::F64, ValueType::I64);
        assert!(eng.call_function(0, &[WasmValue::F64(1e20)]).is_err());
    }

    #[test]
    fn test_i64_trunc_f64_s_accepts_exact_i64_min() {
        // i64::MIN itself (-2^63) is exactly representable in f64 and IS
        // valid (the boundary is exclusive one past it, not at it).
        let mut eng = make_conversion_engine(0xB0, ValueType::F64, ValueType::I64);
        assert_eq!(
            eng.call_function(0, &[WasmValue::F64(-9223372036854775808.0)]).unwrap(),
            vec![WasmValue::I64(i64::MIN)]
        );
    }

    #[test]
    fn test_f32_reinterpret_i32_preserves_the_exact_nan_bit_pattern_end_to_end() {
        // The real WASM13 regression, exercised through the actual
        // interpreter (not just `to_typed`/`from_typed` directly): a pure
        // bit-reinterpretation instruction must return EXACTLY the bits it
        // was given, never a canonicalized NaN -- these are the literal
        // values `conversions.wast`'s `f32.reinterpret_i32` cases assert
        // against. Before the fix, this returned `f32::from_bits`'s input
        // correctly at the OPCODE level, but the surrounding push/pop
        // through the typed operand stack silently canonicalized the NaN
        // payload to `0x7fc00000` on its way back out.
        let mut eng = make_conversion_engine(0xBE, ValueType::I32, ValueType::F32);
        let cases: [i32; 2] = [0x7fa00000u32 as i32, 0xffa00000u32 as i32];
        for bits in cases {
            let result = eng.call_function(0, &[WasmValue::I32(bits)]).unwrap();
            match result[0] {
                WasmValue::F32(v) => assert_eq!(
                    v.to_bits(),
                    bits as u32,
                    "expected f32.reinterpret_i32({bits:#010x}) to preserve the exact bit pattern, got {:#010x}",
                    v.to_bits()
                ),
                other => panic!("expected F32, got {other:?}"),
            }
        }
    }

    #[test]
    fn test_i32_reinterpret_f32_preserves_the_exact_nan_bit_pattern_end_to_end() {
        // The reverse direction of the same real regression.
        let mut eng = make_conversion_engine(0xBC, ValueType::F32, ValueType::I32);
        let cases: [u32; 2] = [0x7fa00000, 0xffa00000];
        for bits in cases {
            let result = eng.call_function(0, &[WasmValue::F32(f32::from_bits(bits))]).unwrap();
            assert_eq!(result[0], WasmValue::I32(bits as i32), "expected i32.reinterpret_f32({bits:#010x}) to preserve the exact bit pattern");
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // Control flow: block, loop, if/else, br, br_if, return
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_block_with_result() {
        // A block that pushes a value and falls through to end.
        // block (result i32); i32.const 42; end; end
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x02, 0x7F, // block (result i32)
                0x41, 0x2A, // i32.const 42
                0x0B, // end (block)
                0x0B, // end (function)
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(42)]
        );
    }

    #[test]
    fn test_if_true_branch() {
        // if true; i32.const 1; else; i32.const 2; end
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0
                0x04, 0x7F, // if (result i32)
                0x41, 0x01, // i32.const 1
                0x05, // else
                0x41, 0x02, // i32.const 2
                0x0B, // end (if)
                0x0B, // end (function)
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[WasmValue::I32(1)]).unwrap(),
            vec![WasmValue::I32(1)]
        );
    }

    #[test]
    fn test_if_false_branch() {
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0
                0x04, 0x7F, // if (result i32)
                0x41, 0x01, // i32.const 1
                0x05, // else
                0x41, 0x02, // i32.const 2
                0x0B, // end (if)
                0x0B, // end (function)
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[WasmValue::I32(0)]).unwrap(),
            vec![WasmValue::I32(2)]
        );
    }

    #[test]
    fn test_br_if_taken() {
        // block(result i32); i32.const 42; i32.const 1; br_if 0; drop; i32.const 0; end
        // Note: 42 = 0x2A in signed LEB128 (bit 6 clear, no sign extension)
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x02, 0x7F, // block (result i32)
                0x41, 0x2A, // i32.const 42
                0x41, 0x01, // i32.const 1
                0x0D, 0x00, // br_if 0
                0x1A, // drop
                0x41, 0x00, // i32.const 0
                0x0B, // end (block)
                0x0B, // end (function)
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(42)]
        );
    }

    #[test]
    fn test_return_instruction() {
        // i32.const 42; return; i32.const 99; end
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x2A, // i32.const 42
                0x0F, // return
                0x41, 0x63, // i32.const 99 (unreachable)
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(42)]
        );
    }

    #[test]
    fn test_unreachable_traps() {
        let func_type = FuncType {
            params: vec![],
            results: vec![],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x00, 0x0B], // unreachable; end
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_nop() {
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x01, 0x01, 0x41, 0x05, 0x0B], // nop; nop; i32.const 5; end
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(5)]
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // Variable instructions: local.set, local.tee, global.get/set
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_local_set_and_get() {
        // local.get 0; local.set 1; local.get 1; end
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![ValueType::I32], // one extra local
            code: vec![
                0x20, 0x00, // local.get 0
                0x21, 0x01, // local.set 1
                0x20, 0x01, // local.get 1
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[WasmValue::I32(42)]).unwrap(),
            vec![WasmValue::I32(42)]
        );
    }

    #[test]
    fn test_local_tee() {
        // i32.const 10; local.tee 0; end
        // local.tee sets the local AND leaves the value on the stack
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x0A, // i32.const 10
                0x22, 0x00, // local.tee 0
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[WasmValue::I32(0)]).unwrap(),
            vec![WasmValue::I32(10)]
        );
    }

    #[test]
    fn test_global_get_set() {
        // global.get 0; i32.const 1; i32.add; global.set 0; global.get 0; end
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x23, 0x00, // global.get 0
                0x41, 0x01, // i32.const 1
                0x6A, // i32.add
                0x24, 0x00, // global.set 0
                0x23, 0x00, // global.get 0
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![WasmValue::I32(10)],
            global_types: vec![GlobalType {
                value_type: ValueType::I32,
                mutable: true,
            }],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(11)]
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // Parametric instructions: drop, select
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_drop_instruction() {
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x01, // i32.const 1
                0x41, 0x02, // i32.const 2
                0x1A, // drop (removes 2)
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(1)]
        );
    }

    #[test]
    fn test_select_true() {
        // select(val1, val2, cond): cond != 0 -> val1
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x0A, // i32.const 10 (val1)
                0x41, 0x14, // i32.const 20 (val2)
                0x41, 0x01, // i32.const 1  (cond = true)
                0x1B, // select
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(10)]
        );
    }

    #[test]
    fn test_select_false() {
        let func_type = FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x0A, // i32.const 10 (val1)
                0x41, 0x14, // i32.const 20 (val2)
                0x41, 0x00, // i32.const 0  (cond = false)
                0x1B, // select
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(
            engine.call_function(0, &[]).unwrap(),
            vec![WasmValue::I32(20)]
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // Engine error paths
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_engine_wrong_arg_count() {
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        // No args when 1 expected
        assert!(engine.call_function(0, &[]).is_err());
    }

    #[test]
    fn test_engine_undefined_function() {
        let engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![],
            func_bodies: vec![],
            host_functions: vec![],
        });
        // Can't call: engine is not mutable and func_index is out of bounds
        // We need a mutable reference; let's just test the config setup
        let mut engine = engine;
        assert!(engine.call_function(0, &[]).is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // decode_signed_64
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_decode_signed_64_basic() {
        // 42 = 0x2A in LEB128
        let data = vec![0x2A];
        let (val, consumed) = decode_signed_64(&data, 0).unwrap();
        assert_eq!(val, 42);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_decode_signed_64_negative() {
        // -1 in signed LEB128 = 0x7F
        let data = vec![0x7F];
        let (val, consumed) = decode_signed_64(&data, 0).unwrap();
        assert_eq!(val, -1);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_decode_signed_64_truncated() {
        // Unterminated LEB128 (high bit set, no more bytes)
        let data = vec![0x80];
        assert!(decode_signed_64(&data, 0).is_err());
    }

    #[test]
    fn memory_copy_decodes_as_one_0xfc_instruction() {
        // `memory.copy` = 0xFC 0x0A 0x00 0x00 (sub-opcode + dst/src memory indices).
        // The decoder must fold all four bytes into ONE instruction carrying the
        // sub-opcode, not mis-read the trailing 0x00s as separate `unreachable`s.
        let body = FunctionBody { locals: vec![], code: vec![0xFC, 0x0A, 0x00, 0x00, 0x0B] };
        let instrs = decode_function_body(&body);
        assert_eq!(instrs.len(), 2, "memory.copy (4 bytes) + end → 2 instrs: {instrs:?}");
        assert_eq!(instrs[0].opcode, 0xFC);
        assert!(
            matches!(instrs[0].operand, DecodedOperand::BulkMemory { sub: 0x0A, data_idx: 0, .. }),
            "sub-opcode 0x0A must be carried in the operand: {:?}",
            instrs[0].operand
        );
    }

    #[test]
    fn linear_memory_copy_moves_bytes_overlap_safe() {
        let mut mem = LinearMemory::new(1, None);
        mem.write_bytes(0, b"HELLO").unwrap();
        // Non-overlapping forward copy: "HELLO" at 0 → 8.
        mem.copy(8, 0, 5).unwrap();
        assert_eq!(&mem.data[8..13], b"HELLO");
        // Overlapping copy (dest > src, ranges overlap) must use memmove semantics.
        mem.write_bytes(0, b"ABCDEF").unwrap();
        mem.copy(2, 0, 4).unwrap(); // ABCDEF → AB ABCD
        assert_eq!(&mem.data[0..6], b"ABABCD");
        // Zero-length copy at exactly the end of memory (one-past-the-end,
        // the same convention as a Rust slice's exclusive upper bound) is a
        // no-op, matching the real spec.
        assert!(mem.copy(mem.data.len(), mem.data.len(), 0).is_ok());
        // Task #94 (found vendoring memory_copy.wast): a zero-length copy
        // whose dest/src is PAST the end of memory must still trap -- it is
        // NOT exempt from bounds-checking just because nothing is copied.
        assert!(mem.copy(999_999, 999_999, 0).is_err());
        // Out-of-range non-zero copy traps rather than panicking.
        assert!(mem.copy(0, 0, 999_999).is_err());
        // A sign-extended-negative operand (e.g. `-1i32 as u32 as usize`) must TRAP
        // via checked arithmetic, not wrap `offset + width` past the bounds check.
        assert!(mem.copy(u32::MAX as usize, 0, 1).is_err());
        assert!(mem.copy(0, u32::MAX as usize, 1).is_err());
        assert!(mem.copy(0, 0, u32::MAX as usize).is_err());
    }

    #[test]
    fn memory_fill_decodes_as_one_0xfc_instruction() {
        // `memory.fill` = 0xFC 0x0B 0x00 (sub-opcode + memory index), task #94.
        let body = FunctionBody { locals: vec![], code: vec![0xFC, 0x0B, 0x00, 0x0B] };
        let instrs = decode_function_body(&body);
        assert_eq!(instrs.len(), 2, "memory.fill (3 bytes) + end → 2 instrs: {instrs:?}");
        assert_eq!(instrs[0].opcode, 0xFC);
        assert!(
            matches!(instrs[0].operand, DecodedOperand::BulkMemory { sub: 0x0B, data_idx: 0, .. }),
            "sub-opcode 0x0B must be carried in the operand: {:?}",
            instrs[0].operand
        );
    }

    #[test]
    fn linear_memory_fill_writes_the_byte_and_bounds_checks() {
        let mut mem = LinearMemory::new(1, None);
        mem.fill(0, 0xAA, 5).unwrap();
        assert_eq!(&mem.data[0..5], &[0xAA; 5]);
        assert_eq!(mem.data[5], 0); // untouched beyond the filled range
        // Zero-length fill at exactly the end of memory is a no-op.
        assert!(mem.fill(mem.data.len(), 0, 0).is_ok());
        // Task #94 (same fix as `copy`, above): a zero-length fill whose
        // dest is PAST the end of memory must still trap.
        assert!(mem.fill(999_999, 0, 0).is_err());
        // Out-of-range non-zero fill traps rather than panicking.
        assert!(mem.fill(0, 0, 999_999).is_err());
        // A sign-extended-negative operand must TRAP via checked arithmetic,
        // not wrap `offset + width` past the bounds check.
        assert!(mem.fill(u32::MAX as usize, 0, 1).is_err());
        assert!(mem.fill(0, 0, u32::MAX as usize).is_err());
    }

    #[test]
    fn memory_init_decodes_and_carries_a_real_data_idx() {
        // `memory.init` = 0xFC 0x08 <data_idx:u32leb> <mem_idx:u8>, task
        // #95. data_idx=2 here (not 0) specifically to prove the decoder
        // reads a REAL immediate, not a hardcoded placeholder like memory.
        // copy/fill's discarded memory-index bytes.
        let body = FunctionBody { locals: vec![], code: vec![0xFC, 0x08, 0x02, 0x00, 0x0B] };
        let instrs = decode_function_body(&body);
        assert_eq!(instrs.len(), 2, "memory.init (4 bytes) + end -> 2 instrs: {instrs:?}");
        assert_eq!(instrs[0].opcode, 0xFC);
        assert!(
            matches!(instrs[0].operand, DecodedOperand::BulkMemory { sub: 0x08, data_idx: 2, .. }),
            "sub-opcode 0x08 and data_idx=2 must be carried in the operand: {:?}",
            instrs[0].operand
        );
    }

    #[test]
    fn data_drop_decodes_and_carries_a_real_data_idx() {
        // `data.drop` = 0xFC 0x09 <data_idx:u32leb>, task #95 -- no
        // trailing memory-index byte at all (unlike memory.init), since
        // data.drop has no memory concept.
        let body = FunctionBody { locals: vec![], code: vec![0xFC, 0x09, 0x01, 0x0B] };
        let instrs = decode_function_body(&body);
        assert_eq!(instrs.len(), 2, "data.drop (3 bytes) + end -> 2 instrs: {instrs:?}");
        assert_eq!(instrs[0].opcode, 0xFC);
        assert!(
            matches!(instrs[0].operand, DecodedOperand::BulkMemory { sub: 0x09, data_idx: 1, .. }),
            "sub-opcode 0x09 and data_idx=1 must be carried in the operand: {:?}",
            instrs[0].operand
        );
    }

    #[test]
    fn table_grow_size_fill_decode_and_carry_a_real_table_idx() {
        // `table.grow`/`table.size`/`table.fill` = `0xFC 0x0F/0x10/0x11
        // <table_idx:u32leb>` (task #98). table_idx=1 here (not 0) to
        // prove the decoder reads a REAL immediate, matching memory.
        // init's own `data_idx` decode -- reusing the same `data_idx`
        // operand slot for a different index space (see `BulkMemory`'s
        // own doc comment).
        for (sub, opcode_name) in [(0x0Fu8, "table.grow"), (0x10, "table.size"), (0x11, "table.fill")] {
            let body = FunctionBody { locals: vec![], code: vec![0xFC, sub, 0x01, 0x0B] };
            let instrs = decode_function_body(&body);
            assert_eq!(instrs.len(), 2, "{opcode_name} (3 bytes) + end -> 2 instrs: {instrs:?}");
            assert_eq!(instrs[0].opcode, 0xFC);
            assert!(
                matches!(instrs[0].operand, DecodedOperand::BulkMemory { sub: s, data_idx: 1, .. } if s == sub),
                "{opcode_name}: sub-opcode {sub:#x} and table_idx=1 must be carried in the operand: {:?}",
                instrs[0].operand
            );
        }
    }

    #[test]
    fn table_size_pushes_the_targeted_tables_own_size() {
        // Two tables; table.size on index 1 must read table 1's size, not
        // table 0's (proving the real decoded table_idx is honored, not
        // hardcoded to 0 the way `get_memory`'s single-memory helper is).
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody { locals: vec![], code: vec![0xFC, 0x10, 0x01, 0x0B] }; // table.size 1
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(3, None), Table::new(7, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(7)]);
    }

    #[test]
    fn table_grow_pushes_old_size_and_actually_grows_the_table() {
        // (ref.null func); i32.const 2; table.grow 0; (ref.null func);
        // table.get at the newly grown slot to prove it's really there.
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0xD0, 0x70, // ref.null func
                0x41, 0x02, // i32.const 2 (delta)
                0xFC, 0x0F, 0x00, // table.grow 0
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(1, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(1)]); // old size was 1
    }

    #[test]
    fn table_grow_failure_returns_negative_one_not_a_trap() {
        // Table starts at size 1 with max 1; growing by 1 must fail --
        // spec-mandated: a growth failure is a normal i32 return value,
        // never a trap.
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0xD0, 0x70, // ref.null func
                0x41, 0x01, // i32.const 1 (delta)
                0xFC, 0x0F, 0x00, // table.grow 0
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(1, Some(1))],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(-1)]);
    }

    /// Security review (task #98, round 2): a per-table `MAX_TABLE_
    /// ELEMENTS` cap alone still permits an AGGREGATE resource-exhaustion
    /// DoS across many tables. Table 0 is ALREADY at the per-table cap;
    /// growing table 1 by just 1 entry must still fail, because the
    /// CROSS-TABLE total would exceed `MAX_TABLE_ELEMENTS` -- even though
    /// table 1's own per-table check trivially passes on its own (1 is
    /// nowhere near the cap in isolation). This is the runtime
    /// counterpart to `wasm-validator`'s declare-time "Check 2b".
    #[test]
    fn table_grow_rejects_growth_that_would_exceed_the_cross_table_aggregate_cap() {
        let func_type = FuncType { params: vec![], results: vec![ValueType::I32] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0xD0, 0x70, // ref.null func
                0x41, 0x01, // i32.const 1 (delta)
                0xFC, 0x0F, 0x01, // table.grow 1
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(MAX_TABLE_ELEMENTS, None), Table::new(0, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert_eq!(engine.call_function(0, &[]).unwrap(), vec![WasmValue::I32(-1)]);
    }

    #[test]
    fn table_fill_writes_the_targeted_range_and_traps_cleanly_out_of_bounds() {
        // i32.const 1 (dest); ref.null func (value); i32.const 2 (len);
        // table.fill 0 -- fills slots [1,3) with a null funcref, i.e. a
        // no-op content-wise but proves the whole pop-order/dispatch path.
        let func_type = FuncType { params: vec![], results: vec![] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x01, // i32.const 1 (dest)
                0xD0, 0x70, // ref.null func (value)
                0x41, 0x02, // i32.const 2 (len)
                0xFC, 0x11, 0x00, // table.fill 0
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        assert!(engine.call_function(0, &[]).is_ok());

        // Out-of-bounds: dest=4, len=3 on a size-5 table -> traps, not a panic.
        let func_type2 = FuncType { params: vec![], results: vec![] };
        let body2 = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x04, // i32.const 4 (dest)
                0xD0, 0x70, // ref.null func (value)
                0x41, 0x03, // i32.const 3 (len)
                0xFC, 0x11, 0x00, // table.fill 0
                0x0B,
            ],
        };
        let mut engine2 = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type2],
            func_bodies: vec![Some(body2)],
            host_functions: vec![None],
        });
        assert!(engine2.call_function(0, &[]).is_err());
    }

    #[test]
    fn memory_init_copies_bytes_from_the_data_segment_into_memory() {
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            results: vec![],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0 (dest)
                0x20, 0x01, // local.get 1 (src)
                0x20, 0x02, // local.get 2 (len)
                0xFC, 0x08, 0x00, 0x00, // memory.init 0 (memidx 0)
                0x0B, // end
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![LinearMemory::new(1, None)],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_data_segments(vec![vec![0xAA, 0xBB, 0xCC, 0xDD]]);
        engine.set_dropped_data_segments(vec![false]);

        engine
            .call_function(0, &[WasmValue::I32(10), WasmValue::I32(1), WasmValue::I32(2)])
            .unwrap();
        let state = engine.into_state();
        assert_eq!(&state.memories[0].data[10..12], &[0xBB, 0xCC]);
    }

    /// Smoke test for task #97's `table.init`/`table.copy`/`elem.drop`:
    /// copy a passive elem segment (mixing a real funcref index and a
    /// `ref.null` entry) into table 0, then `table.copy` that range into
    /// table 1, then `elem.drop` the segment and confirm a later
    /// nonzero-length `table.init` traps while a zero-length one still
    /// succeeds -- mirroring `data_drop_makes_a_later_nonzero_memory_
    /// init_trap_but_zero_length_still_succeeds` below for tables.
    #[test]
    fn table_init_copy_elem_drop_end_to_end() {
        let func_type = FuncType { params: vec![], results: vec![] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x00, // i32.const 0 (dest)
                0x41, 0x00, // i32.const 0 (src)
                0x41, 0x03, // i32.const 3 (len)
                0xFC, 0x0C, 0x00, 0x00, // table.init elem_idx=0 table_idx=0
                0x41, 0x00, // i32.const 0 (dest)
                0x41, 0x00, // i32.const 0 (src)
                0x41, 0x03, // i32.const 3 (len)
                0xFC, 0x0E, 0x01, 0x00, // table.copy dst_table=1 src_table=0
                0xFC, 0x0D, 0x00, // elem.drop 0
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None), Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_elements(vec![vec![Some(7), None, Some(9)]]);
        engine.set_dropped_elements(vec![false]);

        engine.call_function(0, &[]).unwrap();
        let state = engine.into_state();
        assert_eq!(state.tables[0].get(0).unwrap(), Some(7), "table.init must copy the real funcref entry");
        assert_eq!(state.tables[0].get(1).unwrap(), None, "table.init must copy the ref.null entry as None");
        assert_eq!(state.tables[0].get(2).unwrap(), Some(9));
        assert_eq!(state.tables[1].get(0).unwrap(), Some(7), "table.copy must have copied table 0 into table 1");
        assert_eq!(state.tables[1].get(1).unwrap(), None);
        assert_eq!(state.tables[1].get(2).unwrap(), Some(9));
    }

    #[test]
    fn table_init_after_elem_drop_traps_on_nonzero_length_but_succeeds_at_zero_length() {
        let func_type = FuncType { params: vec![], results: vec![] };
        let body_nonzero = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x00, 0x41, 0x00, 0x41, 0x01, // dest=0 src=0 len=1
                0xFC, 0x0C, 0x00, 0x00, // table.init elem_idx=0 table_idx=0
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type.clone()],
            func_bodies: vec![Some(body_nonzero)],
            host_functions: vec![None],
        });
        engine.set_elements(vec![vec![Some(1)]]);
        engine.set_dropped_elements(vec![true]); // already dropped
        assert!(engine.call_function(0, &[]).is_err(), "nonzero-length table.init on a dropped segment must trap");

        let body_zero = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x00, 0x41, 0x00, 0x41, 0x00, // dest=0 src=0 len=0
                0xFC, 0x0C, 0x00, 0x00,
                0x0B,
            ],
        };
        let mut engine2 = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body_zero)],
            host_functions: vec![None],
        });
        engine2.set_elements(vec![vec![Some(1)]]);
        engine2.set_dropped_elements(vec![true]);
        assert!(engine2.call_function(0, &[]).is_ok(), "zero-length table.init on a dropped segment must still succeed");
    }

    /// Security review pattern (task #97, mirroring task #95's own
    /// `memory_init_with_an_out_of_range_data_idx...` test): an
    /// out-of-range `elem_idx` must trap cleanly, even at zero length --
    /// never silently succeed.
    #[test]
    fn table_init_with_an_out_of_range_elem_idx_traps_cleanly_even_at_zero_length() {
        let func_type = FuncType { params: vec![], results: vec![] };
        let body = FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0x00, 0x41, 0x00, 0x41, 0x00, // dest=0 src=0 len=0
                0xFC, 0x0C, 0x05, 0x00, // table.init elem_idx=5 (out of range) table_idx=0
                0x0B,
            ],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: Vec::new(),
            tables: vec![Table::new(5, None)],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        // Deliberately empty -- elem_idx=5 is out of range for both.
        engine.set_elements(vec![]);
        engine.set_dropped_elements(vec![]);
        let result = engine.call_function(0, &[]);
        assert!(result.is_err(), "out-of-range elem_idx must trap, not silently succeed: {result:?}");
    }

    #[test]
    fn data_drop_makes_a_later_nonzero_memory_init_trap_but_zero_length_still_succeeds() {
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            results: vec![],
        };
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x20, 0x02, 0xFC, 0x08, 0x00, 0x00, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![LinearMemory::new(1, None)],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        engine.set_data_segments(vec![vec![0xAA, 0xBB]]);
        // Already dropped -- behaves as if the segment had length 0.
        engine.set_dropped_data_segments(vec![true]);

        // Zero-length init on a dropped segment still succeeds (src=0,
        // len=0 is within the dropped segment's effective 0-length
        // bounds).
        assert!(engine
            .call_function(0, &[WasmValue::I32(0), WasmValue::I32(0), WasmValue::I32(0)])
            .is_ok());
        // Any nonzero-length init on a dropped segment traps.
        assert!(engine
            .call_function(0, &[WasmValue::I32(0), WasmValue::I32(0), WasmValue::I32(1)])
            .is_err());
    }

    /// Security review (task #95): a `memory.init` whose `data_idx` is
    /// out of range for `data_segments`/`dropped_data_segments` (never
    /// happens through the real, intended pipeline -- `wasm-validator`
    /// bounds-checks it and `wasm-runtime` always keeps both `Vec`s
    /// sized to `module.data.len()` -- but this crate's own public
    /// `WasmExecutionEngine` API is legitimately usable standalone, and
    /// never trusts a decoded index at runtime regardless of what
    /// validation SHOULD have caught) must still trap cleanly, not
    /// panic. A zero-length init (`src=0, len=0`) is the exact case that
    /// slipped through: the old bounds-check computed `segment_len = 0`
    /// via `.get(idx).unwrap_or(0)`, which a zero-length op trivially
    /// satisfies (`0 <= 0`), then indexed `ctx.data_segments[idx]`
    /// directly in the copy step -- panicking on the very index the
    /// bounds check had just "passed."
    #[test]
    fn memory_init_with_an_out_of_range_data_idx_traps_cleanly_even_at_zero_length() {
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I32, ValueType::I32],
            results: vec![],
        };
        // data_idx = 5, but no data segments registered at all.
        let body = FunctionBody {
            locals: vec![],
            code: vec![0x20, 0x00, 0x20, 0x01, 0x20, 0x02, 0xFC, 0x08, 0x05, 0x00, 0x0B],
        };
        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memories: vec![LinearMemory::new(1, None)],
            tables: vec![],
            globals: vec![],
            global_types: vec![],
            func_types: vec![func_type],
            func_bodies: vec![Some(body)],
            host_functions: vec![None],
        });
        // Deliberately empty -- `data_idx=5` is out of range for both.
        engine.set_data_segments(vec![]);
        engine.set_dropped_data_segments(vec![]);

        // Zero-length: this is the case that previously panicked instead
        // of returning a clean Err.
        let result = engine.call_function(0, &[WasmValue::I32(0), WasmValue::I32(0), WasmValue::I32(0)]);
        assert!(result.is_err(), "out-of-range data_idx must trap, not silently succeed: {result:?}");
        // Nonzero-length: was already correctly trapping before this fix,
        // included here so both shapes are pinned by the same test.
        let result = engine.call_function(0, &[WasmValue::I32(0), WasmValue::I32(0), WasmValue::I32(1)]);
        assert!(result.is_err());
    }
}
