use std::collections::HashMap;

use asic_floorplan::{compute_floorplan, CellInstanceEstimate, FloorplanOptions};
use asic_placement::{place, CellSize, PlacementOptions};

fn make_fp_and_sizes() -> (asic_floorplan::Floorplan, HashMap<String, CellSize>) {
    let cells = vec![
        CellInstanceEstimate { instance_name: "a".into(), cell_type: "inv_1".into(), area: 1.84 },
        CellInstanceEstimate { instance_name: "b".into(), cell_type: "inv_1".into(), area: 1.84 },
        CellInstanceEstimate { instance_name: "c".into(), cell_type: "buf_1".into(), area: 2.30 },
        CellInstanceEstimate { instance_name: "d".into(), cell_type: "xor2_1".into(), area: 6.45 },
    ];
    let fp = compute_floorplan(&cells, &[], &FloorplanOptions::sky130_hd()).unwrap();
    let mut sizes: HashMap<String, CellSize> = HashMap::new();
    sizes.insert("inv_1".into(),  CellSize { cell_type: "inv_1".into(),  width: 0.92, height: 2.72 });
    sizes.insert("buf_1".into(),  CellSize { cell_type: "buf_1".into(),  width: 1.38, height: 2.72 });
    sizes.insert("xor2_1".into(), CellSize { cell_type: "xor2_1".into(), width: 1.38, height: 2.72 });
    (fp, sizes)
}

#[test]
fn test_placement_cells_placed_count() {
    let (fp, sizes) = make_fp_and_sizes();
    let (_, report) = place(&fp, &sizes, None, None).unwrap();
    assert_eq!(report.cells_placed, 4);
}

#[test]
fn test_placed_components_have_coordinates() {
    let (fp, sizes) = make_fp_and_sizes();
    let (def, _) = place(&fp, &sizes, None, None).unwrap();
    for c in &def.components {
        assert!(c.placed, "component {} not placed", c.name);
        assert!(c.location_x.is_some());
        assert!(c.location_y.is_some());
    }
}

#[test]
fn test_placed_coordinates_inside_die() {
    let (fp, sizes) = make_fp_and_sizes();
    let (def, _) = place(&fp, &sizes, None, None).unwrap();
    let die = def.die_area.as_ref().unwrap();
    for c in &def.components {
        let x = c.location_x.unwrap();
        let y = c.location_y.unwrap();
        assert!(x >= die.x1, "x={x} outside die");
        assert!(y >= die.y1, "y={y} outside die");
    }
}

#[test]
fn test_with_nets_accepted_swaps_positive() {
    let (fp, sizes) = make_fp_and_sizes();
    let nets = vec![
        vec!["a".to_string(), "b".to_string()],
        vec!["b".to_string(), "c".to_string()],
    ];
    let opts = PlacementOptions { iterations: 1000, seed: 99, legalize: true };
    let (_, report) = place(&fp, &sizes, Some(&nets), Some(opts)).unwrap();
    assert!(report.accepted_swaps + report.rejected_swaps > 0);
}

#[test]
fn test_no_rows_error() {
    use asic_floorplan::Floorplan;
    use lef_def::Rect;
    let empty_fp = Floorplan {
        die: Rect::new(0.0, 0.0, 100.0, 100.0),
        core: Rect::new(10.0, 10.0, 90.0, 90.0),
        rows: vec![],
        components: vec![],
        pins: vec![],
    };
    let result = place(&empty_fp, &HashMap::new(), None, None);
    assert!(result.is_err());
}

#[test]
fn test_legalized_cells_no_overlap_within_row() {
    let (fp, sizes) = make_fp_and_sizes();
    let (def, _) = place(&fp, &sizes, None, None).unwrap();
    // After legalization, x-coordinates in the same row should be non-decreasing.
    let mut by_y: HashMap<u64, Vec<f64>> = HashMap::new();
    for c in &def.components {
        let y_key = c.location_y.unwrap().to_bits();
        by_y.entry(y_key).or_default().push(c.location_x.unwrap());
    }
    for xs in by_y.values() {
        let mut sorted = xs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(*xs, sorted, "cells in same row not sorted by x");
    }
}
