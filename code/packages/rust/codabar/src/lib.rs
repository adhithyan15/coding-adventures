//! # codabar
//!
//! Dependency-free Codabar encoder that emits shared barcode runs and paint
//! scenes through `barcode-layout-1d`.

pub const VERSION: &str = "0.1.0";

use barcode_layout_1d::{
    layout_barcode_1d, runs_from_binary_pattern, Barcode1DRun, Barcode1DRunColor, Barcode1DRunRole,
    PaintBarcode1DOptions, RunsFromBinaryPatternOptions,
};
use paint_instructions::PaintScene;

pub use barcode_layout_1d::{
    Barcode1DLayout as Layout, Barcode1DRenderConfig as RenderConfig, Barcode1DSymbolDescriptor,
    Barcode1DSymbolLayout, Barcode1DSymbolRole,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedCodabarSymbol {
    pub ch: String,
    pub pattern: String,
    pub source_index: isize,
    pub role: Barcode1DRunRole,
}

const GUARDS: [&str; 4] = ["A", "B", "C", "D"];
const PATTERNS: [(&str, &str); 20] = [
    ("0", "101010011"),
    ("1", "101011001"),
    ("2", "101001011"),
    ("3", "110010101"),
    ("4", "101101001"),
    ("5", "110101001"),
    ("6", "100101011"),
    ("7", "100101101"),
    ("8", "100110101"),
    ("9", "110100101"),
    ("-", "101001101"),
    ("$", "101100101"),
    (":", "1101011011"),
    ("/", "1101101011"),
    (".", "1101101101"),
    ("+", "1011011011"),
    ("A", "1011001001"),
    ("B", "1001001011"),
    ("C", "1010010011"),
    ("D", "1010011001"),
];

fn pattern_for(ch: &str) -> Option<&'static str> {
    PATTERNS
        .iter()
        .find(|(candidate, _)| *candidate == ch)
        .map(|(_, pattern)| *pattern)
}

fn is_guard(ch: &str) -> bool {
    GUARDS.iter().any(|guard| *guard == ch)
}

fn assert_body_chars(body: &str) -> Result<(), String> {
    for ch in body.chars() {
        let value = ch.to_string();
        if pattern_for(&value).is_none() || is_guard(&value) {
            return Err(format!("invalid Codabar body character {:?}", value));
        }
    }
    Ok(())
}

pub fn normalize_codabar(
    data: &str,
    start: Option<&str>,
    stop: Option<&str>,
) -> Result<String, String> {
    let normalized = data.to_uppercase();

    if normalized.len() >= 2 {
        let first = &normalized[0..1];
        let last = &normalized[normalized.len() - 1..];
        if is_guard(first) && is_guard(last) {
            assert_body_chars(&normalized[1..normalized.len() - 1])?;
            return Ok(normalized);
        }
    }

    assert_body_chars(&normalized)?;
    Ok(format!(
        "{}{}{}",
        start.unwrap_or("A"),
        normalized,
        stop.unwrap_or("A")
    ))
}

pub fn encode_codabar(
    data: &str,
    start: Option<&str>,
    stop: Option<&str>,
) -> Result<Vec<EncodedCodabarSymbol>, String> {
    let normalized = normalize_codabar(data, start, stop)?;
    normalized
        .chars()
        .enumerate()
        .map(|(index, ch)| {
            let value = ch.to_string();
            let role = if index == 0 {
                Barcode1DRunRole::Start
            } else if index == normalized.len() - 1 {
                Barcode1DRunRole::Stop
            } else {
                Barcode1DRunRole::Data
            };

            Ok(EncodedCodabarSymbol {
                ch: value.clone(),
                pattern: pattern_for(&value)
                    .ok_or_else(|| format!("invalid Codabar character {:?}", value))?
                    .to_string(),
                source_index: index as isize,
                role,
            })
        })
        .collect()
}

pub fn expand_codabar_runs(
    data: &str,
    start: Option<&str>,
    stop: Option<&str>,
) -> Result<Vec<Barcode1DRun>, String> {
    let encoded = encode_codabar(data, start, stop)?;
    let mut runs = Vec::new();

    for (index, entry) in encoded.iter().enumerate() {
        runs.extend(runs_from_binary_pattern(
            &entry.pattern,
            &RunsFromBinaryPatternOptions {
                source_label: entry.ch.clone(),
                source_index: entry.source_index,
                role: entry.role.clone(),
            },
        )?);

        if index < encoded.len() - 1 {
            runs.push(Barcode1DRun {
                color: Barcode1DRunColor::Space,
                modules: 1,
                source_label: entry.ch.clone(),
                source_index: entry.source_index,
                role: Barcode1DRunRole::InterCharacterGap,
            });
        }
    }

    Ok(runs)
}

pub fn layout_codabar(
    data: &str,
    start: Option<&str>,
    stop: Option<&str>,
    options: &PaintBarcode1DOptions,
) -> Result<PaintScene, String> {
    let normalized = normalize_codabar(data, start, stop)?;
    let mut layout_options = options.clone();

    if layout_options.label.is_none() {
        layout_options.label = Some(format!("Codabar barcode for {}", normalized));
    }
    if layout_options.human_readable_text.is_none() {
        layout_options.human_readable_text = Some(normalized.clone());
    }

    layout_options
        .metadata
        .insert("symbology".to_string(), "codabar".to_string());
    layout_options
        .metadata
        .insert("start".to_string(), normalized[0..1].to_string());
    layout_options.metadata.insert(
        "stop".to_string(),
        normalized[normalized.len() - 1..].to_string(),
    );

    layout_barcode_1d(
        &expand_codabar_runs(&normalized, None, None)?,
        &layout_options,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_expands_codabar() {
        let normalized = normalize_codabar("40156", Some("B"), Some("D")).unwrap();
        assert_eq!(normalized, "B40156D");

        let runs = expand_codabar_runs("40156", Some("B"), Some("D")).unwrap();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].role, Barcode1DRunRole::Start);
    }

    #[test]
    fn builds_paint_scene() {
        let scene = layout_codabar(
            "40156",
            Some("B"),
            Some("D"),
            &PaintBarcode1DOptions::default(),
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
            Some(&"D".to_string())
        );
    }
}
