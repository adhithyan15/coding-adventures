//! LEF/DEF data model.
//!
//! All types follow the Accellera DEF 5.8 / LEF 5.8 conventions.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shared primitives
// ---------------------------------------------------------------------------

/// IO pin / cell-port direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    Input,
    Output,
    Inout,
}

impl Direction {
    pub fn as_lef_str(&self) -> &'static str {
        match self {
            Direction::Input => "INPUT",
            Direction::Output => "OUTPUT",
            Direction::Inout => "INOUT",
        }
    }
}

/// Signal use classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Use {
    Signal,
    Power,
    Ground,
    Clock,
}

impl Use {
    pub fn as_lef_str(&self) -> &'static str {
        match self {
            Use::Signal => "SIGNAL",
            Use::Power => "POWER",
            Use::Ground => "GROUND",
            Use::Clock => "CLOCK",
        }
    }
}

/// Axis-aligned rectangle in micrometres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Rect {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn width(&self) -> f64 { self.x2 - self.x1 }
    pub fn height(&self) -> f64 { self.y2 - self.y1 }
    pub fn area(&self) -> f64 { self.width() * self.height() }
}

// ---------------------------------------------------------------------------
// LEF — technology + cell definitions
// ---------------------------------------------------------------------------

/// A technology-level layer definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LayerDef {
    pub name: String,
    /// Layer type: "ROUTING", "CUT", "MASTERSLICE", "OVERLAP".
    pub r#type: String,
    /// Routing direction: "HORIZONTAL" or "VERTICAL". None for cut layers.
    pub direction: Option<String>,
    pub pitch: f64,
    pub width: f64,
    pub spacing: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViaLayer {
    pub layer: String,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViaDef {
    pub name: String,
    pub is_default: bool,
    pub layers: Vec<ViaLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteDef {
    /// Site name (e.g. "unithd").
    pub name: String,
    /// Site class: "CORE" or "PAD".
    pub class: String,
    pub width: f64,
    pub height: f64,
}

/// Technology LEF: layers, vias, sites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TechLef {
    pub version: String,
    pub units_microns: u32,
    pub layers: Vec<LayerDef>,
    pub vias: Vec<ViaDef>,
    pub sites: Vec<SiteDef>,
}

impl TechLef {
    pub fn new() -> Self {
        Self {
            version: "5.8".to_string(),
            units_microns: 1000,
            ..Default::default()
        }
    }
}

/// One geometry port for a cell pin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinPort {
    pub layer: String,
    pub rect: Rect,
}

/// A single named pin in a cell LEF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinDef {
    pub name: String,
    pub direction: Direction,
    pub use_: Use,
    pub ports: Vec<PinPort>,
}

/// Cell LEF: physical footprint of one standard cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CellLef {
    pub name: String,
    /// Cell class: "CORE", "PAD", "ENDCAP", etc.
    pub class: String,
    /// External GDS cell name (if different from `name`).
    pub foreign: Option<String>,
    pub width: f64,
    pub height: f64,
    pub site: String,
    pub pins: Vec<PinDef>,
    /// Obstruction layer rectangles: `(layer, rect)`.
    pub obs: Vec<(String, Rect)>,
}

impl CellLef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            class: "CORE".to_string(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// DEF — design instance
// ---------------------------------------------------------------------------

/// One standard-cell row definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub name: String,
    pub site: String,
    pub origin_x: f64,
    pub origin_y: f64,
    /// Cell orientation: "N" (north = normal), "FS" (flip-south = mirrored).
    pub orientation: String,
    pub num_x: u32,
    pub num_y: u32,
    pub step_x: f64,
    pub step_y: f64,
}

/// One placed or unplaced component instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub cell_type: String,
    pub placed: bool,
    pub location_x: Option<f64>,
    pub location_y: Option<f64>,
    pub orientation: String,
}

impl Component {
    pub fn new(name: impl Into<String>, cell_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cell_type: cell_type.into(),
            placed: false,
            location_x: None,
            location_y: None,
            orientation: "N".to_string(),
        }
    }
}

/// A top-level IO pin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefPin {
    pub name: String,
    pub net: String,
    pub direction: Direction,
    pub use_: Use,
    pub layer: Option<String>,
    pub rect: Option<Rect>,
}

/// A routed wire segment on one metal layer. Points are user-unit (µm) coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub layer: String,
    /// Polyline vertices in micrometres.
    pub points: Vec<(f64, f64)>,
}

/// A logical net with optional placed connections and routed geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Net {
    pub name: String,
    /// `(cell_instance_name, pin_name)` pairs.
    pub connections: Vec<(String, String)>,
    pub routed_segments: Vec<Segment>,
}

impl Net {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }
}

/// The DEF top-level design document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Def {
    pub design: String,
    pub version: String,
    pub units_microns: u32,
    pub die_area: Option<Rect>,
    pub rows: Vec<Row>,
    pub components: Vec<Component>,
    pub pins: Vec<DefPin>,
    pub nets: Vec<Net>,
}

impl Def {
    pub fn new(design: impl Into<String>) -> Self {
        Self {
            design: design.into(),
            version: "5.8".to_string(),
            units_microns: 1000,
            ..Default::default()
        }
    }
}
