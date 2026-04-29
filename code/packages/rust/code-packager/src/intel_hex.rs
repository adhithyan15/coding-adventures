//! Intel HEX packager.
//!
//! Encodes binary data as Intel HEX — a text format that represents binary
//! data as ASCII hexadecimal records. It was invented by Intel in 1973 for
//! programming early microprocessors like the 4004 and 8008, and is still
//! widely used for programming microcontrollers, EPROMs, and FPGAs today.
//!
//! ## Why text, not binary?
//!
//! In 1973, the tools for transferring binary data between computers were
//! unreliable. ASCII text could be transferred over serial links (RS-232),
//! printed on paper tape, and punched onto cards without corruption. Intel
//! HEX encodes binary data as printable hex digits, making it completely
//! safe for these channels.
//!
//! ## Record format
//!
//! Each record (line) has this structure:
//!
//! ```text
//! :LLAAAATTDD...CC\n
//!  │ │   │ │      └─ Checksum (2 hex digits)
//!  │ │   │ └──────── Data bytes (2 hex digits each)
//!  │ │   └────────── Record type (2 hex digits)
//!  │ └────────────── Start address (4 hex digits, big-endian)
//!  └──────────────── Byte count (2 hex digits)
//! ^ Colon: record start marker
//! ```
//!
//! ## Record types
//!
//! ```text
//! Type │ Meaning
//! ─────┼────────────────────────────────────────────────────────────────────
//!  00  │ Data record (the actual bytes)
//!  01  │ End-of-file record (terminates the file)
//!  02  │ Extended segment address (for 80286 real-mode segmentation)
//!  03  │ Start segment address (CS:IP for 8086)
//!  04  │ Extended linear address (high 16 bits of 32-bit address)
//!  05  │ Start linear address (32-bit EIP for 80386)
//! ```
//!
//! This packager emits only types 00 (data) and 01 (EOF).
//!
//! ## Checksum algorithm
//!
//! The checksum is the **two's complement** of the sum of all bytes in the
//! record from `:LL` through the last data byte.
//!
//! ```text
//! record bytes = [LL, AAAA_hi, AAAA_lo, TT, DD...]
//! sum = sum of all record bytes mod 256
//! checksum = (256 - sum) mod 256  =  (!sum + 1) & 0xFF
//! ```
//!
//! A reader can verify a record by summing all bytes including the checksum;
//! the result must be 0 (mod 256).
//!
//! ## Data record layout (16 bytes per record)
//!
//! ```text
//! :10 0000 00 AABBCCDD EEFFGGHH IIJJKKLL MMNNOOPP XX\n
//!  │  │    │  └─ 16 data bytes                    └─ checksum
//!  │  │    └──── record type 00
//!  │  └─────────  address (big-endian, 16-bit)
//!  └────────────  byte count = 0x10 = 16
//! ```
//!
//! ## EOF record
//!
//! ```text
//! :00 0000 01 FF\n
//!  │  │    │   └─ checksum of [00, 00, 00, 01] = 0xFF
//!  │  │    └──── record type 01
//!  │  └─────────  address = 0 (unused in EOF)
//!  └────────────  byte count = 0 (no data)
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;

/// Maximum data bytes per record.
/// 16 is the de-facto standard; some tools use 32, but 16 is safest.
const BYTES_PER_RECORD: usize = 16;

/// Record type 00: data.
const RECORD_DATA: u8 = 0x00;
/// Record type 01: end of file.
const RECORD_EOF: u8 = 0x01;

// ── Core encoder ──────────────────────────────────────────────────────────────

/// Compute the Intel HEX checksum for one record's worth of bytes.
///
/// The checksum is appended to each record so that readers can detect
/// transmission errors.
///
/// ## Algorithm
///
/// ```text
/// Step 1: sum all bytes (byte count, address hi, address lo, type, data…)
/// Step 2: take the two's complement = (256 - sum) mod 256
/// ```
///
/// A reader sums all bytes *including* the checksum; the result is 0 if valid.
fn checksum(byte_count: u8, addr: u16, record_type: u8, data: &[u8]) -> u8 {
    let mut sum: u32 = 0;
    sum += byte_count as u32;
    sum += (addr >> 8) as u32;   // high byte of the address
    sum += (addr & 0xFF) as u32; // low byte of the address
    sum += record_type as u32;
    for b in data {
        sum += *b as u32;
    }
    // Two's complement (mod 256): negate the sum.
    ((!sum).wrapping_add(1) & 0xFF) as u8
}

/// Encode a single Intel HEX record as an ASCII string (including trailing `\n`).
///
/// # Arguments
///
/// * `addr` — 16-bit start address for this record.
/// * `record_type` — 0x00 for data, 0x01 for EOF.
/// * `data` — the bytes to encode (may be empty for EOF).
fn encode_record(addr: u16, record_type: u8, data: &[u8]) -> String {
    let byte_count = data.len() as u8;
    let cc = checksum(byte_count, addr, record_type, data);

    // Format: ":LLAAAATTDD...CC\n"
    // Pre-build the hex string piece by piece.
    let mut record = String::with_capacity(1 + 2 + 4 + 2 + data.len() * 2 + 2 + 1);
    record.push(':');
    record.push_str(&format!("{byte_count:02X}"));
    record.push_str(&format!("{addr:04X}"));
    record.push_str(&format!("{record_type:02X}"));
    for b in data {
        record.push_str(&format!("{b:02X}"));
    }
    record.push_str(&format!("{cc:02X}"));
    record.push('\n');
    record
}

/// Encode binary `data` as a complete Intel HEX file.
///
/// # Arguments
///
/// * `data` — the raw bytes to encode.
/// * `origin` — the 16-bit start address.  Each record's address is
///   `origin + byte_offset_into_data`.  If `origin + data.len() > 0xFFFF`
///   the addresses wrap around (Intel HEX 8-bit mode behaviour).
///
/// # Returns
///
/// A `String` containing the complete Intel HEX file, ready to write to disk.
///
/// # Example
///
/// ```text
/// encode_intel_hex(&[0x01, 0x02, 0x03], 0x0000)
///
/// :03000000010203F9\n
/// :00000001FF\n
/// ```
pub fn encode_intel_hex(data: &[u8], origin: u16) -> String {
    let mut output = String::new();

    // Split data into chunks of up to BYTES_PER_RECORD bytes.
    for (chunk_idx, chunk) in data.chunks(BYTES_PER_RECORD).enumerate() {
        // Address = origin + number of bytes already emitted.
        let addr = origin.wrapping_add((chunk_idx * BYTES_PER_RECORD) as u16);
        output.push_str(&encode_record(addr, RECORD_DATA, chunk));
    }

    // Emit the EOF record.
    output.push_str(&encode_record(0x0000, RECORD_EOF, &[]));

    output
}

/// Pack `artifact` into an Intel HEX file.
///
/// The `origin` address is read from `artifact.metadata_int("origin", 0)`
/// and cast to a `u16`.
///
/// # Errors
///
/// Returns `PackagerError::UnsupportedTarget` if `binary_format != "intel_hex"`.
pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    if artifact.target.binary_format != "intel_hex" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "intel_hex packager does not handle binary_format={:?}",
            artifact.target.binary_format
        )));
    }

    let origin = artifact.metadata_int("origin", 0) as u16;
    let hex = encode_intel_hex(&artifact.native_bytes, origin);
    Ok(hex.into_bytes())
}

/// The conventional file extension for Intel HEX files.
pub fn file_extension() -> &'static str {
    ".hex"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::Target;

    // Test 1: Known short input — 3 bytes at origin 0
    //
    // Record analysis:
    //   LL = 03 (3 data bytes)
    //   AAAA = 0000 (address = 0)
    //   TT = 00 (data record)
    //   DD = 01 02 03
    //   sum = 03 + 00 + 00 + 00 + 01 + 02 + 03 = 09
    //   checksum = (256 - 9) & 0xFF = 247 = 0xF7
    #[test]
    fn known_short_input() {
        let hex = encode_intel_hex(&[0x01, 0x02, 0x03], 0x0000);
        assert_eq!(hex, ":03000000010203F7\n:00000001FF\n");
    }

    // Test 2: EOF checksum is always 0xFF when data is empty
    #[test]
    fn eof_record_checksum() {
        // :00 0000 01 FF
        // sum = 0 + 0 + 0 + 1 = 1 → checksum = 256 - 1 = 255 = 0xFF
        let eof = encode_record(0x0000, RECORD_EOF, &[]);
        assert_eq!(eof, ":00000001FF\n");
    }

    // Test 3: Checksum verification — sum of all record bytes (including checksum) = 0
    #[test]
    fn checksum_verifies_to_zero() {
        let data = [0xAA, 0xBB, 0xCC];
        let cc = checksum(3, 0x0010, RECORD_DATA, &data);
        // Sum all record bytes including the checksum byte:
        let sum: u32 = 3 + 0 + 0x10 + 0 + 0xAA + 0xBB + 0xCC + cc as u32;
        assert_eq!(sum & 0xFF, 0, "checksum verification failed: sum mod 256 = {}", sum & 0xFF);
    }

    // Test 4: Each record header starts with ':' and ends with '\n'
    #[test]
    fn record_format_delimiters() {
        let hex = encode_intel_hex(&[0xFF], 0x0000);
        for line in hex.lines() {
            assert!(line.starts_with(':'), "line does not start with ':': {line:?}");
        }
        // Original string ends with '\n'
        assert!(hex.ends_with('\n'));
    }

    // Test 5: 16 bytes fit in a single data record (+ EOF)
    #[test]
    fn sixteen_bytes_one_record() {
        let data: Vec<u8> = (0..16).collect();
        let hex = encode_intel_hex(&data, 0x0000);
        let lines: Vec<&str> = hex.lines().collect();
        // 1 data record + 1 EOF record = 2 lines
        assert_eq!(lines.len(), 2);
        // First line: byte count = 10 (hex for 16)
        assert!(lines[0].starts_with(":10"), "expected :10... got {}", lines[0]);
    }

    // Test 6: 17 bytes produce two data records + EOF
    #[test]
    fn seventeen_bytes_two_records() {
        let data: Vec<u8> = (0..17).collect();
        let hex = encode_intel_hex(&data, 0x0000);
        let lines: Vec<&str> = hex.lines().collect();
        // 2 data records + 1 EOF
        assert_eq!(lines.len(), 3);
    }

    // Test 7: Origin address is encoded correctly in the second record
    #[test]
    fn origin_address_in_second_record() {
        // 17 bytes → 2 data records: first at origin, second at origin + 16
        let data: Vec<u8> = vec![0x00; 17];
        let hex = encode_intel_hex(&data, 0x0100);
        let lines: Vec<&str> = hex.lines().collect();
        // First record address = 0100
        assert!(lines[0].starts_with(":100100"), "got: {}", lines[0]);
        // Second record address = 0110 (0x0100 + 16)
        assert!(lines[1].starts_with(":010110"), "got: {}", lines[1]);
    }

    // Test 8: pack() with intel_4004 target succeeds
    #[test]
    fn pack_intel_4004_succeeds() {
        let art = CodeArtifact::new(vec![0x00, 0xFF], 0, Target::intel_4004());
        let bytes = pack(&art).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains(':'));
        assert!(text.ends_with('\n'));
    }

    // Test 9: pack() rejects elf64 target
    #[test]
    fn pack_rejects_elf64() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::linux_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 10: empty input produces only EOF record
    #[test]
    fn empty_input_eof_only() {
        let hex = encode_intel_hex(&[], 0x0000);
        assert_eq!(hex, ":00000001FF\n");
    }

    // Test 11: file_extension
    #[test]
    fn file_extension_is_hex() {
        assert_eq!(file_extension(), ".hex");
    }
}
