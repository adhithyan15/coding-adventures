//! # sprite-oam
//!
//! GAME22: a deterministic, zero-allocation, 128-sprite hardware **OAM**
//! (Object Attribute Memory) manager, priority sorter, scanline sprite-limit
//! evaluator and metasprite animation sequencer in the style of the NES PPU
//! and the Game Boy Advance.
//!
//! ## Model
//!
//! * [`OamTable`] owns [`OAM_SLOTS`] (128) [`OamEntry`] slots and
//!   [`AFFINE_SLOTS`] (32) [`AffineParams`] matrices (8.8 fixed point,
//!   convertible to and from [`affine2d::Affine2D`]).
//! * An [`OamEntry`] carries position, [`SpriteShape`] / [`SpriteSize`], H/V
//!   flips, palette bank (0..=15), priority (0..=3, lower is *in front*),
//!   affine matrix slot, tile index, and hidden / affine / double-size flags.
//! * Draw order is **priority first, then OAM index**: for equal priority the
//!   lower slot is in front, exactly like the hardware.
//! * Scanline evaluation walks slots in ascending OAM order and keeps at most
//!   `limit` sprites per line (NES: 8, GBA: up to 128), reporting overflow.
//! * [`Animation`] / [`Animator`] sequence [`MetaFrame`]s (groups of up to
//!   [`MAX_PARTS`] hardware sprites) with per-frame durations in
//!   [`PlayMode::OneShot`], [`PlayMode::Loop`] or [`PlayMode::PingPong`] mode.
//! * Output lowers to [`paint_batch_tiles::SpriteQuad`] (and from there to a
//!   `paint-instructions` scene) with palette banks resolved through
//!   `palette-indexed` palettes.
//!
//! ## Safety and resource model
//!
//! * `#![forbid(unsafe_code)]`; no pointer arithmetic.
//! * Sort, scanline evaluation, animation ticks, stamping and quad lowering
//!   write into caller buffers or fixed storage: **zero heap allocation**
//!   (CWE-400). Only [`OamTable::lower_to_paint_scene`] allocates, because the
//!   scene it returns is heap-backed.
//! * Every slot, affine slot, priority, palette bank, tile index, coordinate,
//!   frame count, part count, duration and tick delta is validated before use
//!   (CWE-20 / CWE-125 / CWE-787); arithmetic is checked or done in `i32`/`i64`
//!   with proven headroom (CWE-190); divisors are never caller controlled
//!   (CWE-369).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use affine2d::Affine2D;
pub use paint_batch_tiles::{PaintScene, SpriteQuad};
pub use palette_indexed::Palette;

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

/// Number of hardware sprite slots in OAM.
pub const OAM_SLOTS: usize = 128;

/// Number of affine matrix slots (GBA: 32 groups of PA/PB/PC/PD).
pub const AFFINE_SLOTS: usize = 32;

/// Largest priority value (0 is frontmost, 3 is backmost).
pub const MAX_PRIORITY: u8 = 3;

/// Largest palette bank index (16 banks of 16 colours).
pub const MAX_PALETTE_BANK: u8 = 15;

/// Largest tile index (10-bit, as on the GBA).
pub const MAX_TILE_INDEX: u16 = 1023;

/// Smallest accepted sprite coordinate.
pub const COORD_MIN: i16 = -1024;

/// Largest accepted sprite coordinate.
pub const COORD_MAX: i16 = 1023;

/// Largest accepted screen height for whole-frame scanline statistics.
pub const MAX_SCREEN_HEIGHT: u32 = 4096;

/// Largest accepted number of hardware sprites in one metasprite frame.
pub const MAX_PARTS: usize = 16;

/// Largest accepted number of frames in one animation.
pub const MAX_FRAMES: usize = 64;

/// Largest accepted per-frame duration in ticks (smallest is 1).
pub const MAX_FRAME_TICKS: u16 = u16::MAX;

/// Largest accepted tick delta for one [`Animator::advance`] call.
pub const MAX_TICK_DELTA: u32 = 65_535;

/// The 8.8 fixed-point representation of `1.0` in [`AffineParams`].
pub const AFFINE_ONE: i16 = 256;

/// An RGB colour `[red, green, blue]`.
pub type Rgb8 = [u8; 3];

/// Visible sprite count per priority level (index = priority).
pub type PriorityCounts = [usize; 4];

/// Fixed storage for the parts of one metasprite frame.
pub type MetaParts = [OamEntry; MAX_PARTS];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes of every fallible operation in this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OamError {
    /// An OAM slot index is `>= OAM_SLOTS` (or the slot range runs past it).
    SlotOutOfRange {
        /// The offending slot index.
        slot: usize,
    },
    /// An affine slot index is `>= AFFINE_SLOTS`.
    AffineSlotOutOfRange {
        /// The offending affine slot index.
        slot: usize,
    },
    /// Priority is above [`MAX_PRIORITY`].
    InvalidPriority {
        /// The offending priority.
        value: u8,
    },
    /// Palette bank is above [`MAX_PALETTE_BANK`].
    InvalidPaletteBank {
        /// The offending bank.
        value: u8,
    },
    /// Tile index is above [`MAX_TILE_INDEX`].
    InvalidTile {
        /// The offending tile index.
        value: u16,
    },
    /// A coordinate is outside [`COORD_MIN`]..=[`COORD_MAX`].
    CoordinateOutOfRange {
        /// The offending coordinate.
        value: i32,
    },
    /// `double_size` was set without `affine`.
    DoubleSizeWithoutAffine,
    /// Scanline limit is zero or above [`OAM_SLOTS`].
    InvalidScanlineLimit {
        /// The offending limit.
        value: usize,
    },
    /// Screen height is zero or above [`MAX_SCREEN_HEIGHT`].
    InvalidScreenHeight {
        /// The offending height.
        value: u32,
    },
    /// A caller buffer is shorter than the operation needs.
    BufferTooSmall {
        /// Elements needed.
        required: usize,
        /// Elements supplied.
        actual: usize,
    },
    /// A frame list is empty or longer than [`MAX_FRAMES`].
    InvalidFrameCount {
        /// The offending count.
        count: usize,
    },
    /// A frame duration is zero.
    InvalidFrameDuration {
        /// Index of the offending frame.
        frame: usize,
    },
    /// A frame has no parts or more than [`MAX_PARTS`].
    InvalidPartCount {
        /// The offending part count.
        count: usize,
    },
    /// A tick delta is above [`MAX_TICK_DELTA`].
    TickTooLarge {
        /// The offending delta.
        ticks: u32,
    },
    /// The animator state does not fit the animation it was advanced with.
    AnimatorMismatch,
    /// An affine matrix cannot be represented in 8.8 fixed point.
    AffineNotRepresentable,
    /// The palette has no entry for the requested bank / colour index.
    PaletteEntryUnavailable {
        /// The requested bank.
        bank: u8,
        /// The requested colour index within the bank (0..16).
        index: u8,
    },
    /// Scene width or height is not finite and positive.
    InvalidSceneSize,
    /// `paint-batch-tiles` rejected the lowered batch.
    LoweringRejected,
}

impl std::fmt::Display for OamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SlotOutOfRange { slot } => {
                write!(f, "OAM slot {slot} is outside 0..{OAM_SLOTS}")
            }
            Self::AffineSlotOutOfRange { slot } => {
                write!(f, "affine slot {slot} is outside 0..{AFFINE_SLOTS}")
            }
            Self::InvalidPriority { value } => {
                write!(f, "priority {value} is above {MAX_PRIORITY}")
            }
            Self::InvalidPaletteBank { value } => {
                write!(f, "palette bank {value} is above {MAX_PALETTE_BANK}")
            }
            Self::InvalidTile { value } => {
                write!(f, "tile index {value} is above {MAX_TILE_INDEX}")
            }
            Self::CoordinateOutOfRange { value } => {
                write!(f, "coordinate {value} is outside {COORD_MIN}..={COORD_MAX}")
            }
            Self::DoubleSizeWithoutAffine => write!(f, "double_size requires affine"),
            Self::InvalidScanlineLimit { value } => {
                write!(f, "scanline limit {value} is outside 1..={OAM_SLOTS}")
            }
            Self::InvalidScreenHeight { value } => {
                write!(
                    f,
                    "screen height {value} is outside 1..={MAX_SCREEN_HEIGHT}"
                )
            }
            Self::BufferTooSmall { required, actual } => {
                write!(
                    f,
                    "buffer holds {actual} elements but {required} are needed"
                )
            }
            Self::InvalidFrameCount { count } => {
                write!(f, "frame count {count} is outside 1..={MAX_FRAMES}")
            }
            Self::InvalidFrameDuration { frame } => {
                write!(f, "frame {frame} has a zero duration")
            }
            Self::InvalidPartCount { count } => {
                write!(f, "part count {count} is outside 1..={MAX_PARTS}")
            }
            Self::TickTooLarge { ticks } => {
                write!(f, "tick delta {ticks} is above {MAX_TICK_DELTA}")
            }
            Self::AnimatorMismatch => write!(f, "animator state does not match the animation"),
            Self::AffineNotRepresentable => {
                write!(f, "affine matrix is not representable in 8.8 fixed point")
            }
            Self::PaletteEntryUnavailable { bank, index } => {
                write!(f, "palette has no entry for bank {bank} index {index}")
            }
            Self::InvalidSceneSize => write!(f, "scene size must be finite and positive"),
            Self::LoweringRejected => write!(f, "paint-batch-tiles rejected the sprite batch"),
        }
    }
}

impl std::error::Error for OamError {}

// ---------------------------------------------------------------------------
// Shapes and sizes
// ---------------------------------------------------------------------------

/// Hardware sprite aspect class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteShape {
    /// Square sprites: 8x8, 16x16, 32x32, 64x64.
    Square,
    /// Wide sprites: 16x8, 32x8, 32x16, 64x32.
    Wide,
    /// Tall sprites: 8x16, 8x32, 16x32, 32x64.
    Tall,
}

/// Hardware sprite size selector (combined with [`SpriteShape`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteSize {
    /// Smallest size class.
    Size0,
    /// Second size class.
    Size1,
    /// Third size class.
    Size2,
    /// Largest size class.
    Size3,
}

impl SpriteShape {
    /// Width and height in pixels of this shape at `size`.
    pub const fn dimensions(self, size: SpriteSize) -> (u32, u32) {
        let n = match size {
            SpriteSize::Size0 => 0,
            SpriteSize::Size1 => 1,
            SpriteSize::Size2 => 2,
            SpriteSize::Size3 => 3,
        };
        match self {
            Self::Square => {
                let s = 8u32 << n;
                (s, s)
            }
            Self::Wide => match n {
                0 => (16, 8),
                1 => (32, 8),
                2 => (32, 16),
                _ => (64, 32),
            },
            Self::Tall => match n {
                0 => (8, 16),
                1 => (8, 32),
                2 => (16, 32),
                _ => (32, 64),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Affine parameters
// ---------------------------------------------------------------------------

/// One affine matrix group in 8.8 fixed point (GBA `PA`, `PB`, `PC`, `PD`).
///
/// The matrix maps a *screen* offset from the sprite centre to a *texture*
/// offset from the texture centre:
/// `tx = (pa*dx + pb*dy) / 256`, `ty = (pc*dx + pd*dy) / 256`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AffineParams {
    /// X contribution of screen X (256 = 1.0).
    pub pa: i16,
    /// X contribution of screen Y.
    pub pb: i16,
    /// Y contribution of screen X.
    pub pc: i16,
    /// Y contribution of screen Y (256 = 1.0).
    pub pd: i16,
}

impl AffineParams {
    /// The identity matrix.
    pub const fn identity_params() -> Self {
        Self {
            pa: AFFINE_ONE,
            pb: 0,
            pc: 0,
            pd: AFFINE_ONE,
        }
    }

    /// Converts to an [`Affine2D`] with the same screen-to-texture mapping
    /// (`a = pa`, `b = pc`, `c = pb`, `d = pd`, no translation).
    pub fn to_matrix(&self) -> Affine2D {
        let k = f64::from(AFFINE_ONE);
        Affine2D::new(
            f64::from(self.pa) / k,
            f64::from(self.pc) / k,
            f64::from(self.pb) / k,
            f64::from(self.pd) / k,
            0.0,
            0.0,
        )
    }

    /// Quantizes the linear part of `m` to 8.8 fixed point (round to nearest,
    /// ties away from zero). The translation part is ignored.
    ///
    /// # Errors
    ///
    /// [`OamError::AffineNotRepresentable`] if any coefficient is not finite
    /// or its scaled value does not fit an `i16`.
    pub fn from_matrix(m: &Affine2D) -> Result<Self, OamError> {
        Ok(Self {
            pa: quantize_fixed(m.a)?,
            pb: quantize_fixed(m.c)?,
            pc: quantize_fixed(m.b)?,
            pd: quantize_fixed(m.d)?,
        })
    }

    /// Maps a screen offset from the sprite centre to a texture offset from
    /// the texture centre, in whole pixels (arithmetic shift, floor).
    pub const fn map_offset(&self, dx: i32, dy: i32) -> (i32, i32) {
        let pa = self.pa as i32;
        let pb = self.pb as i32;
        let pc = self.pc as i32;
        let pd = self.pd as i32;
        (
            (pa.wrapping_mul(dx).wrapping_add(pb.wrapping_mul(dy))) >> 8,
            (pc.wrapping_mul(dx).wrapping_add(pd.wrapping_mul(dy))) >> 8,
        )
    }
}

fn quantize_fixed(v: f64) -> Result<i16, OamError> {
    if !v.is_finite() {
        return Err(OamError::AffineNotRepresentable);
    }
    let scaled = (v * f64::from(AFFINE_ONE)).round();
    if scaled < f64::from(i16::MIN) || scaled > f64::from(i16::MAX) {
        return Err(OamError::AffineNotRepresentable);
    }
    Ok(scaled as i16)
}

// ---------------------------------------------------------------------------
// OAM entry
// ---------------------------------------------------------------------------

/// One hardware sprite slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OamEntry {
    /// Left edge of the display box in screen pixels.
    pub x: i16,
    /// Top edge of the display box in screen pixels.
    pub y: i16,
    /// Shape class.
    pub shape: SpriteShape,
    /// Size class.
    pub size: SpriteSize,
    /// Horizontal mirror (ignored when `affine` is set).
    pub h_flip: bool,
    /// Vertical mirror (ignored when `affine` is set).
    pub v_flip: bool,
    /// Palette bank, `0..=`[`MAX_PALETTE_BANK`].
    pub palette_bank: u8,
    /// Priority, `0..=`[`MAX_PRIORITY`]; lower is in front.
    pub priority: u8,
    /// Affine matrix slot, `0..`[`AFFINE_SLOTS`] (used when `affine` is set).
    pub affine_slot: u8,
    /// Tile index, `0..=`[`MAX_TILE_INDEX`].
    pub tile: u16,
    /// Hidden sprites are skipped by every evaluation.
    pub hidden: bool,
    /// Render through the affine matrix in `affine_slot`.
    pub affine: bool,
    /// With `affine`, double the display box so rotation is not clipped.
    pub double_size: bool,
}

impl OamEntry {
    /// A hidden 8x8 sprite at the origin with every attribute zeroed.
    pub const fn blank_entry() -> Self {
        Self {
            x: 0,
            y: 0,
            shape: SpriteShape::Square,
            size: SpriteSize::Size0,
            h_flip: false,
            v_flip: false,
            palette_bank: 0,
            priority: 0,
            affine_slot: 0,
            tile: 0,
            hidden: true,
            affine: false,
            double_size: false,
        }
    }

    /// Validates every attribute against its hardware range.
    ///
    /// # Errors
    ///
    /// The matching [`OamError`] variant for the first invalid attribute.
    pub fn check_entry(&self) -> Result<(), OamError> {
        check_coord(i32::from(self.x))?;
        check_coord(i32::from(self.y))?;
        if self.priority > MAX_PRIORITY {
            return Err(OamError::InvalidPriority {
                value: self.priority,
            });
        }
        if self.palette_bank > MAX_PALETTE_BANK {
            return Err(OamError::InvalidPaletteBank {
                value: self.palette_bank,
            });
        }
        if self.tile > MAX_TILE_INDEX {
            return Err(OamError::InvalidTile { value: self.tile });
        }
        if usize::from(self.affine_slot) >= AFFINE_SLOTS {
            return Err(OamError::AffineSlotOutOfRange {
                slot: usize::from(self.affine_slot),
            });
        }
        if self.double_size && !self.affine {
            return Err(OamError::DoubleSizeWithoutAffine);
        }
        Ok(())
    }

    /// Size in pixels of the on-screen display box (doubled for
    /// `affine && double_size`).
    pub const fn display_size(&self) -> (u32, u32) {
        let (w, h) = self.shape.dimensions(self.size);
        if self.affine && self.double_size {
            (w * 2, h * 2)
        } else {
            (w, h)
        }
    }

    /// `true` if the entry is not hidden.
    pub const fn is_visible(&self) -> bool {
        !self.hidden
    }

    /// `true` if the display box intersects scanline `y`.
    pub const fn covers_scanline(&self, y: i32) -> bool {
        let (_, h) = self.display_size();
        let top = self.y as i32;
        y >= top && y < top + h as i32
    }
}

fn check_coord(v: i32) -> Result<(), OamError> {
    if v < i32::from(COORD_MIN) || v > i32::from(COORD_MAX) {
        return Err(OamError::CoordinateOutOfRange { value: v });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// OAM table
// ---------------------------------------------------------------------------

/// Result of evaluating one scanline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanlineReport {
    /// Sprites kept for this line (written to the caller buffer).
    pub selected: usize,
    /// Sprites dropped because the per-line limit was reached.
    pub dropped: usize,
}

/// Whole-frame scanline pressure statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanlineStats {
    /// Largest number of sprites covering any single line.
    pub max_per_line: usize,
    /// Number of lines on which at least one sprite was dropped.
    pub overflow_lines: u32,
    /// Total sprites dropped across all lines.
    pub dropped_total: u64,
}

/// The 128-slot OAM plus 32 affine matrix groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OamTable {
    entries: [OamEntry; OAM_SLOTS],
    affine: [AffineParams; AFFINE_SLOTS],
}

impl OamTable {
    /// A table where every slot is hidden and every matrix is the identity.
    pub const fn blank_table() -> Self {
        Self {
            entries: [OamEntry::blank_entry(); OAM_SLOTS],
            affine: [AffineParams::identity_params(); AFFINE_SLOTS],
        }
    }

    /// Resets every slot to hidden and every matrix to the identity.
    pub fn clear_all(&mut self) {
        *self = Self::blank_table();
    }

    /// Read-only view of all slots.
    pub fn entries(&self) -> &[OamEntry] {
        &self.entries
    }

    /// Reads slot `slot`.
    ///
    /// # Errors
    ///
    /// [`OamError::SlotOutOfRange`] if `slot >= OAM_SLOTS`.
    pub fn entry(&self, slot: usize) -> Result<&OamEntry, OamError> {
        self.entries
            .get(slot)
            .ok_or(OamError::SlotOutOfRange { slot })
    }

    /// Validates and writes slot `slot`.
    ///
    /// # Errors
    ///
    /// [`OamError::SlotOutOfRange`] or any [`OamEntry::check_entry`] error;
    /// the table is unchanged on error.
    pub fn set_entry(&mut self, slot: usize, entry: OamEntry) -> Result<(), OamError> {
        entry.check_entry()?;
        let dst = self
            .entries
            .get_mut(slot)
            .ok_or(OamError::SlotOutOfRange { slot })?;
        *dst = entry;
        Ok(())
    }

    /// Hides slot `slot`, leaving its other attributes intact.
    ///
    /// # Errors
    ///
    /// [`OamError::SlotOutOfRange`] if `slot >= OAM_SLOTS`.
    pub fn hide_slot(&mut self, slot: usize) -> Result<(), OamError> {
        let dst = self
            .entries
            .get_mut(slot)
            .ok_or(OamError::SlotOutOfRange { slot })?;
        dst.hidden = true;
        Ok(())
    }

    /// Writes affine matrix group `slot`.
    ///
    /// # Errors
    ///
    /// [`OamError::AffineSlotOutOfRange`] if `slot >= AFFINE_SLOTS`.
    pub fn set_affine(&mut self, slot: usize, params: AffineParams) -> Result<(), OamError> {
        let dst = self
            .affine
            .get_mut(slot)
            .ok_or(OamError::AffineSlotOutOfRange { slot })?;
        *dst = params;
        Ok(())
    }

    /// Reads affine matrix group `slot`.
    ///
    /// # Errors
    ///
    /// [`OamError::AffineSlotOutOfRange`] if `slot >= AFFINE_SLOTS`.
    pub fn affine_params(&self, slot: usize) -> Result<AffineParams, OamError> {
        self.affine
            .get(slot)
            .copied()
            .ok_or(OamError::AffineSlotOutOfRange { slot })
    }

    /// Number of non-hidden slots.
    pub fn visible_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_visible()).count()
    }

    /// Visible sprite count per priority level (index = priority).
    pub fn priority_counts(&self) -> PriorityCounts {
        let mut counts = [0usize; 4];
        for e in self.entries.iter().filter(|e| e.is_visible()) {
            if let Some(c) = counts.get_mut(usize::from(e.priority)) {
                *c += 1;
            }
        }
        counts
    }

    /// Writes the visible slots into `out` in **back-to-front** paint order
    /// (priority 3 first; within a priority, higher slot first) and returns
    /// how many were written. Deterministic and allocation free.
    ///
    /// # Errors
    ///
    /// [`OamError::BufferTooSmall`] if `out` cannot hold every visible slot
    /// (nothing is written).
    pub fn draw_order_into(&self, out: &mut [u8]) -> Result<usize, OamError> {
        let needed = self.visible_count();
        if out.len() < needed {
            return Err(OamError::BufferTooSmall {
                required: needed,
                actual: out.len(),
            });
        }
        let mut n = 0usize;
        for prio in (0..=MAX_PRIORITY).rev() {
            for (slot, e) in self.entries.iter().enumerate().rev() {
                if e.is_visible() && e.priority == prio {
                    if let Some(dst) = out.get_mut(n) {
                        *dst = slot as u8;
                        n += 1;
                    }
                }
            }
        }
        Ok(n)
    }

    /// Evaluates scanline `y` the way the hardware does: ascending OAM order,
    /// keeping at most `limit` sprites. Kept slots are written to `out`.
    ///
    /// # Errors
    ///
    /// [`OamError::InvalidScanlineLimit`] if `limit` is `0` or above
    /// [`OAM_SLOTS`]; [`OamError::BufferTooSmall`] if `out.len() < limit`.
    pub fn scanline_sprites_into(
        &self,
        y: i32,
        limit: usize,
        out: &mut [u8],
    ) -> Result<ScanlineReport, OamError> {
        check_limit(limit)?;
        if out.len() < limit {
            return Err(OamError::BufferTooSmall {
                required: limit,
                actual: out.len(),
            });
        }
        let mut selected = 0usize;
        let mut dropped = 0usize;
        for (slot, e) in self.entries.iter().enumerate() {
            if e.is_visible() && e.covers_scanline(y) {
                if selected < limit {
                    if let Some(dst) = out.get_mut(selected) {
                        *dst = slot as u8;
                    }
                    selected += 1;
                } else {
                    dropped += 1;
                }
            }
        }
        Ok(ScanlineReport { selected, dropped })
    }

    /// Evaluates every line `0..screen_height` against `limit` and reports
    /// sprite pressure. Work is `O(screen_height * OAM_SLOTS)`, bounded by
    /// [`MAX_SCREEN_HEIGHT`].
    ///
    /// # Errors
    ///
    /// [`OamError::InvalidScreenHeight`] or [`OamError::InvalidScanlineLimit`].
    pub fn scanline_stats(
        &self,
        screen_height: u32,
        limit: usize,
    ) -> Result<ScanlineStats, OamError> {
        if screen_height == 0 || screen_height > MAX_SCREEN_HEIGHT {
            return Err(OamError::InvalidScreenHeight {
                value: screen_height,
            });
        }
        check_limit(limit)?;
        let mut stats = ScanlineStats {
            max_per_line: 0,
            overflow_lines: 0,
            dropped_total: 0,
        };
        for line in 0..screen_height {
            let y = line as i32;
            let covering = self
                .entries
                .iter()
                .filter(|e| e.is_visible() && e.covers_scanline(y))
                .count();
            stats.max_per_line = stats.max_per_line.max(covering);
            if covering > limit {
                stats.overflow_lines += 1;
                stats.dropped_total += (covering - limit) as u64;
            }
        }
        Ok(stats)
    }

    /// Texel `(tx, ty)` of slot `slot`'s texture that shows at screen pixel
    /// `(sx, sy)`, or `None` if the pixel is outside the display box or (for
    /// affine sprites) maps outside the texture.
    ///
    /// Non-affine sprites honour `h_flip` / `v_flip`; affine sprites sample
    /// through their matrix about the box and texture centres.
    ///
    /// # Errors
    ///
    /// [`OamError::SlotOutOfRange`] if `slot >= OAM_SLOTS`.
    pub fn texel_at(&self, slot: usize, sx: i32, sy: i32) -> Result<Option<(u32, u32)>, OamError> {
        let e = self.entry(slot)?;
        let (w, h) = e.shape.dimensions(e.size);
        let (dw, dh) = e.display_size();
        // i64 so hostile i32::MIN / i32::MAX coordinates cannot overflow (CWE-190).
        let dx64 = i64::from(sx) - i64::from(e.x);
        let dy64 = i64::from(sy) - i64::from(e.y);
        if dx64 < 0 || dy64 < 0 || dx64 >= i64::from(dw) || dy64 >= i64::from(dh) {
            return Ok(None);
        }
        // In range 0..128 after the check above, so the narrowing is lossless.
        let (dx, dy) = (dx64 as i32, dy64 as i32);
        let (wi, hi) = (w as i32, h as i32);
        let (tx, ty) = if e.affine {
            let m = self.affine_params(usize::from(e.affine_slot))?;
            let (ox, oy) = m.map_offset(dx - (dw as i32) / 2, dy - (dh as i32) / 2);
            (ox + wi / 2, oy + hi / 2)
        } else {
            (
                if e.h_flip { wi - 1 - dx } else { dx },
                if e.v_flip { hi - 1 - dy } else { dy },
            )
        };
        if tx < 0 || ty < 0 || tx >= wi || ty >= hi {
            return Ok(None);
        }
        Ok(Some((tx as u32, ty as u32)))
    }

    /// Stamps the parts of `frame` into consecutive slots starting at
    /// `first_slot`, translated by `(origin_x, origin_y)` and optionally
    /// mirrored about the origin. Returns the number of slots written.
    ///
    /// Mirroring reflects each part's position and toggles its flip flag
    /// (affine parts are repositioned but keep their matrix). The operation
    /// is atomic: nothing is written if any part fails validation.
    ///
    /// # Errors
    ///
    /// Frame validation errors, [`OamError::SlotOutOfRange`] if the slot
    /// range leaves OAM, or [`OamError::CoordinateOutOfRange`].
    pub fn stamp_frame(
        &mut self,
        first_slot: usize,
        frame: &MetaFrame,
        origin_x: i32,
        origin_y: i32,
        mirror_h: bool,
        mirror_v: bool,
    ) -> Result<usize, OamError> {
        frame.check_frame()?;
        let count = usize::from(frame.part_count);
        let end = first_slot
            .checked_add(count)
            .ok_or(OamError::SlotOutOfRange { slot: first_slot })?;
        if end > OAM_SLOTS {
            return Err(OamError::SlotOutOfRange {
                slot: end.saturating_sub(1),
            });
        }
        let parts = frame.parts.get(..count).unwrap_or(&[]);
        for part in parts {
            place_part(part, origin_x, origin_y, mirror_h, mirror_v)?;
        }
        for (i, part) in parts.iter().enumerate() {
            let placed = place_part(part, origin_x, origin_y, mirror_h, mirror_v)?;
            if let Some(dst) = self.entries.get_mut(first_slot + i) {
                *dst = placed;
            }
        }
        Ok(count)
    }

    /// Lowers visible sprites to [`SpriteQuad`]s in `out` (ascending slot
    /// order) and returns how many were written. Zero heap allocation.
    ///
    /// Mapping: `id = 127 - slot`, `z_priority = 3 - priority`, so that
    /// `paint-batch-tiles`' ascending `(z_priority, id)` paint order equals
    /// hardware priority/OAM order (see [`slot_from_quad_id`]). The tint is
    /// `palette` entry `palette_bank * 16 + tint_index`, or white when no
    /// palette is given.
    ///
    /// # Errors
    ///
    /// [`OamError::BufferTooSmall`] if `out` cannot hold every visible slot;
    /// [`OamError::PaletteEntryUnavailable`] if a tint cannot be resolved.
    pub fn lower_to_quads_into(
        &self,
        palette: Option<&Palette>,
        tint_index: u8,
        out: &mut [SpriteQuad],
    ) -> Result<usize, OamError> {
        let needed = self.visible_count();
        if out.len() < needed {
            return Err(OamError::BufferTooSmall {
                required: needed,
                actual: out.len(),
            });
        }
        let mut n = 0usize;
        for (slot, e) in self.entries.iter().enumerate() {
            if !e.is_visible() {
                continue;
            }
            let (w, h) = e.display_size();
            let color = match palette {
                Some(p) => bank_tint(p, e.palette_bank, tint_index)?,
                None => [255, 255, 255],
            };
            let quad = SpriteQuad::new(
                quad_id_for_slot(slot),
                f64::from(e.x),
                f64::from(e.y),
                f64::from(w),
                f64::from(h),
            )
            .with_color(color)
            .with_z_priority(MAX_PRIORITY - e.priority)
            .with_flip(e.h_flip && !e.affine, e.v_flip && !e.affine);
            if let Some(dst) = out.get_mut(n) {
                *dst = quad;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Lowers the table to a `paint-instructions` scene through
    /// `paint-batch-tiles`. Allocates (the scene is heap backed); every other
    /// path in this crate does not.
    ///
    /// # Errors
    ///
    /// [`OamError::InvalidSceneSize`], palette resolution errors, or
    /// [`OamError::LoweringRejected`].
    pub fn lower_to_paint_scene(
        &self,
        width: f64,
        height: f64,
        background: &str,
        palette: Option<&Palette>,
        tint_index: u8,
    ) -> Result<PaintScene, OamError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(OamError::InvalidSceneSize);
        }
        let mut quads = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); OAM_SLOTS];
        let n = self.lower_to_quads_into(palette, tint_index, &mut quads)?;
        let used = quads.get(..n).unwrap_or(&[]);
        paint_batch_tiles::lower_sprites_to_paint_scene(width, height, background, used)
            .map_err(|_| OamError::LoweringRejected)
    }
}

fn check_limit(limit: usize) -> Result<(), OamError> {
    if limit == 0 || limit > OAM_SLOTS {
        return Err(OamError::InvalidScanlineLimit { value: limit });
    }
    Ok(())
}

fn place_part(
    part: &OamEntry,
    origin_x: i32,
    origin_y: i32,
    mirror_h: bool,
    mirror_v: bool,
) -> Result<OamEntry, OamError> {
    part.check_entry()?;
    let (pw, ph) = part.display_size();
    let px = i32::from(part.x);
    let py = i32::from(part.y);
    let rel_x = if mirror_h { -(px + pw as i32) } else { px };
    let rel_y = if mirror_v { -(py + ph as i32) } else { py };
    let x = origin_x
        .checked_add(rel_x)
        .ok_or(OamError::CoordinateOutOfRange { value: origin_x })?;
    let y = origin_y
        .checked_add(rel_y)
        .ok_or(OamError::CoordinateOutOfRange { value: origin_y })?;
    check_coord(x)?;
    check_coord(y)?;
    let mut placed = *part;
    placed.x = x as i16;
    placed.y = y as i16;
    if !part.affine {
        placed.h_flip ^= mirror_h;
        placed.v_flip ^= mirror_v;
    }
    Ok(placed)
}

/// The quad id used for OAM slot `slot` by [`OamTable::lower_to_quads_into`]
/// (`127 - slot`, so lower slots paint later, i.e. in front).
pub const fn quad_id_for_slot(slot: usize) -> u32 {
    (OAM_SLOTS - 1 - (slot % OAM_SLOTS)) as u32
}

/// Inverse of [`quad_id_for_slot`]; `None` if `id >= OAM_SLOTS`.
pub const fn slot_from_quad_id(id: u32) -> Option<usize> {
    if id < OAM_SLOTS as u32 {
        Some(OAM_SLOTS - 1 - id as usize)
    } else {
        None
    }
}

/// Resolves the representative colour of palette bank `bank` (16 colours per
/// bank, entry `bank * 16 + tint_index`) from a `palette-indexed` palette.
///
/// # Errors
///
/// [`OamError::InvalidPaletteBank`] if `bank > 15`,
/// [`OamError::PaletteEntryUnavailable`] if `tint_index >= 16` or the palette
/// is too small for the entry.
pub fn bank_tint(palette: &Palette, bank: u8, tint_index: u8) -> Result<Rgb8, OamError> {
    if bank > MAX_PALETTE_BANK {
        return Err(OamError::InvalidPaletteBank { value: bank });
    }
    let unavailable = OamError::PaletteEntryUnavailable {
        bank,
        index: tint_index,
    };
    if tint_index >= 16 {
        return Err(unavailable);
    }
    let idx = bank * 16 + tint_index;
    let rgba = palette.entry(idx).map_err(|_| unavailable)?;
    Ok([rgba[0], rgba[1], rgba[2]])
}

// ---------------------------------------------------------------------------
// Metasprites and animation
// ---------------------------------------------------------------------------

/// One animation frame: a group of hardware sprites shown for `duration`
/// ticks. Part positions are offsets from the metasprite origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetaFrame {
    /// Part storage; only the first `part_count` entries are used.
    pub parts: MetaParts,
    /// Number of used parts, `1..=`[`MAX_PARTS`].
    pub part_count: u8,
    /// Display time in ticks, `1..=`[`MAX_FRAME_TICKS`].
    pub duration: u16,
}

impl MetaFrame {
    /// An invalid placeholder frame (no parts, zero duration) for array
    /// initialisation; it fails [`MetaFrame::check_frame`] until filled.
    pub const fn blank_frame() -> Self {
        Self {
            parts: [OamEntry::blank_entry(); MAX_PARTS],
            part_count: 0,
            duration: 0,
        }
    }

    /// Builds a frame from `parts` (copied) shown for `duration` ticks.
    ///
    /// # Errors
    ///
    /// [`OamError::InvalidPartCount`] for zero or more than [`MAX_PARTS`]
    /// parts, plus every [`MetaFrame::check_frame`] error.
    pub fn frame_from_parts(parts: &[OamEntry], duration: u16) -> Result<Self, OamError> {
        if parts.is_empty() || parts.len() > MAX_PARTS {
            return Err(OamError::InvalidPartCount { count: parts.len() });
        }
        let mut frame = Self::blank_frame();
        for (dst, src) in frame.parts.iter_mut().zip(parts) {
            *dst = *src;
        }
        frame.part_count = parts.len() as u8;
        frame.duration = duration;
        frame.check_frame()?;
        Ok(frame)
    }

    /// Validates part count, duration and every used part.
    ///
    /// # Errors
    ///
    /// [`OamError::InvalidPartCount`], [`OamError::InvalidFrameDuration`] (with
    /// `frame: 0`) or any [`OamEntry::check_entry`] error.
    pub fn check_frame(&self) -> Result<(), OamError> {
        let count = usize::from(self.part_count);
        if count == 0 || count > MAX_PARTS {
            return Err(OamError::InvalidPartCount { count });
        }
        if self.duration == 0 {
            return Err(OamError::InvalidFrameDuration { frame: 0 });
        }
        for part in self.parts.iter().take(count) {
            part.check_entry()?;
        }
        Ok(())
    }
}

/// How an [`Animation`] behaves at its last frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayMode {
    /// Play once and hold the last frame; the animator reports finished.
    OneShot,
    /// Wrap from the last frame back to the first.
    Loop,
    /// Bounce: 0,1,..,n-1,n-2,..,1,0,1,.. (endpoints are not repeated).
    PingPong,
}

/// A validated, borrowed list of frames plus a [`PlayMode`].
#[derive(Clone, Copy, Debug)]
pub struct Animation<'a> {
    frame_list: &'a [MetaFrame],
    play_mode: PlayMode,
}

impl<'a> Animation<'a> {
    /// Validates `frames` (1..=[`MAX_FRAMES`], each passing
    /// [`MetaFrame::check_frame`]) and binds them to `mode`.
    ///
    /// # Errors
    ///
    /// [`OamError::InvalidFrameCount`] or the first frame validation error
    /// (duration errors carry the offending frame index).
    pub fn bind(frames: &'a [MetaFrame], mode: PlayMode) -> Result<Self, OamError> {
        if frames.is_empty() || frames.len() > MAX_FRAMES {
            return Err(OamError::InvalidFrameCount {
                count: frames.len(),
            });
        }
        for (i, f) in frames.iter().enumerate() {
            f.check_frame().map_err(|e| match e {
                OamError::InvalidFrameDuration { .. } => {
                    OamError::InvalidFrameDuration { frame: i }
                }
                other => other,
            })?;
        }
        Ok(Self {
            frame_list: frames,
            play_mode: mode,
        })
    }

    /// The bound frames.
    pub const fn frames(&self) -> &'a [MetaFrame] {
        self.frame_list
    }

    /// The bound play mode.
    pub const fn mode(&self) -> PlayMode {
        self.play_mode
    }

    /// Sum of all frame durations in ticks.
    pub fn total_ticks(&self) -> u32 {
        self.frame_list.iter().map(|f| u32::from(f.duration)).sum()
    }
}

/// What happened during one [`Animator::advance`] call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimStep {
    /// Number of frame transitions.
    pub frames_advanced: u32,
    /// Number of full loop / ping-pong cycles completed.
    pub loops_completed: u32,
    /// `true` if a one-shot animation is (now) finished.
    pub finished: bool,
}

/// Deterministic animation playback state; holds no reference to the
/// animation, so it is `Copy` and trivially snapshot-able for rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Animator {
    frame: u16,
    elapsed: u32,
    reverse: bool,
    finished: bool,
    loops: u32,
}

impl Animator {
    /// A fresh animator at frame 0, tick 0.
    pub const fn start() -> Self {
        Self {
            frame: 0,
            elapsed: 0,
            reverse: false,
            finished: false,
            loops: 0,
        }
    }

    /// Rewinds to frame 0, tick 0.
    pub fn rewind(&mut self) {
        *self = Self::start();
    }

    /// Current frame index.
    pub const fn frame_index(&self) -> usize {
        self.frame as usize
    }

    /// Ticks spent in the current frame.
    pub const fn elapsed_ticks(&self) -> u32 {
        self.elapsed
    }

    /// `true` once a one-shot animation has played to its end.
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    /// Completed loop / ping-pong cycles (saturating).
    pub const fn loops_completed(&self) -> u32 {
        self.loops
    }

    /// The current frame of `anim`.
    ///
    /// # Errors
    ///
    /// [`OamError::AnimatorMismatch`] if the animator is past the end of
    /// `anim` (e.g. it was driven by a different animation).
    pub fn current_frame<'a>(&self, anim: &Animation<'a>) -> Result<&'a MetaFrame, OamError> {
        anim.frames()
            .get(self.frame_index())
            .ok_or(OamError::AnimatorMismatch)
    }

    /// Advances playback by `ticks`. Each loop iteration consumes at least
    /// one tick, so work is bounded by [`MAX_TICK_DELTA`].
    ///
    /// # Errors
    ///
    /// [`OamError::TickTooLarge`] if `ticks > MAX_TICK_DELTA`;
    /// [`OamError::AnimatorMismatch`] if the state does not fit `anim`. The
    /// animator is unchanged on error.
    pub fn advance(&mut self, anim: &Animation<'_>, ticks: u32) -> Result<AnimStep, OamError> {
        if ticks > MAX_TICK_DELTA {
            return Err(OamError::TickTooLarge { ticks });
        }
        let count = anim.frames().len();
        let current = self.current_frame(anim)?;
        if self.elapsed > u32::from(current.duration) {
            return Err(OamError::AnimatorMismatch);
        }
        let mut step = AnimStep {
            frames_advanced: 0,
            loops_completed: 0,
            finished: self.finished,
        };
        let mut remaining = ticks;
        while remaining > 0 && !self.finished {
            let duration = u32::from(
                anim.frames()
                    .get(self.frame_index())
                    .ok_or(OamError::AnimatorMismatch)?
                    .duration,
            );
            let left = duration.saturating_sub(self.elapsed);
            if remaining < left {
                self.elapsed += remaining;
                break;
            }
            remaining -= left;
            match next_position(anim.mode(), count, self.frame_index(), self.reverse) {
                Next::Frame { index, reverse } => {
                    self.frame = index as u16;
                    self.reverse = reverse;
                    self.elapsed = 0;
                    step.frames_advanced += 1;
                }
                Next::Cycled { index, reverse } => {
                    self.frame = index as u16;
                    self.reverse = reverse;
                    self.elapsed = 0;
                    self.loops = self.loops.saturating_add(1);
                    step.frames_advanced += 1;
                    step.loops_completed += 1;
                }
                Next::Done => {
                    self.elapsed = duration;
                    self.finished = true;
                }
            }
        }
        step.finished = self.finished;
        Ok(step)
    }
}

enum Next {
    Frame { index: usize, reverse: bool },
    Cycled { index: usize, reverse: bool },
    Done,
}

fn next_position(mode: PlayMode, count: usize, frame: usize, reverse: bool) -> Next {
    let last = count.saturating_sub(1);
    match mode {
        PlayMode::OneShot => {
            if frame < last {
                Next::Frame {
                    index: frame + 1,
                    reverse: false,
                }
            } else {
                Next::Done
            }
        }
        PlayMode::Loop => {
            if frame < last {
                Next::Frame {
                    index: frame + 1,
                    reverse: false,
                }
            } else {
                Next::Cycled {
                    index: 0,
                    reverse: false,
                }
            }
        }
        PlayMode::PingPong => {
            if count <= 1 {
                Next::Cycled {
                    index: 0,
                    reverse: false,
                }
            } else if !reverse {
                if frame < last {
                    Next::Frame {
                        index: frame + 1,
                        reverse: false,
                    }
                } else {
                    Next::Frame {
                        index: frame - 1,
                        reverse: true,
                    }
                }
            } else if frame > 0 {
                Next::Frame {
                    index: frame - 1,
                    reverse: true,
                }
            } else {
                Next::Cycled {
                    index: 1,
                    reverse: false,
                }
            }
        }
    }
}
