//! HNL → FPGA JSON config bridge.
//!
//! ## LUT expansion
//!
//! An n-input cell has a truth table of 2^n entries.  A physical LUT has
//! `lut_inputs` (default 4) inputs, so its table has 2^lut_inputs = 16 entries.
//! Expansion works by copying the n-input table repeatedly so the high-order
//! bits (which don't exist in the cell) don't affect the output:
//!
//! ```text
//! NOT (1 input, 2-entry table [1, 0]) expanded to 4-input:
//!   indices 0,2,4,6,8,10,12,14 all use table[i % 2 = 0] = 1
//!   indices 1,3,5,7,9,11,13,15 all use table[i % 2 = 1] = 0
//!   → [1,0, 1,0, 1,0, 1,0, 1,0, 1,0, 1,0, 1,0]
//! ```
//!
//! ## CLB naming
//!
//! Each CLB is named `clb_{row}_{col}` using row-major layout:
//! ```text
//! cell index 0 → (row=0, col=0) → "clb_0_0"
//! cell index 1 → (row=0, col=1) → "clb_0_1"
//! cell index 4 → (row=1, col=0) → "clb_1_0"  (for cols=4)
//! ```

use std::collections::HashMap;

use gate_netlist_format::{Direction, Netlist};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Truth table definitions
// ---------------------------------------------------------------------------

/// Look up the truth table for a built-in FPGA primitive cell.
///
/// Returns `Some((input_pin_names, table))` where `table[combo]` is the output
/// value when the inputs form the integer `combo` (bit 0 = first pin, etc.).
/// Returns `None` for unrecognised cell types.
///
/// All pins in the returned slice are stable static strings.
pub fn truth_table(cell_type: &str) -> Option<(&'static [&'static str], &'static [u8])> {
    match cell_type {
        "BUF"     => Some((&["A"],            &[0, 1])),
        "NOT"     => Some((&["A"],            &[1, 0])),
        "AND2"    => Some((&["A", "B"],       &[0, 0, 0, 1])),
        "OR2"     => Some((&["A", "B"],       &[0, 1, 1, 1])),
        "NAND2"   => Some((&["A", "B"],       &[1, 1, 1, 0])),
        "NOR2"    => Some((&["A", "B"],       &[1, 0, 0, 0])),
        "XOR2"    => Some((&["A", "B"],       &[0, 1, 1, 0])),
        "XNOR2"   => Some((&["A", "B"],       &[1, 0, 0, 1])),
        "AND3"    => Some((&["A", "B", "C"],  &[0, 0, 0, 0, 0, 0, 0, 1])),
        "OR3"     => Some((&["A", "B", "C"],  &[0, 1, 1, 1, 1, 1, 1, 1])),
        "NAND3"   => Some((&["A", "B", "C"],  &[1, 1, 1, 1, 1, 1, 1, 0])),
        "NOR3"    => Some((&["A", "B", "C"],  &[1, 0, 0, 0, 0, 0, 0, 0])),
        "XOR3"    => Some((&["A", "B", "C"],  &[0, 1, 1, 0, 1, 0, 0, 1])),
        "AND4"    => Some((&["A", "B", "C", "D"], {
            static T: [u8; 16] = [0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,1];
            &T
        })),
        "OR4"     => Some((&["A", "B", "C", "D"], {
            static T: [u8; 16] = [0,1,1,1,1,1,1,1, 1,1,1,1,1,1,1,1];
            &T
        })),
        "NAND4"   => Some((&["A", "B", "C", "D"], {
            static T: [u8; 16] = [1,1,1,1,1,1,1,1, 1,1,1,1,1,1,1,0];
            &T
        })),
        "NOR4"    => Some((&["A", "B", "C", "D"], {
            static T: [u8; 16] = [1,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0];
            &T
        })),
        // MUX2: pins (A, B, S); Y = S ? B : A
        // combo = S<<2 | B<<1 | A; output table indexed by combo:
        //   000 → Y=A=0  001 → Y=A=1
        //   010 → Y=A=0  011 → Y=A=1
        //   100 → Y=B=0  101 → Y=B=0
        //   110 → Y=B=1  111 → Y=B=1
        "MUX2"    => Some((&["A", "B", "S"],  &[0, 1, 0, 1, 0, 0, 1, 1])),
        "CONST_0" => Some((&[],               &[0])),
        "CONST_1" => Some((&[],               &[1])),
        _         => None,
    }
}

/// All cell type names for which `truth_table` returns `Some(...)`.
pub fn truth_table_types() -> &'static [&'static str] {
    &[
        "BUF", "NOT",
        "AND2", "OR2", "NAND2", "NOR2", "XOR2", "XNOR2",
        "AND3", "OR3", "NAND3", "NOR3", "XOR3",
        "AND4", "OR4", "NAND4", "NOR4",
        "MUX2", "CONST_0", "CONST_1",
    ]
}

// ---------------------------------------------------------------------------
// Options + Report
// ---------------------------------------------------------------------------

/// Configuration knobs for `hnl_to_fpga_json`.
#[derive(Debug, Clone)]
pub struct FpgaBridgeOptions {
    pub rows:       usize,
    pub cols:       usize,
    pub lut_inputs: usize,
    pub seed:       u64,
}

impl Default for FpgaBridgeOptions {
    fn default() -> Self {
        Self { rows: 4, cols: 4, lut_inputs: 4, seed: 42 }
    }
}

/// Statistics produced by `hnl_to_fpga_json`.
#[derive(Debug, Clone, Default)]
pub struct FpgaBridgeReport {
    pub cells_packed:   usize,
    pub cells_unmapped: Vec<String>,
    pub routes_emitted: usize,
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Map an HNL to a FPGA-package-style JSON config.
///
/// Returns `(json_value, report)`.  The JSON has the shape:
///
/// ```json
/// {
///   "device": { "name": "…", "rows": 4, "cols": 4, "lut_inputs": 4, "io_pins": N },
///   "clbs":   { "clb_0_0": { "lut_a": { "truth_table": [0,…], "comment": "…" } } },
///   "routing": [ { "from": "net_a_0", "to": "clb_0_0.lut_a.in0" }, … ],
///   "io":     { "io_pin_a": { "direction": "input", "name": "a" }, … }
/// }
/// ```
pub fn hnl_to_fpga_json(
    netlist: &Netlist,
    options: Option<&FpgaBridgeOptions>,
) -> (Value, FpgaBridgeReport) {
    let default_opts = FpgaBridgeOptions::default();
    let opts = options.unwrap_or(&default_opts);

    let top_mod = &netlist.modules[&netlist.top];
    let mut report = FpgaBridgeReport::default();

    // We collect CLBs, routes, and IO into plain structures then serialise.
    let mut clbs: HashMap<String, Value>            = HashMap::new();
    let mut routes: Vec<Value>                      = Vec::new();
    let mut io:  HashMap<String, Value>             = HashMap::new();

    let mut cell_idx: usize = 0;

    for inst in &top_mod.instances {
        let Some((input_pins, table)) = truth_table(&inst.cell_type) else {
            report.cells_unmapped.push(inst.cell_type.clone());
            continue;
        };
        report.cells_packed += 1;

        let (row, col) = clb_location(cell_idx, opts.cols);
        let clb_name   = format!("clb_{row}_{col}");
        cell_idx      += 1;

        let expanded = expand_truth_table(table, input_pins.len(), opts.lut_inputs);

        clbs.insert(clb_name.clone(), json!({
            "lut_a": {
                "truth_table": expanded,
                "comment": format!("{} ({})", inst.name, inst.cell_type),
            }
        }));

        // Route each input pin connection
        for (i, &pin_name) in input_pins.iter().enumerate() {
            if let Some(slice) = inst.connections.get(pin_name) {
                let bit    = slice.bits.first().copied().unwrap_or(0);
                let source = format!("net_{}_{}", slice.net, bit);
                let target = format!("{clb_name}.lut_a.in{i}");
                routes.push(json!({ "from": source, "to": target }));
                report.routes_emitted += 1;
            }
        }

        // If this cell drives a top-level output port, add a route to the io_pin
        for (pin_name, slice) in &inst.connections {
            if input_pins.contains(&pin_name.as_str()) {
                continue; // only process output pins here
            }
            let is_output = top_mod.ports.iter().any(|p| {
                p.name == slice.net && matches!(p.direction, Direction::Output)
            });
            if is_output {
                let io_pin = format!("io_pin_{}", slice.net);
                routes.push(json!({ "from": format!("{clb_name}.lut_a.out"), "to": &io_pin }));
                io.insert(io_pin, json!({ "direction": "output", "name": slice.net }));
                report.routes_emitted += 1;
            }
        }
    }

    // IO pins for input ports
    for port in &top_mod.ports {
        if matches!(port.direction, Direction::Input) {
            let io_pin = format!("io_pin_{}", port.name);
            io.insert(io_pin, json!({ "direction": "input", "name": port.name }));
        }
    }

    let config = json!({
        "device": {
            "name":       "CinchFPGA-Mini",
            "rows":       opts.rows,
            "cols":       opts.cols,
            "lut_inputs": opts.lut_inputs,
            "io_pins":    io.len(),
        },
        "clbs":    clbs,
        "routing": routes,
        "io":      io,
    });

    (config, report)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Row-major placement: cell index → (row, col).
fn clb_location(idx: usize, cols: usize) -> (usize, usize) {
    (idx / cols, idx % cols)
}

/// Expand a `2^n_inputs`-entry truth table to a `2^target_inputs`-entry table
/// by repeating the original table's output for the high-order bits that the
/// original cell doesn't have.
///
/// # Panics
///
/// Panics if `n_inputs > target_inputs` (can't shrink).
fn expand_truth_table(table: &[u8], n_inputs: usize, target_inputs: usize) -> Vec<u8> {
    assert!(
        n_inputs <= target_inputs,
        "can't expand {n_inputs}-input truth table to {target_inputs}"
    );
    let target_size = 1usize << target_inputs;
    let n_size      = if n_inputs == 0 { 1 } else { 1usize << n_inputs };
    (0..target_size).map(|i| table[i % n_size]).collect()
}
