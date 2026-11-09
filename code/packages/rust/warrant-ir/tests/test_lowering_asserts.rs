//! VER04h: lowering fidelity fixes found while proving `ct_select_bytes`.
//!
//! * `assert_eq!` / `assert!` are active in release builds, so they lower to a
//!   branch into a trap instead of a no-op (the old no-op made the length
//!   mismatch path of `ct_select_bytes` an out-of-bounds scan).
//! * `vec![0; n]` no longer stores a first element (allocation is zero-filled,
//!   and that store wrote past a zero-length buffer); other fills still do.

use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::*;

const SOURCE: &str = include_str!("../../ct-compare/src/lib.rs");

fn select_program() -> Program {
    let file = parse_rust_file(SOURCE).expect("parse");
    lower_source_file(&file, &[("ct_select_bytes", &["a", "b", "choice"])]).expect("lower")
}

#[test]
fn assert_eq_traps_on_a_length_mismatch_and_passes_otherwise() {
    let program = select_program();
    let mut interp = Interpreter::new(&program);
    let a = interp.allocate_bytes(&[1, 2, 3, 4]).unwrap();
    let b = interp.allocate_bytes(&[5, 6, 7, 8, 9]).unwrap();
    match interp.run("ct_select_bytes", &[a, b, WirValue::Bool(true)]) {
        ExecutionOutcome::Trap { reason, .. } => assert!(reason.contains("assertion failed")),
        other => panic!("expected a trap, got {other:?}"),
    }
    let a = interp.allocate_bytes(&[1, 2, 3, 4]).unwrap();
    let b = interp.allocate_bytes(&[5, 6, 7, 8]).unwrap();
    assert!(interp.run("ct_select_bytes", &[a, b, WirValue::Bool(true)]).is_success());
}

#[test]
fn empty_select_no_longer_stores_past_a_zero_length_buffer() {
    let program = select_program();
    let f = program.find_function("ct_select_bytes").unwrap();
    // The instruction right after the zero-filled allocation must not be the
    // old "initialise first element" store.
    let mut checked = false;
    for block in &f.blocks {
        for pair in block.instructions.windows(2) {
            if let WirInst::Alloc { var, .. } = &pair[0] {
                if var.0.starts_with("vec_buf") {
                    assert!(!matches!(pair[1], WirInst::Store { .. }), "zero fill must not store");
                    checked = true;
                }
            }
        }
    }
    assert!(checked, "the allocation must be present");
    let mut interp = Interpreter::new(&program);
    let a = interp.allocate_bytes(&[]).unwrap();
    let b = interp.allocate_bytes(&[]).unwrap();
    assert!(interp.run("ct_select_bytes", &[a, b, WirValue::Bool(false)]).is_success());
}

#[test]
fn non_zero_fill_still_initialises_the_first_element() {
    let src = "pub fn f(n: usize) -> usize {\n    let v = vec![7u8; n];\n    n\n}\npub fn g(n: usize) -> usize {\n    let v = vec![0u8; n];\n    n\n}\n";
    let file = parse_rust_file(src).expect("parse");
    let program = lower_source_file(&file, &[]).expect("lower");
    let count = |name: &str| {
        program
            .find_function(name)
            .unwrap()
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .filter(|i| matches!(i, WirInst::Store { .. }))
            .count()
    };
    assert_eq!(count("f"), 1);
    assert_eq!(count("g"), 0);
}

#[test]
fn plain_assert_lowers_to_a_trap_branch() {
    let src = "pub fn h(a: u64) -> u64 {\n    assert!(a == 3);\n    a\n}\n";
    let file = parse_rust_file(src).expect("parse");
    let program = lower_source_file(&file, &[]).expect("lower");
    let mut interp = Interpreter::new(&program);
    assert!(interp.run("h", &[WirValue::u64(3)]).is_success());
    assert!(matches!(
        interp.run("h", &[WirValue::u64(4)]),
        ExecutionOutcome::Trap { .. }
    ));
}
