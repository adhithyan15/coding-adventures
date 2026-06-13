use standard_cell_library::{build_default_library, select_drive, LookupTable};

// ---------------------------------------------------------------------------
// LUT tests
// ---------------------------------------------------------------------------

fn three_by_three_lut() -> LookupTable {
    LookupTable {
        slew_index: vec![0.0, 1.0, 2.0],
        load_index: vec![0.0, 1.0, 2.0],
        values: vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 2.0, 3.0],
            vec![2.0, 3.0, 4.0],
        ],
    }
}

#[test]
fn test_lut_exact_grid_corner() {
    let lut = three_by_three_lut();
    assert!((lut.lookup(0.0, 0.0) - 0.0).abs() < 1e-9);
    assert!((lut.lookup(2.0, 2.0) - 4.0).abs() < 1e-9);
}

#[test]
fn test_lut_midpoint_interpolation() {
    let lut = three_by_three_lut();
    // At (slew=1.0, load=1.0), which is a grid point, expect value = 2.0.
    assert!((lut.lookup(1.0, 1.0) - 2.0).abs() < 1e-9);
    // At (0.5, 0.5) bilinear of [0,1,1,2] with equal weights → 1.0.
    assert!((lut.lookup(0.5, 0.5) - 1.0).abs() < 1e-9);
}

#[test]
fn test_lut_clamp_below() {
    let lut = three_by_three_lut();
    // Below the first grid point → clamped to first row/col.
    assert!((lut.lookup(-1.0, 0.0) - 0.0).abs() < 1e-9);
}

#[test]
fn test_lut_clamp_above() {
    let lut = three_by_three_lut();
    // Above the last grid point → clamped to last.
    assert!((lut.lookup(99.0, 99.0) - 4.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// Library tests
// ---------------------------------------------------------------------------

#[test]
fn test_library_has_all_teaching_cells() {
    let lib = build_default_library();
    // Library should contain at least as many cells as TEACHING_CELLS.
    assert!(lib.cells.len() >= 33);
}

#[test]
fn test_inv_1_has_timing_arc() {
    let lib = build_default_library();
    let c = lib.get("sky130_fd_sc_hd__inv_1").unwrap();
    assert!(!c.timing_arcs.is_empty());
    assert_eq!(c.timing_arcs[0].sense, "negative_unate");
}

#[test]
fn test_xor2_1_two_arcs() {
    let lib = build_default_library();
    let c = lib.get("sky130_fd_sc_hd__xor2_1").unwrap();
    assert_eq!(c.timing_arcs.len(), 2);
    for arc in &c.timing_arcs { assert_eq!(arc.sense, "non_unate"); }
}

#[test]
fn test_dfxtp_clk_to_q_arc() {
    let lib = build_default_library();
    let c = lib.get("sky130_fd_sc_hd__dfxtp_1").unwrap();
    let arc = &c.timing_arcs[0];
    assert_eq!(arc.related_pin, "CLK");
    assert_eq!(arc.output_pin, "Q");
}

#[test]
fn test_conb_no_timing_arcs() {
    let lib = build_default_library();
    let c = lib.get("sky130_fd_sc_hd__conb_1").unwrap();
    assert!(c.timing_arcs.is_empty());
}

#[test]
fn test_area_greater_than_zero() {
    let lib = build_default_library();
    for (name, cell) in &lib.cells {
        assert!(cell.area > 0.0, "{name}: area = {}", cell.area);
    }
}

#[test]
fn test_delay_increases_with_load() {
    let lib = build_default_library();
    let arc = &lib.get("sky130_fd_sc_hd__inv_1").unwrap().timing_arcs[0];
    let d1 = arc.cell_rise.lookup(0.05, 1.0);
    let d2 = arc.cell_rise.lookup(0.05, 10.0);
    assert!(d2 > d1, "delay should grow with load: {d1} < {d2}");
}

#[test]
fn test_list_drives_inv() {
    let lib = build_default_library();
    let drives = lib.list_drives("sky130_fd_sc_hd__inv");
    assert_eq!(drives, vec![1, 2, 4, 8]);
}

#[test]
fn test_list_drives_buf() {
    let lib = build_default_library();
    let drives = lib.list_drives("sky130_fd_sc_hd__buf");
    assert_eq!(drives, vec![1, 2, 4, 8]);
}

// ---------------------------------------------------------------------------
// Drive-selection tests
// ---------------------------------------------------------------------------

#[test]
fn test_select_drive_no_constraint_returns_smallest() {
    let lib = build_default_library();
    let best = select_drive(&lib, "sky130_fd_sc_hd__inv", 1.0, None);
    assert_eq!(best, "sky130_fd_sc_hd__inv_1");
}

#[test]
fn test_select_drive_tight_constraint_returns_larger() {
    let lib = build_default_library();
    // Very tight delay budget: 0.03 ns → inv_1 and inv_2 will be too slow,
    // should return inv_4 or inv_8.
    let best = select_drive(&lib, "sky130_fd_sc_hd__inv", 2.0, Some(0.03));
    let drive: u32 = best.rsplit('_').next().unwrap().parse().unwrap();
    assert!(drive >= 4, "expected drive >= 4, got {drive}");
}

#[test]
fn test_select_drive_impossible_returns_largest() {
    let lib = build_default_library();
    // Impossible budget (0.0001 ns) → should return largest available.
    let best = select_drive(&lib, "sky130_fd_sc_hd__inv", 10.0, Some(0.0001));
    assert_eq!(best, "sky130_fd_sc_hd__inv_8");
}
