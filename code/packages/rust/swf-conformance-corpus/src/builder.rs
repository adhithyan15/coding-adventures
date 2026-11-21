//! Clean-room synthetic SWF binary builder (`SwfBuilder`) supporting `FWS` and `CWS` (`SWF08`).
//!
//! Produces deterministic, bounded `.swf` binary payloads for all `SWF01..SWF07` layers
//! without external binary fixtures or filesystem I/O.

#![forbid(unsafe_code)]

use swf_container::{compute_adler32, SwfColorTransform, SwfMatrix, SwfRect};

use crate::bit_writer::{min_signed_bit_width, SwfBitWriter};
use crate::error::{CorpusError, MAX_BUILDER_TAGS, MAX_CORPUS_SWF_BYTES};
use crate::{validate_corpus_frame_count, validate_corpus_swf_bytes};

/// Container compression mode for synthetic SWF output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwfCompression {
    /// Uncompressed SWF (`FWS`).
    Fws,
    /// Zlib-compressed SWF (`CWS` via `deflate::compress` + RFC 1950 header/Adler-32).
    Cws,
}

/// Fill style specification for synthetic `DefineShape1..4` tags.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeFillSpec {
    /// Solid RGBA color (`alpha` ignored in `DefineShape1/2`).
    Solid(u8, u8, u8, u8),
    /// Linear gradient (`0x10`) with gradient-space matrix and `(ratio, (r, g, b, a))` stops.
    LinearGradient {
        matrix: SwfMatrix,
        stops: Vec<(u8, (u8, u8, u8, u8))>,
    },
    /// Radial gradient (`0x12`) with gradient-space matrix and `(ratio, (r, g, b, a))` stops.
    RadialGradient {
        matrix: SwfMatrix,
        stops: Vec<(u8, (u8, u8, u8, u8))>,
    },
}

/// Stroke style specification for synthetic `DefineShape1..4` tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeLineSpec {
    /// Stroke width in twips (`20 twips = 1 px`).
    pub width_twips: u16,
    /// Stroke RGBA color.
    pub color: (u8, u8, u8, u8),
}

/// Vector path record specification for synthetic shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeRecordSpec {
    /// Style-change / move-to record (`TypeFlag = 0`).
    StyleChange {
        move_to: Option<(i32, i32)>,
        fill_style0: Option<u32>,
        fill_style1: Option<u32>,
        line_style: Option<u32>,
    },
    /// Straight edge delta (`dx`, `dy` in twips).
    StraightEdge { dx: i32, dy: i32 },
    /// Quadratic Bézier edge (`control_dx/dy` from pen, `anchor_dx/dy` from control, in twips).
    CurvedEdge {
        control_dx: i32,
        control_dy: i32,
        anchor_dx: i32,
        anchor_dy: i32,
    },
}

/// Parameters for encoding a `DefineShape` (2), `DefineShape2` (22), `DefineShape3` (32), or `DefineShape4` (83) tag.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDefSpec {
    /// SWF tag code (`2`, `22`, `32`, or `83`).
    pub tag_code: u16,
    /// Dictionary character ID (`>= 1`).
    pub shape_id: u16,
    /// Shape bounds in twips.
    pub bounds: SwfRect,
    /// Optional non-zero winding flag (`DefineShape4` only).
    pub uses_non_zero_winding: bool,
    /// 1-based fill styles.
    pub fill_styles: Vec<ShapeFillSpec>,
    /// 1-based line styles.
    pub line_styles: Vec<ShapeLineSpec>,
    /// Ordered shape records.
    pub records: Vec<ShapeRecordSpec>,
}

/// Parameters for encoding `PlaceObject2` (Tag 26) or `PlaceObject3` (Tag 70).
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceTagSpec {
    pub depth: u16,
    pub is_move: bool,
    pub character_id: Option<u16>,
    pub matrix: Option<SwfMatrix>,
    pub color_transform: Option<SwfColorTransform>,
    pub ratio: Option<u16>,
    pub name: Option<String>,
    pub clip_depth: Option<u16>,
    pub blend_mode: Option<u8>,
    pub visible: Option<bool>,
}

impl PlaceTagSpec {
    /// Creates a placement spec for a new character at `depth`.
    pub fn place_new(depth: u16, character_id: u16) -> Self {
        Self {
            depth,
            is_move: false,
            character_id: Some(character_id),
            matrix: None,
            color_transform: None,
            ratio: None,
            name: None,
            clip_depth: None,
            blend_mode: None,
            visible: None,
        }
    }

    /// Creates a move/update spec for an existing character at `depth`.
    pub fn place_move(depth: u16) -> Self {
        Self {
            depth,
            is_move: true,
            character_id: None,
            matrix: None,
            color_transform: None,
            ratio: None,
            name: None,
            clip_depth: None,
            blend_mode: None,
            visible: None,
        }
    }
}

/// Encodes an SWF short (`< 0x3F`) or long (`>= 0x3F`) tag frame.
pub fn encode_swf_tag(code: u16, payload: &[u8]) -> Result<Vec<u8>, CorpusError> {
    if code > 1023 {
        return Err(CorpusError::BitWidthExceeded);
    }
    let total_est = 6usize
        .checked_add(payload.len())
        .ok_or(CorpusError::IntegerOverflow)?;
    if total_est > MAX_CORPUS_SWF_BYTES {
        return Err(CorpusError::SwfSizeExceeded);
    }

    let mut out = Vec::with_capacity(total_est);
    if payload.len() < 0x3F {
        let len_u16 = u16::try_from(payload.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        let header_word = (code << 6) | len_u16;
        out.extend_from_slice(&header_word.to_le_bytes());
    } else {
        let header_word = (code << 6) | 0x3F;
        let len_i32 = i32::try_from(payload.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        out.extend_from_slice(&header_word.to_le_bytes());
        out.extend_from_slice(&len_i32.to_le_bytes());
    }
    out.extend_from_slice(payload);
    Ok(out)
}

fn write_fill_style(
    out: &mut Vec<u8>,
    tag_code: u16,
    fill: &ShapeFillSpec,
) -> Result<(), CorpusError> {
    match fill {
        ShapeFillSpec::Solid(r, g, b, a) => {
            out.push(0x00);
            out.push(*r);
            out.push(*g);
            out.push(*b);
            if tag_code >= 32 {
                out.push(*a);
            }
        }
        ShapeFillSpec::LinearGradient { matrix, stops }
        | ShapeFillSpec::RadialGradient { matrix, stops } => {
            if stops.is_empty() || stops.len() > 15 {
                return Err(CorpusError::UnexpectedCaseFailure(
                    "gradient stop count must be 1..=15".to_string(),
                ));
            }
            let kind_byte = if matches!(fill, ShapeFillSpec::LinearGradient { .. }) {
                0x10
            } else {
                0x12
            };
            out.push(kind_byte);

            let mut mw = SwfBitWriter::bit_writer_new();
            mw.bit_writer_write_matrix(matrix)?;
            out.extend_from_slice(&mw.bit_writer_into_bytes()?);

            // SpreadMode=0 (Pad), InterpolationMode=0 (NormalRGB), NumGradients=stops.len()
            let count_u8 = u8::try_from(stops.len()).map_err(|_| CorpusError::IntegerOverflow)?;
            out.push(count_u8 & 0x0F);

            for &(ratio, (r, g, b, a)) in stops {
                out.push(ratio);
                out.push(r);
                out.push(g);
                out.push(b);
                if tag_code >= 32 {
                    out.push(a);
                }
            }
        }
    }
    Ok(())
}

fn write_line_style(
    out: &mut Vec<u8>,
    tag_code: u16,
    line: &ShapeLineSpec,
) -> Result<(), CorpusError> {
    out.extend_from_slice(&line.width_twips.to_le_bytes());
    if tag_code == 83 {
        // LINESTYLE2: Round caps (0), Round join (0), no fill, scaling enabled, no_close=0
        let mut bw = SwfBitWriter::bit_writer_new();
        bw.bit_writer_write_ub(0, 2)?; // start_cap = Round
        bw.bit_writer_write_ub(0, 2)?; // join_style = Round
        bw.bit_writer_write_ub(0, 1)?; // has_fill = 0
        bw.bit_writer_write_ub(0, 1)?; // no_h_scale = 0
        bw.bit_writer_write_ub(0, 1)?; // no_v_scale = 0
        bw.bit_writer_write_ub(0, 1)?; // pixel_hinting = 0
        bw.bit_writer_write_ub(0, 5)?; // reserved = 0
        bw.bit_writer_write_ub(0, 1)?; // no_close = 0
        bw.bit_writer_write_ub(0, 2)?; // end_cap = Round
        out.extend_from_slice(&bw.bit_writer_into_bytes()?);
        out.extend_from_slice(&[line.color.0, line.color.1, line.color.2, line.color.3]);
    } else {
        out.extend_from_slice(&[line.color.0, line.color.1, line.color.2]);
        if tag_code >= 32 {
            out.push(line.color.3);
        }
    }
    Ok(())
}

/// Encodes a `DefineShape` (2), `DefineShape2` (22), `DefineShape3` (32), or `DefineShape4` (83) tag.
pub fn encode_define_shape_tag(spec: &ShapeDefSpec) -> Result<Vec<u8>, CorpusError> {
    if !matches!(spec.tag_code, 2 | 22 | 32 | 83) {
        return Err(CorpusError::UnexpectedCaseFailure(
            "invalid DefineShape tag code".to_string(),
        ));
    }
    if spec.shape_id == 0 {
        return Err(CorpusError::UnexpectedCaseFailure(
            "shape_id must be >= 1".to_string(),
        ));
    }

    let mut payload = Vec::new();
    payload.extend_from_slice(&spec.shape_id.to_le_bytes());

    let mut bw = SwfBitWriter::bit_writer_new();
    bw.bit_writer_write_rect(
        spec.bounds.xmin,
        spec.bounds.xmax,
        spec.bounds.ymin,
        spec.bounds.ymax,
    )?;
    if spec.tag_code == 83 {
        bw.bit_writer_write_rect(
            spec.bounds.xmin,
            spec.bounds.xmax,
            spec.bounds.ymin,
            spec.bounds.ymax,
        )?;
        bw.bit_writer_write_ub(0, 5)?; // reserved
        bw.bit_writer_write_ub(u32::from(spec.uses_non_zero_winding), 1)?;
        bw.bit_writer_write_ub(0, 1)?; // non_scaling_strokes
        bw.bit_writer_write_ub(1, 1)?; // scaling_strokes
    }
    payload.extend_from_slice(&bw.bit_writer_into_bytes()?);

    if spec.fill_styles.len() >= 0xFF || spec.line_styles.len() >= 0xFF {
        return Err(CorpusError::UnexpectedCaseFailure(
            "style count exceeds single-byte encoding bound".to_string(),
        ));
    }
    payload.push(spec.fill_styles.len() as u8);
    for fill in &spec.fill_styles {
        write_fill_style(&mut payload, spec.tag_code, fill)?;
    }

    payload.push(spec.line_styles.len() as u8);
    for line in &spec.line_styles {
        write_line_style(&mut payload, spec.tag_code, line)?;
    }

    let max_fill_idx = spec.fill_styles.len() as u32;
    let max_line_idx = spec.line_styles.len() as u32;
    let num_fill_bits = if max_fill_idx == 0 {
        0
    } else {
        crate::bit_writer::min_unsigned_bit_width(max_fill_idx)?
    };
    let num_line_bits = if max_line_idx == 0 {
        0
    } else {
        crate::bit_writer::min_unsigned_bit_width(max_line_idx)?
    };
    if num_fill_bits > 15 || num_line_bits > 15 {
        return Err(CorpusError::BitWidthExceeded);
    }

    let mut sw = SwfBitWriter::bit_writer_new();
    sw.bit_writer_write_ub(num_fill_bits as u32, 4)?;
    sw.bit_writer_write_ub(num_line_bits as u32, 4)?;

    for rec in &spec.records {
        match *rec {
            ShapeRecordSpec::StyleChange {
                move_to,
                fill_style0,
                fill_style1,
                line_style,
            } => {
                sw.bit_writer_write_ub(0, 1)?; // TypeFlag = 0
                sw.bit_writer_write_ub(0, 1)?; // StateNewStyles = 0
                sw.bit_writer_write_ub(u32::from(line_style.is_some()), 1)?;
                sw.bit_writer_write_ub(u32::from(fill_style1.is_some()), 1)?;
                sw.bit_writer_write_ub(u32::from(fill_style0.is_some()), 1)?;
                sw.bit_writer_write_ub(u32::from(move_to.is_some()), 1)?;

                if let Some((mx, my)) = move_to {
                    let mbits = min_signed_bit_width(&[mx, my])?;
                    sw.bit_writer_write_ub(mbits as u32, 5)?;
                    sw.bit_writer_write_sb(mx, mbits)?;
                    sw.bit_writer_write_sb(my, mbits)?;
                }
                if let Some(f0) = fill_style0 {
                    sw.bit_writer_write_ub(f0, num_fill_bits)?;
                }
                if let Some(f1) = fill_style1 {
                    sw.bit_writer_write_ub(f1, num_fill_bits)?;
                }
                if let Some(ls) = line_style {
                    sw.bit_writer_write_ub(ls, num_line_bits)?;
                }
            }
            ShapeRecordSpec::StraightEdge { dx, dy } => {
                let nbits = min_signed_bit_width(&[dx, dy])?.max(2);
                if nbits > 17 {
                    return Err(CorpusError::BitWidthExceeded);
                }
                sw.bit_writer_write_ub(1, 1)?; // TypeFlag = 1
                sw.bit_writer_write_ub(1, 1)?; // StraightFlag = 1
                sw.bit_writer_write_ub((nbits - 2) as u32, 4)?;
                sw.bit_writer_write_ub(1, 1)?; // GeneralLineFlag = 1
                sw.bit_writer_write_sb(dx, nbits)?;
                sw.bit_writer_write_sb(dy, nbits)?;
            }
            ShapeRecordSpec::CurvedEdge {
                control_dx,
                control_dy,
                anchor_dx,
                anchor_dy,
            } => {
                let nbits =
                    min_signed_bit_width(&[control_dx, control_dy, anchor_dx, anchor_dy])?.max(2);
                if nbits > 17 {
                    return Err(CorpusError::BitWidthExceeded);
                }
                sw.bit_writer_write_ub(1, 1)?; // TypeFlag = 1
                sw.bit_writer_write_ub(0, 1)?; // StraightFlag = 0 (curved)
                sw.bit_writer_write_ub((nbits - 2) as u32, 4)?;
                sw.bit_writer_write_sb(control_dx, nbits)?;
                sw.bit_writer_write_sb(control_dy, nbits)?;
                sw.bit_writer_write_sb(anchor_dx, nbits)?;
                sw.bit_writer_write_sb(anchor_dy, nbits)?;
            }
        }
    }

    // EndShapeRecord (TypeFlag = 0, 5 flag bits = 0)
    sw.bit_writer_write_ub(0, 6)?;
    payload.extend_from_slice(&sw.bit_writer_into_bytes()?);
    encode_swf_tag(spec.tag_code, &payload)
}

/// Encodes a `PlaceObject2` (Tag 26) or `PlaceObject3` (Tag 70) tag.
pub fn encode_place_object_tag(spec: &PlaceTagSpec) -> Result<Vec<u8>, CorpusError> {
    let use_v3 = spec.blend_mode.is_some() || spec.visible.is_some();
    let mut flags1: u8 = 0;
    if spec.is_move {
        flags1 |= 0x01;
    }
    if spec.character_id.is_some() {
        flags1 |= 0x02;
    }
    if spec.matrix.is_some() {
        flags1 |= 0x04;
    }
    if spec.color_transform.is_some() {
        flags1 |= 0x08;
    }
    if spec.ratio.is_some() {
        flags1 |= 0x10;
    }
    if spec.name.is_some() {
        flags1 |= 0x20;
    }
    if spec.clip_depth.is_some() {
        flags1 |= 0x40;
    }

    let mut payload = Vec::new();
    payload.push(flags1);
    if use_v3 {
        let mut flags2: u8 = 0;
        if spec.blend_mode.is_some() {
            flags2 |= 0x02;
        }
        if spec.visible.is_some() {
            flags2 |= 0x20;
        }
        payload.push(flags2);
    }
    payload.extend_from_slice(&spec.depth.to_le_bytes());

    if let Some(cid) = spec.character_id {
        payload.extend_from_slice(&cid.to_le_bytes());
    }
    if let Some(ref m) = spec.matrix {
        let mut mw = SwfBitWriter::bit_writer_new();
        mw.bit_writer_write_matrix(m)?;
        payload.extend_from_slice(&mw.bit_writer_into_bytes()?);
    }
    if let Some(ref cx) = spec.color_transform {
        let mut cw = SwfBitWriter::bit_writer_new();
        cw.bit_writer_write_cxform_alpha(cx)?;
        payload.extend_from_slice(&cw.bit_writer_into_bytes()?);
    }
    if let Some(r) = spec.ratio {
        payload.extend_from_slice(&r.to_le_bytes());
    }
    if let Some(ref name) = spec.name {
        payload.extend_from_slice(name.as_bytes());
        payload.push(0);
    }
    if let Some(cd) = spec.clip_depth {
        payload.extend_from_slice(&cd.to_le_bytes());
    }
    if use_v3 {
        if let Some(bm) = spec.blend_mode {
            payload.push(bm);
        }
        if let Some(vis) = spec.visible {
            payload.push(u8::from(vis));
        }
    }

    let tag_code = if use_v3 { 70 } else { 26 };
    encode_swf_tag(tag_code, &payload)
}

/// Encodes a `RemoveObject2` (Tag 28) tag.
pub fn encode_remove_object2_tag(depth: u16) -> Result<Vec<u8>, CorpusError> {
    encode_swf_tag(28, &depth.to_le_bytes())
}

/// Encodes a `SetBackgroundColor` (Tag 9) tag.
pub fn encode_set_background_color_tag(r: u8, g: u8, b: u8) -> Result<Vec<u8>, CorpusError> {
    encode_swf_tag(9, &[r, g, b])
}

/// Encodes a `DefineSprite` (Tag 39) wrapping inner control tags.
pub fn encode_define_sprite_tag(
    sprite_id: u16,
    frame_count: u16,
    inner_tags: &[u8],
) -> Result<Vec<u8>, CorpusError> {
    if sprite_id == 0 {
        return Err(CorpusError::UnexpectedCaseFailure(
            "sprite_id must be >= 1".to_string(),
        ));
    }
    let mut payload = Vec::with_capacity(4 + inner_tags.len());
    payload.extend_from_slice(&sprite_id.to_le_bytes());
    payload.extend_from_slice(&frame_count.to_le_bytes());
    payload.extend_from_slice(inner_tags);
    encode_swf_tag(39, &payload)
}

/// Encodes a 16-bit little-endian mono `DefineSound` (Tag 14) at 44.1 kHz.
pub fn encode_define_sound_pcm16_tag(
    sound_id: u16,
    samples: &[i16],
) -> Result<Vec<u8>, CorpusError> {
    if sound_id == 0 {
        return Err(CorpusError::UnexpectedCaseFailure(
            "sound_id must be >= 1".to_string(),
        ));
    }
    let sample_count = u32::try_from(samples.len()).map_err(|_| CorpusError::IntegerOverflow)?;
    let mut payload = Vec::with_capacity(7 + samples.len() * 2);
    payload.extend_from_slice(&sound_id.to_le_bytes());
    // Format=3 (UncompressedLe) | Rate=3 (44100 Hz) | Size=1 (16-bit) | Mono=0 -> 0x3E
    payload.push(0x3E);
    payload.extend_from_slice(&sample_count.to_le_bytes());
    for &s in samples {
        payload.extend_from_slice(&s.to_le_bytes());
    }
    encode_swf_tag(14, &payload)
}

/// Encodes a `StartSound` (Tag 15) payload with default `SoundInfo` (play once).
pub fn encode_start_sound_tag(sound_id: u16) -> Result<Vec<u8>, CorpusError> {
    let mut payload = Vec::with_capacity(3);
    payload.extend_from_slice(&sound_id.to_le_bytes());
    payload.push(0x00);
    encode_swf_tag(15, &payload)
}

/// Encodes a `SoundStreamHead2` (Tag 45) for 4-bit Flash ADPCM mono at 44.1 kHz.
pub fn encode_sound_stream_head2_adpcm(avg_sample_count: u16) -> Result<Vec<u8>, CorpusError> {
    let mut payload = Vec::with_capacity(4);
    // Playback: rate=3 (44100 << 4 = 0x30) | size=1 (16-bit << 3 = 0x08) | mono=0 -> 0x38
    payload.push(0x38);
    // Stream: format=1 (ADPCM << 4 = 0x10) | rate=3 (44100 << 2 = 0x0C) | size=1 (16-bit << 1 = 0x02) | mono=0 -> 0x1E
    payload.push(0x1E);
    payload.extend_from_slice(&avg_sample_count.to_le_bytes());
    encode_swf_tag(45, &payload)
}

/// Encodes a 4-bit mono Flash ADPCM `SoundStreamBlock` (Tag 19) with deterministic delta codes.
pub fn encode_adpcm_stream_block_tag(
    initial_sample: i16,
    initial_step_index: u8,
    nibble_codes: &[u8],
) -> Result<Vec<u8>, CorpusError> {
    let mut bw = SwfBitWriter::bit_writer_new();
    // code_size = 2 => 4-bit ADPCM
    bw.bit_writer_write_ub(2, 2)?;
    bw.bit_writer_write_sb(initial_sample as i32, 16)?;
    bw.bit_writer_write_ub((initial_step_index.min(88)) as u32, 6)?;
    for &code in nibble_codes {
        bw.bit_writer_write_ub((code & 0x0F) as u32, 4)?;
    }
    let payload = bw.bit_writer_into_bytes()?;
    encode_swf_tag(19, &payload)
}

/// Encodes an AVM1 `DoAction` (Tag 12) payload.
pub fn encode_do_action_tag(actions: &[u8]) -> Result<Vec<u8>, CorpusError> {
    encode_swf_tag(12, actions)
}

/// Encodes an AVM2 `DoABC` (Tag 82) payload with SWF9 flags (`1`) and NUL-terminated name.
pub fn encode_do_abc_tag(name: &str, abc_bytes: &[u8]) -> Result<Vec<u8>, CorpusError> {
    let mut payload = Vec::with_capacity(4 + name.len() + 1 + abc_bytes.len());
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(name.as_bytes());
    payload.push(0);
    payload.extend_from_slice(abc_bytes);
    encode_swf_tag(82, &payload)
}

/// Helper builder for AVM1 action byte streams.
#[derive(Debug, Clone, Default)]
pub struct Avm1ActionBuilder {
    bytes: Vec<u8>,
}

impl Avm1ActionBuilder {
    /// Creates an empty AVM1 action builder.
    pub fn avm1_new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Appends a single-byte action opcode (`< 0x80`).
    pub fn avm1_op(&mut self, opcode: u8) {
        self.bytes.push(opcode);
    }

    /// Appends `ActionPush` (0x96) for a NUL-terminated UTF-8 string.
    pub fn avm1_push_str(&mut self, s: &str) -> Result<(), CorpusError> {
        let len = 1usize
            .checked_add(s.len())
            .and_then(|v| v.checked_add(1))
            .ok_or(CorpusError::IntegerOverflow)?;
        let len_u16 = u16::try_from(len).map_err(|_| CorpusError::IntegerOverflow)?;
        self.bytes.push(0x96);
        self.bytes.extend_from_slice(&len_u16.to_le_bytes());
        self.bytes.push(0x00); // string
        self.bytes.extend_from_slice(s.as_bytes());
        self.bytes.push(0);
        Ok(())
    }

    /// Appends `ActionPush` (0x96) for a 32-bit signed integer.
    pub fn avm1_push_i32(&mut self, val: i32) {
        self.bytes.push(0x96);
        self.bytes.extend_from_slice(&5u16.to_le_bytes());
        self.bytes.push(0x07); // integer
        self.bytes.extend_from_slice(&(val as u32).to_le_bytes());
    }

    /// Appends `ActionPush` (0x96) for a boolean.
    pub fn avm1_push_bool(&mut self, val: bool) {
        self.bytes.push(0x96);
        self.bytes.extend_from_slice(&2u16.to_le_bytes());
        self.bytes.push(0x05); // boolean
        self.bytes.push(u8::from(val));
    }

    /// Appends `ActionBranchAlways` (0x99) with signed 16-bit relative byte offset.
    pub fn avm1_branch_always(&mut self, offset: i16) {
        self.bytes.push(0x99);
        self.bytes.extend_from_slice(&2u16.to_le_bytes());
        self.bytes.extend_from_slice(&offset.to_le_bytes());
    }

    /// Appends `ActionBranchIfTrue` (0x9D) with signed 16-bit relative byte offset.
    pub fn avm1_branch_if_true(&mut self, offset: i16) {
        self.bytes.push(0x9D);
        self.bytes.extend_from_slice(&2u16.to_le_bytes());
        self.bytes.extend_from_slice(&offset.to_le_bytes());
    }

    /// Appends `ActionGotoFrame` (0x81) with 0-based frame index.
    pub fn avm1_goto_frame(&mut self, frame_zero_based: u16) {
        self.bytes.push(0x81);
        self.bytes.extend_from_slice(&2u16.to_le_bytes());
        self.bytes
            .extend_from_slice(&frame_zero_based.to_le_bytes());
    }

    /// Appends `ActionDefineFunction` (0x9B) with body bytes.
    pub fn avm1_define_function(
        &mut self,
        name: &str,
        params: &[&str],
        body: &[u8],
    ) -> Result<(), CorpusError> {
        let mut header = Vec::new();
        header.extend_from_slice(name.as_bytes());
        header.push(0);
        let pcount = u16::try_from(params.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        header.extend_from_slice(&pcount.to_le_bytes());
        for &p in params {
            header.extend_from_slice(p.as_bytes());
            header.push(0);
        }
        let body_len = u16::try_from(body.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        header.extend_from_slice(&body_len.to_le_bytes());

        let header_len = u16::try_from(header.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        self.bytes.push(0x9B);
        self.bytes.extend_from_slice(&header_len.to_le_bytes());
        self.bytes.extend_from_slice(&header);
        self.bytes.extend_from_slice(body);
        Ok(())
    }

    /// Appends `ActionDefineFunction2` (0x8E) with register-mapped parameters and body bytes.
    pub fn avm1_define_function2(
        &mut self,
        name: &str,
        register_count: u8,
        flags: u16,
        params: &[(u8, &str)],
        body: &[u8],
    ) -> Result<(), CorpusError> {
        let mut header = Vec::new();
        header.extend_from_slice(name.as_bytes());
        header.push(0);
        let pcount = u16::try_from(params.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        header.extend_from_slice(&pcount.to_le_bytes());
        header.push(register_count);
        header.extend_from_slice(&flags.to_le_bytes());
        for &(reg, pname) in params {
            header.push(reg);
            header.extend_from_slice(pname.as_bytes());
            header.push(0);
        }
        let body_len = u16::try_from(body.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        header.extend_from_slice(&body_len.to_le_bytes());

        let header_len = u16::try_from(header.len()).map_err(|_| CorpusError::IntegerOverflow)?;
        self.bytes.push(0x8E);
        self.bytes.extend_from_slice(&header_len.to_le_bytes());
        self.bytes.extend_from_slice(&header);
        self.bytes.extend_from_slice(body);
        Ok(())
    }

    /// Finalizes with `ActionEnd` (0x00) and returns the encoded AVM1 byte stream.
    pub fn avm1_finish(mut self) -> Vec<u8> {
        self.bytes.push(0x00);
        self.bytes
    }
}

/// Encodes a `u30` unsigned variable-length integer for ABC bytecode.
pub fn write_abc_u30(out: &mut Vec<u8>, mut val: u32) -> Result<(), CorpusError> {
    if val > 0x3FFF_FFFF {
        return Err(CorpusError::IntegerOverflow);
    }
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if val == 0 {
            break;
        }
    }
    Ok(())
}

/// Clean-room SWF file builder (`FWS` / `CWS`, `RECT`, 8.8 FPS, bounded tags & frames).
#[derive(Debug, Clone)]
pub struct SwfBuilder {
    version: u8,
    compression: SwfCompression,
    stage_rect: SwfRect,
    frame_rate_8_8: u16,
    frame_count: u16,
    tag_bytes: Vec<u8>,
    tag_count: usize,
}

impl SwfBuilder {
    /// Creates a new `SwfBuilder` with stage dimensions in twips and whole-FPS frame rate.
    pub fn swf_builder_new(
        version: u8,
        compression: SwfCompression,
        stage_width_px: i32,
        stage_height_px: i32,
        fps: u8,
    ) -> Result<Self, CorpusError> {
        let w_twips = stage_width_px
            .checked_mul(20)
            .ok_or(CorpusError::IntegerOverflow)?;
        let h_twips = stage_height_px
            .checked_mul(20)
            .ok_or(CorpusError::IntegerOverflow)?;
        if w_twips <= 0 || h_twips <= 0 || fps == 0 {
            return Err(CorpusError::UnexpectedCaseFailure(
                "invalid stage dimension or zero fps".to_string(),
            ));
        }
        let frame_rate_8_8 = u16::from(fps) << 8;
        Ok(Self {
            version,
            compression,
            stage_rect: SwfRect::create_rect(0, w_twips, 0, h_twips),
            frame_rate_8_8,
            frame_count: 0,
            tag_bytes: Vec::new(),
            tag_count: 0,
        })
    }

    /// Sets the raw 8.8 fixed-point frame rate (`high byte = integer fps`, `low byte = frac/256`).
    pub fn swf_builder_set_frame_rate_8_8(&mut self, raw_8_8: u16) -> Result<(), CorpusError> {
        if raw_8_8 == 0 {
            return Err(CorpusError::UnexpectedCaseFailure(
                "frame_rate_8_8 must be non-zero".to_string(),
            ));
        }
        self.frame_rate_8_8 = raw_8_8;
        Ok(())
    }

    /// Appends a pre-encoded SWF tag byte slice, enforcing `MAX_BUILDER_TAGS` and `MAX_CORPUS_SWF_BYTES`.
    pub fn swf_builder_push_raw_tag(&mut self, encoded_tag: &[u8]) -> Result<(), CorpusError> {
        if self.tag_count >= MAX_BUILDER_TAGS {
            return Err(CorpusError::SwfSizeExceeded);
        }
        let next_len = self
            .tag_bytes
            .len()
            .checked_add(encoded_tag.len())
            .ok_or(CorpusError::IntegerOverflow)?;
        validate_corpus_swf_bytes(next_len.max(1))?;
        self.tag_bytes.extend_from_slice(encoded_tag);
        self.tag_count += 1;
        Ok(())
    }

    /// Appends a `SetBackgroundColor` (Tag 9) tag.
    pub fn swf_builder_set_background(&mut self, r: u8, g: u8, b: u8) -> Result<(), CorpusError> {
        let tag = encode_set_background_color_tag(r, g, b)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `DefineShape1..4` tag.
    pub fn swf_builder_define_shape(&mut self, spec: &ShapeDefSpec) -> Result<(), CorpusError> {
        let tag = encode_define_shape_tag(spec)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `DefineSprite` (Tag 39) tag.
    pub fn swf_builder_define_sprite(
        &mut self,
        sprite_id: u16,
        frame_count: u16,
        inner_tags: &[u8],
    ) -> Result<(), CorpusError> {
        let tag = encode_define_sprite_tag(sprite_id, frame_count, inner_tags)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `PlaceObject2` or `PlaceObject3` tag.
    pub fn swf_builder_place_object(&mut self, spec: &PlaceTagSpec) -> Result<(), CorpusError> {
        let tag = encode_place_object_tag(spec)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `RemoveObject2` (Tag 28) tag.
    pub fn swf_builder_remove_object(&mut self, depth: u16) -> Result<(), CorpusError> {
        let tag = encode_remove_object2_tag(depth)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `DoAction` (Tag 12) tag.
    pub fn swf_builder_do_action(&mut self, actions: &[u8]) -> Result<(), CorpusError> {
        let tag = encode_do_action_tag(actions)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `DoABC` (Tag 82) tag.
    pub fn swf_builder_do_abc(&mut self, name: &str, abc_bytes: &[u8]) -> Result<(), CorpusError> {
        let tag = encode_do_abc_tag(name, abc_bytes)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `DefineSound` (Tag 14) 16-bit mono PCM tag.
    pub fn swf_builder_define_sound_pcm16(
        &mut self,
        sound_id: u16,
        samples: &[i16],
    ) -> Result<(), CorpusError> {
        let tag = encode_define_sound_pcm16_tag(sound_id, samples)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `StartSound` (Tag 15) tag.
    pub fn swf_builder_start_sound(&mut self, sound_id: u16) -> Result<(), CorpusError> {
        let tag = encode_start_sound_tag(sound_id)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `SoundStreamHead2` (Tag 45) for 4-bit ADPCM mono at 44.1 kHz.
    pub fn swf_builder_sound_stream_head2_adpcm(
        &mut self,
        avg_sample_count: u16,
    ) -> Result<(), CorpusError> {
        let tag = encode_sound_stream_head2_adpcm(avg_sample_count)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Appends a `SoundStreamBlock` (Tag 19) carrying 4-bit ADPCM audio.
    pub fn swf_builder_adpcm_stream_block(
        &mut self,
        initial_sample: i16,
        initial_step_index: u8,
        nibble_codes: &[u8],
    ) -> Result<(), CorpusError> {
        let tag = encode_adpcm_stream_block_tag(initial_sample, initial_step_index, nibble_codes)?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Emits a `ShowFrame` (Tag 1) and increments `frame_count` up to `MAX_CORPUS_FRAMES`.
    pub fn swf_builder_show_frame(&mut self) -> Result<(), CorpusError> {
        let next_frames = usize::from(self.frame_count)
            .checked_add(1)
            .ok_or(CorpusError::IntegerOverflow)?;
        validate_corpus_frame_count(next_frames)?;
        self.frame_count = u16::try_from(next_frames).map_err(|_| CorpusError::IntegerOverflow)?;
        let tag = encode_swf_tag(1, &[])?;
        self.swf_builder_push_raw_tag(&tag)
    }

    /// Finalizes the SWF with an `End` (Tag 0) tag and builds `FWS` or `CWS` container bytes.
    pub fn swf_builder_finish(mut self) -> Result<Vec<u8>, CorpusError> {
        let end_tag = encode_swf_tag(0, &[])?;
        self.swf_builder_push_raw_tag(&end_tag)?;
        let declared_frames = self.frame_count.max(1);

        let mut rect_bw = SwfBitWriter::bit_writer_new();
        rect_bw.bit_writer_write_rect(
            self.stage_rect.xmin,
            self.stage_rect.xmax,
            self.stage_rect.ymin,
            self.stage_rect.ymax,
        )?;
        let rect_bytes = rect_bw.bit_writer_into_bytes()?;

        let body_len = rect_bytes
            .len()
            .checked_add(4)
            .and_then(|v| v.checked_add(self.tag_bytes.len()))
            .ok_or(CorpusError::IntegerOverflow)?;
        let uncompressed_len = 8usize
            .checked_add(body_len)
            .ok_or(CorpusError::IntegerOverflow)?;
        validate_corpus_swf_bytes(uncompressed_len)?;
        let file_len_u32 =
            u32::try_from(uncompressed_len).map_err(|_| CorpusError::IntegerOverflow)?;

        let mut body = Vec::with_capacity(body_len);
        body.extend_from_slice(&rect_bytes);
        body.extend_from_slice(&self.frame_rate_8_8.to_le_bytes());
        body.extend_from_slice(&declared_frames.to_le_bytes());
        body.extend_from_slice(&self.tag_bytes);

        match self.compression {
            SwfCompression::Fws => {
                let mut out = Vec::with_capacity(uncompressed_len);
                out.extend_from_slice(b"FWS");
                out.push(self.version);
                out.extend_from_slice(&file_len_u32.to_le_bytes());
                out.extend_from_slice(&body);
                Ok(out)
            }
            SwfCompression::Cws => {
                let raw_deflate = deflate::compress(&body).map_err(CorpusError::Compression)?;
                let adler = compute_adler32(&body);
                let total_cws_len = 8usize
                    .checked_add(2)
                    .and_then(|v| v.checked_add(raw_deflate.len()))
                    .and_then(|v| v.checked_add(4))
                    .ok_or(CorpusError::IntegerOverflow)?;
                validate_corpus_swf_bytes(total_cws_len)?;

                let mut out = Vec::with_capacity(total_cws_len);
                out.extend_from_slice(b"CWS");
                out.push(self.version);
                out.extend_from_slice(&file_len_u32.to_le_bytes());
                // RFC 1950 zlib header: CMF=0x78 (DEFLATE, 32K window), FLG=0x9C (FCHECK mod 31 == 0)
                out.extend_from_slice(&[0x78, 0x9C]);
                out.extend_from_slice(&raw_deflate);
                out.extend_from_slice(&adler.to_be_bytes());
                Ok(out)
            }
        }
    }
}
