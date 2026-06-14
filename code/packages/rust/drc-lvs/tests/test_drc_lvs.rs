use drc_lvs::{lvs, run_drc, DrcRect, LvsCell, LvsNetlist, Rule};

// ---------------------------------------------------------------------------
// DRC tests
// ---------------------------------------------------------------------------

fn met1_rect(x1: f64, y1: f64, x2: f64, y2: f64) -> DrcRect {
    DrcRect { layer: "met1".into(), x1, y1, x2, y2 }
}

#[test]
fn test_drc_clean_for_wide_rect() {
    let rects = vec![met1_rect(0.0, 0.0, 1.0, 0.5)];
    let rules = vec![Rule::min_width("met1.W", "met1", 0.14)];
    let report = run_drc(&rects, &rules);
    assert!(report.clean(), "should be clean: {:?}", report.violations);
}

#[test]
fn test_drc_min_width_violation() {
    let rects = vec![met1_rect(0.0, 0.0, 0.10, 0.28)];
    let rules = vec![Rule::min_width("met1.W", "met1", 0.14)];
    let report = run_drc(&rects, &rules);
    assert!(!report.clean());
    assert_eq!(report.violations.len(), 1);
}

#[test]
fn test_drc_min_spacing_clean() {
    // Two rects 0.20 µm apart on x, rule = 0.14 → clean.
    let rects = vec![
        met1_rect(0.0, 0.0, 0.14, 0.28),
        met1_rect(0.34, 0.0, 0.48, 0.28),
    ];
    let rules = vec![Rule::min_spacing("met1.S", "met1", 0.14)];
    let report = run_drc(&rects, &rules);
    assert!(report.clean(), "{:?}", report.violations);
}

#[test]
fn test_drc_min_spacing_violation() {
    // Two rects only 0.05 µm apart.
    let rects = vec![
        met1_rect(0.0, 0.0, 0.14, 0.28),
        met1_rect(0.19, 0.0, 0.33, 0.28),
    ];
    let rules = vec![Rule::min_spacing("met1.S", "met1", 0.14)];
    let report = run_drc(&rects, &rules);
    assert!(!report.clean());
}

#[test]
fn test_drc_min_area_violation() {
    // 0.10 × 0.10 = 0.01 µm² < 0.02 µm² rule.
    let rects = vec![met1_rect(0.0, 0.0, 0.10, 0.10)];
    let rules = vec![Rule::min_area("met1.A", "met1", 0.02)];
    let report = run_drc(&rects, &rules);
    assert!(!report.clean());
}

#[test]
fn test_drc_empty_rects_clean() {
    let rules = vec![Rule::min_width("met1.W", "met1", 0.14)];
    let report = run_drc(&[], &rules);
    assert!(report.clean());
    assert_eq!(report.violations.len(), 0);
}

#[test]
fn test_drc_rules_checked_count() {
    let rules = vec![
        Rule::min_width("a", "met1", 0.14),
        Rule::min_spacing("b", "met1", 0.14),
    ];
    let report = run_drc(&[], &rules);
    assert_eq!(report.rules_checked, 2);
}

#[test]
fn test_drc_overlapping_rects_no_spacing_violation() {
    // Overlapping rects → spacing = -1 → not caught by min_spacing rule (which checks 0..value).
    let rects = vec![
        met1_rect(0.0, 0.0, 1.0, 1.0),
        met1_rect(0.5, 0.5, 1.5, 1.5),
    ];
    let rules = vec![Rule::min_spacing("met1.S", "met1", 0.14)];
    let report = run_drc(&rects, &rules);
    // No spacing violation for overlapping rects (spacing = -1).
    assert!(report.clean());
}

// ---------------------------------------------------------------------------
// LVS tests
// ---------------------------------------------------------------------------

fn inv_cell(name: &str, a_net: &str, y_net: &str) -> LvsCell {
    LvsCell {
        name: name.into(),
        cell_type: "inv_1".into(),
        pins: vec![("A".into(), a_net.into()), ("Y".into(), y_net.into())],
    }
}

#[test]
fn test_lvs_identical_netlists_match() {
    let cell = inv_cell("inv_0", "a", "y");
    let nl = LvsNetlist { cells: vec![cell] };
    let report = lvs(&nl, &nl);
    assert!(report.matched);
    assert!(report.mismatches.is_empty());
}

#[test]
fn test_lvs_different_cell_count_fails() {
    let layout = LvsNetlist { cells: vec![inv_cell("inv_0", "a", "y"), inv_cell("inv_1", "b", "z")] };
    let schem  = LvsNetlist { cells: vec![inv_cell("inv_0", "a", "y")] };
    let report = lvs(&layout, &schem);
    assert!(!report.matched);
    assert!(!report.mismatches.is_empty());
}

#[test]
fn test_lvs_different_connectivity_fails() {
    // Same cells but different net names (different topology).
    let layout = LvsNetlist { cells: vec![inv_cell("inv_0", "net_a", "net_y")] };
    let schem  = LvsNetlist { cells: vec![
        LvsCell {
            name: "inv_0".into(),
            cell_type: "inv_1".into(),
            // Both pins connected to the same net — topologically different.
            pins: vec![("A".into(), "net_x".into()), ("Y".into(), "net_x".into())],
        }
    ]};
    let report = lvs(&layout, &schem);
    assert!(!report.matched);
}

#[test]
fn test_lvs_renamed_cells_still_match() {
    // Instance names differ but topology is identical.
    let layout = LvsNetlist { cells: vec![inv_cell("u1", "a", "b")] };
    let schem  = LvsNetlist { cells: vec![inv_cell("different_name", "a", "b")] };
    let report = lvs(&layout, &schem);
    assert!(report.matched, "mismatches: {:?}", report.mismatches);
}

#[test]
fn test_lvs_two_cell_chain_matches() {
    // inv → inv chain.
    let make = |_n: &str| LvsNetlist {
        cells: vec![
            inv_cell("u1", "in", "mid"),
            inv_cell("u2", "mid", "out"),
        ],
    };
    let layout = make("l");
    let schem  = make("s");
    let report = lvs(&layout, &schem);
    assert!(report.matched);
}
