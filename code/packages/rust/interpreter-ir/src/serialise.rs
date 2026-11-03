//! Binary serialisation for [`IIRModule`].
//!
//! # Wire format (little-endian throughout)
//!
//! ## Version 1.3 (LANG83 PR5b — raw-byte string literals)
//!
//! Version 1.2 plus one operand kind, written **only** when some instruction
//! carries an [`Operand::Bytes`] literal:
//!
//! ```text
//!     5 = Bytes  → 4 bytes u32 length, then that many raw bytes
//! ```
//!
//! A `Bytes` literal is a byte string that need not be UTF-8 (Perl's
//! `"\xff"`), so it cannot ride in kind 4, whose payload the reader decodes
//! as UTF-8. In 1.3 the string-ceiling section is **optional**: after the
//! imports the stream either ends (`str_max_bytes = None`) or carries tag
//! `0x12` and the `u32` ceiling and then ends. A 1.3 stream with anything
//! after that is an error, and so is kind 5 in a 1.0, 1.1 or 1.2 stream. The
//! full table of what the writer picks:
//!
//! | module                                   | version written | trailing sections                 |
//! |------------------------------------------|-----------------|-----------------------------------|
//! | no `Bytes`, `str_max_bytes = None`       | 1.1             | exports, imports                  |
//! | no `Bytes`, `str_max_bytes = Some(n)`    | 1.2             | exports, imports, ceiling `n`     |
//! | some `Bytes`, `str_max_bytes = None`     | 1.3             | exports, imports                  |
//! | some `Bytes`, `str_max_bytes = Some(n)`  | 1.3             | exports, imports, ceiling `n`     |
//!
//! So a module without a raw-byte literal is written exactly as before, and
//! no digest of an existing module moves.
//!
//! ## Version 1.2 (LANG83 PR5a follow-up, #18800 — a lowered string ceiling)
//!
//! Version 1.1 followed by one more section, written **only** when the
//! module sets `str_max_bytes`:
//!
//! ```text
//!     1 byte   tag         0x12 = string-ceiling section
//!     4 bytes  str_max_bytes  (u32)
//! ```
//!
//! A module whose `str_max_bytes` is `None` (the registry ceiling — every
//! module a frontend emits today) is written as **1.1, byte for byte as
//! before**. That keeps every existing serialised module, and every digest of
//! one (LANG84's `release_classes.csv` hashes serialised IIR), unchanged. The
//! writer picks the oldest version that can say what the module says:
//!
//! | `str_max_bytes` | version written | trailing sections            |
//! |-----------------|-----------------|------------------------------|
//! | `None`          | 1.1             | exports, imports             |
//! | `Some(n)`       | 1.2             | exports, imports, ceiling `n` |
//!
//! Why the ceiling must be on the wire at all: it is part of the program's
//! meaning. A module compiled under a ceiling of 16 traps `Trap.Overflow` on
//! a 17-byte `str_concat`; dropping the field on a round trip (as 1.1 did)
//! silently raised it to `STR_MAX_BYTES`.
//!
//! ## Version 1.1 (LANG33)
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
//!     5 = Bytes  → u32 length + raw bytes (version 1.3 only; LANG83 PR5b)
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
/// The version written for a module with a lowered string ceiling (#18800).
/// The deserialiser accepts 1.0, 1.1 and 1.2.
const VERSION_MINOR_STR_CEILING: u8 = 2;

/// The version written for a module with a raw-byte literal (LANG83 PR5b).
/// The deserialiser accepts 1.0 to 1.3.
const VERSION_MINOR_BYTES: u8 = 3;

/// Operand kind byte of a raw-byte literal (version 1.3).
const KIND_BYTES: u8 = 5;

/// Tag byte marking the start of the exports section (LANG33).
const TAG_EXPORTS: u8 = 0x10;
/// Tag byte marking the start of the imports section (LANG33).
const TAG_IMPORTS: u8 = 0x11;
/// Tag byte marking the string-ceiling section (LANG83 PR5a, version 1.2).
const TAG_STR_CEILING: u8 = 0x12;

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
    write_bytes(buf, s.as_bytes());
}

/// A `u32` length prefix and the bytes: the encoding of every `str` field, and
/// of a raw-byte literal (kind 5).
fn write_bytes(buf: &mut Vec<u8>, encoded: &[u8]) {
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
    // The oldest version that can say what the module says: 1.3 when a
    // raw-byte literal needs operand kind 5, else 1.2 when there is a ceiling
    // to record, otherwise exactly 1.1.
    let has_bytes = module
        .functions
        .iter()
        .flat_map(|f| f.instructions.iter())
        .flat_map(|i| i.srcs.iter())
        .any(|op| matches!(op, Operand::Bytes(_)));
    let minor = if has_bytes {
        VERSION_MINOR_BYTES
    } else if module.str_max_bytes.is_some() {
        VERSION_MINOR_STR_CEILING
    } else {
        VERSION_MINOR
    };
    write_u8(&mut buf, minor);
    write_u32_le(&mut buf, module.functions.len().try_into()
        .expect("function count exceeds u32::MAX — cannot serialise"));
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
    write_u32_le(&mut buf, module.exports.len().try_into()
        .expect("exports count exceeds u32::MAX — cannot serialise"));
    for export in &module.exports {
        serialise_export(&mut buf, export);
    }

    // ── LANG33: imports section ──────────────────────────────────────────────
    write_u8(&mut buf, TAG_IMPORTS);
    write_u32_le(&mut buf, module.imports.len().try_into()
        .expect("imports count exceeds u32::MAX — cannot serialise"));
    for import in &module.imports {
        serialise_import(&mut buf, import);
    }

    // ── LANG83 PR5a: string-ceiling section (version 1.2, and 1.3 when set) ─
    // The raw field, not `str_ceiling()`: a malformed value above
    // `STR_MAX_BYTES` round-trips as itself, so the reader's `validate` (and
    // every column's ceiling check) still sees and refuses it.
    if let Some(max) = module.str_max_bytes {
        write_u8(&mut buf, TAG_STR_CEILING);
        write_u32_le(&mut buf, max);
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
    write_u32_le(buf, import.param_types.len().try_into()
        .expect("param_types count exceeds u32::MAX — cannot serialise"));
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
    write_u32_le(buf, fn_.params.len().try_into()
        .expect("param count exceeds u32::MAX — cannot serialise"));
    for (param_name, param_type) in &fn_.params {
        write_str(buf, param_name);
        write_str(buf, param_type);
    }
    write_u32_le(buf, fn_.instructions.len().try_into()
        .expect("instruction count exceeds u32::MAX — cannot serialise"));
    // register_count: use try_into for the same reason — large register files
    // should fail loudly rather than silently truncate.
    write_u32_le(buf, fn_.register_count.try_into()
        .expect("register_count exceeds u32::MAX — cannot serialise"));
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
    // Use u32 for src_count with try_into to catch truncation on overflow.
    write_u32_le(buf, instr.srcs.len().try_into()
        .expect("src count exceeds u32::MAX — cannot serialise"));
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
            // LANG83 PR5b: a raw-byte literal (kind 5, version 1.3). The same
            // u32 length prefix as a `str`, but the payload is raw bytes the
            // reader does not decode.
            Operand::Bytes(b) => {
                write_u8(buf, KIND_BYTES);
                write_bytes(buf, b);
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

    /// A `u32`-length-prefixed run of raw bytes (a kind-5 literal's payload).
    fn bytes_(&mut self) -> Result<Vec<u8>, DeserialiseError> {
        let len = self.u32_le()? as usize;
        Ok(self.read_exact(len)?.to_vec())
    }

    /// `true` once every byte has been read.
    fn at_end(&self) -> bool {
        self.pos >= self.data.len()
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
    // Accept 1.0 (legacy — no exports/imports), 1.1 (LANG33 — with
    // exports/imports), 1.2 (1.1 plus the string ceiling, #18800) and 1.3
    // (raw-byte literals, an optional ceiling; LANG83 PR5b).
    // Any other version is rejected.
    if major != VERSION_MAJOR || minor > VERSION_MINOR_BYTES {
        return Err(DeserialiseError(format!(
            "unsupported version {major}.{minor} (supported: 1.0, 1.1, 1.2, 1.3)"
        )));
    }
    let has_lang33 = minor >= 1;
    let has_bytes = minor >= VERSION_MINOR_BYTES;

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
        functions.push(deserialise_function(&mut r, has_bytes)?);
    }

    // ── LANG33: read exports and imports (version 1.1+) ─────────────────────
    let (exports, imports) = if has_lang33 {
        let exports = deserialise_exports(&mut r)?;
        let imports = deserialise_imports(&mut r)?;
        (exports, imports)
    } else {
        (Vec::new(), Vec::new())
    };

    // ── LANG83 PR5a: the string ceiling (version 1.2; optional in 1.3) ──────
    // A 1.2 stream *must* carry the section: a truncated one is an error, not
    // a module silently back on the registry ceiling. A 1.3 stream carries it
    // only for a module that sets one, so there the section is present iff
    // bytes remain, and nothing may follow it (PR5b).
    let read_ceiling = |r: &mut Reader<'_>| -> Result<u32, DeserialiseError> {
        let tag = r.u8()?;
        if tag != TAG_STR_CEILING {
            return Err(DeserialiseError(format!(
                "expected string-ceiling tag 0x{TAG_STR_CEILING:02x}, got 0x{tag:02x}"
            )));
        }
        r.u32_le()
    };
    let str_max_bytes = match minor {
        VERSION_MINOR_STR_CEILING => Some(read_ceiling(&mut r)?),
        VERSION_MINOR_BYTES => {
            let max = if r.at_end() { None } else { Some(read_ceiling(&mut r)?) };
            if !r.at_end() {
                return Err(DeserialiseError(format!(
                    "{} trailing byte(s) after the end of a version 1.3 module",
                    r.data.len() - r.pos
                )));
            }
            max
        }
        _ => None,
    };

    Ok(IIRModule {
        name,
        functions,
        entry_point,
        language,
        exports,
        imports,
        // The release ID is not on the wire (see `IIRModule::release`).
        release: None,
        // The string ceiling is, from 1.2 (#18800); before, `None` = the
        // registry's.
        str_max_bytes,
    })
}

/// `has_bytes`: the stream is version 1.3, so operand kind 5 is legal.
fn deserialise_function(r: &mut Reader<'_>, has_bytes: bool) -> Result<IIRFunction, DeserialiseError> {
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
        instructions.push(deserialise_instr(r, has_bytes)?);
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

fn deserialise_instr(r: &mut Reader<'_>, has_bytes: bool) -> Result<IIRInstr, DeserialiseError> {
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
            // LANG83 PR5b: a raw-byte literal (kind 5) — version 1.3 only, so
            // an older stream that has one is corrupt, not silently upgraded.
            KIND_BYTES if has_bytes => Operand::Bytes(r.bytes_()?),
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
    fn release_is_not_on_the_wire() {
        // `IIRModule::release` is in-memory provenance only (LANG84 C4.1);
        // the 1.1 format is unchanged, so a round trip yields `None`.
        let mut m = make_module();
        let without = serialise(&m);
        m.release = Some("5.38.0".into());
        let with = serialise(&m);
        assert_eq!(with, without);
        assert_eq!(deserialise(&with).unwrap().release, None);
    }

    /// #18800 item 2: the string ceiling round-trips. Before, `deserialise`
    /// always returned `None`, so a module compiled under a ceiling of 16 came
    /// back under the registry's 2³⁰ − 64. A malformed value above
    /// `STR_MAX_BYTES` round-trips as itself, so the reader still refuses it.
    #[test]
    fn str_max_bytes_round_trips_as_version_1_2() {
        for max in [0, 16, crate::STR_MAX_BYTES, crate::STR_MAX_BYTES + 1, u32::MAX] {
            let mut m = make_module();
            m.str_max_bytes = Some(max);
            let bytes = serialise(&m);
            assert_eq!((bytes[4], bytes[5]), (1, 2), "a ceiling is written as 1.2");
            // The section is the last thing written: tag 0x12, then the u32.
            let tail = &bytes[bytes.len() - 5..];
            assert_eq!(tail[0], TAG_STR_CEILING);
            assert_eq!(u32::from_le_bytes(tail[1..].try_into().unwrap()), max);
            let back = deserialise(&bytes).unwrap();
            assert_eq!(back.str_max_bytes, Some(max));
            assert_eq!(back.str_ceiling(), m.str_ceiling());
        }
    }

    /// Without a ceiling the bytes are exactly the 1.1 format, so every
    /// existing serialised module, and every digest of one (LANG84's
    /// `release_classes.csv`), is unchanged.
    #[test]
    fn no_ceiling_is_written_byte_for_byte_as_version_1_1() {
        let m = make_module();
        let bytes = serialise(&m);
        assert_eq!((bytes[4], bytes[5]), (1, 1));
        // Ends with the two LANG33 sections, both empty: no ceiling section.
        assert_eq!(&bytes[bytes.len() - 10..], &[TAG_EXPORTS, 0, 0, 0, 0, TAG_IMPORTS, 0, 0, 0, 0]);
        assert_eq!(deserialise(&bytes).unwrap().str_max_bytes, None);
        // The ceiling adds exactly its section and the version byte.
        let mut with = m.clone();
        with.str_max_bytes = Some(16);
        let with = serialise(&with);
        assert_eq!(with.len(), bytes.len() + 5);
        assert_eq!(&with[6..bytes.len()], &bytes[6..]);
    }

    /// A 1.2 stream must carry the section: one cut short, or with another
    /// tag where the section belongs, is an error — never a module quietly
    /// back on the registry ceiling.
    #[test]
    fn a_version_1_2_stream_without_its_ceiling_is_an_error() {
        let mut m = make_module();
        m.str_max_bytes = Some(16);
        let bytes = serialise(&m);
        let cut = &bytes[..bytes.len() - 5];
        assert!(deserialise(cut).unwrap_err().0.contains("unexpected end of data"));
        let mut wrong_tag = bytes.clone();
        let at = wrong_tag.len() - 5;
        wrong_tag[at] = 0x13;
        assert!(deserialise(&wrong_tag).unwrap_err().0.contains("string-ceiling tag"));
        let short = &bytes[..bytes.len() - 2];
        assert!(deserialise(short).is_err());
        // A 1.1 header on the same bytes reads the module without a ceiling.
        let mut as_1_1 = cut.to_vec();
        as_1_1[5] = 1;
        assert_eq!(deserialise(&as_1_1).unwrap().str_max_bytes, None);
    }

    /// `make_module` plus a function whose `str_const` carries the raw bytes
    /// `00 0A 80 FF`, which no `Str` (kind 4, read back as UTF-8) can hold.
    fn module_with_raw_bytes() -> IIRModule {
        let mut m = make_module();
        m.add_or_replace(IIRFunction::new("bytes", vec![], "str", vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Bytes(vec![0x00, 0x0A, 0x80, 0xFF])], "str"),
            IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "str"),
        ]));
        m
    }

    /// LANG83 PR5b: a raw-byte literal round-trips as version 1.3, byte for
    /// byte, with and without a ceiling. Before 1.3 there was no way to write
    /// one: kind 4 is UTF-8 on the wire.
    #[test]
    fn a_raw_byte_literal_round_trips_as_version_1_3() {
        for max in [None, Some(16), Some(u32::MAX)] {
            let mut m = module_with_raw_bytes();
            m.str_max_bytes = max;
            let bytes = serialise(&m);
            assert_eq!((bytes[4], bytes[5]), (1, 3), "a raw-byte literal is written as 1.3");
            let back = deserialise(&bytes).unwrap();
            assert_eq!(back.str_max_bytes, max);
            let instr = &back.get_function("bytes").unwrap().instructions[0];
            assert_eq!(instr.srcs, vec![Operand::Bytes(vec![0x00, 0x0A, 0x80, 0xFF])]);
            assert_eq!(serialise(&back), bytes, "a second round trip is byte-identical");
        }
        // The empty byte string is a literal too.
        let mut m = module_with_raw_bytes();
        m.get_function_mut("bytes").unwrap().instructions[0].srcs[0] = Operand::Bytes(vec![]);
        let back = deserialise(&serialise(&m)).unwrap();
        assert_eq!(back.get_function("bytes").unwrap().instructions[0].srcs[0], Operand::Bytes(vec![]));
    }

    /// A module without a raw-byte literal is still written as 1.1 or 1.2
    /// exactly as before, so no existing digest moves.
    #[test]
    fn version_1_3_is_written_only_for_a_raw_byte_literal() {
        let mut m = make_module();
        m.add_or_replace(IIRFunction::new("s", vec![], "void", vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("é".into())], "str"),
        ]));
        assert_eq!(serialise(&m)[5], 1);
        m.str_max_bytes = Some(16);
        assert_eq!(serialise(&m)[5], 2);
    }

    /// Kind 5 is a 1.3 operand: in a 1.1 or 1.2 stream it is corruption, and a
    /// 1.3 stream may not carry anything after its (optional) ceiling.
    #[test]
    fn a_raw_byte_operand_needs_version_1_3_and_nothing_trails_it() {
        for max in [None, Some(16)] {
            let mut m = module_with_raw_bytes();
            m.str_max_bytes = max;
            let bytes = serialise(&m);
            for older in [0u8, 1, 2] {
                let mut patched = bytes.clone();
                patched[5] = older;
                let err = deserialise(&patched).unwrap_err().0;
                assert!(err.contains("unknown operand kind byte: 5"), "1.{older}: {err}");
            }
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(deserialise(&trailing).is_err(), "a trailing byte after a 1.3 module");
        }
        // Where the optional ceiling would be, only its tag is accepted.
        let mut wrong = serialise(&module_with_raw_bytes());
        wrong.extend_from_slice(&[0x13, 16, 0, 0, 0]);
        assert!(deserialise(&wrong).unwrap_err().0.contains("string-ceiling tag"));
        // A cut-short raw-byte payload is an error, not a shorter literal.
        let bytes = serialise(&module_with_raw_bytes());
        let at = bytes.windows(4).position(|w| w == [0x00, 0x0A, 0x80, 0xFF]).unwrap();
        assert!(deserialise(&bytes[..at + 2]).is_err());
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

    // `3.14` is an arbitrary float payload for round-tripping, not PI.
    #[allow(clippy::approx_constant)]
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
        // 1.4 is not known either (1.3 is, since LANG83 PR5b).
        bytes[5] = 4;
        assert!(deserialise(&bytes).unwrap_err().0.contains("supported: 1.0, 1.1, 1.2, 1.3"));
    }
}
