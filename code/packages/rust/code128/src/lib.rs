//! # code128
//!
//! Code 128 Code Set B encoder that emits barcode runs and paint scenes.

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
pub struct EncodedCode128Symbol {
    pub label: String,
    pub value: usize,
    pub pattern: String,
    pub source_index: isize,
    pub role: Barcode1DRunRole,
}

const START_B: usize = 104;
const STOP: usize = 106;
const PATTERNS: &[&str] = &[
    "11011001100",
    "11001101100",
    "11001100110",
    "10010011000",
    "10010001100",
    "10001001100",
    "10011001000",
    "10011000100",
    "10001100100",
    "11001001000",
    "11001000100",
    "11000100100",
    "10110011100",
    "10011011100",
    "10011001110",
    "10111001100",
    "10011101100",
    "10011100110",
    "11001110010",
    "11001011100",
    "11001001110",
    "11011100100",
    "11001110100",
    "11101101110",
    "11101001100",
    "11100101100",
    "11100100110",
    "11101100100",
    "11100110100",
    "11100110010",
    "11011011000",
    "11011000110",
    "11000110110",
    "10100011000",
    "10001011000",
    "10001000110",
    "10110001000",
    "10001101000",
    "10001100010",
    "11010001000",
    "11000101000",
    "11000100010",
    "10110111000",
    "10110001110",
    "10001101110",
    "10111011000",
    "10111000110",
    "10001110110",
    "11101110110",
    "11010001110",
    "11000101110",
    "11011101000",
    "11011100010",
    "11011101110",
    "11101011000",
    "11101000110",
    "11100010110",
    "11101101000",
    "11101100010",
    "11100011010",
    "11101111010",
    "11001000010",
    "11110001010",
    "10100110000",
    "10100001100",
    "10010110000",
    "10010000110",
    "10000101100",
    "10000100110",
    "10110010000",
    "10110000100",
    "10011010000",
    "10011000010",
    "10000110100",
    "10000110010",
    "11000010010",
    "11001010000",
    "11110111010",
    "11000010100",
    "10001111010",
    "10100111100",
    "10010111100",
    "10010011110",
    "10111100100",
    "10011110100",
    "10011110010",
    "11110100100",
    "11110010100",
    "11110010010",
    "11011011110",
    "11011110110",
    "11110110110",
    "10101111000",
    "10100011110",
    "10001011110",
    "10111101000",
    "10111100010",
    "11110101000",
    "11110100010",
    "10111011110",
    "10111101110",
    "11101011110",
    "11110101110",
    "11010000100",
    "11010010000",
    "11010011100",
    "1100011101011",
];

pub fn normalize_code128_b(data: &str) -> Result<String, String> {
    for ch in data.chars() {
        let code = ch as u32;
        if !(32..=126).contains(&code) {
            return Err("Code 128 Code Set B supports printable ASCII characters only".to_string());
        }
    }
    Ok(data.to_string())
}

fn value_for_code128_b_char(ch: char) -> usize {
    ch as usize - 32
}

pub fn compute_code128_checksum(values: &[usize]) -> usize {
    values
        .iter()
        .enumerate()
        .fold(START_B, |sum, (index, value)| sum + value * (index + 1))
        % 103
}

pub fn encode_code128_b(data: &str) -> Result<Vec<EncodedCode128Symbol>, String> {
    let normalized = normalize_code128_b(data)?;
    let data_symbols: Vec<EncodedCode128Symbol> = normalized
        .chars()
        .enumerate()
        .map(|(index, ch)| {
            let value = value_for_code128_b_char(ch);
            EncodedCode128Symbol {
                label: ch.to_string(),
                value,
                pattern: PATTERNS[value].to_string(),
                source_index: index as isize,
                role: Barcode1DRunRole::Data,
            }
        })
        .collect();
    let checksum = compute_code128_checksum(
        &data_symbols
            .iter()
            .map(|symbol| symbol.value)
            .collect::<Vec<_>>(),
    );

    let mut encoded = vec![EncodedCode128Symbol {
        label: "Start B".to_string(),
        value: START_B,
        pattern: PATTERNS[START_B].to_string(),
        source_index: -1,
        role: Barcode1DRunRole::Start,
    }];

    encoded.extend(data_symbols);
    encoded.push(EncodedCode128Symbol {
        label: format!("Checksum {}", checksum),
        value: checksum,
        pattern: PATTERNS[checksum].to_string(),
        source_index: normalized.len() as isize,
        role: Barcode1DRunRole::Check,
    });
    encoded.push(EncodedCode128Symbol {
        label: "Stop".to_string(),
        value: STOP,
        pattern: PATTERNS[STOP].to_string(),
        source_index: normalized.len() as isize + 1,
        role: Barcode1DRunRole::Stop,
    });

    Ok(encoded)
}

fn build_symbols(encoded: &[EncodedCode128Symbol]) -> Vec<Barcode1DSymbolDescriptor> {
    encoded
        .iter()
        .map(|entry| Barcode1DSymbolDescriptor {
            label: entry.label.clone(),
            modules: if entry.role == Barcode1DRunRole::Stop {
                13
            } else {
                11
            },
            source_index: entry.source_index,
            role: match entry.role {
                Barcode1DRunRole::Start => Barcode1DSymbolRole::Start,
                Barcode1DRunRole::Check => Barcode1DSymbolRole::Check,
                Barcode1DRunRole::Stop => Barcode1DSymbolRole::Stop,
                _ => Barcode1DSymbolRole::Data,
            },
        })
        .collect()
}

pub fn expand_code128_runs(data: &str) -> Result<Vec<Barcode1DRun>, String> {
    let encoded = encode_code128_b(data)?;
    let mut runs = Vec::new();

    for entry in &encoded {
        runs.extend(runs_from_binary_pattern(
            &entry.pattern,
            &RunsFromBinaryPatternOptions {
                source_label: entry.label.clone(),
                source_index: entry.source_index,
                role: entry.role.clone(),
            },
        )?);
    }

    Ok(runs)
}

pub fn layout_code128(data: &str, options: &PaintBarcode1DOptions) -> Result<PaintScene, String> {
    let normalized = normalize_code128_b(data)?;
    let encoded = encode_code128_b(&normalized)?;
    let checksum = encoded[encoded.len() - 2].value;
    let mut layout_options = options.clone();

    layout_options.symbols = Some(build_symbols(&encoded));
    if layout_options.label.is_none() {
        layout_options.label = Some(format!("Code 128 barcode for {}", normalized));
    }
    if layout_options.human_readable_text.is_none() {
        layout_options.human_readable_text = Some(normalized.clone());
    }
    layout_options
        .metadata
        .insert("symbology".to_string(), "code128".to_string());
    layout_options
        .metadata
        .insert("codeSet".to_string(), "B".to_string());
    layout_options
        .metadata
        .insert("checksum".to_string(), checksum.to_string());

    layout_barcode_1d(&expand_code128_runs(&normalized)?, &layout_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_checksum_and_builds_scene() {
        let encoded = encode_code128_b("HELLO").unwrap();
        assert_eq!(encoded[0].role, Barcode1DRunRole::Start);
        assert_eq!(encoded[encoded.len() - 1].role, Barcode1DRunRole::Stop);

        let scene = layout_code128("HELLO", &PaintBarcode1DOptions::default()).unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"code128".to_string())
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("codeSet")),
            Some(&"B".to_string())
        );
    }
}
