//! KRN-006c (VER25 section 8.10): the reference interpreter must check its
//! step budget before every step, terminators included.  A cycle of blocks
//! that hold no instructions has no instruction step to check in, so before
//! the fix it spun forever.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use coding_adventures_warrant_ir::*;

fn block(id: usize, terminator: WirTerminator) -> BasicBlock {
    BasicBlock { id: BlockId(id), instructions: vec![], terminator }
}

fn program(blocks: Vec<BasicBlock>) -> Program {
    Program::new(vec![Function {
        name: "spin".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks,
        ranking_metric: None,
    }])
}

/// Run `spin` with the given step budget on a worker thread and wait at most
/// a few seconds.  If the interpreter never returns, the test fails with a
/// clear message instead of hanging the suite (the stuck worker thread is
/// abandoned and dies with the test process).
fn run_with_timeout(program: Program, max_steps: u64) -> ExecutionOutcome {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let config = InterpreterConfig { max_steps, ..InterpreterConfig::default() };
        let mut interp = Interpreter::with_config(&program, config, DefaultCapabilityDispatcher);
        let _ = tx.send(interp.run("spin", &[]));
    });
    rx.recv_timeout(Duration::from_secs(5))
        .expect("interpreter did not stop within 5 seconds: the step budget is not checked on every step")
}

#[test]
fn two_block_jump_cycle_ends_with_budget_exceeded() {
    let p = program(vec![
        block(0, WirTerminator::Jump(BlockId(1))),
        block(1, WirTerminator::Jump(BlockId(0))),
    ]);
    match run_with_timeout(p, 10) {
        ExecutionOutcome::BudgetExceeded(BudgetLimit::StepLimitExceeded(steps)) => {
            assert!(steps >= 10, "stopped at {steps} steps, below the budget of 10");
        }
        other => panic!("expected BudgetExceeded(StepLimitExceeded), got {other:?}"),
    }
}

#[test]
fn one_block_self_jump_ends_with_budget_exceeded() {
    let p = program(vec![block(0, WirTerminator::Jump(BlockId(0)))]);
    assert!(matches!(
        run_with_timeout(p, 3),
        ExecutionOutcome::BudgetExceeded(BudgetLimit::StepLimitExceeded(_))
    ));
}

#[test]
fn branch_cycle_ends_with_budget_exceeded() {
    // Both arms of the branch go back to the same block: no instructions, a
    // branch terminator on every step.
    let p = program(vec![block(
        0,
        WirTerminator::Branch {
            cond: WirExpr::ConstBool(true),
            then_block: BlockId(0),
            else_block: BlockId(0),
        },
    )]);
    assert!(matches!(
        run_with_timeout(p, 3),
        ExecutionOutcome::BudgetExceeded(BudgetLimit::StepLimitExceeded(_))
    ));
}

#[test]
fn a_terminating_program_within_budget_still_succeeds() {
    let p = program(vec![
        block(0, WirTerminator::Jump(BlockId(1))),
        block(1, WirTerminator::Return(None)),
    ]);
    assert!(run_with_timeout(p, 10).is_success());
}
