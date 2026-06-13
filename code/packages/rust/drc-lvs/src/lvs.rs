//! Layout vs Schematic (LVS) comparison.
//!
//! Strategy: bag-of-cell-signatures comparison via partition refinement.
//!
//! ## Algorithm
//!
//! 1. For each net in a netlist, compute a **connectivity signature**: the
//!    sorted list of `"cell_type.pin_name"` strings for every cell-pin that
//!    touches that net. Nets with identical signatures are equivalent.
//!
//! 2. For each cell, replace each pin's net-name with the net's equivalence
//!    class signature to form a **cell signature**: `"cell_type(pin=sig,...)"`.
//!
//! 3. Compare the multisets of cell signatures from both netlists. If they
//!    match, the netlists are topologically equivalent.
//!
//! This is approximate — it won't catch all subtle topological differences —
//! but it covers the common cases and is O(n log n) in cell count.

use std::collections::HashMap;

/// One cell instance in a flat netlist.
#[derive(Debug, Clone, PartialEq)]
pub struct LvsCell {
    pub name: String,
    pub cell_type: String,
    /// `(pin_name, net_name)` pairs.
    pub pins: Vec<(String, String)>,
}

/// A flat netlist for LVS comparison.
#[derive(Debug, Clone, Default)]
pub struct LvsNetlist {
    pub cells: Vec<LvsCell>,
}

/// LVS result.
#[derive(Debug, Default)]
pub struct LvsReport {
    pub matched: bool,
    pub layout_cells: usize,
    pub schematic_cells: usize,
    pub mismatches: Vec<String>,
}

/// Compare two flat netlists. Returns `LvsReport::matched = true` if they
/// are topologically equivalent.
pub fn lvs(layout: &LvsNetlist, schematic: &LvsNetlist) -> LvsReport {
    let mut report = LvsReport {
        layout_cells: layout.cells.len(),
        schematic_cells: schematic.cells.len(),
        ..Default::default()
    };

    if layout.cells.len() != schematic.cells.len() {
        report.mismatches.push(format!(
            "cell counts differ: layout={} vs schematic={}",
            layout.cells.len(), schematic.cells.len()
        ));
        return report;
    }

    let layout_nets = net_signatures(layout);
    let schem_nets = net_signatures(schematic);

    // Compare net connectivity profile multisets.
    let mut layout_profiles = sorted_values(&layout_nets);
    let mut schem_profiles = sorted_values(&schem_nets);
    layout_profiles.sort();
    schem_profiles.sort();
    if layout_profiles != schem_profiles {
        report.mismatches.push("net connectivity profiles differ".into());
        return report;
    }

    let layout_sigs = cell_signatures(layout, &layout_nets);
    let schem_sigs = cell_signatures(schematic, &schem_nets);

    let mut ls = layout_sigs.clone();
    let mut ss = schem_sigs.clone();
    ls.sort();
    ss.sort();

    if ls != ss {
        report.mismatches.push("cell signatures differ between layout and schematic".into());
        // Report what's different.
        let in_layout: Vec<_> = ls.iter().filter(|s| !ss.contains(s)).cloned().collect();
        let in_schem: Vec<_> = ss.iter().filter(|s| !ls.contains(s)).cloned().collect();
        if !in_layout.is_empty() {
            report.mismatches.push(format!("in layout only: {:?}", &in_layout[..5.min(in_layout.len())]));
        }
        if !in_schem.is_empty() {
            report.mismatches.push(format!("in schematic only: {:?}", &in_schem[..5.min(in_schem.len())]));
        }
        return report;
    }

    report.matched = true;
    report
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// For each net, build a connectivity signature: sorted "cell_type.pin_name" strings.
fn net_signatures(nl: &LvsNetlist) -> HashMap<String, String> {
    let mut net_to_pins: HashMap<String, Vec<String>> = HashMap::new();
    for cell in &nl.cells {
        for (pin_name, net_name) in &cell.pins {
            net_to_pins
                .entry(net_name.clone())
                .or_default()
                .push(format!("{}.{}", cell.cell_type, pin_name));
        }
    }
    net_to_pins
        .into_iter()
        .map(|(net, mut pins)| {
            pins.sort();
            (net, pins.join(" | "))
        })
        .collect()
}

/// Per-cell signature: `"cell_type(pin=net_sig,...)"`.
fn cell_signatures(nl: &LvsNetlist, net_sigs: &HashMap<String, String>) -> Vec<String> {
    nl.cells
        .iter()
        .map(|cell| {
            let mut pin_sigs: Vec<String> = cell
                .pins
                .iter()
                .map(|(pn, net)| {
                    let sig = net_sigs.get(net).map(|s| s.as_str()).unwrap_or("?");
                    format!("{pn}={sig}")
                })
                .collect();
            pin_sigs.sort();
            format!("{}({})", cell.cell_type, pin_sigs.join(","))
        })
        .collect()
}

fn sorted_values(map: &HashMap<String, String>) -> Vec<String> {
    map.values().cloned().collect()
}
