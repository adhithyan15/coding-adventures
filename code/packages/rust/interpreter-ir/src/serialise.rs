//! Binary serialisation for [`IIRModule`].
//!
//! # Wire format (little-endian throughout)
//!
//! ```text
//! Header (magic + version):
//!     4 bytes  magic       0x49 0x49 0x52 0x00  (b"IIR\0")
//!     1 byte   version_major
//!     1 byte   version_minor
//!     4 bytes  fn_count    number of IIRFunction records
//!     str      module name (4-byte length prefix + UTF-8)
//!     str      language
//!     str      entry_point (empty string = no entry point)
//!
//! Strings are encoded as a 4-byte little-endian length (u32) followed by
//! the UTF-8 bytes.  This prevents silent truncation for strings longer than
//! 65535 bytes (a u16 prefix would silently corrupt such strings).
//!
//! For each IIRFunction:
//!     str      name
//!     str      return_type
//!     1 byte   param_count
//!     For each param:
//!         str  param_name
//!         str  type_hint
//!     4 bytes  instr_count
//!     1 byte   register_count
//!     1 byte   type_status   (0=Untyped, 1=PartiallyTyped, 2=FullyTyped)
//!     For each IIRInstr:
//!         str  op
//!         1 byte has_dest  (0 or 1)
//!         if has_dest: str dest
//!         str  type_hint
//!         1 byte src_count
//!         For each src:
//!             1 byte kind  (0=Var, 1=Int, 2=Float, 3=Bool)
//!             if kind==0: str value
//!             if kind==1: 8 bytes i64
//!             if kind==2: 8 bytes f64
//!             if kind==3: 1 byte  (0=false, 1=true)
//! ```
//!
//! **Note:** Runtime profiling fields (`observed_type`, `observation_count`,
//! `observed_slot`, `deopt_anchor`) are NOT serialised — they are transient
//! state that accumulates fresh on each run.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::serialise::{serialise, deserialise};
//!
//! let mut module = IIRModule::new("test", "basic");
//! module.entry_point = None;
//! let bytes = serialise(&module);
//! let recovered = deserialise(&bytes).unwrap();
//! assert_eq!(recovered.name, "test");
//! assert_eq!(recovered.language, "basic");
//! assert_eq!(recovered.entry_point, None);
//! ```

use crate::function::{FunctionTypeStatus, IIRFunction};
use crate::instr::{IIRInstr, Operand};
use crate::module::IIRModule;

const MAGIC: &[u8; 4] = b"IIR\0";
const VERSION_MAJOR: u8 = 1;
const VERSION_MINOR: u8 = 0;

/// Maximum number of functions/instructions to pre-allocate during
/// deserialisation.  The actual count comes from the untrusted wire; capping
/// pre-allocation prevents a crafted header from triggering a heap-exhaustion
/// DoS (e.g. `fn_count = 4_294_967_295` → ~34 GB allocation attempt).
///
/// If a module legitimately has more items than this, `Vec` will grow
/// incrementally as items are pushed — the cap only limits the initial
/// reservation, not the final size.
const MAX_SAFE_PREALLOC: usize = 65_536;

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

fn write_u8(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
}

fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_i64_le(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_f64_le(buf: &mut Vec<u8>, v: f64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    let encoded = s.as_bytes();
    // Use a 4-byte (u32) length prefix so strings longer than 65 535 bytes
    // cannot silently corrupt the stream (a u16 prefix would truncate).
    write_u32_le(buf, encoded.len() as u32);
    buf.extend_from_slice(encoded);
}

// ---------------------------------------------------------------------------
// Serialise
// ---------------------------------------------------------------------------

/// Serialise an `IIRModule` to a compact binary representation.
pub fn serialise(module: &IIRModule) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(MAGIC);
    write_u8(&mut buf, VERSION_MAJOR);
    write_u8(&mut buf, VERSION_MINOR);
    write_u32_le(&mut buf, module.functions.len() as u32);
    write_str(&mut buf, &module.name);
    write_str(&mut buf, &module.language);
    write_str(&mut buf, module.entry_point.as_deref().unwrap_or(""));

    for fn_ in &module.functions {
        serialise_function(&mut buf, fn_);
    }

    buf
}

fn serialise_function(buf: &mut Vec<u8>, fn_: &IIRFunction) {
    write_str(buf, &fn_.name);
    write_str(buf, &fn_.return_type);
    // Use u32 (not u8) for all counts — u8 silently truncates above 255,
    // causing stream desync when the deserialiser reads too few/many records.
    write_u32_le(buf, fn_.params.len() as u32);
    for (param_name, param_type) in &fn_.params {
        write_str(buf, param_name);
        write_str(buf, param_type);
    }
    write_u32_le(buf, fn_.instructions.len() as u32);
    // register_count as u32 to avoid truncation for large register files.
    write_u32_le(buf, fn_.register_count as u32);
    write_u8(buf, type_status_to_byte(&fn_.type_status));
    for instr in &fn_.instructions {
        serialise_instr(buf, instr);
    }
}

fn serialise_instr(buf: &mut Vec<u8>, instr: &IIRInstr) {
    write_str(buf, &instr.op);
    match &instr.dest {
        Some(dest) => {
            write_u8(buf, 1);
            write_str(buf, dest);
        }
        None => write_u8(buf, 0),
    }
    write_str(buf, &instr.type_hint);
    // Use u32 for src_count to avoid truncation above 255.
    write_u32_le(buf, instr.srcs.len() as u32);
    for src in &instr.srcs {
        match src {
            Operand::Var(s) => {
                write_u8(buf, 0);
                write_str(buf, s);
            }
            Operand::Int(n) => {
                write_u8(buf, 1);
                write_i64_le(buf, *n);
            }
            Operand::Float(f) => {
                write_u8(buf, 2);
                write_f64_le(buf, *f);
            }
            Operand::Bool(b) => {
                write_u8(buf, 3);
                write_u8(buf, *b as u8);
            }
        }
    }
}

fn type_status_to_byte(status: &FunctionTypeStatus) -> u8 {
    match status {
        FunctionTypeStatus::Untyped => 0,
        FunctionTypeStatus::PartiallyTyped => 1,
        FunctionTypeStatus::FullyTyped => 2,
    }
}

// ---------------------------------------------------------------------------
// Deserialise
// ---------------------------------------------------------------------------

/// Error type for deserialisation failures.
#[derive(Debug, PartialEq)]
pub struct DeserialiseError(pub String);

impl std::fmt::Display for DeserialiseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IIR deserialise error: {}", self.0)
    }
}

impl std::error::Error for DeserialiseError {}

// Reader helper — a simple cursor wrapper over a byte slice.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    fn read_exact(&mut self, n: usize) -> Result<&[u8], DeserialiseError> {
        if self.pos + n > self.data.len() {
            return Err(DeserialiseError(format!(
                "unexpected end of data at offset {} (need {n} bytes, have {})",
                self.pos,
                self.data.len() - self.pos,
            )));
        }
        let chunk = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(chunk)
    }

    fn u8(&mut self) -> Result<u8, DeserialiseError> {
        Ok(self.read_exact(1)?[0])
    }

    fn u32_le(&mut self) -> Result<u32, DeserialiseError> {
        let b = self.read_exact(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64_le(&mut self) -> Result<i64, DeserialiseError> {
        let b = self.read_exact(8)?;
        Ok(i64::from_le_bytes(b.try_into().unwrap()))
    }

    fn f64_le(&mut self) -> Result<f64, DeserialiseError> {
        let b = self.read_exact(8)?;
        Ok(f64::from_le_bytes(b.try_into().unwrap()))
    }

    fn str_(&mut self) -> Result<String, DeserialiseError> {
        // Read 4-byte u32 length prefix (matches the u32 written by write_str).
        let len = self.u32_le()? as usize;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| DeserialiseError(format!("invalid UTF-8: {e}")))
    }
}

/// Deserialise bytes produced by [`serialise`] back to an `IIRModule`.
pub fn deserialise(data: &[u8]) -> Result<IIRModule, DeserialiseError> {
    let mut r = Reader::new(data);

    let magic = r.read_exact(4)?;
    if magic != MAGIC {
        return Err(DeserialiseError(format!(
            "invalid magic bytes: {magic:?} (expected {MAGIC:?})"
        )));
    }

    let major = r.u8()?;
    let minor = r.u8()?;
    if (major, minor) != (VERSION_MAJOR, VERSION_MINOR) {
        return Err(DeserialiseError(format!(
            "unsupported version {major}.{minor}"
        )));
    }

    let fn_count = r.u32_le()? as usize;
    let name = r.str_()?;
    let language = r.str_()?;
    let ep_raw = r.str_()?;
    let entry_point = if ep_raw.is_empty() { None } else { Some(ep_raw) };

    // Cap pre-allocation to MAX_SAFE_PREALLOC.  A crafted header with
    // fn_count = u32::MAX would otherwise attempt a ~34 GB allocation before
    // any bytes are validated.  Vec grows incrementally for legitimate modules
    // that exceed this sentinel.
    let mut functions = Vec::with_capacity(fn_count.min(MAX_SAFE_PREALLOC));
    for _ in 0..fn_count {
        functions.push(deserialise_function(&mut r)?);
    }

    Ok(IIRModule {
        name,
        functions,
        entry_point,
        language,
    })
}

fn deserialise_function(r: &mut Reader<'_>) -> Result<IIRFunction, DeserialiseError> {
    let name = r.str_()?;
    let return_type = r.str_()?;
    // Read u32 for param_count (matches serialiser which writes u32).
    // A u8 count silently truncates functions with ≥ 256 params, causing
    // stream desync.
    let param_count = r.u32_le()? as usize;
    let mut params = Vec::with_capacity(param_count.min(MAX_SAFE_PREALLOC));
    for _ in 0..param_count {
        let param_name = r.str_()?;
        let param_type = r.str_()?;
        params.push((param_name, param_type));
    }
    let instr_count = r.u32_le()? as usize;
    // Read u32 for register_count (serialiser now writes u32).
    // Cap it to MAX_SAFE_PREALLOC: an uncapped value from a crafted binary can
    // reach u32::MAX, causing VMFrame::new to attempt a ~34 GB allocation via
    // vec![Value::Null; register_count] before any further validation.
    let register_count = (r.u32_le()? as usize).min(MAX_SAFE_PREALLOC);
    let type_status = byte_to_type_status(r.u8()?)?;

    // Same pre-allocation cap as for fn_count — each IIRInstr is a fat struct.
    let mut instructions = Vec::with_capacity(instr_count.min(MAX_SAFE_PREALLOC));
    for _ in 0..instr_count {
        instructions.push(deserialise_instr(r)?);
    }

    Ok(IIRFunction {
        name,
        params,
        return_type,
        instructions,
        register_count,
        type_status,
        call_count: 0,
        feedback_slots: std::collections::HashMap::new(),
        source_map: Vec::new(),
        param_refinements: Vec::new(),
        return_refinement: None,
    })
}

fn deserialise_instr(r: &mut Reader<'_>) -> Result<IIRInstr, DeserialiseError> {
    let op = r.str_()?;
    let has_dest = r.u8()?;
    let dest = if has_dest != 0 { Some(r.str_()?) } else { None };
    let type_hint = r.str_()?;
    // Read u32 for src_count (serialiser now writes u32).
    // Apply the same MAX_SAFE_PREALLOC cap to bound pre-allocation.
    let src_count = r.u32_le()? as usize;

    let mut srcs = Vec::with_capacity(src_count.min(MAX_SAFE_PREALLOC));
    for _ in 0..src_count {
        let kind = r.u8()?;
        let operand = match kind {
            0 => Operand::Var(r.str_()?),
            1 => Operand::Int(r.i64_le()?),
            2 => Operand::Float(r.f64_le()?),
            3 => Operand::Bool(r.u8()? != 0),
            k => {
                return Err(DeserialiseError(format!(
                    "unknown operand kind byte: {k}"
                )));
            }
        };
        srcs.push(operand);
    }

    Ok(IIRInstr::new(op, dest, srcs, type_hint))
}

fn byte_to_type_status(b: u8) -> Result<FunctionTypeStatus, DeserialiseError> {
    match b {
        0 => Ok(FunctionTypeStatus::Untyped),
        1 => Ok(FunctionTypeStatus::PartiallyTyped),
        2 => Ok(FunctionTypeStatus::FullyTyped),
        n => Err(DeserialiseError(format!("unknown type_status byte: {n}"))),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::instr::Operand;
    use crate::function::FunctionTypeStatus;

    fn make_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "add",
            vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
            "u8",
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
            ],
        );
        let mut module = IIRModule::new("test.bas", "basic");
        module.add_or_replace(fn_);
        module
    }

    #[test]
    fn round_trip_module_name_and_language() {
        let module = make_module();
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.name, "test.bas");
        assert_eq!(recovered.language, "basic");
    }

    #[test]
    fn round_trip_entry_point_some() {
        let mut module = make_module();
        module.entry_point = Some("main".into());
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.entry_point, Some("main".to_string()));
    }

    #[test]
    fn round_trip_entry_point_none() {
        let mut module = make_module();
        module.entry_point = None;
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.entry_point, None);
    }

    #[test]
    fn round_trip_function_structure() {
        let module = make_module();
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        let fn_ = recovered.get_function("add").unwrap();
        assert_eq!(fn_.params, vec![("a".into(), "u8".into()), ("b".into(), "u8".into())]);
        assert_eq!(fn_.return_type, "u8");
        assert_eq!(fn_.instructions.len(), 2);
        assert_eq!(fn_.type_status, FunctionTypeStatus::FullyTyped);
    }

    #[test]
    fn round_trip_int_operand() {
        let mut module = IIRModule::new("t", "x");
        module.entry_point = None;
        let fn_ = IIRFunction::new(
            "const_fn",
            vec![],
            "i32",
            vec![IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-42)], "i32")],
        );
        module.add_or_replace(fn_);
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        let instr = &recovered.get_function("const_fn").unwrap().instructions[0];
        assert_eq!(instr.srcs[0], Operand::Int(-42));
    }

    #[test]
    fn round_trip_float_operand() {
        let mut module = IIRModule::new("t", "x");
        module.entry_point = None;
        let fn_ = IIRFunction::new(
            "f",
            vec![],
            "f64",
            vec![IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64")],
        );
        module.add_or_replace(fn_);
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        let instr = &recovered.get_function("f").unwrap().instructions[0];
        assert!(matches!(instr.srcs[0], Operand::Float(v) if (v - 3.14).abs() < 1e-10));
    }

    #[test]
    fn round_trip_bool_operand() {
        let mut module = IIRModule::new("t", "x");
        module.entry_point = None;
        let fn_ = IIRFunction::new(
            "f",
            vec![],
            "bool",
            vec![IIRInstr::new("const", Some("v".into()), vec![Operand::Bool(true)], "bool")],
        );
        module.add_or_replace(fn_);
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        let instr = &recovered.get_function("f").unwrap().instructions[0];
        assert_eq!(instr.srcs[0], Operand::Bool(true));
    }

    #[test]
    fn invalid_magic_returns_error() {
        let bad = b"BAD\0xxxx";
        let err = deserialise(bad).unwrap_err();
        assert!(err.0.contains("magic"));
    }

    #[test]
    fn profiling_fields_are_reset_on_round_trip() {
        let mut module = make_module();
        // Simulate some profiling state on the instruction.
        let fn_ = module.get_function_mut("add").unwrap();
        fn_.instructions[0].record_observation("u8");
        assert_eq!(fn_.instructions[0].observation_count, 1);

        // After serialise/deserialise, profiling state is gone.
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(
            recovered.get_function("add").unwrap().instructions[0].observation_count,
            0
        );
    }
}
