use asic_floorplan::{
    compute_floorplan, floorplan_to_def, CellInstanceEstimate, FloorplanError, FloorplanOptions, IoSpec,
};
use lef_def::Direction;

fn adder_cells() -> Vec<CellInstanceEstimate> {
    vec![
        CellInstanceEstimate { instance_name: "xor_0".into(), cell_type: "xor2_1".into(), area: 6.45 },
        CellInstanceEstimate { instance_name: "xor_1".into(), cell_type: "xor2_1".into(), area: 6.45 },
        CellInstanceEstimate { instance_name: "and_0".into(), cell_type: "and2_1".into(), area: 4.60 },
        CellInstanceEstimate { instance_name: "or_0".into(),  cell_type: "or2_1".into(),  area: 4.60 },
    ]
}

#[test]
fn test_compute_floorplan_basic() {
    let fp = compute_floorplan(&adder_cells(), &[], &FloorplanOptions::sky130_hd()).unwrap();
    assert!(fp.die.width() > 0.0);
    assert!(fp.die.height() > 0.0);
}

#[test]
fn test_die_contains_core() {
    let fp = compute_floorplan(&adder_cells(), &[], &FloorplanOptions::sky130_hd()).unwrap();
    assert!(fp.core.x1 >= fp.die.x1);
    assert!(fp.core.y1 >= fp.die.y1);
    assert!(fp.core.x2 <= fp.die.x2);
    assert!(fp.core.y2 <= fp.die.y2);
}

#[test]
fn test_rows_count_positive() {
    let fp = compute_floorplan(&adder_cells(), &[], &FloorplanOptions::sky130_hd()).unwrap();
    assert!(!fp.rows.is_empty());
}

#[test]
fn test_row_orientation_alternates() {
    let fp = compute_floorplan(&adder_cells(), &[], &FloorplanOptions::sky130_hd()).unwrap();
    if fp.rows.len() >= 2 {
        assert_eq!(fp.rows[0].orientation, "N");
        assert_eq!(fp.rows[1].orientation, "FS");
    }
}

#[test]
fn test_components_unplaced() {
    let fp = compute_floorplan(&adder_cells(), &[], &FloorplanOptions::sky130_hd()).unwrap();
    for c in &fp.components {
        assert!(!c.placed);
    }
}

#[test]
fn test_io_pins_placed() {
    let io = vec![
        IoSpec { name: "a".into(), direction: Direction::Input, use_: lef_def::Use::Signal },
        IoSpec { name: "y".into(), direction: Direction::Output, use_: lef_def::Use::Signal },
    ];
    let fp = compute_floorplan(&adder_cells(), &io, &FloorplanOptions::sky130_hd()).unwrap();
    assert_eq!(fp.pins.len(), 2);
}

#[test]
fn test_floorplan_to_def_sets_design() {
    let fp = compute_floorplan(&adder_cells(), &[], &FloorplanOptions::sky130_hd()).unwrap();
    let def = floorplan_to_def(&fp, "adder4");
    assert_eq!(def.design, "adder4");
    assert!(def.die_area.is_some());
}

#[test]
fn test_zero_area_error() {
    let cells = vec![
        CellInstanceEstimate { instance_name: "x".into(), cell_type: "x".into(), area: 0.0 },
    ];
    let result = compute_floorplan(&cells, &[], &FloorplanOptions::sky130_hd());
    assert!(matches!(result, Err(FloorplanError::ZeroArea)));
}

#[test]
fn test_invalid_utilization_error() {
    let mut opts = FloorplanOptions::sky130_hd();
    opts.utilization = 1.5;
    let result = compute_floorplan(&adder_cells(), &[], &opts);
    assert!(matches!(result, Err(FloorplanError::InvalidUtilization(_))));
}

#[test]
fn test_high_utilization_smaller_area() {
    let cells = adder_cells();
    let mut opts_lo = FloorplanOptions::sky130_hd();
    opts_lo.utilization = 0.3;
    let mut opts_hi = FloorplanOptions::sky130_hd();
    opts_hi.utilization = 0.9;
    let fp_lo = compute_floorplan(&cells, &[], &opts_lo).unwrap();
    let fp_hi = compute_floorplan(&cells, &[], &opts_hi).unwrap();
    assert!(fp_lo.die.area() > fp_hi.die.area());
}
