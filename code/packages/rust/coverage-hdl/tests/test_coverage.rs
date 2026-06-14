//! Integration tests for coverage-hdl.
//!
//! Uses in-memory HIRs (no files) so the suite is self-contained.

use coverage_hdl::{
    bin_default, bin_range, bin_value, Coverpoint, CoverageRecorder, CrossPoint,
};
use hardware_vm::HardwareVm;
use hdl_ir::{ContAssign, Direction, Expr, Hir, Module, Port, Ty};

// ---------------------------------------------------------------------------
// Tiny HIRs
// ---------------------------------------------------------------------------

fn buffer_hir() -> Hir {
    let m = Module {
        name: "bf".into(),
        ports: vec![
            Port { name: "a".into(), ty: Ty::Bit, direction: Direction::In,  provenance: None },
            Port { name: "y".into(), ty: Ty::Bit, direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::port_ref("y"),
            rhs:    Expr::port_ref("a"),
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("bf");
    hir.modules.insert("bf".into(), m);
    hir
}

fn adder4_hir() -> Hir {
    let m = Module {
        name: "adder4".into(),
        ports: vec![
            Port { name: "a".into(),    ty: Ty::vec(4), direction: Direction::In,  provenance: None },
            Port { name: "b".into(),    ty: Ty::vec(4), direction: Direction::In,  provenance: None },
            Port { name: "cin".into(),  ty: Ty::Bit,    direction: Direction::In,  provenance: None },
            Port { name: "sum".into(),  ty: Ty::vec(4), direction: Direction::Out, provenance: None },
            Port { name: "cout".into(), ty: Ty::Bit,    direction: Direction::Out, provenance: None },
        ],
        cont_assigns: vec![ContAssign {
            target: Expr::Concat {
                parts: vec![Expr::port_ref("cout"), Expr::port_ref("sum")],
                provenance: None,
            },
            rhs: Expr::Binary {
                op: "+".into(),
                lhs: Box::new(Expr::Binary {
                    op: "+".into(),
                    lhs: Box::new(Expr::port_ref("a")),
                    rhs: Box::new(Expr::port_ref("b")),
                    provenance: None,
                }),
                rhs: Box::new(Expr::port_ref("cin")),
                provenance: None,
            },
            provenance: None,
        }],
        ..Default::default()
    };
    let mut hir = Hir::new("adder4");
    hir.modules.insert("adder4".into(), m);
    hir
}

// ---------------------------------------------------------------------------
// Bin constructors
// ---------------------------------------------------------------------------

#[test]
fn test_bin_value_matches_exact() {
    let b = bin_value("exact", 5i64);
    assert!((b.matcher)(5));
    assert!(!(b.matcher)(6));
    assert!(!(b.matcher)(0));
}

#[test]
fn test_bin_range_inclusive() {
    let b = bin_range("mid", 1i64, 10i64);
    assert!((b.matcher)(1));
    assert!((b.matcher)(5));
    assert!((b.matcher)(10));
    assert!(!(b.matcher)(0));
    assert!(!(b.matcher)(11));
}

#[test]
fn test_bin_default_matches_all() {
    let b = bin_default();
    assert!((b.matcher)(0));
    assert!((b.matcher)(999));
    assert!((b.matcher)(i64::MIN));
    assert!((b.matcher)(i64::MAX));
}

// ---------------------------------------------------------------------------
// Coverpoint
// ---------------------------------------------------------------------------

#[test]
fn test_coverpoint_initial_zero_hits() {
    let cp = Coverpoint::new("x", "s", vec![bin_value("a", 0), bin_value("b", 1)]);
    assert_eq!(cp.hits["a"], 0);
    assert_eq!(cp.hits["b"], 0);
}

#[test]
fn test_coverpoint_sample_first_match_wins() {
    let mut cp = Coverpoint::new(
        "x", "s",
        vec![bin_range("low", 0, 5), bin_range("high", 6, 10)],
    );
    cp.sample(3);
    assert_eq!(cp.hits["low"],  1);
    assert_eq!(cp.hits["high"], 0);
    cp.sample(8);
    assert_eq!(cp.hits["high"], 1);
}

#[test]
fn test_coverpoint_unmatched_value_no_effect() {
    let mut cp = Coverpoint::new("x", "s", vec![bin_value("a", 0)]);
    cp.sample(99); // no bin matches
    assert_eq!(cp.hits["a"], 0);
}

#[test]
fn test_coverpoint_coverage_fraction() {
    let mut cp = Coverpoint::new("x", "s", vec![bin_value("a", 0), bin_value("b", 1)]);
    assert_eq!(cp.coverage(), 0.0);
    cp.sample(0);
    assert_eq!(cp.coverage(), 0.5);
    cp.sample(1);
    assert_eq!(cp.coverage(), 1.0);
}

#[test]
fn test_coverpoint_no_bins_full_coverage() {
    let cp = Coverpoint::new("x", "s", vec![]);
    assert_eq!(cp.coverage(), 1.0);
}

// ---------------------------------------------------------------------------
// CoverageRecorder — toggle
// ---------------------------------------------------------------------------

#[test]
fn test_toggle_rising_and_falling() {
    let mut vm  = HardwareVm::new(buffer_hir()).unwrap();
    let rec     = CoverageRecorder::new(&mut vm);
    rec.enable_toggle_coverage(&["a", "y"]);

    vm.set_input("a", 1).unwrap(); // 0→1 rising
    vm.set_input("a", 0).unwrap(); // 1→0 falling
    vm.set_input("a", 1).unwrap(); // 0→1 rising

    let rep = rec.report();
    assert_eq!(rep.toggle["a"].rising,  2);
    assert_eq!(rep.toggle["a"].falling, 1);
    // y tracks a (assign y = a), so at least one rising on y as well.
    assert!(rep.toggle["y"].rising >= 1);
}

#[test]
fn test_toggle_only_for_enabled_signals() {
    let mut vm = HardwareVm::new(buffer_hir()).unwrap();
    let rec    = CoverageRecorder::new(&mut vm);
    rec.enable_toggle_coverage(&["a"]); // y is NOT enabled

    vm.set_input("a", 1).unwrap();
    let rep = rec.report();
    assert!(rep.toggle.contains_key("a"));
    assert!(!rep.toggle.contains_key("y"));
}

// ---------------------------------------------------------------------------
// CoverageRecorder — coverpoints
// ---------------------------------------------------------------------------

#[test]
fn test_coverpoint_via_vm_subscribe() {
    let mut vm = HardwareVm::new(buffer_hir()).unwrap();
    let rec    = CoverageRecorder::new(&mut vm);

    rec.add_coverpoint(Coverpoint::new(
        "a_val", "a",
        vec![bin_value("zero", 0), bin_value("one", 1)],
    ));

    vm.set_input("a", 1).unwrap(); // event: 0→1

    let rep = rec.report();
    // Only the "one" bin is hit because the initial 0 → 1 event fires after
    // the recorder was already subscribed — and "one" = 1.
    assert_eq!(rep.coverpoints["a_val"]["one"], 1);
    assert_eq!(rep.coverpoints["a_val"]["zero"], 0);
}

#[test]
fn test_overall_coverage_partial_then_full() {
    let mut vm = HardwareVm::new(buffer_hir()).unwrap();
    let rec    = CoverageRecorder::new(&mut vm);

    rec.add_coverpoint(Coverpoint::new(
        "cp1", "a",
        vec![bin_value("zero", 0), bin_value("one", 1)],
    ));

    assert_eq!(rec.overall_coverage(), 0.0);
    vm.set_input("a", 1).unwrap(); // hits "one" only → 50 %
    assert_eq!(rec.overall_coverage(), 0.5);
    vm.set_input("a", 0).unwrap(); // hits "zero" → 100 %
    assert_eq!(rec.overall_coverage(), 1.0);
}

#[test]
fn test_overall_coverage_empty_returns_zero() {
    let mut vm = HardwareVm::new(buffer_hir()).unwrap();
    let rec    = CoverageRecorder::new(&mut vm);
    assert_eq!(rec.overall_coverage(), 0.0);
}

// ---------------------------------------------------------------------------
// CrossPoint
// ---------------------------------------------------------------------------

#[test]
fn test_cross_records_joint_hit() {
    let mut vm = HardwareVm::new(adder4_hir()).unwrap();
    let rec    = CoverageRecorder::new(&mut vm);

    let cp_cin = Coverpoint::new("cin", "cin",
        vec![bin_value("zero", 0), bin_value("one", 1)]);
    let cp_a   = Coverpoint::new("a", "a",
        vec![bin_range("low", 0, 7), bin_range("high", 8, 15)]);

    rec.add_coverpoint(cp_cin.clone());
    rec.add_coverpoint(cp_a.clone());

    let cross = CrossPoint::new("cin_x_a", vec![cp_cin, cp_a]);
    rec.add_cross(cross);

    vm.set_input("a",   5).unwrap(); // a=low
    vm.set_input("cin", 1).unwrap(); // cin=one
    rec.sample_cross(None);          // snapshot the cross

    let rep = rec.report();
    assert_eq!(rep.crosses["cin_x_a"][&vec!["one".to_string(), "low".to_string()]], 1);
}

#[test]
fn test_cross_coverage_zero_when_unsampled() {
    let cp = Coverpoint::new("a", "a", vec![bin_value("z", 0)]);
    let cross = CrossPoint::new("x", vec![cp]);
    assert_eq!(cross.coverage(), 0.0);
}

#[test]
fn test_cross_coverage_no_coverpoints_is_full() {
    let cross = CrossPoint::new("x", vec![]);
    assert_eq!(cross.coverage(), 1.0);
}

#[test]
fn test_cross_skips_unmatched_value() {
    let mut vm = HardwareVm::new(buffer_hir()).unwrap();
    let rec    = CoverageRecorder::new(&mut vm);

    // bin only matches value 5; we'll drive a=1 which won't match
    let cp = Coverpoint::new("a", "a", vec![bin_value("only_5", 5)]);
    rec.add_coverpoint(cp.clone());
    let cross = CrossPoint::new("x", vec![cp]);
    rec.add_cross(cross);

    vm.set_input("a", 1).unwrap(); // not 5 → no cross sample
    rec.sample_cross(None);

    let rep = rec.report();
    assert!(rep.crosses["x"].is_empty(), "expected no cross hits");
}

#[test]
fn test_sample_cross_by_name() {
    let mut vm  = HardwareVm::new(buffer_hir()).unwrap();
    let rec     = CoverageRecorder::new(&mut vm);

    let cp = Coverpoint::new("cp", "a", vec![bin_default()]);
    rec.add_coverpoint(cp.clone());
    let cross = CrossPoint::new("named_cross", vec![cp]);
    rec.add_cross(cross);

    vm.set_input("a", 3).unwrap();
    rec.sample_cross(Some("named_cross")); // explicit name

    let rep = rec.report();
    assert_eq!(rep.crosses["named_cross"][&vec!["default".to_string()]], 1);
}

#[test]
fn test_cross_no_last_value_skips_sample() {
    // CrossPoint.sample() when signal has never fired → should not panic
    let cp = Coverpoint::new("cp", "nosig", vec![bin_value("v", 0)]);
    let mut cross = CrossPoint::new("cross", vec![cp]);
    cross.sample(); // last_values is empty → drops silently
    assert!(cross.hits.is_empty());
}
