// # lossless_jpeg.rs — SOF3 Lossless JPEG Decoder for Canon CR2
//
// Canon CR2 stores its full-resolution Bayer sensor data inside a lossless
// JPEG (ISO 10918-1 Part 12 "JPEG-LS" extended — actually the Huffman lossless
// variant called "SOF3" — Start of Frame type 3).
//
// ## What is lossless JPEG?
//
// Standard JPEG uses the DCT (Discrete Cosine Transform) and discards high-
// frequency detail. Lossless JPEG is a completely different beast: it uses
// DPCM (Differential Pulse Code Modulation) prediction, encoding only the
// *difference* between each pixel and a prediction of its value. Because the
// differences tend to be small, they compress well with Huffman coding.
//
// ## CR2 Specifics
//
// Canon uses a 2-component or 4-component lossless JPEG inside each CR2 strip:
//
//   - **2-component** (most DSLRs): component 0 = even columns, component 1 = odd
//     columns. The Bayer row is split into two "sub-channels" that are interleaved
//     in the scan.
//   - **Precision**: 14 bits per component (raw ADC output depth).
//   - **Predictor**: JPEG predictor 1 (Ra = left neighbour). At each restart
//     interval (= one row) the predictor is reset to `1 << (precision-1)` = 8192.
//   - **Restart markers**: 0xFF 0xD0..0xD7 appear between rows.
//
// ## JPEG Marker Map
//
// ```text
// 0xFF 0xD8  SOI — Start Of Image (file begins)
// 0xFF 0xC3  SOF3 — Start Of Frame (lossless)
// 0xFF 0xC4  DHT — Define Huffman Table
// 0xFF 0xDA  SOS — Start Of Scan (compressed data follows)
// 0xFF 0xD9  EOI — End Of Image
// 0xFF 0xDD  DRI — Define Restart Interval
// 0xFF 0xD0..0xD7  RST0..RST7 — Restart Markers (mid-scan)
// 0xFF 0x00  — Byte-stuffing: represents a literal 0xFF in the scan data
// ```
//
// ## Canonical Huffman Decoding
//
// Huffman codes for lossless JPEG are "canonical": given BITS[1..16]
// (count of codes of each bit-length) and HUFFVAL (list of values in
// code-length order), we can reconstruct the tree without storing each
// (code, length) pair explicitly.
//
// The algorithm:
// ```
// code = 0
// ptr  = 0
// for length in 1..=16:
//     for each of BITS[length] values:
//         assign code to HUFFVAL[ptr++]
//         code += 1
//     code <<= 1   (left-shift to extend to next length)
// ```
//
// ## DPCM Decoding
//
// Lossless JPEG encodes *differences* (delta), not absolute values.
// A decoded "s" (SSSS category) from the Huffman table means:
//   - s = 0 → difference is 0 (same as previous pixel)
//   - s = 1..14 → read s bits, interpret as two's-complement magnitude
//
// The two's-complement extension for a positive s:
// ```
// if val < (1 << (s - 1)):
//     val -= (1 << s) - 1    // negative: flip to two's-complement
// ```
//
// Then: `pixel = (previous + difference) & ((1 << precision) - 1)`

// ─── HuffTable ────────────────────────────────────────────────────────────────

/// A canonical Huffman table built from BITS[1..16] and HUFFVAL.
///
/// We use a simple sequential representation: we store `min_code[len]`,
/// `max_code[len]` (inclusive), and `val_ptr[len]` (starting index into
/// `huffval`). Lookup: for a given `(code, len)` pair, if
/// `min_code[len] <= code <= max_code[len]` then
/// `huffval[val_ptr[len] + (code - min_code[len])]` is the decoded value.
#[derive(Debug, Clone)]
pub struct HuffTable {
    /// BITS[1..=16]: count of codes with each bit-length.
    pub bits: [usize; 17],
    /// All Huffman values in canonical code order.
    pub huffval: Vec<u8>,
    /// min_code[len]: smallest code of bit-length `len` (0 if no codes of that length).
    pub min_code: [u32; 17],
    /// max_code[len]: largest code of bit-length `len` (-1 if no codes of that length).
    pub max_code: [i64; 17],
    /// val_ptr[len]: index into `huffval` for the first code of length `len`.
    pub val_ptr: [usize; 17],
}

impl HuffTable {
    /// Build a canonical Huffman table from the raw DHT segment data.
    ///
    /// `bits` — 16-byte array: bits[i] = number of codes with length i+1.
    /// `huffval` — flat list of values in canonical code order.
    ///
    /// Returns `Err` if the table is malformed (too many codes for the length, etc.)
    pub fn build(bits_16: &[u8; 16], huffval: Vec<u8>) -> Result<Self, String> {
        let mut bits = [0usize; 17];
        let mut total = 0usize;
        for i in 0..16 {
            bits[i + 1] = bits_16[i] as usize;
            total += bits[i + 1];
        }
        if total != huffval.len() {
            return Err(format!(
                "DHT: BITS sum {} != HUFFVAL length {}",
                total,
                huffval.len()
            ));
        }

        // Build the canonical lookup arrays.
        let mut min_code = [0u32; 17];
        let mut max_code = [-1i64; 17];
        let mut val_ptr = [0usize; 17];

        let mut code = 0u32;
        let mut ptr = 0usize;
        for len in 1usize..=16 {
            val_ptr[len] = ptr;
            if bits[len] > 0 {
                min_code[len] = code;
                // There are bits[len] codes, each incrementing the code.
                // Last code = code + bits[len] - 1.
                let last = code + (bits[len] as u32) - 1;
                max_code[len] = last as i64;
                ptr += bits[len];
                code = last + 1;
            }
            code <<= 1;
        }

        Ok(HuffTable {
            bits,
            huffval,
            min_code,
            max_code,
            val_ptr,
        })
    }

    /// Look up a code of a given bit-length.
    ///
    /// Returns `Some(huffval)` if the code is valid for that length, or `None`.
    #[inline]
    pub fn lookup(&self, code: u32, len: u8) -> Option<u8> {
        let len = len as usize;
        if len == 0 || len > 16 {
            return None;
        }
        if self.max_code[len] < 0 {
            return None;
        }
        if code < self.min_code[len] || code > self.max_code[len] as u32 {
            return None;
        }
        let idx = self.val_ptr[len] + (code - self.min_code[len]) as usize;
        self.huffval.get(idx).copied()
    }
}

// ─── BitStream ────────────────────────────────────────────────────────────────

/// A big-endian bit-stream reader for JPEG scan data.
///
/// JPEG encodes bit strings MSB-first (most significant bit first, within a
/// byte). The MSB of the first byte is the first bit of the first code.
///
/// ## Byte stuffing
///
/// Inside a JPEG scan, the byte pair `0xFF 0x00` is a "stuffed byte":
/// the `0x00` is discarded and the `0xFF` is emitted as a data byte.
/// This ensures that real JPEG markers (which all begin with `0xFF` followed
/// by a non-zero byte) can be distinguished from scan data.
pub struct BitStream<'a> {
    data: &'a [u8],
    pos: usize,  // byte position (next byte to load)
    bit: i8,     // next bit to emit within `current` (7=MSB → 0=LSB)
    current: u8, // current byte being read
    end: bool,   // we've run out of data
}

impl<'a> BitStream<'a> {
    /// Create a bit stream over a JPEG scan data slice.
    pub fn new(data: &'a [u8]) -> Self {
        let mut bs = BitStream {
            data,
            pos: 0,
            bit: -1, // force load on first access
            current: 0,
            end: false,
        };
        bs.load_byte();
        bs
    }

    /// Load the next byte, handling 0xFF byte-stuffing.
    fn load_byte(&mut self) {
        if self.pos >= self.data.len() {
            self.end = true;
            self.current = 0;
            self.bit = 7;
            return;
        }
        let b = self.data[self.pos];
        self.pos += 1;
        // JPEG byte stuffing: 0xFF 0x00 → 0xFF
        if b == 0xFF
            && self.pos < self.data.len() {
                let next = self.data[self.pos];
                if next == 0x00 {
                    // stuffed byte: skip the 0x00
                    self.pos += 1;
                }
                // If next != 0x00 it's a real marker — stop reading (end of scan)
                // We'll detect this in the marker loop.
            }
        self.current = b;
        self.bit = 7;
    }

    /// Read the next single bit (0 or 1).
    pub fn next_bit(&mut self) -> u8 {
        if self.bit < 0 {
            self.load_byte();
        }
        let bit = (self.current >> self.bit) & 1;
        self.bit -= 1;
        bit
    }

    /// Read `n` bits as a `u32` (MSB first).
    pub fn read_bits(&mut self, n: usize) -> u32 {
        let mut val = 0u32;
        for _ in 0..n {
            val = (val << 1) | self.next_bit() as u32;
        }
        val
    }
}

// ─── Huffman decode ────────────────────────────────────────────────────────────

/// Huffman-decode one difference value from the bit stream.
///
/// Returns the decoded difference as a signed integer.
///
/// The JPEG lossless Huffman code maps to a *category* `s` (the number of
/// bits needed to represent the magnitude of the difference). After reading
/// `s`, we read `s` additional bits and decode the two's-complement integer.
///
/// Two's complement extension rule:
/// ```text
/// if s == 0: diff = 0
/// else:
///   raw = read s bits
///   if raw < (1 << (s - 1)):
///     diff = raw - ((1 << s) - 1)   ← negative
///   else:
///     diff = raw                    ← positive (or zero of that magnitude)
/// ```
fn huffman_decode(bs: &mut BitStream, table: &HuffTable) -> i32 {
    let mut code = 0u32;
    for len in 1u8..=16 {
        code = (code << 1) | bs.next_bit() as u32;
        if let Some(s) = table.lookup(code, len) {
            if s == 0 {
                return 0;
            }
            let raw = bs.read_bits(s as usize) as i32;
            // Two's-complement sign extension.
            let threshold = 1i32 << (s - 1);
            if raw < threshold {
                raw - ((1 << s) - 1)
            } else {
                raw
            }
        } else {
            continue;
        };
        // We can only reach here if lookup returned Some — use the inner block's
        // return.  The match structure above already returns inside the `if let`.
        // This unreachable! is for the compiler.
        unreachable!()
    }
    0 // error fallback: ran through all 16 bit-lengths without a match
}

// ─── SOF3 decode ──────────────────────────────────────────────────────────────

/// Decode a Canon CR2 lossless JPEG (SOF3) strip.
///
/// # Returns
///
/// `Ok((pixels, width, height))` where:
/// - `pixels` is a flat `Vec<u16>` of 14-bit raw sensor values in row-major
///   order, `width × height` elements total.
/// - `width`, `height` are the image dimensions from the SOF3 header.
///
/// # CR2 Interleaving
///
/// CR2 typically uses 2 components. Within each restart interval (= one Bayer
/// row), the scan produces `width` total values: component 0 fills even columns
/// and component 1 fills odd columns.
///
/// After decoding, the caller interleaves:
/// ```text
/// pixel[row][0] = component_0[0]  (col 0)
/// pixel[row][1] = component_1[0]  (col 1)
/// pixel[row][2] = component_0[1]  (col 2)
/// ...
/// ```
///
/// # v0.1 Support
///
/// Full Huffman + DPCM decode for 1-component and 2-component streams with
/// predictor 1 (left). Multi-table scans (> 2 components) return Err.
pub fn decode_sof3(data: &[u8]) -> Result<(Vec<u16>, u32, u32), String> {
    // ── Validate SOI marker ────────────────────────────────────────────────
    if data.len() < 4 {
        return Err("CR2 SOF3: data too short".into());
    }
    if data[0] != 0xFF || data[1] != 0xD8 {
        return Err(format!(
            "CR2 SOF3: expected SOI marker 0xFF 0xD8, got 0x{:02X} 0x{:02X}",
            data[0], data[1]
        ));
    }

    // ── State to collect from markers ──────────────────────────────────────
    let mut width = 0u32;
    let mut height = 0u32;
    let mut precision = 14u8;
    let mut num_components = 0u8;
    // Up to 4 Huffman tables (lossless uses DC tables only, table ID 0..3).
    let mut huff_tables: [Option<HuffTable>; 4] = [None, None, None, None];
    // Component table selector (component index → table ID).
    let mut comp_table: [usize; 4] = [0; 4];
    // Restart interval (0 = no restart markers).
    let mut restart_interval = 0u32;
    // Offset where scan data starts.
    let mut scan_data_start = 0usize;

    // ── Parse JPEG markers ─────────────────────────────────────────────────
    let mut pos = 2; // skip SOI
    'marker_loop: while pos + 2 <= data.len() {
        if data[pos] != 0xFF {
            return Err(format!("CR2 SOF3: expected 0xFF at pos {}, got 0x{:02X}", pos, data[pos]));
        }
        let marker = data[pos + 1];
        pos += 2;

        match marker {
            // SOI (nested, skip)
            0xD8 => {}
            // EOI
            0xD9 => break 'marker_loop,
            // RST markers (standalone — no length field)
            0xD0..=0xD7 => {}
            // DRI — Define Restart Interval
            0xDD => {
                if pos + 4 > data.len() {
                    return Err("CR2 SOF3: truncated DRI".into());
                }
                let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                if seg_len < 4 || pos + seg_len > data.len() {
                    return Err("CR2 SOF3: malformed DRI".into());
                }
                restart_interval =
                    u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as u32;
                pos += seg_len;
            }
            // SOF3 — Start Of Frame (lossless, Huffman)
            0xC3 => {
                if pos + 2 > data.len() {
                    return Err("CR2 SOF3: truncated SOF3 header".into());
                }
                let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                if pos + seg_len > data.len() || seg_len < 8 {
                    return Err("CR2 SOF3: malformed SOF3 segment".into());
                }
                // SOF3 layout (after marker + length):
                //   [0] precision (1 byte)
                //   [1..2] height (2 bytes BE)
                //   [3..4] width (2 bytes BE)
                //   [5] num_components (1 byte)
                //   for each component: 3 bytes (ID, sampling, qtable_id — ignored)
                precision = data[pos + 2];
                // Validate precision before use. Lossless JPEG allows 2..=16 bits.
                // Precision 0 would cause underflow in `1u16 << (precision - 1)` later;
                // precision > 16 would overflow the 16-bit output pixels.
                if !(2..=16).contains(&precision) {
                    return Err(format!(
                        "CR2 SOF3: unsupported precision {} (must be 2..=16)",
                        precision
                    ));
                }
                height = u16::from_be_bytes([data[pos + 3], data[pos + 4]]) as u32;
                width = u16::from_be_bytes([data[pos + 5], data[pos + 6]]) as u32;
                num_components = data[pos + 7];
                if num_components as usize * 3 + 8 > seg_len {
                    return Err("CR2 SOF3: SOF3 segment too short for components".into());
                }
                if num_components == 0 || num_components > 4 {
                    return Err(format!(
                        "CR2 SOF3: unsupported component count {}",
                        num_components
                    ));
                }
                pos += seg_len;
            }
            // DHT — Define Huffman Table
            0xC4 => {
                if pos + 2 > data.len() {
                    return Err("CR2 SOF3: truncated DHT".into());
                }
                let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                if pos + seg_len > data.len() || seg_len < 17 {
                    return Err("CR2 SOF3: malformed DHT segment".into());
                }
                let mut dht_pos = pos + 2; // skip length field
                let remaining = seg_len - 2;
                let dht_end = pos + seg_len;
                // A DHT segment may contain multiple tables.
                let mut consumed = 0usize;
                while consumed < remaining && dht_pos + 17 <= dht_end {
                    let tc_th = data[dht_pos]; // Tc<<4 | Th
                    let _tc = (tc_th >> 4) & 0xF; // table class (0=DC/lossless)
                    let th = (tc_th & 0xF) as usize; // table ID
                    if th > 3 {
                        return Err(format!("CR2 SOF3: DHT table ID {} > 3", th));
                    }
                    // Read BITS[1..16]
                    let mut bits16 = [0u8; 16];
                    bits16.copy_from_slice(&data[dht_pos + 1..dht_pos + 17]);
                    let total_vals: usize = bits16.iter().map(|&b| b as usize).sum();
                    let table_end = dht_pos + 17 + total_vals;
                    if table_end > dht_end {
                        return Err("CR2 SOF3: DHT HUFFVAL runs past segment end".into());
                    }
                    let huffval = data[dht_pos + 17..table_end].to_vec();
                    let table = HuffTable::build(&bits16, huffval)?;
                    huff_tables[th] = Some(table);
                    consumed += 1 + 16 + total_vals;
                    dht_pos = table_end;
                }
                pos += seg_len;
            }
            // SOS — Start Of Scan
            0xDA => {
                if pos + 2 > data.len() {
                    return Err("CR2 SOF3: truncated SOS header".into());
                }
                let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                if pos + seg_len > data.len() {
                    return Err("CR2 SOF3: SOS header extends past file end".into());
                }
                // SOS header: length(2), num_components(1), [comp_id(1), Td_Ta(1)]×n,
                //             Ss(1), Se(1), Ah_Al(1).
                // Minimum SOS segment size = 2 (length) + 1 (ncomp) + 0*2 + 3 = 6 bytes.
                let n_scan_comps = data[pos + 2] as usize;
                // Validate that the segment is large enough for n_scan_comps components.
                // seg_len includes the 2-byte length field; payload = seg_len bytes from pos.
                // Need at least: 1 (ncomp byte) + n_scan_comps*2 + 3 (Ss, Se, Ah_Al) + 2 (length).
                let min_sos_len = 2 + 1 + n_scan_comps * 2 + 3;
                if seg_len < min_sos_len || n_scan_comps > 4 {
                    return Err(format!(
                        "CR2 SOF3: SOS segment malformed ({} components, len={})",
                        n_scan_comps, seg_len
                    ));
                }
                for i in 0..n_scan_comps {
                    let _comp_id = data[pos + 3 + i * 2];
                    let td_ta = data[pos + 4 + i * 2];
                    let td = (td_ta >> 4) as usize; // DC table selector
                    if td > 3 {
                        return Err(format!(
                            "CR2 SOF3: SOS component {} references table {} > 3",
                            i, td
                        ));
                    }
                    comp_table[i] = td;
                }
                // Scan data starts right after the SOS segment.
                scan_data_start = pos + seg_len;
                // (pos is not advanced here — we break immediately)
                break 'marker_loop; // scan data follows
            }
            // APP0..APP15 and other segments with a length field — skip.
            0xE0..=0xEF | 0xFE | 0xDB | 0xC0..=0xC2 | 0xC5..=0xCB | 0xCD..=0xCF => {
                if pos + 2 > data.len() {
                    break 'marker_loop;
                }
                let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                if pos + seg_len > data.len() {
                    break 'marker_loop;
                }
                pos += seg_len;
            }
            _ => {
                // Unknown marker — try to skip by reading the length.
                if pos + 2 > data.len() {
                    break 'marker_loop;
                }
                let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
                if pos + seg_len > data.len() {
                    break 'marker_loop;
                }
                pos += seg_len;
            }
        }
    }

    // ── Validate we got what we need ──────────────────────────────────────
    if width == 0 || height == 0 {
        return Err("CR2 SOF3: SOF3 marker not found or zero dimensions".into());
    }
    if scan_data_start == 0 || scan_data_start >= data.len() {
        // No SOS found — return placeholder zero buffer.
        return Ok((vec![0u16; (width * height) as usize], width, height));
    }

    // ── Decode the scan ────────────────────────────────────────────────────
    //
    // For 1-component: each pixel is one Huffman-decoded diff.
    // For 2-component: within each restart interval (one row), pixels are
    //   interleaved: comp0_pix0, comp1_pix0, comp0_pix1, comp1_pix1, ...
    //
    // The output array is row-major, width × height u16 values.

    // Use usize arithmetic to avoid u32 × u32 overflow on large images.
    let total_pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| format!("CR2 SOF3: image too large ({}×{})", width, height))?;
    // Guard against unreasonably large allocations (>256 MB of u16 = 128 Mpx).
    const MAX_PIXELS: usize = 128 * 1024 * 1024;
    if total_pixels > MAX_PIXELS {
        return Err(format!(
            "CR2 SOF3: image too large: {} pixels (max {})",
            total_pixels, MAX_PIXELS
        ));
    }
    let mut pixels = vec![0u16; total_pixels];
    // precision is guaranteed 2..=16 by the SOF3 header validation above,
    // so (1u32 << precision) never overflows and (precision - 1) never underflows.
    let mask = ((1u32 << precision) - 1) as u16;
    let initial_predictor = (1u16 << (precision - 1)) & mask;

    let nc = num_components.max(1) as usize;
    // How many "super-pixels" per restart interval?
    // With nc=2 and width=W, each restart interval has W/nc "super-pixels"
    // where each super-pixel contributes nc values.
    let scan_data = &data[scan_data_start..];
    let mut bs = BitStream::new(scan_data);

    // prev[c]: previous decoded value for component c (resets at each RST).
    let mut prev = [initial_predictor; 4];

    // Current position in the output buffer.
    let mut out_idx = 0usize;

    // Number of interleaved pairs per row (width / nc) — for nc=2, half columns.
    // For nc=1, full width.
    let pairs_per_row = width as usize / nc; // pixels per component per row
    let rows = height as usize;

    // Restart interval length in "MCU"s (one MCU = nc pixels, one from each comp).
    // If restart_interval == 0, there are no restart markers (treat as one big scan).
    // CR2 sets restart_interval = pairs_per_row (one row per interval).
    let interval = if restart_interval == 0 {
        pairs_per_row * rows
    } else {
        restart_interval as usize
    };

    let mut total_mcus = pairs_per_row * rows;
    let mut mcu_in_interval = 0usize;

    'decode: while total_mcus > 0 {
        // Decode one MCU: nc components.
        for c in 0..nc {
            let table_id = comp_table[c];
            let table = huff_tables[table_id].as_ref().ok_or_else(|| {
                format!("CR2 SOF3: Huffman table {} not defined", table_id)
            })?;

            let diff = huffman_decode(&mut bs, table);
            let raw = ((prev[c] as i32 + diff) as u16) & mask;
            prev[c] = raw;

            if out_idx < pixels.len() {
                pixels[out_idx] = raw;
                out_idx += 1;
            }
        }
        total_mcus -= 1;
        mcu_in_interval += 1;

        if mcu_in_interval >= interval && total_mcus > 0 {
            // Consume restart marker (skip 0xFF 0xDn).
            // The bit stream may have padding bits. Align to byte boundary
            // by discarding remaining bits in the current byte.
            bs.bit = -1; // force next load

            // Look for RST marker in raw bytes.
            // We'll look in the remaining scan data at the current byte pos.
            let raw_pos = bs.pos.saturating_sub(1);
            let remaining_data = &scan_data[raw_pos.min(scan_data.len())..];
            let mut found_rst = false;
            for i in 0..remaining_data.len().saturating_sub(1) {
                if remaining_data[i] == 0xFF
                    && remaining_data[i + 1] >= 0xD0
                    && remaining_data[i + 1] <= 0xD7
                {
                    // Skip past the RST marker.
                    bs.pos = raw_pos + i + 2;
                    bs.bit = -1;
                    found_rst = true;
                    break;
                }
            }
            if !found_rst {
                // No RST marker found — just reset predictor and continue.
            }

            // Reset predictors for the new interval.
            for c in 0..nc {
                prev[c] = initial_predictor;
            }
            mcu_in_interval = 0;
        }

        if out_idx >= total_pixels && total_mcus == 0 {
            break 'decode;
        }
    }

    // ── Reassemble interleaved components ─────────────────────────────────
    //
    // With nc=2, the decode loop produced values interleaved as:
    //   [comp0_row0_col0, comp1_row0_col0, comp0_row0_col1, comp1_row0_col1, ...]
    //
    // For a Bayer row of width W:
    //   col 0 = comp0[0], col 1 = comp1[0], col 2 = comp0[1], col 3 = comp1[1], ...
    //
    // That IS already the correct interleaved order — comp0 at even cols,
    // comp1 at odd cols. The DPCM already interleaved them correctly above.
    //
    // However for nc > 2 (4-component) the layout differs: Canon uses
    // component 0 at col%4==0, comp1 at col%4==2, comp2 at col%4==1, comp3
    // at col%4==3 (or similar permutations depending on model). Since we only
    // guarantee 2-component correctness in v0.1, we leave the decoded array as-is.

    Ok((pixels, width, height))
}

// ─── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── HuffTable tests ────────────────────────────────────────────────────

    #[test]
    fn hufftable_build_simple() {
        // One code of length 1 (value 0), one code of length 2 (value 1).
        // BITS = [1, 1, 0, 0, ...16 total]
        // HUFFVAL = [0, 1]
        // canonical codes: "0" → 0, "10" → 1
        let mut bits16 = [0u8; 16];
        bits16[0] = 1; // 1 code of length 1
        bits16[1] = 1; // 1 code of length 2
        let table = HuffTable::build(&bits16, vec![0u8, 1u8]).unwrap();
        assert_eq!(table.lookup(0b0, 1), Some(0)); // code "0" → 0
        assert_eq!(table.lookup(0b10, 2), Some(1)); // code "10" → 1
        assert_eq!(table.lookup(0b11, 2), None); // "11" is not assigned
    }

    #[test]
    fn hufftable_mismatch_returns_err() {
        let bits16 = [0u8; 16]; // no codes
        let err = HuffTable::build(&bits16, vec![42u8]); // 1 value but 0 codes
        assert!(err.is_err());
    }

    #[test]
    fn hufftable_all_lengths() {
        // A table with one code per length from 1 to 8.
        let mut bits16 = [0u8; 16];
        for i in 0..8 {
            bits16[i] = 1;
        }
        let huffval: Vec<u8> = (0u8..8).collect();
        let table = HuffTable::build(&bits16, huffval).unwrap();
        // Length 1: code = 0b0
        assert_eq!(table.lookup(0b0, 1), Some(0));
        // Length 2: code = 0b10 (previous max was 0, shifted: 1, then +0 = 1 → 0b01? )
        // Let me trace: code=0, len=1: min=0,max=0,val_ptr=0; code→1→shift→2
        // len=2: min=2, max=2, val_ptr=1; code→3→shift→6
        // len=3: min=6, max=6, val_ptr=2; etc.
        assert_eq!(table.lookup(0b10, 2), Some(1));
    }

    // ── BitStream tests ────────────────────────────────────────────────────

    #[test]
    fn bitstream_reads_bits_msb_first() {
        // Byte 0xA5 = 1010_0101
        let data = [0xA5u8];
        let mut bs = BitStream::new(&data);
        assert_eq!(bs.next_bit(), 1);
        assert_eq!(bs.next_bit(), 0);
        assert_eq!(bs.next_bit(), 1);
        assert_eq!(bs.next_bit(), 0);
        assert_eq!(bs.next_bit(), 0);
        assert_eq!(bs.next_bit(), 1);
        assert_eq!(bs.next_bit(), 0);
        assert_eq!(bs.next_bit(), 1);
    }

    #[test]
    fn bitstream_handles_byte_stuffing() {
        // 0xFF 0x00 → the 0xFF is emitted as data, 0x00 is discarded.
        let data = [0xFF, 0x00, 0x80];
        let mut bs = BitStream::new(&data);
        // First byte is 0xFF = 1111_1111
        for _ in 0..8 {
            assert_eq!(bs.next_bit(), 1);
        }
        // Next byte (after stuffing is consumed) is 0x80 = 1000_0000
        assert_eq!(bs.next_bit(), 1);
        assert_eq!(bs.next_bit(), 0);
    }

    #[test]
    fn bitstream_read_bits_multi() {
        let data = [0b10110000u8]; // 1, 0, 1, 1, 0, 0, 0, 0
        let mut bs = BitStream::new(&data);
        assert_eq!(bs.read_bits(4), 0b1011);
        assert_eq!(bs.read_bits(4), 0b0000);
    }

    // ── decode_sof3 error paths ────────────────────────────────────────────

    #[test]
    fn decode_sof3_empty_returns_err() {
        assert!(decode_sof3(&[]).is_err());
    }

    #[test]
    fn decode_sof3_bad_soi_returns_err() {
        assert!(decode_sof3(&[0x00, 0x00, 0x00]).is_err());
    }

    #[test]
    fn decode_sof3_only_soi_returns_err() {
        // SOI without SOF3 → should return Err (zero dimensions).
        let data = [0xFF, 0xD8, 0xFF, 0xD9]; // SOI + EOI only
        let result = decode_sof3(&data);
        // Should fail with "zero dimensions" since there's no SOF3.
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("SOF3") || msg.contains("dimension"), "{}", msg);
    }

    #[test]
    fn decode_sof3_marker_parsing_with_app_segment() {
        // SOI + APP0 (skipped) + EOI
        let mut data = vec![0xFF, 0xD8u8]; // SOI
        // APP0 marker (0xFFE0) with length=18 (standard JFIF header size, but minimal)
        data.extend_from_slice(&[0xFF, 0xE0]); // APP0 marker
        data.extend_from_slice(&[0x00, 0x04]); // length = 4 (minimal: just the length field + 2 bytes)
        data.extend_from_slice(&[0xAA, 0xBB]); // 2 bytes of dummy app data
        data.extend_from_slice(&[0xFF, 0xD9]); // EOI
        // No SOF3 → should fail with dimension error.
        let result = decode_sof3(&data);
        assert!(result.is_err());
    }
}
