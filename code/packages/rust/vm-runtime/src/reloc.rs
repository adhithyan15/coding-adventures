//! `RelocationKind` and `RelocationEntry` — the relocation contract for AOT binaries.
//!
//! When the AOT compiler emits code that calls an unspecialised function or
//! a builtin, it does not know the function's runtime index.  The linker
//! resolves these references after all functions are laid out.  The AOT
//! compiler records a **relocation entry** for each unresolved reference.
//!
//! # Relocation entry layout (16 bytes)
//!
//! ```text
//! [0..3]   site_offset    u32-LE  byte offset in the native-code section
//! [4..7]   symbol_offset  u32-LE  offset into the relocation string pool
//! [8..9]   reloc_kind     u16-LE  kind discriminant (see `RelocationKind`)
//! [10..11] addend         u16-LE  constant added to the resolved value
//! [12..15] reserved       u32     zero for now (future: section index)
//! ```
//!
//! # Usage
//!
//! ```
//! use vm_runtime::reloc::{RelocationKind, RelocationEntry};
//!
//! let entry = RelocationEntry {
//!     site_offset: 0x100,
//!     symbol: "main".into(),
//!     kind: RelocationKind::IirFnIndex,
//!     addend: 0,
//! };
//! let bytes = entry.serialise();
//! assert_eq!(bytes.len(), 16);
//! ```

/// Discriminant for how a relocation site should be patched.
///
/// Values 0x0001–0x0006 match the spec table for `reloc_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum RelocationKind {
    /// Patch site with a 32-bit index into the `vm_iir_table`.
    ///
    /// Used when an AOT-compiled function calls an unspecialised function
    /// (i.e. one whose IIR is stored in the IIR table for interpreter
    /// fallback).
    IirFnIndex = 0x0001,

    /// Patch site with a 16-bit index into the builtins registry.
    ///
    /// Used when an AOT-compiled function calls a registered builtin
    /// (e.g. `print`, `len`).
    BuiltinIndex = 0x0002,

    /// Patch site with the **absolute address** of a vm-runtime entry point.
    ///
    /// Used on architectures that support absolute jumps (x86-64, AArch64).
    RtEntryAbs = 0x0003,

    /// Patch site with a **PC-relative offset** to a vm-runtime entry point.
    ///
    /// Used on architectures with PC-relative addressing (RISC-V, Thumb).
    RtEntryPcRel = 0x0004,

    /// Patch site with an offset into the `.aot` file's string pool.
    ///
    /// Used for instructions that need a string literal at runtime.
    StringPool = 0x0005,

    /// Patch site with an offset into the GC root table.
    ///
    /// Only present in level-3 (`Full`) binaries; requires `gc-core` (LANG16).
    GcRootTable = 0x0006,
}

impl RelocationKind {
    /// Decode a raw `u16` kind discriminant.
    ///
    /// ```
    /// use vm_runtime::reloc::RelocationKind;
    /// assert_eq!(RelocationKind::from_u16(0x0001), Some(RelocationKind::IirFnIndex));
    /// assert_eq!(RelocationKind::from_u16(0xDEAD), None);
    /// ```
    pub fn from_u16(raw: u16) -> Option<Self> {
        match raw {
            0x0001 => Some(RelocationKind::IirFnIndex),
            0x0002 => Some(RelocationKind::BuiltinIndex),
            0x0003 => Some(RelocationKind::RtEntryAbs),
            0x0004 => Some(RelocationKind::RtEntryPcRel),
            0x0005 => Some(RelocationKind::StringPool),
            0x0006 => Some(RelocationKind::GcRootTable),
            _ => None,
        }
    }
}

impl std::fmt::Display for RelocationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelocationKind::IirFnIndex => write!(f, "IIR_FN_INDEX"),
            RelocationKind::BuiltinIndex => write!(f, "BUILTIN_INDEX"),
            RelocationKind::RtEntryAbs => write!(f, "RT_ENTRY_ABS"),
            RelocationKind::RtEntryPcRel => write!(f, "RT_ENTRY_PCREL"),
            RelocationKind::StringPool => write!(f, "STRING_POOL"),
            RelocationKind::GcRootTable => write!(f, "GC_ROOT_TABLE"),
        }
    }
}

/// A single relocation entry emitted by the AOT compiler.
///
/// The linker walks the relocation list and patches each `site_offset` in
/// the native code section with the resolved value.
///
/// # Example
///
/// ```
/// use vm_runtime::reloc::{RelocationKind, RelocationEntry};
///
/// let e = RelocationEntry {
///     site_offset: 0x20,
///     symbol: "helper".into(),
///     kind: RelocationKind::IirFnIndex,
///     addend: 0,
/// };
/// assert_eq!(e.kind, RelocationKind::IirFnIndex);
/// let bytes = e.serialise();
/// let back = RelocationEntry::deserialise(&bytes, &["helper".into()]).unwrap();
/// assert_eq!(back.site_offset, 0x20);
/// assert_eq!(back.symbol, "helper");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RelocationEntry {
    /// Byte offset in the native code section where the resolved value is
    /// written.
    pub site_offset: u32,

    /// The symbol name whose resolved address/index is patched in.
    ///
    /// In the binary format this is stored as an offset into a string pool;
    /// the Rust struct holds the resolved name for convenience.
    pub symbol: String,

    /// How to interpret and patch the site.
    pub kind: RelocationKind,

    /// A constant added to the resolved value before patching.  Typically 0.
    pub addend: u16,
}

impl RelocationEntry {
    /// Serialise this entry to 16 bytes.
    ///
    /// The symbol name is stored as its byte index in a separately maintained
    /// string pool.  When serialising standalone (outside a full `.aot`
    /// writer), `symbol_offset` is set to 0 — callers building a real `.aot`
    /// binary should use a string pool manager.
    pub fn serialise(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&self.site_offset.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // symbol_offset (placeholder)
        buf.extend_from_slice(&(self.kind as u16).to_le_bytes());
        buf.extend_from_slice(&self.addend.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved
        buf
    }

    /// Deserialise from 16 bytes, looking up the symbol in `string_pool`.
    ///
    /// `string_pool` is a slice of symbol names in the order they appear in
    /// the pool; the `symbol_offset` field is used as an index here (V1
    /// simplification).
    pub fn deserialise(
        bytes: &[u8],
        string_pool: &[String],
    ) -> Result<Self, RelocationError> {
        if bytes.len() < 16 {
            return Err(RelocationError::TooShort);
        }
        let site_offset = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let sym_idx = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let kind_raw = u16::from_le_bytes([bytes[8], bytes[9]]);
        let addend = u16::from_le_bytes([bytes[10], bytes[11]]);

        let kind = RelocationKind::from_u16(kind_raw)
            .ok_or(RelocationError::UnknownKind(kind_raw))?;

        let symbol = string_pool
            .get(sym_idx)
            .cloned()
            .unwrap_or_else(|| format!("sym_{}", sym_idx));

        Ok(RelocationEntry { site_offset, symbol, kind, addend })
    }
}

/// Errors that can occur while reading or writing relocation entries.
#[derive(Debug, Clone, PartialEq)]
pub enum RelocationError {
    /// The byte slice is too short (less than 16 bytes).
    TooShort,
    /// An unrecognised `reloc_kind` was encountered.
    UnknownKind(u16),
}

impl std::fmt::Display for RelocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelocationError::TooShort => write!(f, "relocation entry too short"),
            RelocationError::UnknownKind(k) => write!(f, "unknown reloc_kind 0x{:04X}", k),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialise_is_16_bytes() {
        let e = RelocationEntry {
            site_offset: 0x100,
            symbol: "main".into(),
            kind: RelocationKind::IirFnIndex,
            addend: 0,
        };
        assert_eq!(e.serialise().len(), 16);
    }

    #[test]
    fn roundtrip_reloc_entry() {
        let e = RelocationEntry {
            site_offset: 0x20,
            symbol: "helper".into(),
            kind: RelocationKind::IirFnIndex,
            addend: 0,
        };
        let bytes = e.serialise();
        let pool = vec!["helper".to_string()];
        let back = RelocationEntry::deserialise(&bytes, &pool).unwrap();
        assert_eq!(back.site_offset, 0x20);
        assert_eq!(back.kind, RelocationKind::IirFnIndex);
        assert_eq!(back.addend, 0);
    }

    #[test]
    fn kind_from_u16_all_known() {
        let kinds = [0x0001u16, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006];
        for k in kinds {
            assert!(RelocationKind::from_u16(k).is_some(), "kind 0x{:04X}", k);
        }
    }

    #[test]
    fn kind_from_u16_unknown() {
        assert_eq!(RelocationKind::from_u16(0xBEEF), None);
    }

    #[test]
    fn kind_display() {
        assert_eq!(RelocationKind::IirFnIndex.to_string(), "IIR_FN_INDEX");
        assert_eq!(RelocationKind::GcRootTable.to_string(), "GC_ROOT_TABLE");
    }

    #[test]
    fn deserialise_too_short() {
        let bytes = vec![0u8; 5];
        assert_eq!(
            RelocationEntry::deserialise(&bytes, &[]),
            Err(RelocationError::TooShort)
        );
    }

    #[test]
    fn deserialise_unknown_kind() {
        let mut bytes = vec![0u8; 16];
        bytes[8] = 0xFF; // kind_raw low byte
        bytes[9] = 0xFF; // kind_raw high byte
        assert!(matches!(
            RelocationEntry::deserialise(&bytes, &[]),
            Err(RelocationError::UnknownKind(_))
        ));
    }
}
