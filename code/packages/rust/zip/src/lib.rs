//! zip — CMP09: ZIP archive format (PKZIP, 1989).
//!
//! ZIP bundles one or more files into a single `.zip` archive, compressing
//! each entry independently with DEFLATE (method 8) or storing it verbatim
//! (method 0). The same format underlies Java JARs, Office Open XML (.docx),
//! Android APKs, Python wheels, and many more.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  [Local File Header + File Data]  ← entry 1         │
//! │  [Local File Header + File Data]  ← entry 2         │
//! │  ...                                                │
//! │  ══════════ Central Directory ══════════            │
//! │  [Central Dir Header]  ← entry 1 (has local offset)│
//! │  [Central Dir Header]  ← entry 2                   │
//! │  [End of Central Directory Record]                  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! The dual-header design enables two workflows:
//! - **Sequential write**: append Local Headers one-by-one, write CD at the end.
//! - **Random-access read**: seek to EOCD at the end, read CD, jump to any entry.
//!
//! # Wire Format (all integers little-endian)
//!
//! Local File Header (30 + n + e bytes):
//! ```text
//! [0x04034B50]  signature
//! [version_needed u16]  20=DEFLATE, 10=Stored
//! [flags u16]           bit 11 = UTF-8 filename
//! [method u16]          0=Stored, 8=DEFLATE
//! [mod_time u16]        MS-DOS packed time
//! [mod_date u16]        MS-DOS packed date
//! [crc32 u32]
//! [compressed_size u32]
//! [uncompressed_size u32]
//! [name_len u16]
//! [extra_len u16]
//! [name bytes...]
//! [extra bytes...]
//! [file data...]
//! ```
//!
//! Central Directory Header (46 + n + e + c bytes):
//! ```text
//! [0x02014B50]  signature
//! [version_made_by u16]
//! [version_needed u16]
//! [flags u16]
//! [method u16]
//! [mod_time u16]
//! [mod_date u16]
//! [crc32 u32]
//! [compressed_size u32]
//! [uncompressed_size u32]
//! [name_len u16]
//! [extra_len u16]
//! [comment_len u16]
//! [disk_start u16]
//! [int_attrs u16]
//! [ext_attrs u32]   Unix: (mode << 16)
//! [local_offset u32]
//! [name bytes...]
//! [extra bytes...]
//! [comment bytes...]
//! ```
//!
//! End of Central Directory Record (22 bytes):
//! ```text
//! [0x06054B50]  signature
//! [disk_num u16]
//! [cd_disk u16]
//! [entries_this_disk u16]
//! [entries_total u16]
//! [cd_size u32]
//! [cd_offset u32]
//! [comment_len u16]
//! ```
//!
//! # DEFLATE Inside ZIP
//!
//! ZIP method 8 stores **raw RFC 1951 DEFLATE** — no zlib wrapper (no CMF/FLG
//! header, no Adler-32 checksum). This implementation produces RFC 1951 fixed-
//! Huffman compressed blocks (BTYPE=01) using the `lzss` crate for LZ77 match-
//! finding, giving real compression without transmitting dynamic Huffman tables.
//!
//! # Series
//!
//! ```text
//! CMP00 (LZ77,    1977) — Sliding-window backreferences.
//! CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//! CMP02 (LZSS,    1982) — LZ77 + flag bits.
//! CMP03 (LZW,     1984) — LZ78 + pre-initialized alphabet; GIF.
//! CMP04 (Huffman, 1952) — Entropy coding.
//! CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
//! CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  (this crate)
//! ```

use lzss::Token;

// =============================================================================
// CRC-32
// =============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
// It detects accidental corruption of decompressed content. It is NOT a
// cryptographic hash — for tamper-detection use AES-GCM or a signed manifest.

/// Precomputed CRC-32 lookup table (polynomial 0xEDB88320).
const fn make_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0usize;
        while k < 8 {
            if c & 1 != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = make_crc_table();

/// Compute CRC-32 over `data`, starting from `initial` (use 0 for a fresh hash,
/// or the previous result for an incremental update).
///
/// Note: the initial seed for the first call is `0`, not `0xFFFFFFFF`. The
/// internal pre/post XOR with `0xFFFFFFFF` is handled inside this function.
///
/// ```
/// # use zip::crc32;
/// assert_eq!(crc32(b"hello world", 0), 0x0D4A_1185);
/// // Incremental: same as one-shot.
/// let c1 = crc32(b"hello ", 0);
/// let c2 = crc32(b"world", c1);
/// assert_eq!(c2, 0x0D4A_1185);
/// ```
pub fn crc32(data: &[u8], initial: u32) -> u32 {
    // XOR in the initial state (first call: initial=0 → crc starts at 0xFFFFFFFF).
    let mut crc = initial ^ 0xFFFF_FFFF;
    for &byte in data {
        crc = CRC_TABLE[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    // XOR out to produce the final CRC.
    crc ^ 0xFFFF_FFFF
}

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O
// =============================================================================
//
// RFC 1951 packs bits LSB-first within bytes. Huffman codes are sent with the
// most-significant bit first — so before writing a Huffman code we reverse its
// bits and then write the reversed value LSB-first. Extra bits (length/distance
// extras, stored block headers) are written directly LSB-first without reversal.

/// Writes bits into a byte stream, LSB-first.
struct BitWriter {
    buf: u64,
    bits: u32, // number of valid bits in buf
    out: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self { Self { buf: 0, bits: 0, out: Vec::new() } }

    /// Write `nbits` low bits of `value`, LSB-first (for extra bits and headers).
    fn write_lsb(&mut self, value: u32, nbits: u32) {
        debug_assert!(nbits <= 32);
        self.buf |= (value as u64) << self.bits;
        self.bits += nbits;
        while self.bits >= 8 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            self.bits -= 8;
        }
    }

    /// Write a Huffman code (MSB-first logically → bit-reverse then write LSB-first).
    fn write_huffman(&mut self, code: u32, nbits: u32) {
        debug_assert!(nbits > 0 && nbits <= 16);
        // Reverse the top `nbits` bits of `code`.
        let reversed = code.reverse_bits() >> (32 - nbits);
        self.write_lsb(reversed, nbits);
    }

    /// Align to the next byte boundary (used before stored blocks).
    fn align(&mut self) {
        if self.bits > 0 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf = 0;
            self.bits = 0;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.align();
        self.out
    }
}

/// Reads bits from a byte slice, LSB-first.
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u64,
    bits: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, buf: 0, bits: 0 }
    }

    /// Fill the buffer with more bytes until we have at least `need` bits.
    fn fill(&mut self, need: u32) -> bool {
        while self.bits < need {
            if self.pos >= self.data.len() {
                return false; // exhausted
            }
            self.buf |= (self.data[self.pos] as u64) << self.bits;
            self.pos += 1;
            self.bits += 8;
        }
        true
    }

    /// Read `nbits` bits, returns them LSB-first. Returns None on EOF.
    fn read_lsb(&mut self, nbits: u32) -> Option<u32> {
        if nbits == 0 { return Some(0); }
        if !self.fill(nbits) { return None; }
        let mask = (1u64 << nbits) - 1;
        let val = (self.buf & mask) as u32;
        self.buf >>= nbits;
        self.bits -= nbits;
        Some(val)
    }

    /// Read `nbits` bits and reverse them (for decoding Huffman codes MSB-first).
    fn read_msb(&mut self, nbits: u32) -> Option<u32> {
        let v = self.read_lsb(nbits)?;
        Some(v.reverse_bits() >> (32 - nbits))
    }

    /// Discard any partial byte, aligning to the next byte boundary.
    fn align(&mut self) {
        let discard = self.bits % 8;
        if discard > 0 {
            self.buf >>= discard;
            self.bits -= discard;
        }
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// =============================================================================
//
// RFC 1951 §3.2.6 specifies fixed (pre-defined) Huffman code lengths.
// Using fixed Huffman blocks (BTYPE=01) means we never transmit code tables —
// both encoder and decoder know the tables in advance. This is simpler than
// dynamic Huffman (BTYPE=10) and still achieves real compression via LZ77.
//
// Literal/Length code lengths:
//   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
//   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
//   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
//   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
//
// Distance codes:
//   Symbols 0–29: 5-bit codes equal to the symbol number.

/// Returns the RFC 1951 fixed Huffman code and bit-width for a LL symbol 0-287.
fn fixed_ll_encode(sym: u16) -> (u32, u32) {
    match sym {
        0..=143   => (0b0011_0000 + sym as u32,        8),
        144..=255 => (0b1_1001_0000 + (sym as u32 - 144), 9),
        256..=279 => (sym as u32 - 256,                 7),
        280..=287 => (0b1100_0000 + (sym as u32 - 280), 8),
        _ => panic!("fixed_ll_encode: invalid LL symbol {}", sym),
    }
}

/// Decode a Huffman code from `br` using the RFC 1951 fixed LL table.
///
/// We read bits incrementally — first 7, then up to 9 — and decode in order
/// of increasing code length per the canonical Huffman property.
fn fixed_ll_decode(br: &mut BitReader<'_>) -> Option<u16> {
    // Read 7 bits (enough for the shortest codes: 7-bit range 256-279).
    let v7 = br.read_msb(7)?;
    if v7 <= 23 {
        // 7-bit code: symbols 256-279.
        return Some(v7 as u16 + 256);
    }
    // Need one more bit for 8-bit codes.
    let v8 = (v7 << 1) | br.read_lsb(1)?;
    match v8 {
        48..=191 => Some((v8 - 48) as u16),            // literals 0-143
        192..=199 => Some((v8 + 88) as u16),            // symbols 280-287  (192+88=280)
        _ => {
            // Need one more bit for 9-bit codes (literals 144-255).
            let v9 = (v8 << 1) | br.read_lsb(1)?;
            if (400..=511).contains(&v9) {
                Some((v9 - 256) as u16)                  // literals 144-255 (400-256=144)
            } else {
                None // malformed
            }
        }
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// =============================================================================
//
// Match lengths (3-255) map to LL symbols 257-284 + extra bits.
// Match distances (1-32768) map to distance codes 0-29 + extra bits.

/// (base_length, extra_bits) for LL symbols 257..=284.
static LENGTH_TABLE: [(u32, u32); 28] = [
    (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), // 257-264
    (11, 1), (13, 1), (15, 1), (17, 1),                                 // 265-268
    (19, 2), (23, 2), (27, 2), (31, 2),                                 // 269-272
    (35, 3), (43, 3), (51, 3), (59, 3),                                 // 273-276
    (67, 4), (83, 4), (99, 4), (115, 4),                               // 277-280
    (131, 5), (163, 5), (195, 5), (227, 5),                            // 281-284
];

/// (base_offset, extra_bits) for distance codes 0..=29.
static DIST_TABLE: [(u32, u32); 30] = [
    (1, 0), (2, 0), (3, 0), (4, 0),
    (5, 1), (7, 1), (9, 2), (13, 2),
    (17, 3), (25, 3), (33, 4), (49, 4),
    (65, 5), (97, 5), (129, 6), (193, 6),
    (257, 7), (385, 7), (513, 8), (769, 8),
    (1025, 9), (1537, 9), (2049, 10), (3073, 10),
    (4097, 11), (6145, 11), (8193, 12), (12289, 12),
    (16385, 13), (24577, 13),
];

/// Map a match length (3-255) to its RFC 1951 LL symbol, base, and extra-bits.
fn encode_length(length: u8) -> (u16 /*sym*/, u32 /*base*/, u32 /*extra*/) {
    debug_assert!(length >= 3);
    let l = length as u32;
    for (i, &(base, extra)) in LENGTH_TABLE.iter().enumerate().rev() {
        if l >= base {
            return (257 + i as u16, base, extra);
        }
    }
    panic!("encode_length: unreachable for length={}", length);
}

/// Map a match offset (1-32768) to its distance code, base, and extra-bits.
fn encode_dist(offset: u16) -> (u8 /*code*/, u32 /*base*/, u32 /*extra*/) {
    let o = offset as u32;
    for (code, &(base, extra)) in DIST_TABLE.iter().enumerate().rev() {
        if o >= base {
            return (code as u8, base, extra);
        }
    }
    panic!("encode_dist: unreachable for offset={}", offset);
}

// =============================================================================
// RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
// =============================================================================
//
// Strategy:
//   1. Run LZ77/LZSS match-finding (window=32768, max match=255, min=3).
//   2. Emit a single BTYPE=01 (fixed Huffman) block containing the token stream.
//   3. Literal bytes → fixed LL Huffman code.
//   4. Match (offset, length) → length LL code + extra bits + distance code + extra.
//   5. End-of-block symbol (256) → fixed LL Huffman code.
//
// We produce the entire input as one block. RFC 1951 does not limit Huffman
// block sizes (only stored blocks are capped at 65535 bytes).

/// Compress `data` to a raw RFC 1951 DEFLATE bit-stream (fixed Huffman, single block).
/// The output starts directly with the 3-bit block header — no zlib wrapper.
pub(crate) fn deflate_compress(data: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::new();

    if data.is_empty() {
        // Empty stored block: BFINAL=1 BTYPE=00 + 2-byte LEN=0 + 2-byte NLEN.
        bw.write_lsb(1, 1); // BFINAL=1
        bw.write_lsb(0, 2); // BTYPE=00 (stored)
        bw.align();
        bw.write_lsb(0x0000, 16); // LEN=0
        bw.write_lsb(0xFFFF, 16); // NLEN=~0
        return bw.finish();
    }

    // Run LZ77/LZSS tokenizer. We use window=32768 and max_match=255 so every
    // match maps into the RFC 1951 length table (3-255) and distance table (1-32768).
    let tokens = lzss::encode(data, 32768, 255, 3);

    // Block header: BFINAL=1 (last block), BTYPE=01 (fixed Huffman).
    bw.write_lsb(1, 1); // BFINAL
    bw.write_lsb(1, 1); // BTYPE bit 0 = 1
    bw.write_lsb(0, 1); // BTYPE bit 1 = 0  →  BTYPE = 01

    for tok in tokens {
        match tok {
            Token::Literal(b) => {
                let (code, nbits) = fixed_ll_encode(b as u16);
                bw.write_huffman(code, nbits);
            }
            Token::Match { offset, length } => {
                // --- Length ---
                let (sym, base_len, extra_len_bits) = encode_length(length);
                let (code, nbits) = fixed_ll_encode(sym);
                bw.write_huffman(code, nbits);
                if extra_len_bits > 0 {
                    bw.write_lsb(length as u32 - base_len, extra_len_bits);
                }

                // --- Distance ---
                let (dist_code, base_dist, extra_dist_bits) = encode_dist(offset);
                // Distance codes are 5-bit fixed codes equal to the code number.
                bw.write_huffman(dist_code as u32, 5);
                if extra_dist_bits > 0 {
                    bw.write_lsb(offset as u32 - base_dist, extra_dist_bits);
                }
            }
        }
    }

    // End-of-block symbol (256).
    let (eob_code, eob_bits) = fixed_ll_encode(256);
    bw.write_huffman(eob_code, eob_bits);

    bw.finish()
}

// =============================================================================
// RFC 1951 DEFLATE — Decompress
// =============================================================================
//
// Handles stored blocks (BTYPE=00) and fixed Huffman blocks (BTYPE=01).
// Dynamic Huffman blocks (BTYPE=10) return an error — we only produce BTYPE=01,
// but we must be able to decompress stored blocks written by other tools.

/// Decompress a raw RFC 1951 DEFLATE bit-stream into its original bytes.
/// Returns `Err` on malformed or unsupported (BTYPE=10) input.
pub(crate) fn deflate_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut br = BitReader::new(data);
    let mut out: Vec<u8> = Vec::new();

    loop {
        let bfinal = br.read_lsb(1)
            .ok_or("deflate: unexpected EOF reading BFINAL")?;
        let btype = br.read_lsb(2)
            .ok_or("deflate: unexpected EOF reading BTYPE")?;

        match btype {
            0b00 => {
                // ── Stored block ──────────────────────────────────────────
                // Discard partial byte to align to byte boundary.
                br.align();
                let len = br.read_lsb(16)
                    .ok_or("deflate: EOF reading stored LEN")? as usize;
                let nlen = br.read_lsb(16)
                    .ok_or("deflate: EOF reading stored NLEN")?;
                // Validate: NLEN must be one's complement of LEN.
                if (nlen ^ 0xFFFF) != len as u32 {
                    return Err(format!(
                        "deflate: stored block LEN/NLEN mismatch: {} vs {}", len, nlen
                    ));
                }
                // Guard against decompression bombs (256 MB limit).
                if out.len() + len > 256 * 1024 * 1024 {
                    return Err("deflate: output size limit exceeded".into());
                }
                for _ in 0..len {
                    let b = br.read_lsb(8)
                        .ok_or("deflate: EOF inside stored block data")?;
                    out.push(b as u8);
                }
            }
            0b01 => {
                // ── Fixed Huffman block ───────────────────────────────────
                loop {
                    let sym = fixed_ll_decode(&mut br)
                        .ok_or("deflate: EOF decoding fixed Huffman symbol")?;
                    match sym {
                        0..=255 => {
                            // Guard against decompression bombs.
                            if out.len() >= 256 * 1024 * 1024 {
                                return Err("deflate: output size limit exceeded".into());
                            }
                            out.push(sym as u8);
                        }
                        256 => break, // end-of-block
                        257..=285 => {
                            // Back-reference: decode length + distance.
                            let idx = (sym - 257) as usize;
                            if idx >= LENGTH_TABLE.len() {
                                return Err(format!("deflate: invalid length sym {}", sym));
                            }
                            let (base_len, extra_len_bits) = LENGTH_TABLE[idx];
                            let extra_len = br.read_lsb(extra_len_bits)
                                .ok_or("deflate: EOF reading length extra bits")?;
                            let length = (base_len + extra_len) as usize;

                            // Distance code: 5-bit fixed, read MSB-first.
                            let dist_code = br.read_msb(5)
                                .ok_or("deflate: EOF reading distance code")? as usize;
                            if dist_code >= DIST_TABLE.len() {
                                return Err(format!("deflate: invalid dist code {}", dist_code));
                            }
                            let (base_dist, extra_dist_bits) = DIST_TABLE[dist_code];
                            let extra_dist = br.read_lsb(extra_dist_bits)
                                .ok_or("deflate: EOF reading distance extra bits")?;
                            let offset = (base_dist + extra_dist) as usize;

                            // Bounds check: offset must not exceed decoded output.
                            if offset > out.len() {
                                return Err(format!(
                                    "deflate: back-reference offset {} > output len {}",
                                    offset, out.len()
                                ));
                            }
                            if out.len() + length > 256 * 1024 * 1024 {
                                return Err("deflate: output size limit exceeded".into());
                            }
                            // Copy byte-by-byte to handle overlapping matches
                            // (e.g. offset=1, length=10 encodes a run of one byte × 10).
                            for _ in 0..length {
                                let src = out.len() - offset;
                                let b = out[src];
                                out.push(b);
                            }
                        }
                        _ => return Err(format!("deflate: invalid LL symbol {}", sym)),
                    }
                }
            }
            0b10 => return Err("deflate: dynamic Huffman blocks (BTYPE=10) not supported".into()),
            _    => return Err("deflate: reserved BTYPE=11".into()),
        }

        if bfinal == 1 { break; }
    }
    Ok(out)
}

// =============================================================================
// MS-DOS Date / Time Encoding
// =============================================================================
//
// ZIP stores timestamps in the 16-bit MS-DOS packed format inherited from FAT:
//
//   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
//   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
//
// The combined 32-bit value is (date << 16) | time.
// Year 0 in DOS time = 1980; max representable = 2107.

/// Encode a (year, month, day, hour, minute, second) tuple into the 32-bit
/// MS-DOS datetime used by ZIP Local and Central Directory headers.
pub fn dos_datetime(year: u16, month: u16, day: u16, hour: u16, min: u16, sec: u16) -> u32 {
    let t: u16 = (hour << 11) | (min << 5) | (sec / 2);
    let d: u16 = ((year.saturating_sub(1980)) << 9) | (month << 5) | day;
    ((d as u32) << 16) | (t as u32)
}

/// Fixed timestamp (1980-01-01 00:00:00) used when no real mtime is available.
/// date field: (0<<9)|(1<<5)|1 = 33 = 0x0021; time = 0 → 0x00210000
pub const DOS_EPOCH: u32 = 0x0021_0000;

// =============================================================================
// ZIP Write — ZipWriter
// =============================================================================
//
// ZipWriter accumulates entries in memory: for each file it writes a Local
// File Header immediately, then the (possibly compressed) data, records the
// metadata needed for the Central Directory, and assembles the full archive
// on `finish()`.
//
// Auto-compression policy:
//   - Try DEFLATE. If the compressed output is smaller than the original,
//     use method=8 (DEFLATE).
//   - Otherwise use method=0 (Stored) — common for already-compressed formats
//     like JPEG, PNG, or ZIP inside ZIP.

/// Metadata recorded per entry during writing, used to build the Central Directory.
struct CdRecord {
    name: Vec<u8>,
    method: u16,
    dos_datetime: u32,
    crc: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    local_offset: u32,
    external_attrs: u32,
}

/// Builds a ZIP archive incrementally in memory.
///
/// ```
/// # use zip::ZipWriter;
/// let mut w = ZipWriter::new();
/// w.add_file("hello.txt", b"hello, world!", true);
/// w.add_directory("mydir/");
/// let bytes = w.finish();
/// // bytes is a valid .zip file
/// ```
pub struct ZipWriter {
    buf: Vec<u8>,
    entries: Vec<CdRecord>,
}

impl ZipWriter {
    /// Create a new, empty ZipWriter.
    pub fn new() -> Self {
        Self { buf: Vec::new(), entries: Vec::new() }
    }

    /// Add a file entry.
    ///
    /// If `compress` is true, DEFLATE is attempted; the compressed form is used
    /// only if it is strictly smaller than the uncompressed original.
    pub fn add_file(&mut self, name: &str, data: &[u8], compress: bool) {
        self.add_entry(name, data, compress, 0o100_644);
    }

    /// Add a directory entry (name should end with '/').
    pub fn add_directory(&mut self, name: &str) {
        self.add_entry(name, b"", false, 0o040_755);
    }

    /// Internal: add any entry (file or directory) with given Unix mode.
    fn add_entry(&mut self, name: &str, data: &[u8], compress: bool, unix_mode: u32) {
        let name_bytes = name.as_bytes().to_vec();
        let crc = crc32(data, 0);
        let uncompressed_size = data.len() as u32;

        // Compress if requested; fall back to Stored if it doesn't help.
        let (method, file_data): (u16, Vec<u8>) = if compress && !data.is_empty() {
            let compressed = deflate_compress(data);
            if compressed.len() < data.len() {
                (8, compressed)
            } else {
                (0, data.to_vec())
            }
        } else {
            (0, data.to_vec())
        };

        let compressed_size = file_data.len() as u32;
        let local_offset = self.buf.len() as u32;

        // ── Local File Header ─────────────────────────────────────────────
        let version_needed: u16 = if method == 8 { 20 } else { 10 };
        // GP flag bit 11 = UTF-8 filename.
        let flags: u16 = 0x0800;

        self.buf.extend_from_slice(&0x04034B50u32.to_le_bytes()); // signature
        self.buf.extend_from_slice(&version_needed.to_le_bytes());
        self.buf.extend_from_slice(&flags.to_le_bytes());
        self.buf.extend_from_slice(&method.to_le_bytes());
        self.buf.extend_from_slice(&(DOS_EPOCH as u16).to_le_bytes());  // mod_time
        self.buf.extend_from_slice(&((DOS_EPOCH >> 16) as u16).to_le_bytes()); // mod_date
        self.buf.extend_from_slice(&crc.to_le_bytes());
        self.buf.extend_from_slice(&compressed_size.to_le_bytes());
        self.buf.extend_from_slice(&uncompressed_size.to_le_bytes());
        self.buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // extra_field_length = 0
        self.buf.extend_from_slice(&name_bytes);
        // (no extra field)
        self.buf.extend_from_slice(&file_data);

        // Record for Central Directory.
        self.entries.push(CdRecord {
            name: name_bytes,
            method,
            dos_datetime: DOS_EPOCH,
            crc,
            compressed_size,
            uncompressed_size,
            local_offset,
            external_attrs: unix_mode << 16,
        });
    }

    /// Finish writing: append Central Directory and EOCD, return the archive bytes.
    pub fn finish(mut self) -> Vec<u8> {
        let cd_offset = self.buf.len() as u32;
        let num_entries = self.entries.len() as u16;

        // ── Central Directory ─────────────────────────────────────────────
        let cd_start = self.buf.len();
        for e in &self.entries {
            let version_needed: u16 = if e.method == 8 { 20 } else { 10 };
            self.buf.extend_from_slice(&0x02014B50u32.to_le_bytes()); // signature
            self.buf.extend_from_slice(&0x031Eu16.to_le_bytes());     // version_made_by (Unix, v30)
            self.buf.extend_from_slice(&version_needed.to_le_bytes());
            self.buf.extend_from_slice(&0x0800u16.to_le_bytes());     // flags (UTF-8)
            self.buf.extend_from_slice(&e.method.to_le_bytes());
            self.buf.extend_from_slice(&(e.dos_datetime as u16).to_le_bytes()); // mod_time
            self.buf.extend_from_slice(&((e.dos_datetime >> 16) as u16).to_le_bytes()); // mod_date
            self.buf.extend_from_slice(&e.crc.to_le_bytes());
            self.buf.extend_from_slice(&e.compressed_size.to_le_bytes());
            self.buf.extend_from_slice(&e.uncompressed_size.to_le_bytes());
            self.buf.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
            self.buf.extend_from_slice(&0u16.to_le_bytes()); // extra_len
            self.buf.extend_from_slice(&0u16.to_le_bytes()); // comment_len
            self.buf.extend_from_slice(&0u16.to_le_bytes()); // disk_start
            self.buf.extend_from_slice(&0u16.to_le_bytes()); // internal_attrs
            self.buf.extend_from_slice(&e.external_attrs.to_le_bytes());
            self.buf.extend_from_slice(&e.local_offset.to_le_bytes());
            self.buf.extend_from_slice(&e.name);
            // (no extra, no comment)
        }
        let cd_size = (self.buf.len() - cd_start) as u32;

        // ── End of Central Directory Record ──────────────────────────────
        self.buf.extend_from_slice(&0x06054B50u32.to_le_bytes()); // signature
        self.buf.extend_from_slice(&0u16.to_le_bytes());           // disk_number
        self.buf.extend_from_slice(&0u16.to_le_bytes());           // cd_disk
        self.buf.extend_from_slice(&num_entries.to_le_bytes());    // entries this disk
        self.buf.extend_from_slice(&num_entries.to_le_bytes());    // entries total
        self.buf.extend_from_slice(&cd_size.to_le_bytes());
        self.buf.extend_from_slice(&cd_offset.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes());           // comment_len

        self.buf
    }
}

impl Default for ZipWriter {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// ZIP Read — ZipEntry and ZipReader
// =============================================================================
//
// ZipReader uses the "EOCD-first" strategy for reliable random-access:
//
//   1. Scan backwards for the EOCD signature (PK\x05\x06).
//      Limit the scan to the last 65535 + 22 bytes (EOCD comment max = 65535).
//   2. Read the CD offset and size from EOCD.
//   3. Parse all Central Directory headers into ZipEntry objects.
//   4. On `read(entry)`: seek to the Local Header via `local_offset`, skip
//      the variable-length name + extra fields, read compressed data,
//      decompress, verify CRC-32.
//
// We use CD entries as the authoritative source for sizes and compression
// method. Local headers are only consulted for their variable-length fields
// (name_len + extra_len) so we can skip to the data.

/// Metadata for a single entry inside a ZIP archive.
#[derive(Debug, Clone)]
pub struct ZipEntry {
    /// File name (UTF-8).
    pub name: String,
    /// Uncompressed size in bytes.
    pub size: u32,
    /// Compressed size in bytes.
    pub compressed_size: u32,
    /// Compression method: 0 = Stored, 8 = DEFLATE.
    pub method: u16,
    /// CRC-32 of the uncompressed content.
    pub crc32: u32,
    /// True if this entry is a directory (name ends with '/').
    pub is_directory: bool,
    /// Byte offset of the Local File Header within the archive.
    pub(crate) local_offset: u32,
}

/// Reads entries from an in-memory ZIP archive.
///
/// ```
/// # use zip::{ZipReader, ZipWriter};
/// # let mut w = ZipWriter::new();
/// # w.add_file("f.txt", b"hello", true);
/// # let data = w.finish();
/// let reader = ZipReader::new(&data).unwrap();
/// for entry in reader.entries() {
///     println!("{}: {} bytes", entry.name, entry.size);
/// }
/// ```
pub struct ZipReader<'a> {
    data: &'a [u8],
    entries: Vec<ZipEntry>,
}

/// Read a little-endian u16 from `data` at `offset`. Returns None on OOB.
fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let b = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

/// Read a little-endian u32 from `data` at `offset`. Returns None on OOB.
fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let b = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

impl<'a> ZipReader<'a> {
    /// Parse an in-memory ZIP archive.
    ///
    /// Returns `Err` if no valid EOCD record is found or the archive is
    /// structurally malformed.
    pub fn new(data: &'a [u8]) -> Result<Self, String> {
        let eocd_offset = Self::find_eocd(data)
            .ok_or("zip: no End of Central Directory record found")?;

        // Read EOCD fields.
        let cd_offset = read_u32(data, eocd_offset + 16)
            .ok_or("zip: EOCD too short")? as usize;
        let cd_size = read_u32(data, eocd_offset + 12)
            .ok_or("zip: EOCD too short")? as usize;
        let _num_entries = read_u16(data, eocd_offset + 10)
            .ok_or("zip: EOCD too short")?;

        // Validate CD range.
        if cd_offset + cd_size > data.len() {
            return Err(format!(
                "zip: Central Directory [{}, {}) out of bounds (file size {})",
                cd_offset, cd_offset + cd_size, data.len()
            ));
        }

        // Parse all Central Directory headers.
        let mut entries = Vec::new();
        let mut pos = cd_offset;
        while pos + 4 <= cd_offset + cd_size {
            let sig = read_u32(data, pos).unwrap_or(0);
            if sig != 0x02014B50 {
                break; // end of CD or padding
            }

            let method           = read_u16(data, pos + 10).ok_or("zip: CD entry truncated")?;
            let crc32            = read_u32(data, pos + 16).ok_or("zip: CD entry truncated")?;
            let compressed_size  = read_u32(data, pos + 20).ok_or("zip: CD entry truncated")?;
            let size             = read_u32(data, pos + 24).ok_or("zip: CD entry truncated")?;
            let name_len         = read_u16(data, pos + 28).ok_or("zip: CD entry truncated")? as usize;
            let extra_len        = read_u16(data, pos + 30).ok_or("zip: CD entry truncated")? as usize;
            let comment_len      = read_u16(data, pos + 32).ok_or("zip: CD entry truncated")? as usize;
            let local_offset     = read_u32(data, pos + 42).ok_or("zip: CD entry truncated")?;

            let name_start = pos + 46;
            let name_end   = name_start + name_len;
            if name_end > data.len() {
                return Err("zip: CD entry name out of bounds".into());
            }
            let name = String::from_utf8_lossy(&data[name_start..name_end]).into_owned();
            let is_directory = name.ends_with('/');

            entries.push(ZipEntry {
                name, size, compressed_size, method, crc32, is_directory, local_offset,
            });

            pos = name_end + extra_len + comment_len;
        }

        Ok(Self { data, entries })
    }

    /// Return all entries in the archive (files and directories).
    pub fn entries(&self) -> &[ZipEntry] {
        &self.entries
    }

    /// Decompress and return the data for `entry`. Verifies CRC-32.
    ///
    /// Returns `Err` on CRC mismatch, unsupported method, or corrupt data.
    pub fn read(&self, entry: &ZipEntry) -> Result<Vec<u8>, String> {
        if entry.is_directory {
            return Ok(Vec::new());
        }
        // Reject encrypted entries (GP flag bit 0).
        let local_flags = read_u16(self.data, entry.local_offset as usize + 6)
            .ok_or("zip: local header out of bounds")?;
        if local_flags & 1 != 0 {
            return Err(format!("zip: entry '{}' is encrypted; not supported", entry.name));
        }

        // Skip the Local Header to reach the file data.
        // The Local Header has variable-length name + extra fields (which may
        // differ in length from the CD header). We must re-read them here.
        let lh_off = entry.local_offset as usize;
        let lh_name_len  = read_u16(self.data, lh_off + 26).ok_or("zip: local header truncated")? as usize;
        let lh_extra_len = read_u16(self.data, lh_off + 28).ok_or("zip: local header truncated")? as usize;
        let data_start   = lh_off + 30 + lh_name_len + lh_extra_len;
        let data_end     = data_start + entry.compressed_size as usize;

        if data_end > self.data.len() {
            return Err(format!(
                "zip: entry '{}' data [{}, {}) out of bounds",
                entry.name, data_start, data_end
            ));
        }
        let compressed = &self.data[data_start..data_end];

        // Decompress according to method.
        let decompressed = match entry.method {
            0 => compressed.to_vec(), // Stored — verbatim copy
            8 => deflate_decompress(compressed)
                    .map_err(|e| format!("zip: entry '{}': {}", entry.name, e))?,
            m => return Err(format!("zip: unsupported compression method {} for '{}'", m, entry.name)),
        };

        // Trim to declared uncompressed size (guards against decompressor over-read).
        let decompressed = if decompressed.len() > entry.size as usize {
            decompressed[..entry.size as usize].to_vec()
        } else {
            decompressed
        };

        // Verify CRC-32.
        let actual_crc = crc32(&decompressed, 0);
        if actual_crc != entry.crc32 {
            return Err(format!(
                "zip: CRC-32 mismatch for '{}': expected {:08X}, got {:08X}",
                entry.name, entry.crc32, actual_crc
            ));
        }

        Ok(decompressed)
    }

    /// Find an entry by name and return its decompressed data.
    pub fn read_by_name(&self, name: &str) -> Result<Vec<u8>, String> {
        let entry = self.entries.iter()
            .find(|e| e.name == name)
            .ok_or_else(|| format!("zip: entry '{}' not found", name))?;
        self.read(entry)
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Scan backwards from the end of `data` for the EOCD signature 0x06054B50.
    ///
    /// The EOCD record is at most 22 + 65535 bytes from the end (comment field
    /// can be 0-65535 bytes). We limit the scan to prevent unbounded searches.
    fn find_eocd(data: &[u8]) -> Option<usize> {
        const SIG: u32 = 0x06054B50;
        const MAX_COMMENT: usize = 65535;
        const EOCD_MIN_SIZE: usize = 22;

        if data.len() < EOCD_MIN_SIZE {
            return None;
        }

        // The earliest possible EOCD position (accounting for max comment length).
        let scan_start = data.len().saturating_sub(EOCD_MIN_SIZE + MAX_COMMENT);

        // Scan from end backwards.
        for i in (scan_start..=data.len() - EOCD_MIN_SIZE).rev() {
            if read_u32(data, i) == Some(SIG) {
                // Validate: comment_len at offset 20 must match remaining bytes.
                if let Some(comment_len) = read_u16(data, i + 20) {
                    if i + EOCD_MIN_SIZE + comment_len as usize == data.len() {
                        return Some(i);
                    }
                }
            }
        }
        None
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Compress a list of `(name, data)` pairs into a ZIP archive.
///
/// Each file is compressed with DEFLATE if it reduces size; otherwise stored.
///
/// ```
/// # use zip::zip;
/// let archive = zip(&[("hello.txt", b"hello, world!")]);
/// // archive is a valid .zip file
/// ```
pub fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = ZipWriter::new();
    for (name, data) in entries {
        w.add_file(name, data, true);
    }
    w.finish()
}

/// Decompress all file entries from a ZIP archive.
///
/// Returns a `Vec<(name, data)>` in Central Directory order.
/// Directories (names ending with '/') are skipped.
///
/// ```
/// # use zip::{zip, unzip};
/// let archive = zip(&[("f.txt", b"hello")]);
/// let files = unzip(&archive).unwrap();
/// assert_eq!(files[0].0, "f.txt");
/// assert_eq!(files[0].1, b"hello");
/// ```
pub fn unzip(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let reader = ZipReader::new(data)?;
    let mut out = Vec::new();
    for entry in reader.entries() {
        if !entry.is_directory {
            let content = reader.read(entry)?;
            out.push((entry.name.clone(), content));
        }
    }
    Ok(out)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── CRC-32 ────────────────────────────────────────────────────────────────

    #[test]
    fn test_crc32_known_value() {
        // CRC-32 of "hello world" — verified against Python's binascii.crc32().
        assert_eq!(crc32(b"hello world", 0), 0x0D4A_1185);
        // Standard test vector: CRC-32 of "123456789" = 0xCBF43926.
        assert_eq!(crc32(b"123456789", 0), 0xCBF4_3926);
    }

    #[test]
    fn test_crc32_empty() {
        assert_eq!(crc32(b"", 0), 0x0000_0000);
    }

    #[test]
    fn test_crc32_incremental() {
        let full   = crc32(b"hello world", 0);
        let part1  = crc32(b"hello ", 0);
        let part2  = crc32(b"world", part1);
        assert_eq!(part2, full);
    }

    // ── DEFLATE round-trips ────────────────────────────────────────────────

    fn deflate_rt(data: &[u8]) {
        let compressed   = deflate_compress(data);
        let decompressed = deflate_decompress(&compressed).expect("deflate_decompress failed");
        assert_eq!(decompressed, data, "DEFLATE round-trip mismatch");
    }

    #[test]
    fn test_deflate_empty()        { deflate_rt(b""); }
    #[test]
    fn test_deflate_single_byte()  { deflate_rt(b"A"); }
    #[test]
    fn test_deflate_all_bytes() {
        deflate_rt(&(0u8..=255).collect::<Vec<_>>());
    }
    #[test]
    fn test_deflate_repetitive() {
        let data: Vec<u8> = b"ABCABCABC".repeat(100);
        let compressed = deflate_compress(&data);
        let decompressed = deflate_decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
        assert!(compressed.len() < data.len(), "DEFLATE must compress repetitive data");
    }
    #[test]
    fn test_deflate_long_string() {
        deflate_rt(&b"the quick brown fox jumps over the lazy dog ".repeat(20));
    }
    #[test]
    fn test_deflate_binary() {
        let data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
        deflate_rt(&data);
    }

    // ── ZIP TC-1: Stored round-trip ───────────────────────────────────────

    #[test]
    fn test_zip_stored_roundtrip() {
        let data = b"hello, world";
        let mut w = ZipWriter::new();
        w.add_file("hello.txt", data, false); // compress=false → Stored
        let archive = w.finish();
        let files = unzip(&archive).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "hello.txt");
        assert_eq!(files[0].1, data);
    }

    // ── ZIP TC-2: DEFLATE round-trip ─────────────────────────────────────

    #[test]
    fn test_zip_deflate_roundtrip() {
        let text: Vec<u8> = b"the quick brown fox jumps over the lazy dog ".repeat(10);
        let archive = zip(&[("text.txt", &text)]);
        let files = unzip(&archive).unwrap();
        assert_eq!(files[0].0, "text.txt");
        assert_eq!(files[0].1, text);
    }

    // ── ZIP TC-3: Multiple files ──────────────────────────────────────────

    #[test]
    fn test_zip_multiple_files() {
        let all_bytes: Vec<u8> = (0u8..=255).collect();
        let entries: &[(&str, &[u8])] = &[
            ("a.txt", b"file A content"),
            ("b.txt", b"file B content"),
            ("c.bin", &all_bytes),
        ];
        let archive = zip(entries);
        let files = unzip(&archive).unwrap();
        assert_eq!(files.len(), 3);
        for (name, data) in entries {
            let found = files.iter().find(|(n, _)| n == name).unwrap();
            assert_eq!(found.1.as_slice(), *data, "mismatch for {}", name);
        }
    }

    // ── ZIP TC-4: Directory entry ─────────────────────────────────────────

    #[test]
    fn test_zip_directory_entry() {
        let mut w = ZipWriter::new();
        w.add_directory("mydir/");
        w.add_file("mydir/file.txt", b"contents", true);
        let archive = w.finish();

        let reader  = ZipReader::new(&archive).unwrap();
        let names: Vec<&str> = reader.entries().iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"mydir/"), "directory entry missing");
        assert!(names.contains(&"mydir/file.txt"), "file inside dir missing");

        let dir_entry = reader.entries().iter().find(|e| e.name == "mydir/").unwrap();
        assert!(dir_entry.is_directory);
    }

    // ── ZIP TC-5: CRC-32 mismatch detected ───────────────────────────────

    #[test]
    fn test_zip_crc_mismatch_detected() {
        let archive = zip(&[("f.txt", b"test data")]);
        let mut corrupted = archive.clone();

        // Corrupt a data byte directly (offset 35 = 30-byte fixed header + 5-byte name "f.txt").
        // This changes the decompressed content so it no longer matches the stored CRC-32.
        // (Corrupting only the Local Header's CRC field has no effect because we validate
        //  against the Central Directory's CRC, which is the authoritative source per spec.)
        corrupted[35] ^= 0xFF;

        let result = unzip(&corrupted);
        assert!(result.is_err(), "expected CRC error, got: {:?}", result);
        assert!(result.unwrap_err().contains("CRC"), "error should mention CRC");
    }

    // ── ZIP TC-6: Random access (read single entry) ───────────────────────

    #[test]
    fn test_zip_random_access() {
        let entries: Vec<(String, Vec<u8>)> = (0..10)
            .map(|i| {
                let name = format!("f{}.txt", i);
                let data = format!("content {}", i).into_bytes();
                (name, data)
            })
            .collect();
        let refs: Vec<(&str, &[u8])> = entries.iter()
            .map(|(n, d)| (n.as_ref(), d.as_slice()))
            .collect();
        let archive = zip(&refs);

        let reader = ZipReader::new(&archive).unwrap();
        let entry5 = reader.entries().iter().find(|e| e.name == "f5.txt").unwrap();
        let data5  = reader.read(entry5).unwrap();
        assert_eq!(data5, b"content 5");
    }

    // ── ZIP TC-7: Incompressible data uses Stored ─────────────────────────

    #[test]
    fn test_zip_incompressible_stored() {
        // Pseudo-random data via LCG (seed=42): compresses poorly with DEFLATE.
        let mut seed = 42u32;
        let data: Vec<u8> = (0..1024).map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 24) as u8
        }).collect();

        let archive = zip(&[("random.bin", &data)]);
        let reader  = ZipReader::new(&archive).unwrap();
        let entry   = reader.entries().iter().find(|e| e.name == "random.bin").unwrap();
        // Method should be 0 (Stored) because DEFLATE would make it larger.
        assert_eq!(entry.method, 0, "expected Stored for incompressible data");
        assert_eq!(reader.read(entry).unwrap(), data);
    }

    // ── ZIP TC-8: Empty file ──────────────────────────────────────────────

    #[test]
    fn test_zip_empty_file() {
        let archive = zip(&[("empty.txt", b"")]);
        let files   = unzip(&archive).unwrap();
        assert_eq!(files[0].0, "empty.txt");
        assert_eq!(files[0].1, b"");
    }

    // ── ZIP TC-9: Large file with compression ─────────────────────────────

    #[test]
    fn test_zip_large_file_compressed() {
        let data: Vec<u8> = b"abcdefghij".repeat(10_000); // 100 KB
        let archive = zip(&[("big.bin", &data)]);
        let files   = unzip(&archive).unwrap();
        assert_eq!(files[0].1, data);
        assert!(
            archive.len() < data.len(),
            "repetitive 100 KB must compress: archive={} data={}",
            archive.len(), data.len()
        );
    }

    // ── ZIP TC-10: Unicode filename ───────────────────────────────────────

    #[test]
    fn test_zip_unicode_filename() {
        let archive = zip(&[("日本語/résumé.txt", b"content")]);
        let files   = unzip(&archive).unwrap();
        assert_eq!(files[0].0, "日本語/résumé.txt");
        assert_eq!(files[0].1, b"content");
    }

    // ── ZIP TC-11: Nested paths ───────────────────────────────────────────

    #[test]
    fn test_zip_nested_paths() {
        let entries: &[(&str, &[u8])] = &[
            ("root.txt",           b"root"),
            ("dir/file.txt",       b"nested"),
            ("dir/sub/deep.txt",   b"deep"),
        ];
        let archive = zip(entries);
        let files   = unzip(&archive).unwrap();
        for (name, data) in entries {
            let found = files.iter().find(|(n, _)| n == name).unwrap();
            assert_eq!(found.1.as_slice(), *data, "mismatch for {}", name);
        }
    }

    // ── ZIP TC-12: Empty archive ──────────────────────────────────────────

    #[test]
    fn test_zip_empty_archive() {
        let archive = zip(&[]);
        let files   = unzip(&archive).unwrap();
        assert!(files.is_empty());
    }

    // ── ZIP: read_by_name ─────────────────────────────────────────────────

    #[test]
    fn test_zip_read_by_name() {
        let archive = zip(&[("alpha.txt", b"AAA"), ("beta.txt", b"BBB")]);
        let reader  = ZipReader::new(&archive).unwrap();
        assert_eq!(reader.read_by_name("beta.txt").unwrap(), b"BBB");
        assert!(reader.read_by_name("nope.txt").is_err());
    }

    // ── ZIP: dos_datetime ─────────────────────────────────────────────────

    #[test]
    fn test_dos_datetime_epoch() {
        // 1980-01-01 00:00:00 → year_offset=0, month=1, day=1 → date=(0<<9)|(1<<5)|1=33
        // time = 0
        let dt = dos_datetime(1980, 1, 1, 0, 0, 0);
        assert_eq!(dt >> 16, 33);  // date field
        assert_eq!(dt & 0xFFFF, 0); // time field
    }
}
