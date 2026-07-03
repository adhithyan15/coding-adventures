//! Drive-strength selection.
//!
//! Given a target load (fF) and optional target delay (ns), picks the
//! smallest drive strength in the library that can meet the constraint.
//!
//! ## Why this matters
//!
//! Standard cells come in multiple drive strengths (1, 2, 4, 8, …). A
//! larger drive strength:
//! - Has lower output resistance → shorter delay at the same load
//! - Has higher input capacitance → more load on *its* driver
//! - Occupies more area
//!
//! The classic sizing rule: pick the *smallest* cell that meets the timing
//! budget. This minimises area and input cap (which helps upstream).

use crate::library::Library;

/// Pick the smallest drive strength for `base_name` that drives
/// `target_load_ff` fF within `target_delay_ns` ns.
///
/// - If `target_delay_ns` is `None`, returns the smallest available drive.
/// - If no cell meets the budget, returns the largest available as best-effort.
///
/// Delay is estimated at slew=0.05 ns (a typical internal slew), using the
/// worst of cell_rise and cell_fall.
///
/// # Panics
/// Panics if no drives are available for `base_name`.
pub fn select_drive(
    lib: &Library,
    base_name: &str,
    target_load_ff: f64,
    target_delay_ns: Option<f64>,
) -> String {
    let drives = lib.list_drives(base_name);
    assert!(!drives.is_empty(), "no drives found for {base_name:?}");

    if target_delay_ns.is_none() {
        return format!("{base_name}_{}", drives[0]);
    }
    let max_delay = target_delay_ns.unwrap();

    let ref_slew = 0.05; // ns — typical internal slew
    for drive in &drives {
        let cell_name = format!("{base_name}_{drive}");
        if let Some(cell) = lib.get(&cell_name) {
            if cell.timing_arcs.is_empty() { continue; }
            let arc = &cell.timing_arcs[0];
            let rise = arc.cell_rise.lookup(ref_slew, target_load_ff);
            let fall = arc.cell_fall.lookup(ref_slew, target_load_ff);
            let worst = rise.max(fall);
            if worst <= max_delay {
                return cell_name;
            }
        }
    }
    // No cell met the budget — return the largest (best-effort).
    format!("{base_name}_{}", drives[drives.len() - 1])
}
