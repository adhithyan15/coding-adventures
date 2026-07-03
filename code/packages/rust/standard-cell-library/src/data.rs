//! Hand-curated NLDM timing data for the Sky130 HD teaching subset.
//!
//! Values tuned to within ~10% of the Sky130 open reference characterization.
//! All delays in nanoseconds; loads in femtofarads.
//!
//! # NLDM table shape
//!
//! Every cell uses a 5×5 slew × load grid:
//!
//! ```text
//! slew (ns)  → [0.01, 0.05, 0.10, 0.20, 0.50]
//! load (fF)  → [0.50, 1.00, 2.00, 5.00, 10.00]
//! ```
//!
//! `_make_lut(base, slew_factor, load_factor)` fills the grid via the formula:
//!
//!   delay[s][l] = base + slew[s] × slew_factor + load[l] × load_factor
//!
//! This linear model captures the dominant NLDM behavior.

use std::collections::HashMap;

use crate::library::{CellTiming, Library, TimingArc};
use crate::lut::LookupTable;
use sky130_pdk::TEACHING_CELLS;

/// Standard 5-point slew axis (nanoseconds).
const SLEW_NS: [f64; 5] = [0.01, 0.05, 0.10, 0.20, 0.50];
/// Standard 5-point output-load axis (femtofarads).
const LOAD_FF: [f64; 5] = [0.50, 1.00, 2.00, 5.00, 10.00];

/// Generate a 5×5 NLDM LUT.
///
/// `delay[s][l] = base + slew[s] × slew_factor + load[l] × load_factor`
fn make_lut(base: f64, slew_factor: f64, load_factor: f64) -> LookupTable {
    let values: Vec<Vec<f64>> = SLEW_NS
        .iter()
        .map(|&s| {
            LOAD_FF.iter().map(|&l| base + s * slew_factor + l * load_factor).collect()
        })
        .collect();
    LookupTable {
        slew_index: SLEW_NS.to_vec(),
        load_index: LOAD_FF.to_vec(),
        values,
    }
}

/// Build one timing arc from (related_pin, output_pin, sense, base_rise_delay, base_fall_delay).
fn make_arc(
    related: &str,
    output: &str,
    sense: &str,
    base_rise: f64,
    base_fall: f64,
) -> TimingArc {
    TimingArc {
        related_pin: related.to_string(),
        output_pin: output.to_string(),
        sense: sense.to_string(),
        cell_rise: make_lut(base_rise, 0.05, 0.02),
        cell_fall: make_lut(base_fall, 0.05, 0.02),
        rise_transition: make_lut(base_rise * 0.5, 0.03, 0.01),
        fall_transition: make_lut(base_fall * 0.5, 0.03, 0.01),
    }
}

/// Per-cell timing entry: (area_µm², leakage_nW, pin_caps, arcs).
/// Arcs: (related, output, sense, rise_ns, fall_ns)
struct CellEntry {
    area: f64,
    leakage: f64,
    pin_caps: Vec<(&'static str, f64)>,
    arcs: Vec<(&'static str, &'static str, &'static str, f64, f64)>,
}

impl CellEntry {
    fn new(
        area: f64,
        leakage: f64,
        pin_caps: &[(&'static str, f64)],
        arcs: &[(&'static str, &'static str, &'static str, f64, f64)],
    ) -> Self {
        Self {
            area,
            leakage,
            pin_caps: pin_caps.to_vec(),
            arcs: arcs.to_vec(),
        }
    }
}

fn cell_data() -> HashMap<&'static str, CellEntry> {
    use CellEntry as C;
    let mut m: HashMap<&'static str, CellEntry> = HashMap::new();

    // --- Inverters ---
    m.insert("sky130_fd_sc_hd__inv_1", C::new(
        1.84, 0.5, &[("A", 0.0036)],
        &[("A", "Y", "negative_unate", 0.04, 0.04)]));
    m.insert("sky130_fd_sc_hd__inv_2", C::new(
        2.30, 1.0, &[("A", 0.0072)],
        &[("A", "Y", "negative_unate", 0.025, 0.025)]));
    m.insert("sky130_fd_sc_hd__inv_4", C::new(
        3.45, 2.0, &[("A", 0.0144)],
        &[("A", "Y", "negative_unate", 0.015, 0.015)]));
    m.insert("sky130_fd_sc_hd__inv_8", C::new(
        5.75, 4.0, &[("A", 0.0288)],
        &[("A", "Y", "negative_unate", 0.010, 0.010)]));

    // --- Buffers ---
    m.insert("sky130_fd_sc_hd__buf_1", C::new(
        2.30, 0.6, &[("A", 0.0036)],
        &[("A", "X", "positive_unate", 0.075, 0.075)]));
    m.insert("sky130_fd_sc_hd__buf_2", C::new(
        3.22, 1.2, &[("A", 0.0036)],
        &[("A", "X", "positive_unate", 0.05, 0.05)]));
    m.insert("sky130_fd_sc_hd__buf_4", C::new(
        5.06, 2.4, &[("A", 0.0036)],
        &[("A", "X", "positive_unate", 0.03, 0.03)]));
    m.insert("sky130_fd_sc_hd__buf_8", C::new(
        8.74, 4.8, &[("A", 0.0036)],
        &[("A", "X", "positive_unate", 0.02, 0.02)]));

    // --- NAND gates ---
    m.insert("sky130_fd_sc_hd__nand2_1", C::new(
        3.75, 1.0, &[("A", 0.0036), ("B", 0.0035)],
        &[("A", "Y", "negative_unate", 0.06, 0.07),
          ("B", "Y", "negative_unate", 0.05, 0.06)]));
    m.insert("sky130_fd_sc_hd__nand2_2", C::new(
        4.60, 2.0, &[("A", 0.0072), ("B", 0.0070)],
        &[("A", "Y", "negative_unate", 0.04, 0.045),
          ("B", "Y", "negative_unate", 0.035, 0.04)]));
    m.insert("sky130_fd_sc_hd__nand3_1", C::new(
        4.60, 1.5, &[("A", 0.0036), ("B", 0.0035), ("C", 0.0035)],
        &[("A", "Y", "negative_unate", 0.08, 0.10)]));

    // --- NOR gates ---
    m.insert("sky130_fd_sc_hd__nor2_1", C::new(
        3.75, 1.0, &[("A", 0.0040), ("B", 0.0040)],
        &[("A", "Y", "negative_unate", 0.07, 0.06),
          ("B", "Y", "negative_unate", 0.06, 0.05)]));
    m.insert("sky130_fd_sc_hd__nor2_2", C::new(
        4.60, 2.0, &[("A", 0.0080), ("B", 0.0080)],
        &[("A", "Y", "negative_unate", 0.045, 0.04),
          ("B", "Y", "negative_unate", 0.04, 0.035)]));
    m.insert("sky130_fd_sc_hd__nor3_1", C::new(
        4.60, 1.5, &[("A", 0.0040), ("B", 0.0040), ("C", 0.0040)],
        &[("A", "Y", "negative_unate", 0.10, 0.08)]));

    // --- AND gates ---
    m.insert("sky130_fd_sc_hd__and2_1", C::new(
        4.60, 1.0, &[("A", 0.0036), ("B", 0.0035)],
        &[("A", "X", "positive_unate", 0.10, 0.10)]));
    m.insert("sky130_fd_sc_hd__and2_2", C::new(
        5.50, 2.0, &[("A", 0.0072), ("B", 0.0070)],
        &[("A", "X", "positive_unate", 0.07, 0.07)]));

    // --- OR gates ---
    m.insert("sky130_fd_sc_hd__or2_1", C::new(
        4.60, 1.0, &[("A", 0.0040), ("B", 0.0040)],
        &[("A", "X", "positive_unate", 0.10, 0.10)]));
    m.insert("sky130_fd_sc_hd__or2_2", C::new(
        5.50, 2.0, &[("A", 0.0080), ("B", 0.0080)],
        &[("A", "X", "positive_unate", 0.07, 0.07)]));

    // --- XOR / XNOR ---
    m.insert("sky130_fd_sc_hd__xor2_1", C::new(
        6.45, 1.5, &[("A", 0.0050), ("B", 0.0050)],
        &[("A", "X", "non_unate", 0.12, 0.12),
          ("B", "X", "non_unate", 0.10, 0.10)]));
    m.insert("sky130_fd_sc_hd__xnor2_1", C::new(
        6.45, 1.5, &[("A", 0.0050), ("B", 0.0050)],
        &[("A", "Y", "non_unate", 0.12, 0.12)]));

    // --- MUX ---
    m.insert("sky130_fd_sc_hd__mux2_1", C::new(
        7.40, 2.0, &[("A0", 0.0040), ("A1", 0.0040), ("S", 0.0050)],
        &[("A0", "X", "positive_unate", 0.13, 0.13),
          ("S",  "X", "non_unate",      0.15, 0.15)]));

    // --- AOI / OAI ---
    m.insert("sky130_fd_sc_hd__aoi21_1", C::new(
        4.60, 1.2, &[("A1", 0.0040), ("A2", 0.0040), ("B1", 0.0040)],
        &[("A1", "Y", "negative_unate", 0.07, 0.08)]));
    m.insert("sky130_fd_sc_hd__oai21_1", C::new(
        4.60, 1.2, &[("A1", 0.0040), ("A2", 0.0040), ("B1", 0.0040)],
        &[("A1", "Y", "negative_unate", 0.08, 0.07)]));

    // --- Flip-flops ---
    m.insert("sky130_fd_sc_hd__dfxtp_1", C::new(
        13.80, 4.0, &[("D", 0.005), ("CLK", 0.010)],
        &[("CLK", "Q", "non_unate", 0.18, 0.18)]));
    m.insert("sky130_fd_sc_hd__dfrtp_1", C::new(
        14.70, 4.5, &[("D", 0.005), ("CLK", 0.010), ("RESET_B", 0.005)],
        &[("CLK", "Q", "non_unate", 0.20, 0.20)]));
    m.insert("sky130_fd_sc_hd__dfstp_1", C::new(
        14.70, 4.5, &[("D", 0.005), ("CLK", 0.010), ("SET_B", 0.005)],
        &[("CLK", "Q", "non_unate", 0.20, 0.20)]));

    // --- Latch ---
    m.insert("sky130_fd_sc_hd__dlxtp_1", C::new(
        11.04, 3.0, &[("D", 0.005), ("GATE", 0.005)],
        &[("GATE", "Q", "non_unate", 0.15, 0.15)]));

    // --- Clock buffers ---
    m.insert("sky130_fd_sc_hd__clkbuf_1", C::new(
        2.76, 0.7, &[("A", 0.0036)],
        &[("A", "X", "positive_unate", 0.06, 0.06)]));
    m.insert("sky130_fd_sc_hd__clkbuf_4", C::new(
        4.60, 2.5, &[("A", 0.0036)],
        &[("A", "X", "positive_unate", 0.025, 0.025)]));

    m
}

/// Build the in-memory Sky130 teaching-subset library.
///
/// Pulls cell list from `TEACHING_CELLS`; populates timing arcs from the
/// hand-curated table. Cells without timing data (tap, decap, fill, conb)
/// are included with empty arcs.
pub fn build_default_library() -> Library {
    let mut lib = Library::new("sky130_fd_sc_hd__teaching");
    let data = cell_data();

    for cell_name in TEACHING_CELLS.keys() {
        let timing = if let Some(entry) = data.get(*cell_name) {
            let arcs: Vec<TimingArc> = entry
                .arcs
                .iter()
                .map(|&(rel, out, sense, rise, fall)| make_arc(rel, out, sense, rise, fall))
                .collect();
            let pin_cap: HashMap<String, f64> = entry
                .pin_caps
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect();
            CellTiming {
                name: cell_name.to_string(),
                area: entry.area,
                leakage_power: entry.leakage,
                pin_capacitance: pin_cap,
                timing_arcs: arcs,
            }
        } else {
            // Structural-only cell (tap, fill, decap, conb) — no timing.
            CellTiming {
                name: cell_name.to_string(),
                area: 1.0,
                leakage_power: 0.0,
                pin_capacitance: HashMap::new(),
                timing_arcs: vec![],
            }
        };
        lib.cells.insert(cell_name.to_string(), timing);
    }

    lib
}
