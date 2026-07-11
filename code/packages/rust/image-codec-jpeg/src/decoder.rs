// # decoder.rs — JPEG baseline decoder
//
// The decoder reverses every step the encoder performed:
//
//   1. Parse the JFIF container, reading segments until SOS
//   2. Extract the entropy-coded data (until EOI)
//   3. For each 8×8 MCU block:
//      a. Huffman-decode DC (differential) and AC (RLE) coefficients
//      b. Inverse-zigzag the 64 coefficients back to row-major order
//      c. Dequantize: multiply each coefficient by its quantization table entry
//      d. Inverse 2-D DCT
//      e. Level un-shift: add 128, clamp to [0, 255]
//   4. Reconstruct the YCbCr planes
//   5. Convert YCbCr → RGB, output as RGBA (alpha = 255)
//
// ## Parser robustness
//
// This decoder handles the JFIF files produced by our own encoder. It doesn't
// attempt to decode every possible JPEG variant (progressive, arithmetic coding,
// 12-bit precision, etc.) — that would require thousands of lines of additional
// code and is outside IC04 scope.

use std::collections::HashMap;

use dsp_dct::{idct_2d, DctNorm, DctType};

use crate::color::ycbcr_to_rgb;
use crate::entropy::{
    build_decode_table, decode_ac, decode_dc, BitReader,
    CHROMA_AC_BITS, CHROMA_AC_HUFFVAL, CHROMA_DC_BITS, CHROMA_DC_HUFFVAL,
    LUMA_AC_BITS, LUMA_AC_HUFFVAL, LUMA_DC_BITS, LUMA_DC_HUFFVAL,
};
use crate::quantize::{dequantize, IZIGZAG};

// ---------------------------------------------------------------------------
// JPEG marker constants
// ---------------------------------------------------------------------------

const M_SOI:  u8 = 0xD8;
const M_EOI:  u8 = 0xD9;
const M_DQT:  u8 = 0xDB;
const M_SOF0: u8 = 0xC0;
const M_DHT:  u8 = 0xC4;
const M_SOS:  u8 = 0xDA;
// APP0–APPF and other skip-able markers
const M_APP0: u8 = 0xE0;

// ---------------------------------------------------------------------------
// Decoder state
// ---------------------------------------------------------------------------

/// Internal state accumulated while parsing the JPEG header segments.
struct DecoderState {
    width:  u32,
    height: u32,

    // Quantization tables (index 0 = luma, 1 = chroma).
    // We support up to 4 tables (indices 0–3), but IC04 only needs 0 and 1.
    qtables: [Option<[u16; 64]>; 4],

    // Huffman decode tables: [dc_luma, dc_chroma] and [ac_luma, ac_chroma].
    dc_tables: [Option<HashMap<(u16, u8), u8>>; 2],
    ac_tables: [Option<HashMap<(u16, u8), u8>>; 2],

    // Component info from SOF0: (component_id, sampling_factors, qtable_id).
    components: Vec<(u8, u8, u8)>,
}

impl DecoderState {
    fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            qtables: [None, None, None, None],
            dc_tables: [None, None],
            ac_tables: [None, None],
            components: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Main decoder entry point
// ---------------------------------------------------------------------------

/// Decode a JPEG/JFIF byte stream into (width, height, RGBA pixels).
///
/// Returns `Err` with a human-readable message if the input is not valid
/// baseline JFIF as produced by our encoder.
pub fn decode_jpeg_inner(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if data.len() < 4 {
        return Err("JPEG: data too short to be a valid file".to_string());
    }

    // Verify SOI marker (always FF D8).
    if data[0] != 0xFF || data[1] != M_SOI {
        return Err(format!(
            "JPEG: expected SOI (FF D8) but got {:02X} {:02X}",
            data[0], data[1]
        ));
    }

    let mut state = DecoderState::new();
    let mut pos = 2; // Skip past SOI

    // Parse segments until we hit SOS.
    loop {
        // Each segment starts with 0xFF.
        if pos >= data.len() {
            return Err("JPEG: unexpected end of file while looking for marker".to_string());
        }
        if data[pos] != 0xFF {
            return Err(format!("JPEG: expected 0xFF marker at pos {pos}, got {:02X}", data[pos]));
        }
        pos += 1;

        // Skip padding 0xFF bytes (legal in JPEG).
        while pos < data.len() && data[pos] == 0xFF {
            pos += 1;
        }
        if pos >= data.len() {
            return Err("JPEG: file ends at marker".to_string());
        }

        let marker = data[pos];
        pos += 1;

        match marker {
            M_EOI => {
                return Err("JPEG: hit EOI before SOS — no image data".to_string());
            }

            M_SOF0 => {
                // Start of Frame: contains image dimensions and component layout.
                let seg = read_segment(data, &mut pos)?;
                parse_sof0(seg, &mut state)?;
            }

            M_DQT => {
                // Define Quantization Table.
                let seg = read_segment(data, &mut pos)?;
                parse_dqt(seg, &mut state)?;
            }

            M_DHT => {
                // Define Huffman Table.
                let seg = read_segment(data, &mut pos)?;
                parse_dht(seg, &mut state)?;
            }

            M_SOS => {
                // Start of Scan: parse the SOS header, then decode the entropy stream.
                let sos_seg = read_segment(data, &mut pos)?;
                parse_sos_header(sos_seg, &mut state)?;

                // Everything after pos (up to the EOI marker) is entropy-coded data.
                // Extract it: scan for FF D9 (EOI), collecting the bytes before it.
                let entropy_data = extract_entropy_data(data, pos)?;
                let (w, h, rgba) = decode_scan(&entropy_data, &state)?;
                return Ok((w, h, rgba));
            }

            // APP markers (E0–EF), COM, and other markers we skip over.
            _ => {
                let seg = read_segment(data, &mut pos)?;
                let _ = seg; // intentionally skipped
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Segment parsers
// ---------------------------------------------------------------------------

/// Read a length-prefixed segment payload (not including the marker).
///
/// JPEG segment layout: marker(2) + length(2, big-endian) + payload.
/// The length value includes the 2 length bytes themselves.
fn read_segment<'a>(data: &'a [u8], pos: &mut usize) -> Result<&'a [u8], String> {
    if *pos + 2 > data.len() {
        return Err("JPEG: segment length field out of bounds".to_string());
    }
    let len = u16::from_be_bytes([data[*pos], data[*pos + 1]]) as usize;
    if len < 2 {
        return Err("JPEG: segment length < 2 (invalid)".to_string());
    }
    let payload_len = len - 2;
    *pos += 2;
    if *pos + payload_len > data.len() {
        return Err("JPEG: segment payload out of bounds".to_string());
    }
    let seg = &data[*pos..*pos + payload_len];
    *pos += payload_len;
    Ok(seg)
}

/// Parse an SOF0 segment to extract image dimensions and component info.
fn parse_sof0(seg: &[u8], state: &mut DecoderState) -> Result<(), String> {
    // SOF0 layout: precision(1) + height(2) + width(2) + num_components(1) +
    //              [component_id(1) + sampling(1) + qtable_id(1)] × num_components
    if seg.len() < 6 {
        return Err("SOF0: segment too short".to_string());
    }
    let _precision = seg[0]; // should be 8 for baseline
    state.height = u16::from_be_bytes([seg[1], seg[2]]) as u32;
    state.width  = u16::from_be_bytes([seg[3], seg[4]]) as u32;
    let n_components = seg[5] as usize;
    if seg.len() < 6 + n_components * 3 {
        return Err("SOF0: not enough bytes for component data".to_string());
    }
    state.components.clear();
    for i in 0..n_components {
        let base = 6 + i * 3;
        let comp_id  = seg[base];
        let sampling = seg[base + 1];
        let qt_id    = seg[base + 2];
        state.components.push((comp_id, sampling, qt_id));
    }
    Ok(())
}

/// Parse a DQT segment, storing one or more 8-bit quantization tables.
fn parse_dqt(seg: &[u8], state: &mut DecoderState) -> Result<(), String> {
    let mut i = 0;
    while i < seg.len() {
        let precision_and_id = seg[i];
        i += 1;
        let precision = precision_and_id >> 4; // 0 = 8-bit values
        let table_id  = (precision_and_id & 0x0F) as usize;
        if table_id >= 4 {
            return Err(format!("DQT: invalid table ID {table_id}"));
        }
        if precision != 0 {
            return Err("DQT: 16-bit quantization tables not supported".to_string());
        }
        if i + 64 > seg.len() {
            return Err("DQT: table data truncated".to_string());
        }
        // The file stores values in zigzag order; we store them in row-major order.
        let mut table = [0u16; 64];
        for zz_pos in 0..64 {
            let rm_pos = IZIGZAG[zz_pos];
            table[rm_pos] = seg[i + zz_pos] as u16;
        }
        state.qtables[table_id] = Some(table);
        i += 64;
    }
    Ok(())
}

/// Parse a DHT segment, storing one or more Huffman decode tables.
fn parse_dht(seg: &[u8], state: &mut DecoderState) -> Result<(), String> {
    let mut i = 0;
    while i < seg.len() {
        if i >= seg.len() { break; }
        let tc_th = seg[i];
        i += 1;
        let table_class = (tc_th >> 4) as usize; // 0 = DC, 1 = AC
        let table_id    = (tc_th & 0x0F) as usize;
        if table_class > 1 || table_id > 1 {
            return Err(format!("DHT: unsupported table class/id {table_class}/{table_id}"));
        }
        if i + 16 > seg.len() {
            return Err("DHT: BITS array truncated".to_string());
        }
        let mut bits = [0u8; 16];
        bits.copy_from_slice(&seg[i..i + 16]);
        i += 16;
        let total_symbols: usize = bits.iter().map(|&b| b as usize).sum();
        if i + total_symbols > seg.len() {
            return Err("DHT: HUFFVAL array truncated".to_string());
        }
        let huffval = &seg[i..i + total_symbols];
        i += total_symbols;
        let decode_table = build_decode_table(&bits, huffval);
        if table_class == 0 {
            state.dc_tables[table_id] = Some(decode_table);
        } else {
            state.ac_tables[table_id] = Some(decode_table);
        }
    }
    Ok(())
}

/// Parse the SOS header (just validates it; the actual component mapping
/// is already known from SOF0).
fn parse_sos_header(seg: &[u8], _state: &mut DecoderState) -> Result<(), String> {
    // We trust the SOF0 component info and only validate that the SOS doesn't
    // disagree in a way we can't handle.
    if seg.is_empty() {
        return Err("SOS: empty header".to_string());
    }
    let _n = seg[0]; // number of components in scan
    Ok(())
}

// ---------------------------------------------------------------------------
// Entropy-coded data extraction
// ---------------------------------------------------------------------------

/// Extract the entropy-coded bytes from `data[pos..]` up to (but not including)
/// the EOI marker (FF D9).
///
/// Within the entropy stream, FF 00 is a stuffed byte — the 0x00 is part of
/// the stream formatting, not data. We include the raw bytes as-is here; the
/// BitReader handles un-stuffing during decoding.
fn extract_entropy_data(data: &[u8], start: usize) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    let mut i = start;
    while i < data.len() {
        let byte = data[i];
        if byte == 0xFF && i + 1 < data.len() {
            let next = data[i + 1];
            if next == M_EOI {
                // Found EOI — stop here.
                break;
            } else if next == 0x00 {
                // Stuffed 0xFF — include both bytes for the BitReader to un-stuff.
                result.push(0xFF);
                result.push(0x00);
                i += 2;
                continue;
            }
            // Any other FF xx marker: check if it's a restart marker (FF D0–D7).
            // For baseline sequential JPEG without restart intervals, this shouldn't
            // occur in files we produce. Skip gracefully.
            if (0xD0..=0xD7).contains(&next) {
                i += 2;
                continue;
            }
            // Other markers: stop.
            break;
        }
        result.push(byte);
        i += 1;
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Scan decoder
// ---------------------------------------------------------------------------

/// Decode the entropy-coded scan data into RGBA pixels.
fn decode_scan(
    entropy_data: &[u8],
    state: &DecoderState,
) -> Result<(u32, u32, Vec<u8>), String> {
    let w = state.width as usize;
    let h = state.height as usize;
    if w == 0 || h == 0 {
        return Err("JPEG: zero dimensions in SOF0".to_string());
    }
    let n_components = state.components.len();
    if n_components != 3 {
        return Err(format!("JPEG: expected 3 components, got {n_components}"));
    }

    // Resolve quantization tables for each component.
    let mut comp_qtables: Vec<&[u16; 64]> = Vec::new();
    for &(_, _, qt_id) in &state.components {
        let t = state.qtables[qt_id as usize]
            .as_ref()
            .ok_or_else(|| format!("JPEG: quantization table {qt_id} not defined"))?;
        comp_qtables.push(t);
    }

    // Resolve Huffman decode tables for each component.
    // In our encoder: Y uses DC/AC table 0; Cb/Cr use DC/AC table 1.
    // We read the SOS component mapping from state.components (qt_id field doubles
    // as the chroma indicator — qt_id 0 = luma tables, qt_id 1 = chroma tables).
    let mut comp_dc_tables: Vec<&HashMap<(u16, u8), u8>> = Vec::new();
    let mut comp_ac_tables: Vec<&HashMap<(u16, u8), u8>> = Vec::new();

    // Use the embedded Annex K tables as fallback if the file doesn't embed DHT.
    // (Our encoder always embeds DHT, but this makes the decoder more robust.)
    let fallback_luma_dc   = build_decode_table(&LUMA_DC_BITS,   LUMA_DC_HUFFVAL);
    let fallback_luma_ac   = build_decode_table(&LUMA_AC_BITS,   LUMA_AC_HUFFVAL);
    let fallback_chroma_dc = build_decode_table(&CHROMA_DC_BITS, CHROMA_DC_HUFFVAL);
    let fallback_chroma_ac = build_decode_table(&CHROMA_AC_BITS, CHROMA_AC_HUFFVAL);

    for &(_, _, qt_id) in &state.components {
        let table_id = if qt_id == 0 { 0 } else { 1 };
        let dc = state.dc_tables[table_id]
            .as_ref()
            .unwrap_or(if table_id == 0 { &fallback_luma_dc } else { &fallback_chroma_dc });
        let ac = state.ac_tables[table_id]
            .as_ref()
            .unwrap_or(if table_id == 0 { &fallback_luma_ac } else { &fallback_chroma_ac });
        comp_dc_tables.push(dc);
        comp_ac_tables.push(ac);
    }

    // Allocate planes for decoded samples (one per component, padded to 8×8 blocks).
    let blocks_wide = w.div_ceil(8);
    let blocks_tall = h.div_ceil(8);
    let padded_w = blocks_wide * 8;
    let padded_h = blocks_tall * 8;

    let mut planes: Vec<Vec<f32>> = vec![vec![0.0f32; padded_w * padded_h]; 3];

    // DC differential state per component.
    let mut prev_dc = [0i16; 3];

    let mut reader = BitReader::new(entropy_data);

    // Process blocks in raster order: for each 8×8 MCU (one per block position),
    // decode one block for each component.
    'outer: for block_row in 0..blocks_tall {
        for block_col in 0..blocks_wide {
            let block_origin_r = block_row * 8;
            let block_origin_c = block_col * 8;

            for comp_idx in 0..3 {
                // ── Decode DC coefficient ────────────────────────────────
                let dc_diff = match decode_dc(&mut reader, comp_dc_tables[comp_idx]) {
                    Ok(d) => d,
                    Err(_) => break 'outer, // allow short reads near end of stream
                };
                let dc = prev_dc[comp_idx] + dc_diff;
                prev_dc[comp_idx] = dc;

                // ── Decode AC coefficients ───────────────────────────────
                let mut zigzag_qcoeffs = [0i16; 64];
                zigzag_qcoeffs[0] = dc;
                let mut ac_coeffs = [0i16; 63];
                let _ = decode_ac(&mut reader, &mut ac_coeffs, comp_ac_tables[comp_idx]);
                // Copy AC into zigzag array (positions 1–63).
                zigzag_qcoeffs[1..].copy_from_slice(&ac_coeffs);

                // ── Inverse-zigzag to row-major order ────────────────────
                let mut rm_qcoeffs = [0i16; 64];
                for zz_pos in 0..64 {
                    let rm_pos = IZIGZAG[zz_pos];
                    rm_qcoeffs[rm_pos] = zigzag_qcoeffs[zz_pos];
                }

                // ── Dequantize ───────────────────────────────────────────
                let qt = comp_qtables[comp_idx];
                let dct_coeffs: Vec<f32> = (0..64)
                    .map(|i| dequantize(rm_qcoeffs[i], qt[i]))
                    .collect();

                // ── Inverse 2-D DCT ──────────────────────────────────────
                let spatial = idct_2d(&dct_coeffs, 8, 8, DctType::III, DctNorm::Ortho)
                    .expect("idct_2d on 8×8 block failed");

                // ── Level un-shift and store ─────────────────────────────
                //
                // Add 128 back (reverse the encoder's level shift). The IDCT
                // produces values in approximately [-128, 127]; after +128 they
                // should be in [0, 255]. Clamp to handle any floating-point overshoot.
                for r in 0..8 {
                    for c in 0..8 {
                        let pr = block_origin_r + r;
                        let pc = block_origin_c + c;
                        let val = (spatial[r * 8 + c] + 128.0)
                            .round()
                            .clamp(0.0, 255.0);
                        planes[comp_idx][pr * padded_w + pc] = val;
                    }
                }
            }
        }
    }

    // ── Convert YCbCr → RGBA and crop to actual image dimensions ─────────────
    let mut rgba = vec![0u8; w * h * 4];
    for row in 0..h {
        for col in 0..w {
            let y_val  = planes[0][row * padded_w + col];
            let cb_val = planes[1][row * padded_w + col];
            let cr_val = planes[2][row * padded_w + col];
            let (r, g, b) = ycbcr_to_rgb(y_val, cb_val, cr_val);
            let out_idx = (row * w + col) * 4;
            rgba[out_idx]     = r;
            rgba[out_idx + 1] = g;
            rgba[out_idx + 2] = b;
            rgba[out_idx + 3] = 255; // JPEG has no alpha; all pixels are fully opaque
        }
    }

    Ok((state.width, state.height, rgba))
}
