//! VP8 lossy codec — intra-only I-frame encoder and decoder.
//!
//! # Scope (v0.3.0)
//!
//! This implementation covers **intra-only** VP8 keyframes with full color:
//! - All macroblocks use 16×16 DC prediction (luma + chroma)
//! - YCbCr 4:2:0 color: Y plane (16×16 per MB) + Cb/Cr planes (8×8 per MB)
//! - Residuals are DC-only; AC coefficients are all zero (skip AC)
//! - Loop filter is disabled (loop_filter_level = 0)
//! - One DCT partition
//!
//! # VP8 frame structure
//!
//! ```text
//! ┌────────────────────────────────────────┐
//! │ 10-byte uncompressed header            │
//! │  (frame_tag 3B + start_code 3B +       │
//! │   width/height 4B)                     │
//! ├────────────────────────────────────────┤
//! │ Bool-coded first partition             │
//! │  frame header + all MB mode data       │
//! ├────────────────────────────────────────┤
//! │ DCT partition (1 of 1)                 │
//! │  DC coefficients: Y, Cb, Cr per MB     │
//! └────────────────────────────────────────┘
//! ```
//!
//! # Color model
//!
//! Pixels are converted RGB→YCbCr on encode and YCbCr→RGB on decode using
//! BT.601 integer arithmetic.  Each macroblock carries one Y DC coefficient
//! (for the 16×16 luma block) and one Cb + one Cr DC coefficient (for the
//! 8×8 chroma blocks), allowing full-color round-trips within the quantization
//! tolerance.

use pixel_container::PixelContainer;
use range_coder::{BoolDecoder, BoolEncoder};

mod quant;
mod wht;

pub use quant::qp_from_quality;

// ─────────────────────────────────────────────────────────────────────────────
// YCbCr ↔ RGB  (BT.601, full-range integer arithmetic)
// ─────────────────────────────────────────────────────────────────────────────

/// RGB → YCbCr using BT.601 integer approximation.
///
/// Coefficients (×256):
///   Y  =  77R + 150G +  29B           (range 0..255, no offset)
///   Cb = −43R −  85G + 128B  + 128    (range 0..255, neutral = 128)
///   Cr = 128R − 107G −  21B  + 128    (range 0..255, neutral = 128)
///
/// Note: −43−85+128=0 and 128−107−21=0, so grey inputs (R=G=B) always
/// produce Cb=Cr=128 exactly.
fn rgb_to_ycbcr(r: u8, g: u8, b: u8) -> (i32, i32, i32) {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let y  = ( 77 * r + 150 * g +  29 * b + 128) >> 8;
    let cb = ((-43 * r -  85 * g + 128 * b + 128) >> 8) + 128;
    let cr = ((128 * r - 107 * g -  21 * b + 128) >> 8) + 128;
    (y, cb.clamp(0, 255), cr.clamp(0, 255))
}

/// YCbCr → RGB using BT.601 integer approximation (inverse of `rgb_to_ycbcr`).
///
/// Coefficients (×256):
///   R = 256Y + 359(Cr−128)
///   G = 256Y −  88(Cb−128) − 183(Cr−128)
///   B = 256Y + 454(Cb−128)
fn ycbcr_to_rgb(y: i32, cb: i32, cr: i32) -> (u8, u8, u8) {
    let r = (256 * y + 359 * (cr - 128)                       + 128) >> 8;
    let g = (256 * y -  88 * (cb - 128) - 183 * (cr - 128)   + 128) >> 8;
    let b = (256 * y + 454 * (cb - 128)                       + 128) >> 8;
    (r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, b.clamp(0, 255) as u8)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry points
// ─────────────────────────────────────────────────────────────────────────────

/// Encode `pixels` as a VP8 lossy keyframe.
///
/// `quality` is in [0, 100]; 100 = highest quality (smallest quantization
/// step), 0 = lowest quality. Returns the raw VP8 bitstream bytes — the
/// caller wraps this in a RIFF/WEBP/VP8 container.
pub fn encode(pixels: &PixelContainer, quality: u8) -> Vec<u8> {
    let width  = pixels.width  as u16;
    let height = pixels.height as u16;
    let qp     = qp_from_quality(quality);

    // ── First partition (bool-coded frame header + MB modes) ─────────────
    let (first_part, mb_skips) = encode_first_partition(width, height, qp, pixels);

    // ── DCT partition (Y/Cb/Cr DC coefficients for non-skipped MBs) ──────
    let dct_part = encode_dct_partition(pixels, qp, &mb_skips);

    // ── Assemble: 10-byte uncompressed header + first_part + dct_part ────
    let first_part_size = first_part.len() as u32;
    let mut out = Vec::with_capacity(10 + first_part.len() + dct_part.len());

    // frame_tag (3 bytes, little-endian):
    //   bits [2:0]  = frame_type (0 = keyframe)
    //   bits [5:3]  = version   (0)
    //   bit  [6]    = show_frame (1)
    //   bits [25:7] = first_part_size
    let frame_tag: u32 = (1 << 6) | (first_part_size << 7);
    out.extend_from_slice(&frame_tag.to_le_bytes()[..3]);

    // Start code for keyframes
    out.extend_from_slice(&[0x9D, 0x01, 0x2A]);

    // Horizontal: scale(2) | width(14), little-endian
    out.extend_from_slice(&width.to_le_bytes());
    // Vertical:   scale(2) | height(14), little-endian
    out.extend_from_slice(&height.to_le_bytes());

    out.extend_from_slice(&first_part);
    out.extend_from_slice(&dct_part);
    out
}

/// Decode a VP8 keyframe bitstream back to a `PixelContainer`.
///
/// `data` is the raw VP8 data (not RIFF-wrapped). Returns `Err` on any
/// format violation.
pub fn decode(data: &[u8]) -> Result<PixelContainer, String> {
    if data.len() < 10 {
        return Err("VP8: frame too short (< 10 bytes)".to_string());
    }

    // ── Parse 10-byte uncompressed header ────────────────────────────────
    let frame_tag = u32::from_le_bytes([data[0], data[1], data[2], 0]);
    let frame_type       = frame_tag & 0x1;
    let show_frame       = (frame_tag >> 6) & 0x1;
    let first_part_size  = (frame_tag >> 7) as usize;

    if frame_type != 0 {
        return Err("VP8: only keyframes supported".to_string());
    }
    if show_frame == 0 {
        return Err("VP8: show_frame must be 1".to_string());
    }
    if data[3..6] != [0x9D, 0x01, 0x2A] {
        return Err("VP8: invalid keyframe start code".to_string());
    }

    let width  = u16::from_le_bytes([data[6], data[7]]) & 0x3FFF;
    let height = u16::from_le_bytes([data[8], data[9]]) & 0x3FFF;

    if width == 0 || height == 0 {
        return Err("VP8: zero dimensions".to_string());
    }
    if 10 + first_part_size > data.len() {
        return Err("VP8: first partition extends past end of data".to_string());
    }

    let first_part = &data[10..10 + first_part_size];
    let dct_part   = &data[10 + first_part_size..];

    // ── Decode first partition ────────────────────────────────────────────
    let (qp, mb_skips) = decode_first_partition(
        first_part, width as u32, height as u32,
    )?;

    // ── Decode DCT partition + reconstruct pixels ─────────────────────────
    decode_dct_partition(dct_part, width as u32, height as u32, qp, &mb_skips)
}

// ─────────────────────────────────────────────────────────────────────────────
// First partition — encode
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `(bool_coded_bytes, mb_skips)` where `mb_skips[i]` is true when
/// macroblock `i` has all-zero DCT residuals (Y and Cb and Cr all zero).
fn encode_first_partition(
    width: u16, height: u16, qp: u8, pixels: &PixelContainer,
) -> (Vec<u8>, Vec<bool>) {
    let mb_cols = (width as u32).div_ceil(16);
    let mb_rows = (height as u32).div_ceil(16);

    let mut enc = BoolEncoder::new();

    // ── Frame header fields ───────────────────────────────────────────────
    enc.write_bit(false, 128); // color_space = 0 (YCbCr)
    enc.write_bit(false, 128); // clamping_type = 0

    // No segment updates
    enc.write_bit(false, 128); // update_mb_segmentation = 0

    // Filter: simple filter disabled (filter_type=0, level=0, sharpness=0, no adj)
    enc.write_bit(false, 128); // filter_type = 0 (normal)
    enc.write_bits(0, 6);      // loop_filter_level = 0
    enc.write_bits(0, 3);      // sharpness_level = 0
    enc.write_bit(false, 128); // lf_adj_enable = 0

    // log2_nbr_of_dct_partitions = 0 (one partition)
    enc.write_bits(0, 2);

    // Dequantization: base QP = qp, all deltas = 0
    enc.write_bits(qp as u32, 7); // y1_ac_qi
    enc.write_bit(false, 128);    // y1_dc_delta_present = 0
    enc.write_bit(false, 128);    // y2_dc_delta_present = 0
    enc.write_bit(false, 128);    // y2_ac_delta_present = 0
    enc.write_bit(false, 128);    // uv_dc_delta_present = 0
    enc.write_bit(false, 128);    // uv_ac_delta_present = 0

    // Token probability updates: 96 flags, all 0 (no updates)
    for _ in 0..96 {
        enc.write_bit(false, 128);
    }

    // mb_no_coeff_skip = 1 (enable per-MB skip signalling)
    enc.write_bit(true, 128);
    // prob_skip_false = 1 (almost never have residuals → 1/255 chance)
    enc.write_bits(1, 8);

    // ── Macroblock mode data ──────────────────────────────────────────────
    let mb_skips = compute_mb_skips(pixels, qp, mb_cols, mb_rows);

    for &skip in &mb_skips {
        // skip_coeff: 1 = all residuals zero (prob_skip_false=1 → P(no skip)=1/255)
        enc.write_bit(skip, 1);

        // Intra 16×16 luma mode = DC_PRED (1) for all MBs
        enc.write_bit(true,  145); // is_16x16 (not B_PRED)
        enc.write_bit(false, 156); // DC_PRED (vs V/H/TM)

        // UV mode = DC_PRED
        enc.write_bit(false, 142); // bit0: DC(0) vs non-DC
    }

    (enc.finish(), mb_skips)
}

// ─────────────────────────────────────────────────────────────────────────────
// First partition — decode
// ─────────────────────────────────────────────────────────────────────────────

fn decode_first_partition(
    data: &[u8], width: u32, height: u32,
) -> Result<(u8, Vec<bool>), String> {
    if data.len() < 2 {
        return Err("VP8: first partition too short".to_string());
    }
    let mut dec = BoolDecoder::new(data);

    dec.read_bit(128); // color_space
    dec.read_bit(128); // clamping_type

    let has_seg = dec.read_bit(128);
    if has_seg {
        return Err("VP8: segmentation not supported in v1".to_string());
    }

    dec.read_bit(128); // filter_type
    dec.read_bits(6);  // loop_filter_level
    dec.read_bits(3);  // sharpness_level
    let lf_adj = dec.read_bit(128);
    if lf_adj {
        return Err("VP8: lf_adj not supported in v1".to_string());
    }

    dec.read_bits(2); // log2_nbr_of_dct_partitions

    let qp = dec.read_bits(7) as u8;
    if dec.read_bit(128) { dec.read_bits(5); } // y1_dc_delta
    if dec.read_bit(128) { dec.read_bits(5); } // y2_dc_delta
    if dec.read_bit(128) { dec.read_bits(5); } // y2_ac_delta
    if dec.read_bit(128) { dec.read_bits(5); } // uv_dc_delta
    if dec.read_bit(128) { dec.read_bits(5); } // uv_ac_delta

    // Token probability updates (96 flags)
    for _ in 0..96 {
        let update = dec.read_bit(128);
        if update { dec.read_bits(8); }
    }

    let mb_no_skip = dec.read_bit(128);
    let _prob_skip_false = if mb_no_skip { dec.read_bits(8) as u8 } else { 0 };

    let mb_cols = width.div_ceil(16);
    let mb_rows = height.div_ceil(16);
    let num_mb  = (mb_cols * mb_rows) as usize;

    let mut mb_skips = Vec::with_capacity(num_mb);
    for _ in 0..num_mb {
        let skip = if mb_no_skip { dec.read_bit(1) } else { false };
        mb_skips.push(skip);

        let is_16x16 = dec.read_bit(145);
        if !is_16x16 {
            // B_PRED: 4×4 modes — drain 16 × 4 bits as best effort
            for _ in 0..16 { dec.read_bits(4); }
        } else {
            dec.read_bit(156); // luma mode
        }
        dec.read_bit(142); // UV mode
    }

    Ok((qp, mb_skips))
}

// ─────────────────────────────────────────────────────────────────────────────
// DCT partition — encode
// ─────────────────────────────────────────────────────────────────────────────

/// Determine which MBs have all-zero quantized residuals (Y, Cb, Cr all zero).
fn compute_mb_skips(
    pixels: &PixelContainer, qp: u8, mb_cols: u32, mb_rows: u32,
) -> Vec<bool> {
    let y_step  = quant::dc_quant_step(qp);
    let uv_step = quant::uv_quant_step(qp);
    let mut skips = Vec::with_capacity((mb_cols * mb_rows) as usize);

    let mut top_y  = vec![128i32; (mb_cols * 16) as usize];
    let mut left_y = vec![128i32; 16usize];
    let mut top_cb  = vec![128i32; (mb_cols * 8) as usize];
    let mut left_cb = vec![128i32; 8usize];
    let mut top_cr  = vec![128i32; (mb_cols * 8) as usize];
    let mut left_cr = vec![128i32; 8usize];

    for mb_row in 0..mb_rows {
        for mb_col in 0..mb_cols {
            let (y_avg, cb_avg, cr_avg) = compute_mb_ycbcr_avg(pixels, mb_row, mb_col);

            let y_pred  = dc_16x16_predictor(mb_row, mb_col, &top_y,  &left_y,  mb_cols);
            let cb_pred = dc_8x8_predictor  (mb_row, mb_col, &top_cb, &left_cb);
            let cr_pred = dc_8x8_predictor  (mb_row, mb_col, &top_cr, &left_cr);

            let y_q  = quant::quantize(y_avg  - y_pred,  y_step);
            let cb_q = quant::quantize(cb_avg - cb_pred, uv_step);
            let cr_q = quant::quantize(cr_avg - cr_pred, uv_step);

            let recon_y  = (y_pred  + quant::dequantize(y_q,  y_step )).clamp(0, 255);
            let recon_cb = (cb_pred + quant::dequantize(cb_q, uv_step)).clamp(0, 255);
            let recon_cr = (cr_pred + quant::dequantize(cr_q, uv_step)).clamp(0, 255);

            update_luma_context  (&mut top_y,  &mut left_y,  mb_col, recon_y);
            update_chroma_context(&mut top_cb, &mut left_cb, mb_col, recon_cb);
            update_chroma_context(&mut top_cr, &mut left_cr, mb_col, recon_cr);

            skips.push(y_q == 0 && cb_q == 0 && cr_q == 0);
        }
    }
    skips
}

fn encode_dct_partition(
    pixels: &PixelContainer, qp: u8, mb_skips: &[bool],
) -> Vec<u8> {
    let mb_cols = pixels.width.div_ceil(16);
    let mb_rows = pixels.height.div_ceil(16);
    let y_step  = quant::dc_quant_step(qp);
    let uv_step = quant::uv_quant_step(qp);

    let mut enc = BoolEncoder::new();

    let mut top_y  = vec![128i32; (mb_cols * 16) as usize];
    let mut left_y = vec![128i32; 16usize];
    let mut top_cb  = vec![128i32; (mb_cols * 8) as usize];
    let mut left_cb = vec![128i32; 8usize];
    let mut top_cr  = vec![128i32; (mb_cols * 8) as usize];
    let mut left_cr = vec![128i32; 8usize];

    let mut mb_idx = 0;
    for mb_row in 0..mb_rows {
        for mb_col in 0..mb_cols {
            let y_pred  = dc_16x16_predictor(mb_row, mb_col, &top_y,  &left_y,  mb_cols);
            let cb_pred = dc_8x8_predictor  (mb_row, mb_col, &top_cb, &left_cb);
            let cr_pred = dc_8x8_predictor  (mb_row, mb_col, &top_cr, &left_cr);

            let (recon_y, recon_cb, recon_cr) = if mb_skips[mb_idx] {
                (y_pred, cb_pred, cr_pred)
            } else {
                let (y_avg, cb_avg, cr_avg) = compute_mb_ycbcr_avg(pixels, mb_row, mb_col);
                let y_q  = quant::quantize(y_avg  - y_pred,  y_step);
                let cb_q = quant::quantize(cb_avg - cb_pred, uv_step);
                let cr_q = quant::quantize(cr_avg - cr_pred, uv_step);
                encode_coeff(&mut enc, y_q);
                encode_coeff(&mut enc, cb_q);
                encode_coeff(&mut enc, cr_q);
                let ry  = (y_pred  + quant::dequantize(y_q,  y_step )).clamp(0, 255);
                let rcb = (cb_pred + quant::dequantize(cb_q, uv_step)).clamp(0, 255);
                let rcr = (cr_pred + quant::dequantize(cr_q, uv_step)).clamp(0, 255);
                (ry, rcb, rcr)
            };

            update_luma_context  (&mut top_y,  &mut left_y,  mb_col, recon_y);
            update_chroma_context(&mut top_cb, &mut left_cb, mb_col, recon_cb);
            update_chroma_context(&mut top_cr, &mut left_cr, mb_col, recon_cr);
            mb_idx += 1;
        }
    }
    enc.finish()
}

// ─────────────────────────────────────────────────────────────────────────────
// DCT partition — decode
// ─────────────────────────────────────────────────────────────────────────────

fn decode_dct_partition(
    data: &[u8],
    width: u32, height: u32, qp: u8,
    mb_skips: &[bool],
) -> Result<PixelContainer, String> {
    let mb_cols = width.div_ceil(16);
    let mb_rows = height.div_ceil(16);
    let y_step  = quant::dc_quant_step(qp);
    let uv_step = quant::uv_quant_step(qp);

    let mut dec = if data.len() >= 2 {
        BoolDecoder::new(data)
    } else {
        BoolDecoder::new(&[0, 0])
    };

    let mut rgba = vec![0u8; (width * height * 4) as usize];

    let mut top_y  = vec![128i32; (mb_cols * 16) as usize];
    let mut left_y = vec![128i32; 16usize];
    let mut top_cb  = vec![128i32; (mb_cols * 8) as usize];
    let mut left_cb = vec![128i32; 8usize];
    let mut top_cr  = vec![128i32; (mb_cols * 8) as usize];
    let mut left_cr = vec![128i32; 8usize];

    let mut mb_idx = 0;
    for mb_row in 0..mb_rows {
        for mb_col in 0..mb_cols {
            let y_pred  = dc_16x16_predictor(mb_row, mb_col, &top_y,  &left_y,  mb_cols);
            let cb_pred = dc_8x8_predictor  (mb_row, mb_col, &top_cb, &left_cb);
            let cr_pred = dc_8x8_predictor  (mb_row, mb_col, &top_cr, &left_cr);

            let (recon_y, recon_cb, recon_cr) = if mb_skips[mb_idx] {
                (y_pred, cb_pred, cr_pred)
            } else {
                let y_q  = decode_coeff(&mut dec);
                let cb_q = decode_coeff(&mut dec);
                let cr_q = decode_coeff(&mut dec);
                let ry  = (y_pred  + quant::dequantize(y_q,  y_step )).clamp(0, 255);
                let rcb = (cb_pred + quant::dequantize(cb_q, uv_step)).clamp(0, 255);
                let rcr = (cr_pred + quant::dequantize(cr_q, uv_step)).clamp(0, 255);
                (ry, rcb, rcr)
            };

            update_luma_context  (&mut top_y,  &mut left_y,  mb_col, recon_y);
            update_chroma_context(&mut top_cb, &mut left_cb, mb_col, recon_cb);
            update_chroma_context(&mut top_cr, &mut left_cr, mb_col, recon_cr);

            fill_macroblock(
                &mut rgba, width, height, mb_row, mb_col,
                recon_y as u8, recon_cb as u8, recon_cr as u8,
            );
            mb_idx += 1;
        }
    }

    Ok(PixelContainer::from_data(width, height, rgba))
}

// ─────────────────────────────────────────────────────────────────────────────
// Macroblock helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Average Y, Cb, Cr across the 16×16 luma block of macroblock (mb_row, mb_col).
///
/// Chroma averaging covers all pixels in the MB (not just 4:2:0 sub-samples)
/// because we're computing a single scalar DC value per block.
fn compute_mb_ycbcr_avg(
    pixels: &PixelContainer, mb_row: u32, mb_col: u32,
) -> (i32, i32, i32) {
    let x0 = mb_col * 16;
    let y0 = mb_row * 16;
    let mut y_sum = 0i32; let mut cb_sum = 0i32; let mut cr_sum = 0i32;
    let mut count = 0i32;
    for dy in 0..16u32 {
        for dx in 0..16u32 {
            let x = x0 + dx;
            let y = y0 + dy;
            if x < pixels.width && y < pixels.height {
                let (r, g, b, _) = pixels.pixel_at(x, y);
                let (yv, cb, cr) = rgb_to_ycbcr(r, g, b);
                y_sum  += yv; cb_sum += cb; cr_sum += cr;
                count  += 1;
            }
        }
    }
    if count == 0 { return (128, 128, 128); }
    (
        (y_sum  + count / 2) / count,
        (cb_sum + count / 2) / count,
        (cr_sum + count / 2) / count,
    )
}

/// 16×16 DC predictor for luma — average of 16 pixels above + 16 pixels left.
fn dc_16x16_predictor(
    mb_row: u32, mb_col: u32,
    top_row: &[i32], left_col: &[i32],
    _mb_cols: u32,
) -> i32 {
    let have_top  = mb_row > 0;
    let have_left = mb_col > 0;
    let x0 = (mb_col * 16) as usize;
    if have_top && have_left {
        let t: i32 = top_row[x0..x0 + 16].iter().sum();
        let l: i32 = left_col.iter().sum();
        (t + l + 16) / 32
    } else if have_top {
        (top_row[x0..x0 + 16].iter().sum::<i32>() + 8) / 16
    } else if have_left {
        (left_col.iter().sum::<i32>() + 8) / 16
    } else {
        128
    }
}

/// 8×8 DC predictor for chroma — average of 8 pixels above + 8 pixels left.
fn dc_8x8_predictor(
    mb_row: u32, mb_col: u32,
    top_row: &[i32], left_col: &[i32],
) -> i32 {
    let have_top  = mb_row > 0;
    let have_left = mb_col > 0;
    let x0 = (mb_col * 8) as usize;
    if have_top && have_left {
        let t: i32 = top_row[x0..x0 + 8].iter().sum();
        let l: i32 = left_col.iter().sum();
        (t + l + 8) / 16
    } else if have_top {
        (top_row[x0..x0 + 8].iter().sum::<i32>() + 4) / 8
    } else if have_left {
        (left_col.iter().sum::<i32>() + 4) / 8
    } else {
        128
    }
}

fn update_luma_context(top_row: &mut [i32], left_col: &mut [i32], mb_col: u32, recon: i32) {
    let x0 = (mb_col * 16) as usize;
    for i in 0..16 { top_row[x0 + i] = recon; left_col[i] = recon; }
}

fn update_chroma_context(top_row: &mut [i32], left_col: &mut [i32], mb_col: u32, recon: i32) {
    let x0 = (mb_col * 8) as usize;
    for i in 0..8 { top_row[x0 + i] = recon; left_col[i] = recon; }
}

/// Fill a 16×16 macroblock with the reconstructed YCbCr values, converted to RGBA.
fn fill_macroblock(
    rgba: &mut [u8], width: u32, height: u32,
    mb_row: u32, mb_col: u32, y: u8, cb: u8, cr: u8,
) {
    let (r, g, b) = ycbcr_to_rgb(y as i32, cb as i32, cr as i32);
    let x0 = mb_col * 16;
    let y0 = mb_row * 16;
    for dy in 0..16u32 {
        for dx in 0..16u32 {
            let x = x0 + dx;
            let py = y0 + dy;
            if x < width && py < height {
                let idx = ((py * width + x) * 4) as usize;
                rgba[idx]     = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Coefficient encoding / decoding (signed integer via bool coder)
// ─────────────────────────────────────────────────────────────────────────────

fn encode_coeff(enc: &mut BoolEncoder, coeff: i32) {
    if coeff == 0 {
        enc.write_bit(true, 128); // is_zero
        return;
    }
    enc.write_bit(false, 128);        // not zero
    enc.write_bit(coeff < 0, 128);    // sign
    let mag = coeff.unsigned_abs();
    enc.write_bits(mag - 1, 16); // magnitude - 1
}

fn decode_coeff(dec: &mut BoolDecoder) -> i32 {
    let is_zero = dec.read_bit(128);
    if is_zero { return 0; }
    let negative = dec.read_bit(128);
    let mag = dec.read_bits(16) as i32 + 1;
    if negative { -mag } else { mag }
}
