//! # sim-crt-vector-beam
//!
//! Dual CRT beam simulator: cycle-accurate raster beam deflection with phosphor persistence
//! decay and oscilloscope / vector beam display stroke synthesis (GAME15b, GAME00 §3.3).
//!
//! Provides:
//! - **Raster CRT Beam Deflection (`CrtRasterBeam`)**: Configurable NTSC and arcade timings,
//!   HBlank/VBlank interval flags, P4/P31/P1 phosphor persistence decay modeling in Q16.16,
//!   zero-allocation raster stepping, and lowering to `PixelContainer` and `PaintScene`.
//! - **Vector Beam Display (`VectorBeamDisplay`)**: X/Y deflection amplifier simulation with
//!   bounded slewing rate, Z-axis intensity blanking modulation, bright-spot dwell timing,
//!   zero-allocation stroke rasterization, and lowering to `PaintScene`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use paint_instructions::{
    PaintBase, PaintEllipse, PaintInstruction, PaintLine, PaintRect, PaintScene, StrokeCap,
};
pub use pixel_container::PixelContainer;

/// RGB 24-bit color triplet.
pub type RgbColor = [u8; 3];

/// Maximum batch instruction count for lowered PaintScene representations (CWE-400 resource limit).
pub const MAX_CRT_SCENE_INSTRUCTIONS: usize = 4096;

/// Standard maximum amplifier slew rate per simulation step.
pub const MAX_SLEW_RATE_LIMIT: f64 = 100_000.0;

/// Errors produced by CRT raster and vector display operations.
#[derive(Clone, Debug, PartialEq)]
pub enum CrtError {
    /// Invalid raster or display dimensions (dimensions must be non-zero and within supported limits).
    InvalidDimensions {
        /// Configured width.
        width: u32,
        /// Configured height.
        height: u32,
    },
    /// Invalid raster timing parameters.
    InvalidTiming(String),
    /// Invalid phosphor persistence decay factor (must be in range 1..=65536 in Q16.16).
    InvalidPhosphorDecay(u32),
    /// Invalid deflection amplifier slewing rate (must be finite, positive, and <= max threshold).
    InvalidSlewRate(f64),
    /// The supplied phosphor or pixel buffer is smaller than required.
    BufferTooSmall {
        /// Minimum buffer elements required.
        required: usize,
        /// Number of elements provided in the caller slice.
        provided: usize,
    },
    /// Coordinate is non-finite (NaN or infinite).
    NonFiniteCoordinate {
        /// X coordinate value.
        x: f64,
        /// Y coordinate value.
        y: f64,
    },
    /// Display coordinate is out of bounds for the display area.
    CoordinateOutOfBounds {
        /// X coordinate value.
        x: f64,
        /// Y coordinate value.
        y: f64,
    },
    /// Lowering error when constructing paint instructions or pixel buffers.
    LoweringError(String),
}

impl std::fmt::Display for CrtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid CRT dimensions {width}x{height}")
            }
            Self::InvalidTiming(msg) => write!(f, "invalid CRT raster timing: {msg}"),
            Self::InvalidPhosphorDecay(decay) => {
                write!(f, "invalid phosphor decay factor {decay} (must be in 1..=65536)")
            }
            Self::InvalidSlewRate(rate) => {
                write!(f, "invalid vector amplifier slew rate {rate}")
            }
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "phosphor buffer too small: required {required} elements, provided {provided}"
                )
            }
            Self::NonFiniteCoordinate { x, y } => {
                write!(f, "non-finite deflection coordinate ({x}, {y})")
            }
            Self::CoordinateOutOfBounds { x, y } => {
                write!(f, "coordinate ({x}, {y}) is out of display bounds")
            }
            Self::LoweringError(msg) => write!(f, "CRT lowering error: {msg}"),
        }
    }
}

impl std::error::Error for CrtError {}

/// Historical cathode-ray tube phosphor types and persistence characteristics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhosphorType {
    /// P4 white phosphor (typical monochrome television and early arcade raster displays like Pong).
    P4White,
    /// P31 green phosphor (medium persistence, common in oscilloscopes and vector arcade monitors).
    P31Green,
    /// P1 green phosphor (long persistence, historical radar scopes and DEC Type 30 CRT).
    P1Green,
    /// Custom phosphor with a caller-specified decay factor in Q16.16 fixed-point and RGB tint.
    Custom {
        /// Retention factor per refresh cycle in Q16.16 fixed-point (1..=65536).
        decay_q16: u32,
        /// Display RGB phosphor luminescence tint.
        tint: RgbColor,
    },
}

impl PhosphorType {
    /// Returns the per-refresh persistence retention factor in Q16.16 fixed point format.
    ///
    /// `65536` corresponds to unity (1.0, 100% retention / no decay), while lower values decay.
    pub const fn phosphor_decay_factor_q16(&self) -> u32 {
        match self {
            Self::P4White => 32768,   // 50% persistence retention per frame
            Self::P31Green => 45056,  // ~68.75% persistence retention per frame
            Self::P1Green => 59904,   // ~91.4% persistence retention per frame (slow decay)
            Self::Custom { decay_q16, .. } => *decay_q16,
        }
    }

    /// Returns the characteristic nominal RGB luminescence tint for this phosphor chemistry.
    pub const fn phosphor_rgb_tint(&self) -> RgbColor {
        match self {
            Self::P4White => [240, 245, 255],
            Self::P31Green => [64, 255, 64],
            Self::P1Green => [50, 220, 50],
            Self::Custom { tint, .. } => *tint,
        }
    }

    /// Validates phosphor decay parameters (CWE-20).
    pub fn validate_phosphor(&self) -> Result<(), CrtError> {
        let decay = self.phosphor_decay_factor_q16();
        if decay == 0 || decay > 65536 {
            return Err(CrtError::InvalidPhosphorDecay(decay));
        }
        Ok(())
    }
}

/// Horizontal and vertical scan timing intervals for raster CRT beam deflection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RasterTiming {
    /// Number of active visible pixels per scanline.
    pub visible_width: u32,
    /// Number of active visible scanlines per frame.
    pub visible_height: u32,
    /// Horizontal front porch dot clocks.
    pub h_front_porch: u32,
    /// Horizontal sync pulse duration in dot clocks.
    pub h_sync: u32,
    /// Horizontal back porch dot clocks.
    pub h_back_porch: u32,
    /// Vertical front porch scanlines.
    pub v_front_porch: u32,
    /// Vertical sync pulse duration in scanlines.
    pub v_sync: u32,
    /// Vertical back porch scanlines.
    pub v_back_porch: u32,
}

impl RasterTiming {
    /// Standard NTSC-style raster display timing (320x240 visible, 416x262 total raster sweep).
    pub const fn standard_ntsc() -> Self {
        Self {
            visible_width: 320,
            visible_height: 240,
            h_front_porch: 16,
            h_sync: 32,
            h_back_porch: 48,
            v_front_porch: 3,
            v_sync: 3,
            v_back_porch: 16,
        }
    }

    /// Classic arcade CRT raster timing (256x224 visible, 344x260 total raster sweep).
    pub const fn standard_arcade() -> Self {
        Self {
            visible_width: 256,
            visible_height: 224,
            h_front_porch: 16,
            h_sync: 32,
            h_back_porch: 40,
            v_front_porch: 4,
            v_sync: 4,
            v_back_porch: 28,
        }
    }

    /// Total horizontal scanline duration in dot clocks (visible + porches + sync).
    pub const fn total_raster_width(&self) -> u32 {
        self.visible_width + self.h_front_porch + self.h_sync + self.h_back_porch
    }

    /// Total vertical frame duration in scanlines (visible + porches + sync).
    pub const fn total_raster_height(&self) -> u32 {
        self.visible_height + self.v_front_porch + self.v_sync + self.v_back_porch
    }

    /// Total active visible pixel count (`visible_width * visible_height`).
    pub const fn total_raster_pixels(&self) -> usize {
        (self.visible_width as usize) * (self.visible_height as usize)
    }

    /// Validates timing parameters for non-zero bounds and overflow protection (CWE-20, CWE-190).
    pub fn validate_timing(&self) -> Result<(), CrtError> {
        if self.visible_width == 0
            || self.visible_height == 0
            || self.visible_width > 4096
            || self.visible_height > 4096
        {
            return Err(CrtError::InvalidDimensions {
                width: self.visible_width,
                height: self.visible_height,
            });
        }
        let total_w = self
            .visible_width
            .checked_add(self.h_front_porch)
            .and_then(|w| w.checked_add(self.h_sync))
            .and_then(|w| w.checked_add(self.h_back_porch));
        let total_h = self
            .visible_height
            .checked_add(self.v_front_porch)
            .and_then(|h| h.checked_add(self.v_sync))
            .and_then(|h| h.checked_add(self.v_back_porch));

        match (total_w, total_h) {
            (Some(w), Some(h)) if w <= 8192 && h <= 8192 => Ok(()),
            _ => Err(CrtError::InvalidTiming(
                "total raster sweep exceeds supported coordinate limits".to_string(),
            )),
        }
    }
}

/// Standalone timing validator matching the formal Warrant operation.
pub fn validate_raster_timing(timing: RasterTiming) -> Result<(), CrtError> {
    timing.validate_timing()
}

/// Real-time deflection state flags for a raster electron beam.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BeamFlags {
    /// True when the beam is in the horizontal blanking interval.
    pub in_hblank: bool,
    /// True when the beam is in the vertical blanking interval.
    pub in_vblank: bool,
    /// True during the active horizontal synchronization pulse.
    pub in_hsync: bool,
    /// True during the active vertical synchronization pulse.
    pub in_vsync: bool,
    /// True on the exact cycle where the vertical retrace completes and a new frame begins.
    pub frame_completed: bool,
    /// True on the exact cycle where the horizontal retrace completes and a new scanline begins.
    pub line_completed: bool,
    /// True when the beam is inside the visible active display area.
    pub visible_pixel: bool,
}

/// Simulates electron beam deflection and phosphor luminescence across a raster CRT.
#[derive(Clone, Debug, PartialEq)]
pub struct CrtRasterBeam {
    /// Active timing configuration.
    pub timing: RasterTiming,
    /// Phosphor model.
    pub phosphor_type: PhosphorType,
    /// Current dot clock along the horizontal sweep (`0..total_raster_width`).
    pub beam_x: u32,
    /// Current scanline along the vertical sweep (`0..total_raster_height`).
    pub beam_y: u32,
    /// Total completed raster frames.
    pub frame_count: u64,
}

impl CrtRasterBeam {
    /// Constructs a new raster CRT beam generator with validated timing and phosphor.
    pub fn new_raster_beam(
        timing: RasterTiming,
        phosphor_type: PhosphorType,
    ) -> Result<Self, CrtError> {
        timing.validate_timing()?;
        phosphor_type.validate_phosphor()?;
        Ok(Self {
            timing,
            phosphor_type,
            beam_x: 0,
            beam_y: 0,
            frame_count: 0,
        })
    }

    /// Current beam horizontal dot position.
    pub fn raster_beam_x(&self) -> u32 {
        self.beam_x
    }

    /// Current beam vertical scanline position.
    pub fn raster_beam_y(&self) -> u32 {
        self.beam_y
    }

    /// Total frames completed since reset.
    pub fn raster_frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Resets the beam coordinates to the top-left origin (0, 0).
    pub fn reset_raster_beam(&mut self) {
        self.beam_x = 0;
        self.beam_y = 0;
    }

    /// Advances the electron beam by one dot clock with zero heap allocation.
    ///
    /// If the beam is within the active display area, the caller's `phosphor` buffer is
    /// excited with `video_luma` using saturating 16-bit accumulation (CWE-190).
    /// Bounds checks protect against out-of-bounds slice access (CWE-125).
    pub fn step_raster_beam(
        &mut self,
        video_luma: u8,
        phosphor: &mut [u16],
    ) -> Result<BeamFlags, CrtError> {
        let is_visible_x = self.beam_x < self.timing.visible_width;
        let is_visible_y = self.beam_y < self.timing.visible_height;
        let visible = is_visible_x && is_visible_y;

        let h_fp_end = self.timing.visible_width + self.timing.h_front_porch;
        let h_sync_end = h_fp_end + self.timing.h_sync;
        let in_hblank = !is_visible_x;
        let in_hsync = self.beam_x >= h_fp_end && self.beam_x < h_sync_end;

        let v_fp_end = self.timing.visible_height + self.timing.v_front_porch;
        let v_sync_end = v_fp_end + self.timing.v_sync;
        let in_vblank = !is_visible_y;
        let in_vsync = self.beam_y >= v_fp_end && self.beam_y < v_sync_end;

        if visible {
            let idx = (self.beam_y as usize) * (self.timing.visible_width as usize)
                + (self.beam_x as usize);
            if idx >= phosphor.len() {
                return Err(CrtError::BufferTooSmall {
                    required: idx + 1,
                    provided: phosphor.len(),
                });
            }
            let excitation = ((video_luma as u16) << 8) | (video_luma as u16);
            phosphor[idx] = phosphor[idx].saturating_add(excitation);
        }

        let mut line_completed = false;
        let mut frame_completed = false;

        self.beam_x += 1;
        if self.beam_x >= self.timing.total_raster_width() {
            self.beam_x = 0;
            line_completed = true;
            self.beam_y += 1;
            if self.beam_y >= self.timing.total_raster_height() {
                self.beam_y = 0;
                frame_completed = true;
                self.frame_count = self.frame_count.wrapping_add(1);
            }
        }

        Ok(BeamFlags {
            in_hblank,
            in_vblank,
            in_hsync,
            in_vsync,
            frame_completed,
            line_completed,
            visible_pixel: visible,
        })
    }

    /// Lowers active phosphor buffer luminescence to an RGBA8 [`PixelContainer`].
    pub fn lower_raster_to_pixel_buffer(
        &self,
        phosphor: &[u16],
        out_pixels: &mut PixelContainer,
    ) -> Result<(), CrtError> {
        let required = self.timing.total_raster_pixels();
        if phosphor.len() < required {
            return Err(CrtError::BufferTooSmall {
                required,
                provided: phosphor.len(),
            });
        }
        if out_pixels.width != self.timing.visible_width
            || out_pixels.height != self.timing.visible_height
        {
            return Err(CrtError::InvalidDimensions {
                width: out_pixels.width,
                height: out_pixels.height,
            });
        }

        let tint = self.phosphor_type.phosphor_rgb_tint();
        let width = self.timing.visible_width;
        let height = self.timing.visible_height;

        for y in 0..height {
            for x in 0..width {
                let idx = (y as usize) * (width as usize) + (x as usize);
                let lum = phosphor[idx];
                let r = ((tint[0] as u32 * lum as u32) / 65535) as u8;
                let g = ((tint[1] as u32 * lum as u32) / 65535) as u8;
                let b = ((tint[2] as u32 * lum as u32) / 65535) as u8;
                out_pixels.set_pixel(x, y, r, g, b, 255);
            }
        }
        Ok(())
    }

    /// Lowers the phosphor state into a compact [`PaintScene`] intermediate representation.
    ///
    /// Pixels with intensity above `min_intensity_threshold` are grouped into scanline rectangles.
    pub fn lower_raster_to_paint_scene(
        &self,
        phosphor: &[u16],
        min_intensity_threshold: u16,
    ) -> Result<PaintScene, CrtError> {
        let required = self.timing.total_raster_pixels();
        if phosphor.len() < required {
            return Err(CrtError::BufferTooSmall {
                required,
                provided: phosphor.len(),
            });
        }

        let tint = self.phosphor_type.phosphor_rgb_tint();
        let width = self.timing.visible_width;
        let height = self.timing.visible_height;
        let mut instructions = Vec::new();

        for y in 0..height {
            let mut run_start: Option<(u32, u16)> = None;
            for x in 0..width {
                let idx = (y as usize) * (width as usize) + (x as usize);
                let lum = phosphor[idx];

                if lum >= min_intensity_threshold {
                    if let Some((start_x, acc_lum)) = run_start {
                        run_start = Some((start_x, acc_lum.max(lum)));
                    } else {
                        run_start = Some((x, lum));
                    }
                } else if let Some((start_x, max_lum)) = run_start.take() {
                    let r = ((tint[0] as u32 * max_lum as u32) / 65535) as u8;
                    let g = ((tint[1] as u32 * max_lum as u32) / 65535) as u8;
                    let b = ((tint[2] as u32 * max_lum as u32) / 65535) as u8;
                    let color = format!("#{r:02x}{g:02x}{b:02x}");

                    instructions.push(PaintInstruction::Rect(PaintRect {
                        base: PaintBase {
                            id: Some(format!("raster_line_{y}_{start_x}")),
                            metadata: None,
                        },
                        x: start_x as f64,
                        y: y as f64,
                        width: (x - start_x) as f64,
                        height: 1.0,
                        fill: Some(color),
                        stroke: None,
                        stroke_width: None,
                        corner_radius: None,
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));

                    if instructions.len() >= MAX_CRT_SCENE_INSTRUCTIONS {
                        break;
                    }
                }
            }

            if let Some((start_x, max_lum)) = run_start.take() {
                if instructions.len() < MAX_CRT_SCENE_INSTRUCTIONS {
                    let r = ((tint[0] as u32 * max_lum as u32) / 65535) as u8;
                    let g = ((tint[1] as u32 * max_lum as u32) / 65535) as u8;
                    let b = ((tint[2] as u32 * max_lum as u32) / 65535) as u8;
                    let color = format!("#{r:02x}{g:02x}{b:02x}");

                    instructions.push(PaintInstruction::Rect(PaintRect {
                        base: PaintBase {
                            id: Some(format!("raster_line_{y}_{start_x}")),
                            metadata: None,
                        },
                        x: start_x as f64,
                        y: y as f64,
                        width: (width - start_x) as f64,
                        height: 1.0,
                        fill: Some(color),
                        stroke: None,
                        stroke_width: None,
                        corner_radius: None,
                        stroke_dash: None,
                        stroke_dash_offset: None,
                    }));
                }
            }

            if instructions.len() >= MAX_CRT_SCENE_INSTRUCTIONS {
                break;
            }
        }

        Ok(PaintScene {
            width: width as f64,
            height: height as f64,
            background: "#000000".to_string(),
            instructions,
            id: None,
            metadata: None,
        })
    }
}

/// Applies exponential persistence decay across a caller-supplied phosphor buffer with zero heap allocation.
pub fn decay_phosphor_buffer(
    phosphor: &mut [u16],
    decay_factor_q16: u32,
) -> Result<(), CrtError> {
    if decay_factor_q16 == 0 || decay_factor_q16 > 65536 {
        return Err(CrtError::InvalidPhosphorDecay(decay_factor_q16));
    }
    for val in phosphor.iter_mut() {
        *val = ((*val as u32 * decay_factor_q16) >> 16) as u16;
    }
    Ok(())
}

/// Clears a phosphor persistence buffer to complete darkness (0).
pub fn clear_phosphor_buffer(phosphor: &mut [u16]) {
    phosphor.fill(0);
}

/// Pure mathematical phosphor sample decay helper.
pub fn decay_phosphor_sample(sample: u16, decay_factor_q16: u32) -> u16 {
    let factor = if decay_factor_q16 > 65536 {
        65536
    } else {
        decay_factor_q16
    };
    ((sample as u32 * factor) >> 16) as u16
}

/// Pure saturating intensity accumulation helper (CWE-190).
pub fn accumulate_intensity(current: u16, add_intensity: u16) -> u16 {
    current.saturating_add(add_intensity)
}

/// A directed 2D line segment or dwell point for vector beam deflection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorStroke {
    /// Start X coordinate.
    pub x0: f64,
    /// Start Y coordinate.
    pub y0: f64,
    /// End X coordinate.
    pub x1: f64,
    /// End Y coordinate.
    pub y1: f64,
    /// Beam luminescence intensity (0 = blanked transit, 1..=255 = unblanked).
    pub intensity: u8,
    /// Additional dwell cycles at the target endpoint (for bright spot rendering).
    pub dwell_ticks: u32,
}

impl VectorStroke {
    /// Creates a new unblanked or transit stroke from (x0, y0) to (x1, y1).
    pub const fn new_vector_stroke(x0: f64, y0: f64, x1: f64, y1: f64, intensity: u8) -> Self {
        Self {
            x0,
            y0,
            x1,
            y1,
            intensity,
            dwell_ticks: 0,
        }
    }

    /// Creates a stationary dwell spot at (x, y) with bright-spot dwell cycles.
    pub const fn new_dwell_spot(x: f64, y: f64, intensity: u8, dwell_ticks: u32) -> Self {
        Self {
            x0: x,
            y0: y,
            x1: x,
            y1: y,
            intensity,
            dwell_ticks,
        }
    }

    /// Calculates Euclidean length of this vector stroke.
    pub fn stroke_length(&self) -> f64 {
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Physical deflection amplifier state and beam luminescence for a single time step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorBeamPoint {
    /// Current deflection beam X position.
    pub x: f64,
    /// Current deflection beam Y position.
    pub y: f64,
    /// True if the beam is blanked (Z-axis cutoff).
    pub blanked: bool,
    /// Active luminescence intensity on the phosphor.
    pub intensity: u8,
    /// True if the deflection amplifier has settled onto the commanded target position.
    pub settled: bool,
}

/// Standalone slew rate validator matching the formal Warrant contract.
pub fn validate_slew_rate(slew_rate: f64) -> Result<(), CrtError> {
    if !slew_rate.is_finite() || slew_rate <= 0.0 || slew_rate > MAX_SLEW_RATE_LIMIT {
        Err(CrtError::InvalidSlewRate(slew_rate))
    } else {
        Ok(())
    }
}

/// Simulates an oscilloscope or vector CRT display (DEC Type 30 / Asteroids-style).
#[derive(Clone, Debug, PartialEq)]
pub struct VectorBeamDisplay {
    /// Logical display viewport width.
    pub display_width: f64,
    /// Logical display viewport height.
    pub display_height: f64,
    /// Maximum deflection slewing speed (distance per simulation time step).
    pub max_slew_rate: f64,
    /// Phosphor persistence model.
    pub phosphor_type: PhosphorType,
    /// Current instantaneous beam X coordinate.
    pub current_x: f64,
    /// Current instantaneous beam Y coordinate.
    pub current_y: f64,
    /// Current Z-axis blanking state.
    pub z_blanked: bool,
    /// Current beam intensity.
    pub current_intensity: u8,
}

impl VectorBeamDisplay {
    /// Constructs a new vector beam display model with validated dimensions and slew limits.
    pub fn new_vector_display(
        display_width: f64,
        display_height: f64,
        max_slew_rate: f64,
        phosphor_type: PhosphorType,
    ) -> Result<Self, CrtError> {
        if !display_width.is_finite()
            || !display_height.is_finite()
            || display_width <= 0.0
            || display_height <= 0.0
        {
            return Err(CrtError::InvalidDimensions {
                width: display_width as u32,
                height: display_height as u32,
            });
        }
        validate_slew_rate(max_slew_rate)?;
        phosphor_type.validate_phosphor()?;

        Ok(Self {
            display_width,
            display_height,
            max_slew_rate,
            phosphor_type,
            current_x: 0.0,
            current_y: 0.0,
            z_blanked: true,
            current_intensity: 0,
        })
    }

    /// Instantaneous horizontal coordinate of the deflection beam.
    pub fn vector_beam_x(&self) -> f64 {
        self.current_x
    }

    /// Instantaneous vertical coordinate of the deflection beam.
    pub fn vector_beam_y(&self) -> f64 {
        self.current_y
    }

    /// Resets deflection position to (0.0, 0.0) with blanked Z-axis beam.
    pub fn reset_vector_beam(&mut self) {
        self.current_x = 0.0;
        self.current_y = 0.0;
        self.z_blanked = true;
        self.current_intensity = 0;
    }

    /// Advances the deflection amplifier toward `(target_x, target_y)` by one simulation step.
    ///
    /// Movement is strictly bounded by `max_slew_rate` (CWE-20). Operates with zero heap allocation.
    pub fn step_vector_beam(
        &mut self,
        target_x: f64,
        target_y: f64,
        blanked: bool,
        intensity: u8,
    ) -> Result<VectorBeamPoint, CrtError> {
        if !target_x.is_finite() || !target_y.is_finite() {
            return Err(CrtError::NonFiniteCoordinate {
                x: target_x,
                y: target_y,
            });
        }

        if target_x.abs() > 1.0e12 || target_y.abs() > 1.0e12 {
            return Err(CrtError::CoordinateOutOfBounds {
                x: target_x,
                y: target_y,
            });
        }

        let dx = target_x - self.current_x;
        let dy = target_y - self.current_y;
        let dist = dx.hypot(dy);

        let settled = if dist <= self.max_slew_rate || dist == 0.0 {
            self.current_x = target_x;
            self.current_y = target_y;
            true
        } else {
            let ratio = self.max_slew_rate / dist;
            self.current_x += dx * ratio;
            self.current_y += dy * ratio;
            false
        };

        self.z_blanked = blanked;
        self.current_intensity = if blanked { 0 } else { intensity };

        Ok(VectorBeamPoint {
            x: self.current_x,
            y: self.current_y,
            blanked: self.z_blanked,
            intensity: self.current_intensity,
            settled,
        })
    }

    /// Rasterizes a vector stroke or dwell spot into a caller-supplied phosphor buffer.
    ///
    /// Performs zero heap allocations on the hot path. Returns the number of excited pixels.
    pub fn rasterize_vector_stroke_into(
        &mut self,
        stroke: &VectorStroke,
        phosphor: &mut [u16],
        buf_width: u32,
        buf_height: u32,
    ) -> Result<usize, CrtError> {
        if !stroke.x0.is_finite()
            || !stroke.y0.is_finite()
            || !stroke.x1.is_finite()
            || !stroke.y1.is_finite()
        {
            return Err(CrtError::NonFiniteCoordinate {
                x: stroke.x0,
                y: stroke.y0,
            });
        }
        if buf_width == 0 || buf_height == 0 {
            return Err(CrtError::InvalidDimensions {
                width: buf_width,
                height: buf_height,
            });
        }

        let total_pixels = (buf_width as usize) * (buf_height as usize);
        if phosphor.len() < total_pixels {
            return Err(CrtError::BufferTooSmall {
                required: total_pixels,
                provided: phosphor.len(),
            });
        }

        // Commanded transit to start point
        self.current_x = stroke.x0;
        self.current_y = stroke.y0;

        if stroke.intensity == 0 {
            self.z_blanked = true;
            self.current_intensity = 0;
            self.current_x = stroke.x1;
            self.current_y = stroke.y1;
            return Ok(0);
        }

        self.z_blanked = false;
        self.current_intensity = stroke.intensity;
        let mut excited_count = 0;

        if stroke.x0.abs() > 1.0e12 || stroke.y0.abs() > 1.0e12 {
            return Err(CrtError::CoordinateOutOfBounds {
                x: stroke.x0,
                y: stroke.y0,
            });
        }
        if stroke.x1.abs() > 1.0e12 || stroke.y1.abs() > 1.0e12 {
            return Err(CrtError::CoordinateOutOfBounds {
                x: stroke.x1,
                y: stroke.y1,
            });
        }

        let dx = stroke.x1 - stroke.x0;
        let dy = stroke.y1 - stroke.y0;
        let len = dx.hypot(dy);

        // Stationary dwell point handling
        if len < 0.5 {
            let px = stroke.x0.round() as i64;
            let py = stroke.y0.round() as i64;
            if px >= 0 && px < buf_width as i64 && py >= 0 && py < buf_height as i64 {
                let idx = (py as usize) * (buf_width as usize) + (px as usize);
                let dwell_factor = 1u32.saturating_add(stroke.dwell_ticks.min(255));
                let excitation = ((stroke.intensity as u32) * dwell_factor).min(65535) as u16;
                phosphor[idx] = phosphor[idx].saturating_add(excitation);
                excited_count += 1;
            }
            self.current_x = stroke.x1;
            self.current_y = stroke.y1;
            return Ok(excited_count);
        }

        // Stepped interpolation along vector stroke
        let steps = (len.ceil() as usize).clamp(1, 4096);
        let excitation = ((stroke.intensity as u16) << 8) | (stroke.intensity as u16);

        for step in 0..=steps {
            let t = (step as f64) / (steps as f64);
            let cur_x = stroke.x0 + t * dx;
            let cur_y = stroke.y0 + t * dy;

            let px = cur_x.round() as i64;
            let py = cur_y.round() as i64;

            if px >= 0 && px < buf_width as i64 && py >= 0 && py < buf_height as i64 {
                let idx = (py as usize) * (buf_width as usize) + (px as usize);
                phosphor[idx] = phosphor[idx].saturating_add(excitation);
                excited_count += 1;
            }
        }

        // If dwell ticks requested at endpoint
        if stroke.dwell_ticks > 0 {
            let px = stroke.x1.round() as i64;
            let py = stroke.y1.round() as i64;
            if px >= 0 && px < buf_width as i64 && py >= 0 && py < buf_height as i64 {
                let idx = (py as usize) * (buf_width as usize) + (px as usize);
                let dwell_extra =
                    ((stroke.intensity as u32) * stroke.dwell_ticks.min(255)).min(65535) as u16;
                phosphor[idx] = phosphor[idx].saturating_add(dwell_extra);
            }
        }

        self.current_x = stroke.x1;
        self.current_y = stroke.y1;
        Ok(excited_count)
    }

    /// Lowers an array of vector strokes to a [`PaintScene`] intermediate representation.
    pub fn lower_vector_strokes_to_paint_scene(
        strokes: &[VectorStroke],
        scene_width: f64,
        scene_height: f64,
        phosphor_type: PhosphorType,
    ) -> Result<PaintScene, CrtError> {
        if !scene_width.is_finite()
            || !scene_height.is_finite()
            || scene_width <= 0.0
            || scene_height <= 0.0
        {
            return Err(CrtError::InvalidDimensions {
                width: scene_width as u32,
                height: scene_height as u32,
            });
        }
        phosphor_type.validate_phosphor()?;

        let tint = phosphor_type.phosphor_rgb_tint();
        let mut instructions = Vec::with_capacity(strokes.len().min(MAX_CRT_SCENE_INSTRUCTIONS));

        for (i, stroke) in strokes.iter().enumerate() {
            if instructions.len() >= MAX_CRT_SCENE_INSTRUCTIONS {
                break;
            }
            if stroke.intensity == 0 {
                continue;
            }
            if !stroke.x0.is_finite()
                || !stroke.y0.is_finite()
                || stroke.x0.abs() > 1.0e12
                || stroke.y0.abs() > 1.0e12
            {
                return Err(CrtError::CoordinateOutOfBounds {
                    x: stroke.x0,
                    y: stroke.y0,
                });
            }
            if !stroke.x1.is_finite()
                || !stroke.y1.is_finite()
                || stroke.x1.abs() > 1.0e12
                || stroke.y1.abs() > 1.0e12
            {
                return Err(CrtError::CoordinateOutOfBounds {
                    x: stroke.x1,
                    y: stroke.y1,
                });
            }

            let r = ((tint[0] as u32 * stroke.intensity as u32) / 255) as u8;
            let g = ((tint[1] as u32 * stroke.intensity as u32) / 255) as u8;
            let b = ((tint[2] as u32 * stroke.intensity as u32) / 255) as u8;
            let color = format!("#{r:02x}{g:02x}{b:02x}");

            let dx = stroke.x1 - stroke.x0;
            let dy = stroke.y1 - stroke.y0;
            let len = (dx * dx + dy * dy).sqrt();

            if len < 0.5 {
                // Bright spot dwell point
                let radius = 1.0 + (stroke.dwell_ticks.min(10) as f64) * 0.25;
                instructions.push(PaintInstruction::Ellipse(PaintEllipse {
                    base: PaintBase {
                        id: Some(format!("vector_dwell_{i}")),
                        metadata: None,
                    },
                    cx: stroke.x0,
                    cy: stroke.y0,
                    rx: radius,
                    ry: radius,
                    fill: Some(color),
                    stroke: None,
                    stroke_width: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            } else {
                instructions.push(PaintInstruction::Line(PaintLine {
                    base: PaintBase {
                        id: Some(format!("vector_stroke_{i}")),
                        metadata: None,
                    },
                    x1: stroke.x0,
                    y1: stroke.y0,
                    x2: stroke.x1,
                    y2: stroke.y1,
                    stroke: color,
                    stroke_width: Some(1.5),
                    stroke_cap: Some(StrokeCap::Round),
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
        }

        Ok(PaintScene {
            width: scene_width,
            height: scene_height,
            background: "#000000".to_string(),
            instructions,
            id: None,
            metadata: None,
        })
    }
}
