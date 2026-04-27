//! # code39
//!
//! Dependency-free Code 39 encoder that emits shared barcode runs and paint
//! scenes.

pub const VERSION: &str = "0.1.0";

use barcode_layout_1d::{
    layout_barcode_1d, Barcode1DRenderConfig, Barcode1DRun, Barcode1DRunColor,
};
use paint_instructions::PaintScene;

pub use barcode_layout_1d::{
    Barcode1DLayout as Layout, Barcode1DRenderConfig as RenderConfig, Barcode1DRun as BarcodeRun,
    Barcode1DRunColor as RunColor, Barcode1DRunRole, Barcode1DSymbolDescriptor,
    Barcode1DSymbolLayout, Barcode1DSymbolRole, PaintBarcode1DOptions,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedCharacter {
    pub ch: String,
    pub is_start_stop: bool,
    pub pattern: String,
}

pub fn default_render_config() -> Barcode1DRenderConfig {
    Barcode1DRenderConfig::default()
}

const CODE39_PATTERNS: [(&str, &str); 44] = [
    ("0", "bwbWBwBwb"),
    ("1", "BwbWbwbwB"),
    ("2", "bwBWbwbwB"),
    ("3", "BwBWbwbwb"),
    ("4", "bwbWBwbwB"),
    ("5", "BwbWBwbwb"),
    ("6", "bwBWBwbwb"),
    ("7", "bwbWbwBwB"),
    ("8", "BwbWbwBwb"),
    ("9", "bwBWbwBwb"),
    ("A", "BwbwbWbwB"),
    ("B", "bwBwbWbwB"),
    ("C", "BwBwbWbwb"),
    ("D", "bwbwBWbwB"),
    ("E", "BwbwBWbwb"),
    ("F", "bwBwBWbwb"),
    ("G", "bwbwbWBwB"),
    ("H", "BwbwbWBwb"),
    ("I", "bwBwbWBwb"),
    ("J", "bwbwBWBwb"),
    ("K", "BwbwbwbWB"),
    ("L", "bwBwbwbWB"),
    ("M", "BwBwbwbWb"),
    ("N", "bwbwBwbWB"),
    ("O", "BwbwBwbWb"),
    ("P", "bwBwBwbWb"),
    ("Q", "bwbwbwBWB"),
    ("R", "BwbwbwBWb"),
    ("S", "bwBwbwBWb"),
    ("T", "bwbwBwBWb"),
    ("U", "BWbwbwbwB"),
    ("V", "bWBwbwbwB"),
    ("W", "BWBwbwbwb"),
    ("X", "bWbwBwbwB"),
    ("Y", "BWbwBwbwb"),
    ("Z", "bWBwBwbwb"),
    ("-", "bWbwbwBwB"),
    (".", "BWbwbwBwb"),
    (" ", "bWBwbwBwb"),
    ("$", "bWbWbWbwb"),
    ("/", "bWbWbwbWb"),
    ("+", "bWbwbWbWb"),
    ("%", "bwbWbWbWb"),
    ("*", "bWbwBwBwb"),
];

fn patterns(ch: &str) -> Option<&'static str> {
    CODE39_PATTERNS
        .iter()
        .find(|(value, _)| *value == ch)
        .map(|(_, pattern)| *pattern)
}

fn width_pattern(pattern: &str) -> String {
    pattern
        .chars()
        .map(|part| if part.is_uppercase() { 'W' } else { 'N' })
        .collect()
}

fn run_role_for(
    source_index: usize,
    encoded_len: usize,
    encoded_char: &EncodedCharacter,
) -> Barcode1DRunRole {
    if !encoded_char.is_start_stop {
        return Barcode1DRunRole::Data;
    }

    if source_index == 0 {
        Barcode1DRunRole::Start
    } else if source_index == encoded_len - 1 {
        Barcode1DRunRole::Stop
    } else {
        Barcode1DRunRole::Guard
    }
}

pub fn normalize_code39(data: &str) -> Result<String, String> {
    let normalized = data.to_uppercase();
    for ch in normalized.chars() {
        let value = ch.to_string();
        if value == "*" {
            return Err(
                "input must not contain \"*\" because it is reserved for start/stop".into(),
            );
        }
        if patterns(&value).is_none() {
            return Err(format!(
                "invalid character: {:?} is not supported by Code 39",
                value
            ));
        }
    }
    Ok(normalized)
}

pub fn encode_code39_char(ch: &str) -> Result<EncodedCharacter, String> {
    let pattern = patterns(ch)
        .ok_or_else(|| format!("invalid character: {:?} is not supported by Code 39", ch))?;
    Ok(EncodedCharacter {
        ch: ch.into(),
        is_start_stop: ch == "*",
        pattern: width_pattern(pattern),
    })
}

pub fn encode_code39(data: &str) -> Result<Vec<EncodedCharacter>, String> {
    let normalized = normalize_code39(data)?;
    ("*".to_owned() + &normalized + "*")
        .chars()
        .map(|ch| encode_code39_char(&ch.to_string()))
        .collect()
}

pub fn expand_code39_runs(data: &str) -> Result<Vec<Barcode1DRun>, String> {
    let encoded = encode_code39(data)?;
    let colors = [
        Barcode1DRunColor::Bar,
        Barcode1DRunColor::Space,
        Barcode1DRunColor::Bar,
        Barcode1DRunColor::Space,
        Barcode1DRunColor::Bar,
        Barcode1DRunColor::Space,
        Barcode1DRunColor::Bar,
        Barcode1DRunColor::Space,
        Barcode1DRunColor::Bar,
    ];

    let mut runs = Vec::new();
    for (source_index, encoded_char) in encoded.iter().enumerate() {
        let role = run_role_for(source_index, encoded.len(), encoded_char);

        for (element_index, element) in encoded_char.pattern.chars().enumerate() {
            runs.push(Barcode1DRun {
                color: colors[element_index].clone(),
                modules: if element == 'W' { 3 } else { 1 },
                source_label: encoded_char.ch.clone(),
                source_index: source_index as isize,
                role: role.clone(),
            });
        }

        if source_index < encoded.len() - 1 {
            runs.push(Barcode1DRun {
                color: Barcode1DRunColor::Space,
                modules: 1,
                source_label: encoded_char.ch.clone(),
                source_index: source_index as isize,
                role: Barcode1DRunRole::InterCharacterGap,
            });
        }
    }

    Ok(runs)
}

pub fn layout_code39(data: &str, options: &PaintBarcode1DOptions) -> Result<PaintScene, String> {
    let normalized = normalize_code39(data)?;
    let runs = expand_code39_runs(&normalized)?;
    let mut layout_options = options.clone();

    if layout_options.label.is_none() {
        layout_options.label = Some(if normalized.is_empty() {
            "Code 39 barcode".to_string()
        } else {
            format!("Code 39 barcode for {}", normalized)
        });
    }

    if layout_options.human_readable_text.is_none() {
        layout_options.human_readable_text = Some(normalized.clone());
    }

    layout_options
        .metadata
        .insert("symbology".to_string(), "code39".to_string());
    layout_options
        .metadata
        .insert("encodedText".to_string(), normalized.clone());

    layout_barcode_1d(&runs, &layout_options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::PaintInstruction;

    #[test]
    fn version_and_encode() {
        assert_eq!(VERSION, "0.1.0");
        let encoded = encode_code39_char("A").unwrap();
        assert_eq!(encoded.pattern, "WNNNNWNNW");
    }

    #[test]
    fn expands_runs_and_builds_paint_scene() {
        let runs = expand_code39_runs("A").unwrap();
        assert_eq!(runs.len(), 29);
        assert_eq!(runs[0].role, Barcode1DRunRole::Start);
        assert_eq!(runs[10].role, Barcode1DRunRole::Data);

        let scene = layout_code39("A", &PaintBarcode1DOptions::default()).unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"code39".to_string())
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("label")),
            Some(&"Code 39 barcode for A".to_string())
        );
    }

    #[test]
    fn default_render_config_is_paint_friendly() {
        let config = default_render_config();
        assert!(!config.include_human_readable_text);
        assert_eq!(config.module_width, 4.0);
        assert_eq!(config.bar_height, 120.0);
    }

    #[test]
    #[cfg(any(target_os = "windows", target_vendor = "apple"))]
    fn human_readable_text_emits_glyph_runs_when_enabled() {
        let mut options = PaintBarcode1DOptions::default();
        options.render_config.include_human_readable_text = true;
        let scene = layout_code39("OK", &options).unwrap();
        assert!(scene
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_))));
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("humanReadableText")),
            Some(&"OK".to_string())
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn human_readable_text_renders_to_direct2d_pixels() {
        let mut options = PaintBarcode1DOptions::default();
        options.render_config.include_human_readable_text = true;
        let scene = layout_code39("OK", &options).unwrap();
        let bar_height = options.render_config.bar_height as usize;

        let pixels = paint_vm_direct2d::render(&scene);
        let label_dark_pixels = pixels
            .data
            .chunks_exact(4)
            .enumerate()
            .filter(|(index, px)| {
                let y = index / pixels.width as usize;
                y >= bar_height && px[0] < 64 && px[1] < 64 && px[2] < 64 && px[3] > 0
            })
            .count();

        assert!(
            label_dark_pixels > 20,
            "expected visible text pixels below the bars"
        );
    }

    #[cfg(all(not(target_os = "windows"), not(target_vendor = "apple")))]
    #[test]
    fn human_readable_text_waits_for_platform_text_backend() {
        let mut options = PaintBarcode1DOptions::default();
        options.render_config.include_human_readable_text = true;
        let error = layout_code39("OK", &options).unwrap_err();
        assert!(error.contains("font resolution failed") || error.contains("LoadFailed"));
    }
}
