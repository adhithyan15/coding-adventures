//! BEAM bytecode encoder — build and serialize Erlang `.beam` files.
//!
//! # The BEAM file format
//!
//! A `.beam` file is an IFF (Interchange File Format) container:
//!
//! ```text
//! "FOR1"              4 bytes  magic
//! <u32 BE>            4 bytes  total size - 8 (everything after this field)
//! "BEAM"              4 bytes  form type
//! [chunks …]
//! ```
//!
//! Each chunk has the layout:
//!
//! ```text
//! <4 bytes ASCII tag>
//! <u32 BE>  payload size (not counting header)
//! <payload bytes>
//! <0-3 zero bytes padding to 4-byte alignment>
//! ```
//!
//! # Compact-term encoding
//!
//! BEAM operands use a variable-width encoding.  The bottom 3 bits are the
//! **type tag**; the upper bits carry the value.  Three forms:
//!
//! ```text
//! Small  (value < 16):   [ val:4 | tag:3 | 0 ]             1 byte
//! Medium (16 ≤ v < 2048):[ hi:3 | 1 | tag:3 | 1 ] [ lo:8 ] 2 bytes
//! Large  (v ≥ 2048):     [ (len-2):3 | 11 | tag:3 ] [big-endian bytes …]
//! ```
//!
//! Tags are defined by the `BEAMTag` enum below.

// ===========================================================================
// BEAMTag
// ===========================================================================

/// 3-bit type tags for compact-term-encoded operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BEAMTag {
    /// Unsigned literal (u).
    U = 0,
    /// Signed integer literal (i).
    I = 1,
    /// Atom-table index (a).
    A = 2,
    /// X register — function argument / scratch.
    X = 3,
    /// Y register — stack-local (callee-saved; not used in v1 lowering).
    Y = 4,
    /// Label / function reference (f).
    F = 5,
    // H = 6 — legacy character; never emitted
    // Z = 7 — extended (list, fpreg, alloc-list, lit-table); not used in v1
}

// ===========================================================================
// BEAMOperand, BEAMInstruction
// ===========================================================================

/// A single BEAM instruction operand: tag + unsigned value.
///
/// The sign-bit dance for signed integers is handled when building
/// instructions, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BEAMOperand {
    /// Type tag (bottom 3 bits in the encoded form).
    pub tag: BEAMTag,
    /// Non-negative encoded value.
    pub value: u64,
}

impl BEAMOperand {
    /// Construct an unsigned-literal operand.
    pub fn u(value: u64) -> Self { Self { tag: BEAMTag::U, value } }
    /// Construct a signed-integer operand (stores the raw non-negative value).
    pub fn i(value: u64) -> Self { Self { tag: BEAMTag::I, value } }
    /// Construct an atom-index operand (1-based).
    pub fn a(index: u32) -> Self { Self { tag: BEAMTag::A, value: index as u64 } }
    /// Construct an x-register operand.
    pub fn x(reg: u8) -> Self { Self { tag: BEAMTag::X, value: reg as u64 } }
    /// Construct a label operand.
    pub fn f(label: u32) -> Self { Self { tag: BEAMTag::F, value: label as u64 } }
}

/// A single BEAM instruction: opcode byte + zero or more operands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BEAMInstruction {
    /// Single-byte opcode.
    pub opcode: u8,
    /// Operands in order.
    pub operands: Vec<BEAMOperand>,
}

impl BEAMInstruction {
    /// Build a new instruction.
    pub fn new(opcode: u8, operands: Vec<BEAMOperand>) -> Self {
        Self { opcode, operands }
    }
}

// ===========================================================================
// BEAMImport, BEAMExport
// ===========================================================================

/// A row in the `ImpT` (import) chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BEAMImport {
    /// 1-based atom-table index of the module name.
    pub module_atom_index: u32,
    /// 1-based atom-table index of the function name.
    pub function_atom_index: u32,
    /// Arity.
    pub arity: u32,
}

/// A row in the `ExpT` (export) or `LocT` (local) chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BEAMExport {
    /// 1-based atom-table index of the function name.
    pub function_atom_index: u32,
    /// Arity.
    pub arity: u32,
    /// 1-based BEAM label number of the function entry point.
    pub label: u32,
}

// ===========================================================================
// BEAMModule
// ===========================================================================

/// A complete BEAM module ready to be encoded.
///
/// Use [`encode_beam`] to produce a `.beam` file from this struct.
#[derive(Debug, Clone)]
pub struct BEAMModule {
    /// Module name (must equal `atoms[0]`).
    pub name: String,
    /// Atom table: 1-based, first entry is the module name.
    pub atoms: Vec<String>,
    /// Flat instruction stream.
    pub instructions: Vec<BEAMInstruction>,
    /// Import table rows.
    pub imports: Vec<BEAMImport>,
    /// Export table rows (exported functions).
    pub exports: Vec<BEAMExport>,
    /// Local table rows (non-exported callables).
    pub locals: Vec<BEAMExport>,
    /// Max label + 1 (used in Code chunk header).
    pub label_count: u32,
    /// Max opcode number across all instructions (0 = auto-derive).
    pub max_opcode: u32,
    /// Instruction-set format version (default 0).
    pub instruction_set_version: u32,
    /// Extra chunks to append verbatim.
    pub extra_chunks: Vec<([u8; 4], Vec<u8>)>,
}

// ===========================================================================
// Compact-term encoding
// ===========================================================================

/// Encode one operand using BEAM's variable-width compact-term format.
///
/// # Format
///
/// ```text
/// Bits 0-2: type tag
/// Bits 3+:  value
///
/// value < 16    → 1 byte:  (value << 4) | tag
/// 16 ≤ v < 2048 → 2 bytes: ((v >> 3) & 0xE0) | 0b1000 | tag, v & 0xFF
/// v ≥ 2048      → 1 header + BE bytes of v
/// ```
pub fn encode_compact_term(tag: BEAMTag, value: u64) -> Vec<u8> {
    let t = tag as u8;

    if value < 16 {
        // Small form: 1 byte, value in top 4 bits
        vec![((value as u8) << 4) | t]
    } else if value < 2048 {
        // Medium form: 2 bytes, value in 11 bits
        let hi = ((value >> 3) as u8) & 0xE0;
        let lo = (value & 0xFF) as u8;
        vec![hi | 0x08 | t, lo]
    } else {
        // Large form: header byte + big-endian bytes
        let bytes = value_to_be_bytes(value);
        let n = bytes.len();
        if n <= 8 {
            // (len - 2) encoded in 3 bits
            let len_field = ((n - 2) as u8) & 0x07;
            let header = (len_field << 5) | 0x18 | t;
            let mut out = vec![header];
            out.extend_from_slice(&bytes);
            out
        } else {
            // Very large: length itself is encoded as a compact-u
            let mut len_enc = encode_compact_term(BEAMTag::U, (n - 9) as u64);
            // Set bit pattern: 0b111 in top bits of header
            let header = 0b111_11000 | t;
            let mut out = vec![header];
            out.append(&mut len_enc);
            out.extend_from_slice(&bytes);
            out
        }
    }
}

/// Return the big-endian byte representation of `value`, minimum 1 byte,
/// no leading zero bytes.
fn value_to_be_bytes(value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }
    let raw = value.to_be_bytes(); // 8 bytes
    // Strip leading zeros
    let first = raw.iter().position(|&b| b != 0).unwrap_or(7);
    raw[first..].to_vec()
}

// ===========================================================================
// IFF chunk helpers
// ===========================================================================

/// Wrap `payload` in a 4-byte-aligned IFF chunk with the given 4-byte tag.
fn wrap_chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len() + 3);
    out.extend_from_slice(tag);
    // Use a checked cast: BEAM's IFF format stores the chunk length in a
    // 32-bit big-endian field.  A payload > 4 GiB cannot be represented and
    // would silently truncate without this check.
    let len = u32::try_from(payload.len())
        .expect("BEAM chunk payload exceeds 4 GiB — this should never occur in practice");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    // Pad to 4-byte alignment
    let pad = (4 - (payload.len() % 4)) % 4;
    out.extend(std::iter::repeat(0u8).take(pad));
    out
}

// ===========================================================================
// Chunk encoders
// ===========================================================================

/// Encode the `AtU8` atom-table chunk.
///
/// Format: `<u32 BE count> (<u8 len> <utf-8 bytes>)*`
fn encode_atu8(atoms: &[String]) -> Vec<u8> {
    let mut payload = Vec::new();
    let count = atoms.len() as u32;
    payload.extend_from_slice(&count.to_be_bytes());
    for atom in atoms {
        let bytes = atom.as_bytes();
        // Do not include the atom content in the assertion message — it may
        // contain user-controlled data that should not leak into crash logs.
        assert!(bytes.len() <= 255,
            "BEAM atom exceeds 255-byte limit: {} bytes", bytes.len());
        payload.push(bytes.len() as u8);
        payload.extend_from_slice(bytes);
    }
    payload
}

/// Encode the `Code` chunk.
///
/// Header (5 × u32 BE): sub_size=16, instruction_set, max_opcode,
/// label_count, function_count.  Then for each instruction: opcode byte
/// + compact-term-encoded operands.
fn encode_code(module: &BEAMModule) -> Vec<u8> {
    let max_op = if module.max_opcode > 0 {
        module.max_opcode
    } else {
        module.instructions.iter().map(|i| i.opcode as u32).max().unwrap_or(0)
    };
    let function_count = module.exports.len() as u32;

    let mut payload = Vec::new();
    // Header
    payload.extend_from_slice(&16u32.to_be_bytes());    // sub_size
    payload.extend_from_slice(&module.instruction_set_version.to_be_bytes());
    payload.extend_from_slice(&max_op.to_be_bytes());
    payload.extend_from_slice(&module.label_count.to_be_bytes());
    payload.extend_from_slice(&function_count.to_be_bytes());

    // Instructions
    for instr in &module.instructions {
        payload.push(instr.opcode);
        for op in &instr.operands {
            payload.extend(encode_compact_term(op.tag, op.value));
        }
    }
    payload
}

/// Encode an import or export table (both have the same 3-field row layout).
fn encode_table_3(rows: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&(rows.len() as u32).to_be_bytes());
    for &(a, b, c) in rows {
        payload.extend_from_slice(&a.to_be_bytes());
        payload.extend_from_slice(&b.to_be_bytes());
        payload.extend_from_slice(&c.to_be_bytes());
    }
    payload
}

// ===========================================================================
// Main encoder
// ===========================================================================

/// Encode a `BEAMModule` into a complete `.beam` file binary.
///
/// # Panics
///
/// Panics if `module.atoms` is empty or if any atom is longer than 255 bytes.
pub fn encode_beam(module: &BEAMModule) -> Vec<u8> {
    // ── Chunk payloads ─────────────────────────────────────────────────────
    let atu8_payload = encode_atu8(&module.atoms);
    let code_payload = encode_code(module);
    let impt_payload = encode_table_3(
        &module.imports.iter()
            .map(|i| (i.module_atom_index, i.function_atom_index, i.arity))
            .collect::<Vec<_>>()
    );
    let expt_payload = encode_table_3(
        &module.exports.iter()
            .map(|e| (e.function_atom_index, e.arity, e.label))
            .collect::<Vec<_>>()
    );
    let loct_payload = encode_table_3(
        &module.locals.iter()
            .map(|e| (e.function_atom_index, e.arity, e.label))
            .collect::<Vec<_>>()
    );
    // Empty StrT chunk (required by BEAM loader)
    let strt_payload: Vec<u8> = Vec::new();

    // ── Assemble chunks ────────────────────────────────────────────────────
    let mut chunks = Vec::new();
    chunks.extend(wrap_chunk(b"AtU8", &atu8_payload));
    chunks.extend(wrap_chunk(b"Code", &code_payload));
    chunks.extend(wrap_chunk(b"StrT", &strt_payload));
    chunks.extend(wrap_chunk(b"ImpT", &impt_payload));
    chunks.extend(wrap_chunk(b"ExpT", &expt_payload));
    if !loct_payload.is_empty() || !module.locals.is_empty() {
        chunks.extend(wrap_chunk(b"LocT", &loct_payload));
    }
    for (tag, payload) in &module.extra_chunks {
        chunks.extend(wrap_chunk(tag, payload));
    }

    // ── IFF container ──────────────────────────────────────────────────────
    let form_type = b"BEAM";
    let total_inner = form_type.len() + chunks.len();

    let mut out = Vec::new();
    out.extend_from_slice(b"FOR1");
    // Checked cast: the IFF header stores the total size in 32 bits.
    // Truncation would produce a corrupt file with a wrong length field.
    let total_inner_u32 = u32::try_from(total_inner)
        .expect("BEAM file exceeds 4 GiB IFF limit — this should never occur in practice");
    out.extend_from_slice(&total_inner_u32.to_be_bytes());
    out.extend_from_slice(form_type);
    out.extend(chunks);
    out
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // compact-term encoding
    // ------------------------------------------------------------------

    #[test]
    fn test_small_form_tag_u() {
        // value=0 tag=U → [(0<<4)|0] = [0x00]
        assert_eq!(encode_compact_term(BEAMTag::U, 0), vec![0x00]);
    }

    #[test]
    fn test_small_form_value_15() {
        // value=15 tag=U → [(15<<4)|0] = [0xF0]
        assert_eq!(encode_compact_term(BEAMTag::U, 15), vec![0xF0]);
    }

    #[test]
    fn test_small_form_with_tag_x() {
        // value=0 tag=X(3) → [3]
        assert_eq!(encode_compact_term(BEAMTag::X, 0), vec![0x03]);
    }

    #[test]
    fn test_medium_form() {
        // value=16: hi = (16>>3) & 0xE0 = 2<<5 = 0; lo = 16 & 0xFF = 16
        // header = 0 | 0x08 | tag=0 = 0x08
        let enc = encode_compact_term(BEAMTag::U, 16);
        assert_eq!(enc.len(), 2);
        assert_eq!(enc[0] & 0x0F, 0x08 | BEAMTag::U as u8); // low nibble: 0b1000
    }

    #[test]
    fn test_large_form_2048() {
        let enc = encode_compact_term(BEAMTag::U, 2048);
        // 2048 = 0x0800, 2 bytes: [0x08, 0x00]
        // n=2, len_field=(2-2)=0 → header = 0<<5 | 0b11000 | 0 = 0x18
        assert_eq!(enc[0], 0x18);
        assert_eq!(enc[1..], [0x08, 0x00]);
    }

    #[test]
    fn test_value_to_be_bytes_zero() {
        assert_eq!(value_to_be_bytes(0), vec![0x00]);
    }

    #[test]
    fn test_value_to_be_bytes_256() {
        assert_eq!(value_to_be_bytes(256), vec![0x01, 0x00]);
    }

    // ------------------------------------------------------------------
    // chunk encoding
    // ------------------------------------------------------------------

    #[test]
    fn test_wrap_chunk_pads_to_4() {
        // payload of 1 byte → padded to 4
        let chunk = wrap_chunk(b"Test", &[0xAB]);
        assert_eq!(chunk.len(), 12); // 4 tag + 4 size + 1 payload + 3 padding
    }

    #[test]
    fn test_wrap_chunk_no_padding_needed() {
        let chunk = wrap_chunk(b"Test", &[0; 4]);
        assert_eq!(chunk.len(), 12); // 4 tag + 4 size + 4 payload = 12
    }

    #[test]
    fn test_encode_atu8() {
        let atoms = vec!["hello".to_string(), "world".to_string()];
        let encoded = encode_atu8(&atoms);
        // count=2 (4 bytes) + [5, h,e,l,l,o] + [5, w,o,r,l,d]
        assert_eq!(&encoded[0..4], &2u32.to_be_bytes());
        assert_eq!(encoded[4], 5); // len of "hello"
        assert_eq!(&encoded[5..10], b"hello");
    }

    // ------------------------------------------------------------------
    // encode_beam end-to-end
    // ------------------------------------------------------------------

    #[test]
    fn test_encode_beam_starts_with_for1() {
        let module = BEAMModule {
            name: "test".to_string(),
            atoms: vec!["test".to_string()],
            instructions: vec![BEAMInstruction::new(3, vec![])], // INT_CODE_END
            imports: vec![],
            exports: vec![],
            locals: vec![],
            label_count: 1,
            max_opcode: 3,
            instruction_set_version: 0,
            extra_chunks: vec![],
        };
        let bytes = encode_beam(&module);
        assert_eq!(&bytes[0..4], b"FOR1");
        assert_eq!(&bytes[8..12], b"BEAM");
    }

    #[test]
    fn test_encode_beam_contains_atu8_chunk() {
        let module = BEAMModule {
            name: "m".to_string(),
            atoms: vec!["m".to_string()],
            instructions: vec![BEAMInstruction::new(3, vec![])],
            imports: vec![],
            exports: vec![],
            locals: vec![],
            label_count: 1,
            max_opcode: 3,
            instruction_set_version: 0,
            extra_chunks: vec![],
        };
        let bytes = encode_beam(&module);
        let pos = bytes.windows(4).position(|w| w == b"AtU8");
        assert!(pos.is_some(), "AtU8 chunk not found");
    }

    #[test]
    fn test_operand_helpers() {
        assert_eq!(BEAMOperand::u(42).tag, BEAMTag::U);
        assert_eq!(BEAMOperand::x(0).tag, BEAMTag::X);
        assert_eq!(BEAMOperand::f(1).value, 1);
        assert_eq!(BEAMOperand::a(2).tag, BEAMTag::A);
    }
}
