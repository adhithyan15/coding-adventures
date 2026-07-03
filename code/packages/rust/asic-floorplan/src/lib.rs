//! # ASIC Floorplanning
//!
//! Given a list of cells with area estimates and a target utilization,
//! computes a die area + row grid + IO pin placement and returns a DEF
//! ready for the placement pass.
//!
//! ## Algorithm
//!
//! 1. Compute `core_area = total_cell_area / utilization`.
//! 2. From `aspect = core_width / core_height` and `core_area`, solve for
//!    `core_height = sqrt(core_area / aspect)`, `core_width = aspect × core_height`.
//! 3. Snap height to an integer number of cell rows; snap width to an
//!    integer number of sites.
//! 4. Add `io_ring_width` margin on every side to get the die area.
//! 5. Generate `Row` records (alternating N / FS orientation).
//! 6. Distribute IO pins evenly on the die boundary:
//!    inputs on the left edge, outputs on the right edge, others on the bottom.
//!
//! ## Usage
//!
//! ```rust
//! use asic_floorplan::{compute_floorplan, floorplan_to_def, CellInstanceEstimate, FloorplanOptions, IoSpec};
//! use lef_def::Direction;
//!
//! let cells = vec![
//!     CellInstanceEstimate { instance_name: "xor_0".into(), cell_type: "xor2_1".into(), area: 6.45 },
//!     CellInstanceEstimate { instance_name: "and_0".into(), cell_type: "and2_1".into(), area: 4.60 },
//! ];
//! let opts = FloorplanOptions::sky130_hd();
//! let fp = compute_floorplan(&cells, &[], &opts).unwrap();
//! let def = floorplan_to_def(&fp, "adder4");
//! assert!(def.die_area.is_some());
//! ```

use lef_def::{Component, Def, DefPin, Direction, Rect, Row, Use};

/// One cell instance to be placed. Only the area matters for floorplanning.
pub struct CellInstanceEstimate {
    pub instance_name: String,
    pub cell_type: String,
    /// Cell area in square micrometres.
    pub area: f64,
}

/// One top-level IO pin.
pub struct IoSpec {
    pub name: String,
    pub direction: Direction,
    pub use_: Use,
}

/// Floorplan parameters.
pub struct FloorplanOptions {
    /// Height of one standard-cell row in µm. Sky130 HD = 2.72.
    pub site_height: f64,
    /// Width of one site in µm. Sky130 HD ≈ 0.46.
    pub site_width: f64,
    /// Site name (e.g. "unithd").
    pub site_name: String,
    /// Target core utilization: 0 < u ≤ 1. Default 0.7.
    pub utilization: f64,
    /// core_width / core_height ratio. 1.0 = square.
    pub aspect: f64,
    /// Margin around the core for the IO ring, in µm. Default 10.0.
    pub io_ring_width: f64,
    /// Layer for IO pin shapes. Default "met2".
    pub pin_layer: String,
}

impl FloorplanOptions {
    /// Defaults for the Sky130 HD (high-density) cell family.
    pub fn sky130_hd() -> Self {
        Self {
            site_height: 2.72,
            site_width: 0.46,
            site_name: "unithd".to_string(),
            utilization: 0.70,
            aspect: 1.0,
            io_ring_width: 10.0,
            pin_layer: "met2".to_string(),
        }
    }
}

/// A computed floorplan ready to be converted to DEF.
pub struct Floorplan {
    pub die: Rect,
    pub core: Rect,
    pub rows: Vec<Row>,
    pub components: Vec<Component>,
    pub pins: Vec<DefPin>,
}

/// Floorplan computation errors.
#[derive(Debug)]
pub enum FloorplanError {
    /// utilization ∉ (0, 1].
    InvalidUtilization(f64),
    /// aspect ≤ 0.
    InvalidAspect(f64),
    /// Total cell area is zero or negative.
    ZeroArea,
}

impl std::fmt::Display for FloorplanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FloorplanError::InvalidUtilization(u) => {
                write!(f, "utilization must be in (0,1], got {u}")
            }
            FloorplanError::InvalidAspect(a) => write!(f, "aspect must be > 0, got {a}"),
            FloorplanError::ZeroArea => write!(f, "total cell area must be > 0"),
        }
    }
}

impl std::error::Error for FloorplanError {}

/// Compute a floorplan from cell estimates and IO pins.
pub fn compute_floorplan(
    cells: &[CellInstanceEstimate],
    io_pins: &[IoSpec],
    opts: &FloorplanOptions,
) -> Result<Floorplan, FloorplanError> {
    if opts.utilization <= 0.0 || opts.utilization > 1.0 {
        return Err(FloorplanError::InvalidUtilization(opts.utilization));
    }
    if opts.aspect <= 0.0 {
        return Err(FloorplanError::InvalidAspect(opts.aspect));
    }
    let total_area: f64 = cells.iter().map(|c| c.area).sum();
    if total_area <= 0.0 {
        return Err(FloorplanError::ZeroArea);
    }

    let core_area = total_area / opts.utilization;

    // core_width × core_height = core_area;  core_width = aspect × core_height
    let core_height_raw = (core_area / opts.aspect).sqrt();
    let core_width_raw = opts.aspect * core_height_raw;

    // Snap to integer rows / sites.
    let n_rows = core_height_raw.div_euclid(opts.site_height).ceil().max(1.0) as u32;
    let n_sites = core_width_raw.div_euclid(opts.site_width).ceil().max(1.0) as u32;
    let core_height = n_rows as f64 * opts.site_height;
    let core_width = n_sites as f64 * opts.site_width;

    let core_x0 = opts.io_ring_width;
    let core_y0 = opts.io_ring_width;
    let core_x1 = core_x0 + core_width;
    let core_y1 = core_y0 + core_height;
    let die = Rect::new(0.0, 0.0, core_x1 + opts.io_ring_width, core_y1 + opts.io_ring_width);

    // Generate rows (alternating orientation for abut-friendly flips).
    let rows: Vec<Row> = (0..n_rows)
        .map(|i| Row {
            name: format!("row_{i}"),
            site: opts.site_name.clone(),
            origin_x: core_x0,
            origin_y: core_y0 + i as f64 * opts.site_height,
            orientation: if i % 2 == 0 { "N".to_string() } else { "FS".to_string() },
            num_x: n_sites,
            num_y: 1,
            step_x: opts.site_width,
            step_y: 0.0,
        })
        .collect();

    // Components are unplaced at this stage; the placer fills in coordinates.
    let components: Vec<Component> = cells
        .iter()
        .map(|c| Component::new(&c.instance_name, &c.cell_type))
        .collect();

    let pins = place_io_pins(io_pins, &die, &opts.pin_layer);

    Ok(Floorplan {
        die,
        core: Rect::new(core_x0, core_y0, core_x1, core_y1),
        rows,
        components,
        pins,
    })
}

/// Distribute IO pins evenly around the die boundary.
///
/// - Inputs → left edge (x = die.x1)
/// - Outputs → right edge (x = die.x2)
/// - Others (inout, power, ground) → bottom edge (y = die.y1)
fn place_io_pins(io: &[IoSpec], die: &Rect, pin_layer: &str) -> Vec<DefPin> {
    let inputs: Vec<&IoSpec> = io.iter().filter(|p| p.direction == Direction::Input).collect();
    let outputs: Vec<&IoSpec> = io.iter().filter(|p| p.direction == Direction::Output).collect();
    let others: Vec<&IoSpec> = io.iter()
        .filter(|p| p.direction != Direction::Input && p.direction != Direction::Output)
        .collect();

    let mut pins = Vec::new();

    // Inputs on left edge.
    if !inputs.is_empty() {
        let edge_h = die.y2 - die.y1;
        let spacing = edge_h / (inputs.len() + 1) as f64;
        for (i, p) in inputs.iter().enumerate() {
            let y = die.y1 + (i + 1) as f64 * spacing;
            pins.push(DefPin {
                name: p.name.clone(),
                net: p.name.clone(),
                direction: p.direction.clone(),
                use_: p.use_.clone(),
                layer: Some(pin_layer.to_string()),
                rect: Some(Rect::new(die.x1 - 0.5, y - 0.1, die.x1, y + 0.1)),
            });
        }
    }

    // Outputs on right edge.
    if !outputs.is_empty() {
        let edge_h = die.y2 - die.y1;
        let spacing = edge_h / (outputs.len() + 1) as f64;
        for (i, p) in outputs.iter().enumerate() {
            let y = die.y1 + (i + 1) as f64 * spacing;
            pins.push(DefPin {
                name: p.name.clone(),
                net: p.name.clone(),
                direction: p.direction.clone(),
                use_: p.use_.clone(),
                layer: Some(pin_layer.to_string()),
                rect: Some(Rect::new(die.x2, y - 0.1, die.x2 + 0.5, y + 0.1)),
            });
        }
    }

    // Others on bottom edge.
    if !others.is_empty() {
        let edge_w = die.x2 - die.x1;
        let spacing = edge_w / (others.len() + 1) as f64;
        for (i, p) in others.iter().enumerate() {
            let x = die.x1 + (i + 1) as f64 * spacing;
            pins.push(DefPin {
                name: p.name.clone(),
                net: p.name.clone(),
                direction: p.direction.clone(),
                use_: p.use_.clone(),
                layer: Some(pin_layer.to_string()),
                rect: Some(Rect::new(x - 0.1, die.y1 - 0.5, x + 0.1, die.y1)),
            });
        }
    }

    pins
}

/// Convert a Floorplan to an unplaced DEF.
pub fn floorplan_to_def(fp: &Floorplan, design_name: &str) -> Def {
    Def {
        design: design_name.to_string(),
        version: "5.8".to_string(),
        units_microns: 1000,
        die_area: Some(fp.die.clone()),
        rows: fp.rows.clone(),
        components: fp.components.clone(),
        pins: fp.pins.clone(),
        nets: vec![],
    }
}
