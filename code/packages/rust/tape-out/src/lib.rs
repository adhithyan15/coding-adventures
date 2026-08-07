//! # Tape-Out Bundle
//!
//! Assembles and validates an Efabless chipIgnite silicon-submission bundle.
//!
//! A submission bundle consists of:
//! - GDSII layout binary
//! - LEF + DEF physical design interchange
//! - Behavioral Verilog (for testbench)
//! - DRC / LVS signoff reports
//! - `manifest.yaml` — project metadata, pad locations, clock/power specs
//! - `README.md` — human-readable summary
//!
//! The library does **not** write files itself.  Call [`render_manifest`] and
//! [`render_readme`] to produce the YAML / Markdown text, then persist them
//! however your application sees fit.
//!
//! ## Example
//!
//! ```rust
//! use tape_out::{TapeoutBundle, TapeoutMetadata, Shuttle, validate_for_chipignite};
//!
//! let meta = TapeoutMetadata {
//!     project_name: "adder4".into(),
//!     designer: "Alice".into(),
//!     email: "alice@example.com".into(),
//!     top_module: "adder4".into(),
//!     ..TapeoutMetadata::default()
//! };
//! let mut bundle = TapeoutBundle::new(meta);
//! bundle.signoff.insert("drc".into(), "clean".into());
//! bundle.signoff.insert("lvs".into(), "clean".into());
//! bundle.files.insert("gds".into(), "adder4.gds".into());
//! bundle.files.insert("lef".into(), "adder4.lef".into());
//! bundle.files.insert("def".into(), "adder4.def".into());
//! bundle.files.insert("verilog".into(), "adder4.v".into());
//! bundle.files.insert("drc_report".into(), "drc.rpt".into());
//! bundle.files.insert("lvs_report".into(), "lvs.rpt".into());
//!
//! let report = validate_for_chipignite(&bundle);
//! assert!(report.passed);
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Efabless shuttle programme variant.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum Shuttle {
    #[default]
    ChipigniteOpenMpw,
    ChipignitePaidMpw,
    TinyTapeout,
}

impl Shuttle {
    /// Canonical shuttle identifier string (used in manifest.yaml).
    pub fn as_str(&self) -> &'static str {
        match self {
            Shuttle::ChipigniteOpenMpw => "chipignite_open_mpw",
            Shuttle::ChipignitePaidMpw => "chipignite_paid_mpw",
            Shuttle::TinyTapeout       => "tiny_tapeout",
        }
    }
}


/// IO pad location on the die boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct PadLocation {
    pub name: String,
    /// "input" | "output" | "inout" | "power" | "ground"
    pub direction: String,
    pub x: f64,
    pub y: f64,
}

/// Project-level metadata written into manifest.yaml.
#[derive(Debug, Clone)]
pub struct TapeoutMetadata {
    pub project_name: String,
    pub designer: String,
    pub email: String,
    pub shuttle: Shuttle,
    pub pdk: String,
    pub pdk_version: Option<String>,
    pub license: String,
    pub top_module: String,
    pub git_url: Option<String>,
    pub clock_frequency_mhz: f64,
    pub clock_signal: String,
    pub vdd_voltage: f64,
}

impl Default for TapeoutMetadata {
    fn default() -> Self {
        TapeoutMetadata {
            project_name: String::new(),
            designer: String::new(),
            email: String::new(),
            shuttle: Shuttle::default(),
            pdk: "sky130A".into(),
            pdk_version: None,
            license: "Apache-2.0".into(),
            top_module: String::new(),
            git_url: None,
            clock_frequency_mhz: 0.0,
            clock_signal: "clk".into(),
            vdd_voltage: 1.8,
        }
    }
}

/// A tape-out submission bundle.
///
/// - `files` — logical name → filename string (e.g. `"gds" → "adder4.gds"`)
/// - `signoff` — check name → result string (e.g. `"drc" → "clean"`)
/// - `pad_locations` — die IO ring pads
pub struct TapeoutBundle {
    pub metadata: TapeoutMetadata,
    /// Logical name → filename (not a `Path` — no filesystem coupling in the library).
    pub files: HashMap<String, String>,
    pub pad_locations: Vec<PadLocation>,
    pub signoff: HashMap<String, String>,
}

impl TapeoutBundle {
    pub fn new(metadata: TapeoutMetadata) -> Self {
        TapeoutBundle {
            metadata,
            files: HashMap::new(),
            pad_locations: Vec::new(),
            signoff: HashMap::new(),
        }
    }
}

/// Validation result for a tape-out bundle.
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Required files per chipIgnite acceptance criteria
// ---------------------------------------------------------------------------

const REQUIRED_FILES: &[&str] = &["gds", "lef", "def", "verilog", "drc_report", "lvs_report"];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate a bundle against chipIgnite acceptance criteria.
///
/// Checks required fields, required files, and DRC/LVS signoff state.
pub fn validate_for_chipignite(bundle: &TapeoutBundle) -> ValidationReport {
    let mut report = ValidationReport { passed: true, ..Default::default() };
    let m = &bundle.metadata;

    if m.project_name.is_empty() { report.errors.push("project_name is required".into()); }
    if m.designer.is_empty()     { report.errors.push("designer is required".into()); }
    if m.email.is_empty()        { report.errors.push("email is required".into()); }
    if m.top_module.is_empty()   { report.errors.push("top_module is required".into()); }

    for &req in REQUIRED_FILES {
        if !bundle.files.contains_key(req) {
            report.errors.push(format!("missing required file: {req}"));
        }
    }

    match bundle.signoff.get("drc").map(|s| s.as_str()) {
        Some("clean") => {},
        Some(v) => report.errors.push(format!("DRC not clean: {v:?}")),
        None    => report.errors.push("DRC signoff missing".into()),
    }
    match bundle.signoff.get("lvs").map(|s| s.as_str()) {
        Some("clean") => {},
        Some(v) => report.errors.push(format!("LVS not clean: {v:?}")),
        None    => report.errors.push("LVS signoff missing".into()),
    }

    if m.shuttle == Shuttle::ChipigniteOpenMpw && bundle.pad_locations.is_empty() {
        report.warnings.push("no pad_locations specified; chipIgnite may reject".into());
    }

    if !report.errors.is_empty() { report.passed = false; }
    report
}

/// Render `manifest.yaml` content as a String.
pub fn render_manifest(bundle: &TapeoutBundle) -> String {
    let m = &bundle.metadata;
    let mut lines: Vec<String> = vec![
        format!("project_name: {}", m.project_name),
        format!("designer: {}", m.designer),
        format!("email: {}", m.email),
        format!("shuttle: {}", m.shuttle.as_str()),
        format!("pdk: {}", m.pdk),
    ];
    if let Some(ref v) = m.pdk_version {
        lines.push(format!("pdk_version: {v}"));
    }
    lines.push(format!("license: {}", m.license));
    lines.push(format!("top_module: {}", m.top_module));
    if let Some(ref url) = m.git_url {
        lines.push(format!("git_url: {url}"));
    }
    lines.push(String::new());
    lines.push("clock:".into());
    lines.push(format!("  primary: {}", m.clock_signal));
    lines.push(format!("  frequency_mhz: {}", m.clock_frequency_mhz));
    lines.push(String::new());
    lines.push("power:".into());
    lines.push(format!("  vdd_voltage: {}", m.vdd_voltage));

    if !bundle.pad_locations.is_empty() {
        lines.push(String::new());
        lines.push("pads:".into());
        for pad in &bundle.pad_locations {
            lines.push(format!(
                "  - {{name: '{}', dir: {}, x: {}, y: {}}}",
                pad.name, pad.direction, pad.x, pad.y
            ));
        }
    }

    if !bundle.signoff.is_empty() {
        lines.push(String::new());
        lines.push("signoff:".into());
        let mut keys: Vec<&String> = bundle.signoff.keys().collect();
        keys.sort();
        for k in keys {
            lines.push(format!("  {k}: {}", bundle.signoff[k]));
        }
    }

    lines.join("\n") + "\n"
}

/// Render `README.md` content as a String.
pub fn render_readme(bundle: &TapeoutBundle) -> String {
    let m = &bundle.metadata;
    let mut out = format!(
        "# {}\n\nTape-out bundle for {}.\n\n- Designer: {} <{}>\n- PDK: {}",
        m.project_name, m.shuttle.as_str(), m.designer, m.email, m.pdk
    );
    if let Some(ref v) = m.pdk_version { out.push_str(&format!(" ({v})")); }
    out.push('\n');
    out.push_str(&format!("- Top module: {}\n", m.top_module));
    out.push_str(&format!("- License: {}\n\n## Files\n\n", m.license));
    let mut file_keys: Vec<&String> = bundle.files.keys().collect();
    file_keys.sort();
    for k in file_keys {
        out.push_str(&format!("- {k}: `{}`\n", bundle.files[k]));
    }
    out
}
