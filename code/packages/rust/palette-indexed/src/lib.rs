//! # palette-indexed
//!
//! GAME20: deterministic, zero-allocation indexed-colour pixel formats
//! ([`BitDepth::Bpp2`], [`BitDepth::Bpp4`], [`BitDepth::Bpp8`]), fixed-size
//! palette storage, quantized palette fading (NES / Game Boy / GBA style),
//! colour-0 transparency, and scanline / framebuffer blitting into the
//! universal RGBA8 [`PixelContainer`] (`IMG00`).
//!
//! ## Packed layout
//!
//! Indices are stored *chunky*, most-significant-bits first, so the leftmost
//! pixel of a byte lives in its high bits:
//!
//! ```text
//! Bpp2: byte = [p0 p0 | p1 p1 | p2 p2 | p3 p3]   4 pixels / byte
//! Bpp4: byte = [p0 p0 p0 p0 | p1 p1 p1 p1]       2 pixels / byte
//! Bpp8: byte = [p0 p0 p0 p0 p0 p0 p0 p0]         1 pixel  / byte
//! ```
//!
//! Every row starts on a byte boundary; rows are `stride` bytes apart.
//!
//! ## Safety and resource model
//!
//! * `#![forbid(unsafe_code)]`; no pointer arithmetic anywhere.
//! * Every hot path (`pack_row_into`, `unpack_row_into`, fades, blits) writes
//!   into caller-supplied buffers and performs **zero heap allocation**.
//! * Dimensions are capped at [`MAX_DIMENSION`] (CWE-400); every size is
//!   computed with checked arithmetic (CWE-190); divisors are validated
//!   (CWE-369); indices, strides and buffer lengths are validated before any
//!   read or write (CWE-20 / CWE-125 / CWE-787).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use pixel_container::PixelContainer;

/// An RGBA8 colour: `[red, green, blue, alpha]`.
pub type Rgba8 = [u8; 4];

/// Fully transparent black.
pub const TRANSPARENT: Rgba8 = [0, 0, 0, 0];

/// Largest accepted image width or height in pixels (CWE-400 resource cap).
pub const MAX_DIMENSION: u32 = 16_384;

/// Number of colour slots in palette storage (the `Bpp8` maximum).
pub const MAX_PALETTE_ENTRIES: usize = 256;

/// Largest accepted number of fade steps (CWE-400 / CWE-190 cap).
pub const MAX_FADE_STEPS: u16 = 1024;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes of every fallible operation in this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteError {
    /// Width or height is zero.
    InvalidDimensions,
    /// Width or height exceeds [`MAX_DIMENSION`].
    DimensionTooLarge {
        /// The offending dimension.
        value: u32,
    },
    /// A size computation overflowed `usize` (CWE-190).
    SizeOverflow,
    /// The row stride is smaller than one packed row.
    StrideTooSmall {
        /// Supplied stride in bytes.
        stride: usize,
        /// Minimum stride in bytes.
        required: usize,
    },
    /// A buffer is shorter than the operation needs.
    BufferTooSmall {
        /// Bytes (or elements) needed.
        required: usize,
        /// Bytes (or elements) supplied.
        actual: usize,
    },
    /// A palette index does not fit the bit depth or palette.
    IndexOutOfRange {
        /// The offending index.
        index: u8,
        /// Exclusive upper bound.
        limit: usize,
    },
    /// A palette has fewer entries than the image bit depth can address.
    PaletteTooSmall {
        /// Entries required.
        required: usize,
        /// Entries available.
        actual: usize,
    },
    /// More colours were supplied than the palette depth can hold.
    TooManyColors {
        /// Colours supplied.
        supplied: usize,
        /// Palette capacity.
        capacity: usize,
    },
    /// Fade `steps` is zero / above [`MAX_FADE_STEPS`], or `step > steps`.
    InvalidFadeStep,
    /// Two operands were required to share a bit depth but do not.
    DepthMismatch,
    /// A pixel coordinate lies outside the image.
    OutOfBounds,
    /// A `PixelContainer` has `data.len() != width * height * 4`.
    ContainerMismatch,
}

impl std::fmt::Display for PaletteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions => write!(f, "width and height must be non-zero"),
            Self::DimensionTooLarge { value } => {
                write!(f, "dimension {value} exceeds maximum {MAX_DIMENSION}")
            }
            Self::SizeOverflow => write!(f, "size computation overflowed"),
            Self::StrideTooSmall { stride, required } => {
                write!(f, "stride {stride} is smaller than packed row {required}")
            }
            Self::BufferTooSmall { required, actual } => {
                write!(f, "buffer too small: need {required}, have {actual}")
            }
            Self::IndexOutOfRange { index, limit } => {
                write!(f, "palette index {index} out of range (limit {limit})")
            }
            Self::PaletteTooSmall { required, actual } => {
                write!(f, "palette too small: need {required}, have {actual}")
            }
            Self::TooManyColors { supplied, capacity } => {
                write!(f, "{supplied} colours exceed palette capacity {capacity}")
            }
            Self::InvalidFadeStep => write!(f, "invalid fade step or step count"),
            Self::DepthMismatch => write!(f, "bit depth mismatch"),
            Self::OutOfBounds => write!(f, "coordinate out of bounds"),
            Self::ContainerMismatch => {
                write!(f, "pixel container length does not match dimensions")
            }
        }
    }
}

impl std::error::Error for PaletteError {}

// ---------------------------------------------------------------------------
// Bit depth
// ---------------------------------------------------------------------------

/// Indexed pixel bit depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BitDepth {
    /// 2 bits per pixel, 4 colours (Game Boy).
    Bpp2,
    /// 4 bits per pixel, 16 colours (NES sprites, GBA 16-colour tiles).
    Bpp4,
    /// 8 bits per pixel, 256 colours (GBA 256-colour tiles, VGA mode 13h).
    Bpp8,
}

impl BitDepth {
    /// Bits used by one pixel index.
    pub const fn bits(self) -> usize {
        match self {
            Self::Bpp2 => 2,
            Self::Bpp4 => 4,
            Self::Bpp8 => 8,
        }
    }

    /// Number of distinct colours addressable at this depth.
    pub const fn color_count(self) -> usize {
        1usize << self.bits()
    }

    /// Pixel indices packed into one byte.
    pub const fn pixels_per_byte(self) -> usize {
        8 / self.bits()
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

fn check_dimension(value: u32) -> Result<(), PaletteError> {
    if value == 0 {
        Err(PaletteError::InvalidDimensions)
    } else if value > MAX_DIMENSION {
        Err(PaletteError::DimensionTooLarge { value })
    } else {
        Ok(())
    }
}

/// Bytes occupied by one packed row of `width` pixels.
///
/// # Errors
///
/// [`PaletteError::InvalidDimensions`] / [`PaletteError::DimensionTooLarge`].
pub fn packed_row_len(depth: BitDepth, width: u32) -> Result<usize, PaletteError> {
    check_dimension(width)?;
    let ppb = depth.pixels_per_byte();
    // width <= 16384, so this cannot overflow.
    Ok((width as usize).div_ceil(ppb))
}

/// Minimum backing-buffer length for a `width x height` image with `stride`.
///
/// The last row only needs one packed row, not a full stride.
///
/// # Errors
///
/// Dimension errors, [`PaletteError::StrideTooSmall`], or
/// [`PaletteError::SizeOverflow`].
pub fn required_buffer_len(
    depth: BitDepth,
    width: u32,
    height: u32,
    stride: usize,
) -> Result<usize, PaletteError> {
    check_dimension(height)?;
    let row = packed_row_len(depth, width)?;
    if stride < row {
        return Err(PaletteError::StrideTooSmall {
            stride,
            required: row,
        });
    }
    (height as usize - 1)
        .checked_mul(stride)
        .and_then(|n| n.checked_add(row))
        .ok_or(PaletteError::SizeOverflow)
}

// ---------------------------------------------------------------------------
// Pack / unpack
// ---------------------------------------------------------------------------

/// Packs one row of unpacked pixel `indices` into `out` (MSB-first).
///
/// All indices are validated before `out` is touched. Returns the number of
/// packed bytes written. Zero heap allocation.
///
/// # Errors
///
/// Dimension errors, [`PaletteError::IndexOutOfRange`] (index does not fit
/// `depth`), or [`PaletteError::BufferTooSmall`].
pub fn pack_row_into(
    depth: BitDepth,
    indices: &[u8],
    out: &mut [u8],
) -> Result<usize, PaletteError> {
    let width = u32::try_from(indices.len()).map_err(|_| PaletteError::SizeOverflow)?;
    let need = packed_row_len(depth, width)?;
    if out.len() < need {
        return Err(PaletteError::BufferTooSmall {
            required: need,
            actual: out.len(),
        });
    }
    let limit = depth.color_count();
    for &index in indices {
        if usize::from(index) >= limit {
            return Err(PaletteError::IndexOutOfRange { index, limit });
        }
    }
    let bits = depth.bits();
    let ppb = depth.pixels_per_byte();
    for byte in out.iter_mut().take(need) {
        *byte = 0;
    }
    for (i, &index) in indices.iter().enumerate() {
        let shift = 8 - bits * (i % ppb + 1);
        if let Some(byte) = out.get_mut(i / ppb) {
            *byte |= index << shift;
        }
    }
    Ok(need)
}

/// Unpacks `width` pixel indices from `packed` into `out`.
///
/// Zero heap allocation. Unpacked indices are always `< depth.color_count()`.
///
/// # Errors
///
/// Dimension errors or [`PaletteError::BufferTooSmall`] for either buffer.
pub fn unpack_row_into(
    depth: BitDepth,
    packed: &[u8],
    width: u32,
    out: &mut [u8],
) -> Result<(), PaletteError> {
    let need = packed_row_len(depth, width)?;
    if packed.len() < need {
        return Err(PaletteError::BufferTooSmall {
            required: need,
            actual: packed.len(),
        });
    }
    let w = width as usize;
    if out.len() < w {
        return Err(PaletteError::BufferTooSmall {
            required: w,
            actual: out.len(),
        });
    }
    for (x, slot) in out.iter_mut().take(w).enumerate() {
        *slot = raw_index(depth, packed, x).ok_or(PaletteError::BufferTooSmall {
            required: need,
            actual: packed.len(),
        })?;
    }
    Ok(())
}

/// Reads the index of pixel `x` from a packed row. `None` if out of range.
fn raw_index(depth: BitDepth, row: &[u8], x: usize) -> Option<u8> {
    let ppb = depth.pixels_per_byte();
    let shift = 8 - depth.bits() * (x % ppb + 1);
    let mask = (depth.color_count() - 1) as u8;
    let byte = *row.get(x / ppb)?;
    Some((byte >> shift) & mask)
}

/// Reads the palette index of pixel `x` from a packed row.
///
/// # Errors
///
/// [`PaletteError::OutOfBounds`] if `x` lies beyond `packed`.
pub fn get_index(depth: BitDepth, packed: &[u8], x: usize) -> Result<u8, PaletteError> {
    raw_index(depth, packed, x).ok_or(PaletteError::OutOfBounds)
}

/// Writes the palette index of pixel `x` into a packed row.
///
/// # Errors
///
/// [`PaletteError::IndexOutOfRange`] if `index` does not fit `depth`, or
/// [`PaletteError::OutOfBounds`] if `x` lies beyond `packed`.
pub fn set_index(
    depth: BitDepth,
    packed: &mut [u8],
    x: usize,
    index: u8,
) -> Result<(), PaletteError> {
    let limit = depth.color_count();
    if usize::from(index) >= limit {
        return Err(PaletteError::IndexOutOfRange { index, limit });
    }
    let ppb = depth.pixels_per_byte();
    let shift = 8 - depth.bits() * (x % ppb + 1);
    let mask = (limit - 1) as u8;
    let byte = packed.get_mut(x / ppb).ok_or(PaletteError::OutOfBounds)?;
    *byte = (*byte & !(mask << shift)) | (index << shift);
    Ok(())
}

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

/// Fixed-size (1 KiB), `Copy` colour palette with `depth.color_count()` slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    depth: BitDepth,
    colors: [Rgba8; MAX_PALETTE_ENTRIES],
}

impl Palette {
    /// A palette of `depth` where every entry is [`TRANSPARENT`].
    pub const fn blank(depth: BitDepth) -> Self {
        Self {
            depth,
            colors: [TRANSPARENT; MAX_PALETTE_ENTRIES],
        }
    }

    /// Builds a palette from up to `depth.color_count()` colours; unspecified
    /// trailing entries stay [`TRANSPARENT`].
    ///
    /// # Errors
    ///
    /// [`PaletteError::TooManyColors`] if `colors` is too long.
    pub fn from_colors(depth: BitDepth, colors: &[Rgba8]) -> Result<Self, PaletteError> {
        let capacity = depth.color_count();
        if colors.len() > capacity {
            return Err(PaletteError::TooManyColors {
                supplied: colors.len(),
                capacity,
            });
        }
        let mut palette = Self::blank(depth);
        for (slot, color) in palette.colors.iter_mut().zip(colors) {
            *slot = *color;
        }
        Ok(palette)
    }

    /// The bit depth this palette serves.
    pub const fn palette_depth(&self) -> BitDepth {
        self.depth
    }

    /// Number of addressable entries (`4`, `16` or `256`).
    pub const fn entry_count(&self) -> usize {
        self.depth.color_count()
    }

    /// The active entries as a slice.
    pub fn active_entries(&self) -> &[Rgba8] {
        let end = self.entry_count().min(MAX_PALETTE_ENTRIES);
        self.colors.get(..end).unwrap_or(&[])
    }

    /// Reads entry `index`.
    ///
    /// # Errors
    ///
    /// [`PaletteError::IndexOutOfRange`] if `index >= entry_count()`.
    pub fn entry(&self, index: u8) -> Result<Rgba8, PaletteError> {
        let limit = self.entry_count();
        if usize::from(index) >= limit {
            return Err(PaletteError::IndexOutOfRange { index, limit });
        }
        self.colors
            .get(usize::from(index))
            .copied()
            .ok_or(PaletteError::IndexOutOfRange { index, limit })
    }

    /// Writes entry `index`.
    ///
    /// # Errors
    ///
    /// [`PaletteError::IndexOutOfRange`] if `index >= entry_count()`.
    pub fn set_entry(&mut self, index: u8, color: Rgba8) -> Result<(), PaletteError> {
        let limit = self.entry_count();
        if usize::from(index) >= limit {
            return Err(PaletteError::IndexOutOfRange { index, limit });
        }
        match self.colors.get_mut(usize::from(index)) {
            Some(slot) => {
                *slot = color;
                Ok(())
            }
            None => Err(PaletteError::IndexOutOfRange { index, limit }),
        }
    }

    /// Resolves `index` to a colour, honouring colour-0 transparency: when
    /// `color0_transparent` is set, index 0 resolves to [`TRANSPARENT`].
    ///
    /// # Errors
    ///
    /// [`PaletteError::IndexOutOfRange`] if `index >= entry_count()`.
    pub fn resolve(&self, index: u8, color0_transparent: bool) -> Result<Rgba8, PaletteError> {
        if index == 0 && color0_transparent {
            return Ok(TRANSPARENT);
        }
        self.entry(index)
    }
}

// ---------------------------------------------------------------------------
// Quantized fading
// ---------------------------------------------------------------------------

/// Per-channel precision of a console's colour hardware.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelBits {
    /// 8 bits per channel (no quantization).
    Bits8,
    /// 6 bits per channel (VGA DAC).
    Bits6,
    /// 5 bits per channel (GBA, SNES, Game Boy Color).
    Bits5,
    /// 4 bits per channel (Amiga OCS, 12-bit RGB).
    Bits4,
    /// 3 bits per channel (Mega Drive / Genesis, Atari ST).
    Bits3,
    /// 2 bits per channel (64-colour RGB222 systems).
    Bits2,
}

impl ChannelBits {
    /// Number of significant bits per channel.
    pub const fn channel_width(self) -> u32 {
        match self {
            Self::Bits8 => 8,
            Self::Bits6 => 6,
            Self::Bits5 => 5,
            Self::Bits4 => 4,
            Self::Bits3 => 3,
            Self::Bits2 => 2,
        }
    }
}

/// Quantizes one 8-bit channel to `bits` of precision, then expands back to
/// 8 bits by bit replication so full intensity stays `255`.
pub fn quantize_channel(value: u8, bits: ChannelBits) -> u8 {
    let n = bits.channel_width();
    let top = u16::from(value >> (8 - n));
    let mut acc = top;
    let mut width = n;
    while width < 8 {
        acc = (acc << n) | top;
        width += n;
    }
    // width is in 8..=13 here; the shifted value always fits 8 bits.
    (acc >> (width - 8)) as u8
}

/// Quantizes the RGB channels of `color` (alpha is preserved).
pub fn quantize_color(color: Rgba8, bits: ChannelBits) -> Rgba8 {
    [
        quantize_channel(color[0], bits),
        quantize_channel(color[1], bits),
        quantize_channel(color[2], bits),
        color[3],
    ]
}

fn check_fade(step: u16, steps: u16) -> Result<(), PaletteError> {
    if steps == 0 || steps > MAX_FADE_STEPS || step > steps {
        Err(PaletteError::InvalidFadeStep)
    } else {
        Ok(())
    }
}

fn lerp_channel(from: u8, to: u8, step: u16, steps: u16) -> u8 {
    let steps = u32::from(steps);
    let step = u32::from(step);
    // Rounded integer lerp; max numerator 255 * 1024 + 512 fits u32 easily.
    let v = (u32::from(from) * (steps - step) + u32::from(to) * step + steps / 2) / steps;
    v.min(255) as u8
}

/// Fades `from` toward `to` by `step / steps` and quantizes the result.
///
/// `step == 0` yields (quantized) `from`; `step == steps` yields (quantized)
/// `to`. Alpha is taken from `from`.
///
/// # Errors
///
/// [`PaletteError::InvalidFadeStep`] if `steps` is `0` or above
/// [`MAX_FADE_STEPS`], or if `step > steps` (CWE-369 / CWE-20).
pub fn fade_color(
    from: Rgba8,
    to: Rgba8,
    step: u16,
    steps: u16,
    bits: ChannelBits,
) -> Result<Rgba8, PaletteError> {
    check_fade(step, steps)?;
    let mixed = [
        lerp_channel(from[0], to[0], step, steps),
        lerp_channel(from[1], to[1], step, steps),
        lerp_channel(from[2], to[2], step, steps),
        from[3],
    ];
    Ok(quantize_color(mixed, bits))
}

/// Fades every entry of `src` toward the solid colour `target` into `dst`
/// (fade-to-black / fade-to-white). `dst` takes `src`'s depth. Zero heap
/// allocation.
///
/// # Errors
///
/// [`PaletteError::InvalidFadeStep`] (see [`fade_color`]).
pub fn fade_palette_into(
    src: &Palette,
    dst: &mut Palette,
    target: Rgba8,
    step: u16,
    steps: u16,
    bits: ChannelBits,
) -> Result<(), PaletteError> {
    check_fade(step, steps)?;
    dst.depth = src.depth;
    let count = src.entry_count();
    for (out, color) in dst.colors.iter_mut().zip(src.colors.iter()).take(count) {
        *out = fade_color(*color, target, step, steps, bits)?;
    }
    Ok(())
}

/// Cross-fades palette `a` into palette `b` by `step / steps` into `dst`.
/// Zero heap allocation.
///
/// # Errors
///
/// [`PaletteError::DepthMismatch`] if `a` and `b` differ in depth, or
/// [`PaletteError::InvalidFadeStep`].
pub fn crossfade_palette_into(
    a: &Palette,
    b: &Palette,
    dst: &mut Palette,
    step: u16,
    steps: u16,
    bits: ChannelBits,
) -> Result<(), PaletteError> {
    if a.depth != b.depth {
        return Err(PaletteError::DepthMismatch);
    }
    check_fade(step, steps)?;
    dst.depth = a.depth;
    let count = a.entry_count();
    for ((out, from), to) in dst
        .colors
        .iter_mut()
        .zip(a.colors.iter())
        .zip(b.colors.iter())
        .take(count)
    {
        *out = fade_color(*from, *to, step, steps, bits)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Blit options and clipping
// ---------------------------------------------------------------------------

/// Options controlling indexed blits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BlitOptions {
    /// Treat palette index 0 as transparent: such pixels leave the
    /// destination untouched.
    pub color0_transparent: bool,
    /// Mirror the source horizontally.
    pub h_flip: bool,
    /// Mirror the source vertically.
    pub v_flip: bool,
}

/// Opaque, unflipped blit (background layers).
pub const BLIT_OPAQUE: BlitOptions = BlitOptions {
    color0_transparent: false,
    h_flip: false,
    v_flip: false,
};

/// Colour-0-transparent, unflipped blit (sprites).
pub const BLIT_SPRITE: BlitOptions = BlitOptions {
    color0_transparent: true,
    h_flip: false,
    v_flip: false,
};

/// Destination rectangle after clipping, in destination coordinates.
struct Clip {
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
    dst_x: i64,
    dst_y: i64,
}

fn clip_rect(
    src_w: usize,
    src_h: usize,
    dst_w: usize,
    dst_h: usize,
    dst_x: i32,
    dst_y: i32,
) -> Option<Clip> {
    let dx = i64::from(dst_x);
    let dy = i64::from(dst_y);
    // All operands are bounded (< 2^32 + 2^31) so i64 cannot overflow.
    let x0 = dx.max(0);
    let y0 = dy.max(0);
    let x1 = (dx + src_w as i64).min(dst_w as i64);
    let y1 = (dy + src_h as i64).min(dst_h as i64);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(Clip {
        x0: x0 as usize,
        x1: x1 as usize,
        y0: y0 as usize,
        y1: y1 as usize,
        dst_x: dx,
        dst_y: dy,
    })
}

// ---------------------------------------------------------------------------
// Indexed image views
// ---------------------------------------------------------------------------

/// Borrowed, read-only view of a packed indexed image.
#[derive(Clone, Copy, Debug)]
pub struct IndexedImage<'a> {
    depth: BitDepth,
    width: u32,
    height: u32,
    stride: usize,
    data: &'a [u8],
}

impl<'a> IndexedImage<'a> {
    /// Wraps `data` as a `width x height` image with `stride` bytes per row.
    ///
    /// # Errors
    ///
    /// Dimension / stride errors, or [`PaletteError::BufferTooSmall`] when
    /// `data` is shorter than [`required_buffer_len`].
    pub fn view(
        depth: BitDepth,
        width: u32,
        height: u32,
        stride: usize,
        data: &'a [u8],
    ) -> Result<Self, PaletteError> {
        let required = required_buffer_len(depth, width, height, stride)?;
        if data.len() < required {
            return Err(PaletteError::BufferTooSmall {
                required,
                actual: data.len(),
            });
        }
        Ok(Self {
            depth,
            width,
            height,
            stride,
            data,
        })
    }

    /// Pixel bit depth.
    pub const fn bit_depth(&self) -> BitDepth {
        self.depth
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Bytes between the starts of consecutive rows.
    pub const fn stride(&self) -> usize {
        self.stride
    }

    /// The packed bytes of row `y`, or `None` if `y >= height`.
    pub fn row_bytes(&self, y: u32) -> Option<&'a [u8]> {
        if y >= self.height {
            return None;
        }
        let row = packed_row_len(self.depth, self.width).ok()?;
        let start = (y as usize).checked_mul(self.stride)?;
        let end = start.checked_add(row)?;
        self.data.get(start..end)
    }

    /// The palette index at `(x, y)`, or `None` if out of bounds.
    pub fn index_at(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.width {
            return None;
        }
        raw_index(self.depth, self.row_bytes(y)?, x as usize)
    }

    /// Unpacks row `y` into `out` (one index per byte). Zero allocation.
    ///
    /// # Errors
    ///
    /// [`PaletteError::OutOfBounds`] if `y >= height`, or buffer errors.
    pub fn unpack_scanline_into(&self, y: u32, out: &mut [u8]) -> Result<(), PaletteError> {
        let row = self.row_bytes(y).ok_or(PaletteError::OutOfBounds)?;
        unpack_row_into(self.depth, row, self.width, out)
    }
}

/// Borrowed, mutable view of a packed indexed image (an indexed framebuffer).
#[derive(Debug)]
pub struct IndexedCanvas<'a> {
    depth: BitDepth,
    width: u32,
    height: u32,
    stride: usize,
    data: &'a mut [u8],
}

impl<'a> IndexedCanvas<'a> {
    /// Wraps `data` as a mutable `width x height` framebuffer.
    ///
    /// # Errors
    ///
    /// Same as [`IndexedImage::view`].
    pub fn attach(
        depth: BitDepth,
        width: u32,
        height: u32,
        stride: usize,
        data: &'a mut [u8],
    ) -> Result<Self, PaletteError> {
        let required = required_buffer_len(depth, width, height, stride)?;
        if data.len() < required {
            return Err(PaletteError::BufferTooSmall {
                required,
                actual: data.len(),
            });
        }
        Ok(Self {
            depth,
            width,
            height,
            stride,
            data,
        })
    }

    /// Re-borrows the canvas as a read-only [`IndexedImage`].
    pub fn as_image(&self) -> IndexedImage<'_> {
        IndexedImage {
            depth: self.depth,
            width: self.width,
            height: self.height,
            stride: self.stride,
            data: self.data,
        }
    }

    fn row_range(&self, y: usize) -> Option<(usize, usize)> {
        let start = y.checked_mul(self.stride)?;
        let row = packed_row_len(self.depth, self.width).ok()?;
        Some((start, start.checked_add(row)?))
    }

    /// Reads the palette index at `(x, y)`, or `None` if out of bounds.
    pub fn peek_index(&self, x: u32, y: u32) -> Option<u8> {
        self.as_image().index_at(x, y)
    }

    /// Writes palette `index` at `(x, y)`.
    ///
    /// # Errors
    ///
    /// [`PaletteError::OutOfBounds`] or [`PaletteError::IndexOutOfRange`].
    pub fn put_index(&mut self, x: u32, y: u32, index: u8) -> Result<(), PaletteError> {
        if x >= self.width || y >= self.height {
            return Err(PaletteError::OutOfBounds);
        }
        let (start, end) = self
            .row_range(y as usize)
            .ok_or(PaletteError::SizeOverflow)?;
        let row = self
            .data
            .get_mut(start..end)
            .ok_or(PaletteError::OutOfBounds)?;
        set_index(self.depth, row, x as usize, index)
    }

    /// Fills every pixel with palette `index`. Zero allocation.
    ///
    /// # Errors
    ///
    /// [`PaletteError::IndexOutOfRange`] if `index` does not fit the depth.
    pub fn clear_indices(&mut self, index: u8) -> Result<(), PaletteError> {
        let limit = self.depth.color_count();
        if usize::from(index) >= limit {
            return Err(PaletteError::IndexOutOfRange { index, limit });
        }
        let ppb = self.depth.pixels_per_byte();
        let bits = self.depth.bits();
        let mut fill = 0u8;
        for slot in 0..ppb {
            fill |= index << (8 - bits * (slot + 1));
        }
        for y in 0..self.height as usize {
            let (start, end) = self.row_range(y).ok_or(PaletteError::SizeOverflow)?;
            let row = self
                .data
                .get_mut(start..end)
                .ok_or(PaletteError::OutOfBounds)?;
            for byte in row.iter_mut() {
                *byte = fill;
            }
            // Padding pixels past `width` in a final partial byte receive the
            // fill value too; they are never addressable.
        }
        Ok(())
    }

    /// Blits indexed image `src` onto this canvas at `(dst_x, dst_y)`
    /// (clipped), honouring colour-0 transparency and flips. Both images must
    /// share a bit depth. Returns the number of pixels written. Zero
    /// allocation.
    ///
    /// # Errors
    ///
    /// [`PaletteError::DepthMismatch`].
    pub fn blit_indexed(
        &mut self,
        src: &IndexedImage<'_>,
        dst_x: i32,
        dst_y: i32,
        opts: BlitOptions,
    ) -> Result<usize, PaletteError> {
        if src.depth != self.depth {
            return Err(PaletteError::DepthMismatch);
        }
        let Some(clip) = clip_rect(
            src.width as usize,
            src.height as usize,
            self.width as usize,
            self.height as usize,
            dst_x,
            dst_y,
        ) else {
            return Ok(0);
        };
        let mut written = 0usize;
        for dy in clip.y0..clip.y1 {
            let sy = map_axis(dy as i64 - clip.dst_y, src.height as usize, opts.v_flip);
            let src_row = src.row_bytes(sy as u32).ok_or(PaletteError::OutOfBounds)?;
            let (start, end) = self.row_range(dy).ok_or(PaletteError::SizeOverflow)?;
            let dst_row = self
                .data
                .get_mut(start..end)
                .ok_or(PaletteError::OutOfBounds)?;
            for dx in clip.x0..clip.x1 {
                let sx = map_axis(dx as i64 - clip.dst_x, src.width as usize, opts.h_flip);
                let index = raw_index(src.depth, src_row, sx).ok_or(PaletteError::OutOfBounds)?;
                if index == 0 && opts.color0_transparent {
                    continue;
                }
                set_index(self.depth, dst_row, dx, index)?;
                written += 1;
            }
        }
        Ok(written)
    }
}

/// Maps a destination offset to a source coordinate, optionally mirrored.
fn map_axis(offset: i64, extent: usize, flip: bool) -> usize {
    let off = offset.max(0) as usize;
    if flip {
        extent.saturating_sub(1).saturating_sub(off)
    } else {
        off
    }
}

// ---------------------------------------------------------------------------
// Blitting into RGBA8
// ---------------------------------------------------------------------------

fn check_palette(img_depth: BitDepth, palette: &Palette) -> Result<(), PaletteError> {
    let required = img_depth.color_count();
    let actual = palette.entry_count();
    if actual < required {
        Err(PaletteError::PaletteTooSmall { required, actual })
    } else {
        Ok(())
    }
}

fn check_container(dst: &PixelContainer) -> Result<(), PaletteError> {
    let expected = (dst.width as usize)
        .checked_mul(dst.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(PaletteError::SizeOverflow)?;
    if dst.data.len() == expected {
        Ok(())
    } else {
        Err(PaletteError::ContainerMismatch)
    }
}

/// Expands one packed row into RGBA8 bytes in `out` (a line buffer).
///
/// When `color0_transparent` is set, index 0 becomes [`TRANSPARENT`].
/// Returns the number of bytes written (`width * 4`). Zero heap allocation.
///
/// # Errors
///
/// Dimension errors, [`PaletteError::PaletteTooSmall`], or
/// [`PaletteError::BufferTooSmall`] for either buffer.
pub fn expand_row_to_rgba_into(
    depth: BitDepth,
    packed: &[u8],
    width: u32,
    palette: &Palette,
    color0_transparent: bool,
    out: &mut [u8],
) -> Result<usize, PaletteError> {
    check_palette(depth, palette)?;
    let need = packed_row_len(depth, width)?;
    if packed.len() < need {
        return Err(PaletteError::BufferTooSmall {
            required: need,
            actual: packed.len(),
        });
    }
    let bytes = (width as usize)
        .checked_mul(4)
        .ok_or(PaletteError::SizeOverflow)?;
    if out.len() < bytes {
        return Err(PaletteError::BufferTooSmall {
            required: bytes,
            actual: out.len(),
        });
    }
    for (x, px) in out
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .take(width as usize)
        .enumerate()
    {
        let index = raw_index(depth, packed, x).ok_or(PaletteError::OutOfBounds)?;
        *px = palette.resolve(index, color0_transparent)?;
    }
    Ok(bytes)
}

fn blit_row(
    img: &IndexedImage<'_>,
    src_y: u32,
    palette: &Palette,
    opts: BlitOptions,
    dst: &mut PixelContainer,
    dst_x: i32,
    dst_y: usize,
) -> Result<usize, PaletteError> {
    let src_row = img.row_bytes(src_y).ok_or(PaletteError::OutOfBounds)?;
    let dst_w = dst.width as usize;
    let dx0 = i64::from(dst_x);
    let x0 = dx0.max(0);
    let x1 = (dx0 + i64::from(img.width)).min(dst_w as i64);
    if x1 <= x0 {
        return Ok(0);
    }
    let mut written = 0usize;
    for dx in (x0 as usize)..(x1 as usize) {
        let sx = map_axis(dx as i64 - dx0, img.width as usize, opts.h_flip);
        let index = raw_index(img.depth, src_row, sx).ok_or(PaletteError::OutOfBounds)?;
        if index == 0 && opts.color0_transparent {
            continue;
        }
        let color = palette.entry(index)?;
        let offset = dst_y
            .checked_mul(dst_w)
            .and_then(|n| n.checked_add(dx))
            .and_then(|n| n.checked_mul(4))
            .ok_or(PaletteError::SizeOverflow)?;
        let end = offset.checked_add(4).ok_or(PaletteError::SizeOverflow)?;
        dst.data
            .get_mut(offset..end)
            .ok_or(PaletteError::OutOfBounds)?
            .copy_from_slice(&color);
        written += 1;
    }
    Ok(written)
}

/// Blits one scanline `src_y` of `img` into `dst` at `(dst_x, dst_y)`,
/// clipped to the container. Calling this per scanline with different
/// palettes gives raster (HBlank) palette effects. Returns pixels written.
/// Zero heap allocation.
///
/// # Errors
///
/// [`PaletteError::OutOfBounds`] (`src_y >= height`),
/// [`PaletteError::PaletteTooSmall`], [`PaletteError::ContainerMismatch`].
pub fn blit_scanline_to_container(
    img: &IndexedImage<'_>,
    src_y: u32,
    palette: &Palette,
    opts: BlitOptions,
    dst: &mut PixelContainer,
    dst_x: i32,
    dst_y: i32,
) -> Result<usize, PaletteError> {
    check_palette(img.depth, palette)?;
    check_container(dst)?;
    if src_y >= img.height {
        return Err(PaletteError::OutOfBounds);
    }
    if dst_y < 0 || i64::from(dst_y) >= i64::from(dst.height) {
        return Ok(0);
    }
    blit_row(img, src_y, palette, opts, dst, dst_x, dst_y as usize)
}

/// Blits the whole indexed image `img` into `dst` at `(dst_x, dst_y)`,
/// clipped to the container, honouring colour-0 transparency and flips.
/// Returns the number of pixels written. Zero heap allocation.
///
/// # Errors
///
/// [`PaletteError::PaletteTooSmall`] or [`PaletteError::ContainerMismatch`].
pub fn blit_indexed_to_container(
    img: &IndexedImage<'_>,
    palette: &Palette,
    opts: BlitOptions,
    dst: &mut PixelContainer,
    dst_x: i32,
    dst_y: i32,
) -> Result<usize, PaletteError> {
    check_palette(img.depth, palette)?;
    check_container(dst)?;
    let Some(clip) = clip_rect(
        img.width as usize,
        img.height as usize,
        dst.width as usize,
        dst.height as usize,
        dst_x,
        dst_y,
    ) else {
        return Ok(0);
    };
    let mut written = 0usize;
    for dy in clip.y0..clip.y1 {
        let sy = map_axis(dy as i64 - clip.dst_y, img.height as usize, opts.v_flip);
        written += blit_row(img, sy as u32, palette, opts, dst, dst_x, dy)?;
    }
    Ok(written)
}
