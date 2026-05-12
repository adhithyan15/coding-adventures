//! Binary serialisation for [`IIRModule`].
//!
//! # Wire format (little-endian throughout)
//!
//! ## Version 1.1 (LANG33 — current)
//!
//! ```text
//! Header (magic + version):
//!     4 bytes  magic       0x49 0x49 0x52 0x00  (b"IIR\0")
//!     1 byte   version_major   (1)
//!     1 byte   version_minor   (1)
//!     4 bytes  fn_count    number of IIRFunction records
//!     str      module name (4-byte length prefix + UTF-8)
//!     str      language
//!     str      entry_point (empty string = no entry point)
//!
//!     For each IIRFunction: (same as version 1.0)
//!
//! LANG33 extension (appended after all functions):
//!     1 byte   tag         0x10 = exports section
//!     4 bytes  export_count
//!     For each IIRExport:
//!         str  function_name
//!         str  alias  (empty string = no alias / use function_name)
//!
//!     1 byte   tag         0x11 = imports section
//!     4 bytes  import_count
//!     For each IIRImport:
//!         str  module_name
//!         str  function_name
//!         str  local_alias  (empty = no alias)
//!         4 bytes  param_count
//!         For each param type: str  type_string
//!         str  return_type
//! ```
//!
//! ## Version 1.0 (legacy — no exports/imports)
//!
//! Identical to 1.1 but without the LANG33 extension.  The deserialiser
//! accepts 1.0 and returns an `IIRModule` with empty `exports`/`imports`.
//!
//! ## Operand kind bytes
//!
//! ```text
//!     0 = Var    → str (variable name)
//!     1 = Int    → 8 bytes i64
//!     2 = Float  → 8 bytes f64
//!     3 = Bool   → 1 byte (0=false, 1=true)
//!     4 = Str    → str (compile-time string literal, e.g. global variable name)
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
use crate::module_exports::{IIRExport, IIRImport};

const MAGIC: &[u8; 4] = b"IIR\0";
const VERSION_MAJOR: u8 = 1;
/// LANG33 bumped the minor version from 0 to 1 to add exports/imports sections.
/// The deserialiser accepts both 1.0 (legacy) and 1.1 (current).
const VERSION_MINOR: u8 = 1;

/// Tag byte marking the start of the exports section (LANG33).
const TAG_EXPORTS: u8 = 0x10;
/// Tag byte marking the start of the imports section (LANG33).
const TAG_IMPORTS: u8 = 0x11;

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
    //
    // Security: use `try_into().expect()` rather than `as u32` to turn a
    // silent truncation (stream corruption on strings > 4 GiB) into a loud,
    // early panic.  IIR string fields are compiler-generated and should never
    // approach this limit; if they somehow do, we want a clear failure rather
    // than a quietly malformed binary.
    let len: u32 = encoded.len().try_into()
        .expect("IIR string field exceeds 4 GiB — cannot serialise");
    write_u32_le(buf, len);
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

    // ── LANG33: exports section ──────────────────────────────────────────────
    // Tag 0x10 followed by u32 count and one record per export.
    // Always written (count = 0 for modules with no exports) so the format is
    // symmetric with the imports section and simpler to parse.
    write_u8(&mut buf, TAG_EXPORTS);
    write_u32_le(&mut buf, module.exports.len() as u32);
    for export in &module.exports {
        serialise_export(&mut buf, export);
    }

    // ── LANG33: imports section ──────────────────────────────────────────────
    write_u8(&mut buf, TAG_IMPORTS);
    write_u32_le(&mut buf, module.imports.len() as u32);
    for import in &module.imports {
        serialise_import(&mut buf, import);
    }

    buf
}

fn serialise_export(buf: &mut Vec<u8>, export: &IIRExport) {
    write_str(buf, &export.function_name);
    // Empty string encodes `alias = None` (no alias).
    write_str(buf, export.alias.as_deref().unwrap_or(""));
}

fn serialise_import(buf: &mut Vec<u8>, import: &IIRImport) {
    write_str(buf, &import.module_name);
    write_str(buf, &import.function_name);
    // Empty string encodes `local_alias = None`.
    write_str(buf, import.local_alias.as_deref().unwrap_or(""));
    write_u32_le(buf, import.param_types.len() as u32);
    for pt in &import.param_types {
        write_str(buf, pt);
    }
    write_str(buf, &import.return_type);
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
            // LANG32: compile-time string literal (e.g. global variable name).
            // Kind byte 4 — same length-prefixed string encoding as Var (kind 0).
            Operand::Str(s) => {
                write_u8(buf, 4);
                write_str(buf, s);
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
        // Use checked_add to guard against wrapping on 32-bit targets.
        // Without this, a hostile payload with n ≈ usize::MAX and self.pos > 0
        // would wrap the addition to a small value, pass the bounds check, then
        // panic with a "slice ends before start" error — a clean DoS vector.
        let end = self.pos.checked_add(n).ok_or_else(|| {
            DeserialiseError(format!(
                "offset overflow at pos {} requesting {n} bytes",
                self.pos
            ))
        })?;
        if end > self.data.len() {
            return Err(DeserialiseError(format!(
                "unexpected end of data at offset {} (need {n} bytes, have {})",
                self.pos,
                self.data.len() - self.pos,
            )));
        }
        let chunk = &self.data[self.pos..end];
        self.pos = end;
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
    // Accept 1.0 (legacy — no exports/imports) and 1.1 (LANG33 — with exports/imports).
    // Any other version is rejected.
    if major != VERSION_MAJOR || minor > VERSION_MINOR {
        return Err(DeserialiseError(format!(
            "unsupported version {major}.{minor} (supported: 1.0, 1.1)"
        )));
    }
    let has_lang33 = minor >= 1;

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

    // ── LANG33: read exports and imports (version 1.1+) ─────────────────────
    let (exports, imports) = if has_lang33 {
        let exports = deserialise_exports(&mut r)?;
        let imports = deserialise_imports(&mut r)?;
        (exports, imports)
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(IIRModule {
        name,
        functions,
        entry_point,
        language,
        exports,
        imports,
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

fn deserialise_exports(r: &mut Reader<'_>) -> Result<Vec<IIRExport>, DeserialiseError> {
    let tag = r.u8()?;
    if tag != TAG_EXPORTS {
        return Err(DeserialiseError(format!(
            "expected exports tag 0x{TAG_EXPORTS:02x}, got 0x{tag:02x}"
        )));
    }
    let count = r.u32_le()? as usize;
    let mut exports = Vec::with_capacity(count.min(MAX_SAFE_PREALLOC));
    for _ in 0..count {
        let function_name = r.str_()?;
        let alias_raw = r.str_()?;
        // Empty string encodes `alias = None`.
        let alias = if alias_raw.is_empty() { None } else { Some(alias_raw) };
        exports.push(IIRExport { function_name, alias });
    }
    Ok(exports)
}

fn deserialise_imports(r: &mut Reader<'_>) -> Result<Vec<IIRImport>, DeserialiseError> {
    let tag = r.u8()?;
    if tag != TAG_IMPORTS {
        return Err(DeserialiseError(format!(
            "expected imports tag 0x{TAG_IMPORTS:02x}, got 0x{tag:02x}"
        )));
    }
    let count = r.u32_le()? as usize;
    let mut imports = Vec::with_capacity(count.min(MAX_SAFE_PREALLOC));
    for _ in 0..count {
        let module_name = r.str_()?;
        let function_name = r.str_()?;
        let local_alias_raw = r.str_()?;
        let local_alias = if local_alias_raw.is_empty() { None } else { Some(local_alias_raw) };
        let param_count = r.u32_le()? as usize;
        let mut param_types = Vec::with_capacity(param_count.min(MAX_SAFE_PREALLOC));
        for _ in 0..param_count {
            param_types.push(r.str_()?);
        }
        let return_type = r.str_()?;
        imports.push(IIRImport { module_name, function_name, local_alias, param_types, return_type });
    }
    Ok(imports)
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
            // LANG32: compile-time string literal (kind 4).
            4 => Operand::Str(r.str_()?),
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

    // ── LANG33: exports/imports serialisation ─────────────────────────────────

    #[test]
    fn round_trip_export_no_alias() {
        use crate::module_exports::IIRExport;
        let mut module = make_module();
        module.entry_point = None;
        module.exports.push(IIRExport::new("add"));
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.exports.len(), 1);
        assert_eq!(recovered.exports[0].function_name, "add");
        assert_eq!(recovered.exports[0].alias, None);
        assert_eq!(recovered.exports[0].public_name(), "add");
    }

    #[test]
    fn round_trip_export_with_alias() {
        use crate::module_exports::IIRExport;
        let mut module = make_module();
        module.entry_point = None;
        module.exports.push(IIRExport::new("add").with_alias("public_add"));
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.exports.len(), 1);
        assert_eq!(recovered.exports[0].function_name, "add");
        assert_eq!(recovered.exports[0].alias, Some("public_add".to_string()));
        assert_eq!(recovered.exports[0].public_name(), "public_add");
    }

    #[test]
    fn round_trip_multiple_exports() {
        use crate::module_exports::IIRExport;
        let mut module = IIRModule::new("t", "x");
        module.entry_point = None;
        for name in &["foo", "bar", "baz"] {
            module.add_or_replace(IIRFunction::new(
                *name, vec![], "void",
                vec![IIRInstr::new("ret_void", None, vec![], "void")],
            ));
            module.exports.push(IIRExport::new(*name));
        }
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.exports.len(), 3);
        let names: Vec<&str> = recovered.exports.iter().map(|e| e.public_name()).collect();
        assert_eq!(names, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn round_trip_import_no_alias_no_params() {
        use crate::module_exports::IIRImport;
        let mut module = IIRModule::new("main", "twig");
        module.entry_point = None;
        module.imports.push(IIRImport::new("math", "sqrt", "f64"));
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.imports.len(), 1);
        let imp = &recovered.imports[0];
        assert_eq!(imp.module_name, "math");
        assert_eq!(imp.function_name, "sqrt");
        assert_eq!(imp.local_alias, None);
        assert_eq!(imp.param_types, Vec::<String>::new());
        assert_eq!(imp.return_type, "f64");
        assert_eq!(imp.local_name(), "sqrt");
    }

    #[test]
    fn round_trip_import_with_alias_and_params() {
        use crate::module_exports::IIRImport;
        let mut module = IIRModule::new("main", "twig");
        module.entry_point = None;
        module.imports.push(
            IIRImport::new("math", "add", "i64")
                .with_local_alias("math_add")
                .with_params(vec!["i64".into(), "i64".into()]),
        );
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        let imp = &recovered.imports[0];
        assert_eq!(imp.local_alias, Some("math_add".to_string()));
        assert_eq!(imp.param_types, vec!["i64", "i64"]);
        assert_eq!(imp.return_type, "i64");
        assert_eq!(imp.local_name(), "math_add");
    }

    #[test]
    fn round_trip_module_with_both_exports_and_imports() {
        use crate::module_exports::{IIRExport, IIRImport};
        let mut module = IIRModule::new("app", "twig");
        module.add_or_replace(IIRFunction::new(
            "main", vec![], "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        ));
        module.exports.push(IIRExport::new("main").with_alias("start"));
        module.imports.push(IIRImport::new("io", "print", "void").with_params(vec!["str".into()]));
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.exports.len(), 1);
        assert_eq!(recovered.exports[0].public_name(), "start");
        assert_eq!(recovered.imports.len(), 1);
        assert_eq!(recovered.imports[0].function_name, "print");
    }

    #[test]
    fn round_trip_empty_exports_and_imports() {
        // A module with no exports/imports (the common case for programs).
        let mut module = make_module();
        module.entry_point = None;
        assert!(module.exports.is_empty());
        assert!(module.imports.is_empty());
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        assert!(recovered.exports.is_empty());
        assert!(recovered.imports.is_empty());
    }

    #[test]
    fn str_operand_round_trips() {
        // Operand::Str is used by global_store/global_load (LANG32).
        let mut module = IIRModule::new("t", "x");
        module.entry_point = None;
        let fn_ = IIRFunction::new(
            "f",
            vec![("v".into(), "i64".into())],
            "void",
            vec![IIRInstr::new(
                "global_store",
                None,
                vec![Operand::Str("counter".into()), Operand::Var("v".into())],
                "void",
            )],
        );
        module.add_or_replace(fn_);
        let bytes = serialise(&module);
        let recovered = deserialise(&bytes).unwrap();
        let srcs = &recovered.get_function("f").unwrap().instructions[0].srcs;
        assert_eq!(srcs[0], Operand::Str("counter".into()));
        assert_eq!(srcs[1], Operand::Var("v".into()));
    }

    #[test]
    fn version_10_accepted_with_empty_exports_imports() {
        // Manually craft a version 1.0 header (no exports/imports section).
        // Ensure the deserialiser accepts it and returns empty exports/imports.
        let mut module = IIRModule::new("legacy", "basic");
        module.entry_point = None;
        let mut bytes = serialise(&module);
        // Patch version_minor byte (index 5) from 1 to 0.
        bytes[5] = 0;
        // Strip the exports+imports suffix (the last 10 bytes written by v1.1):
        // TAG_EXPORTS(1) + count(4) + TAG_IMPORTS(1) + count(4) = 10 bytes.
        let new_len = bytes.len() - 10;
        bytes.truncate(new_len);
        let recovered = deserialise(&bytes).unwrap();
        assert_eq!(recovered.name, "legacy");
        assert!(recovered.exports.is_empty());
        assert!(recovered.imports.is_empty());
    }

    #[test]
    fn unsupported_version_returns_error() {
        let mut module = IIRModule::new("t", "x");
        module.entry_point = None;
        let mut bytes = serialise(&module);
        // Patch version_minor to a future unknown version (9).
        bytes[5] = 9;
        let err = deserialise(&bytes).unwrap_err();
        assert!(err.0.contains("unsupported version"));
    }
}
