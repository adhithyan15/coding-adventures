mod common;

use common::{bl, blr, bv, conditional, immediate, installed, low_sign, set_state};
use pa_risc11_simulator::PaRisc11Error;

#[test]
fn compare_conditions_and_true_false_senses_are_exhaustive() {
    let cases = [
        (0, 1, 1, false),
        (1, 7, 7, true),
        (2, 0xffff_ffff, 0, true),
        (3, 9, 9, true),
        (4, 1, 2, true),
        (5, 2, 2, true),
        (6, 0x8000_0000, 1, true),
        (7, 8, 3, true),
    ];
    for (condition, left, right, expected) in cases {
        for (primary, inverted) in [(0x20, false), (0x22, true)] {
            let mut cpu = installed(&[conditional(primary, 2, 1, condition, 8, false)], 0x100);
            set_state(&mut cpu, |state| {
                state.gpr[1] = left;
                state.gpr[2] = right;
            });
            cpu.step().unwrap();
            let taken = cpu.state().next_pc == 0x110;
            assert_eq!(taken, expected ^ inverted, "condition {condition}");
        }
    }
}

#[test]
fn immediate_compare_and_add_branches_use_low_sign_and_write_sum() {
    let imm = low_sign(-1, 5);
    let mut compare = installed(&[conditional(0x21, 2, imm, 1, 8, false)], 0);
    set_state(&mut compare, |state| state.gpr[2] = u32::MAX);
    compare.step().unwrap();
    assert_eq!(compare.state().next_pc, 16);

    for (condition, left, right, expected) in [
        (0, 1, 1, false),
        (1, u32::MAX, 1, true),
        (2, 0xffff_fffd, 2, true),
        (3, u32::MAX, 1, true),
        (4, u32::MAX, 1, false),
        (5, u32::MAX, 1, true),
        (6, 0x7fff_ffff, 1, true),
        (7, 2, 1, true),
    ] {
        let mut cpu = installed(&[conditional(0x28, 2, 1, condition, 8, false)], 0);
        set_state(&mut cpu, |state| {
            state.gpr[1] = left;
            state.gpr[2] = right;
        });
        cpu.step().unwrap();
        assert_eq!(cpu.state().gpr[2], left.wrapping_add(right));
        assert_eq!(
            cpu.state().next_pc == 16,
            expected,
            "add condition {condition}"
        );
    }

    let mut addi = installed(&[conditional(0x2b, 2, low_sign(1, 5), 1, 8, false)], 0);
    set_state(&mut addi, |state| state.gpr[2] = u32::MAX);
    addi.step().unwrap();
    assert_eq!(addi.state().gpr[2], 0);
    assert_eq!(
        addi.state().next_pc,
        8,
        "false form inverts the true zero condition"
    );
}

#[test]
fn direction_and_outcome_control_conditional_nullification() {
    let mut forward_taken = installed(
        &[
            conditional(0x20, 1, 1, 1, 8, true),
            immediate(0x0d, 0, 2, 1),
            0,
            0,
        ],
        0,
    );
    set_state(&mut forward_taken, |state| state.gpr[1] = 4);
    forward_taken.step().unwrap();
    assert!(forward_taken.state().nullify_next);
    let trace = forward_taken.step().unwrap();
    assert!(trace.nullified);
    assert_eq!(trace.mnemonic, "NULLIFIED");
    assert_eq!(forward_taken.state().gpr[2], 0);
    assert_eq!(forward_taken.state().pc, 16);

    let mut backward_not_taken = installed(
        &[
            conditional(0x20, 1, 2, 1, -8, true),
            immediate(0x0d, 0, 3, 1),
            0,
        ],
        0x100,
    );
    set_state(&mut backward_not_taken, |state| {
        state.gpr[1] = 1;
        state.gpr[2] = 2;
    });
    backward_not_taken.step().unwrap();
    assert!(backward_not_taken.state().nullify_next);
    backward_not_taken.step().unwrap();
    assert_eq!(backward_not_taken.state().pc, 0x108);
    assert_eq!(backward_not_taken.state().gpr[3], 0);
}

#[test]
fn bl_delay_slot_call_and_bv_return_preserve_queue_and_link() {
    let program = [
        bl(2, 8, false),
        immediate(0x0d, 0, 3, 1),
        0,
        0,
        immediate(0x0d, 0, 4, 2),
        bv(2, 0, false),
        immediate(0x0d, 0, 5, 3),
    ];
    let mut cpu = installed(&program, 0x100);
    cpu.step().unwrap();
    assert_eq!(cpu.state().gpr[2], 0x108);
    assert_eq!((cpu.state().pc, cpu.state().next_pc), (0x104, 0x110));
    cpu.step().unwrap();
    assert_eq!(cpu.state().gpr[3], 1);
    assert_eq!(cpu.state().pc, 0x110);
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!((cpu.state().pc, cpu.state().next_pc), (0x118, 0x108));
    cpu.step().unwrap();
    assert_eq!(cpu.state().gpr[5], 3);
    assert_eq!(cpu.state().pc, 0x108);
    let halt = cpu.step().unwrap();
    assert_eq!(halt.mnemonic, "BREAK");
    assert!(cpu.state().halted);
}

#[test]
fn blr_links_and_nullifies_while_delayed_control_is_rejected() {
    let mut cpu = installed(&[blr(2, 1, true), immediate(0x0d, 0, 3, 1), 0, 0], 0x100);
    set_state(&mut cpu, |state| state.gpr[1] = 1);
    cpu.step().unwrap();
    assert_eq!(cpu.state().gpr[2], 0x108);
    assert_eq!(cpu.state().next_pc, 0x110);
    assert!(cpu.state().nullify_next);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 0x110);
    assert_eq!(cpu.state().gpr[3], 0);

    let mut nested = installed(&[bl(2, 8, false), bl(3, 8, false), 0, 0, 0], 0);
    nested.step().unwrap();
    let before = nested.snapshot().unwrap();
    assert_eq!(
        nested.step(),
        Err(PaRisc11Error::UnsupportedDelayedControl { pc: 4 })
    );
    assert_eq!(nested.state(), &before);

    let mut linear_queue = installed(
        &[conditional(0x20, 1, 2, 1, 0, false), bl(3, 0, false), 0],
        0,
    );
    set_state(&mut linear_queue, |state| {
        state.gpr[1] = 1;
        state.gpr[2] = 2;
    });
    linear_queue.step().unwrap();
    assert_eq!(
        (linear_queue.state().pc, linear_queue.state().next_pc),
        (4, 8)
    );
    assert!(linear_queue.state().in_delay_slot);
    assert_eq!(
        linear_queue.step(),
        Err(PaRisc11Error::UnsupportedDelayedControl { pc: 4 })
    );
}

#[test]
fn bv_normalizes_privilege_bits_without_losing_the_address_sum() {
    for privilege in 0..=3 {
        let mut cpu = installed(
            &[bv(1, 2, false), immediate(0x0d, 0, 3, 9), 0, 0, 0, 0, 0],
            0,
        );
        set_state(&mut cpu, |state| {
            state.gpr[1] = 0x10 | privilege;
            state.gpr[2] = 1;
        });

        cpu.step().unwrap();
        assert_eq!(cpu.state().next_pc, 0x18, "PL bits {privilege:02b}");
        cpu.step().unwrap();
        assert_eq!(cpu.state().pc, 0x18, "PL bits {privilege:02b}");
        assert_eq!(cpu.state().gpr[3], 9);
        let halt = cpu.step().unwrap();
        assert_eq!(halt.mnemonic, "BREAK");
    }
}
