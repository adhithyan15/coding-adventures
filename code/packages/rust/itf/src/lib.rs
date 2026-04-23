//! # itf
//!
//! Interleaved 2 of 5 encoder that emits barcode runs and paint scenes through
//! `barcode-layout-1d`.

pub const VERSION: &str = "0.1.0";

use barcode_layout_1d::{
    layout_barcode_1d, runs_from_binary_pattern, Barcode1DRun, Barcode1DRunRole,
    Barcode1DSymbolDescriptor, Barcode1DSymbolRole, PaintBarcode1DOptions,
    RunsFromBinaryPatternOptions,
};
use paint_instructions::PaintScene;

pub use barcode_layout_1d::{
    Barcode1DLayout as Layout, Barcode1DRenderConfig as RenderConfig, Barcode1DSymbolLayout,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedPair {
    pub pair: String,
    pub bar_pattern: String,
    pub space_pattern: String,
    pub binary_pattern: String,
    pub source_index: isize,
}

const START_PATTERN: &str = "1010";
const STOP_PATTERN: &str = "11101";
const DIGIT_PATTERNS: [&str; 10] = [
    "00110", "10001", "01001", "11000", "00101", "10100", "01100", "00011", "10010", "01010",
];

pub fn normalize_itf(data: &str) -> Result<String, String> {
    if data.is_empty() || !data.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("ITF input must contain digits only".to_string());
    }

    if data.len() % 2 != 0 {
        return Err("ITF input must contain an even number of digits".to_string());
    }

    Ok(data.to_string())
}

fn encode_pair(pair: &str, source_index: isize) -> EncodedPair {
    let bar_pattern = DIGIT_PATTERNS[pair[0..1].parse::<usize>().unwrap()].to_string();
    let space_pattern = DIGIT_PATTERNS[pair[1..2].parse::<usize>().unwrap()].to_string();
    let binary_pattern = bar_pattern
        .chars()
        .zip(space_pattern.chars())
        .map(|(bar_marker, space_marker)| {
            format!(
                "{}{}",
                if bar_marker == '1' { "111" } else { "1" },
                if space_marker == '1' { "000" } else { "0" }
            )
        })
        .collect::<Vec<_>>()
        .join("");

    EncodedPair {
        pair: pair.to_string(),
        bar_pattern,
        space_pattern,
        binary_pattern,
        source_index,
    }
}

pub fn encode_itf(data: &str) -> Result<Vec<EncodedPair>, String> {
    let normalized = normalize_itf(data)?;
    Ok((0..normalized.len())
        .step_by(2)
        .enumerate()
        .map(|(index, start)| encode_pair(&normalized[start..start + 2], index as isize))
        .collect())
}

fn build_symbols(encoded_pairs: &[EncodedPair]) -> Vec<Barcode1DSymbolDescriptor> {
    let mut symbols = vec![Barcode1DSymbolDescriptor {
        label: "start".to_string(),
        modules: START_PATTERN.len() as u32,
        source_index: -1,
        role: Barcode1DSymbolRole::Start,
    }];

    symbols.extend(encoded_pairs.iter().map(|pair| Barcode1DSymbolDescriptor {
        label: pair.pair.clone(),
        modules: pair.binary_pattern.len() as u32,
        source_index: pair.source_index,
        role: Barcode1DSymbolRole::Data,
    }));

    symbols.push(Barcode1DSymbolDescriptor {
        label: "stop".to_string(),
        modules: STOP_PATTERN.len() as u32,
        source_index: -2,
        role: Barcode1DSymbolRole::Stop,
    });

    symbols
}

pub fn expand_itf_runs(data: &str) -> Result<Vec<Barcode1DRun>, String> {
    let encoded_pairs = encode_itf(data)?;
    let mut runs = runs_from_binary_pattern(
        START_PATTERN,
        &RunsFromBinaryPatternOptions {
            source_label: "start".to_string(),
            source_index: -1,
            role: Barcode1DRunRole::Start,
        },
    )?;

    for pair in &encoded_pairs {
        runs.extend(runs_from_binary_pattern(
            &pair.binary_pattern,
            &RunsFromBinaryPatternOptions {
                source_label: pair.pair.clone(),
                source_index: pair.source_index,
                role: Barcode1DRunRole::Data,
            },
        )?);
    }

    runs.extend(runs_from_binary_pattern(
        STOP_PATTERN,
        &RunsFromBinaryPatternOptions {
            source_label: "stop".to_string(),
            source_index: -2,
            role: Barcode1DRunRole::Stop,
        },
    )?);

    Ok(runs)
}

pub fn layout_itf(data: &str, options: &PaintBarcode1DOptions) -> Result<PaintScene, String> {
    let normalized = normalize_itf(data)?;
    let encoded_pairs = encode_itf(&normalized)?;
    let mut layout_options = options.clone();

    layout_options.symbols = Some(build_symbols(&encoded_pairs));
    if layout_options.label.is_none() {
        layout_options.label = Some(format!("ITF barcode for {}", normalized));
    }
    if layout_options.human_readable_text.is_none() {
        layout_options.human_readable_text = Some(normalized.clone());
    }
    layout_options
        .metadata
        .insert("symbology".to_string(), "itf".to_string());
    layout_options
        .metadata
        .insert("pairCount".to_string(), encoded_pairs.len().to_string());

    layout_barcode_1d(&expand_itf_runs(&normalized)?, &layout_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_odd_length_inputs() {
        let error = normalize_itf("123").unwrap_err();
        assert!(error.contains("even number of digits"));
    }

    #[test]
    fn builds_runs_and_paint_scene() {
        let runs = expand_itf_runs("123456").unwrap();
        assert!(!runs.is_empty());
        assert_eq!(runs[0].role, Barcode1DRunRole::Start);

        let scene = layout_itf("123456", &PaintBarcode1DOptions::default()).unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"itf".to_string())
        );
    }
}
