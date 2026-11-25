//! KRN-006b (VER25 section 8.5): the type checker must accept a read only when
//! the variable is bound on *every* path from the entry block, whatever order
//! the blocks sit in the function's vector.  Also: a function needs an entry
//! block (id 0) and its block ids must be exactly `0..n`.

use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::*;

fn u64_ty() -> WirType {
    WirType::U64
}

fn x() -> VarId {
    VarId::from("x")
}

fn let_x(val: i128) -> WirInst {
    WirInst::Let {
        var: x(),
        ty: u64_ty(),
        label: Label::PUBLIC,
        expr: WirExpr::ConstInt { val, ty: u64_ty() },
    }
}

fn read_x() -> WirTerminator {
    WirTerminator::Return(Some(WirExpr::Var(x())))
}

fn ret0() -> WirTerminator {
    WirTerminator::Return(Some(WirExpr::ConstInt { val: 0, ty: u64_ty() }))
}

fn block(id: usize, instructions: Vec<WirInst>, terminator: WirTerminator) -> BasicBlock {
    BasicBlock { id: BlockId(id), instructions, terminator }
}

fn branch(then_block: usize, else_block: usize) -> WirTerminator {
    WirTerminator::Branch {
        cond: WirExpr::Var(VarId::from("c")),
        then_block: BlockId(then_block),
        else_block: BlockId(else_block),
    }
}

/// A one-function program `f(c: bool) -> u64` with the given blocks.
fn prog(blocks: Vec<BasicBlock>) -> Program {
    Program::new(vec![Function {
        name: "f".to_string(),
        params: vec![(VarId::from("c"), WirType::Bool, Label::PUBLIC)],
        return_type: u64_ty(),
        blocks,
        ranking_metric: None,
    }])
}

fn check(p: &Program) -> Result<(), TypeError> {
    TypeChecker::new(p).check_program()
}

fn unbound(block: usize) -> TypeError {
    TypeError::UseBeforeDefinition {
        function: "f".to_string(),
        block: BlockId(block),
        var: x(),
    }
}

#[test]
fn read_reached_by_a_path_that_never_binds_is_rejected() {
    // The KRN-006b counterexample.  Vector order is 0, 1, 2, so a walk in
    // vector order sees block 1 bind `x` before block 2 reads it.  But the
    // entry jumps straight to 2.
    let p = prog(vec![
        block(0, vec![], WirTerminator::Jump(BlockId(2))),
        block(1, vec![let_x(1)], WirTerminator::Jump(BlockId(2))),
        block(2, vec![], read_x()),
    ]);
    assert_eq!(check(&p), Err(unbound(2)));
}

#[test]
fn diamond_where_both_arms_bind_is_accepted() {
    let p = prog(vec![
        block(0, vec![], branch(1, 2)),
        block(1, vec![let_x(1)], WirTerminator::Jump(BlockId(3))),
        block(2, vec![let_x(2)], WirTerminator::Jump(BlockId(3))),
        block(3, vec![], read_x()),
    ]);
    assert_eq!(check(&p), Ok(()));
}

#[test]
fn diamond_where_one_arm_binds_is_rejected() {
    let p = prog(vec![
        block(0, vec![], branch(1, 2)),
        block(1, vec![let_x(1)], WirTerminator::Jump(BlockId(3))),
        block(2, vec![], WirTerminator::Jump(BlockId(3))),
        block(3, vec![], read_x()),
    ]);
    assert_eq!(check(&p), Err(unbound(3)));
}

#[test]
fn blocks_listed_out_of_flow_order_are_accepted_when_flow_is_sound() {
    // Vector order is 0, 2, 1 but flow is 0 -> 1 -> 2.  The old walk saw the
    // read first and reported an undefined variable.  (Ids are still 0..n.)
    let p = prog(vec![
        block(0, vec![], WirTerminator::Jump(BlockId(1))),
        block(2, vec![], read_x()),
        block(1, vec![let_x(1)], WirTerminator::Jump(BlockId(2))),
    ]);
    assert_eq!(check(&p), Ok(()));
}

#[test]
fn loop_with_binding_before_the_loop_is_accepted() {
    // 0: let x; jump 1.   1 (header): branch c -> 2 | 3.   2 (body): jump 1.
    // 3: return x.
    let p = prog(vec![
        block(0, vec![let_x(1)], WirTerminator::Jump(BlockId(1))),
        block(1, vec![], branch(2, 3)),
        block(2, vec![], WirTerminator::Jump(BlockId(1))),
        block(3, vec![], read_x()),
    ]);
    assert_eq!(check(&p), Ok(()));
}

#[test]
fn loop_binding_only_in_the_body_but_read_in_the_header_is_rejected() {
    // The header reads `x`, but `x` is bound only in the body, which runs
    // after the header's first visit.
    let p = prog(vec![
        block(0, vec![], WirTerminator::Jump(BlockId(1))),
        block(
            1,
            vec![WirInst::Let {
                var: VarId::from("y"),
                ty: u64_ty(),
                label: Label::PUBLIC,
                expr: WirExpr::Var(x()),
            }],
            branch(2, 3),
        ),
        block(2, vec![let_x(1)], WirTerminator::Jump(BlockId(1))),
        block(3, vec![], ret0()),
    ]);
    assert_eq!(check(&p), Err(unbound(1)));
}

#[test]
fn function_without_block_zero_is_rejected() {
    let p = prog(vec![block(1, vec![], ret0())]);
    assert_eq!(check(&p), Err(TypeError::NoEntryBlock("f".to_string())));
}

#[test]
fn block_ids_with_a_gap_are_rejected() {
    let p = prog(vec![
        block(0, vec![], WirTerminator::Jump(BlockId(2))),
        block(2, vec![], ret0()),
    ]);
    assert_eq!(
        check(&p),
        Err(TypeError::BlockIdsNotDense { function: "f".to_string(), block: BlockId(2), count: 2 })
    );
}

#[test]
fn duplicate_block_ids_are_still_reported_as_duplicates() {
    let p = prog(vec![block(0, vec![], ret0()), block(0, vec![], ret0())]);
    assert_eq!(check(&p), Err(TypeError::DuplicateBlock(BlockId(0))));
}

/// Every in-repo adopter: the lowering of the real ct-compare source and the
/// small sources the other tests lower.
///
/// This asks only the KRN-006b question (`check_definite_assignment`: entry
/// block, dense ids, every read bound on every path).  The full type rules
/// are a separate matter: the lowering emits tail expressions as
/// `Return(None)`-style unit returns that `check_function` rejects with a
/// `TypeMismatch` (a finding reported separately), so `check_program` cannot
/// be the oracle here.
#[test]
fn lowered_adopters_satisfy_definite_assignment() {
    let ct = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(ct).expect("parse ct-compare");
    let program = lower_source_file(
        &file,
        &[
            ("ct_eq", &["a", "b"]),
            ("ct_eq_equal_length", &["a", "b"]),
            ("ct_select_bytes", &["a", "b", "choice"]),
            ("ct_eq_u64", &["a", "b"]),
        ],
    )
    .expect("lower ct-compare");
    assert_adopter_is_sound(&program, "ct-compare");

    let small = [
        "pub fn f(n: usize) -> usize {\n    let v = vec![7u8; n];\n    n\n}\n",
        "pub fn h(a: u64) -> u64 {\n    assert!(a == 3);\n    a\n}\n",
        "pub fn w(n: u64) -> u64 {\n    let mut s = 0u64;\n    for idx in 0..n {\n        let t = idx + 1;\n        s = s + t;\n    }\n    if s == 3 {\n        let r = 1u64;\n        return r;\n    }\n    s\n}\n",
    ];
    for src in small {
        let program = lower_source_file(&parse_rust_file(src).expect("parse"), &[]).expect("lower");
        assert_adopter_is_sound(&program, src);
    }
}

fn assert_adopter_is_sound(program: &Program, what: &str) {
    let checker = TypeChecker::new(program);
    for func in &program.functions {
        assert_eq!(
            checker.check_definite_assignment(func),
            Ok(()),
            "{what}: function {}",
            func.name
        );
    }
}
