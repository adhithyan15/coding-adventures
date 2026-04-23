//! # upc-a
//!
//! UPC-A encoder that emits shared barcode runs and paint scenes.

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
pub struct EncodedDigit {
    pub digit: String,
    pub encoding: String,
    pub pattern: String,
    pub source_index: isize,
    pub role: Barcode1DRunRole,
}

const SIDE_GUARD: &str = "101";
const CENTER_GUARD: &str = "01010";
const LEFT_PATTERNS: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011",
    "0110111", "0001011",
];
const RIGHT_PATTERNS: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100",
    "1001000", "1110100",
];

fn assert_digits(data: &str, expected_lengths: &[usize]) -> Result<(), String> {
    if !data.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("UPC-A input must contain digits only".to_string());
    }
    if !expected_lengths.contains(&data.len()) {
        return Err("UPC-A input must contain 11 digits or 12 digits".to_string());
    }
    Ok(())
}

pub fn compute_upc_a_check_digit(payload11: &str) -> Result<String, String> {
    assert_digits(payload11, &[11])?;

    let mut odd_sum = 0u32;
    let mut even_sum = 0u32;
    for (index, digit) in payload11.chars().enumerate() {
        let value = digit.to_digit(10).unwrap();
        if index % 2 == 0 {
            odd_sum += value;
        } else {
            even_sum += value;
        }
    }

    Ok(((10 - ((odd_sum * 3 + even_sum) % 10)) % 10).to_string())
}

pub fn normalize_upc_a(data: &str) -> Result<String, String> {
    assert_digits(data, &[11, 12])?;

    if data.len() == 11 {
        return Ok(format!("{}{}", data, compute_upc_a_check_digit(data)?));
    }

    let expected = compute_upc_a_check_digit(&data[..11])?;
    let actual = &data[11..12];
    if expected != actual {
        return Err(format!(
            "invalid UPC-A check digit: expected {} but received {}",
            expected, actual
        ));
    }

    Ok(data.to_string())
}

pub fn encode_upc_a(data: &str) -> Result<Vec<EncodedDigit>, String> {
    let normalized = normalize_upc_a(data)?;
    Ok(normalized
        .chars()
        .enumerate()
        .map(|(index, digit)| EncodedDigit {
            digit: digit.to_string(),
            encoding: if index < 6 { "L" } else { "R" }.to_string(),
            pattern: if index < 6 {
                LEFT_PATTERNS[digit.to_digit(10).unwrap() as usize]
            } else {
                RIGHT_PATTERNS[digit.to_digit(10).unwrap() as usize]
            }
            .to_string(),
            source_index: index as isize,
            role: if index == 11 {
                Barcode1DRunRole::Check
            } else {
                Barcode1DRunRole::Data
            },
        })
        .collect())
}

fn build_symbols(encoded_digits: &[EncodedDigit]) -> Vec<Barcode1DSymbolDescriptor> {
    let mut symbols = vec![Barcode1DSymbolDescriptor {
        label: "start".to_string(),
        modules: 3,
        source_index: -1,
        role: Barcode1DSymbolRole::Guard,
    }];

    symbols.extend(
        encoded_digits[..6]
            .iter()
            .map(|entry| Barcode1DSymbolDescriptor {
                label: entry.digit.clone(),
                modules: 7,
                source_index: entry.source_index,
                role: if entry.role == Barcode1DRunRole::Check {
                    Barcode1DSymbolRole::Check
                } else {
                    Barcode1DSymbolRole::Data
                },
            }),
    );

    symbols.push(Barcode1DSymbolDescriptor {
        label: "center".to_string(),
        modules: 5,
        source_index: -2,
        role: Barcode1DSymbolRole::Guard,
    });

    symbols.extend(
        encoded_digits[6..]
            .iter()
            .map(|entry| Barcode1DSymbolDescriptor {
                label: entry.digit.clone(),
                modules: 7,
                source_index: entry.source_index,
                role: if entry.role == Barcode1DRunRole::Check {
                    Barcode1DSymbolRole::Check
                } else {
                    Barcode1DSymbolRole::Data
                },
            }),
    );

    symbols.push(Barcode1DSymbolDescriptor {
        label: "end".to_string(),
        modules: 3,
        source_index: -3,
        role: Barcode1DSymbolRole::Guard,
    });

    symbols
}

pub fn expand_upc_a_runs(data: &str) -> Result<Vec<Barcode1DRun>, String> {
    let encoded_digits = encode_upc_a(data)?;
    let mut runs = runs_from_binary_pattern(
        SIDE_GUARD,
        &RunsFromBinaryPatternOptions {
            source_label: "start".to_string(),
            source_index: -1,
            role: Barcode1DRunRole::Guard,
        },
    )?;

    for entry in &encoded_digits[..6] {
        runs.extend(runs_from_binary_pattern(
            &entry.pattern,
            &RunsFromBinaryPatternOptions {
                source_label: entry.digit.clone(),
                source_index: entry.source_index,
                role: entry.role.clone(),
            },
        )?);
    }

    runs.extend(runs_from_binary_pattern(
        CENTER_GUARD,
        &RunsFromBinaryPatternOptions {
            source_label: "center".to_string(),
            source_index: -2,
            role: Barcode1DRunRole::Guard,
        },
    )?);

    for entry in &encoded_digits[6..] {
        runs.extend(runs_from_binary_pattern(
            &entry.pattern,
            &RunsFromBinaryPatternOptions {
                source_label: entry.digit.clone(),
                source_index: entry.source_index,
                role: entry.role.clone(),
            },
        )?);
    }

    runs.extend(runs_from_binary_pattern(
        SIDE_GUARD,
        &RunsFromBinaryPatternOptions {
            source_label: "end".to_string(),
            source_index: -3,
            role: Barcode1DRunRole::Guard,
        },
    )?);

    Ok(runs)
}

pub fn layout_upc_a(data: &str, options: &PaintBarcode1DOptions) -> Result<PaintScene, String> {
    let normalized = normalize_upc_a(data)?;
    let encoded_digits = encode_upc_a(&normalized)?;
    let mut layout_options = options.clone();

    layout_options.symbols = Some(build_symbols(&encoded_digits));
    if layout_options.label.is_none() {
        layout_options.label = Some(format!("UPC-A barcode for {}", normalized));
    }
    if layout_options.human_readable_text.is_none() {
        layout_options.human_readable_text = Some(normalized.clone());
    }
    layout_options
        .metadata
        .insert("symbology".to_string(), "upc-a".to_string());

    layout_barcode_1d(&expand_upc_a_runs(&normalized)?, &layout_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_upc_check_digit() {
        assert_eq!(compute_upc_a_check_digit("03600029145").unwrap(), "2");
    }

    #[test]
    fn normalizes_and_builds_scene() {
        let normalized = normalize_upc_a("03600029145").unwrap();
        assert_eq!(normalized, "036000291452");

        let scene = layout_upc_a("03600029145", &PaintBarcode1DOptions::default()).unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"upc-a".to_string())
        );
    }
}
