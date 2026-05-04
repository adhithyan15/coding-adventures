//! # `instagram-filters` — end-to-end matrix execution layer demo
//!
//! Library half of the CLI demo.  Owns the filter dispatch logic so
//! the CLI binary stays a thin wrapper over [`apply_filter`] and the
//! parsing / I/O code in `main.rs`.
//!
//! ## What this program proves
//!
//! Every filter in this demo composes from the matrix execution layer:
//!
//! ```text
//!   PPM bytes  →  PixelContainer  →  image-gpu-core (MatrixIR builder)
//!                                           ↓
//!                              matrix-runtime planner
//!                                           ↓
//!                              matrix-cpu executor
//!                                           ↓
//!                                    PixelContainer
//!                                           ↓
//!                                       PPM bytes
//! ```
//!
//! If the layer is wrong, the output image is visibly broken — channels
//! shifted, gamma off, alpha lost.  This is a stronger end-to-end test
//! than asserting `[1.0, 2.0, 3.0]` round-trips through a graph.

use image_gpu_core::{
    gpu_brightness, gpu_contrast, gpu_gamma, gpu_greyscale, gpu_invert, gpu_posterize, gpu_sepia,
    GpuError, LuminanceWeights,
};
use pixel_container::PixelContainer;

/// The set of filters this program supports.  Each variant maps to a
/// single image-gpu-core function which in turn builds a MatrixIR
/// graph and runs it through the matrix execution layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Filter {
    /// Invert RGB channels.  Alpha unchanged.
    Invert,
    /// Greyscale via Rec.709 luminance weights.  Linear-light math.
    Greyscale,
    /// Classic sepia tone (3×3 colour matrix in linear light).
    Sepia,
    /// Additive brightness shift in sRGB byte space.  `delta ∈ [-255, 255]`.
    Brightness { delta: i16 },
    /// Power-law gamma in linear light.  γ < 1 brightens midtones, γ > 1 darkens.
    Gamma { gamma: f32 },
    /// Contrast scale around mid-grey 128.  `scale > 1` stretches, `0 < scale < 1` flattens.
    Contrast { scale: f32 },
    /// Posterize: reduce to `levels` distinct values per channel.
    Posterize { levels: u8 },
}

/// Parameter validation errors — produced by `Filter::parse_with_args`
/// and surfaced to the CLI user as a clean error message.
#[derive(Debug, PartialEq)]
pub enum FilterParamError {
    UnknownFilter(String),
    BrightnessOutOfRange(i32),
    GammaNonPositive(f32),
    PosterizeZeroLevels,
    MissingRequiredArg { filter: String, arg: String },
    InvalidNumber { arg: String, value: String },
}

impl core::fmt::Display for FilterParamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FilterParamError::UnknownFilter(name) => {
                write!(f, "unknown filter '{}': try invert, greyscale, sepia, brightness, gamma, contrast, or posterize", name)
            }
            FilterParamError::BrightnessOutOfRange(v) => {
                write!(f, "brightness delta {} is outside [-255, 255]", v)
            }
            FilterParamError::GammaNonPositive(g) => {
                write!(f, "gamma must be positive (got {})", g)
            }
            FilterParamError::PosterizeZeroLevels => {
                write!(f, "posterize levels must be at least 1")
            }
            FilterParamError::MissingRequiredArg { filter, arg } => {
                write!(f, "filter '{}' requires --{}", filter, arg)
            }
            FilterParamError::InvalidNumber { arg, value } => {
                write!(f, "--{} expects a number, got '{}'", arg, value)
            }
        }
    }
}

impl std::error::Error for FilterParamError {}

impl Filter {
    /// Parse a `--filter NAME` token plus an associative array of
    /// remaining named arguments (`--amount`, `--gamma`, `--scale`,
    /// `--levels`).  Returns the validated filter or a typed error
    /// the caller can render to stderr.
    pub fn parse_with_args(
        name: &str,
        args: &std::collections::HashMap<String, String>,
    ) -> Result<Filter, FilterParamError> {
        match name {
            "invert" => Ok(Filter::Invert),
            "greyscale" | "grayscale" => Ok(Filter::Greyscale),
            "sepia" => Ok(Filter::Sepia),
            "brightness" => {
                let v = args
                    .get("amount")
                    .ok_or_else(|| FilterParamError::MissingRequiredArg {
                        filter: "brightness".into(),
                        arg: "amount".into(),
                    })?;
                let delta: i32 = v.parse().map_err(|_| FilterParamError::InvalidNumber {
                    arg: "amount".into(),
                    value: v.clone(),
                })?;
                if !(-255..=255).contains(&delta) {
                    return Err(FilterParamError::BrightnessOutOfRange(delta));
                }
                Ok(Filter::Brightness { delta: delta as i16 })
            }
            "gamma" => {
                let v = args
                    .get("gamma")
                    .ok_or_else(|| FilterParamError::MissingRequiredArg {
                        filter: "gamma".into(),
                        arg: "gamma".into(),
                    })?;
                let g: f32 = v.parse().map_err(|_| FilterParamError::InvalidNumber {
                    arg: "gamma".into(),
                    value: v.clone(),
                })?;
                if g <= 0.0 {
                    return Err(FilterParamError::GammaNonPositive(g));
                }
                Ok(Filter::Gamma { gamma: g })
            }
            "contrast" => {
                let v = args
                    .get("scale")
                    .ok_or_else(|| FilterParamError::MissingRequiredArg {
                        filter: "contrast".into(),
                        arg: "scale".into(),
                    })?;
                let s: f32 = v.parse().map_err(|_| FilterParamError::InvalidNumber {
                    arg: "scale".into(),
                    value: v.clone(),
                })?;
                Ok(Filter::Contrast { scale: s })
            }
            "posterize" => {
                let v = args
                    .get("levels")
                    .ok_or_else(|| FilterParamError::MissingRequiredArg {
                        filter: "posterize".into(),
                        arg: "levels".into(),
                    })?;
                let l: u32 = v.parse().map_err(|_| FilterParamError::InvalidNumber {
                    arg: "levels".into(),
                    value: v.clone(),
                })?;
                if l == 0 || l > 255 {
                    return Err(FilterParamError::PosterizeZeroLevels);
                }
                Ok(Filter::Posterize { levels: l as u8 })
            }
            other => Err(FilterParamError::UnknownFilter(other.to_string())),
        }
    }

    /// Human-readable name of this filter, useful for log lines.
    pub fn name(&self) -> &'static str {
        match self {
            Filter::Invert => "invert",
            Filter::Greyscale => "greyscale",
            Filter::Sepia => "sepia",
            Filter::Brightness { .. } => "brightness",
            Filter::Gamma { .. } => "gamma",
            Filter::Contrast { .. } => "contrast",
            Filter::Posterize { .. } => "posterize",
        }
    }
}

/// Apply `filter` to `image`, dispatching through image-gpu-core.
///
/// Each filter call builds a MatrixIR graph, plans it, and runs it
/// on matrix-cpu.  The end-to-end execution path is what this whole
/// program is designed to exercise.
pub fn apply_filter(filter: Filter, image: &PixelContainer) -> Result<PixelContainer, GpuError> {
    match filter {
        Filter::Invert => gpu_invert(image),
        Filter::Greyscale => gpu_greyscale(image, LuminanceWeights::Rec709),
        Filter::Sepia => gpu_sepia(image),
        Filter::Brightness { delta } => gpu_brightness(image, delta),
        Filter::Gamma { gamma } => gpu_gamma(image, gamma),
        Filter::Contrast { scale } => gpu_contrast(image, scale),
        Filter::Posterize { levels } => gpu_posterize(image, levels),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn args_with(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        let mut m = HashMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v.to_string());
        }
        m
    }

    #[test]
    fn parse_invert_no_args() {
        let f = Filter::parse_with_args("invert", &HashMap::new()).unwrap();
        assert_eq!(f, Filter::Invert);
    }

    #[test]
    fn parse_greyscale_accepts_alt_spelling() {
        assert_eq!(
            Filter::parse_with_args("grayscale", &HashMap::new()).unwrap(),
            Filter::Greyscale
        );
        assert_eq!(
            Filter::parse_with_args("greyscale", &HashMap::new()).unwrap(),
            Filter::Greyscale
        );
    }

    #[test]
    fn parse_brightness_with_amount() {
        let args = args_with(&[("amount", "30")]);
        let f = Filter::parse_with_args("brightness", &args).unwrap();
        assert_eq!(f, Filter::Brightness { delta: 30 });
    }

    #[test]
    fn brightness_out_of_range_errors() {
        let args = args_with(&[("amount", "1000")]);
        let err = Filter::parse_with_args("brightness", &args).unwrap_err();
        assert_eq!(err, FilterParamError::BrightnessOutOfRange(1000));
    }

    #[test]
    fn brightness_missing_amount_errors() {
        let err = Filter::parse_with_args("brightness", &HashMap::new()).unwrap_err();
        assert!(matches!(
            err,
            FilterParamError::MissingRequiredArg { .. }
        ));
    }

    #[test]
    fn gamma_must_be_positive() {
        let args = args_with(&[("gamma", "-0.5")]);
        let err = Filter::parse_with_args("gamma", &args).unwrap_err();
        assert!(matches!(err, FilterParamError::GammaNonPositive(_)));
    }

    #[test]
    fn posterize_zero_levels_errors() {
        let args = args_with(&[("levels", "0")]);
        let err = Filter::parse_with_args("posterize", &args).unwrap_err();
        assert_eq!(err, FilterParamError::PosterizeZeroLevels);
    }

    #[test]
    fn unknown_filter_errors() {
        let err = Filter::parse_with_args("xyzzy", &HashMap::new()).unwrap_err();
        assert!(matches!(err, FilterParamError::UnknownFilter(_)));
    }

    #[test]
    fn invalid_number_errors() {
        let args = args_with(&[("amount", "lots")]);
        let err = Filter::parse_with_args("brightness", &args).unwrap_err();
        assert!(matches!(err, FilterParamError::InvalidNumber { .. }));
    }

    fn solid(r: u8, g: u8, b: u8, a: u8) -> PixelContainer {
        let mut pc = PixelContainer::new(2, 2);
        pc.fill(r, g, b, a);
        pc
    }

    #[test]
    fn apply_invert_runs_pipeline() {
        let src = solid(100, 150, 200, 255);
        let out = apply_filter(Filter::Invert, &src).unwrap();
        assert_eq!(out.pixel_at(0, 0), (155, 105, 55, 255));
    }

    #[test]
    fn apply_brightness_runs_pipeline() {
        let src = solid(100, 100, 100, 255);
        let out = apply_filter(Filter::Brightness { delta: 50 }, &src).unwrap();
        assert_eq!(out.pixel_at(0, 0), (150, 150, 150, 255));
    }

    #[test]
    fn apply_sepia_warms_grey() {
        let src = solid(120, 120, 120, 255);
        let out = apply_filter(Filter::Sepia, &src).unwrap();
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert!(r >= g, "sepia should warm: R={} G={}", r, g);
        assert!(g >= b, "sepia should warm: G={} B={}", g, b);
    }

    #[test]
    fn apply_posterize_quantizes_to_levels() {
        let src = solid(200, 100, 50, 255);
        let out = apply_filter(Filter::Posterize { levels: 4 }, &src).unwrap();
        let (r, g, b, _) = out.pixel_at(0, 0);
        // 4 levels → step 64; values must be multiples of 64.
        assert_eq!(r % 64, 0);
        assert_eq!(g % 64, 0);
        assert_eq!(b % 64, 0);
    }
}
