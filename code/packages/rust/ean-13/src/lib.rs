//! # ean-13
//!
//! EAN-13 encoder that emits barcode runs and paint scenes.

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
const G_PATTERNS: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001",
    "0001001", "0010111",
];
const RIGHT_PATTERNS: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100",
    "1001000", "1110100",
];
const LEFT_PARITY_PATTERNS: [&str; 10] = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL",
    "LGGLGL",
];

fn assert_digits(data: &str, expected_lengths: &[usize]) -> Result<(), String> {
    if !data.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("EAN-13 input must contain digits only".to_string());
    }
    if !expected_lengths.contains(&data.len()) {
        return Err("EAN-13 input must contain 12 digits or 13 digits".to_string());
    }
    Ok(())
}

pub fn compute_ean13_check_digit(payload12: &str) -> Result<String, String> {
    assert_digits(payload12, &[12])?;

    let total = payload12
        .chars()
        .rev()
        .enumerate()
        .map(|(index, digit)| digit.to_digit(10).unwrap() * if index % 2 == 0 { 3 } else { 1 })
        .sum::<u32>();

    Ok(((10 - (total % 10)) % 10).to_string())
}

pub fn normalize_ean13(data: &str) -> Result<String, String> {
    assert_digits(data, &[12, 13])?;

    if data.len() == 12 {
        return Ok(format!("{}{}", data, compute_ean13_check_digit(data)?));
    }

    let expected = compute_ean13_check_digit(&data[..12])?;
    let actual = &data[12..13];
    if expected != actual {
        return Err(format!(
            "invalid EAN-13 check digit: expected {} but received {}",
            expected, actual
        ));
    }

    Ok(data.to_string())
}

pub fn left_parity_pattern(data: &str) -> Result<String, String> {
    let normalized = normalize_ean13(data)?;
    Ok(LEFT_PARITY_PATTERNS[normalized[0..1].parse::<usize>().unwrap()].to_string())
}

pub fn encode_ean13(data: &str) -> Result<Vec<EncodedDigit>, String> {
    let normalized = normalize_ean13(data)?;
    let parity = LEFT_PARITY_PATTERNS[normalized[0..1].parse::<usize>().unwrap()];
    let digits: Vec<char> = normalized.chars().collect();

    let mut encoded = Vec::new();
    for offset in 0..6 {
        let digit = digits[offset + 1];
        let encoding = parity[offset..offset + 1].to_string();
        let pattern = if &encoding == "L" {
            LEFT_PATTERNS[digit.to_digit(10).unwrap() as usize]
        } else {
            G_PATTERNS[digit.to_digit(10).unwrap() as usize]
        };
        encoded.push(EncodedDigit {
            digit: digit.to_string(),
            encoding,
            pattern: pattern.to_string(),
            source_index: (offset + 1) as isize,
            role: Barcode1DRunRole::Data,
        });
    }

    for offset in 0..6 {
        let digit = digits[offset + 7];
        encoded.push(EncodedDigit {
            digit: digit.to_string(),
            encoding: "R".to_string(),
            pattern: RIGHT_PATTERNS[digit.to_digit(10).unwrap() as usize].to_string(),
            source_index: (offset + 7) as isize,
            role: if offset == 5 {
                Barcode1DRunRole::Check
            } else {
                Barcode1DRunRole::Data
            },
        });
    }

    Ok(encoded)
}

fn build_symbols(
    normalized: &str,
    encoded_digits: &[EncodedDigit],
) -> Vec<Barcode1DSymbolDescriptor> {
    let mut symbols = vec![Barcode1DSymbolDescriptor {
        label: normalized[0..1].to_string(),
        modules: 0,
        source_index: 0,
        role: Barcode1DSymbolRole::Data,
    }];

    symbols.push(Barcode1DSymbolDescriptor {
        label: "start".to_string(),
        modules: 3,
        source_index: -1,
        role: Barcode1DSymbolRole::Guard,
    });

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
        .into_iter()
        .filter(|symbol| symbol.modules > 0)
        .collect()
}

pub fn expand_ean13_runs(data: &str) -> Result<Vec<Barcode1DRun>, String> {
    let encoded_digits = encode_ean13(data)?;
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

pub fn layout_ean13(data: &str, options: &PaintBarcode1DOptions) -> Result<PaintScene, String> {
    let normalized = normalize_ean13(data)?;
    let encoded_digits = encode_ean13(&normalized)?;
    let mut layout_options = options.clone();

    layout_options.symbols = Some(build_symbols(&normalized, &encoded_digits));
    if layout_options.label.is_none() {
        layout_options.label = Some(format!("EAN-13 barcode for {}", normalized));
    }
    if layout_options.human_readable_text.is_none() {
        layout_options.human_readable_text = Some(normalized.clone());
    }
    layout_options
        .metadata
        .insert("symbology".to_string(), "ean-13".to_string());
    layout_options
        .metadata
        .insert("leadingDigit".to_string(), normalized[0..1].to_string());
    layout_options.metadata.insert(
        "leftParity".to_string(),
        LEFT_PARITY_PATTERNS[normalized[0..1].parse::<usize>().unwrap()].to_string(),
    );

    layout_barcode_1d(&expand_ean13_runs(&normalized)?, &layout_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_check_digit_and_layout() {
        assert_eq!(compute_ean13_check_digit("400638133393").unwrap(), "1");
        assert_eq!(left_parity_pattern("4006381333931").unwrap(), "LGLLGG");

        let scene = layout_ean13("400638133393", &PaintBarcode1DOptions::default()).unwrap();
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("symbology")),
            Some(&"ean-13".to_string())
        );
    }
}
