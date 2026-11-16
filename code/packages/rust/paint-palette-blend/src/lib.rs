//! # paint-palette-blend
//!
//! P2D10: deterministic, zero-allocation indexed-color scanline blits, hardware
//! blend/compositing modes, and retained-mode scene diffing for `PaintVM`.
//!
//! * **Indexed blits** — 8-bit palette indices are expanded through a palette of at
//!   most [`MAX_PALETTE_ENTRIES`] RGBA colors into an RGBA8 destination, with optional
//!   color-key transparency, horizontal/vertical flipping, clipping, and stride-aware
//!   framebuffer addressing.
//! * **Blend modes** — [`BlendMode::Normal`], [`BlendMode::Add`],
//!   [`BlendMode::Multiply`], [`BlendMode::Screen`] and [`BlendMode::Alpha`], computed in
//!   exact integer arithmetic so results are bit-identical on every platform.
//! * **Scene diffing** — [`diff_paint_scene_into`] compares two
//!   [`paint_instructions::PaintScene`] graphs by stable `PaintBase.id` and writes
//!   [`PatchOp`] records into a caller-supplied slice with zero heap allocation.
//!
//! Every entry point validates dimensions, strides, buffer lengths, palette indices and
//! alpha/blend inputs *before* mutating any destination byte, so a rejected call leaves
//! the destination untouched (CWE-20/125/190/369/400/787).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use paint_instructions::{PaintBase, PaintInstruction, PaintScene};
pub use pixel_container::PixelContainer;

/// An RGBA color with 8 bits per channel, stored as `[r, g, b, a]`.
pub type Rgba8 = [u8; 4];

/// Maximum number of colors in a palette (8-bit indices).
pub const MAX_PALETTE_ENTRIES: usize = 256;

/// Maximum image, scanline, or framebuffer width/height in pixels (CWE-400 limit).
pub const MAX_DIMENSION: u32 = 8192;

/// Maximum number of instructions per scene accepted by the diff (CWE-400 limit).
pub const MAX_SCENE_INSTRUCTIONS: usize = 4096;

/// Sentinel used in [`PatchOp`] when an index does not apply.
pub const NO_INDEX: u32 = u32::MAX;

/// Blit parameters for an opaque, unflipped, non-keyed copy.
pub const OPAQUE_BLIT: BlitParams = BlitParams {
    mode: BlendMode::Normal,
    alpha: 255,
    transparent_index: None,
    h_flip: false,
    v_flip: false,
};

/// Hardware-style compositing mode applied per pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlendMode {
    /// Replace the destination with the source color, ignoring the alpha parameter.
    #[default]
    Normal,
    /// Saturating per-channel addition, weighted by effective alpha.
    Add,
    /// Per-channel `src * dst / 255`, weighted by effective alpha.
    Multiply,
    /// Per-channel `255 - (255 - src) * (255 - dst) / 255`, weighted by effective alpha.
    Screen,
    /// Source-over alpha blending, exact for opaque destinations.
    Alpha,
}

/// Parameters controlling an indexed blit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlitParams {
    /// Compositing mode.
    pub mode: BlendMode,
    /// Global alpha multiplier (0 = invisible, 255 = full strength).
    pub alpha: u8,
    /// Palette index treated as fully transparent (color-key), if any.
    pub transparent_index: Option<u8>,
    /// Mirror the source horizontally.
    pub h_flip: bool,
    /// Mirror the source vertically (image blits only).
    pub v_flip: bool,
}

/// A read-only view of an 8-bit indexed image with an explicit row stride.
#[derive(Clone, Copy, Debug)]
pub struct IndexedImage<'a> {
    /// Width in pixels (1..=[`MAX_DIMENSION`]).
    pub width: u32,
    /// Height in pixels (1..=[`MAX_DIMENSION`]).
    pub height: u32,
    /// Distance in bytes between the starts of consecutive rows (>= `width`).
    pub stride: usize,
    /// Palette indices, one byte per pixel, row-major.
    pub pixels: &'a [u8],
}

/// A mutable view of an RGBA8 framebuffer with an explicit row stride.
#[derive(Debug)]
pub struct RgbaTarget<'a> {
    /// Width in pixels (1..=[`MAX_DIMENSION`]).
    pub width: u32,
    /// Height in pixels (1..=[`MAX_DIMENSION`]).
    pub height: u32,
    /// Distance in bytes between the starts of consecutive rows (>= `width * 4`).
    pub stride: usize,
    /// RGBA8 bytes, row-major.
    pub pixels: &'a mut [u8],
}

/// Classification of a retained-mode scene change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKind {
    /// Instruction exists only in the next scene.
    Added,
    /// Instruction exists in both scenes with different content.
    Modified,
    /// Instruction exists only in the previous scene.
    Removed,
}

/// A zero-allocation patch record referring to instructions by index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatchOp {
    /// Kind of change.
    pub kind: PatchKind,
    /// Index in the previous scene, or [`NO_INDEX`] when added.
    pub old_index: u32,
    /// Index in the next scene, or [`NO_INDEX`] when removed.
    pub new_index: u32,
}

/// Summary of a scene diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatchSummary {
    /// Number of [`PatchOp`] records written to the output slice.
    pub ops_written: usize,
    /// Count of added instructions.
    pub added: usize,
    /// Count of modified instructions.
    pub modified: usize,
    /// Count of removed instructions.
    pub removed: usize,
    /// Count of instructions identical in both scenes.
    pub unchanged: usize,
    /// True when scene id, size, or background differ.
    pub header_changed: bool,
}

/// Errors reported by palette, blend, blit and diff operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteBlendError {
    /// Width or height is zero or exceeds [`MAX_DIMENSION`].
    InvalidDimensions {
        /// Provided width.
        width: u32,
        /// Provided height.
        height: u32,
    },
    /// Row stride is smaller than one row of pixels.
    InvalidStride {
        /// Provided stride in bytes.
        stride: usize,
        /// Minimum stride in bytes.
        min: usize,
    },
    /// A buffer is shorter than the geometry requires.
    BufferTooSmall {
        /// Required length.
        required: usize,
        /// Provided length.
        provided: usize,
    },
    /// Buffer length arithmetic overflowed (CWE-190).
    ArithmeticOverflow,
    /// Palette is empty.
    EmptyPalette,
    /// Palette has more than [`MAX_PALETTE_ENTRIES`] entries.
    PaletteTooLarge {
        /// Provided palette length.
        len: usize,
        /// Maximum palette length.
        max: usize,
    },
    /// A palette index is outside the palette.
    PaletteIndexOutOfRange {
        /// Offending index.
        index: u8,
        /// Palette length.
        palette_len: usize,
    },
    /// Two parallel buffers have different pixel counts.
    LengthMismatch {
        /// Destination length in bytes.
        dst: usize,
        /// Source length in bytes.
        src: usize,
    },
    /// A byte span is not a whole number of RGBA pixels.
    MisalignedSpan {
        /// Provided length in bytes.
        len: usize,
    },
    /// Unknown blend mode code.
    InvalidBlendMode(u8),
    /// Alpha is NaN, infinite, or outside `0.0..=1.0`.
    InvalidAlpha,
    /// A scene holds more than [`MAX_SCENE_INSTRUCTIONS`] instructions.
    SceneTooLarge {
        /// Instruction count.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// An instruction lacks a stable `PaintBase.id`.
    MissingInstructionId {
        /// Index of the instruction.
        index: usize,
    },
    /// Two instructions in one scene share an id.
    DuplicateInstructionId {
        /// Index of the second occurrence.
        index: usize,
    },
    /// The patch output slice is too small.
    PatchBufferTooSmall {
        /// Required capacity.
        required: usize,
        /// Provided capacity.
        provided: usize,
    },
}

impl std::fmt::Display for PaletteBlendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid dimensions: {width}x{height}")
            }
            Self::InvalidStride { stride, min } => {
                write!(f, "invalid stride {stride}, minimum {min}")
            }
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "buffer too small: required {required}, provided {provided}"
                )
            }
            Self::ArithmeticOverflow => write!(f, "buffer length arithmetic overflow"),
            Self::EmptyPalette => write!(f, "palette is empty"),
            Self::PaletteTooLarge { len, max } => {
                write!(f, "palette has {len} entries, maximum {max}")
            }
            Self::PaletteIndexOutOfRange { index, palette_len } => {
                write!(
                    f,
                    "palette index {index} out of range for {palette_len} entries"
                )
            }
            Self::LengthMismatch { dst, src } => {
                write!(f, "length mismatch: dst {dst} bytes, src {src} bytes")
            }
            Self::MisalignedSpan { len } => {
                write!(f, "span of {len} bytes is not a multiple of 4")
            }
            Self::InvalidBlendMode(code) => write!(f, "invalid blend mode code {code}"),
            Self::InvalidAlpha => write!(f, "alpha must be finite and within 0.0..=1.0"),
            Self::SceneTooLarge { count, max } => {
                write!(f, "scene has {count} instructions, maximum {max}")
            }
            Self::MissingInstructionId { index } => {
                write!(f, "instruction {index} is missing a stable PaintBase.id")
            }
            Self::DuplicateInstructionId { index } => {
                write!(
                    f,
                    "instruction {index} duplicates an earlier instruction id"
                )
            }
            Self::PatchBufferTooSmall { required, provided } => {
                write!(
                    f,
                    "patch buffer too small: required {required}, provided {provided}"
                )
            }
        }
    }
}

impl std::error::Error for PaletteBlendError {}

// ---------------------------------------------------------------------------
// Input conversion
// ---------------------------------------------------------------------------

/// Converts a numeric blend-mode code (0 Normal, 1 Add, 2 Multiply, 3 Screen, 4 Alpha).
pub fn blend_mode_from_code(code: u8) -> Result<BlendMode, PaletteBlendError> {
    match code {
        0 => Ok(BlendMode::Normal),
        1 => Ok(BlendMode::Add),
        2 => Ok(BlendMode::Multiply),
        3 => Ok(BlendMode::Screen),
        4 => Ok(BlendMode::Alpha),
        other => Err(PaletteBlendError::InvalidBlendMode(other)),
    }
}

/// Maps a `paint-instructions` blend mode onto a hardware mode, when one exists.
pub fn blend_mode_from_paint(mode: &paint_instructions::BlendMode) -> Option<BlendMode> {
    match mode {
        paint_instructions::BlendMode::Normal => Some(BlendMode::Normal),
        paint_instructions::BlendMode::Multiply => Some(BlendMode::Multiply),
        paint_instructions::BlendMode::Screen => Some(BlendMode::Screen),
        _ => None,
    }
}

/// Converts a unit-interval alpha to 8 bits, rejecting NaN, infinity and out-of-range input.
pub fn alpha_from_unit_f32(alpha: f32) -> Result<u8, PaletteBlendError> {
    if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
        return Err(PaletteBlendError::InvalidAlpha);
    }
    Ok((alpha * 255.0 + 0.5) as u8)
}

// ---------------------------------------------------------------------------
// Per-pixel blending
// ---------------------------------------------------------------------------

/// Exact rounded division of `x` (0..=65025) by 255.
#[inline]
fn div255(x: u32) -> u32 {
    let t = x + 128;
    (t + (t >> 8)) >> 8
}

#[inline]
fn blend_channel(mode: BlendMode, s: u8, d: u8) -> u8 {
    let (s, d) = (u32::from(s), u32::from(d));
    let v = match mode {
        BlendMode::Add => (s + d).min(255),
        BlendMode::Multiply => div255(s * d),
        BlendMode::Screen => 255 - div255((255 - s) * (255 - d)),
        BlendMode::Normal | BlendMode::Alpha => s,
    };
    v as u8
}

/// Blends one `src` pixel over one `dst` pixel with global `alpha`.
///
/// [`BlendMode::Normal`] returns `src` unchanged. Every other mode weights the blended
/// color by the effective alpha `src.a * alpha / 255`; an effective alpha of zero returns
/// `dst` exactly. The result alpha is the source-over union of effective and destination
/// alpha. Pure integer arithmetic: deterministic on every platform.
pub fn blend_pixel(mode: BlendMode, src: Rgba8, dst: Rgba8, alpha: u8) -> Rgba8 {
    if mode == BlendMode::Normal {
        return src;
    }
    let ea = div255(u32::from(src[3]) * u32::from(alpha));
    if ea == 0 {
        return dst;
    }
    let inv = 255 - ea;
    let mix = |i: usize| -> u8 {
        let b = u32::from(blend_channel(mode, src[i], dst[i]));
        div255(u32::from(dst[i]) * inv + b * ea) as u8
    };
    let out_a = (ea + div255(u32::from(dst[3]) * inv)).min(255) as u8;
    [mix(0), mix(1), mix(2), out_a]
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_palette(palette: &[Rgba8]) -> Result<(), PaletteBlendError> {
    if palette.is_empty() {
        return Err(PaletteBlendError::EmptyPalette);
    }
    if palette.len() > MAX_PALETTE_ENTRIES {
        return Err(PaletteBlendError::PaletteTooLarge {
            len: palette.len(),
            max: MAX_PALETTE_ENTRIES,
        });
    }
    Ok(())
}

fn validate_indices(indices: &[u8], palette_len: usize) -> Result<(), PaletteBlendError> {
    match indices.iter().find(|&&i| usize::from(i) >= palette_len) {
        Some(&index) => Err(PaletteBlendError::PaletteIndexOutOfRange { index, palette_len }),
        None => Ok(()),
    }
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), PaletteBlendError> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(PaletteBlendError::InvalidDimensions { width, height });
    }
    Ok(())
}

/// Bytes needed for `height` rows of `row_bytes` data at `stride`.
fn required_bytes(
    height: u32,
    stride: usize,
    row_bytes: usize,
) -> Result<usize, PaletteBlendError> {
    let rows = (height as usize).saturating_sub(1);
    rows.checked_mul(stride)
        .and_then(|v| v.checked_add(row_bytes))
        .ok_or(PaletteBlendError::ArithmeticOverflow)
}

fn check_geometry(
    width: u32,
    height: u32,
    stride: usize,
    bytes_per_pixel: usize,
    provided: usize,
) -> Result<(), PaletteBlendError> {
    validate_dimensions(width, height)?;
    let row_bytes = (width as usize)
        .checked_mul(bytes_per_pixel)
        .ok_or(PaletteBlendError::ArithmeticOverflow)?;
    if stride < row_bytes {
        return Err(PaletteBlendError::InvalidStride {
            stride,
            min: row_bytes,
        });
    }
    let required = required_bytes(height, stride, row_bytes)?;
    if provided < required {
        return Err(PaletteBlendError::BufferTooSmall { required, provided });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scanline blits
// ---------------------------------------------------------------------------

/// Draws one pixel; returns whether it was drawn (not color-keyed, index valid).
#[inline]
fn blit_pixel(chunk: &mut [u8], index: u8, palette: &[Rgba8], params: &BlitParams) -> bool {
    if params.transparent_index == Some(index) {
        return false;
    }
    let Some(&color) = palette.get(usize::from(index)) else {
        return false;
    };
    let Some(dst) = chunk.get_mut(..4) else {
        return false;
    };
    let current = [dst[0], dst[1], dst[2], dst[3]];
    let out = blend_pixel(params.mode, color, current, params.alpha);
    dst.copy_from_slice(&out);
    true
}

/// Blits a run of already-validated indices into a destination row slice.
fn blit_row(dst: &mut [u8], indices: &[u8], palette: &[Rgba8], params: &BlitParams) -> usize {
    let mut drawn = 0;
    if params.h_flip {
        for (chunk, &index) in dst.chunks_exact_mut(4).zip(indices.iter().rev()) {
            drawn += usize::from(blit_pixel(chunk, index, palette, params));
        }
    } else {
        for (chunk, &index) in dst.chunks_exact_mut(4).zip(indices.iter()) {
            drawn += usize::from(blit_pixel(chunk, index, palette, params));
        }
    }
    drawn
}

/// Expands one scanline of palette `indices` into RGBA8 `dst` using `params`.
///
/// `dst` must hold at least `indices.len() * 4` bytes; only that prefix is written.
/// All indices are validated before the first byte is written. Returns the number of
/// pixels drawn (color-keyed pixels are skipped). `params.v_flip` is ignored.
pub fn blit_indexed_scanline(
    dst: &mut [u8],
    indices: &[u8],
    palette: &[Rgba8],
    params: &BlitParams,
) -> Result<usize, PaletteBlendError> {
    validate_palette(palette)?;
    if indices.len() > MAX_DIMENSION as usize {
        return Err(PaletteBlendError::InvalidDimensions {
            width: u32::try_from(indices.len()).unwrap_or(u32::MAX),
            height: 1,
        });
    }
    let required = indices
        .len()
        .checked_mul(4)
        .ok_or(PaletteBlendError::ArithmeticOverflow)?;
    let provided = dst.len();
    let target = dst
        .get_mut(..required)
        .ok_or(PaletteBlendError::BufferTooSmall { required, provided })?;
    validate_indices(indices, palette.len())?;
    Ok(blit_row(target, indices, palette, params))
}

/// Blends a solid `color` across every pixel of an RGBA8 span.
///
/// `dst.len()` must be a multiple of 4 and at most `MAX_DIMENSION * 4`.
/// Returns the number of pixels processed.
pub fn blend_solid_span(
    dst: &mut [u8],
    color: Rgba8,
    mode: BlendMode,
    alpha: u8,
) -> Result<usize, PaletteBlendError> {
    if dst.len() & 3 != 0 {
        return Err(PaletteBlendError::MisalignedSpan { len: dst.len() });
    }
    if dst.len() / 4 > MAX_DIMENSION as usize {
        return Err(PaletteBlendError::BufferTooSmall {
            required: dst.len(),
            provided: MAX_DIMENSION as usize * 4,
        });
    }
    let mut count = 0;
    for chunk in dst.chunks_exact_mut(4) {
        let out = blend_pixel(mode, color, [chunk[0], chunk[1], chunk[2], chunk[3]], alpha);
        chunk.copy_from_slice(&out);
        count += 1;
    }
    Ok(count)
}

/// Composites an RGBA8 source span onto an RGBA8 destination span of equal length.
///
/// Both slices must be multiples of 4 bytes, equal in length, and at most
/// `MAX_DIMENSION * 4` bytes. Returns the number of pixels processed.
pub fn blend_rgba_scanline(
    dst: &mut [u8],
    src: &[u8],
    mode: BlendMode,
    alpha: u8,
) -> Result<usize, PaletteBlendError> {
    if dst.len() != src.len() {
        return Err(PaletteBlendError::LengthMismatch {
            dst: dst.len(),
            src: src.len(),
        });
    }
    if dst.len() & 3 != 0 {
        return Err(PaletteBlendError::MisalignedSpan { len: dst.len() });
    }
    if dst.len() / 4 > MAX_DIMENSION as usize {
        return Err(PaletteBlendError::BufferTooSmall {
            required: dst.len(),
            provided: MAX_DIMENSION as usize * 4,
        });
    }
    let mut count = 0;
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        let out = blend_pixel(
            mode,
            [s[0], s[1], s[2], s[3]],
            [d[0], d[1], d[2], d[3]],
            alpha,
        );
        d.copy_from_slice(&out);
        count += 1;
    }
    Ok(count)
}

// ---------------------------------------------------------------------------
// Image blits
// ---------------------------------------------------------------------------

/// Visible region of a clipped blit, in destination and source coordinates.
struct Clip {
    dx0: usize,
    dx1: usize,
    dy0: usize,
    dy1: usize,
    sx_lo: usize,
    sx_hi: usize,
    x: i64,
    y: i64,
}

fn compute_clip(
    target_w: u32,
    target_h: u32,
    x: i32,
    y: i32,
    src: &IndexedImage<'_>,
    h_flip: bool,
) -> Option<Clip> {
    let (x, y) = (i64::from(x), i64::from(y));
    let (sw, sh) = (i64::from(src.width), i64::from(src.height));
    let x0 = x.max(0);
    let x1 = (x + sw).min(i64::from(target_w));
    let y0 = y.max(0);
    let y1 = (y + sh).min(i64::from(target_h));
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    let (lo, hi) = if h_flip {
        (sw - (x1 - x), sw - (x0 - x))
    } else {
        (x0 - x, x1 - x)
    };
    Some(Clip {
        dx0: usize::try_from(x0).ok()?,
        dx1: usize::try_from(x1).ok()?,
        dy0: usize::try_from(y0).ok()?,
        dy1: usize::try_from(y1).ok()?,
        sx_lo: usize::try_from(lo).ok()?,
        sx_hi: usize::try_from(hi).ok()?,
        x,
        y,
    })
}

/// Source row index for destination row `dy`.
fn source_row(clip: &Clip, dy: usize, height: u32, v_flip: bool) -> Option<usize> {
    let rel = i64::try_from(dy).ok()?.checked_sub(clip.y)?;
    let row = if v_flip {
        i64::from(height).checked_sub(1)?.checked_sub(rel)?
    } else {
        rel
    };
    usize::try_from(row).ok()
}

/// Blits an indexed image onto an RGBA8 target at `(x, y)` with clipping.
///
/// The image may extend past any target edge; only the visible region is drawn. All
/// visible palette indices are validated before the first byte is written. Returns the
/// number of pixels drawn.
pub fn blit_indexed_image(
    target: &mut RgbaTarget<'_>,
    x: i32,
    y: i32,
    src: &IndexedImage<'_>,
    palette: &[Rgba8],
    params: &BlitParams,
) -> Result<usize, PaletteBlendError> {
    validate_palette(palette)?;
    check_geometry(src.width, src.height, src.stride, 1, src.pixels.len())?;
    check_geometry(
        target.width,
        target.height,
        target.stride,
        4,
        target.pixels.len(),
    )?;
    let Some(clip) = compute_clip(target.width, target.height, x, y, src, params.h_flip) else {
        return Ok(0);
    };
    let _ = clip.x;

    // Pass 1: validate every visible index so a failure leaves the target untouched.
    for dy in clip.dy0..clip.dy1 {
        let row = source_row(&clip, dy, src.height, params.v_flip)
            .ok_or(PaletteBlendError::ArithmeticOverflow)?;
        let start = row
            .checked_mul(src.stride)
            .and_then(|v| v.checked_add(clip.sx_lo))
            .ok_or(PaletteBlendError::ArithmeticOverflow)?;
        let end = start
            .checked_add(clip.sx_hi - clip.sx_lo)
            .ok_or(PaletteBlendError::ArithmeticOverflow)?;
        let span = src
            .pixels
            .get(start..end)
            .ok_or(PaletteBlendError::BufferTooSmall {
                required: end,
                provided: src.pixels.len(),
            })?;
        validate_indices(span, palette.len())?;
    }

    // Pass 2: draw.
    let stride = target.stride;
    let mut drawn = 0;
    for dy in clip.dy0..clip.dy1 {
        let row = source_row(&clip, dy, src.height, params.v_flip)
            .ok_or(PaletteBlendError::ArithmeticOverflow)?;
        let sstart = row * src.stride + clip.sx_lo;
        let send = sstart + (clip.sx_hi - clip.sx_lo);
        let dstart = dy * stride + clip.dx0 * 4;
        let dend = dy * stride + clip.dx1 * 4;
        let provided = target.pixels.len();
        let (Some(span), Some(out)) = (
            src.pixels.get(sstart..send),
            target.pixels.get_mut(dstart..dend),
        ) else {
            return Err(PaletteBlendError::BufferTooSmall {
                required: dend,
                provided,
            });
        };
        drawn += blit_row(out, span, palette, params);
    }
    Ok(drawn)
}

/// Blits an indexed image into a [`PixelContainer`] (RGBA8, tightly packed).
///
/// Rejects containers whose `data` length disagrees with `width * height * 4`.
pub fn blit_indexed_to_pixel_container(
    container: &mut PixelContainer,
    x: i32,
    y: i32,
    src: &IndexedImage<'_>,
    palette: &[Rgba8],
    params: &BlitParams,
) -> Result<usize, PaletteBlendError> {
    validate_dimensions(container.width, container.height)?;
    let stride = (container.width as usize)
        .checked_mul(4)
        .ok_or(PaletteBlendError::ArithmeticOverflow)?;
    let expected = stride
        .checked_mul(container.height as usize)
        .ok_or(PaletteBlendError::ArithmeticOverflow)?;
    if container.data.len() != expected {
        return Err(PaletteBlendError::BufferTooSmall {
            required: expected,
            provided: container.data.len(),
        });
    }
    let mut target = RgbaTarget {
        width: container.width,
        height: container.height,
        stride,
        pixels: container.data.as_mut_slice(),
    };
    blit_indexed_image(&mut target, x, y, src, palette, params)
}

// ---------------------------------------------------------------------------
// Retained-mode scene diffing
// ---------------------------------------------------------------------------

/// Returns the stable id of an instruction, if it has one.
pub fn instruction_id(instr: &PaintInstruction) -> Option<&str> {
    let base = match instr {
        PaintInstruction::Rect(r) => &r.base,
        PaintInstruction::Ellipse(e) => &e.base,
        PaintInstruction::Path(p) => &p.base,
        PaintInstruction::Text(t) => &t.base,
        PaintInstruction::GlyphRun(g) => &g.base,
        PaintInstruction::Group(g) => &g.base,
        PaintInstruction::Layer(l) => &l.base,
        PaintInstruction::Line(l) => &l.base,
        PaintInstruction::Clip(c) => &c.base,
        PaintInstruction::Gradient(g) => &g.base,
        PaintInstruction::Image(i) => &i.base,
    };
    base.id.as_deref()
}

/// Validates a scene's instruction list: bounded, every id present, ids unique.
fn validate_scene(instructions: &[PaintInstruction]) -> Result<(), PaletteBlendError> {
    if instructions.len() > MAX_SCENE_INSTRUCTIONS {
        return Err(PaletteBlendError::SceneTooLarge {
            count: instructions.len(),
            max: MAX_SCENE_INSTRUCTIONS,
        });
    }
    for (i, instr) in instructions.iter().enumerate() {
        let id =
            instruction_id(instr).ok_or(PaletteBlendError::MissingInstructionId { index: i })?;
        let earlier = instructions.iter().take(i);
        if earlier.into_iter().any(|e| instruction_id(e) == Some(id)) {
            return Err(PaletteBlendError::DuplicateInstructionId { index: i });
        }
    }
    Ok(())
}

/// Finds the index of the instruction with `id`, probing `hint` first.
fn find_by_id(list: &[PaintInstruction], id: &str, hint: usize) -> Option<usize> {
    if let Some(candidate) = list.get(hint) {
        if instruction_id(candidate) == Some(id) {
            return Some(hint);
        }
    }
    list.iter().position(|i| instruction_id(i) == Some(id))
}

fn emit_op(out: &mut [PatchOp], total: &mut usize, op: PatchOp) {
    if let Some(slot) = out.get_mut(*total) {
        *slot = op;
    }
    *total += 1;
}

/// Diffs two scenes by stable instruction id into a caller-supplied slice.
///
/// Emits, in deterministic order, every [`PatchKind::Removed`] op (previous-scene order)
/// followed by [`PatchKind::Modified`] and [`PatchKind::Added`] ops in next-scene order,
/// so applying the ops in sequence reproduces the next scene's painter order. Unchanged
/// instructions are counted but not emitted. Performs zero heap allocation; when
/// `out_ops` is too small the call fails with
/// [`PaletteBlendError::PatchBufferTooSmall`] reporting the exact required capacity.
pub fn diff_paint_scene_into(
    prev: &PaintScene,
    next: &PaintScene,
    out_ops: &mut [PatchOp],
) -> Result<PatchSummary, PaletteBlendError> {
    validate_scene(&prev.instructions)?;
    validate_scene(&next.instructions)?;

    let mut total = 0usize;
    let (mut added, mut modified, mut removed, mut unchanged) = (0usize, 0usize, 0usize, 0usize);

    for (i, instr) in prev.instructions.iter().enumerate() {
        let id = instruction_id(instr).unwrap_or_default();
        if find_by_id(&next.instructions, id, i).is_none() {
            removed += 1;
            emit_op(
                out_ops,
                &mut total,
                PatchOp {
                    kind: PatchKind::Removed,
                    old_index: i as u32,
                    new_index: NO_INDEX,
                },
            );
        }
    }

    for (j, instr) in next.instructions.iter().enumerate() {
        let id = instruction_id(instr).unwrap_or_default();
        match find_by_id(&prev.instructions, id, j) {
            None => {
                added += 1;
                emit_op(
                    out_ops,
                    &mut total,
                    PatchOp {
                        kind: PatchKind::Added,
                        old_index: NO_INDEX,
                        new_index: j as u32,
                    },
                );
            }
            Some(i) => {
                if prev.instructions.get(i) == Some(instr) {
                    unchanged += 1;
                } else {
                    modified += 1;
                    emit_op(
                        out_ops,
                        &mut total,
                        PatchOp {
                            kind: PatchKind::Modified,
                            old_index: i as u32,
                            new_index: j as u32,
                        },
                    );
                }
            }
        }
    }

    if total > out_ops.len() {
        return Err(PaletteBlendError::PatchBufferTooSmall {
            required: total,
            provided: out_ops.len(),
        });
    }

    let header_changed = prev.id != next.id
        || prev.width != next.width
        || prev.height != next.height
        || prev.background != next.background;

    Ok(PatchSummary {
        ops_written: total,
        added,
        modified,
        removed,
        unchanged,
        header_changed,
    })
}
