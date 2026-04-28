//! # barcode-1d
//!
//! High-level Rust pipeline for 1D barcodes.

pub const VERSION: &str = "0.1.0";

use barcode_layout_1d::{Barcode1DRenderConfig, PaintBarcode1DOptions};
use paint_instructions::{PaintScene, PixelContainer};

pub use barcode_layout_1d::{
    Barcode1DLayoutTarget, Barcode1DRenderConfig as RenderConfig,
    PaintBarcode1DOptions as PaintOptions,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbology {
    Codabar,
    Code128,
    Code39,
    Ean13,
    Itf,
    UpcA,
}

impl Symbology {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Codabar => "codabar",
            Self::Code128 => "code128",
            Self::Code39 => "code39",
            Self::Ean13 => "ean13",
            Self::Itf => "itf",
            Self::UpcA => "upca",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub symbology: Symbology,
    pub paint: PaintBarcode1DOptions,
    pub codabar_start: Option<String>,
    pub codabar_stop: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            symbology: Symbology::Code39,
            paint: PaintBarcode1DOptions::default(),
            codabar_start: None,
            codabar_stop: None,
        }
    }
}

pub fn default_render_config() -> Barcode1DRenderConfig {
    Barcode1DRenderConfig::default()
}

pub fn current_backend() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "direct2d"
    }
    #[cfg(all(not(target_os = "windows"), target_vendor = "apple"))]
    {
        "metal"
    }
    #[cfg(all(not(target_os = "windows"), not(target_vendor = "apple")))]
    {
        "unavailable"
    }
}

pub fn normalize_symbology(symbology: &str) -> Result<Symbology, String> {
    let normalized = symbology
        .trim()
        .to_ascii_lowercase()
        .replace('-', "")
        .replace('_', "");
    let normalized = if normalized.is_empty() {
        "code39".to_string()
    } else {
        normalized
    };

    match normalized.as_str() {
        "codabar" => Ok(Symbology::Codabar),
        "code128" => Ok(Symbology::Code128),
        "code39" => Ok(Symbology::Code39),
        "ean13" => Ok(Symbology::Ean13),
        "itf" => Ok(Symbology::Itf),
        "upca" => Ok(Symbology::UpcA),
        _ => Err(format!("unsupported symbology: {symbology}")),
    }
}

pub fn build_scene(data: &str, options: Option<&Options>) -> Result<PaintScene, String> {
    let options = options.cloned().unwrap_or_default();
    match options.symbology {
        Symbology::Codabar => codabar::layout_codabar(
            data,
            options.codabar_start.as_deref(),
            options.codabar_stop.as_deref(),
            &options.paint,
        ),
        Symbology::Code128 => code128::layout_code128(data, &options.paint),
        Symbology::Code39 => code39::layout_code39(data, &options.paint),
        Symbology::Ean13 => ean_13::layout_ean13(data, &options.paint),
        Symbology::Itf => itf::layout_itf(data, &options.paint),
        Symbology::UpcA => upc_a::layout_upc_a(data, &options.paint),
    }
}

pub fn build_scene_for_symbology(
    symbology: &str,
    data: &str,
    options: Option<&Options>,
) -> Result<PaintScene, String> {
    let mut options = options.cloned().unwrap_or_default();
    options.symbology = normalize_symbology(symbology)?;
    build_scene(data, Some(&options))
}

pub fn render_pixels(data: &str, options: Option<&Options>) -> Result<PixelContainer, String> {
    let scene = build_scene(data, options)?;
    render_scene_to_pixels(&scene)
}

pub fn render_pixels_for_symbology(
    symbology: &str,
    data: &str,
    options: Option<&Options>,
) -> Result<PixelContainer, String> {
    let scene = build_scene_for_symbology(symbology, data, options)?;
    render_scene_to_pixels(&scene)
}

pub fn render_png(data: &str, options: Option<&Options>) -> Result<Vec<u8>, String> {
    let pixels = render_pixels(data, options)?;
    Ok(paint_codec_png::encode_png(&pixels))
}

pub fn render_png_for_symbology(
    symbology: &str,
    data: &str,
    options: Option<&Options>,
) -> Result<Vec<u8>, String> {
    let pixels = render_pixels_for_symbology(symbology, data, options)?;
    Ok(paint_codec_png::encode_png(&pixels))
}

fn render_scene_to_pixels(scene: &PaintScene) -> Result<PixelContainer, String> {
    #[cfg(target_os = "windows")]
    {
        return Ok(paint_vm_direct2d::render(scene));
    }
    #[cfg(all(not(target_os = "windows"), target_vendor = "apple"))]
    {
        return Ok(paint_metal::render(scene));
    }
    #[cfg(all(not(target_os = "windows"), not(target_vendor = "apple")))]
    {
        let _ = scene;
        Err(
            "native barcode rendering is not wired on this platform yet; build_scene() is available, but pixel rendering awaits a paint backend"
                .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn normalize_symbology_accepts_common_spellings() {
        assert_eq!(normalize_symbology("code39").unwrap(), Symbology::Code39);
        assert_eq!(normalize_symbology("code-128").unwrap(), Symbology::Code128);
        assert_eq!(normalize_symbology("ean_13").unwrap(), Symbology::Ean13);
        assert_eq!(normalize_symbology("").unwrap(), Symbology::Code39);
    }

    #[test]
    fn build_scene_routes_to_code39_by_default() {
        let scene = build_scene("HELLO-123", None).unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"code39".to_string())
        );
    }

    #[test]
    fn build_scene_routes_to_other_symbologies() {
        let scene = build_scene(
            "400638133393",
            Some(&Options {
                symbology: Symbology::Ean13,
                ..Options::default()
            }),
        )
        .unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"ean-13".to_string())
        );
    }

    #[test]
    fn codabar_start_stop_can_be_selected() {
        let scene = build_scene(
            "40156",
            Some(&Options {
                symbology: Symbology::Codabar,
                codabar_start: Some("B".to_string()),
                codabar_stop: Some("C".to_string()),
                ..Options::default()
            }),
        )
        .unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"codabar".to_string())
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("start")),
            Some(&"B".to_string())
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("stop")),
            Some(&"C".to_string())
        );
    }

    #[test]
    fn build_scene_for_symbology_accepts_string_input() {
        let scene = build_scene_for_symbology("code-128", "HELLO-123", None).unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"code128".to_string())
        );
    }

    #[cfg(any(target_os = "windows", target_vendor = "apple"))]
    #[test]
    fn render_png_returns_bytes() {
        let mut options = Options::default();
        options.paint.render_config.include_human_readable_text = true;
        let png = render_png("HELLO-123", Some(&options)).unwrap();
        assert!(png.len() > 8);
        assert_eq!(
            &png[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[cfg(any(target_os = "windows", target_vendor = "apple"))]
    #[test]
    fn render_png_for_symbology_accepts_string_input() {
        let mut options = Options::default();
        options.paint.render_config.include_human_readable_text = true;
        let png = render_png_for_symbology("ean-13", "4006381333931", Some(&options)).unwrap();
        assert!(png.len() > 8);
        assert_eq!(
            &png[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[cfg(all(not(target_os = "windows"), not(target_vendor = "apple")))]
    #[test]
    fn render_pixels_is_honest_when_backend_is_missing() {
        let err = render_pixels("HELLO-123", None).unwrap_err();
        assert!(err.contains("not wired"));
    }
}
