//! Library, CellTiming, and TimingArc data model.

use std::collections::HashMap;

use crate::lut::LookupTable;

/// One timing arc (e.g., `A → Y rising delay`).
///
/// A cell can have multiple arcs — one per (input, output, sense) triple.
/// `sense` follows Liberty conventions:
/// - `"positive_unate"` — output rises when input rises (e.g., BUF)
/// - `"negative_unate"` — output falls when input rises (e.g., INV, NAND)
/// - `"non_unate"`      — no monotonic relationship (e.g., XOR, MUX, DFF)
#[derive(Debug, Clone)]
pub struct TimingArc {
    /// The input pin that triggers this arc.
    pub related_pin: String,
    /// The output pin that transitions.
    pub output_pin: String,
    /// Timing sense: "positive_unate", "negative_unate", or "non_unate".
    pub sense: String,
    /// Cell rise delay LUT — time for output to rise to 50% (ns).
    pub cell_rise: LookupTable,
    /// Cell fall delay LUT — time for output to fall to 50% (ns).
    pub cell_fall: LookupTable,
    /// Rise transition LUT — 10%→90% output rise time (ns).
    pub rise_transition: LookupTable,
    /// Fall transition LUT — 90%→10% output fall time (ns).
    pub fall_transition: LookupTable,
}

/// Per-cell characterization data.
#[derive(Debug, Clone)]
pub struct CellTiming {
    /// Full cell name, e.g. `"sky130_fd_sc_hd__inv_1"`.
    pub name: String,
    /// Cell area in square micrometres.
    pub area: f64,
    /// Leakage power in nanowatts (quiescent state).
    pub leakage_power: f64,
    /// Input pin capacitances in picofarads.
    pub pin_capacitance: HashMap<String, f64>,
    /// All timing arcs for this cell.
    pub timing_arcs: Vec<TimingArc>,
}

/// A complete standard-cell library.
///
/// Keyed on full cell name (e.g., `"sky130_fd_sc_hd__inv_1"`).
pub struct Library {
    /// Library name (e.g., `"sky130_fd_sc_hd__teaching"`).
    pub name: String,
    /// Supply voltage in volts.
    pub voltage: f64,
    /// Characterization temperature in °C.
    pub temperature: f64,
    /// Process corner: `"tt"` (typical), `"ss"` (slow), `"ff"` (fast).
    pub process: String,
    /// All cells in the library.
    pub cells: HashMap<String, CellTiming>,
}

impl Library {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            voltage: 1.8,
            temperature: 25.0,
            process: "tt".to_string(),
            cells: HashMap::new(),
        }
    }

    /// Look up a cell by name. Returns `None` if not present.
    pub fn get(&self, cell_name: &str) -> Option<&CellTiming> {
        self.cells.get(cell_name)
    }

    /// List all available drive strengths for a cell function family.
    ///
    /// E.g., `"sky130_fd_sc_hd__inv"` → `[1, 2, 4, 8]`.
    /// The drive strength is parsed from the numeric suffix after the last `_`.
    pub fn list_drives(&self, base_name: &str) -> Vec<u32> {
        let prefix = format!("{base_name}_");
        let mut drives: Vec<u32> = self
            .cells
            .keys()
            .filter_map(|name| {
                name.strip_prefix(&prefix)
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .collect();
        drives.sort();
        drives
    }
}
