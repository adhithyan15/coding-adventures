//! Integration tests for testbench-framework.
//!
//! All tests build a tiny HIR in memory (no files on disk) so the suite is
//! self-contained and fast.

use hdl_ir::{ContAssign, Direction, Expr, Hir, Module, Port, Ty};
use testbench_framework::{
    clear_registry, discover, exhaustive, random_stimulus, register_test, run, DutHandle,
    TestCase, TestReport,
};

// ---------------------------------------------------------------------------
// Helpers: tiny HIRs used across tests
// ---------------------------------------------------------------------------

/// Single-bit buffer: y = a.
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

/// 4-bit adder: {cout, sum[3:0]} = a + b + cin  (5-bit result).
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
// Registry
// ---------------------------------------------------------------------------

#[test]
fn test_register_and_discover() {
    clear_registry();
    register_test("t1", |_dut| {});
    register_test("t2", |_dut| {});
    let cases = discover();
    assert_eq!(cases.len(), 2);
    let names: std::collections::HashSet<_> = cases.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains("t1"));
    assert!(names.contains("t2"));
}

#[test]
fn test_clear_registry() {
    clear_registry();
    register_test("x", |_dut| {});
    assert_eq!(discover().len(), 1);
    clear_registry();
    assert_eq!(discover().len(), 0);
}

// ---------------------------------------------------------------------------
// run()
// ---------------------------------------------------------------------------

#[test]
fn test_run_passing() {
    let tc = TestCase::new("buf_high", |dut: &mut DutHandle| {
        dut.set("a", 1);
        assert_eq!(dut.get("y"), 1);
    });
    let rep = run(buffer_hir(), Some(vec![tc]));
    assert!(rep.all_passed());
    assert!(rep.passed.contains(&"buf_high".into()));
    assert!(rep.failed.is_empty());
}

#[test]
fn test_run_failing_assertion() {
    let tc = TestCase::new("wrong_assert", |dut: &mut DutHandle| {
        dut.set("a", 1);
        assert_eq!(dut.get("y"), 0); // wrong on purpose
    });
    let rep = run(buffer_hir(), Some(vec![tc]));
    assert!(!rep.all_passed());
    assert_eq!(rep.failed.len(), 1);
    assert_eq!(rep.failed[0].0, "wrong_assert");
}

#[test]
fn test_run_unexpected_panic() {
    let tc = TestCase::new("explodes", |_dut: &mut DutHandle| {
        panic!("boom");
    });
    let rep = run(buffer_hir(), Some(vec![tc]));
    assert!(!rep.all_passed());
    assert!(rep.failed[0].1.contains("boom"));
}

#[test]
fn test_run_negative_test_passes_when_it_panics() {
    let tc = TestCase::new("must_fail", |_dut: &mut DutHandle| {
        // Intentional panic: this negative test verifies expect_fail() passes
        // when the body panics. panic! preserves the original assert! behavior.
        panic!("intentional");
    }).expect_fail();
    let rep = run(buffer_hir(), Some(vec![tc]));
    assert!(rep.all_passed());
}

#[test]
fn test_run_negative_test_fails_when_no_panic() {
    let tc = TestCase::new("should_have_failed", |_dut: &mut DutHandle| {
        // no panic — expected to fail but it passes
    }).expect_fail();
    let rep = run(buffer_hir(), Some(vec![tc]));
    assert!(!rep.all_passed());
    assert!(rep.failed[0].1.contains("expected failure but test passed"));
}

#[test]
fn test_run_isolates_state_between_tests() {
    // first sets a=1; second should see a=0 because it gets a fresh VM
    let tc1 = TestCase::new("sets_a", |dut: &mut DutHandle| {
        dut.set("a", 1);
        assert_eq!(dut.get("y"), 1);
    });
    let tc2 = TestCase::new("sees_a_zero", |dut: &mut DutHandle| {
        assert_eq!(dut.get("a"), 0); // fresh VM → default 0
        assert_eq!(dut.get("y"), 0);
    });
    let rep = run(buffer_hir(), Some(vec![tc1, tc2]));
    assert!(rep.all_passed(), "failed: {:?}", rep.failed);
}

#[test]
fn test_run_with_discover() {
    clear_registry();
    register_test("disc_buf", |dut: &mut DutHandle| {
        dut.set("a", 1);
        assert_eq!(dut.get("y"), 1);
    });
    // None → uses discover() internally
    let rep = run(buffer_hir(), None);
    assert!(rep.all_passed(), "failed: {:?}", rep.failed);
    clear_registry();
}

// ---------------------------------------------------------------------------
// TestReport
// ---------------------------------------------------------------------------

#[test]
fn test_report_all_passed() {
    let rep = TestReport {
        passed: vec!["a".into(), "b".into()],
        failed: vec![],
        skipped: vec![],
        duration_s: 0.001,
    };
    assert!(rep.all_passed());
}

#[test]
fn test_report_summary() {
    let rep = TestReport {
        passed: vec!["a".into()],
        failed: vec![("b".into(), "boom".into())],
        skipped: vec![],
        duration_s: 0.042,
    };
    let s = rep.summary();
    assert!(s.contains("1 passed"));
    assert!(s.contains("1 failed"));
    assert!(s.contains("0 skipped"));
}

// ---------------------------------------------------------------------------
// DutHandle
// ---------------------------------------------------------------------------

#[test]
fn test_dut_handle_buffer_round_trip() {
    let tc = TestCase::new("buf_rt", |dut: &mut DutHandle| {
        for v in [0i64, 1] {
            dut.set("a", v);
            assert_eq!(dut.get("y"), v);
        }
    });
    let rep = run(buffer_hir(), Some(vec![tc]));
    assert!(rep.all_passed(), "{:?}", rep.failed);
}

// ---------------------------------------------------------------------------
// Exhaustive stimulus
// ---------------------------------------------------------------------------

#[test]
fn test_exhaustive_adder4_all_combos() {
    let tc = TestCase::new("adder_full", |dut: &mut DutHandle| {
        let mut count = 0u32;
        exhaustive(
            dut,
            &[("a", 4), ("b", 4), ("cin", 1)],
            Some(&mut |d: &mut DutHandle| {
                let a   = d.get("a")   as u32;
                let b   = d.get("b")   as u32;
                let cin = d.get("cin") as u32;
                let expected = (a + b + cin) & 0x1F;
                let actual   = ((d.get("cout") as u32) << 4) | (d.get("sum") as u32);
                assert_eq!(actual, expected, "a={a} b={b} cin={cin}");
                count += 1;
            }),
        ).unwrap();
        // 16 values for a, 16 for b, 2 for cin
        assert_eq!(count, 16 * 16 * 2);
    });
    let rep = run(adder4_hir(), Some(vec![tc]));
    assert!(rep.all_passed(), "{:?}", rep.failed);
}

#[test]
fn test_exhaustive_too_many_bits_returns_err() {
    use hardware_vm::HardwareVm;
    let vm   = HardwareVm::new(buffer_hir()).unwrap();
    let mut dut  = DutHandle::new(vm);
    let result   = exhaustive(&mut dut, &[("a", 25)], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("exhaustive over"));
}

// ---------------------------------------------------------------------------
// Random stimulus
// ---------------------------------------------------------------------------

#[test]
fn test_random_stimulus_same_seed_same_sequence() {
    // Run random stimulus twice with the same seed; collect 'a' values.
    // Both runs must produce identical sequences.
    let tc = TestCase::new("rng_repro", |dut: &mut DutHandle| {
        let mut seq_a: Vec<i64> = Vec::new();
        let mut seq_b: Vec<i64> = Vec::new();

        random_stimulus(
            dut,
            &[("a", 4), ("b", 4), ("cin", 1)],
            20,
            42,
            Some(&mut |d: &mut DutHandle| seq_a.push(d.get("a"))),
        );
        random_stimulus(
            dut,
            &[("a", 4), ("b", 4), ("cin", 1)],
            20,
            42,
            Some(&mut |d: &mut DutHandle| seq_b.push(d.get("a"))),
        );

        assert_eq!(seq_a, seq_b, "same seed must produce same sequence");
    });
    let rep = run(adder4_hir(), Some(vec![tc]));
    assert!(rep.all_passed(), "{:?}", rep.failed);
}

#[test]
fn test_random_stimulus_different_seeds_differ() {
    let tc = TestCase::new("rng_seeds", |dut: &mut DutHandle| {
        let mut seq_42: Vec<i64> = Vec::new();
        let mut seq_99: Vec<i64> = Vec::new();

        random_stimulus(dut, &[("a", 4)], 20, 42,
            Some(&mut |d: &mut DutHandle| seq_42.push(d.get("a"))));
        random_stimulus(dut, &[("a", 4)], 20, 99,
            Some(&mut |d: &mut DutHandle| seq_99.push(d.get("a"))));

        // Two distinct seeds must not produce the identical sequence.
        assert_ne!(seq_42, seq_99, "different seeds should differ");
    });
    let rep = run(adder4_hir(), Some(vec![tc]));
    assert!(rep.all_passed(), "{:?}", rep.failed);
}

// ---------------------------------------------------------------------------
// TestCase builder methods
// ---------------------------------------------------------------------------

#[test]
fn test_testcase_with_timeout() {
    let tc = TestCase::new("t", |_dut| {}).with_timeout(10.0);
    assert_eq!(tc.timeout_s, 10.0);
}

#[test]
fn test_testcase_expect_fail_field() {
    let tc = TestCase::new("t", |_dut| {}).expect_fail();
    assert!(tc.should_fail);
}
