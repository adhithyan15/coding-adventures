// # encoder.rs — JPEG baseline encoder
//
// This module ties together all the codec pieces into a complete JPEG encoder:
//
//   1. Convert RGBA pixels to YCbCr colour space (colour.rs)
//   2. Divide the image into 8×8 blocks, padding edges with replicated pixels
//   3. For each block: level-shift (subtract 128), forward 2-D DCT (dsp-dct)
//   4. Quantize DCT coefficients (quantize.rs)
//   5. Entropy-code coefficients using Huffman coding (entropy.rs)
//   6. Assemble a complete JFIF byte stream with all required segments
//
// ## JFIF container structure
//
// A JPEG/JFIF file looks like:
//
//   FF D8           SOI  — Start of Image (always first)
//   FF E0 ... APP0 — JFIF application marker (version, density)
//   FF DB ... DQT  — Define Quantization Table(s)
//   FF C0 ... SOF0 — Start of Frame (dimensions, component info) — Baseline DCT
//   FF C4 ... DHT  — Define Huffman Table(s)
//   FF DA ... SOS  — Start of Scan (then raw entropy-coded data)
//   FF D9           EOI  — End of Image (always last)
//
// ## 4:4:4 sampling
//
// We use 4:4:4 chroma subsampling: every pixel gets its own Y, Cb, and Cr
// sample. No chroma downsampling. This maximises colour accuracy at the cost
// of slightly larger files. The scan interleaves one Y MCU, one Cb MCU, one
// Cr MCU per 8×8 region.

use dsp_dct::{dct_2d, DctNorm, DctType};

use crate::color::rgb_to_ycbcr;
use crate::entropy::{
    build_huffman_codes, encode_ac, encode_dc, BitWriter, CHROMA_AC_BITS,
    CHROMA_AC_HUFFVAL, CHROMA_DC_BITS, CHROMA_DC_HUFFVAL, LUMA_AC_BITS,
    LUMA_AC_HUFFVAL, LUMA_DC_BITS, LUMA_DC_HUFFVAL,
};
use crate::quantize::{scale_qtable, quantize, CHROMA_QTABLE, LUMA_QTABLE, ZIGZAG};

// ---------------------------------------------------------------------------
// JFIF marker constants
// ---------------------------------------------------------------------------
//
// Every JPEG marker is a two-byte sequence: 0xFF followed by the marker byte.
// We only write the second byte here; the helper functions prefix 0xFF.

const M_SOI:  u8 = 0xD8; // Start of Image
const M_APP0: u8 = 0xE0; // JFIF application extension
const M_DQT:  u8 = 0xDB; // Define Quantization Table
const M_SOF0: u8 = 0xC0; // Start of Frame (baseline DCT)
const M_DHT:  u8 = 0xC4; // Define Huffman Table
const M_SOS:  u8 = 0xDA; // Start of Scan
const M_EOI:  u8 = 0xD9; // End of Image

// ---------------------------------------------------------------------------
// Segment writing helpers
// ---------------------------------------------------------------------------

/// Write a two-byte JPEG marker (0xFF + marker_byte) with no length or data.
fn write_marker(out: &mut Vec<u8>, marker: u8) {
    out.push(0xFF);
    out.push(marker);
}

/// Write a JPEG segment: marker + 2-byte length + data.
///
/// The length field includes the 2 length bytes themselves but not the marker.
fn write_segment(out: &mut Vec<u8>, marker: u8, data: &[u8]) {
    write_marker(out, marker);
    let len = (data.len() + 2) as u16; // length includes itself (2 bytes)
    out.push((len >> 8) as u8);
    out.push((len & 0xFF) as u8);
    out.extend_from_slice(data);
}

// ---------------------------------------------------------------------------
// Segment builders
// ---------------------------------------------------------------------------

/// Build an APP0/JFIF segment (18 bytes of payload).
///
/// APP0 identifies this as a JFIF file and specifies the pixel aspect ratio.
/// Most JPEG decoders require APP0 to be present immediately after SOI.
fn build_app0() -> Vec<u8> {
    let mut d = Vec::with_capacity(16);
    d.extend_from_slice(b"JFIF\0"); // 5-byte identifier
    d.push(0x01); d.push(0x01);     // JFIF version 1.1
    d.push(0x00);                   // aspect ratio units = no units (just a ratio)
    d.push(0x00); d.push(0x01);     // X pixel density = 1
    d.push(0x00); d.push(0x01);     // Y pixel density = 1
    d.push(0x00);                   // thumbnail width = 0 (no thumbnail)
    d.push(0x00);                   // thumbnail height = 0
    d
}

/// Build a DQT (Define Quantization Table) segment for one 8-bit table.
///
/// The quantization table is stored in zigzag order, with a precision/ID byte
/// preceding the 64 table values.
///
/// `table_id` is 0 for luma, 1 for chroma.
fn build_dqt(qtable: &[u16; 64], table_id: u8) -> Vec<u8> {
    let mut d = Vec::with_capacity(65);
    // Precision (upper nibble) | table ID (lower nibble).
    // Precision 0 = 8-bit values (we always use 8-bit since quality≥1 keeps entries ≤255).
    d.push(table_id & 0x0F);
    // Write the 64 quantization values in zigzag order.
    // qtable is in row-major order; we reorder to zigzag for the file.
    let mut zigzag_vals = [0u8; 64];
    for rm_pos in 0..64 {
        let zz_pos = ZIGZAG[rm_pos];
        zigzag_vals[zz_pos] = qtable[rm_pos] as u8;
    }
    d.extend_from_slice(&zigzag_vals);
    d
}

/// Build a SOF0 (Start of Frame, baseline DCT) segment.
///
/// SOF0 describes the image dimensions and colour component layout.
/// We always write 3 components (Y, Cb, Cr) with 4:4:4 sampling.
fn build_sof0(width: u32, height: u32) -> Vec<u8> {
    let mut d = Vec::with_capacity(17);
    d.push(0x08); // Sample precision: 8 bits per component
    // Image height (2 bytes, big-endian):
    d.push((height >> 8) as u8);
    d.push((height & 0xFF) as u8);
    // Image width (2 bytes, big-endian):
    d.push((width >> 8) as u8);
    d.push((width & 0xFF) as u8);
    d.push(0x03); // Number of components: 3 (Y, Cb, Cr)

    // Component 1: Y (luma)
    d.push(0x01); // Component ID = 1
    d.push(0x11); // Sampling factors: 1 horizontal, 1 vertical (4:4:4)
    d.push(0x00); // Quantization table ID = 0 (luma)

    // Component 2: Cb (blue chroma)
    d.push(0x02); // Component ID = 2
    d.push(0x11); // Sampling factors: 1×1
    d.push(0x01); // Quantization table ID = 1 (chroma)

    // Component 3: Cr (red chroma)
    d.push(0x03); // Component ID = 3
    d.push(0x11); // Sampling factors: 1×1
    d.push(0x01); // Quantization table ID = 1 (chroma)

    d
}

/// Build a DHT (Define Huffman Table) segment for one table.
///
/// The segment contains the BITS[1..16] counts followed by all HUFFVAL bytes.
/// `tc` = table class (0 = DC, 1 = AC); `th` = table identifier (0 = luma, 1 = chroma).
fn build_dht(bits: &[u8; 16], huffval: &[u8], tc: u8, th: u8) -> Vec<u8> {
    let mut d = Vec::with_capacity(1 + 16 + huffval.len());
    // Table class (upper nibble) | table ID (lower nibble).
    d.push((tc << 4) | (th & 0x0F));
    d.extend_from_slice(bits); // 16 bytes of length counts
    d.extend_from_slice(huffval); // symbol values
    d
}

/// Build a SOS (Start of Scan) header for a 3-component scan.
///
/// The SOS header identifies which component uses which Huffman table.
/// After this header comes the raw entropy-coded bitstream.
fn build_sos_header() -> Vec<u8> {
    let mut d = Vec::with_capacity(12);
    d.push(0x03); // Number of components in scan: 3

    // Component 1: Y — DC table 0, AC table 0 (luma)
    d.push(0x01);         // Component ID
    d.push(0); // DC table 0 | AC table 0

    // Component 2: Cb — DC table 1, AC table 1 (chroma)
    d.push(0x02);
    d.push((1 << 4) | 1);

    // Component 3: Cr — DC table 1, AC table 1 (chroma)
    d.push(0x03);
    d.push((1 << 4) | 1);

    // Spectral selection and approximation (baseline = full-block scan):
    d.push(0x00); // Ss = 0  (start of spectral selection)
    d.push(0x3F); // Se = 63 (end of spectral selection: all AC coefficients)
    d.push(0x00); // Ah = 0, Al = 0 (no successive approximation)

    d
}

// ---------------------------------------------------------------------------
// Block extraction with edge replication
// ---------------------------------------------------------------------------

/// Extract an 8×8 block from a component plane, replicating edge pixels.
///
/// When the image dimensions aren't multiples of 8, the last row/column is
/// replicated to fill the block. This is the simplest valid padding strategy —
/// it avoids introducing artificial discontinuities at image boundaries, which
/// would create ringing artefacts in the DCT domain.
///
/// `plane` is a flat row-major buffer of size `width * height`.
/// `block_row`, `block_col` are the block's top-left pixel coordinates.
fn extract_block(
    plane: &[f32],
    width: usize,
    height: usize,
    block_row: usize,
    block_col: usize,
) -> [f32; 64] {
    let mut block = [0.0f32; 64];
    for r in 0..8 {
        for c in 0..8 {
            // Clamp to image bounds (edge replication).
            let pr = (block_row + r).min(height - 1);
            let pc = (block_col + c).min(width - 1);
            block[r * 8 + c] = plane[pr * width + pc];
        }
    }
    block
}

// ---------------------------------------------------------------------------
// Main encoder
// ---------------------------------------------------------------------------

/// Encode an RGBA image to a complete JFIF/JPEG byte stream.
///
/// `quality` is 1–100 (higher = better quality, larger file). Values outside
/// this range are clamped.
///
/// # Steps
///
/// 1. Convert all pixels from RGB to Y, Cb, Cr.
/// 2. Build scaled quantization tables for luma and chroma.
/// 3. Build Huffman code tables (standard Annex K).
/// 4. Process each 8×8 MCU (Minimum Coded Unit): DCT → quantize → Huffman encode.
/// 5. Assemble the JFIF container around the entropy-coded data.
pub fn encode_jpeg_inner(width: u32, height: u32, rgba: &[u8], quality: u8) -> Vec<u8> {
    let quality = quality.clamp(1, 100);
    let w = width as usize;
    let h = height as usize;

    // ── Step 1: Convert RGBA → YCbCr planes ────────────────────────────────
    //
    // We create three separate flat buffers, one per channel. Having them
    // separate makes the per-block extraction in Step 4 straightforward.
    let n_pixels = w * h;
    let mut y_plane  = vec![0.0f32; n_pixels];
    let mut cb_plane = vec![0.0f32; n_pixels];
    let mut cr_plane = vec![0.0f32; n_pixels];

    for row in 0..h {
        for col in 0..w {
            let idx = (row * w + col) * 4;
            let (y, cb, cr) = rgb_to_ycbcr(rgba[idx], rgba[idx + 1], rgba[idx + 2]);
            y_plane [row * w + col] = y;
            cb_plane[row * w + col] = cb;
            cr_plane[row * w + col] = cr;
        }
    }

    // ── Step 2: Scale quantization tables ──────────────────────────────────
    let luma_qt   = scale_qtable(&LUMA_QTABLE,   quality);
    let chroma_qt = scale_qtable(&CHROMA_QTABLE, quality);

    // ── Step 3: Build Huffman code tables ──────────────────────────────────
    let luma_dc_codes   = build_huffman_codes(&LUMA_DC_BITS,   LUMA_DC_HUFFVAL);
    let luma_ac_codes   = build_huffman_codes(&LUMA_AC_BITS,   LUMA_AC_HUFFVAL);
    let chroma_dc_codes = build_huffman_codes(&CHROMA_DC_BITS, CHROMA_DC_HUFFVAL);
    let chroma_ac_codes = build_huffman_codes(&CHROMA_AC_BITS, CHROMA_AC_HUFFVAL);

    // ── Step 4: Entropy-encode all MCUs ────────────────────────────────────
    //
    // We iterate over 8×8 blocks in raster order. For 4:4:4, each MCU consists
    // of exactly one 8×8 Y block, one 8×8 Cb block, one 8×8 Cr block.
    let mut scan_writer = BitWriter::new();

    // DC differential state: the previous block's DC quantized coefficient
    // for each component. Starts at 0 at the beginning of the scan.
    let mut prev_dc_y:  i16 = 0;
    let mut prev_dc_cb: i16 = 0;
    let mut prev_dc_cr: i16 = 0;

    // Number of 8×8 blocks horizontally and vertically.
    let blocks_wide  = w.div_ceil(8);
    let blocks_tall  = h.div_ceil(8);

    for block_row in 0..blocks_tall {
        for block_col in 0..blocks_wide {
            let br = block_row * 8; // top-left pixel row of this block
            let bc = block_col * 8; // top-left pixel col of this block

            // Encode Y (luma) block.
            encode_block(
                &extract_block(&y_plane, w, h, br, bc),
                &luma_qt,
                &luma_dc_codes, &luma_ac_codes,
                &mut prev_dc_y,
                &mut scan_writer,
            );

            // Encode Cb (chroma) block.
            encode_block(
                &extract_block(&cb_plane, w, h, br, bc),
                &chroma_qt,
                &chroma_dc_codes, &chroma_ac_codes,
                &mut prev_dc_cb,
                &mut scan_writer,
            );

            // Encode Cr (chroma) block.
            encode_block(
                &extract_block(&cr_plane, w, h, br, bc),
                &chroma_qt,
                &chroma_dc_codes, &chroma_ac_codes,
                &mut prev_dc_cr,
                &mut scan_writer,
            );
        }
    }

    // Pad the entropy stream to a byte boundary with 1-bits.
    scan_writer.flush();
    let scan_data = scan_writer.into_bytes();

    // ── Step 5: Assemble the JFIF container ────────────────────────────────
    let mut out = Vec::with_capacity(scan_data.len() + 1024);

    // SOI — Start of Image
    write_marker(&mut out, M_SOI);

    // APP0 — JFIF identifier
    write_segment(&mut out, M_APP0, &build_app0());

    // DQT — Luma quantization table (ID = 0)
    write_segment(&mut out, M_DQT, &build_dqt(&luma_qt, 0));

    // DQT — Chroma quantization table (ID = 1)
    write_segment(&mut out, M_DQT, &build_dqt(&chroma_qt, 1));

    // SOF0 — Start of Frame (baseline DCT, 3-component)
    write_segment(&mut out, M_SOF0, &build_sof0(width, height));

    // DHT — Luma DC table (class 0, ID 0)
    write_segment(&mut out, M_DHT, &build_dht(&LUMA_DC_BITS, LUMA_DC_HUFFVAL, 0, 0));

    // DHT — Luma AC table (class 1, ID 0)
    write_segment(&mut out, M_DHT, &build_dht(&LUMA_AC_BITS, LUMA_AC_HUFFVAL, 1, 0));

    // DHT — Chroma DC table (class 0, ID 1)
    write_segment(&mut out, M_DHT, &build_dht(&CHROMA_DC_BITS, CHROMA_DC_HUFFVAL, 0, 1));

    // DHT — Chroma AC table (class 1, ID 1)
    write_segment(&mut out, M_DHT, &build_dht(&CHROMA_AC_BITS, CHROMA_AC_HUFFVAL, 1, 1));

    // SOS — Start of Scan header
    write_segment(&mut out, M_SOS, &build_sos_header());

    // Entropy-coded scan data (no length — it runs until EOI).
    out.extend_from_slice(&scan_data);

    // EOI — End of Image
    write_marker(&mut out, M_EOI);

    out
}

/// Encode a single 8×8 block for one colour component.
///
/// Steps:
///   1. Level-shift: subtract 128 from all 64 samples.
///   2. Forward 2-D DCT (ortho normalisation).
///   3. Quantize all 64 coefficients.
///   4. Reorder into zigzag order.
///   5. Entropy-encode DC and AC coefficients.
fn encode_block(
    block: &[f32; 64],
    qtable: &[u16; 64],
    dc_codes: &[(u8, u16, u8)],
    ac_codes: &[(u8, u16, u8)],
    prev_dc: &mut i16,
    writer: &mut BitWriter,
) {
    // ── Level shift: centre values around zero ──────────────────────────────
    //
    // JPEG requires that the DCT is computed on samples in [-128, 127] rather
    // than [0, 255]. Subtracting 128 achieves this. Without this step, the DC
    // coefficient would be huge (up to 255*8*8 = 16320) and the AC terms would
    // have a bias, wasting bits.
    let shifted: Vec<f32> = block.iter().map(|&v| v - 128.0).collect();

    // ── Forward 2-D DCT ─────────────────────────────────────────────────────
    //
    // We use the orthonormal (Ortho) normalisation, which produces coefficients
    // in a range compatible with standard quantization tables.
    let dct_coeffs = dct_2d(&shifted, 8, 8, DctType::II, DctNorm::Ortho)
        .expect("dct_2d on 8×8 block failed");

    // ── Quantize and zigzag reorder ─────────────────────────────────────────
    //
    // We quantize in row-major order then reorder to zigzag. Alternatively we
    // could combine the steps, but keeping them separate is clearer.
    let mut zigzag_qcoeffs = [0i16; 64];
    for rm_pos in 0..64 {
        let qc = quantize(dct_coeffs[rm_pos], qtable[rm_pos]);
        let zz_pos = ZIGZAG[rm_pos];
        zigzag_qcoeffs[zz_pos] = qc;
    }

    // ── DC: encode differential ─────────────────────────────────────────────
    //
    // Instead of encoding the raw DC coefficient, we encode the difference from
    // the previous block's DC coefficient. Adjacent blocks tend to have similar
    // DC values (similar average brightness/colour), so the differences are
    // usually small and compress well.
    let dc = zigzag_qcoeffs[0];
    let diff = dc - *prev_dc;
    *prev_dc = dc;
    encode_dc(writer, diff, dc_codes).expect("encode_dc failed");

    // ── AC: encode positions 1–63 ───────────────────────────────────────────
    let ac_coeffs: [i16; 63] = {
        let mut arr = [0i16; 63];
        arr.copy_from_slice(&zigzag_qcoeffs[1..]);
        arr
    };
    encode_ac(writer, &ac_coeffs, ac_codes).expect("encode_ac failed");
}
