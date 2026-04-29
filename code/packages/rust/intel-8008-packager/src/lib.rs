//! # intel-8008-packager — Intel HEX ROM image encoder/decoder for the Intel 8008.
//!
//! Converts raw binary machine code into the Intel HEX format used by EPROM
//! programmers, and parses Intel HEX back to binary for round-trip verification.
//!
//! ## Pipeline position
//!
//! ```text
//! Oct source (.oct)
//!   → oct-lexer, oct-parser, oct-type-checker
//!   → oct-ir-compiler
//! IrProgram
//!   → intel-8008-ir-validator
//!   → ir-to-intel-8008-compiler
//! 8008 Assembly text (.asm)
//!   → intel-8008-assembler         ← feeds THIS crate
//! Binary bytes
//!   → intel-8008-packager          ← THIS crate
//! Intel HEX file (.hex)
//!   → intel8008-simulator
//! ```
//!
//! ## Intel HEX record format
//!
//! Each line in an Intel HEX file is a "record":
//!
//! ```text
//! :LLAAAATTDDDDDD...CC
//!
//!   :     — start code
//!   LL    — byte count (hex), number of data bytes (0–255)
//!   AAAA  — load address (16-bit big-endian hex)
//!   TT    — record type:
//!           00 = Data
//!           01 = End Of File
//!   DD... — data bytes (LL × 2 hex chars)
//!   CC    — checksum: two's complement of the byte-sum of all fields
//! ```
//!
//! ## Checksum
//!
//! `checksum = (0x100 - (sum_of_all_bytes % 256)) % 256`
//!
//! The checksum is chosen so that summing **all** bytes in the record
//! (including the checksum itself) yields 0x00 mod 256.  ROM-programmer
//! firmware verifies integrity by summing each record byte: if the result
//! is non-zero, the record is corrupt.
//!
//! ## Intel 8008 address space
//!
//! The Intel 8008 has a **14-bit address space** (16 KB = 0x0000–0x3FFF):
//!
//! ```text
//! 0x0000–0x1FFF   ROM: program code (8 KB)
//! 0x2000–0x3FFF   RAM: static variable data (8 KB)
//! ```
//!
//! We use 16 bytes per data record (standard "ihex16" format, compatible with
//! all EPROM programmers).  The maximum image size is 16 384 bytes (16 KB).
//!
//! ## Quick start
//!
//! ```
//! use intel_8008_packager::{encode_hex, decode_hex};
//!
//! // Binary → Intel HEX
//! let binary = vec![0x06u8, 0x00, 0xFF];   // MVI B, 0; HLT
//! let hex_text = encode_hex(&binary, 0).unwrap();
//! assert!(hex_text.contains(":00000001FF"));   // EOF record always present
//!
//! // Intel HEX → binary (round-trip)
//! let decoded = decode_hex(&hex_text).unwrap();
//! assert_eq!(decoded.origin, 0);
//! assert_eq!(decoded.binary, binary);
//! ```

use std::collections::BTreeMap;

// ===========================================================================
// Constants
// ===========================================================================

/// Number of data bytes per Intel HEX data record (standard "ihex16").
const BYTES_PER_RECORD: usize = 16;

/// Intel HEX record type: Data.
const RECORD_TYPE_DATA: u8 = 0x00;

/// Intel HEX record type: End Of File.
const RECORD_TYPE_EOF: u8 = 0x01;

/// Maximum image size for the Intel 8008: 14-bit address space = 16 KB.
///
/// An adversarial HEX file that claims records at widely-separated addresses
/// (e.g. 0x0000 and 0xFFFF) would cause a huge allocation if we trusted the
/// span naively.  We cap at 16 KB, which is the full 8008 address space.
const MAX_IMAGE_SIZE: usize = 0x4000;

// ===========================================================================
// PackagerError — the single public error type
// ===========================================================================

/// An error from the Intel 8008 packager.
///
/// Covers both encoding errors (empty binary, address overflow) and
/// decoding errors (malformed records, bad checksums, unsupported types).
///
/// # Display
///
/// ```
/// use intel_8008_packager::PackagerError;
/// let e = PackagerError("binary must be non-empty".to_string());
/// assert_eq!(e.to_string(), "binary must be non-empty");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagerError(pub String);

impl std::fmt::Display for PackagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PackagerError {}

// ===========================================================================
// DecodedHex — result of decode_hex()
// ===========================================================================

/// Result of parsing an Intel HEX file.
///
/// Contains the lowest load address seen across all data records and the
/// concatenated binary payload in address order.
///
/// # Example
///
/// ```
/// use intel_8008_packager::{encode_hex, decode_hex};
///
/// let binary = vec![0x7Cu8, 0x03, 0x00, 0xFF];
/// let decoded = decode_hex(&encode_hex(&binary, 0x0010).unwrap()).unwrap();
/// assert_eq!(decoded.origin, 0x0010);
/// assert_eq!(decoded.binary, binary);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHex {
    /// Lowest load address from all data records.
    pub origin: usize,
    /// Concatenated binary payload starting at `origin`.
    pub binary: Vec<u8>,
}

// ===========================================================================
// Internal helpers
// ===========================================================================

/// Compute the Intel HEX checksum for a record.
///
/// The checksum is the two's complement of the byte-sum of all record fields:
/// `[byte_count, addr_hi, addr_lo, record_type, data0, data1, ...]`.
///
/// It is chosen so that summing **all** bytes in the record (including the
/// checksum byte) yields 0 mod 256 — making verification trivial.
///
/// ```text
/// Example: fields = [0x03, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03]
/// sum = 3 + 0 + 0 + 0 + 1 + 2 + 3 = 9
/// checksum = (0x100 - 9) % 0x100 = 0xF7
/// ```
fn checksum(fields: &[u8]) -> u8 {
    let total: u32 = fields.iter().map(|b| *b as u32).sum();
    ((0x100 - (total % 0x100)) % 0x100) as u8
}

/// Format a single Intel HEX data record.
///
/// Layout: `:{LL}{AAAA}00{DD...}{CC}\n`
/// - LL   = byte count (2 hex digits)
/// - AAAA = 16-bit address (4 hex digits, big-endian)
/// - 00   = record type DATA
/// - DD   = data bytes (2 hex digits each)
/// - CC   = checksum (2 hex digits)
fn data_record(address: usize, chunk: &[u8]) -> String {
    let n = chunk.len() as u8;
    let addr_hi = ((address >> 8) & 0xFF) as u8;
    let addr_lo = (address & 0xFF) as u8;

    // Build the fields list for checksum computation
    let mut fields = vec![n, addr_hi, addr_lo, RECORD_TYPE_DATA];
    fields.extend_from_slice(chunk);
    let cs = checksum(&fields);

    // Encode data bytes as uppercase hex
    let data_hex: String = chunk.iter().map(|b| format!("{b:02X}")).collect();
    format!(":{n:02X}{addr_hi:02X}{addr_lo:02X}{:02X}{data_hex}{cs:02X}\n", RECORD_TYPE_DATA)
}

/// Return the Intel HEX end-of-file record.
///
/// The EOF record is always `:00000001FF`:
/// - byte count = 0x00
/// - address = 0x0000
/// - record type = 0x01 (EOF)
/// - checksum = 0xFF  (two's complement of 0x01)
fn eof_record() -> &'static str {
    ":00000001FF\n"
}

/// Maximum permitted line length in Intel HEX input.
///
/// A well-formed data record with 255 data bytes encodes as:
///   1 (`:`) + 2 (LL) + 4 (AAAA) + 2 (TT) + 510 (255 × 2 hex chars) + 2 (CS) = 521 chars
/// We allow up to 1 024 chars to accommodate any plausible valid record.
/// Lines longer than this cannot be valid Intel HEX; rejecting them early avoids
/// a transient O(n) char-iterator allocation for adversarially long lines.
const MAX_HEX_LINE_LEN: usize = 1024;

/// Decode a hex string slice into bytes.
///
/// Returns `Err(())` if the string has odd length or contains non-hex chars.
/// Does **not** allocate an intermediate `Vec<char>` — iterates directly over
/// char pairs to keep memory bounded to the output slice length.
fn parse_hex_bytes(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    let mut iter = s.chars();
    loop {
        match (iter.next(), iter.next()) {
            (Some(hi), Some(lo)) => {
                let hi_val = hi.to_digit(16).ok_or(())?;
                let lo_val = lo.to_digit(16).ok_or(())?;
                bytes.push((hi_val * 16 + lo_val) as u8);
            }
            (None, _) => break,
            _ => return Err(()), // odd number of chars (shouldn't happen after len check)
        }
    }
    Ok(bytes)
}

// ===========================================================================
// Public API
// ===========================================================================

/// Convert raw binary bytes to an Intel HEX string.
///
/// Splits `binary` into records of up to 16 bytes each, starting at `origin`,
/// then appends the mandatory end-of-file record.
///
/// # Arguments
///
/// - `binary`  — the compiled ROM image.  For the Intel 8008: up to 16 384
///   bytes (0x0000–0x3FFF), covering ROM code and RAM regions.
/// - `origin`  — ROM load address for the first byte (0–65535; default 0).
///   `origin + len(binary)` must not exceed 65536 (the 16-bit address space).
///
/// # Errors
///
/// Returns [`PackagerError`] if:
/// - `binary` is empty
/// - `origin` is greater than 65535
/// - `origin + len(binary)` would overflow the 16-bit address space
///
/// # Example
///
/// ```
/// use intel_8008_packager::encode_hex;
///
/// let binary = vec![0x06u8, 0x00, 0xFF];   // MVI B, 0; HLT
/// let hex = encode_hex(&binary, 0).unwrap();
/// let lines: Vec<&str> = hex.lines().collect();
/// // data record for 3 bytes at address 0x0000
/// assert!(lines[0].starts_with(":03000000"));
/// // EOF record
/// assert_eq!(lines[1], ":00000001FF");
/// ```
pub fn encode_hex(binary: &[u8], origin: usize) -> Result<String, PackagerError> {
    if binary.is_empty() {
        return Err(PackagerError("binary must be non-empty".to_string()));
    }
    if origin > 0xFFFF {
        return Err(PackagerError(format!(
            "origin must be 0–65535, got {origin:#06x}"
        )));
    }
    if origin + binary.len() > 0x10000 {
        return Err(PackagerError(format!(
            "image overflows 16-bit address space: origin={origin:#06x}, size={}",
            binary.len()
        )));
    }

    let mut output = String::new();
    let mut offset = 0;
    while offset < binary.len() {
        let end = (offset + BYTES_PER_RECORD).min(binary.len());
        output.push_str(&data_record(origin + offset, &binary[offset..end]));
        offset = end;
    }
    output.push_str(eof_record());
    Ok(output)
}

/// Parse an Intel HEX string back to the origin address and binary bytes.
///
/// Only handles Type 00 (Data) and Type 01 (EOF) records — the subset
/// produced by [`encode_hex`].  Type 02+ records cause an error.
///
/// The image size is capped at [`MAX_IMAGE_SIZE`] (16 KB) to guard against
/// adversarial HEX files that claim widely-separated addresses and would
/// cause a large allocation.
///
/// # Errors
///
/// Returns [`PackagerError`] if:
/// - Any record line is missing the leading `:`
/// - A line contains non-hex characters
/// - A record is too short to contain the claimed byte count
/// - A checksum is incorrect
/// - An unsupported record type (≥ 0x02) is encountered
/// - The decoded image span exceeds 16 KB
/// - The EOF record (type 0x01) is absent (truncated file)
/// - Two data records have overlapping or duplicate addresses
///
/// # Example
///
/// ```
/// use intel_8008_packager::{encode_hex, decode_hex};
///
/// let binary = vec![0x06u8, 0x00, 0xFF];
/// let decoded = decode_hex(&encode_hex(&binary, 0).unwrap()).unwrap();
/// assert_eq!(decoded.origin, 0);
/// assert_eq!(decoded.binary, binary);
/// ```
pub fn decode_hex(text: &str) -> Result<DecodedHex, PackagerError> {
    // BTreeMap keeps addresses sorted so we reconstruct the image in order.
    let mut segments: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    // Tracks whether we saw the mandatory type-0x01 EOF sentinel.
    // A truncated file that stops before the EOF record is treated as malformed.
    let mut found_eof = false;

    for (line_idx, raw) in text.lines().enumerate() {
        let line_num = line_idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // Guard against adversarially long lines before doing any allocation.
        // A maximum-payload data record (255 bytes) encodes as 521 chars.
        // We allow up to MAX_HEX_LINE_LEN (1 024) to cover all valid records;
        // anything longer cannot be a well-formed Intel HEX record.
        if line.len() > MAX_HEX_LINE_LEN {
            return Err(PackagerError(format!(
                "line {line_num}: line too long ({} chars, maximum {MAX_HEX_LINE_LEN})",
                line.len()
            )));
        }
        if !line.starts_with(':') {
            return Err(PackagerError(format!(
                "line {line_num}: expected ':', got {:?}",
                &line[..line.len().min(1)]
            )));
        }

        // Parse the hex body (everything after the leading ':')
        let record = parse_hex_bytes(&line[1..])
            .map_err(|_| PackagerError(format!("line {line_num}: invalid hex data")))?;

        // Minimum: byte_count(1) + addr_hi(1) + addr_lo(1) + type(1) + checksum(1) = 5
        if record.len() < 5 {
            return Err(PackagerError(format!(
                "line {line_num}: record too short ({} bytes, need ≥5)",
                record.len()
            )));
        }

        let byte_count = record[0] as usize;
        let address = ((record[1] as usize) << 8) | record[2] as usize;
        let rec_type = record[3];

        // Validate that the record is long enough for byte_count data bytes + 1 checksum.
        // Without this check, a record claiming byte_count=255 with only a few bytes
        // would either panic on the checksum index or silently truncate data.
        let expected_len = 4 + byte_count + 1;
        if record.len() < expected_len {
            return Err(PackagerError(format!(
                "line {line_num}: record claims {byte_count} data bytes \
                 but only {} total bytes present (need {expected_len})",
                record.len()
            )));
        }

        // Verify checksum: the stored checksum is the last byte.
        let stored_cs = record[4 + byte_count];
        let computed_cs = checksum(&record[..4 + byte_count]);
        if computed_cs != stored_cs {
            return Err(PackagerError(format!(
                "line {line_num}: checksum mismatch \
                 (expected {computed_cs:#04x}, got {stored_cs:#04x})"
            )));
        }

        if rec_type == RECORD_TYPE_EOF {
            // Stop processing at the mandatory end-of-file sentinel.
            found_eof = true;
            break;
        }
        if rec_type != RECORD_TYPE_DATA {
            return Err(PackagerError(format!(
                "line {line_num}: unsupported record type {rec_type:#04x}"
            )));
        }

        // Check for overlapping or duplicate data records.
        //
        // Because segments is a BTreeMap, looking up the greatest key ≤ address
        // (via range(..=address).next_back()) gives us the record whose address
        // range might overlap with this one.  Two records overlap if the previous
        // record's end address (prev_addr + prev_data.len()) is greater than the
        // start address of the current record.
        //
        // Allowing overlaps would cause a later record to silently overwrite bytes
        // written by an earlier record during image assembly, making it trivially
        // easy for a crafted HEX file to target specific ROM offsets even though
        // each individual record has a valid checksum.
        if let Some((&prev_addr, prev_data)) = segments.range(..=address).next_back() {
            let prev_end = prev_addr + prev_data.len();
            if prev_end > address {
                return Err(PackagerError(format!(
                    "line {line_num}: record at {address:#06x} overlaps \
                     previous record (ends at {prev_end:#06x})"
                )));
            }
        }

        // Also check whether this record would overlap a *later* record already
        // inserted (possible if records arrive out of address order).  We look
        // up the first key that is strictly greater than `address` and verify
        // that the current record ends at or before that key.
        if let Some((&next_addr, _)) = segments.range((address + 1)..).next() {
            let current_end = address + byte_count;
            if current_end > next_addr {
                return Err(PackagerError(format!(
                    "line {line_num}: record at {address:#06x} (ends at {current_end:#06x}) \
                     overlaps next record at {next_addr:#06x}"
                )));
            }
        }

        // Store data segment keyed by address.
        let data = record[4..4 + byte_count].to_vec();
        segments.insert(address, data);
    }

    // The Intel HEX specification mandates that every file ends with an EOF
    // record (type 0x01).  A file without one is truncated or corrupt.
    if !found_eof {
        return Err(PackagerError(
            "missing EOF record (type 0x01) — file may be truncated".to_string(),
        ));
    }

    if segments.is_empty() {
        return Ok(DecodedHex {
            origin: 0,
            binary: vec![],
        });
    }

    // Compute the span of the image: from lowest to highest address+len.
    let origin = *segments.keys().next().unwrap();
    let end = segments
        .iter()
        .map(|(addr, data)| addr + data.len())
        .max()
        .unwrap_or(origin);

    // Guard against adversarial inputs claiming widely-separated addresses
    // (e.g. one record at 0x0000 and one at 0xFFFF) that would cause a
    // multi-megabyte allocation even if almost no data is present.
    // The 8008 address space is 16 KB (0x4000), so cap there.
    let span = end.saturating_sub(origin);
    if span > MAX_IMAGE_SIZE {
        return Err(PackagerError(format!(
            "decoded image too large: {span} bytes (maximum {MAX_IMAGE_SIZE} bytes \
             for Intel 8008 address space)"
        )));
    }

    // Assemble bytes in address order.
    let mut buffer = vec![0u8; span];
    for (addr, data) in &segments {
        let start = addr - origin;
        buffer[start..start + data.len()].copy_from_slice(data);
    }

    Ok(DecodedHex {
        origin,
        binary: buffer,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // checksum tests
    // ------------------------------------------------------------------

    /// The classic example from the Intel HEX specification.
    /// fields = [0x03, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03]
    /// sum = 9  →  checksum = 0xF7
    #[test]
    fn checksum_classic_example() {
        let fields = [0x03u8, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03];
        assert_eq!(checksum(&fields), 0xF7);
    }

    /// Summing all bytes of a record (including its checksum) must be 0 mod 256.
    #[test]
    fn checksum_verification_property() {
        let binary = vec![0x06u8, 0x00, 0xFF];
        let hex = encode_hex(&binary, 0).unwrap();
        for line in hex.lines() {
            if line == ":00000001FF" {
                continue;
            }
            let record_bytes = parse_hex_bytes(&line[1..]).unwrap();
            assert_eq!(
                record_bytes.iter().map(|b| *b as u32).sum::<u32>() % 256,
                0,
                "checksum verification failed for: {line}"
            );
        }
    }

    /// EOF record checksum is 0xFF (two's complement of 0x01).
    #[test]
    fn checksum_eof_record() {
        let fields = [0x00u8, 0x00, 0x00, 0x01]; // byte_count=0, addr=0, type=EOF
        assert_eq!(checksum(&fields), 0xFF);
    }

    // ------------------------------------------------------------------
    // encode_hex: format tests
    // ------------------------------------------------------------------

    #[test]
    fn encode_single_byte() {
        let hex = encode_hex(&[0xFFu8], 0).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with(':'));
        assert_eq!(lines[1], ":00000001FF");
    }

    #[test]
    fn encode_starts_with_colon() {
        let hex = encode_hex(&[0x01u8, 0x02, 0x03], 0).unwrap();
        for line in hex.lines() {
            assert!(line.starts_with(':'), "line does not start with ':': {line}");
        }
    }

    #[test]
    fn encode_eof_always_last() {
        let hex = encode_hex(&[0xAAu8, 0xBB], 0).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(*lines.last().unwrap(), ":00000001FF");
    }

    #[test]
    fn encode_three_byte_program_format() {
        // MVI B, 0; HLT → [0x06, 0x00, 0xFF]
        let hex = encode_hex(&[0x06u8, 0x00, 0xFF], 0).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        let data = lines[0];
        assert_eq!(&data[1..3], "03");   // byte count = 3
        assert_eq!(&data[3..7], "0000"); // address = 0x0000
        assert_eq!(&data[7..9], "00");   // record type = DATA
        assert_eq!(&data[9..15], "0600FF"); // data bytes
    }

    #[test]
    fn encode_16_bytes_one_record() {
        // Exactly 16 bytes → 1 data record + 1 EOF
        let binary: Vec<u8> = (0..16).collect();
        let hex = encode_hex(&binary, 0).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(&lines[0][1..3], "10"); // byte count = 16 = 0x10
    }

    #[test]
    fn encode_17_bytes_splits_into_two_records() {
        let binary: Vec<u8> = (0..17).collect();
        let hex = encode_hex(&binary, 0).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(lines.len(), 3); // 2 data + 1 EOF
        assert_eq!(&lines[0][1..3], "10"); // first: 16 bytes
        assert_eq!(&lines[1][1..3], "01"); // second: 1 byte
    }

    #[test]
    fn encode_32_bytes_two_records() {
        let binary: Vec<u8> = (0..32).collect();
        let hex = encode_hex(&binary, 0).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(lines.len(), 3); // 2 × 16-byte data + EOF
    }

    #[test]
    fn encode_address_increments_by_16() {
        // 32 bytes: first record at 0x0000, second at 0x0010
        let binary: Vec<u8> = vec![0u8; 32];
        let hex = encode_hex(&binary, 0).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(&lines[0][3..7], "0000"); // first record address
        assert_eq!(&lines[1][3..7], "0010"); // second record address = 0x10
    }

    #[test]
    fn encode_with_nonzero_origin() {
        // origin = 0x0100: first record address should be 0x0100
        let binary = vec![0x7Cu8, 0x03, 0x00, 0xFF];
        let hex = encode_hex(&binary, 0x0100).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(&lines[0][3..7], "0100");
    }

    #[test]
    fn encode_large_address() {
        // origin = 0x2000 (start of 8008 RAM region)
        let binary = vec![0x00u8; 4];
        let hex = encode_hex(&binary, 0x2000).unwrap();
        let lines: Vec<&str> = hex.lines().collect();
        assert_eq!(&lines[0][3..7], "2000");
    }

    // ------------------------------------------------------------------
    // encode_hex: error cases
    // ------------------------------------------------------------------

    #[test]
    fn encode_empty_binary_errors() {
        let result = encode_hex(&[], 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-empty"));
    }

    #[test]
    fn encode_origin_overflow_errors() {
        // origin = 0xFFFF, binary = 2 bytes → origin + len = 0x10001 > 0x10000
        let result = encode_hex(&[0x01u8, 0x02], 0xFFFF);
        assert!(result.is_err());
    }

    #[test]
    fn encode_origin_too_large_errors() {
        let result = encode_hex(&[0x01u8], 0x10000);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // decode_hex: round-trip tests
    // ------------------------------------------------------------------

    #[test]
    fn decode_round_trip_single_byte() {
        let binary = vec![0xFFu8];
        let decoded = decode_hex(&encode_hex(&binary, 0).unwrap()).unwrap();
        assert_eq!(decoded.origin, 0);
        assert_eq!(decoded.binary, binary);
    }

    #[test]
    fn decode_round_trip_three_bytes() {
        let binary = vec![0x06u8, 0x00, 0xFF];
        let decoded = decode_hex(&encode_hex(&binary, 0).unwrap()).unwrap();
        assert_eq!(decoded.origin, 0);
        assert_eq!(decoded.binary, binary);
    }

    #[test]
    fn decode_round_trip_17_bytes() {
        let binary: Vec<u8> = (0..17).collect();
        let decoded = decode_hex(&encode_hex(&binary, 0).unwrap()).unwrap();
        assert_eq!(decoded.binary, binary);
    }

    #[test]
    fn decode_round_trip_with_origin() {
        let binary = vec![0x7Cu8, 0x03, 0x00, 0xFF];
        let decoded = decode_hex(&encode_hex(&binary, 0x0100).unwrap()).unwrap();
        assert_eq!(decoded.origin, 0x0100);
        assert_eq!(decoded.binary, binary);
    }

    #[test]
    fn decode_round_trip_full_16kb() {
        // Full 8008 address space — should not trigger the size guard
        let binary = vec![0xFFu8; MAX_IMAGE_SIZE];
        let decoded = decode_hex(&encode_hex(&binary, 0).unwrap()).unwrap();
        assert_eq!(decoded.binary.len(), MAX_IMAGE_SIZE);
    }

    // ------------------------------------------------------------------
    // decode_hex: error cases
    // ------------------------------------------------------------------

    #[test]
    fn decode_missing_colon_errors() {
        let result = decode_hex("03000000060000F7\n:00000001FF\n");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("':'"));
    }

    #[test]
    fn decode_invalid_hex_errors() {
        // Non-hex character in data
        let result = decode_hex(":0ZZZZ000060000F7\n:00000001FF\n");
        assert!(result.is_err());
    }

    #[test]
    fn decode_bad_checksum_errors() {
        // Encode valid data, then corrupt the checksum byte
        let binary = vec![0x01u8, 0x02, 0x03];
        let mut hex = encode_hex(&binary, 0).unwrap();
        // Last 2 chars before \n in first line are the checksum — corrupt them
        let first_newline = hex.find('\n').unwrap();
        let cs_start = first_newline - 2;
        hex.replace_range(cs_start..first_newline, "00");
        let result = decode_hex(&hex);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("checksum"));
    }

    #[test]
    fn decode_unsupported_record_type_errors() {
        // Type 0x02 = Extended Segment Address — not supported.
        // Record: :020000020000FC
        //   byte_count=0x02, addr=0x0000, type=0x02, data=[0x00,0x00]
        //   fields=[0x02,0x00,0x00,0x02,0x00,0x00] → sum=4 → cs=(256-4)=0xFC
        let invalid = ":020000020000FC\n:00000001FF\n";
        let result = decode_hex(invalid);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsupported"));
    }

    #[test]
    fn decode_record_too_short_errors() {
        // ":05" claims 5 data bytes but the record has fewer bytes total
        let invalid = ":050000000102\n:00000001FF\n";
        let result = decode_hex(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn decode_image_too_large_errors() {
        // Construct two records at widely-separated addresses to trigger the cap
        // Record at 0x0000 with 1 byte, record at 0x3FFF with 1 byte
        // This creates a 0x4000-byte span — exactly at the limit, so we need
        // an address that pushes us over.
        // Use record at 0x0000 (1 byte) and fake record at 0x4000 (1 byte, span = 16385)
        // We craft this manually so we don't accidentally test the 16-bit address overflow.
        // Manually build a record at address 0x4001 (span would be 0x4002):
        //   :01 4001 00 FF CS
        //   byte_count=1, addr=0x4001, type=0x00, data=0xFF
        //   fields = [0x01, 0x40, 0x01, 0x00, 0xFF] → sum = 0x141 → cs = (0x100 - 0x41) = 0xBF
        let record_start = ":010000000001\n";  // won't use this
        // Build valid record at 0x0000 (1 byte, value=0x00):
        // fields=[0x01,0x00,0x00,0x00,0x00], sum=0x01, cs=0xFF
        let r1 = ":0100000000FF\n";
        // Record at 0x4001 (1 byte, value=0xFF):
        // fields=[0x01,0x40,0x01,0x00,0xFF], sum=0x141, 0x141%0x100=0x41, cs=(0x100-0x41)%0x100=0xBF
        let r2 = ":01400100FFBF\n";
        let _ = record_start;
        let hex = format!("{r1}{r2}:00000001FF\n");
        let result = decode_hex(&hex);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("large"));
    }

    #[test]
    fn decode_empty_input_returns_empty() {
        // An EOF-only file (no data records) decodes to empty binary.
        let decoded = decode_hex(":00000001FF\n").unwrap();
        assert_eq!(decoded.origin, 0);
        assert!(decoded.binary.is_empty());
    }

    #[test]
    fn decode_missing_eof_errors() {
        // A file that ends without the type-0x01 EOF record is treated as truncated.
        //
        // Craft a valid 1-byte data record at 0x0000 (value=0x00):
        //   fields = [0x01, 0x00, 0x00, 0x00, 0x00]  sum = 0x01 → cs = 0xFF
        //   line = ":0100000000FF"
        let no_eof = ":0100000000FF\n";
        let result = decode_hex(no_eof);
        assert!(result.is_err(), "expected error for missing EOF record");
        assert!(result.unwrap_err().to_string().contains("EOF"));
    }

    #[test]
    fn decode_overlapping_records_errors() {
        // Record A: 16 bytes starting at 0x0000 (fills bytes 0x0000–0x000F).
        // Record B:  1 byte  starting at 0x0005 (byte 0x0005 is inside A).
        //
        // Both records have valid individual checksums, but together they
        // describe overlapping regions — the second record would silently
        // corrupt the assembled image.
        //
        // Record B: :01 0005 00 00 CS
        //   fields = [0x01, 0x00, 0x05, 0x00, 0x00]  sum = 0x06 → cs = 0xFA
        //   line = ":0100050000FA"
        let binary_a: Vec<u8> = vec![0u8; 16];
        let hex_a = encode_hex(&binary_a, 0x0000).unwrap();
        let data_record_a = hex_a.lines().next().unwrap(); // the 16-byte record at 0x0000
        let record_b = ":0100050000FA"; // 1 byte at 0x0005, overlaps record A
        let combined = format!("{data_record_a}\n{record_b}\n:00000001FF\n");
        let result = decode_hex(&combined);
        assert!(result.is_err(), "expected error for overlapping records");
        assert!(
            result.unwrap_err().to_string().contains("overlap"),
            "error message should mention overlap"
        );
    }

    #[test]
    fn decode_duplicate_address_errors() {
        // Two records at the exact same address should be rejected as overlapping
        // (the second record starts at the same address as the first — a zero-offset
        // overlap that would silently discard the first record's data).
        //
        // Record at 0x0000, 1 byte = 0x42:
        //   fields = [0x01, 0x00, 0x00, 0x00, 0x42]  sum = 0x43 → cs = 0xBD
        //   line = ":0100000042BD"
        let dup_record = ":0100000042BD";
        let combined = format!("{dup_record}\n{dup_record}\n:00000001FF\n");
        let result = decode_hex(&combined);
        assert!(result.is_err(), "expected error for duplicate address records");
    }

    #[test]
    fn decode_out_of_order_overlapping_records_errors() {
        // Records delivered in *descending* address order: B (0x0005) before A (0x0000).
        // The backward overlap check cannot catch this case because when B is processed
        // the BTreeMap is empty — B is inserted first with no prior record to compare
        // against.  When A (16 bytes at 0x0000, ending at 0x0010) is processed next,
        // the *forward* overlap check finds B already at 0x0005 and rejects.
        //
        // Record B: 1 byte at 0x0005, value=0x00
        //   fields = [0x01, 0x00, 0x05, 0x00, 0x00]  sum=0x06 → cs=(0x100-0x06)=0xFA
        let record_b = ":0100050000FA";

        // Record A: 16 bytes at 0x0000, built via encode_hex for a correct checksum.
        // address=0x0000, byte_count=16, current_end=0x0010.
        // Forward check: next key > 0x0000 is 0x0005; 0x0010 > 0x0005 → overlap error.
        let binary_a: Vec<u8> = vec![0u8; 16];
        let hex_a = encode_hex(&binary_a, 0x0000).unwrap();
        let data_record_a = hex_a.lines().next().unwrap(); // the 16-byte record at 0x0000

        let combined = format!("{record_b}\n{data_record_a}\n:00000001FF\n");
        let result = decode_hex(&combined);
        assert!(result.is_err(), "expected error for out-of-order overlapping records");
        assert!(
            result.unwrap_err().to_string().contains("overlap"),
            "error message should mention overlap"
        );
    }

    #[test]
    fn decode_line_too_long_errors() {
        // Craft a line that exceeds MAX_HEX_LINE_LEN (1024 chars).
        // We don't need a valid record — just a very long line starting with ':'.
        let long_line = format!(":{}", "AA".repeat(600)); // 1 + 1200 = 1201 chars
        let input = format!("{long_line}\n:00000001FF\n");
        let result = decode_hex(&input);
        assert!(result.is_err(), "expected error for line exceeding MAX_HEX_LINE_LEN");
        assert!(
            result.unwrap_err().to_string().contains("long"),
            "error message should mention line length"
        );
    }

    // ------------------------------------------------------------------
    // PackagerError display
    // ------------------------------------------------------------------

    #[test]
    fn packager_error_display() {
        let e = PackagerError("test error".to_string());
        assert_eq!(e.to_string(), "test error");
    }

    // ------------------------------------------------------------------
    // parse_hex_bytes
    // ------------------------------------------------------------------

    #[test]
    fn parse_hex_bytes_roundtrip() {
        let bytes = [0x03u8, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0xF7];
        let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
        assert_eq!(parse_hex_bytes(&hex).unwrap(), bytes);
    }

    #[test]
    fn parse_hex_bytes_odd_length_errors() {
        assert!(parse_hex_bytes("ABC").is_err());
    }

    #[test]
    fn parse_hex_bytes_non_hex_errors() {
        assert!(parse_hex_bytes("GG").is_err());
    }

    // ------------------------------------------------------------------
    // 8008-specific: addresses > 0xFF (multi-byte address)
    // ------------------------------------------------------------------

    #[test]
    fn encode_decode_address_at_0x3ff0() {
        // Near top of 8008 address space (0x3FF0 + 16 = 0x4000, exactly at max)
        let binary = vec![0xFFu8; 16];
        let hex = encode_hex(&binary, 0x3FF0).unwrap();
        let decoded = decode_hex(&hex).unwrap();
        assert_eq!(decoded.origin, 0x3FF0);
        assert_eq!(decoded.binary, binary);
    }
}
