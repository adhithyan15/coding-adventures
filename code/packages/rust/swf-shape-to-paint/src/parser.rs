//! Binary parser for SWF `DefineShape`, `DefineShape2`, `DefineShape3`, and `DefineShape4`.

#![deny(unsafe_code)]

use swf_container::{BitReader, SwfRect};

use crate::error::ShapeError;
use crate::records::ShapeEdge;
use crate::styles::{
    ColorRgba, SwfBitmapFill, SwfCapStyle, SwfFillStyle, SwfGradient, SwfGradientKind,
    SwfGradientRecord, SwfInterpolationMode, SwfJoinStyle, SwfLineStyle, SwfSpreadMode,
};
use crate::twips::{checked_twips_point_add, validate_twips};

/// Maximum allowed number of fill or line styles (CWE-400 guard).
pub const MAX_STYLES: usize = 65_535;
/// Maximum allowed shape records in a single shape tag (CWE-400 guard).
pub const MAX_SHAPE_RECORDS: usize = 100_000;
/// Maximum allowed edges per shape (CWE-400 guard).
pub const MAX_EDGES: usize = 100_000;

/// Validates that a tag code corresponds to an SWF shape definition tag.
#[inline]
pub fn validate_shape_tag(tag_code: usize) -> Result<bool, ShapeError> {
    match tag_code {
        2 | 22 | 32 | 83 => Ok(true),
        _ => Err(ShapeError::InvalidTag),
    }
}

/// Validates that a style count does not exceed security limits (CWE-400).
#[inline]
pub fn validate_style_count(count: usize) -> Result<bool, ShapeError> {
    if count <= MAX_STYLES {
        Ok(true)
    } else {
        Err(ShapeError::ExcessiveStyles)
    }
}

/// Fully parsed vector shape definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedShape {
    /// Tag code (`2`, `22`, `32`, or `83`).
    pub tag_code: u16,
    /// Character ID of this shape.
    pub shape_id: u16,
    /// Bounding rectangle in twips.
    pub bounds: SwfRect,
    /// Edge bounds rectangle in twips (DefineShape4 only).
    pub edge_bounds: Option<SwfRect>,
    /// Whether non-zero winding rule should be applied (`UsesFillWindingNos`).
    pub uses_non_zero_winding: bool,
    /// Array of fill styles defined by this shape.
    pub fill_styles: Vec<SwfFillStyle>,
    /// Array of line styles defined by this shape.
    pub line_styles: Vec<SwfLineStyle>,
    /// Extracted edge segments tagged with left/right fill and stroke styles.
    pub edges: Vec<ShapeEdge>,
}

impl ParsedShape {
    /// Total number of edges in this shape.
    #[inline]
    pub fn parsed_edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Total number of fill styles.
    #[inline]
    pub fn parsed_fill_style_count(&self) -> usize {
        self.fill_styles.len()
    }

    /// Total number of line styles.
    #[inline]
    pub fn parsed_line_style_count(&self) -> usize {
        self.line_styles.len()
    }
}

#[inline]
fn read_ui8(reader: &mut BitReader<'_>) -> Result<u8, ShapeError> {
    Ok(reader.read_ub(8)? as u8)
}

#[inline]
fn read_ui16_le(reader: &mut BitReader<'_>) -> Result<u16, ShapeError> {
    let b0 = reader.read_ub(8)? as u16;
    let b1 = reader.read_ub(8)? as u16;
    Ok(b0 | (b1 << 8))
}

#[inline]
fn read_rgb(reader: &mut BitReader<'_>) -> Result<ColorRgba, ShapeError> {
    let r = read_ui8(reader)?;
    let g = read_ui8(reader)?;
    let b = read_ui8(reader)?;
    Ok(ColorRgba::rgb(r, g, b))
}

#[inline]
fn read_rgba(reader: &mut BitReader<'_>) -> Result<ColorRgba, ShapeError> {
    let r = read_ui8(reader)?;
    let g = read_ui8(reader)?;
    let b = read_ui8(reader)?;
    let a = read_ui8(reader)?;
    Ok(ColorRgba::rgba(r, g, b, a))
}

fn read_style_count(reader: &mut BitReader<'_>, tag_code: u16) -> Result<usize, ShapeError> {
    let initial = read_ui8(reader)? as usize;
    let count = if initial == 0xFF && tag_code != 2 {
        read_ui16_le(reader)? as usize
    } else {
        initial
    };
    validate_style_count(count)?;
    Ok(count)
}

fn parse_gradient(
    reader: &mut BitReader<'_>,
    tag_code: u16,
    kind: SwfGradientKind,
) -> Result<SwfGradient, ShapeError> {
    let matrix = reader.read_matrix()?;
    let spread_code = reader.read_ub(2)?;
    let interp_code = reader.read_ub(2)?;
    let num_gradients = reader.read_ub(4)? as usize;

    if num_gradients == 0 || num_gradients > 15 {
        return Err(ShapeError::CorruptedRecord);
    }

    let spread = match spread_code {
        1 => SwfSpreadMode::Reflect,
        2 => SwfSpreadMode::Repeat,
        _ => SwfSpreadMode::Pad,
    };

    let interpolation = match interp_code {
        1 => SwfInterpolationMode::LinearRgb,
        _ => SwfInterpolationMode::NormalRgb,
    };

    let mut stops = Vec::with_capacity(num_gradients);
    for _ in 0..num_gradients {
        let ratio = read_ui8(reader)?;
        let color = if tag_code >= 32 {
            read_rgba(reader)?
        } else {
            read_rgb(reader)?
        };
        stops.push(SwfGradientRecord { ratio, color });
    }

    let focal_point = if kind == SwfGradientKind::FocalRadial {
        let raw = reader.read_sb(16)?;
        (raw as f64 / 256.0).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    Ok(SwfGradient {
        kind,
        spread,
        interpolation,
        stops,
        matrix,
        focal_point,
    })
}

fn parse_fill_style(reader: &mut BitReader<'_>, tag_code: u16) -> Result<SwfFillStyle, ShapeError> {
    let fill_type = read_ui8(reader)?;
    match fill_type {
        0x00 => {
            let color = if tag_code >= 32 {
                read_rgba(reader)?
            } else {
                read_rgb(reader)?
            };
            Ok(SwfFillStyle::Solid(color))
        }
        0x10 => {
            let grad = parse_gradient(reader, tag_code, SwfGradientKind::Linear)?;
            Ok(SwfFillStyle::LinearGradient(grad))
        }
        0x12 => {
            let grad = parse_gradient(reader, tag_code, SwfGradientKind::Radial)?;
            Ok(SwfFillStyle::RadialGradient(grad))
        }
        0x13 => {
            let grad = parse_gradient(reader, tag_code, SwfGradientKind::FocalRadial)?;
            Ok(SwfFillStyle::FocalGradient(grad))
        }
        0x40 => {
            let bitmap_id = read_ui16_le(reader)?;
            let matrix = reader.read_matrix()?;
            Ok(SwfFillStyle::RepeatingBitmap(SwfBitmapFill {
                bitmap_id,
                matrix,
                is_repeating: true,
                is_smoothed: true,
            }))
        }
        0x41 => {
            let bitmap_id = read_ui16_le(reader)?;
            let matrix = reader.read_matrix()?;
            Ok(SwfFillStyle::ClippedBitmap(SwfBitmapFill {
                bitmap_id,
                matrix,
                is_repeating: false,
                is_smoothed: true,
            }))
        }
        0x42 => {
            let bitmap_id = read_ui16_le(reader)?;
            let matrix = reader.read_matrix()?;
            Ok(SwfFillStyle::NonSmoothedRepeatingBitmap(SwfBitmapFill {
                bitmap_id,
                matrix,
                is_repeating: true,
                is_smoothed: false,
            }))
        }
        0x43 => {
            let bitmap_id = read_ui16_le(reader)?;
            let matrix = reader.read_matrix()?;
            Ok(SwfFillStyle::NonSmoothedClippedBitmap(SwfBitmapFill {
                bitmap_id,
                matrix,
                is_repeating: false,
                is_smoothed: false,
            }))
        }
        _ => Err(ShapeError::CorruptedRecord),
    }
}

fn parse_line_style(reader: &mut BitReader<'_>, tag_code: u16) -> Result<SwfLineStyle, ShapeError> {
    if tag_code == 83 {
        // LINESTYLE2 in DefineShape4
        let width_twips = read_ui16_le(reader)?;
        let start_cap_code = reader.read_ub(2)?;
        let cap_style = match start_cap_code {
            1 => SwfCapStyle::Butt,
            2 => SwfCapStyle::Square,
            _ => SwfCapStyle::Round,
        };

        let join_style_code = reader.read_ub(2)?;
        let join_style = match join_style_code {
            1 => SwfJoinStyle::Bevel,
            2 => SwfJoinStyle::Miter,
            _ => SwfJoinStyle::Round,
        };

        let has_fill = reader.read_ub(1)? != 0;
        let _no_h_scale = reader.read_ub(1)? != 0;
        let _no_v_scale = reader.read_ub(1)? != 0;
        let _pixel_hinting = reader.read_ub(1)? != 0;
        let _reserved = reader.read_ub(5)?;
        let no_close = reader.read_ub(1)? != 0;
        let _end_cap_code = reader.read_ub(2)?;

        let miter_limit = if join_style_code == 2 {
            let raw_miter = read_ui16_le(reader)?;
            raw_miter as f64 / 256.0
        } else {
            3.0
        };

        let (color, fill_style) = if has_fill {
            let fill = parse_fill_style(reader, tag_code)?;
            (ColorRgba::rgba(0, 0, 0, 255), Some(fill))
        } else {
            let c = read_rgba(reader)?;
            (c, None)
        };

        Ok(SwfLineStyle {
            width_twips,
            color,
            cap_style,
            join_style,
            miter_limit,
            no_close,
            fill_style,
        })
    } else {
        // LINESTYLE in DefineShape 1, 2, 3
        let width_twips = read_ui16_le(reader)?;
        let color = if tag_code >= 32 {
            read_rgba(reader)?
        } else {
            read_rgb(reader)?
        };
        Ok(SwfLineStyle {
            width_twips,
            color,
            cap_style: SwfCapStyle::Round,
            join_style: SwfJoinStyle::Round,
            miter_limit: 3.0,
            no_close: false,
            fill_style: None,
        })
    }
}

/// Parses a binary SWF `DefineShape`, `DefineShape2`, `DefineShape3`, or `DefineShape4` tag payload.
pub fn parse_define_shape(tag_code: u16, data: &[u8]) -> Result<ParsedShape, ShapeError> {
    validate_shape_tag(tag_code as usize)?;

    let mut reader = BitReader::from_bits(data);

    let shape_id = read_ui16_le(&mut reader)?;
    let bounds = reader.read_rect()?;

    let (edge_bounds, uses_non_zero_winding) = if tag_code == 83 {
        let eb = reader.read_rect()?;
        let _reserved = reader.read_ub(5)?;
        let uses_winding = reader.read_ub(1)? != 0;
        let _non_scaling = reader.read_ub(1)? != 0;
        let _scaling = reader.read_ub(1)? != 0;
        (Some(eb), uses_winding)
    } else {
        (None, false)
    };

    // SHAPEWITHSTYLE
    let mut fill_styles = Vec::new();
    let initial_fill_count = read_style_count(&mut reader, tag_code)?;
    for _ in 0..initial_fill_count {
        fill_styles.push(parse_fill_style(&mut reader, tag_code)?);
    }

    let mut line_styles = Vec::new();
    let initial_line_count = read_style_count(&mut reader, tag_code)?;
    for _ in 0..initial_line_count {
        line_styles.push(parse_line_style(&mut reader, tag_code)?);
    }

    let mut num_fill_bits = reader.read_ub(4)? as usize;
    let mut num_line_bits = reader.read_ub(4)? as usize;

    let mut edges = Vec::new();
    let mut current_pen = (0i32, 0i32);
    let mut current_fill0 = 0u32;
    let mut current_fill1 = 0u32;
    let mut current_line = 0u32;

    let mut fill_offset = 0usize;
    let mut line_offset = 0usize;

    let mut record_count = 0usize;

    loop {
        record_count += 1;
        if record_count > MAX_SHAPE_RECORDS {
            return Err(ShapeError::ExcessiveEdges);
        }

        let type_flag = reader.read_ub(1)?;
        if type_flag == 0 {
            // Style change or end shape
            let flags = reader.read_ub(5)?;
            if flags == 0 {
                // EndShapeRecord
                break;
            }

            let state_new_styles = (flags & 0x10) != 0;
            let state_line_style = (flags & 0x08) != 0;
            let state_fill_style1 = (flags & 0x04) != 0;
            let state_fill_style0 = (flags & 0x02) != 0;
            let state_move_to = (flags & 0x01) != 0;

            if state_move_to {
                let move_bits = reader.read_ub(5)? as usize;
                let move_x = reader.read_sb(move_bits)?;
                let move_y = reader.read_sb(move_bits)?;
                validate_twips(move_x)?;
                validate_twips(move_y)?;
                current_pen = (move_x, move_y);
            }

            if state_fill_style0 {
                let idx = reader.read_ub(num_fill_bits)?;
                current_fill0 = if idx > 0 {
                    (fill_offset as u32) + idx
                } else {
                    0
                };
            }

            if state_fill_style1 {
                let idx = reader.read_ub(num_fill_bits)?;
                current_fill1 = if idx > 0 {
                    (fill_offset as u32) + idx
                } else {
                    0
                };
            }

            if state_line_style {
                let idx = reader.read_ub(num_line_bits)?;
                current_line = if idx > 0 {
                    (line_offset as u32) + idx
                } else {
                    0
                };
            }

            if state_new_styles {
                reader.align_to_byte();

                fill_offset = fill_styles.len();
                let new_fill_count = read_style_count(&mut reader, tag_code)?;
                for _ in 0..new_fill_count {
                    fill_styles.push(parse_fill_style(&mut reader, tag_code)?);
                }

                line_offset = line_styles.len();
                let new_line_count = read_style_count(&mut reader, tag_code)?;
                for _ in 0..new_line_count {
                    line_styles.push(parse_line_style(&mut reader, tag_code)?);
                }

                num_fill_bits = reader.read_ub(4)? as usize;
                num_line_bits = reader.read_ub(4)? as usize;
            }
        } else {
            // Edge record
            if edges.len() >= MAX_EDGES {
                return Err(ShapeError::ExcessiveEdges);
            }

            let straight_flag = reader.read_ub(1)?;
            let nbits = (reader.read_ub(4)? as usize) + 2;

            if straight_flag == 1 {
                let general_line = reader.read_ub(1)?;
                let (dx, dy) = if general_line == 1 {
                    let x = reader.read_sb(nbits)?;
                    let y = reader.read_sb(nbits)?;
                    (x, y)
                } else {
                    let vert = reader.read_ub(1)?;
                    if vert == 0 {
                        let x = reader.read_sb(nbits)?;
                        (x, 0)
                    } else {
                        let y = reader.read_sb(nbits)?;
                        (0, y)
                    }
                };

                let new_pen = checked_twips_point_add(current_pen.0, current_pen.1, dx, dy)?;
                edges.push(ShapeEdge {
                    start: current_pen,
                    end: new_pen,
                    control: None,
                    fill_style0: current_fill0,
                    fill_style1: current_fill1,
                    line_style: current_line,
                });
                current_pen = new_pen;
            } else {
                let control_dx = reader.read_sb(nbits)?;
                let control_dy = reader.read_sb(nbits)?;
                let anchor_dx = reader.read_sb(nbits)?;
                let anchor_dy = reader.read_sb(nbits)?;

                let ctrl =
                    checked_twips_point_add(current_pen.0, current_pen.1, control_dx, control_dy)?;
                let end = checked_twips_point_add(ctrl.0, ctrl.1, anchor_dx, anchor_dy)?;

                edges.push(ShapeEdge {
                    start: current_pen,
                    end,
                    control: Some(ctrl),
                    fill_style0: current_fill0,
                    fill_style1: current_fill1,
                    line_style: current_line,
                });
                current_pen = end;
            }
        }
    }

    Ok(ParsedShape {
        tag_code,
        shape_id,
        bounds,
        edge_bounds,
        uses_non_zero_winding,
        fill_styles,
        line_styles,
        edges,
    })
}
