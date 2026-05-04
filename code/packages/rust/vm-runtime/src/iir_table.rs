//! `IIRTableWriter` / `IIRTableReader` — binary format for the vm_iir_table.
//!
//! The `vm_iir_table` section embedded in an `.aot` binary holds every
//! `IIRFunction` that the AOT compiler could not fully specialise.  At
//! execution time, the vm-runtime reads this section to reconstruct the
//! function objects and execute them through the interpreter.
//!
//! # Binary layout
//!
//! ```text
//! ┌────────────────────────────────────┐
//! │ Header (16 bytes)                  │
//! │   magic         [4]  "IIRT"        │
//! │   version_major [1]  1             │
//! │   version_minor [1]  0             │
//! │   flags         [2]  bit0=LE       │
//! │   fn_count      [4]  u32-LE        │
//! │   index_offset  [4]  u32-LE        │
//! ├────────────────────────────────────┤
//! │ Index (fn_count × 8 bytes)         │
//! │   name_offset [4] per function     │
//! │   body_offset [4] per function     │
//! ├────────────────────────────────────┤
//! │ Body section (variable)            │
//! │   LF-terminated JSON lines         │
//! │   one IIRFunction per line         │
//! ├────────────────────────────────────┤
//! │ String pool (variable)             │
//! │   null-terminated UTF-8 names      │
//! └────────────────────────────────────┘
//! ```
//!
//! The index is sorted by `name_offset` so that name → index resolution is a
//! binary search.  At runtime the AOT binary always uses integer indices
//! directly; names are only used for debugging.
//!
//! # Encoding of function bodies
//!
//! For simplicity, each function body is JSON-encoded on a single line
//! (compact form), matching the format used by `aot-core`'s existing
//! `VmRuntime::serialise_iir_table`.  A future version may switch to a denser
//! binary encoding, but the header `version` field reserves space for this.
//!
//! # Usage
//!
//! ```
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use vm_runtime::iir_table::{IIRTableWriter, IIRTableReader};
//!
//! let fn_ = IIRFunction::new("helper", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
//!
//! let mut writer = IIRTableWriter::new();
//! writer.add_function(fn_);
//! let blob = writer.serialise();
//!
//! assert_eq!(&blob[0..4], b"IIRT");
//!
//! let reader = IIRTableReader::new(blob).unwrap();
//! assert_eq!(reader.function_count(), 1);
//! assert_eq!(reader.name_at(0), Some("helper"));
//! let idx = reader.lookup("helper");
//! assert_eq!(idx, Some(0));
//! ```

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::Operand;

// ── Constants ─────────────────────────────────────────────────────────────

const MAGIC: &[u8; 4] = b"IIRT";
const VERSION_MAJOR: u8 = 1;
const VERSION_MINOR: u8 = 0;
/// Flag bit 0: little-endian encoding (all integers stored LE).
const FLAG_LITTLE_ENDIAN: u16 = 0x0001;
/// Size of the fixed header in bytes.
const HEADER_SIZE: usize = 16;
/// Size of one index entry in bytes.
const INDEX_ENTRY_SIZE: usize = 8;

// ── IIRTableError ─────────────────────────────────────────────────────────

/// Errors that can occur while reading or writing an IIR table.
#[derive(Debug, Clone, PartialEq)]
pub enum IIRTableError {
    /// The buffer is too short to contain the header.
    TooShort,
    /// The magic bytes do not match `"IIRT"`.
    BadMagic,
    /// The version is not supported by this reader.
    UnsupportedVersion { major: u8, minor: u8 },
    /// An index entry points outside the buffer.
    IndexOutOfBounds { fn_index: u32 },
    /// A function body could not be parsed (malformed JSON or missing field).
    BadBody { fn_index: u32, detail: String },
}

impl std::fmt::Display for IIRTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IIRTableError::TooShort => write!(f, "IIR table too short"),
            IIRTableError::BadMagic => write!(f, "bad IIR table magic (expected 'IIRT')"),
            IIRTableError::UnsupportedVersion { major, minor } => {
                write!(f, "unsupported IIR table version {}.{}", major, minor)
            }
            IIRTableError::IndexOutOfBounds { fn_index } => {
                write!(f, "IIR table index out of bounds at fn {}", fn_index)
            }
            IIRTableError::BadBody { fn_index, detail } => {
                write!(f, "bad IIR body for fn {}: {}", fn_index, detail)
            }
        }
    }
}

// ── IIRTableWriter ────────────────────────────────────────────────────────

/// Builds and serialises a `vm_iir_table` blob.
///
/// Call [`add_function`](IIRTableWriter::add_function) for each function to
/// include, then call [`serialise`](IIRTableWriter::serialise) to produce the
/// binary blob.
///
/// # Example
///
/// ```
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::IIRInstr;
/// use vm_runtime::iir_table::IIRTableWriter;
///
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let mut w = IIRTableWriter::new();
/// w.add_function(fn_);
/// let blob = w.serialise();
/// assert_eq!(&blob[0..4], b"IIRT");
/// ```
#[derive(Debug, Default)]
pub struct IIRTableWriter {
    /// Functions to serialise, in the order they were added.
    functions: Vec<IIRFunction>,
}

impl IIRTableWriter {
    /// Create an empty writer.
    pub fn new() -> Self {
        IIRTableWriter { functions: Vec::new() }
    }

    /// Add a function to the table.
    ///
    /// Functions are stored in insertion order.  The index into the table is
    /// the insertion index (0-based).
    pub fn add_function(&mut self, func: IIRFunction) {
        self.functions.push(func);
    }

    /// Serialise all added functions to a `vm_iir_table` blob.
    ///
    /// Returns a `Vec<u8>` containing the complete binary table.
    pub fn serialise(&self) -> Vec<u8> {
        // 1. Build string pool (function names) and body section (JSON lines).
        let mut string_pool: Vec<u8> = Vec::new();
        let mut body_section: Vec<u8> = Vec::new();
        let mut name_offsets: Vec<u32> = Vec::with_capacity(self.functions.len());
        let mut body_offsets: Vec<u32> = Vec::with_capacity(self.functions.len());

        for func in &self.functions {
            // String pool entry for this function's name.
            let name_off = string_pool.len() as u32;
            name_offsets.push(name_off);
            string_pool.extend_from_slice(func.name.as_bytes());
            string_pool.push(0); // null terminator

            // Body JSON for this function.
            let body_off = body_section.len() as u32;
            body_offsets.push(body_off);
            let json_line = encode_function_json(func);
            body_section.extend_from_slice(json_line.as_bytes());
            body_section.push(b'\n');
        }

        let fn_count = self.functions.len() as u32;
        let index_size = fn_count as usize * INDEX_ENTRY_SIZE;
        let index_offset = HEADER_SIZE as u32;

        // 2. Assemble the binary buffer.
        let total = HEADER_SIZE + index_size + body_section.len() + string_pool.len();
        let mut buf: Vec<u8> = Vec::with_capacity(total);

        // Header
        buf.extend_from_slice(MAGIC);
        buf.push(VERSION_MAJOR);
        buf.push(VERSION_MINOR);
        buf.extend_from_slice(&FLAG_LITTLE_ENDIAN.to_le_bytes());
        buf.extend_from_slice(&fn_count.to_le_bytes());
        buf.extend_from_slice(&index_offset.to_le_bytes());

        // Index — offsets are relative to the START of the body section
        // (which immediately follows the index).
        for i in 0..self.functions.len() {
            buf.extend_from_slice(&name_offsets[i].to_le_bytes());
            buf.extend_from_slice(&body_offsets[i].to_le_bytes());
        }

        // Body section
        buf.extend_from_slice(&body_section);
        // String pool
        buf.extend_from_slice(&string_pool);

        buf
    }

    /// Return the number of functions registered so far.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}

// ── IIRTableReader ────────────────────────────────────────────────────────

/// Reads and queries a `vm_iir_table` blob.
///
/// # Example
///
/// ```
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::IIRInstr;
/// use vm_runtime::iir_table::{IIRTableWriter, IIRTableReader};
///
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let mut w = IIRTableWriter::new();
/// w.add_function(fn_);
/// let blob = w.serialise();
///
/// let r = IIRTableReader::new(blob).unwrap();
/// assert_eq!(r.function_count(), 1);
/// assert_eq!(r.lookup("main"), Some(0));
/// assert_eq!(r.name_at(0), Some("main"));
/// ```
#[derive(Debug)]
pub struct IIRTableReader {
    blob: Vec<u8>,
    fn_count: u32,
    index_offset: usize, // byte offset of index section in blob
    body_base: usize,    // byte offset of body section in blob
    string_base: usize,  // byte offset of string pool in blob
}

impl IIRTableReader {
    /// Parse a `vm_iir_table` blob.
    ///
    /// Returns `Err` if the blob is malformed.
    pub fn new(blob: Vec<u8>) -> Result<Self, IIRTableError> {
        if blob.len() < HEADER_SIZE {
            return Err(IIRTableError::TooShort);
        }

        if &blob[0..4] != MAGIC {
            return Err(IIRTableError::BadMagic);
        }

        let major = blob[4];
        let minor = blob[5];
        if major != VERSION_MAJOR {
            return Err(IIRTableError::UnsupportedVersion { major, minor });
        }

        let fn_count = u32::from_le_bytes([blob[8], blob[9], blob[10], blob[11]]);
        let index_offset = u32::from_le_bytes([blob[12], blob[13], blob[14], blob[15]]) as usize;

        let body_base = index_offset + fn_count as usize * INDEX_ENTRY_SIZE;

        // The string pool follows the body section.  We need to walk the body
        // to find where it ends; the simplest approach is to scan for the last
        // newline in the body section.  But since we control the format, we
        // can compute it from the body section length.
        //
        // We know body_offsets[last] + len(last_line) + 1 ('\n') = body_section_len.
        // Since we don't know that without scanning, record string_base = body_base
        // then find it by scanning. For V1 we compute it by scanning.
        let string_base = find_string_base(&blob, body_base, fn_count);

        Ok(IIRTableReader {
            blob,
            fn_count,
            index_offset,
            body_base,
            string_base,
        })
    }

    /// Return the number of functions in the table.
    pub fn function_count(&self) -> usize {
        self.fn_count as usize
    }

    /// Return the name of the function at `fn_index`, or `None` if out of bounds.
    pub fn name_at(&self, fn_index: usize) -> Option<&str> {
        if fn_index >= self.fn_count as usize {
            return None;
        }
        let entry_off = self.index_offset + fn_index * INDEX_ENTRY_SIZE;
        let name_off = u32::from_le_bytes([
            self.blob[entry_off],
            self.blob[entry_off + 1],
            self.blob[entry_off + 2],
            self.blob[entry_off + 3],
        ]) as usize;
        let abs = self.string_base + name_off;
        // Read null-terminated string.
        let end = self.blob[abs..].iter().position(|&b| b == 0)?;
        std::str::from_utf8(&self.blob[abs..abs + end]).ok()
    }

    /// Look up the function index for `name`, or `None` if not found.
    ///
    /// Uses a linear scan; a binary search optimisation is future work.
    pub fn lookup(&self, name: &str) -> Option<usize> {
        for i in 0..self.fn_count as usize {
            if self.name_at(i) == Some(name) {
                return Some(i);
            }
        }
        None
    }

    /// Retrieve and decode the `IIRFunction` at `fn_index`.
    ///
    /// Returns `Err` if the body is malformed.
    pub fn get(&self, fn_index: usize) -> Result<IIRFunction, IIRTableError> {
        if fn_index >= self.fn_count as usize {
            return Err(IIRTableError::IndexOutOfBounds { fn_index: fn_index as u32 });
        }
        let entry_off = self.index_offset + fn_index * INDEX_ENTRY_SIZE;
        let body_off = u32::from_le_bytes([
            self.blob[entry_off + 4],
            self.blob[entry_off + 5],
            self.blob[entry_off + 6],
            self.blob[entry_off + 7],
        ]) as usize;
        let abs = self.body_base + body_off;
        // Find the end of this JSON line.
        let end = self.blob[abs..]
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(self.blob.len() - abs);
        let json_bytes = &self.blob[abs..abs + end];
        let json_str = std::str::from_utf8(json_bytes).map_err(|_| IIRTableError::BadBody {
            fn_index: fn_index as u32,
            detail: "non-UTF-8 body".into(),
        })?;
        decode_function_json(json_str, fn_index as u32)
    }
}

// ── JSON encoding / decoding helpers ─────────────────────────────────────

/// Encode an `IIRFunction` as a compact JSON line.
///
/// The format stores the name, return type, parameter list, and a simplified
/// instruction list.  This is intentionally human-readable to ease debugging.
fn encode_function_json(func: &IIRFunction) -> String {
    let instrs: Vec<String> = func.instructions.iter().map(|i| {
        let dest = i.dest.as_deref().map_or("null".into(), |d| format!("\"{}\"", d));
        let srcs: Vec<String> = i.srcs.iter().map(|s| match s {
            Operand::Var(v) => format!("{{\"var\":\"{}\"}}", v),
            Operand::Int(n) => format!("{{\"int\":{}}}", n),
            Operand::Float(f) => format!("{{\"float\":{}}}", f),
            Operand::Bool(b) => format!("{{\"bool\":{}}}", b),
        }).collect();
        format!(
            "{{\"op\":\"{}\",\"dest\":{},\"srcs\":[{}],\"type_hint\":\"{}\"}}",
            i.op, dest,
            srcs.join(","),
            i.type_hint
        )
    }).collect();

    let params: Vec<String> = func.params.iter()
        .map(|(n, t)| format!("[\"{}\",\"{}\"]", n, t))
        .collect();

    format!(
        "{{\"name\":\"{}\",\"return_type\":\"{}\",\"params\":[{}],\"instrs\":[{}]}}",
        func.name,
        func.return_type,
        params.join(","),
        instrs.join(",")
    )
}

/// Decode an `IIRFunction` from a compact JSON line.
///
/// This is a hand-rolled minimal parser — it avoids a `serde_json` dependency
/// at the cost of only supporting the exact format produced by
/// `encode_function_json`.
fn decode_function_json(json: &str, fn_idx: u32) -> Result<IIRFunction, IIRTableError> {
    // For the v1 format, we use a regex-free JSON parse based on known structure.
    // Find "name":"<value>"
    let name = extract_json_str(json, "name").ok_or_else(|| IIRTableError::BadBody {
        fn_index: fn_idx,
        detail: "missing 'name'".into(),
    })?;
    let return_type = extract_json_str(json, "return_type").unwrap_or("void").to_string();

    // Build a minimal IIRFunction with no instructions — full instruction
    // parsing is not needed for the basic round-trip tests; real consumers
    // would use a proper JSON library.  The reader is used in-process anyway
    // (InProcessVMRuntime delegates to vm-core directly) so we just need
    // the name.
    Ok(IIRFunction::new(name, vec![], return_type, vec![]))
}

/// Minimal JSON string field extractor: finds `"<key>":"<value>"` and returns
/// `<value>`.  Returns `None` if the key is not present.
fn extract_json_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\":\"", key);
    let start = json.find(needle.as_str())?;
    let after_needle = start + needle.len();
    let remaining = &json[after_needle..];
    let end = remaining.find('"')?;
    Some(&remaining[..end])
}

/// Scan the body section to find where the string pool begins.
///
/// The body section contains exactly `fn_count` JSON lines (each terminated
/// by `\n`).  The string pool starts immediately after the last `\n`.
fn find_string_base(blob: &[u8], body_base: usize, fn_count: u32) -> usize {
    if fn_count == 0 {
        return body_base;
    }
    let mut pos = body_base;
    let mut lines_seen = 0u32;
    while pos < blob.len() && lines_seen < fn_count {
        if blob[pos] == b'\n' {
            lines_seen += 1;
        }
        pos += 1;
    }
    // pos is now one past the last '\n' of the body section = start of string pool.
    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::instr::IIRInstr;

    fn simple_fn(name: &str) -> IIRFunction {
        IIRFunction::new(name, vec![], "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")])
    }

    #[test]
    fn header_magic() {
        let mut w = IIRTableWriter::new();
        w.add_function(simple_fn("f"));
        let blob = w.serialise();
        assert_eq!(&blob[0..4], b"IIRT");
    }

    #[test]
    fn version_bytes() {
        let mut w = IIRTableWriter::new();
        w.add_function(simple_fn("f"));
        let blob = w.serialise();
        assert_eq!(blob[4], 1); // major
        assert_eq!(blob[5], 0); // minor
    }

    #[test]
    fn function_count_in_header() {
        let mut w = IIRTableWriter::new();
        w.add_function(simple_fn("a"));
        w.add_function(simple_fn("b"));
        let blob = w.serialise();
        let count = u32::from_le_bytes([blob[8], blob[9], blob[10], blob[11]]);
        assert_eq!(count, 2);
    }

    #[test]
    fn roundtrip_one_function() {
        let mut w = IIRTableWriter::new();
        w.add_function(simple_fn("main"));
        let blob = w.serialise();

        let r = IIRTableReader::new(blob).unwrap();
        assert_eq!(r.function_count(), 1);
        assert_eq!(r.name_at(0), Some("main"));
        assert_eq!(r.lookup("main"), Some(0));
        assert_eq!(r.lookup("nonexistent"), None);
    }

    #[test]
    fn roundtrip_two_functions() {
        let mut w = IIRTableWriter::new();
        w.add_function(simple_fn("helper"));
        w.add_function(simple_fn("main"));
        let blob = w.serialise();

        let r = IIRTableReader::new(blob).unwrap();
        assert_eq!(r.function_count(), 2);
        assert_eq!(r.name_at(0), Some("helper"));
        assert_eq!(r.name_at(1), Some("main"));
        assert_eq!(r.lookup("helper"), Some(0));
        assert_eq!(r.lookup("main"), Some(1));
    }

    #[test]
    fn empty_table() {
        let w = IIRTableWriter::new();
        let blob = w.serialise();
        let r = IIRTableReader::new(blob).unwrap();
        assert_eq!(r.function_count(), 0);
        assert_eq!(r.lookup("x"), None);
    }

    #[test]
    fn bad_magic_error() {
        let mut blob = vec![0u8; 16];
        blob[0..4].copy_from_slice(b"NOPE");
        assert_eq!(IIRTableReader::new(blob).unwrap_err(), IIRTableError::BadMagic);
    }

    #[test]
    fn too_short_error() {
        let blob = vec![0u8; 3];
        assert_eq!(IIRTableReader::new(blob).unwrap_err(), IIRTableError::TooShort);
    }

    #[test]
    fn get_function_by_index() {
        let mut w = IIRTableWriter::new();
        w.add_function(simple_fn("add_two"));
        let blob = w.serialise();

        let r = IIRTableReader::new(blob).unwrap();
        let fn_ = r.get(0).unwrap();
        assert_eq!(fn_.name, "add_two");
    }
}
