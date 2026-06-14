// # image-raw-pipeline — Shared RAW Colour Development Pipeline
//
// ## What this crate provides
//
// Camera RAW formats (TIFF, DNG, CR2, NEF, ARW, RAF, ORF, RW2) all require
// the same four-stage colour pipeline to turn raw sensor values into a
// displayable sRGB image. Before this crate, that pipeline was duplicated
// in three separate `color.rs` files across the RAW codec crates. This crate
// extracts the canonical implementation.
//
// ## Exported API
//
// | Function | Description |
// |----------|-------------|
// | `srgb_gamma(linear)` | Apply IEC 61966-2-1 EOTF: linear → display-encoded |
// | `srgb_decode(encoded)` | Inverse EOTF: display-encoded → linear light |
// | `mat3x3_mul(m, v)` | 3×3 × column-vector, no heap allocation |
// | `invert_3x3(m)` | Analytic 3×3 inversion via Cramer's rule |
// | `apply_color_pipeline(…)` | Full 4-stage RAW development: normalize → WB → matrix → gamma |
//
// ## Dependency policy
//
// Zero external dependencies. All math is elementary scalar arithmetic on
// stack-allocated `[f64;3]` arrays and `[[f64;3];3]` matrices.
//
// ## Spec
//
// See `code/specs/IMG07-image-raw-pipeline.md`.

pub mod gamma;
pub mod matrix;
pub mod pipeline;

pub use gamma::{srgb_gamma, srgb_decode};
pub use matrix::{mat3x3_mul, invert_3x3};
pub use pipeline::apply_color_pipeline;
