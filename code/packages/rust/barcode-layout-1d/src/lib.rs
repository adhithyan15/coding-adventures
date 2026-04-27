//! # barcode-layout-1d
//!
//! Shared layout crate for linear barcode symbologies in Rust.
//!
//! This crate owns the reusable seam between symbology logic and the paint
//! pipeline:
//!
//! ```text
//! barcode package
//!   -> Barcode1DRun[]
//!   -> compute_barcode_1d_layout()
//!   -> layout_barcode_1d()
//!   -> PaintScene
//!   -> paint-metal / paint-vm-direct2d / paint-vm-gdi
//!   -> PixelContainer
//!   -> paint-codec-png
//! ```

pub const VERSION: &str = "0.1.0";

use paint_instructions::{
    GlyphPosition, PaintBase, PaintGlyphRun, PaintInstruction, PaintRect, PaintScene,
};
use std::collections::HashMap;
use text_interfaces::{
    FontMetrics, FontQuery, FontResolver, FontStretch, FontStyle, FontWeight, ShapeOptions,
    TextShaper,
};
use text_native::{NativeMetrics, NativeResolver, NativeShaper};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Barcode1DRunColor {
    Bar,
    Space,
}

impl Barcode1DRunColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Space => "space",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Barcode1DRunRole {
    Data,
    Start,
    Stop,
    Guard,
    Check,
    InterCharacterGap,
}

impl Barcode1DRunRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Guard => "guard",
            Self::Check => "check",
            Self::InterCharacterGap => "inter-character-gap",
        }
    }

    fn symbol_role(&self) -> Option<Barcode1DSymbolRole> {
        match self {
            Self::Data => Some(Barcode1DSymbolRole::Data),
            Self::Start => Some(Barcode1DSymbolRole::Start),
            Self::Stop => Some(Barcode1DSymbolRole::Stop),
            Self::Guard => Some(Barcode1DSymbolRole::Guard),
            Self::Check => Some(Barcode1DSymbolRole::Check),
            Self::InterCharacterGap => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Barcode1DSymbolRole {
    Data,
    Start,
    Stop,
    Guard,
    Check,
}

impl Barcode1DSymbolRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Guard => "guard",
            Self::Check => "check",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Barcode1DRun {
    pub color: Barcode1DRunColor,
    pub modules: u32,
    pub source_label: String,
    pub source_index: isize,
    pub role: Barcode1DRunRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Barcode1DSymbolLayout {
    pub label: String,
    pub start_module: u32,
    pub end_module: u32,
    pub source_index: isize,
    pub role: Barcode1DSymbolRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Barcode1DLayout {
    pub left_quiet_zone_modules: u32,
    pub right_quiet_zone_modules: u32,
    pub content_modules: u32,
    pub total_modules: u32,
    pub symbol_layouts: Vec<Barcode1DSymbolLayout>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Barcode1DSymbolDescriptor {
    pub label: String,
    pub modules: u32,
    pub source_index: isize,
    pub role: Barcode1DSymbolRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Barcode1DLayoutTarget {
    NativePaintVm,
    CanvasPaintVm,
    DomPaintVm,
}

impl Barcode1DLayoutTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NativePaintVm => "native-paint-vm",
            Self::CanvasPaintVm => "canvas-paint-vm",
            Self::DomPaintVm => "dom-paint-vm",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Barcode1DRenderConfig {
    pub layout_target: Barcode1DLayoutTarget,
    pub module_width: f64,
    pub bar_height: f64,
    pub quiet_zone_modules: u32,
    pub include_human_readable_text: bool,
    pub text_font_size: f64,
    pub text_margin: f64,
    pub foreground: String,
    pub background: String,
}

impl Default for Barcode1DRenderConfig {
    fn default() -> Self {
        Self {
            layout_target: Barcode1DLayoutTarget::NativePaintVm,
            module_width: 4.0,
            bar_height: 120.0,
            quiet_zone_modules: 10,
            include_human_readable_text: false,
            text_font_size: 16.0,
            text_margin: 8.0,
            foreground: "#000000".to_string(),
            background: "#ffffff".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintBarcode1DOptions {
    pub render_config: Barcode1DRenderConfig,
    pub human_readable_text: Option<String>,
    pub metadata: HashMap<String, String>,
    pub label: Option<String>,
    pub symbols: Option<Vec<Barcode1DSymbolDescriptor>>,
}

impl Default for PaintBarcode1DOptions {
    fn default() -> Self {
        Self {
            render_config: Barcode1DRenderConfig::default(),
            human_readable_text: None,
            metadata: HashMap::new(),
            label: None,
            symbols: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunsFromBinaryPatternOptions {
    pub source_label: String,
    pub source_index: isize,
    pub role: Barcode1DRunRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunsFromWidthPatternOptions {
    pub source_label: String,
    pub source_index: isize,
    pub role: Barcode1DRunRole,
    pub narrow_modules: u32,
    pub wide_modules: u32,
    pub narrow_marker: char,
    pub wide_marker: char,
    pub starting_color: Barcode1DRunColor,
}

impl RunsFromWidthPatternOptions {
    pub fn new(source_label: &str, source_index: isize, role: Barcode1DRunRole) -> Self {
        Self {
            source_label: source_label.to_string(),
            source_index,
            role,
            narrow_modules: 1,
            wide_modules: 3,
            narrow_marker: 'N',
            wide_marker: 'W',
            starting_color: Barcode1DRunColor::Bar,
        }
    }
}

fn assert_positive_number(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name} must be a positive number"));
    }
    Ok(())
}

fn validate_render_config(config: &Barcode1DRenderConfig) -> Result<(), String> {
    assert_positive_number(config.module_width, "module_width")?;
    assert_positive_number(config.bar_height, "bar_height")?;
    assert_positive_number(config.text_font_size, "text_font_size")?;

    if config.quiet_zone_modules == 0 {
        return Err("quiet_zone_modules must be greater than zero".to_string());
    }

    if !config.text_margin.is_finite() || config.text_margin < 0.0 {
        return Err("text_margin must be zero or greater".to_string());
    }

    Ok(())
}

fn validate_runs(runs: &[Barcode1DRun]) -> Result<(), String> {
    for (index, run) in runs.iter().enumerate() {
        if run.modules == 0 {
            return Err(format!("runs[{index}].modules must be greater than zero"));
        }

        if index > 0 && runs[index - 1].color == run.color {
            return Err("runs must alternate between bars and spaces".to_string());
        }
    }

    Ok(())
}

pub fn total_modules(runs: &[Barcode1DRun]) -> u32 {
    runs.iter().map(|run| run.modules).sum()
}

pub fn compute_barcode_1d_layout(
    runs: &[Barcode1DRun],
    quiet_zone_modules: u32,
    symbols: Option<&[Barcode1DSymbolDescriptor]>,
) -> Result<Barcode1DLayout, String> {
    validate_runs(runs)?;

    if quiet_zone_modules == 0 {
        return Err("quiet_zone_modules must be greater than zero".to_string());
    }

    let content_modules = total_modules(runs);
    let mut symbol_layouts = Vec::new();

    if let Some(symbols) = symbols {
        let mut cursor = 0;

        for symbol in symbols {
            if symbol.modules == 0 {
                return Err(format!(
                    "symbol \"{}\" modules must be greater than zero",
                    symbol.label
                ));
            }

            symbol_layouts.push(Barcode1DSymbolLayout {
                label: symbol.label.clone(),
                start_module: cursor,
                end_module: cursor + symbol.modules,
                source_index: symbol.source_index,
                role: symbol.role.clone(),
            });
            cursor += symbol.modules;
        }

        if cursor != content_modules {
            return Err(
                "symbol descriptors must add up to the same total width as the run stream"
                    .to_string(),
            );
        }
    } else {
        let mut cursor = 0;
        let mut current_start = 0;
        let mut current_label: Option<String> = None;
        let mut current_source_index = 0;
        let mut current_role: Option<Barcode1DSymbolRole> = None;

        for run in runs {
            if let Some(symbol_role) = run.role.symbol_role() {
                let is_same_symbol = current_label.as_deref() == Some(run.source_label.as_str())
                    && current_source_index == run.source_index
                    && current_role.as_ref() == Some(&symbol_role);

                if !is_same_symbol {
                    if let (Some(label), Some(role)) =
                        (current_label.as_ref(), current_role.as_ref())
                    {
                        symbol_layouts.push(Barcode1DSymbolLayout {
                            label: label.clone(),
                            start_module: current_start,
                            end_module: cursor,
                            source_index: current_source_index,
                            role: role.clone(),
                        });
                    }
                    current_start = cursor;
                    current_label = Some(run.source_label.clone());
                    current_source_index = run.source_index;
                    current_role = Some(symbol_role);
                }
            }

            cursor += run.modules;
        }

        if let (Some(label), Some(role)) = (current_label.as_ref(), current_role.as_ref()) {
            symbol_layouts.push(Barcode1DSymbolLayout {
                label: label.clone(),
                start_module: current_start,
                end_module: cursor,
                source_index: current_source_index,
                role: role.clone(),
            });
        }
    }

    Ok(Barcode1DLayout {
        left_quiet_zone_modules: quiet_zone_modules,
        right_quiet_zone_modules: quiet_zone_modules,
        content_modules,
        total_modules: quiet_zone_modules + content_modules + quiet_zone_modules,
        symbol_layouts,
    })
}

pub fn runs_from_binary_pattern(
    pattern: &str,
    options: &RunsFromBinaryPatternOptions,
) -> Result<Vec<Barcode1DRun>, String> {
    if pattern.is_empty() {
        return Err("binary pattern must not be empty".to_string());
    }

    if !pattern.chars().all(|bit| bit == '0' || bit == '1') {
        return Err(format!(
            "binary pattern must contain only 0 or 1, got \"{pattern}\""
        ));
    }

    let mut runs = Vec::new();
    let mut chars = pattern.chars();
    let mut current_bit = chars.next().expect("pattern was checked to be non-empty");
    let mut width = 1u32;

    for bit in chars {
        if bit == current_bit {
            width += 1;
            continue;
        }

        runs.push(Barcode1DRun {
            color: if current_bit == '1' {
                Barcode1DRunColor::Bar
            } else {
                Barcode1DRunColor::Space
            },
            modules: width,
            source_label: options.source_label.clone(),
            source_index: options.source_index,
            role: options.role.clone(),
        });

        current_bit = bit;
        width = 1;
    }

    runs.push(Barcode1DRun {
        color: if current_bit == '1' {
            Barcode1DRunColor::Bar
        } else {
            Barcode1DRunColor::Space
        },
        modules: width,
        source_label: options.source_label.clone(),
        source_index: options.source_index,
        role: options.role.clone(),
    });

    Ok(runs)
}

pub fn runs_from_width_pattern(
    pattern: &str,
    options: &RunsFromWidthPatternOptions,
) -> Result<Vec<Barcode1DRun>, String> {
    if pattern.is_empty() {
        return Err("width pattern must not be empty".to_string());
    }

    if options.narrow_modules == 0 {
        return Err("narrow_modules must be greater than zero".to_string());
    }

    if options.wide_modules == 0 {
        return Err("wide_modules must be greater than zero".to_string());
    }

    let mut runs = Vec::new();
    let mut color = options.starting_color.clone();

    for marker in pattern.chars() {
        let modules = if marker == options.narrow_marker {
            options.narrow_modules
        } else if marker == options.wide_marker {
            options.wide_modules
        } else {
            return Err(format!(
                "unknown width marker \"{marker}\" in pattern \"{pattern}\""
            ));
        };

        runs.push(Barcode1DRun {
            color: color.clone(),
            modules,
            source_label: options.source_label.clone(),
            source_index: options.source_index,
            role: options.role.clone(),
        });

        color = match color {
            Barcode1DRunColor::Bar => Barcode1DRunColor::Space,
            Barcode1DRunColor::Space => Barcode1DRunColor::Bar,
        };
    }

    Ok(runs)
}

#[cfg(target_vendor = "apple")]
fn default_text_family() -> &'static str {
    "Helvetica"
}

#[cfg(not(target_vendor = "apple"))]
fn default_text_family() -> &'static str {
    "sans-serif"
}

fn compute_text_line_height(
    metrics: &NativeMetrics,
    handle: &text_native::NativeHandle,
    size: f64,
) -> f64 {
    let upem = metrics.units_per_em(handle).max(1) as f64;
    let ascent = metrics.ascent(handle) as f64 * size / upem;
    let descent = metrics.descent(handle) as f64 * size / upem;
    let gap = metrics.line_gap(handle) as f64 * size / upem;
    (ascent + descent + gap).max(size)
}

fn build_human_readable_text_instructions(
    scene_width: f64,
    bar_height: f64,
    text: &str,
    config: &Barcode1DRenderConfig,
) -> Result<(Vec<PaintInstruction>, f64), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok((Vec::new(), 0.0));
    }

    let resolver = NativeResolver::new();
    let metrics = NativeMetrics::new();
    let shaper = NativeShaper::new();
    let handle = resolver
        .resolve(&FontQuery {
            family_names: vec![default_text_family().to_string()],
            weight: FontWeight::REGULAR,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
        })
        .map_err(|err| format!("human-readable text font resolution failed: {err}"))?;
    let shaped = shaper
        .shape(
            text,
            &handle,
            config.text_font_size as f32,
            &ShapeOptions::default(),
        )
        .map_err(|err| format!("human-readable text shaping failed: {err}"))?;

    let text_width = shaped.total_advance() as f64;
    let line_height = compute_text_line_height(&metrics, &handle, config.text_font_size);
    let ascent = metrics.ascent(&handle) as f64 * config.text_font_size
        / metrics.units_per_em(&handle).max(1) as f64;
    let baseline_x = ((scene_width - text_width) / 2.0).max(0.0);
    let baseline_y = bar_height + config.text_margin + ascent;

    let mut instructions = Vec::new();
    let mut line_pen_x = 0.0f64;
    let mut line_pen_y = 0.0f64;

    for run in &shaped.runs {
        if run.glyphs.is_empty() {
            continue;
        }

        let mut positions = Vec::with_capacity(run.glyphs.len());
        let mut seg_pen_x = 0.0f64;
        let mut seg_pen_y = 0.0f64;
        for glyph in &run.glyphs {
            positions.push(GlyphPosition {
                glyph_id: glyph.glyph_id,
                x: baseline_x + line_pen_x + seg_pen_x + glyph.x_offset as f64,
                y: baseline_y + line_pen_y + seg_pen_y + glyph.y_offset as f64,
            });
            seg_pen_x += glyph.x_advance as f64;
            seg_pen_y += glyph.y_advance as f64;
        }

        instructions.push(PaintInstruction::GlyphRun(PaintGlyphRun {
            base: PaintBase::default(),
            glyphs: positions,
            font_ref: run.font_ref.clone(),
            font_size: config.text_font_size,
            fill: Some(config.foreground.clone()),
        }));

        line_pen_x += run.x_advance_total as f64;
        line_pen_y += run
            .glyphs
            .iter()
            .map(|glyph| glyph.y_advance as f64)
            .sum::<f64>();
    }

    Ok((instructions, config.text_margin + line_height))
}

pub fn layout_barcode_1d(
    runs: &[Barcode1DRun],
    options: &PaintBarcode1DOptions,
) -> Result<PaintScene, String> {
    validate_render_config(&options.render_config)?;

    let layout = compute_barcode_1d_layout(
        runs,
        options.render_config.quiet_zone_modules,
        options.symbols.as_deref(),
    )?;

    let mut instructions = Vec::new();
    let mut module_cursor = layout.left_quiet_zone_modules;

    for run in runs {
        let x = module_cursor as f64 * options.render_config.module_width;
        let width = run.modules as f64 * options.render_config.module_width;

        if run.color == Barcode1DRunColor::Bar {
            let mut rect = PaintRect::filled(
                x,
                0.0,
                width,
                options.render_config.bar_height,
                &options.render_config.foreground,
            );
            rect.base.metadata = Some(HashMap::from([
                ("sourceLabel".to_string(), run.source_label.clone()),
                ("sourceIndex".to_string(), run.source_index.to_string()),
                ("role".to_string(), run.role.as_str().to_string()),
                ("moduleStart".to_string(), module_cursor.to_string()),
                (
                    "moduleEnd".to_string(),
                    (module_cursor + run.modules).to_string(),
                ),
            ]));
            instructions.push(PaintInstruction::Rect(rect));
        }

        module_cursor += run.modules;
    }

    let scene_width = layout.total_modules as f64 * options.render_config.module_width;
    let text_height = if options.render_config.include_human_readable_text {
        let text = options.human_readable_text.as_deref().ok_or_else(|| {
            "include_human_readable_text requires human_readable_text to be provided".to_string()
        })?;
        let (text_instructions, extra_height) = match options.render_config.layout_target {
            Barcode1DLayoutTarget::NativePaintVm => build_human_readable_text_instructions(
                scene_width,
                options.render_config.bar_height,
                text,
                &options.render_config,
            )?,
            Barcode1DLayoutTarget::CanvasPaintVm => {
                return Err(
                    "human-readable text for the canvas paint target is not wired yet; choose NativePaintVm or leave include_human_readable_text disabled"
                        .to_string(),
                )
            }
            Barcode1DLayoutTarget::DomPaintVm => {
                return Err(
                    "human-readable text for the DOM paint target is not wired yet; choose NativePaintVm or leave include_human_readable_text disabled"
                        .to_string(),
                )
            }
        };
        instructions.extend(text_instructions);
        extra_height
    } else {
        0.0
    };
    let scene_height = options.render_config.bar_height + text_height;

    let mut metadata = options.metadata.clone();
    metadata.insert(
        "label".to_string(),
        options
            .label
            .clone()
            .unwrap_or_else(|| "1D barcode".to_string()),
    );
    metadata.insert(
        "leftQuietZoneModules".to_string(),
        layout.left_quiet_zone_modules.to_string(),
    );
    metadata.insert(
        "rightQuietZoneModules".to_string(),
        layout.right_quiet_zone_modules.to_string(),
    );
    metadata.insert(
        "contentModules".to_string(),
        layout.content_modules.to_string(),
    );
    metadata.insert("totalModules".to_string(), layout.total_modules.to_string());
    metadata.insert(
        "moduleWidthPx".to_string(),
        options.render_config.module_width.to_string(),
    );
    metadata.insert(
        "barHeightPx".to_string(),
        options.render_config.bar_height.to_string(),
    );
    metadata.insert("sceneWidthPx".to_string(), scene_width.to_string());
    metadata.insert("sceneHeightPx".to_string(), scene_height.to_string());
    metadata.insert(
        "symbolCount".to_string(),
        layout.symbol_layouts.len().to_string(),
    );
    metadata.insert(
        "layoutTarget".to_string(),
        options.render_config.layout_target.as_str().to_string(),
    );

    if let Some(text) = options.human_readable_text.as_ref() {
        metadata.insert("humanReadableText".to_string(), text.clone());
    }

    if options.render_config.include_human_readable_text {
        metadata.insert("humanReadableTextEnabled".to_string(), "true".to_string());
        metadata.insert(
            "textFontSizePx".to_string(),
            options.render_config.text_font_size.to_string(),
        );
        metadata.insert(
            "textMarginPx".to_string(),
            options.render_config.text_margin.to_string(),
        );
    }

    let mut scene = PaintScene::new(scene_width, scene_height);
    scene.background = options.render_config.background.clone();
    scene.instructions = instructions;
    scene.metadata = Some(metadata);
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn binary_pattern_expands_to_runs() {
        let runs = runs_from_binary_pattern(
            "11001",
            &RunsFromBinaryPatternOptions {
                source_label: "start".to_string(),
                source_index: -1,
                role: Barcode1DRunRole::Guard,
            },
        )
        .unwrap();

        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].color, Barcode1DRunColor::Bar);
        assert_eq!(runs[0].modules, 2);
        assert_eq!(runs[1].color, Barcode1DRunColor::Space);
        assert_eq!(runs[2].modules, 1);
    }

    #[test]
    fn width_pattern_expands_to_runs() {
        let runs = runs_from_width_pattern(
            "NWN",
            &RunsFromWidthPatternOptions::new("A", 0, Barcode1DRunRole::Data),
        )
        .unwrap();

        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].modules, 1);
        assert_eq!(runs[1].modules, 3);
        assert_eq!(runs[2].color, Barcode1DRunColor::Bar);
    }

    #[test]
    fn computes_quiet_zone_aware_layout() {
        let runs = vec![
            Barcode1DRun {
                color: Barcode1DRunColor::Bar,
                modules: 1,
                source_label: "*".to_string(),
                source_index: 0,
                role: Barcode1DRunRole::Start,
            },
            Barcode1DRun {
                color: Barcode1DRunColor::Space,
                modules: 1,
                source_label: "*".to_string(),
                source_index: 0,
                role: Barcode1DRunRole::InterCharacterGap,
            },
            Barcode1DRun {
                color: Barcode1DRunColor::Bar,
                modules: 2,
                source_label: "A".to_string(),
                source_index: 1,
                role: Barcode1DRunRole::Data,
            },
        ];

        let layout = compute_barcode_1d_layout(&runs, 10, None).unwrap();
        assert_eq!(layout.content_modules, 4);
        assert_eq!(layout.total_modules, 24);
        assert_eq!(layout.symbol_layouts.len(), 2);
        assert_eq!(layout.symbol_layouts[0].label, "*");
        assert_eq!(layout.symbol_layouts[0].end_module, 2);
    }

    #[test]
    fn lays_out_runs_into_paint_scene() {
        let runs = runs_from_binary_pattern(
            "101",
            &RunsFromBinaryPatternOptions {
                source_label: "demo".to_string(),
                source_index: 0,
                role: Barcode1DRunRole::Guard,
            },
        )
        .unwrap();

        let mut options = PaintBarcode1DOptions::default();
        options.label = Some("Demo barcode".to_string());
        let scene = layout_barcode_1d(&runs, &options).unwrap();

        assert_eq!(scene.background, "#ffffff");
        assert_eq!(scene.instructions.len(), 2);
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("label")),
            Some(&"Demo barcode".to_string())
        );
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("totalModules")),
            Some(&"23".to_string())
        );
    }

    #[test]
    #[cfg(any(target_os = "windows", target_vendor = "apple"))]
    fn human_readable_text_emits_glyph_runs() {
        let runs = runs_from_binary_pattern(
            "101",
            &RunsFromBinaryPatternOptions {
                source_label: "demo".to_string(),
                source_index: 0,
                role: Barcode1DRunRole::Guard,
            },
        )
        .unwrap();

        let mut options = PaintBarcode1DOptions::default();
        options.render_config.include_human_readable_text = true;
        options.human_readable_text = Some("demo".to_string());
        let scene = layout_barcode_1d(&runs, &options).unwrap();
        assert!(scene.height > options.render_config.bar_height);
        assert!(scene
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, PaintInstruction::GlyphRun(_))));
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("layoutTarget")),
            Some(&"native-paint-vm".to_string())
        );
    }

    #[cfg(all(not(target_os = "windows"), not(target_vendor = "apple")))]
    #[test]
    fn human_readable_text_reports_missing_backend() {
        let runs = runs_from_binary_pattern(
            "101",
            &RunsFromBinaryPatternOptions {
                source_label: "demo".to_string(),
                source_index: 0,
                role: Barcode1DRunRole::Guard,
            },
        )
        .unwrap();

        let mut options = PaintBarcode1DOptions::default();
        options.render_config.include_human_readable_text = true;
        options.human_readable_text = Some("demo".to_string());

        let error = layout_barcode_1d(&runs, &options).unwrap_err();
        assert!(error.contains("font resolution failed") || error.contains("LoadFailed"));
    }

    #[test]
    fn canvas_target_rejects_native_glyph_label_path() {
        let runs = runs_from_binary_pattern(
            "101",
            &RunsFromBinaryPatternOptions {
                source_label: "demo".to_string(),
                source_index: 0,
                role: Barcode1DRunRole::Guard,
            },
        )
        .unwrap();

        let mut options = PaintBarcode1DOptions::default();
        options.render_config.include_human_readable_text = true;
        options.render_config.layout_target = Barcode1DLayoutTarget::CanvasPaintVm;
        options.human_readable_text = Some("demo".to_string());

        let error = layout_barcode_1d(&runs, &options).unwrap_err();
        assert!(error.contains("canvas paint target"));
    }
}
