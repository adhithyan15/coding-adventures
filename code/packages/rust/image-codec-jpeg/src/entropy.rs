// # entropy.rs — JPEG Huffman entropy coding
//
// Entropy coding is the final compression step in JPEG. By this point we have
// 64 quantized DCT coefficients per block. Entropy coding encodes those integers
// into the fewest possible bits by assigning shorter codes to values that appear
// more often (Huffman coding).
//
// ## JPEG's specific entropy coding format
//
// JPEG baseline uses two kinds of Huffman codes per colour component:
//
// 1. **DC codes**: encode the difference between the current block's DC
//    coefficient (position 0 in the zigzag) and the previous block's DC
//    coefficient. This exploits the fact that adjacent blocks tend to have
//    similar brightness/colour averages — the difference is usually small.
//
// 2. **AC codes**: encode positions 1–63 of the zigzag-ordered block using
//    a run-length encoding scheme. A Huffman symbol encodes (run_of_zeros, size)
//    pairs, followed by the actual coefficient value if size > 0.
//
// ## Bit order: MSB first
//
// JPEG uses MSB-first bit packing — the most significant bit of each code word
// goes into the output stream first. This is the OPPOSITE of DEFLATE/ZIP, which
// use LSB-first packing. Be careful not to confuse them.
//
// ## Byte stuffing
//
// JPEG uses 0xFF as a marker byte (to delimit segments). In the entropy-coded
// data stream, if we happen to output 0xFF as a data byte, we must immediately
// follow it with 0x00. Decoders skip the 0x00; they see only 0xFF as data.
// This is called "byte stuffing".
//
// ## Standard Huffman tables (Annex K of ITU-T T.81)
//
// JPEG mandates that baseline files use one of the standard Huffman table sets
// or embed their own. We always use the standard Annex K tables — they are
// pre-computed to be good for typical photographic content. Using these tables
// means we don't need to analyse the image to build optimal tables, saving time
// and simplifying the format (the decoder can use the same built-in tables).
//
// The tables are specified as:
//   BITS[1..16] — how many codes exist at each code length (1 to 16 bits)
//   HUFFVAL     — the symbol values, in the order their codes are assigned
//
// From BITS + HUFFVAL, we reconstruct the canonical Huffman code assignments.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Standard Annex K Huffman table data
// ---------------------------------------------------------------------------
//
// These are the exact BITS and HUFFVAL arrays from Table K.3 (Luma DC),
// Table K.4 (Luma AC), Table K.5 (Chroma DC), Table K.6 (Chroma AC) of
// ITU-T T.81 (JPEG standard).
//
// BITS: 16 bytes, one per code length. BITS[0] = count of 1-bit codes,
//       BITS[1] = count of 2-bit codes, ..., BITS[15] = count of 16-bit codes.
//
// HUFFVAL: the symbols in ascending code-length order.

/// Luma DC BITS: counts by code length 1..16.
pub const LUMA_DC_BITS: [u8; 16] = [
    0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
];
/// Luma DC symbols (12 categories: 0–11).
pub const LUMA_DC_HUFFVAL: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
];

/// Luma AC BITS: counts by code length 1..16.
pub const LUMA_AC_BITS: [u8; 16] = [
    0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125,
];
/// Luma AC symbols (162 symbols from Annex K Table K.5).
///
/// Each byte encodes (run_of_zeros << 4) | size. Special symbols:
///   0x00 = EOB (End of Block — all remaining AC coefficients are zero)
///   0xF0 = ZRL (Zero Run Length — 16 zeros in a row)
pub const LUMA_AC_HUFFVAL: &[u8] = &[
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12,
    0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08,
    0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16,
    0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
    0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
    0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79,
    0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98,
    0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
    0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
    0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4,
    0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea,
    0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

/// Chroma DC BITS: counts by code length 1..16.
pub const CHROMA_DC_BITS: [u8; 16] = [
    0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
];
/// Chroma DC symbols (12 categories: 0–11).
pub const CHROMA_DC_HUFFVAL: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
];

/// Chroma AC BITS: counts by code length 1..16.
pub const CHROMA_AC_BITS: [u8; 16] = [
    0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 119,
];
/// Chroma AC symbols (162 symbols from Annex K Table K.6).
pub const CHROMA_AC_HUFFVAL: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21,
    0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
    0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91,
    0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
    0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34,
    0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
    0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38,
    0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
    0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
    0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96,
    0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
    0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4,
    0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
    0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2,
    0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
    0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9,
    0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

// ---------------------------------------------------------------------------
// Canonical Huffman code builder
// ---------------------------------------------------------------------------

/// Build canonical Huffman code assignments from BITS + HUFFVAL arrays.
///
/// Returns a `Vec` of `(symbol, code, code_length)` tuples.
///
/// ## Canonical Huffman algorithm
///
/// Canonical codes are assigned in order: first all 1-bit codes (if any),
/// then all 2-bit codes, etc. Within each length, codes are assigned
/// numerically. The assignment rule is:
///
/// ```text
/// code = 0
/// for length in 1..=16:
///     for each symbol in HUFFVAL at this length:
///         assign (symbol, code, length)
///         code += 1
///     code <<= 1  // left-shift when moving to next length
/// ```
///
/// This produces the unique canonical Huffman code set for the given BITS table.
pub fn build_huffman_codes(bits: &[u8; 16], huffval: &[u8]) -> Vec<(u8, u16, u8)> {
    let mut codes = Vec::new();
    let mut code: u16 = 0;
    let mut huffval_idx = 0;

    for length in 1u8..=16 {
        let count = bits[(length - 1) as usize] as usize;
        for _ in 0..count {
            if huffval_idx < huffval.len() {
                let symbol = huffval[huffval_idx];
                codes.push((symbol, code, length));
                huffval_idx += 1;
            }
            code = code.wrapping_add(1);
        }
        // Shift left to move to the next code length. This preserves the prefix-free
        // property: no code of length n is a prefix of any code of length n+1.
        code <<= 1;
    }
    codes
}

/// Build a decode lookup table from BITS + HUFFVAL.
///
/// Returns `HashMap<(code_value, code_length), symbol>`.
///
/// During decoding, we read bits one at a time and check if the accumulated
/// bits match any code of that length. This table makes that lookup O(1).
pub fn build_decode_table(bits: &[u8; 16], huffval: &[u8]) -> HashMap<(u16, u8), u8> {
    let codes = build_huffman_codes(bits, huffval);
    let mut table = HashMap::with_capacity(codes.len());
    for (symbol, code, length) in codes {
        table.insert((code, length), symbol);
    }
    table
}

// ---------------------------------------------------------------------------
// MSB-first bit writer with 0xFF byte stuffing
// ---------------------------------------------------------------------------

/// JPEG bit writer: packs bits MSB-first into bytes, with 0xFF byte stuffing.
///
/// ## MSB-first packing
///
/// For a 3-bit code `101` followed by a 5-bit code `00110`, the output byte
/// would be `10100110`. The most significant (leftmost) bits come first.
///
/// ## 0xFF stuffing
///
/// If a completed byte equals 0xFF, we immediately write 0x00 after it. This
/// prevents the decoder from misinterpreting data bytes as JPEG markers (which
/// all begin with 0xFF). The decoder knows to discard the 0x00 after any 0xFF.
pub struct BitWriter {
    buf: Vec<u8>,
    /// The byte currently being assembled (bits filled from the MSB side).
    current_byte: u8,
    /// How many bits have been written into `current_byte` so far.
    bits_filled: u8,
}

impl BitWriter {
    /// Create a new empty BitWriter.
    pub fn new() -> Self {
        Self { buf: Vec::new(), current_byte: 0, bits_filled: 0 }
    }

    /// Write `n_bits` bits from `value`, MSB of `value` first.
    ///
    /// For example, `write_bits(0b101, 3)` writes bits `1`, `0`, `1`.
    pub fn write_bits(&mut self, value: u32, n_bits: u8) {
        // Process each bit from the most significant to least significant.
        for i in (0..n_bits).rev() {
            let bit = (value >> i) & 1;
            // Place the bit at the current MSB-first position.
            self.current_byte |= (bit as u8) << (7 - self.bits_filled);
            self.bits_filled += 1;
            if self.bits_filled == 8 {
                self.push_byte(self.current_byte);
                self.current_byte = 0;
                self.bits_filled = 0;
            }
        }
    }

    /// Flush the partial byte by padding with 1-bits to reach a byte boundary.
    ///
    /// The JPEG standard says the entropy-coded segment must be padded to a byte
    /// boundary with 1-bits (0xFF padding). This is called at the end of the scan.
    pub fn flush(&mut self) {
        if self.bits_filled > 0 {
            // Pad remaining bits with 1-bits.
            let remaining = 8 - self.bits_filled;
            let padding = (1u8 << remaining) - 1; // e.g., 3 bits remain → 0b111
            self.current_byte |= padding;
            self.push_byte(self.current_byte);
            self.current_byte = 0;
            self.bits_filled = 0;
        }
    }

    /// Push a completed byte to the buffer, applying 0xFF byte stuffing.
    fn push_byte(&mut self, byte: u8) {
        self.buf.push(byte);
        if byte == 0xFF {
            // Byte stuffing: always follow 0xFF with 0x00 in the entropy stream.
            self.buf.push(0x00);
        }
    }

    /// Consume the writer and return the byte buffer.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

// ---------------------------------------------------------------------------
// MSB-first bit reader with 0xFF un-stuffing
// ---------------------------------------------------------------------------

/// JPEG bit reader: reads bits MSB-first, un-stuffing 0xFF 0x00 sequences.
pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    /// The current byte being consumed.
    current_byte: u8,
    /// How many bits remain available in `current_byte`.
    bits_left: u8,
}

impl<'a> BitReader<'a> {
    /// Create a new BitReader over the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, current_byte: 0, bits_left: 0 }
    }

    /// Read the next byte from the stream, un-stuffing 0xFF 0x00 sequences.
    ///
    /// If the byte is 0xFF, peek at the next byte. If it's 0x00, discard the
    /// 0x00 (it's a stuffed byte — the data byte is 0xFF). If it's non-zero,
    /// it's a marker byte; we return an error.
    fn next_byte(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err("BitReader: unexpected end of entropy-coded data".to_string());
        }
        let byte = self.data[self.pos];
        self.pos += 1;
        if byte == 0xFF {
            if self.pos >= self.data.len() {
                return Err("BitReader: 0xFF at end of data with no following byte".to_string());
            }
            let next = self.data[self.pos];
            if next == 0x00 {
                // Stuffed 0xFF data byte — consume the 0x00 and return 0xFF.
                self.pos += 1;
            } else {
                // Non-zero next byte after 0xFF is a JPEG marker, not data.
                // Step back so the caller can see the marker.
                self.pos -= 1;
                return Err(format!("BitReader: hit marker FF {:02X} in entropy stream", next));
            }
        }
        Ok(byte)
    }

    /// Read `n` bits from the stream, MSB first.
    ///
    /// Fills the internal byte buffer as needed, then extracts bits from the top.
    pub fn read_bits(&mut self, n: u8) -> Result<u32, String> {
        let mut result = 0u32;
        for _ in 0..n {
            if self.bits_left == 0 {
                self.current_byte = self.next_byte()?;
                self.bits_left = 8;
            }
            // Extract the MSB of the current byte.
            let bit = (self.current_byte >> 7) & 1;
            self.current_byte <<= 1;
            self.bits_left -= 1;
            result = (result << 1) | (bit as u32);
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// DC coefficient encoding / decoding
// ---------------------------------------------------------------------------

/// Compute the "category" (also called "size" or "ssss") of a DC difference.
///
/// In JPEG, the DC difference is encoded as two parts:
///   1. A Huffman code for the number of bits needed (the "category")
///   2. The actual value bits (the "amplitude")
///
/// The category is the minimum number of bits needed to represent the magnitude
/// of the difference value. `category(0) = 0`, `category(±1) = 1`,
/// `category(±2..±3) = 2`, `category(±4..±7) = 3`, ..., up to 11.
///
/// # Examples
///
/// ```
/// // dc_category(0) == 0 (no bits needed)
/// // dc_category(1) == 1 (one bit: "1")
/// // dc_category(-1) == 1 (one bit: "0", per JPEG sign convention)
/// // dc_category(127) == 7
/// ```
pub fn dc_category(diff: i16) -> u8 {
    let abs = diff.unsigned_abs() as u32;
    if abs == 0 {
        0
    } else {
        // Number of bits needed to represent abs in binary.
        // e.g., abs=1 → 1 bit, abs=2 → 2 bits, abs=3 → 2 bits, abs=4 → 3 bits.
        (32 - abs.leading_zeros()) as u8
    }
}

/// Encode the DC difference coefficient to the bit writer.
///
/// Format: Huffman(category) followed by `category` raw amplitude bits.
///
/// The amplitude bits encode the value:
///   positive value v: write v in binary (category bits)
///   negative value v: write (v - 1) in binary (category bits, two's-complement-like)
///
/// This encoding ensures that 0 is never written as an amplitude (it only
/// appears as a category-0 with no following bits), and positive/negative
/// values have distinct bit patterns.
pub fn encode_dc(
    writer: &mut BitWriter,
    diff: i16,
    codes: &[(u8, u16, u8)],
) -> Result<(), String> {
    let category = dc_category(diff);

    // Look up and write the Huffman code for the category.
    let (_, code, code_len) = codes
        .iter()
        .find(|(sym, _, _)| *sym == category)
        .ok_or_else(|| format!("DC: no Huffman code for category {category}"))?;
    writer.write_bits(*code as u32, *code_len);

    // Write the amplitude bits (if category > 0).
    if category > 0 {
        let amplitude = if diff > 0 {
            diff as u32
        } else {
            // For negative values: write (diff - 1) in unsigned form.
            // e.g., diff=-1 → amplitude=0b0 (1 bit), diff=-2 → amplitude=0b01 (2 bits).
            (diff - 1) as u16 as u32 & ((1u32 << category) - 1)
        };
        writer.write_bits(amplitude, category);
    }

    Ok(())
}

/// Decode a DC coefficient from the bit reader.
///
/// Returns the coefficient difference (to be added to prev_dc).
pub fn decode_dc(
    reader: &mut BitReader,
    decode_table: &HashMap<(u16, u8), u8>,
) -> Result<i16, String> {
    // Read bits one at a time, checking if we have a valid code at each length.
    let mut code: u16 = 0;
    for len in 1u8..=16 {
        let bit = reader.read_bits(1)? as u16;
        code = (code << 1) | bit;
        if let Some(&category) = decode_table.get(&(code, len)) {
            if category == 0 {
                return Ok(0);
            }
            // Read `category` amplitude bits.
            let raw = reader.read_bits(category)? as u16;
            return Ok(amplitude_to_signed(raw, category));
        }
    }
    Err("DC decode: no valid Huffman code found".to_string())
}

// ---------------------------------------------------------------------------
// AC coefficient encoding / decoding
// ---------------------------------------------------------------------------

/// Encode 63 AC coefficients (positions 1–63 in zigzag order).
///
/// JPEG AC encoding uses run-length + Huffman encoding:
/// - Each non-zero coefficient is preceded by a count of zero coefficients.
/// - A Huffman symbol byte encodes (run_of_zeros << 4) | category.
/// - Special symbols:
///   - EOB (0x00): End Of Block — remaining coefficients are all zero.
///   - ZRL (0xF0): 16 zeros — used when there are more than 15 zeros in a row.
pub fn encode_ac(
    writer: &mut BitWriter,
    coeffs: &[i16; 63],
    codes: &[(u8, u16, u8)],
) -> Result<(), String> {
    let mut zero_run = 0u8;

    // Find the last non-zero coefficient. Everything after it is encoded with EOB.
    let last_nonzero = coeffs.iter().rposition(|&c| c != 0);

    for (i, &coeff) in coeffs.iter().enumerate() {
        if coeff == 0 {
            zero_run += 1;
            // After 16 zeros, we must emit ZRL (0xF0) and reset the counter.
            if zero_run == 16 {
                encode_ac_symbol(writer, 0xF0, codes)?;
                zero_run = 0;
            }
        } else {
            // Non-zero coefficient: encode (run, category) symbol + amplitude.
            let category = dc_category(coeff); // same formula works for AC
            let symbol = (zero_run << 4) | category;
            encode_ac_symbol(writer, symbol, codes)?;
            // Write the amplitude bits.
            let amplitude = if coeff > 0 {
                coeff as u32
            } else {
                (coeff - 1) as u16 as u32 & ((1u32 << category) - 1)
            };
            writer.write_bits(amplitude, category);
            zero_run = 0;

            // If this was the last non-zero coefficient, stop here (implicit EOB).
            if Some(i) == last_nonzero {
                break;
            }
        }
    }

    // If there were trailing zeros (i.e., we didn't reach the last_nonzero
    // position), emit EOB.
    if last_nonzero.is_none() || zero_run > 0 || last_nonzero == Some(62) && coeffs[62] == 0 {
        // Actually we need EOB if we didn't emit all 63 coefficients.
        // Simplified: always emit EOB after the loop to signal end.
        // But we only need it if there are trailing zeros.
        if last_nonzero.is_none_or(|lz| lz < 62) {
            encode_ac_symbol(writer, 0x00, codes)?; // EOB
        }
    }

    Ok(())
}

/// Write a single AC Huffman symbol to the bit writer.
fn encode_ac_symbol(
    writer: &mut BitWriter,
    symbol: u8,
    codes: &[(u8, u16, u8)],
) -> Result<(), String> {
    let (_, code, code_len) = codes
        .iter()
        .find(|(sym, _, _)| *sym == symbol)
        .ok_or_else(|| format!("AC: no Huffman code for symbol 0x{symbol:02X}"))?;
    writer.write_bits(*code as u32, *code_len);
    Ok(())
}

/// Decode 63 AC coefficients from the bit reader.
pub fn decode_ac(
    reader: &mut BitReader,
    coeffs: &mut [i16; 63],
    decode_table: &HashMap<(u16, u8), u8>,
) -> Result<(), String> {
    let mut pos = 0;
    while pos < 63 {
        // Read the Huffman symbol (run_of_zeros | category byte).
        let symbol = decode_huffman_symbol(reader, decode_table)?;

        if symbol == 0x00 {
            // EOB: remaining coefficients are zero (already zero-initialised).
            break;
        } else if symbol == 0xF0 {
            // ZRL: 16 consecutive zeros.
            pos += 16;
        } else {
            let run = (symbol >> 4) as usize;
            let category = symbol & 0x0F;
            // Skip `run` zero coefficients.
            pos += run;
            if pos >= 63 {
                break;
            }
            // Read `category` amplitude bits.
            let raw = reader.read_bits(category)? as u16;
            coeffs[pos] = amplitude_to_signed(raw, category);
            pos += 1;
        }
    }
    Ok(())
}

/// Read one Huffman-encoded symbol from the bit reader.
fn decode_huffman_symbol(
    reader: &mut BitReader,
    decode_table: &HashMap<(u16, u8), u8>,
) -> Result<u8, String> {
    let mut code: u16 = 0;
    for len in 1u8..=16 {
        let bit = reader.read_bits(1)? as u16;
        code = (code << 1) | bit;
        if let Some(&symbol) = decode_table.get(&(code, len)) {
            return Ok(symbol);
        }
    }
    Err("AC decode: no valid Huffman code found".to_string())
}

// ---------------------------------------------------------------------------
// Amplitude encoding helper
// ---------------------------------------------------------------------------

/// Convert a raw amplitude bit pattern back to a signed coefficient value.
///
/// In JPEG's amplitude encoding:
/// - If the MSB of the raw pattern is 1, the value is positive: value = raw.
/// - If the MSB of the raw pattern is 0, the value is negative: value = raw - (2^category - 1).
///
/// This is essentially JPEG's variant of offset binary (the opposite mapping
/// from two's complement): positive values have a leading 1, negative values
/// have a leading 0.
///
/// # Examples
///
/// ```text
/// category=1: raw=1 → +1, raw=0 → -1
/// category=2: raw=2 (10b) → +2, raw=1 (01b) → -2, raw=3 → +3, raw=0 → -3
/// category=3: raw=7 → +7, raw=4 → +4, raw=3 → -4, raw=0 → -7
/// ```
fn amplitude_to_signed(raw: u16, category: u8) -> i16 {
    if category == 0 {
        return 0;
    }
    // If the MSB of the raw value is set, it's a positive number.
    let msb = raw >> (category - 1);
    if msb != 0 {
        raw as i16
    } else {
        // Negative: subtract (2^category - 1).
        raw as i16 - ((1i16 << category) - 1)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── BitWriter ────────────────────────────────────────────────

    #[test]
    fn bitwriter_byte_aligned_output() {
        let mut w = BitWriter::new();
        // Write 0xAB = 10101011 as 8 bits.
        w.write_bits(0xAB, 8);
        let bytes = w.into_bytes();
        assert_eq!(bytes, vec![0xAB]);
    }

    #[test]
    fn bitwriter_ff_stuffing() {
        let mut w = BitWriter::new();
        w.write_bits(0xFF, 8);
        w.flush();
        let bytes = w.into_bytes();
        // 0xFF should be followed by 0x00 (byte stuffing).
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0x00, "0xFF must be followed by stuffed 0x00");
    }

    #[test]
    fn bitwriter_multi_bit_sequence() {
        let mut w = BitWriter::new();
        // Write 3 bits (101) then 5 bits (10101) = 10110101 = 0xB5
        w.write_bits(0b101, 3);
        w.write_bits(0b10101, 5);
        let bytes = w.into_bytes();
        assert_eq!(bytes, vec![0b10110101]);
    }

    #[test]
    fn bitwriter_flush_pads_with_ones() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3); // Write 3 bits
        w.flush(); // Should pad with 5 ones: 10111111 = 0xBF
        let bytes = w.into_bytes();
        assert_eq!(bytes[0], 0b10111111);
    }

    // ── BitReader ────────────────────────────────────────────────

    #[test]
    fn bitreader_reads_bits_msb_first() {
        // 0xAB = 10101011
        let data = [0xABu8];
        let mut r = BitReader::new(&data);
        assert_eq!(r.read_bits(4).unwrap(), 0b1010); // high nibble
        assert_eq!(r.read_bits(4).unwrap(), 0b1011); // low nibble
    }

    #[test]
    fn bitreader_unstuffs_ff00() {
        // 0xFF 0x00 should be read as a single 0xFF data byte.
        let data = [0xFF, 0x00, 0b11000000];
        let mut r = BitReader::new(&data);
        let byte = r.read_bits(8).unwrap();
        assert_eq!(byte, 0xFF);
        // Next 2 bits from the 0b11000000 byte.
        let top2 = r.read_bits(2).unwrap();
        assert_eq!(top2, 0b11);
    }

    #[test]
    fn bitreader_roundtrip_with_writer() {
        let mut w = BitWriter::new();
        // Write a sequence of codes.
        w.write_bits(0b101, 3);
        w.write_bits(0b00110, 5);
        w.write_bits(0xFF, 8); // will be stuffed
        w.flush();
        let data = w.into_bytes();

        let mut r = BitReader::new(&data);
        assert_eq!(r.read_bits(3).unwrap(), 0b101);
        assert_eq!(r.read_bits(5).unwrap(), 0b00110);
        assert_eq!(r.read_bits(8).unwrap(), 0xFF); // un-stuffed back to 0xFF
    }

    // ── DC category ─────────────────────────────────────────────

    #[test]
    fn dc_category_zero() {
        assert_eq!(dc_category(0), 0);
    }

    #[test]
    fn dc_category_powers_of_two() {
        assert_eq!(dc_category(1), 1);
        assert_eq!(dc_category(-1), 1);
        assert_eq!(dc_category(2), 2);
        assert_eq!(dc_category(-2), 2);
        assert_eq!(dc_category(4), 3);
        assert_eq!(dc_category(127), 7);
        assert_eq!(dc_category(-128), 8);
        assert_eq!(dc_category(1023), 10);
    }

    // ── Amplitude encoding ───────────────────────────────────────

    #[test]
    fn amplitude_to_signed_category_1() {
        // category=1: 1 bit. raw=1 → +1, raw=0 → -1
        assert_eq!(amplitude_to_signed(1, 1), 1);
        assert_eq!(amplitude_to_signed(0, 1), -1);
    }

    #[test]
    fn amplitude_to_signed_category_2() {
        // raw=3 (11b) → +3, raw=2 (10b) → +2
        // raw=1 (01b) → -2, raw=0 (00b) → -3
        assert_eq!(amplitude_to_signed(3, 2), 3);
        assert_eq!(amplitude_to_signed(2, 2), 2);
        assert_eq!(amplitude_to_signed(1, 2), -2);
        assert_eq!(amplitude_to_signed(0, 2), -3);
    }

    #[test]
    fn amplitude_roundtrip_dc() {
        let codes = build_huffman_codes(&LUMA_DC_BITS, LUMA_DC_HUFFVAL);
        let decode_table = build_decode_table(&LUMA_DC_BITS, LUMA_DC_HUFFVAL);

        for diff in [-1023i16, -127, -1, 0, 1, 50, 127, 1023] {
            let mut w = BitWriter::new();
            encode_dc(&mut w, diff, &codes).expect("encode_dc failed");
            w.flush();
            let bytes = w.into_bytes();
            let mut r = BitReader::new(&bytes);
            let decoded = decode_dc(&mut r, &decode_table).expect("decode_dc failed");
            assert_eq!(decoded, diff, "DC round-trip failed for diff={diff}");
        }
    }

    // ── Canonical Huffman code builder ───────────────────────────

    #[test]
    fn huffman_codes_are_prefix_free() {
        // Verify no shorter code is a prefix of a longer code.
        let codes = build_huffman_codes(&LUMA_AC_BITS, LUMA_AC_HUFFVAL);
        for i in 0..codes.len() {
            for j in 0..codes.len() {
                if i == j { continue; }
                let (_, ci, li) = codes[i];
                let (_, cj, lj) = codes[j];
                if li < lj {
                    // Shift ci to align with cj's length.
                    let shifted = (ci as u32) << (lj - li);
                    assert_ne!(shifted as u16, cj,
                        "code {i} (len={li}) is a prefix of code {j} (len={lj})");
                }
            }
        }
    }

    #[test]
    fn luma_ac_table_has_162_symbols() {
        // Annex K specifies exactly 162 symbols in the luma AC table.
        assert_eq!(LUMA_AC_HUFFVAL.len(), 162);
    }

    #[test]
    fn chroma_ac_table_has_162_symbols() {
        assert_eq!(CHROMA_AC_HUFFVAL.len(), 162);
    }

    #[test]
    fn ac_encode_decode_roundtrip() {
        let codes = build_huffman_codes(&LUMA_AC_BITS, LUMA_AC_HUFFVAL);
        let decode_table = build_decode_table(&LUMA_AC_BITS, LUMA_AC_HUFFVAL);

        let mut original = [0i16; 63];
        original[0] = 10;
        original[2] = -5;
        original[15] = 3;
        original[62] = 1;

        let mut w = BitWriter::new();
        encode_ac(&mut w, &original, &codes).expect("encode_ac failed");
        w.flush();
        let bytes = w.into_bytes();

        let mut decoded = [0i16; 63];
        let mut r = BitReader::new(&bytes);
        decode_ac(&mut r, &mut decoded, &decode_table).expect("decode_ac failed");

        assert_eq!(decoded, original, "AC round-trip mismatch");
    }

    #[test]
    fn ac_encode_all_zeros_produces_eob() {
        // An all-zero block should produce just EOB.
        let codes = build_huffman_codes(&LUMA_AC_BITS, LUMA_AC_HUFFVAL);
        let decode_table = build_decode_table(&LUMA_AC_BITS, LUMA_AC_HUFFVAL);

        let zeros = [0i16; 63];
        let mut w = BitWriter::new();
        encode_ac(&mut w, &zeros, &codes).unwrap();
        w.flush();
        let bytes = w.into_bytes();

        let mut decoded = [0i16; 63];
        let mut r = BitReader::new(&bytes);
        decode_ac(&mut r, &mut decoded, &decode_table).unwrap();
        assert_eq!(decoded, zeros);
    }
}
