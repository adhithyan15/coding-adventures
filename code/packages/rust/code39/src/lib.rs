//! # code39
//!
//! Dependency-free Code 39 encoder that emits backend-neutral draw scenes.

use draw_instructions::{create_scene, draw_rect, draw_text, DrawScene, Metadata, Renderer};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedCharacter {
    pub ch: String,
    pub is_start_stop: bool,
    pub pattern: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarcodeRun {
    pub color: String,
    pub width: String,
    pub source_char: String,
    pub source_index: usize,
    pub is_inter_character_gap: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderConfig {
    pub narrow_unit: i32,
    pub wide_unit: i32,
    pub bar_height: i32,
    pub quiet_zone_units: i32,
    pub include_human_readable_text: bool,
}

pub fn default_render_config() -> RenderConfig {
    RenderConfig { narrow_unit: 4, wide_unit: 12, bar_height: 120, quiet_zone_units: 10, include_human_readable_text: true }
}

const TEXT_MARGIN: i32 = 8;
const TEXT_FONT_SIZE: i32 = 16;
const TEXT_BLOCK_HEIGHT: i32 = TEXT_MARGIN + TEXT_FONT_SIZE + 4;

fn patterns(ch: &str) -> Option<&'static str> {
    match ch {
        "0" => Some("bwbWBwBwb"), "1" => Some("BwbWbwbwB"), "2" => Some("bwBWbwbwB"), "3" => Some("BwBWbwbwb"),
        "4" => Some("bwbWBwbwB"), "5" => Some("BwbWBwbwb"), "6" => Some("bwBWBwbwb"), "7" => Some("bwbWbwBwB"),
        "8" => Some("BwbWbwBwb"), "9" => Some("bwBWbwBwb"), "A" => Some("BwbwbWbwB"), "B" => Some("bwBwbWbwB"),
        "C" => Some("BwBwbWbwb"), "D" => Some("bwbwBWbwB"), "E" => Some("BwbwBWbwb"), "F" => Some("bwBwBWbwb"),
        "G" => Some("bwbwbWBwB"), "H" => Some("BwbwbWBwb"), "I" => Some("bwBwbWBwb"), "J" => Some("bwbwBWBwb"),
        "K" => Some("BwbwbwbWB"), "L" => Some("bwBwbwbWB"), "M" => Some("BwBwbwbWb"), "N" => Some("bwbwBwbWB"),
        "O" => Some("BwbwBwbWb"), "P" => Some("bwBwBwbWb"), "Q" => Some("bwbwbwBWB"), "R" => Some("BwbwbwBWb"),
        "S" => Some("bwBwbwBWb"), "T" => Some("bwbwBwBWb"), "U" => Some("BWbwbwbwB"), "V" => Some("bWBwbwbwB"),
        "W" => Some("BWBwbwbwb"), "X" => Some("bWbwBwbwB"), "Y" => Some("BWbwBwbwb"), "Z" => Some("bWBwBwbwb"),
        "-" => Some("bWbwbwBwB"), "." => Some("BWbwbwBwb"), " " => Some("bWBwbwBwb"), "$" => Some("bWbWbWbwb"),
        "/" => Some("bWbWbwbWb"), "+" => Some("bWbwbWbWb"), "%" => Some("bwbWbWbWb"), "*" => Some("bWbwBwBwb"),
        _ => None,
    }
}

fn width_pattern(pattern: &str) -> String {
    pattern.chars().map(|part| if part.is_uppercase() { 'W' } else { 'N' }).collect()
}

pub fn normalize_code39(data: &str) -> Result<String, String> {
    let normalized = data.to_uppercase();
    for ch in normalized.chars() {
        let value = ch.to_string();
        if value == "*" {
            return Err("input must not contain \"*\" because it is reserved for start/stop".into());
        }
        if patterns(&value).is_none() {
            return Err(format!("invalid character: {:?} is not supported by Code 39", value));
        }
    }
    Ok(normalized)
}

pub fn encode_code39_char(ch: &str) -> Result<EncodedCharacter, String> {
    let pattern = patterns(ch).ok_or_else(|| format!("invalid character: {:?} is not supported by Code 39", ch))?;
    Ok(EncodedCharacter { ch: ch.into(), is_start_stop: ch == "*", pattern: width_pattern(pattern) })
}

pub fn encode_code39(data: &str) -> Result<Vec<EncodedCharacter>, String> {
    let normalized = normalize_code39(data)?;
    ("*".to_owned() + &normalized + "*")
        .chars()
        .map(|ch| encode_code39_char(&ch.to_string()))
        .collect()
}

pub fn expand_code39_runs(data: &str) -> Result<Vec<BarcodeRun>, String> {
    let encoded = encode_code39(data)?;
    let colors = ["bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"];
    let mut runs = Vec::new();
    for (source_index, encoded_char) in encoded.iter().enumerate() {
        for (element_index, element) in encoded_char.pattern.chars().enumerate() {
            runs.push(BarcodeRun {
                color: colors[element_index].into(),
                width: if element == 'W' { "wide".into() } else { "narrow".into() },
                source_char: encoded_char.ch.clone(),
                source_index,
                is_inter_character_gap: false,
            });
        }
        if source_index < encoded.len() - 1 {
            runs.push(BarcodeRun {
                color: "space".into(),
                width: "narrow".into(),
                source_char: encoded_char.ch.clone(),
                source_index,
                is_inter_character_gap: true,
            });
        }
    }
    Ok(runs)
}

pub fn draw_code39(data: &str, config: &RenderConfig) -> Result<DrawScene, String> {
    if config.wide_unit <= config.narrow_unit || config.narrow_unit <= 0 || config.bar_height <= 0 || config.quiet_zone_units <= 0 {
        return Err("invalid render config".into());
    }
    let normalized = normalize_code39(data)?;
    let quiet_zone_width = config.quiet_zone_units * config.narrow_unit;
    let runs = expand_code39_runs(&normalized)?;
    let mut instructions = Vec::new();
    let mut cursor_x = quiet_zone_width;
    for run in runs {
        let width = if run.width == "wide" { config.wide_unit } else { config.narrow_unit };
        if run.color == "bar" {
            let mut metadata = Metadata::new();
            metadata.insert("char".into(), run.source_char.clone());
            metadata.insert("index".into(), run.source_index.to_string());
            instructions.push(draw_rect(cursor_x, 0, width, config.bar_height, "#000000", metadata));
        }
        cursor_x += width;
    }
    if config.include_human_readable_text {
        let mut metadata = Metadata::new();
        metadata.insert("role".into(), "label".into());
        instructions.push(draw_text((cursor_x + quiet_zone_width) / 2, config.bar_height + TEXT_MARGIN + TEXT_FONT_SIZE - 2, &normalized, metadata));
    }
    let mut scene_meta = Metadata::new();
    scene_meta.insert("label".into(), format!("Code 39 barcode for {}", normalized));
    scene_meta.insert("symbology".into(), "code39".into());
    Ok(create_scene(cursor_x + quiet_zone_width, config.bar_height + if config.include_human_readable_text { TEXT_BLOCK_HEIGHT } else { 0 }, instructions, "", scene_meta))
}

pub fn render_code39<T>(data: &str, renderer: &impl Renderer<T>, config: &RenderConfig) -> Result<T, String> {
    let scene = draw_code39(data, config)?;
    Ok(renderer.render(&scene))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRenderer;
    impl Renderer<String> for TestRenderer {
        fn render(&self, scene: &DrawScene) -> String {
            format!("{}:{}", scene.width, scene.instructions.len())
        }
    }

    #[test]
    fn version_and_encode() {
        assert_eq!(VERSION, "0.1.0");
        let encoded = encode_code39_char("A").unwrap();
        assert_eq!(encoded.pattern, "WNNNNWNNW");
    }

    #[test]
    fn expands_runs_and_draws_scene() {
        let runs = expand_code39_runs("A").unwrap();
        assert_eq!(runs.len(), 29);
        let scene = draw_code39("A", &default_render_config()).unwrap();
        assert_eq!(scene.metadata.get("symbology").unwrap(), "code39");
    }

    #[test]
    fn renders_with_backend() {
        let output = render_code39("OK", &TestRenderer, &default_render_config()).unwrap();
        assert!(output.contains(':'));
    }
}
