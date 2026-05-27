//! GIF-variant LZW encoder and decoder.
//!
//! GIF uses a specific flavour of LZW that differs from the generic CMP03
//! encoding in two important ways:
//!
//! 1. **Configurable minimum code size**: regular LZW always starts at 9-bit
//!    codes (CLEAR=256, first dynamic code=258). GIF parameterises this with
//!    `lzw_minimum_code_size` (range 2–8), so a 4-colour image uses 2-bit
//!    initial indices and a 256-colour image uses 8-bit ones. This means
//!    CLEAR_CODE = 2^lzw_minimum_code_size and initial code width =
//!    lzw_minimum_code_size + 1 bits.
//!
//! 2. **Sub-block framing**: GIF packs the raw bit stream inside sub-blocks
//!    of at most 255 bytes each, prefixed by a 1-byte length. A zero-length
//!    sub-block terminates the data.
//!
//! The bit-packing convention (LSB-first within each byte) is the same as the
//! CMP03 `lzw` crate, so we can share the intuition but not the code.
//!
//! # Code table
//!
//! Entries in the code table are stored as byte vectors. In practice we use
//! the "first byte + extension" trick: to reconstruct a code's byte sequence
//! during decode we only need to store the *first byte* and a *parent code*,
//! then walk the parent chain. This avoids allocating a `Vec<u8>` per entry.
//!
//! Max code table size: 4096 (= 2^12). When `next_code` would exceed 4095,
//! the encoder emits a CLEAR and resets.

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of entries in the GIF LZW code table.
/// Codes are at most 12 bits wide.
const MAX_TABLE_SIZE: u16 = 4096;

// ── Bit writer (LSB-first, byte-buffered) ─────────────────────────────────────

/// Collects variable-width codes into a flat byte vector, LSB-first.
///
/// GIF codes are packed LSB-first: the first code occupies the lowest bits of
/// byte 0, spilling into byte 1 if needed, etc.
struct BitWriter {
    buf: Vec<u8>,
    /// Accumulator for the current byte-in-progress.
    acc: u32,
    /// Number of valid bits in `acc` (0..32).
    bits: u8,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter {
            buf: Vec::new(),
            acc: 0,
            bits: 0,
        }
    }

    /// Push `width` low-order bits of `code`.
    fn write(&mut self, code: u16, width: u8) {
        // Append bits LSB-first into the accumulator.
        self.acc |= (code as u32) << self.bits;
        self.bits += width;
        // Flush complete bytes.
        while self.bits >= 8 {
            self.buf.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.bits -= 8;
        }
    }

    /// Flush any remaining bits (padding with zeros) and return the byte stream.
    fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.buf.push((self.acc & 0xFF) as u8);
        }
        self.buf
    }
}

// ── Bit reader (LSB-first, byte-buffered) ──────────────────────────────────────

/// Reads variable-width codes from a flat byte slice, LSB-first.
///
/// The reader works directly on the concatenated sub-block data; sub-block
/// framing is removed by the caller before construction.
pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    bits: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            pos: 0,
            acc: 0,
            bits: 0,
        }
    }

    /// Read the next `width` bits as a u16. Returns None if the stream ends.
    pub fn read(&mut self, width: u8) -> Option<u16> {
        // Refill accumulator until we have enough bits.
        while self.bits < width {
            if self.pos >= self.data.len() {
                // Allow reading from a partially empty stream (padding zeros).
                if self.bits == 0 {
                    return None;
                }
                // Pad with zeros.
                self.acc |= 0u32 << self.bits;
                self.bits = width; // pretend we have enough
                break;
            }
            self.acc |= (self.data[self.pos] as u32) << self.bits;
            self.bits += 8;
            self.pos += 1;
        }
        let val = (self.acc & ((1u32 << width) - 1)) as u16;
        self.acc >>= width;
        self.bits -= width;
        Some(val)
    }
}

// ── Encoder ───────────────────────────────────────────────────────────────────

/// Wrap a flat byte stream into GIF sub-blocks (≤ 255 bytes each).
///
/// Each sub-block is prefixed by its 1-byte length. A terminator block (length
/// 0x00) follows at the end.
fn to_sub_blocks(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in data.chunks(255) {
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
    out.push(0x00); // terminator
    out
}

/// Encode `pixels` (palette indices) using GIF-variant LZW.
///
/// Returns the full image-data section: one byte for `lzw_minimum_code_size`,
/// followed by sub-blocks.
///
/// # Arguments
/// - `pixels`: flat slice of palette indices (row-major)
/// - `min_code_size`: `lzw_minimum_code_size` (2–8)
pub fn encode(pixels: &[u8], min_code_size: u8) -> Vec<u8> {
    assert!(
        (2..=8).contains(&min_code_size),
        "lzw_minimum_code_size must be 2-8, got {}",
        min_code_size
    );

    let clear_code = 1u16 << min_code_size;
    let eoi_code = clear_code + 1;
    let first_dynamic = eoi_code + 1;

    // Code table: each entry is (parent_code: Option<u16>, byte: u8).
    // We only need to find whether (current_string + next_byte) is in the table.
    // We use a HashMap keyed by (parent_code, byte) → code for fast lookup.
    let mut table: std::collections::HashMap<(u16, u8), u16> = std::collections::HashMap::new();

    let mut next_code = first_dynamic;
    let mut code_size = min_code_size + 1;
    let mut code_size_limit = (1u16 << code_size) - 1;

    let mut bw = BitWriter::new();

    // Emit initial CLEAR.
    bw.write(clear_code, code_size);

    if pixels.is_empty() {
        bw.write(eoi_code, code_size);
        let raw = bw.finish();
        let mut out = vec![min_code_size];
        out.extend(to_sub_blocks(&raw));
        return out;
    }

    let mut prev_code = pixels[0] as u16; // first code = first pixel index

    for &byte in &pixels[1..] {
        let key = (prev_code, byte);
        if let Some(&code) = table.get(&key) {
            // Current string + byte is in the table.
            prev_code = code;
        } else {
            // Emit prev_code; add the new string to the table.
            bw.write(prev_code, code_size);

            if next_code < MAX_TABLE_SIZE {
                table.insert(key, next_code);
                next_code += 1;
                // Grow code size if next_code exceeds the current limit.
                if next_code > code_size_limit && code_size < 12 {
                    code_size += 1;
                    code_size_limit = (1u16 << code_size) - 1;
                }
            } else {
                // Table full: emit CLEAR and reset.
                bw.write(clear_code, code_size);
                table.clear();
                next_code = first_dynamic;
                code_size = min_code_size + 1;
                code_size_limit = (1u16 << code_size) - 1;
            }

            prev_code = byte as u16;
        }
    }

    // Emit the final code.
    bw.write(prev_code, code_size);
    // Emit EOI.
    bw.write(eoi_code, code_size);

    let raw = bw.finish();
    let mut out = vec![min_code_size];
    out.extend(to_sub_blocks(&raw));
    out
}

// ── Decoder ───────────────────────────────────────────────────────────────────

/// Decode GIF image data into a flat vector of palette indices.
///
/// `data` should be the raw image-data section starting with the
/// `lzw_minimum_code_size` byte, followed by sub-blocks.
///
/// `max_output` caps the number of decoded bytes to prevent decompression-bomb
/// attacks.  Pass `width * height` (the declared pixel count) from the caller;
/// any stream that expands beyond this returns `Err`.
///
/// Returns `Err` on invalid input.
pub fn decode(data: &[u8], max_output: usize) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("GIF LZW: empty data".into());
    }
    let min_code_size = data[0];
    if !(2..=8).contains(&min_code_size) {
        return Err(format!(
            "GIF LZW: invalid lzw_minimum_code_size {} (expected 2-8)",
            min_code_size
        ));
    }

    // Read sub-blocks into a flat byte buffer.
    let raw = read_sub_blocks(&data[1..])?;

    let clear_code = 1u16 << min_code_size;
    let eoi_code = clear_code + 1;
    let first_dynamic = eoi_code + 1;

    // Code table stored as (parent: u16, byte: u8). Index = code number.
    // Codes 0..clear_code are literals; clear_code and eoi_code are special.
    // Dynamic codes start at first_dynamic.
    //
    // We track: parent[code] and first_byte[code].
    // To reconstruct a code's string: walk parent chain to get the string,
    // first_byte[root] gives the first byte.
    const TABLE_CAP: usize = 4096;
    let mut parent: [u16; TABLE_CAP] = [0u16; TABLE_CAP];
    let mut first_byte: [u8; TABLE_CAP] = [0u8; TABLE_CAP];
    // `code_byte[i]` = the byte appended at code i (= last byte of its string).
    let mut code_byte: [u8; TABLE_CAP] = [0u8; TABLE_CAP];

    // Initialize literal entries.
    for i in 0..(clear_code as usize) {
        code_byte[i] = i as u8;
        first_byte[i] = i as u8;
        parent[i] = u16::MAX; // no parent (root)
    }

    let mut next_code = first_dynamic;
    let mut code_size = min_code_size + 1;

    let mut br = BitReader::new(&raw);
    let mut output: Vec<u8> = Vec::new();

    // Helper: reconstruct the byte string for a code (appended in reverse).
    // Returns (string_in_forward_order, first_byte_of_string).
    let decode_string = |code: u16,
                         parent: &[u16; TABLE_CAP],
                         code_byte: &[u8; TABLE_CAP]|
     -> (Vec<u8>, u8) {
        let mut stack = Vec::new();
        let mut c = code;
        loop {
            stack.push(code_byte[c as usize]);
            let p = parent[c as usize];
            if p == u16::MAX {
                break;
            }
            c = p;
        }
        let first = *stack.last().unwrap();
        stack.reverse();
        (stack, first)
    };

    // Read and discard the first CLEAR code (required by GIF spec).
    let first = br
        .read(code_size)
        .ok_or("GIF LZW: truncated before first code")?;
    if first != clear_code {
        return Err(format!(
            "GIF LZW: expected CLEAR_CODE ({}) as first code, got {}",
            clear_code, first
        ));
    }

    // Read the first actual code.
    let mut prev_code = br
        .read(code_size)
        .ok_or("GIF LZW: truncated after first CLEAR")?;
    if prev_code == eoi_code {
        return Ok(output); // empty image
    }
    if prev_code >= next_code {
        return Err(format!(
            "GIF LZW: invalid first code {} (table has {} entries)",
            prev_code, next_code
        ));
    }
    let (s, _) = decode_string(prev_code, &parent, &code_byte);
    output.extend_from_slice(&s);
    if output.len() > max_output {
        return Err("GIF LZW: decompressed output exceeds declared image size".into());
    }

    loop {
        let code = match br.read(code_size) {
            None => break,
            Some(c) => c,
        };

        if code == eoi_code {
            break;
        }

        if code == clear_code {
            // Reset table and code-width back to the initial state.
            next_code = first_dynamic;
            code_size = min_code_size + 1;
            prev_code = match br.read(code_size) {
                None => break,
                Some(c) => {
                    if c == eoi_code {
                        break;
                    }
                    // Guard against "double CLEAR" or a dynamic code used as
                    // the first literal after a reset — both would be invalid
                    // because the table only contains literal codes 0..clear_code
                    // right after a reset.
                    if c == clear_code || c >= next_code {
                        return Err(format!(
                            "GIF LZW: invalid post-CLEAR code {} \
                             (expected literal 0..{})",
                            c, clear_code
                        ));
                    }
                    c
                }
            };
            let (s, _) = decode_string(prev_code, &parent, &code_byte);
            output.extend_from_slice(&s);
            if output.len() > max_output {
                return Err(
                    "GIF LZW: decompressed output exceeds declared image size".into(),
                );
            }
            continue;
        }

        // Normal code.
        let entry: Vec<u8>;
        let first_of_entry: u8;

        if code < next_code {
            // Code is in the table.
            let (s, fb) = decode_string(code, &parent, &code_byte);
            first_of_entry = fb;
            entry = s;
        } else if code == next_code {
            // Special case: code is the next entry being added.
            // Its string = string(prev_code) + first_byte(string(prev_code)).
            let (s, fb) = decode_string(prev_code, &parent, &code_byte);
            first_of_entry = fb;
            let mut s2 = s.clone();
            s2.push(fb);
            entry = s2;
        } else {
            return Err(format!(
                "GIF LZW: invalid code {} (next_code = {})",
                code, next_code
            ));
        }

        output.extend_from_slice(&entry);
        if output.len() > max_output {
            return Err("GIF LZW: decompressed output exceeds declared image size".into());
        }

        // Add new entry to the table: string(prev_code) + first_byte(entry).
        if next_code < MAX_TABLE_SIZE {
            parent[next_code as usize] = prev_code;
            code_byte[next_code as usize] = first_of_entry;
            first_byte[next_code as usize] = if parent[prev_code as usize] == u16::MAX {
                code_byte[prev_code as usize]
            } else {
                first_byte[prev_code as usize]
            };
            next_code += 1;
            // Grow code size when the table fills the current width.
            //
            // The decoder is always one entry behind the encoder: the encoder
            // adds entry K and grows *before* writing the next code, while the
            // decoder adds entry K-1 (from the previous iteration) and must
            // grow *before reading* that same next code.  Concretely, the
            // encoder grows when next_code >= 2^code_size (after incrementing);
            // the decoder must grow one step earlier, at next_code >= 2^code_size - 1,
            // so that the subsequent read uses the new (wider) code width.
            if next_code >= (1u16 << code_size) - 1 && code_size < 12 {
                code_size += 1;
            }
        }
        // When table full we stop adding but keep decoding.

        prev_code = code;
    }

    Ok(output)
}

/// Read GIF sub-blocks into a flat byte vector.
///
/// A sub-block starts with a length byte (0–255). A length of 0 is the
/// terminator. Returns an error if data ends prematurely.
pub fn read_sub_blocks(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        if pos >= data.len() {
            // Tolerate missing terminator at end of file.
            break;
        }
        let len = data[pos] as usize;
        pos += 1;
        if len == 0 {
            break; // terminator
        }
        if pos + len > data.len() {
            return Err(format!(
                "GIF LZW: sub-block length {} extends past end of data at offset {}",
                len,
                pos - 1
            ));
        }
        out.extend_from_slice(&data[pos..pos + len]);
        pos += len;
    }
    Ok(out)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(pixels: &[u8], min_code_size: u8) {
        let encoded = encode(pixels, min_code_size);
        // usize::MAX used in tests — test data is not adversarial.
        let decoded = decode(&encoded, usize::MAX).expect("decode failed");
        assert_eq!(
            decoded, pixels,
            "round-trip failed for min_code_size={}",
            min_code_size
        );
    }

    #[test]
    fn round_trip_empty() {
        round_trip(&[], 2);
    }

    #[test]
    fn round_trip_single_pixel() {
        round_trip(&[0], 2);
        round_trip(&[3], 2);
    }

    #[test]
    fn round_trip_solid_color() {
        // Pixel value must be < clear_code (2^mcs = 4 for mcs=2).
        // Values 4 (CLEAR) and 5 (EOI) are reserved, so use 3.
        let pixels = vec![3u8; 64];
        round_trip(&pixels, 2);
    }

    #[test]
    fn round_trip_2color() {
        let pixels: Vec<u8> = (0u8..64).map(|i| i % 2).collect();
        round_trip(&pixels, 2);
    }

    #[test]
    fn round_trip_256color() {
        let pixels: Vec<u8> = (0u8..=255).collect();
        round_trip(&pixels, 8);
    }

    #[test]
    fn round_trip_rle_pattern() {
        // Long run of the same color — LZW should compress well.
        let pixels = vec![42u8; 1024];
        round_trip(&pixels, 8);
    }

    #[test]
    fn round_trip_gradient() {
        let pixels: Vec<u8> = (0u8..128).collect();
        round_trip(&pixels, 7);
    }

    #[test]
    fn round_trip_all_min_code_sizes() {
        let pixels = vec![0u8, 1, 0, 1, 0];
        for mcs in 2u8..=8 {
            round_trip(&pixels, mcs);
        }
    }

    #[test]
    fn encoded_starts_with_min_code_size() {
        let encoded = encode(&[0, 1, 0], 4);
        assert_eq!(encoded[0], 4, "first byte must be lzw_minimum_code_size");
    }

    #[test]
    fn sub_block_framing_max_255() {
        // Each sub-block must be at most 255 bytes.
        let pixels = vec![0u8; 10000];
        let encoded = encode(&pixels, 8);
        let mut pos = 1; // skip min_code_size byte
        while pos < encoded.len() {
            let len = encoded[pos] as usize;
            if len == 0 {
                break;
            }
            assert!(len <= 255, "sub-block length {} > 255", len);
            pos += 1 + len;
        }
    }

    #[test]
    fn decode_bad_min_code_size() {
        // min_code_size = 0 is invalid.
        assert!(decode(&[0], usize::MAX).is_err());
        assert!(decode(&[1], usize::MAX).is_err());
        assert!(decode(&[9], usize::MAX).is_err());
    }

    #[test]
    fn decode_empty_data() {
        assert!(decode(&[], usize::MAX).is_err());
    }

    #[test]
    fn decode_rejects_output_beyond_limit() {
        // Encode a long RLE sequence then try to decode with a tiny limit.
        let pixels = vec![0u8; 512];
        let encoded = encode(&pixels, 8);
        // Limit of 10 is well below 512 — must fail.
        assert!(decode(&encoded, 10).is_err());
        // Limit of 512 must succeed.
        assert!(decode(&encoded, 512).is_ok());
    }

    #[test]
    fn sub_blocks_read_write_roundtrip() {
        let data: Vec<u8> = (0u8..200).collect();
        let blocks = to_sub_blocks(&data);
        let recovered = read_sub_blocks(&blocks).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn code_size_grows_at_threshold() {
        // With min_code_size=2, initial code_size=3 (8 codes fit).
        // After 5 dynamic entries (4096=max, but first grow happens earlier),
        // verify we can still round-trip.
        let pixels: Vec<u8> = (0u8..30).map(|i| i % 4).collect();
        round_trip(&pixels, 2);
    }
}
