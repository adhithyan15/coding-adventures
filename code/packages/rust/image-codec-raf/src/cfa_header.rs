// # cfa_header.rs — CFA metadata header parser
//
// The CFA header is a variable-length block immediately following the outer
// header in the RAF file.  It is a sequence of *tag blocks*, each with the
// structure:
//
// ```text
// u16 (BE): tag identifier
// u16 (BE): byte_count — number of value bytes that follow
// [byte_count bytes]: value
// ```
//
// The block sequence continues until we run out of bytes (byte_count == 0 on
// the last tag stops the loop gracefully, or we simply exhaust the slice).
//
// ## Known tags
//
// | Tag    | Name             | Value encoding                         |
// |--------|------------------|----------------------------------------|
// | 0x0100 | Image size       | u16 BE width, u16 BE height            |
// | 0x0110 | Raw image size   | u16 BE raw_width, u16 BE raw_height    |
// | 0x0111 | CFA pattern      | 4 bytes (Bayer) or 36 bytes (X-Trans)  |
// | 0x0130 | Auto WB          | 3× u32 LE [R, G, B] multipliers        |
// | 0x0131 | Fine WB          | 3× u32 LE (fallback)                   |
// | 0x0141 | Black levels     | 4× u32 LE per CFA plane                |
// | 0x0142 | White level      | u32 LE saturation point                |
//
// All value bytes for WB and black/white levels are little-endian because
// they live *inside* the CFA header block, which uses its own endianness
// separate from the outer header's big-endian convention.

use crate::header::{read_u16_be, read_u32_le};

/// Maximum number of tag blocks we will parse in one CFA header.
///
/// Real RAF files have far fewer tags (typically 8–20), so 256 is a generous
/// safety cap that prevents unbounded loops on malformed data.
const MAX_TAG_BLOCKS: usize = 256;

/// Maximum image dimension (pixels per side).
///
/// 4096×4096 = 16 MP is well above what any Fujifilm compact camera produces.
/// (X-Trans sensors peak at ~26 MP, but those files are only partially
/// handled in v0.1 — we apply the same cap and return Err for oversized
/// inputs to avoid allocating gigantic buffers from hostile data.)
pub const MAX_IMAGE_DIM: u16 = 4096;

/// The CFA colour filter pattern variant.
///
/// Fujifilm cameras use either:
/// - A classic 2×2 RGGB Bayer grid (older FinePix compacts), or
/// - A 6×6 X-Trans grid (X-Pro, X-T, X-E, X100 series).
#[derive(Debug, Clone)]
pub enum CfaPattern {
    /// 2×2 Bayer pattern, stored row-major [TL, TR, BL, BR].
    /// Values: 0=R, 1=G, 2=B.
    Bayer([u8; 4]),
    /// 6×6 X-Trans pattern, stored row-major (36 bytes).
    /// Values: 0=R, 1=G, 2=B.
    XTrans([u8; 36]),
}

/// All the metadata extracted from the CFA header block.
#[derive(Debug)]
pub struct CfaHeader {
    /// Displayed image dimensions (from tag 0x0100).
    pub width: u16,
    pub height: u16,
    /// Raw sensor dimensions (from tag 0x0110).
    /// Use these for the pixel grid layout; they may differ from width/height
    /// due to active-area cropping.
    pub raw_width: u16,
    pub raw_height: u16,
    /// Colour filter array pattern.
    pub pattern: CfaPattern,
    /// Auto white-balance raw multipliers [R, G, B].
    /// Stored as raw ADC counts; normalise to G=1.0 before applying.
    pub wb: [u32; 3],
    /// Per-plane black levels for CFA planes [R, G1, G2, B].
    pub black_level: [u32; 4],
    /// Sensor saturation point (white level).
    pub white_level: u32,
}

impl Default for CfaHeader {
    fn default() -> Self {
        CfaHeader {
            width: 0,
            height: 0,
            raw_width: 0,
            raw_height: 0,
            // Default to RGGB Bayer (safe fallback for any compact camera).
            pattern: CfaPattern::Bayer([0, 1, 1, 2]),
            // Neutral WB: equal multipliers → each channel kept as-is.
            wb: [1024, 1024, 1024],
            // 0 black level (no pedestal).
            black_level: [0, 0, 0, 0],
            // 12-bit sensor white level (4095 = 2^12 − 1).
            white_level: 4095,
        }
    }
}

/// Parse the CFA header block and return the extracted metadata.
///
/// # Arguments
///
/// * `data` — the raw bytes of the CFA header block (already sliced from the
///   full file using the offsets from the outer header).
///
/// # Errors
///
/// Returns `Err` if:
/// - Any tag block claims more value bytes than remain in `data`
/// - Image dimensions exceed `MAX_IMAGE_DIM` on either axis
/// - `raw_width` or `raw_height` is zero (cannot build a pixel grid)
pub fn parse_cfa_header(data: &[u8]) -> Result<CfaHeader, String> {
    let mut cfa = CfaHeader::default();
    let mut pos = 0usize;
    let mut tag_count = 0usize;

    // ── iterate over tag blocks ──────────────────────────────────────────────
    // Each block is at least 4 bytes: 2-byte tag + 2-byte byte_count.
    // After reading byte_count we advance past the value bytes.
    while pos + 4 <= data.len() {
        if tag_count >= MAX_TAG_BLOCKS {
            break; // safety cap — stop, don't error
        }
        tag_count += 1;

        let tag        = read_u16_be(data, pos);
        let byte_count = read_u16_be(data, pos + 2) as usize;
        pos += 4;

        // Guard: value must fit inside the remaining slice.
        if pos + byte_count > data.len() {
            return Err(format!(
                "RAF CFA header: tag 0x{tag:04X} value ({byte_count} bytes) \
                 would read past end of CFA header block"
            ));
        }
        let value = &data[pos..pos + byte_count];
        pos += byte_count;

        match tag {
            // ── 0x0100: displayed image size (u16 BE width, u16 BE height) ──
            0x0100 if byte_count >= 4 => {
                cfa.width  = read_u16_be(value, 0);
                cfa.height = read_u16_be(value, 2);
                if cfa.width > MAX_IMAGE_DIM || cfa.height > MAX_IMAGE_DIM {
                    return Err(format!(
                        "RAF: image size {}×{} exceeds maximum {}×{}",
                        cfa.width, cfa.height, MAX_IMAGE_DIM, MAX_IMAGE_DIM
                    ));
                }
            }

            // ── 0x0110: raw image size (u16 BE raw_width, u16 BE raw_height) ─
            0x0110 if byte_count >= 4 => {
                cfa.raw_width  = read_u16_be(value, 0);
                cfa.raw_height = read_u16_be(value, 2);
                if cfa.raw_width > MAX_IMAGE_DIM || cfa.raw_height > MAX_IMAGE_DIM {
                    return Err(format!(
                        "RAF: raw image size {}×{} exceeds maximum {}×{}",
                        cfa.raw_width, cfa.raw_height, MAX_IMAGE_DIM, MAX_IMAGE_DIM
                    ));
                }
            }

            // ── 0x0111: CFA pattern ──────────────────────────────────────────
            // 4 bytes → 2×2 Bayer; 36 bytes → 6×6 X-Trans.
            0x0111 if byte_count == 4 => {
                cfa.pattern = CfaPattern::Bayer([value[0], value[1], value[2], value[3]]);
            }
            0x0111 if byte_count == 36 => {
                let mut arr = [0u8; 36];
                arr.copy_from_slice(value);
                cfa.pattern = CfaPattern::XTrans(arr);
            }

            // ── 0x0130: auto WB (3× u32 LE) — prefer this over fine WB ──────
            0x0130 if byte_count >= 12 => {
                cfa.wb = [
                    read_u32_le(value, 0),
                    read_u32_le(value, 4),
                    read_u32_le(value, 8),
                ];
            }

            // ── 0x0131: fine WB (fallback when 0x0130 is absent) ────────────
            0x0131 if byte_count >= 12 => {
                // Only use fine WB if auto WB hasn't been set yet (still at
                // the default equal-multiplier value).
                if cfa.wb == [1024, 1024, 1024] {
                    cfa.wb = [
                        read_u32_le(value, 0),
                        read_u32_le(value, 4),
                        read_u32_le(value, 8),
                    ];
                }
            }

            // ── 0x0141: black levels (4× u32 LE, one per CFA plane) ─────────
            0x0141 if byte_count >= 16 => {
                cfa.black_level = [
                    read_u32_le(value,  0),
                    read_u32_le(value,  4),
                    read_u32_le(value,  8),
                    read_u32_le(value, 12),
                ];
            }

            // ── 0x0142: white level (u32 LE) ────────────────────────────────
            0x0142 if byte_count >= 4 => {
                cfa.white_level = read_u32_le(value, 0);
            }

            // Unknown or undersized tags are silently skipped.
            _ => {}
        }
    }

    Ok(cfa)
}
