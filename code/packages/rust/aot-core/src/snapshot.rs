//! AOT snapshot format: writer and reader for `.aot` binaries.
//!
//! The `.aot` file is a self-contained binary that can be executed on the
//! target architecture.  Its layout is:
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ Header (26 bytes)                       │
//! │   magic              4 bytes  "AOT\0"  │
//! │   version            2 bytes  0x0100   │
//! │   flags              4 bytes           │
//! │   entry_point_offset 4 bytes           │
//! │   vm_iir_table_offset 4 bytes          │
//! │   vm_iir_table_size  4 bytes           │
//! │   native_code_size   4 bytes           │
//! ├─────────────────────────────────────────┤
//! │ Code section                            │
//! │   N bytes  native machine code          │
//! ├─────────────────────────────────────────┤
//! │ IIR table section (optional)            │
//! │   M bytes  serialised IIR for dynamic   │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Flags
//!
//! | Bit | Name | Meaning |
//! |-----|------|---------|
//! | 0 | `FLAG_VM_RUNTIME` | IIR table section is present |
//! | 1 | `FLAG_DEBUG_INFO` | Debug section present (future; always 0) |
//!
//! **Endianness**: little-endian throughout.
//!
//! # Example
//!
//! ```
//! use aot_core::snapshot::{write, read};
//!
//! let code = b"\xde\xad\xbe\xef";
//! let raw = write(code, None, 0);
//! let snap = read(&raw).unwrap();
//! assert_eq!(snap.native_code, code);
//! assert!(snap.iir_table.is_none());
//! assert_eq!(snap.entry_point_offset, 0);
//! ```

use crate::errors::AOTError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The magic bytes that open every `.aot` file.
pub const MAGIC: &[u8; 4] = b"AOT\x00";

/// Version word: major=1, minor=0 packed as little-endian u16.
pub const VERSION: u16 = 0x0100;

/// Flag bit 0: the IIR table (vm-runtime) section is present.
pub const FLAG_VM_RUNTIME: u32 = 0x01;

/// Flag bit 1: a debug-info section is present (reserved; not yet implemented).
pub const FLAG_DEBUG_INFO: u32 = 0x02;

/// Size of the fixed-length header in bytes.
///
/// Layout: magic(4) + version(2) + flags(4) + entry_point_offset(4)
///         + vm_iir_table_offset(4) + vm_iir_table_size(4) + native_code_size(4) = 26.
pub const HEADER_SIZE: usize = 26;

// ---------------------------------------------------------------------------
// AOTSnapshot — parsed representation
// ---------------------------------------------------------------------------

/// Parsed contents of a `.aot` binary.
///
/// Produced by [`read`]; consumed by the AOT runtime or simulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AOTSnapshot {
    /// Packed version word (`0x0100` = v1.0).
    pub version: u16,

    /// Flag bitmask (see `FLAG_*` constants).
    pub flags: u32,

    /// Byte offset within `native_code` where the entry-point function begins.
    pub entry_point_offset: u32,

    /// Raw native machine code (all compiled functions concatenated).
    pub native_code: Vec<u8>,

    /// Serialised IIR bytes for uncompiled functions, or `None` if the
    /// `FLAG_VM_RUNTIME` flag is not set.
    pub iir_table: Option<Vec<u8>>,
}

impl AOTSnapshot {
    /// True if the IIR table section is present.
    pub fn has_vm_runtime(&self) -> bool {
        self.flags & FLAG_VM_RUNTIME != 0
    }

    /// True if the debug-info section is present.
    pub fn has_debug_info(&self) -> bool {
        self.flags & FLAG_DEBUG_INFO != 0
    }
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Serialise a compiled module to the `.aot` binary format.
///
/// # Parameters
///
/// - `native_code` — combined native binary (all functions concatenated, e.g.
///   via [`link::link`](crate::link::link)).
/// - `iir_table` — serialised IIR for functions that could not be compiled.
///   Pass `None` when all functions were compiled.
/// - `entry_point_offset` — byte offset within `native_code` where `main`
///   begins.
///
/// # Returns
///
/// A `Vec<u8>` containing the complete `.aot` binary.
///
/// # Example
///
/// ```
/// use aot_core::snapshot::{write, HEADER_SIZE};
///
/// let raw = write(b"\x01\x02\x03", None, 0);
/// assert_eq!(raw.len(), HEADER_SIZE + 3);
/// assert_eq!(&raw[0..4], b"AOT\x00");
/// ```
pub fn write(native_code: &[u8], iir_table: Option<&[u8]>, entry_point_offset: u32) -> Vec<u8> {
    let mut flags: u32 = 0;
    let iir_bytes = iir_table.unwrap_or(&[]);
    if iir_table.is_some() {
        flags |= FLAG_VM_RUNTIME;
    }

    let native_code_size = native_code.len() as u32;
    let vm_iir_table_size = iir_bytes.len() as u32;

    let vm_iir_table_offset: u32 = if iir_table.is_some() {
        (HEADER_SIZE as u32) + native_code_size
    } else {
        0
    };

    let mut out: Vec<u8> = Vec::with_capacity(HEADER_SIZE + native_code.len() + iir_bytes.len());

    // magic (4 bytes)
    out.extend_from_slice(MAGIC);
    // version (2 bytes, little-endian)
    out.extend_from_slice(&VERSION.to_le_bytes());
    // flags (4 bytes, little-endian)
    out.extend_from_slice(&flags.to_le_bytes());
    // entry_point_offset (4 bytes, little-endian)
    out.extend_from_slice(&entry_point_offset.to_le_bytes());
    // vm_iir_table_offset (4 bytes, little-endian)
    out.extend_from_slice(&vm_iir_table_offset.to_le_bytes());
    // vm_iir_table_size (4 bytes, little-endian)
    out.extend_from_slice(&vm_iir_table_size.to_le_bytes());
    // native_code_size (4 bytes, little-endian)
    out.extend_from_slice(&native_code_size.to_le_bytes());

    debug_assert_eq!(out.len(), HEADER_SIZE);

    // Code section.
    out.extend_from_slice(native_code);
    // IIR table section.
    out.extend_from_slice(iir_bytes);

    out
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Parse a `.aot` binary into an [`AOTSnapshot`].
///
/// # Errors
///
/// Returns [`AOTError::Snapshot`] if:
/// - `data` is shorter than the 26-byte header.
/// - The magic bytes do not match `b"AOT\0"`.
/// - The declared code or IIR table sections exceed the length of `data`.
/// - The IIR table offset would overlap the header or code section.
///
/// # Example
///
/// ```
/// use aot_core::snapshot::{write, read, VERSION};
///
/// let raw = write(b"\xAA\xBB", None, 0);
/// let snap = read(&raw).unwrap();
/// assert_eq!(snap.version, VERSION);
/// assert_eq!(snap.native_code, b"\xAA\xBB");
/// assert!(!snap.has_vm_runtime());
/// ```
pub fn read(data: &[u8]) -> Result<AOTSnapshot, AOTError> {
    if data.len() < HEADER_SIZE {
        return Err(AOTError::snapshot(format!(
            "data too short: {} < {}",
            data.len(),
            HEADER_SIZE
        )));
    }

    let magic = &data[0..4];
    if magic != MAGIC {
        return Err(AOTError::snapshot(format!(
            "bad magic: {:?} (expected {:?})",
            magic, MAGIC
        )));
    }

    let version = u16::from_le_bytes([data[4], data[5]]);
    let flags   = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);
    let entry_point_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);
    let vm_iir_table_offset = u32::from_le_bytes([data[14], data[15], data[16], data[17]]) as usize;
    let vm_iir_table_size   = u32::from_le_bytes([data[18], data[19], data[20], data[21]]) as usize;
    let native_code_size    = u32::from_le_bytes([data[22], data[23], data[24], data[25]]) as usize;

    let code_start = HEADER_SIZE;
    // Use checked arithmetic to prevent integer overflow on crafted binaries.
    // If `native_code_size` were close to `usize::MAX`, an unchecked addition
    // could wrap around and bypass the bounds check below.
    let code_end = code_start
        .checked_add(native_code_size)
        .ok_or_else(|| AOTError::snapshot("native_code_size overflow"))?;
    if code_end > data.len() {
        return Err(AOTError::snapshot(format!(
            "native_code section truncated: need {} bytes, have {}",
            code_end, data.len()
        )));
    }
    let native_code = data[code_start..code_end].to_vec();

    let iir_table: Option<Vec<u8>> = if flags & FLAG_VM_RUNTIME != 0 {
        if vm_iir_table_size == 0 {
            Some(vec![])
        } else {
            // `expected_min` is the first byte after the code section — the
            // IIR table must start at or after this point.
            let expected_min = HEADER_SIZE
                .checked_add(native_code_size)
                .ok_or_else(|| AOTError::snapshot("native_code_size overflow in iir_table check"))?;
            if vm_iir_table_offset < expected_min {
                return Err(AOTError::snapshot(format!(
                    "iir_table offset {} overlaps header or code section (expected >= {})",
                    vm_iir_table_offset, expected_min
                )));
            }
            // Checked addition guards against a crafted
            // `vm_iir_table_offset + vm_iir_table_size` wrap-around that
            // would make `iir_end` smaller than `data.len()`, bypassing the
            // bounds check and causing a panic (or DoS).
            let iir_end = vm_iir_table_offset
                .checked_add(vm_iir_table_size)
                .ok_or_else(|| AOTError::snapshot("iir_table offset+size overflow"))?;
            if iir_end > data.len() {
                return Err(AOTError::snapshot(format!(
                    "iir_table section truncated: need {} bytes, have {}",
                    iir_end, data.len()
                )));
            }
            Some(data[vm_iir_table_offset..iir_end].to_vec())
        }
    } else {
        None
    };

    Ok(AOTSnapshot {
        version,
        flags,
        entry_point_offset,
        native_code,
        iir_table,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size_is_26() {
        // magic(4) + version(2) + flags(4) + ep(4) + iir_off(4) + iir_sz(4) + code_sz(4) = 26
        assert_eq!(HEADER_SIZE, 26);
    }

    #[test]
    fn write_magic() {
        let raw = write(b"", None, 0);
        assert_eq!(&raw[0..4], b"AOT\x00");
    }

    #[test]
    fn write_version() {
        let raw = write(b"", None, 0);
        let v = u16::from_le_bytes([raw[4], raw[5]]);
        assert_eq!(v, VERSION);
    }

    #[test]
    fn write_no_iir_table() {
        let raw = write(b"\xDE\xAD", None, 0);
        assert_eq!(raw.len(), HEADER_SIZE + 2);
        let flags = u32::from_le_bytes([raw[6], raw[7], raw[8], raw[9]]);
        assert_eq!(flags & FLAG_VM_RUNTIME, 0);
    }

    #[test]
    fn write_with_iir_table_sets_flag() {
        let raw = write(b"\x01", Some(b"\x02\x03"), 0);
        let flags = u32::from_le_bytes([raw[6], raw[7], raw[8], raw[9]]);
        assert_ne!(flags & FLAG_VM_RUNTIME, 0);
    }

    #[test]
    fn write_total_length() {
        let raw = write(b"\x01\x02\x03", Some(b"\x04\x05"), 0);
        assert_eq!(raw.len(), HEADER_SIZE + 3 + 2);
    }

    #[test]
    fn round_trip_no_iir() {
        let code = b"\xDE\xAD\xBE\xEF";
        let raw = write(code, None, 7);
        let snap = read(&raw).unwrap();
        assert_eq!(snap.native_code, code);
        assert!(snap.iir_table.is_none());
        assert_eq!(snap.entry_point_offset, 7);
        assert_eq!(snap.version, VERSION);
    }

    #[test]
    fn round_trip_with_iir() {
        let code = b"\x01\x02";
        let iir  = b"\xAA\xBB\xCC";
        let raw = write(code, Some(iir), 0);
        let snap = read(&raw).unwrap();
        assert_eq!(snap.native_code, code);
        assert_eq!(snap.iir_table.as_deref(), Some(iir.as_ref()));
        assert!(snap.has_vm_runtime());
    }

    #[test]
    fn read_too_short_error() {
        let result = read(b"\x00\x01");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn read_bad_magic_error() {
        let mut raw = write(b"\x01", None, 0);
        raw[0] = b'X'; // corrupt magic
        let result = read(&raw);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bad magic"));
    }

    #[test]
    fn read_truncated_code_error() {
        let mut raw = write(b"\x01\x02\x03", None, 0);
        // Declare 3 bytes of code but chop 1 byte off the data.
        raw.pop();
        let result = read(&raw);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("truncated"));
    }

    #[test]
    fn has_vm_runtime_false_without_flag() {
        let snap = AOTSnapshot {
            version: VERSION,
            flags: 0,
            entry_point_offset: 0,
            native_code: vec![],
            iir_table: None,
        };
        assert!(!snap.has_vm_runtime());
    }

    #[test]
    fn has_debug_info_false_by_default() {
        let raw = write(b"", None, 0);
        let snap = read(&raw).unwrap();
        assert!(!snap.has_debug_info());
    }

    #[test]
    fn empty_native_code() {
        let raw = write(b"", None, 0);
        let snap = read(&raw).unwrap();
        assert!(snap.native_code.is_empty());
    }

    #[test]
    fn empty_iir_table_flag_set() {
        // Writing Some(&[]) should still set FLAG_VM_RUNTIME.
        let raw = write(b"", Some(b""), 0);
        let snap = read(&raw).unwrap();
        assert!(snap.has_vm_runtime());
        assert_eq!(snap.iir_table.as_deref(), Some(&[][..]));
    }

    // ------------------------------------------------------------------
    // Security: checked arithmetic / overflow protection
    // ------------------------------------------------------------------

    /// A crafted header claiming `native_code_size` is exactly `data.len() - HEADER_SIZE + 1`
    /// bytes (one byte past the end) must be rejected cleanly, not panic.
    #[test]
    fn overflow_native_code_size_rejected() {
        // Write a valid snapshot of code `[0xFF]`.
        let mut raw = write(b"\xFF", None, 0);
        // Overwrite native_code_size (bytes 22..26) with a value that
        // would overflow if added to HEADER_SIZE.
        let bad_size: u32 = u32::MAX;
        raw[22..26].copy_from_slice(&bad_size.to_le_bytes());
        let result = read(&raw);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        // Should mention overflow or truncation — not panic.
        assert!(msg.contains("overflow") || msg.contains("truncated"));
    }

    /// A crafted IIR table header where offset + size wraps around must be rejected.
    #[test]
    fn overflow_iir_table_offset_rejected() {
        // Build a snapshot with an IIR table present.
        let mut raw = write(b"\x01", Some(b"\x02"), 0);
        // Set vm_iir_table_size (bytes 18..22) to u32::MAX.
        let bad_size: u32 = u32::MAX;
        raw[18..22].copy_from_slice(&bad_size.to_le_bytes());
        let result = read(&raw);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("overflow") || msg.contains("truncated"));
    }
}
