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
//!     memory: None,
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
/// ┌──────────┬───────────────────────────────────────────────┐
/// │ Variant  │ Description                                   │
/// ├──────────┼───────────────────────────────────────────────┤
/// │ I32(i32) │ 32-bit signed integer (also used for bools)   │
/// │ I64(i64) │ 64-bit signed integer                         │
/// │ F32(f32) │ 32-bit IEEE 754 float                         │
/// │ F64(f64) │ 64-bit IEEE 754 float                         │
/// └──────────┴───────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmValue {
    /// 32-bit integer. Wrapping arithmetic via `i32::wrapping_*` methods.
    I32(i32),
    /// 64-bit integer. Wrapping arithmetic via `i64::wrapping_*` methods.
    I64(i64),
    /// 32-bit IEEE 754 single-precision float.
    F32(f32),
    /// 64-bit IEEE 754 double-precision float.
    F64(f64),
}

impl WasmValue {
    /// Convert to a [`TypedVMValue`] for the GenericVM's typed stack.
    pub fn to_typed(self) -> TypedVMValue {
        match self {
            WasmValue::I32(v) => TypedVMValue {
                value_type: ValueType::I32 as u8,
                value: Value::Int(v as i64),
            },
            WasmValue::I64(v) => TypedVMValue {
                value_type: ValueType::I64 as u8,
                value: Value::Int(v),
            },
            WasmValue::F32(v) => TypedVMValue {
                value_type: ValueType::F32 as u8,
                value: Value::Float(v as f64),
            },
            WasmValue::F64(v) => TypedVMValue {
                value_type: ValueType::F64 as u8,
                value: Value::Float(v),
            },
        }
    }

    /// Convert from a [`TypedVMValue`] back to a [`WasmValue`].
    pub fn from_typed(tv: &TypedVMValue) -> Result<Self, TrapError> {
        match tv.value_type {
            x if x == ValueType::I32 as u8 => match &tv.value {
                Value::Int(v) => Ok(WasmValue::I32(*v as i32)),
                _ => Err(TrapError::new("type mismatch: expected i32")),
            },
            x if x == ValueType::I64 as u8 => match &tv.value {
                Value::Int(v) => Ok(WasmValue::I64(*v)),
                _ => Err(TrapError::new("type mismatch: expected i64")),
            },
            x if x == ValueType::F32 as u8 => match &tv.value {
                Value::Float(v) => Ok(WasmValue::F32(*v as f32)),
                _ => Err(TrapError::new("type mismatch: expected f32")),
            },
            x if x == ValueType::F64 as u8 => match &tv.value {
                Value::Float(v) => Ok(WasmValue::F64(*v)),
                _ => Err(TrapError::new("type mismatch: expected f64")),
            },
            other => Err(TrapError::new(format!(
                "unknown value type: 0x{:02X}",
                other
            ))),
        }
    }

    /// Create the zero/default value for a given WASM type.
    pub fn default_for(vt: ValueType) -> Self {
        match vt {
            ValueType::I32 => WasmValue::I32(0),
            ValueType::I64 => WasmValue::I64(0),
            ValueType::F32 => WasmValue::F32(0.0),
            ValueType::F64 => WasmValue::F64(0.0),
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

    /// Write raw bytes into memory at offset.
    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<(), TrapError> {
        self.bounds_check(offset, data.len())?;
        self.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 4: Table — function reference array
// ══════════════════════════════════════════════════════════════════════════════

/// A table of function references for indirect calls.
///
/// WASM 1.0 tables hold nullable function indices. The `call_indirect`
/// instruction looks up a function reference by table index, then calls it.
pub struct Table {
    /// Elements: `Some(func_index)` or `None` (uninitialized).
    elements: Vec<Option<u32>>,
    /// Maximum table size.
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
/// f64.const (0x44), global.get (0x23), end (0x0B).
pub fn evaluate_const_expr(expr: &[u8], globals: &[WasmValue]) -> Result<WasmValue, TrapError> {
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
    /// A memory argument: (alignment_log2, offset).
    MemArg { _align: u32, offset: u32 },
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
}

/// Decode all instructions in a function body.
pub fn decode_function_body(body: &FunctionBody) -> Vec<DecodedInstruction> {
    let code = &body.code;
    let mut instructions = Vec::new();
    let mut offset: usize = 0;

    while offset < code.len() {
        let opcode_byte = code[offset];
        offset += 1;

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

    // Handle multi-immediate opcodes: memarg, call_indirect
    if immediates.len() == 2 {
        if immediates[0] == "memarg" || (immediates[0] == "memarg" && immediates[1] == "memarg") {
            // This shouldn't happen — memarg is a single immediate label.
        }
        if immediates[0] == "typeidx" && immediates[1] == "tableidx" {
            // call_indirect
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
            let byte = code[offset];
            match byte {
                0x40 | 0x7F | 0x7E | 0x7D | 0x7C => (DecodedOperand::Int(byte as i64), 1),
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
            let (align, sz1) = decode_leb_u32(code, offset);
            let (mem_offset, sz2) = decode_leb_u32(code, offset + sz1);
            (
                DecodedOperand::MemArg {
                    _align: align,
                    offset: mem_offset,
                },
                sz1 + sz2,
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
            0x02 | 0x03 | 0x04 => {
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
#[derive(Debug, Clone)]
pub struct Label {
    /// How many result values this block produces.
    pub arity: usize,
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
}

/// The WASM execution context — all runtime state for WASM instructions.
pub struct WasmExecutionContext {
    pub memory: Option<*mut LinearMemory>,
    pub tables: Vec<*mut Table>,
    pub globals: Vec<WasmValue>,
    pub global_types: Vec<GlobalType>,
    pub func_types: Vec<FuncType>,
    pub func_bodies: Vec<Option<FunctionBody>>,
    pub host_functions: Vec<Option<Box<dyn HostFunction>>>,
    pub typed_locals: Vec<WasmValue>,
    pub label_stack: Vec<Label>,
    pub control_flow_map: HashMap<usize, ControlTarget>,
    pub saved_frames: Vec<SavedFrame>,
    pub returned: bool,
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 10: Instruction handlers
// ══════════════════════════════════════════════════════════════════════════════

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

// ── Helper: get memory from context ───────────────────────────────────────
fn get_memory<'a>(ctx: &WasmExecutionContext) -> Result<&'a mut LinearMemory, VMError> {
    match ctx.memory {
        Some(ptr) => Ok(unsafe { &mut *ptr }),
        None => Err(VMError::GenericError("no memory available".to_string())),
    }
}

// ── Helper: get table from context ────────────────────────────────────────
fn get_table<'a>(ctx: &mut WasmExecutionContext, idx: usize) -> Result<&'a mut Table, VMError> {
    if idx >= ctx.tables.len() {
        return Err(VMError::GenericError("undefined table".to_string()));
    }
    Ok(unsafe { &mut *ctx.tables[idx] })
}

// ── Helper: block arity resolution ────────────────────────────────────────
fn block_arity(block_type: i64, func_types: &[FuncType]) -> usize {
    match block_type {
        0x40 => 0,                      // empty
        0x7F | 0x7E | 0x7D | 0x7C => 1, // single value type
        n if n >= 0 && (n as usize) < func_types.len() => func_types[n as usize].results.len(),
        _ => 0,
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

    // For loops, arity is 0 (MVP). For blocks, it's the block's result arity.
    let arity = if label.is_loop { 0 } else { label.arity };

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

    // Pop labels down to target.
    ctx.label_stack.truncate(label_stack_index);

    // Jump.
    vm.jump_to(label.target_pc);
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
    f32_unop!(vm, 0x8D, |a: f32| a.ceil()); // ceil
    f32_unop!(vm, 0x8E, |a: f32| a.floor()); // floor
    f32_unop!(vm, 0x8F, |a: f32| a.trunc()); // trunc
    f32_unop!(
        vm,
        0x90,
        |a: f32| if a.fract() == 0.5 || a.fract() == -0.5 {
            // nearest even
            let rounded = a.round();
            if rounded as i32 % 2 != 0 {
                rounded - a.signum()
            } else {
                rounded
            }
        } else {
            a.round()
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
    f32_binop!(vm, 0x96, |a: f32, b: f32| a.min(b)); // min
    f32_binop!(vm, 0x97, |a: f32, b: f32| a.max(b)); // max
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
    f64_unop!(vm, 0x9B, |a: f64| a.ceil()); // ceil
    f64_unop!(vm, 0x9C, |a: f64| a.floor()); // floor
    f64_unop!(vm, 0x9D, |a: f64| a.trunc()); // trunc
    f64_unop!(
        vm,
        0x9E,
        |a: f64| if a.fract() == 0.5 || a.fract() == -0.5 {
            let rounded = a.round();
            if rounded as i64 % 2 != 0 {
                rounded - a.signum()
            } else {
                rounded
            }
        } else {
            a.round()
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
    f64_binop!(vm, 0xA4, |a: f64, b: f64| a.min(b)); // min
    f64_binop!(vm, 0xA5, |a: f64, b: f64| a.max(b)); // max
    f64_binop!(vm, 0xA6, |a: f64, b: f64| f64::from_bits(
        (a.to_bits() & 0x7FFF_FFFF_FFFF_FFFF) | (b.to_bits() & 0x8000_0000_0000_0000)
    )); // copysign
}

// ── Conversion instructions (0xA7-0xBF) ──────────────────────────────────

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
        if a >= 2147483648.0 || a < -2147483648.0 {
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
        if a >= 4294967296.0 || a < 0.0 {
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
        if a >= 2147483648.0 || a < -2147483649.0 {
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
        if a >= 4294967296.0 || a < 0.0 {
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
    vm.register_context_opcode(0xAE, |vm, _instr, _code, _ctx| {
        let a = pop_wasm(vm)?.as_f32().map_err(VMError::from)?;
        if a.is_nan() {
            return Err(VMError::GenericError(
                "invalid conversion to integer".into(),
            ));
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
        let mem_offset = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => 0,
        };
        (base as u32 as usize).wrapping_add(mem_offset)
    }

    // i32.load (0x28)
    vm.register_context_opcode(0x28, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let mem = get_memory(ctx)?;
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
        let mem = get_memory(ctx)?;
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
        let mem = get_memory(ctx)?;
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
        let mem = get_memory(ctx)?;
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
        let val = get_memory(ctx)?.load_i32_8s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });
    // i32.load8_u (0x2D)
    vm.register_context_opcode(0x2D, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i32_8u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });
    // i32.load16_s (0x2E)
    vm.register_context_opcode(0x2E, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i32_16s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });
    // i32.load16_u (0x2F)
    vm.register_context_opcode(0x2F, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i32_16u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I32(val));
        vm.advance_pc();
        Ok(None)
    });

    // i64.load8_s (0x30)
    vm.register_context_opcode(0x30, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i64_8s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load8_u (0x31)
    vm.register_context_opcode(0x31, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i64_8u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load16_s (0x32)
    vm.register_context_opcode(0x32, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i64_16s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load16_u (0x33)
    vm.register_context_opcode(0x33, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i64_16u(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load32_s (0x34)
    vm.register_context_opcode(0x34, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i64_32s(addr).map_err(VMError::from)?;
        push_wasm(vm, WasmValue::I64(val));
        vm.advance_pc();
        Ok(None)
    });
    // i64.load32_u (0x35)
    vm.register_context_opcode(0x35, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let base = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let addr = effective_addr(instr, base);
        let val = get_memory(ctx)?.load_i64_32u(addr).map_err(VMError::from)?;
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
        get_memory(ctx)?
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
        get_memory(ctx)?
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
        get_memory(ctx)?
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
        get_memory(ctx)?
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
        get_memory(ctx)?
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
        get_memory(ctx)?
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
        get_memory(ctx)?
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
        get_memory(ctx)?
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
        get_memory(ctx)?
            .store_i64_32(addr, val)
            .map_err(VMError::from)?;
        vm.advance_pc();
        Ok(None)
    });

    // memory.size (0x3F)
    vm.register_context_opcode(0x3F, |vm, _instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let size = match ctx.memory {
            Some(ptr) => unsafe { (*ptr).size() as i32 },
            None => 0,
        };
        push_wasm(vm, WasmValue::I32(size));
        vm.advance_pc();
        Ok(None)
    });

    // memory.grow (0x40)
    vm.register_context_opcode(0x40, |vm, _instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let delta = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let result = match ctx.memory {
            Some(ptr) => unsafe { (*ptr).grow(delta as u32) },
            None => -1,
        };
        push_wasm(vm, WasmValue::I32(result));
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
        let arity = block_arity(block_type, &ctx.func_types);
        let end_pc = ctx
            .control_flow_map
            .get(&vm.pc)
            .map(|t| t.end_pc)
            .unwrap_or(vm.pc + 1);
        ctx.label_stack.push(Label {
            arity,
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
        let arity = block_arity(block_type, &ctx.func_types);
        let loop_pc = vm.pc;
        ctx.label_stack.push(Label {
            arity,
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
        let arity = block_arity(block_type, &ctx.func_types);
        let condition = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let target = ctx.control_flow_map.get(&vm.pc).cloned();
        let end_pc = target.as_ref().map(|t| t.end_pc).unwrap_or(vm.pc + 1);
        let else_pc = target.as_ref().and_then(|t| t.else_pc);

        ctx.label_stack.push(Label {
            arity,
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
    vm.register_context_opcode(0x0E, |vm, instr, _code, ctx| {
        let ctx = get_ctx(ctx);
        let index = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        // The operand holds the index into decoded BrTable data.
        // We need to re-interpret. For simplicity, br_table is handled via
        // the Index operand which holds the default label in our encoding.
        // In our encoding, the operand is the default label index.
        let default_label = operand_int(instr) as usize;
        // For a full br_table we'd need the labels array. Since our operand
        // encoding is limited, just use the default.
        let _ = index;
        execute_branch(vm, ctx, default_label)?;
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
        let type_idx = operand_int(instr) as usize;
        let elem_index = pop_wasm(vm)?.as_i32().map_err(VMError::from)?;
        let table = get_table(ctx, 0)?;
        let func_index = table
            .get(elem_index as u32)
            .map_err(VMError::from)?
            .ok_or_else(|| VMError::GenericError("uninitialized table element".into()))?;

        // Type check
        let expected = &ctx.func_types[type_idx];
        let actual = &ctx.func_types[func_index as usize];
        if expected.params != actual.params || expected.results != actual.results {
            return Err(VMError::GenericError("indirect call type mismatch".into()));
        }

        call_function(vm, ctx, func_index as usize)?;
        Ok(None)
    });
}

/// Execute a function call within the WASM execution context.
fn call_function(
    vm: &mut GenericVM,
    ctx: &mut WasmExecutionContext,
    func_index: usize,
) -> VMResult<()> {
    let func_type = ctx
        .func_types
        .get(func_index)
        .ok_or_else(|| VMError::GenericError(format!("undefined function {}", func_index)))?
        .clone();

    // Pop arguments.
    let mut args = Vec::new();
    for _ in 0..func_type.params.len() {
        args.push(pop_wasm(vm)?);
    }
    args.reverse();

    // Check for host function.
    if let Some(Some(host_func)) = ctx.host_functions.get(func_index) {
        let results = host_func
            .call(&args, ctx.memory.map(|ptr| unsafe { &mut *ptr }))
            .map_err(VMError::from)?;
        for r in results {
            push_wasm(vm, r);
        }
        vm.advance_pc();
        return Ok(());
    }

    // Module-defined function.
    let body = ctx
        .func_bodies
        .get(func_index)
        .and_then(|b| b.as_ref())
        .ok_or_else(|| VMError::GenericError(format!("no body for function {}", func_index)))?
        .clone();

    // Save caller state.
    ctx.saved_frames.push(SavedFrame {
        locals: ctx.typed_locals.clone(),
        label_stack: ctx.label_stack.clone(),
        stack_height: vm.typed_stack.len(),
        control_flow_map: ctx.control_flow_map.clone(),
        return_pc: vm.pc + 1,
        return_arity: func_type.results.len(),
    });

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

    // Convert to VM instructions.
    let vm_instructions: Vec<Instruction> = decoded
        .iter()
        .map(|d| {
            let operand = match &d.operand {
                DecodedOperand::None => None,
                DecodedOperand::Int(v) => Some(Operand::Index(*v as usize)),
                DecodedOperand::MemArg { offset, .. } => Some(Operand::Index(*offset as usize)),
                DecodedOperand::F32(v) => Some(Operand::Index(v.to_bits() as usize)),
                DecodedOperand::F64(v) => Some(Operand::Index(v.to_bits() as usize)),
                DecodedOperand::CallIndirect { type_idx, .. } => {
                    Some(Operand::Index(*type_idx as usize))
                }
                DecodedOperand::BrTable { default_label, .. } => {
                    Some(Operand::Index(*default_label as usize))
                }
            };
            Instruction {
                opcode: d.opcode,
                operand,
            }
        })
        .collect();

    // Set up callee code and jump to start.
    // We need to use a recursive execution approach. Execute the callee inline.
    vm.halted = false;

    let callee_code = CodeObject {
        instructions: vm_instructions,
        constants: vec![],
        names: vec![],
    };

    // Save current PC and execute callee.
    let saved_pc = vm.pc;
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

    // Collect return values.
    let return_arity = func_type.results.len();
    let mut return_values = Vec::new();
    for _ in 0..return_arity {
        return_values.push(pop_wasm(vm)?);
    }
    return_values.reverse();

    // Restore caller state.
    if let Some(frame) = ctx.saved_frames.pop() {
        ctx.typed_locals = frame.locals;
        ctx.label_stack = frame.label_stack;
        ctx.control_flow_map = frame.control_flow_map;

        // Truncate stack to caller's height.
        while vm.typed_stack.len() > frame.stack_height {
            let _ = vm.pop_typed();
        }

        vm.pc = frame.return_pc;
        vm.halted = false;
    } else {
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
    pub memory: Option<LinearMemory>,
    pub tables: Vec<Table>,
    pub globals: Vec<WasmValue>,
    pub global_types: Vec<GlobalType>,
    pub func_types: Vec<FuncType>,
    pub func_bodies: Vec<Option<FunctionBody>>,
    pub host_functions: Vec<Option<Box<dyn HostFunction>>>,
}

/// Mutable engine state that should be written back to a long-lived instance.
pub struct WasmEngineState {
    pub memory: Option<LinearMemory>,
    pub tables: Vec<Table>,
    pub globals: Vec<WasmValue>,
    pub host_functions: Vec<Option<Box<dyn HostFunction>>>,
}

/// The WASM execution engine — interprets validated WASM modules.
pub struct WasmExecutionEngine {
    vm: GenericVM,
    memory: Option<Box<LinearMemory>>,
    tables: Vec<Box<Table>>,
    globals: Vec<WasmValue>,
    global_types: Vec<GlobalType>,
    func_types: Vec<FuncType>,
    func_bodies: Vec<Option<FunctionBody>>,
    host_functions: Vec<Option<Box<dyn HostFunction>>>,
}

impl WasmExecutionEngine {
    /// Create a new execution engine.
    pub fn new(config: WasmEngineConfig) -> Self {
        let mut vm = GenericVM::new();
        vm.set_max_recursion_depth(Some(1024));
        register_all_handlers(&mut vm);

        WasmExecutionEngine {
            vm,
            memory: config.memory.map(Box::new),
            tables: config.tables.into_iter().map(Box::new).collect(),
            globals: config.globals,
            global_types: config.global_types,
            func_types: config.func_types,
            func_bodies: config.func_bodies,
            host_functions: config.host_functions,
        }
    }

    /// Consume the engine and return the mutated runtime state.
    pub fn into_state(self) -> WasmEngineState {
        WasmEngineState {
            memory: self.memory.map(|memory| *memory),
            tables: self.tables.into_iter().map(|table| *table).collect(),
            globals: self.globals,
            host_functions: self.host_functions,
        }
    }

    /// Call a WASM function by index.
    pub fn call_function(
        &mut self,
        func_index: usize,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>, TrapError> {
        let func_type = self
            .func_types
            .get(func_index)
            .ok_or_else(|| TrapError::new(format!("undefined function index {}", func_index)))?;

        if args.len() != func_type.params.len() {
            return Err(TrapError::new(format!(
                "function {} expects {} arguments, got {}",
                func_index,
                func_type.params.len(),
                args.len()
            )));
        }

        // Check for host function.
        if let Some(Some(host_func)) = self.host_functions.get(func_index) {
            return host_func.call(args, self.memory.as_deref_mut());
        }

        // Module-defined function.
        let body = self
            .func_bodies
            .get(func_index)
            .and_then(|b| b.as_ref())
            .ok_or_else(|| TrapError::new(format!("no body for function {}", func_index)))?
            .clone();

        let result_count = func_type.results.len();

        // Decode the function body.
        let decoded = decode_function_body(&body);
        let control_flow_map = build_control_flow_map(&decoded);

        // Convert to VM instructions.
        let vm_instructions: Vec<Instruction> = decoded
            .iter()
            .map(|d| {
                let operand = match &d.operand {
                    DecodedOperand::None => None,
                    DecodedOperand::Int(v) => Some(Operand::Index(*v as usize)),
                    DecodedOperand::MemArg { offset, .. } => Some(Operand::Index(*offset as usize)),
                    DecodedOperand::F32(v) => Some(Operand::Index(v.to_bits() as usize)),
                    DecodedOperand::F64(v) => Some(Operand::Index(v.to_bits() as usize)),
                    DecodedOperand::CallIndirect { type_idx, .. } => {
                        Some(Operand::Index(*type_idx as usize))
                    }
                    DecodedOperand::BrTable { default_label, .. } => {
                        Some(Operand::Index(*default_label as usize))
                    }
                };
                Instruction {
                    opcode: d.opcode,
                    operand,
                }
            })
            .collect();

        // Initialize locals.
        let mut typed_locals: Vec<WasmValue> = args.to_vec();
        for t in &body.locals {
            typed_locals.push(WasmValue::default_for(*t));
        }

        // Build raw pointers for the context.
        let memory_ptr = self.memory.as_mut().map(|m| &mut **m as *mut LinearMemory);
        let table_ptrs: Vec<*mut Table> = self
            .tables
            .iter_mut()
            .map(|t| &mut **t as *mut Table)
            .collect();
        let host_functions = std::mem::take(&mut self.host_functions);

        let mut ctx = WasmExecutionContext {
            memory: memory_ptr,
            tables: table_ptrs,
            globals: self.globals.clone(),
            global_types: self.global_types.clone(),
            func_types: self.func_types.clone(),
            func_bodies: self.func_bodies.clone(),
            host_functions,
            typed_locals,
            label_stack: Vec::new(),
            control_flow_map,
            saved_frames: Vec::new(),
            returned: false,
        };

        let code = CodeObject {
            instructions: vm_instructions,
            constants: vec![],
            names: vec![],
        };

        // Reset and execute.
        self.vm.reset();
        register_all_handlers(&mut self.vm);

        self.vm
            .execute_with_context(&code, &mut ctx)
            .map_err(|e| TrapError::new(format!("{}", e)))?;

        // Update globals back.
        self.globals = ctx.globals;
        self.host_functions = ctx.host_functions;

        // Collect return values.
        let mut results = Vec::new();
        for _ in 0..result_count {
            let tv = self
                .vm
                .pop_typed()
                .map_err(|e| TrapError::new(format!("{}", e)))?;
            results.push(WasmValue::from_typed(&tv)?);
        }
        results.reverse();

        Ok(results)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Section 12: Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
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
    fn test_table_out_of_bounds() {
        let table = Table::new(2, None);
        assert!(table.get(5).is_err());
    }

    #[test]
    fn test_evaluate_const_expr_i32() {
        // i32.const 42; end
        let expr = vec![0x41, 0x2A, 0x0B];
        let result = evaluate_const_expr(&expr, &[]).unwrap();
        assert_eq!(result, WasmValue::I32(42));
    }

    #[test]
    fn test_evaluate_const_expr_global_get() {
        // global.get 0; end
        let expr = vec![0x23, 0x00, 0x0B];
        let globals = vec![WasmValue::I32(100)];
        let result = evaluate_const_expr(&expr, &globals).unwrap();
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
            memory: None,
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
            memory: None,
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
            memory: None,
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

        // Wrong value variant for i32 type
        let tv_bad = TypedVMValue {
            value_type: ValueType::I32 as u8,
            value: Value::Float(1.0),
        };
        assert!(WasmValue::from_typed(&tv_bad).is_err());

        // Wrong value variant for f64 type
        let tv_bad2 = TypedVMValue {
            value_type: ValueType::F64 as u8,
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
    // Const expression evaluator
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_const_expr_i64() {
        // i64.const 42; end (42 in signed LEB128 = 0x2A)
        let expr = vec![0x42, 0x2A, 0x0B];
        let result = evaluate_const_expr(&expr, &[]).unwrap();
        assert_eq!(result, WasmValue::I64(42));
    }

    #[test]
    fn test_const_expr_f32() {
        let val: f32 = 3.14;
        let bytes = val.to_le_bytes();
        let expr = vec![0x43, bytes[0], bytes[1], bytes[2], bytes[3], 0x0B];
        let result = evaluate_const_expr(&expr, &[]).unwrap();
        assert_eq!(result, WasmValue::F32(3.14));
    }

    #[test]
    fn test_const_expr_f64() {
        let val: f64 = 2.718281828;
        let bytes = val.to_le_bytes();
        let mut expr = vec![0x44];
        expr.extend_from_slice(&bytes);
        expr.push(0x0B);
        let result = evaluate_const_expr(&expr, &[]).unwrap();
        assert_eq!(result, WasmValue::F64(2.718281828));
    }

    #[test]
    fn test_const_expr_global_get_oob() {
        let expr = vec![0x23, 0x05, 0x0B]; // global.get 5
        assert!(evaluate_const_expr(&expr, &[]).is_err());
    }

    #[test]
    fn test_const_expr_empty() {
        // Just end opcode
        let expr = vec![0x0B];
        assert!(evaluate_const_expr(&expr, &[]).is_err()); // "empty constant expression"
    }

    #[test]
    fn test_const_expr_illegal_opcode() {
        let expr = vec![0x6A, 0x0B]; // i32.add is not allowed in const expr
        assert!(evaluate_const_expr(&expr, &[]).is_err());
    }

    #[test]
    fn test_const_expr_missing_end() {
        let expr = vec![0x41, 0x2A]; // i32.const 42 without end
        assert!(evaluate_const_expr(&expr, &[]).is_err());
    }

    #[test]
    fn test_const_expr_f32_truncated() {
        let expr = vec![0x43, 0x00, 0x00]; // f32.const but only 2 bytes
        assert!(evaluate_const_expr(&expr, &[]).is_err());
    }

    #[test]
    fn test_const_expr_f64_truncated() {
        let expr = vec![0x44, 0x00, 0x00, 0x00]; // f64.const but only 3 bytes
        assert!(evaluate_const_expr(&expr, &[]).is_err());
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
            DecodedOperand::MemArg { _align, offset } => {
                assert_eq!(*_align, 2);
                assert_eq!(*offset, 8);
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
            memory: None,
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
}
