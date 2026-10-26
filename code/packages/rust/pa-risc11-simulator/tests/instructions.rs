mod common;

use common::{alu, assemble_21_inverse, immediate, installed, memory, set_state, shd};
use pa_risc11_simulator::PaRisc11Error;

#[test]
fn register_alu_families_and_gr0_are_exact() {
    for (extension, expected, mnemonic) in [
        (0x00, 0x0f00_0f00, "ANDCM"),
        (0x08, 0x0000_000f, "AND"),
        (0x09, 0x0fff_0fff, "OR"),
        (0x0a, 0x0fff_0ff0, "XOR"),
        (0x10, 0x0e01_0e10, "SUB"),
        (0x18, 0x0fff_100e, "ADD"),
    ] {
        let mut cpu = installed(&[alu(extension, 2, 1, 3)], 0x100);
        set_state(&mut cpu, |state| {
            state.gpr[1] = 0x0f00_0f0f;
            state.gpr[2] = 0x00ff_00ff;
        });
        let trace = cpu.step().unwrap();
        assert_eq!(trace.mnemonic, mnemonic);
        assert_eq!(cpu.state().gpr[3], expected);
    }

    let mut cpu = installed(&[alu(0x18, 1, 2, 0)], 0);
    set_state(&mut cpu, |state| {
        state.gpr[1] = 7;
        state.gpr[2] = 9;
    });
    cpu.step().unwrap();
    assert_eq!(cpu.state().gpr[0], 0);
}

#[test]
fn arithmetic_updates_each_psw_nibble_carry_and_logic_preserves_it() {
    let mut add = installed(&[alu(0x18, 2, 1, 3), alu(0x08, 2, 1, 3)], 0);
    set_state(&mut add, |state| {
        state.gpr[1] = 0x0000_000f;
        state.gpr[2] = 1;
        state.carry_borrow = [true; 8];
    });
    add.step().unwrap();
    assert_eq!(add.state().gpr[3], 0x10);
    assert_eq!(
        add.state().carry_borrow,
        [false, false, false, false, false, false, false, true]
    );
    let arithmetic_carries = add.state().carry_borrow;
    add.step().unwrap();
    assert_eq!(add.state().carry_borrow, arithmetic_carries);

    let mut wrapping_add = installed(&[alu(0x18, 2, 1, 3)], 0);
    set_state(&mut wrapping_add, |state| {
        state.gpr[1] = u32::MAX;
        state.gpr[2] = 1;
    });
    wrapping_add.step().unwrap();
    assert_eq!(wrapping_add.state().carry_borrow, [true; 8]);

    let mut subtract = installed(&[alu(0x10, 2, 1, 3)], 0);
    set_state(&mut subtract, |state| {
        state.gpr[1] = 0x10;
        state.gpr[2] = 1;
    });
    subtract.step().unwrap();
    assert_eq!(subtract.state().gpr[3], 0xf);
    assert_eq!(
        subtract.state().carry_borrow,
        [true, true, true, true, true, true, true, false]
    );

    let mut borrowing = installed(&[alu(0x10, 2, 1, 3)], 0);
    set_state(&mut borrowing, |state| {
        state.gpr[1] = 0;
        state.gpr[2] = 1;
    });
    borrowing.step().unwrap();
    assert_eq!(borrowing.state().carry_borrow, [false; 8]);
}

#[test]
fn address_immediates_cover_low_sign_and_permuted_boundaries() {
    for value in [-8192, -1, 0, 1, 8191] {
        let mut cpu = installed(&[immediate(0x0d, 1, 2, value)], 0);
        set_state(&mut cpu, |state| state.gpr[1] = 0x1000);
        cpu.step().unwrap();
        assert_eq!(cpu.state().gpr[2], 0x1000_u32.wrapping_add(value as u32));
    }

    for assembled in [0, 1, 3, 0x15555, 0x1f_ffff] {
        let encoded = assemble_21_inverse(assembled);
        let mut ldil = installed(&[(0x08 << 26) | (2 << 21) | encoded], 0);
        ldil.step().unwrap();
        assert_eq!(ldil.state().gpr[2], assembled << 11);

        let mut addil = installed(&[(0x0a << 26) | (3 << 21) | encoded], 0);
        set_state(&mut addil, |state| state.gpr[3] = 17);
        addil.step().unwrap();
        assert_eq!(addil.state().gpr[1], 17_u32.wrapping_add(assembled << 11));
    }
}

#[test]
fn big_endian_memory_widths_alignment_and_atomicity() {
    let program = [
        memory(0x1a, 1, 2, 0),
        memory(0x12, 1, 3, 0),
        memory(0x19, 1, 2, 4),
        memory(0x11, 1, 4, 4),
        memory(0x18, 1, 2, 6),
        memory(0x10, 1, 5, 6),
    ];
    let mut cpu = installed(&program, 0);
    set_state(&mut cpu, |state| {
        state.gpr[1] = 0x200;
        state.gpr[2] = 0x89ab_cdef;
    });
    for _ in 0..program.len() {
        cpu.step().unwrap();
    }
    assert_eq!(
        &cpu.state().memory[0x200..0x207],
        &[0x89, 0xab, 0xcd, 0xef, 0xcd, 0xef, 0xef]
    );
    assert_eq!(cpu.state().gpr[3], 0x89ab_cdef);
    assert_eq!(cpu.state().gpr[4], 0xcdef);
    assert_eq!(cpu.state().gpr[5], 0xef);

    let mut misaligned = installed(&[memory(0x1a, 1, 2, 0)], 0);
    set_state(&mut misaligned, |state| {
        state.gpr[1] = 3;
        state.gpr[2] = u32::MAX;
    });
    let before = misaligned.snapshot().unwrap();
    assert_eq!(
        misaligned.step(),
        Err(PaRisc11Error::MisalignedData {
            address: 3,
            width: 4
        })
    );
    assert_eq!(misaligned.state(), &before);

    let mut outside = installed(&[memory(0x11, 1, 2, 0)], 0);
    set_state(&mut outside, |state| state.gpr[1] = 65_535);
    assert!(matches!(
        outside.step(),
        Err(PaRisc11Error::MisalignedData { .. }) | Err(PaRisc11Error::DataOutOfRange { .. })
    ));
}

#[test]
fn shd_covers_zero_middle_and_31() {
    let left = 0x1234_5678_u32;
    let right = 0x9abc_def0_u32;
    for count in [0, 1, 16, 31] {
        let mut cpu = installed(&[shd(2, 1, count, 3)], 0);
        set_state(&mut cpu, |state| {
            state.gpr[1] = left;
            state.gpr[2] = right;
        });
        cpu.step().unwrap();
        let expected = if count == 0 {
            right
        } else {
            (left << (32 - count)) | (right >> count)
        };
        assert_eq!(cpu.state().gpr[3], expected);
    }
}

#[test]
fn reserved_neighbors_are_typed_and_atomic() {
    for word in [
        alu(0x3f, 1, 2, 3),
        immediate(0x0d, 1, 2, 0) | (1 << 14),
        shd(1, 2, 3, 4) | (1 << 13),
        (0x3a << 26) | (1 << 13),
        (0x3a << 26) | (2 << 13) | (1 << 2),
        1,
    ] {
        let mut cpu = installed(&[word], 0);
        let before = cpu.snapshot().unwrap();
        assert!(matches!(
            cpu.step(),
            Err(PaRisc11Error::UnsupportedInstruction { .. })
        ));
        assert_eq!(cpu.state(), &before);
    }
    let mut unknown = installed(&[0xfc00_0000], 0);
    assert!(matches!(
        unknown.step(),
        Err(PaRisc11Error::UnknownInstruction { .. })
    ));

    let mut outside = installed(&[memory(0x10, 1, 2, 0)], 0);
    set_state(&mut outside, |state| state.gpr[1] = 65_536);
    assert_eq!(
        outside.step(),
        Err(PaRisc11Error::DataOutOfRange {
            address: 65_536,
            width: 1
        })
    );
}

#[test]
fn every_primary_opcode_has_an_explicit_classification() {
    let accepted = [
        0x00_u32, 0x02, 0x08, 0x0a, 0x0d, 0x10, 0x11, 0x12, 0x18, 0x19, 0x1a, 0x20, 0x21, 0x22,
        0x23, 0x28, 0x29, 0x2a, 0x2b, 0x34, 0x3a,
    ];
    for primary in 0..64_u32 {
        let word = match primary {
            0x34 => shd(0, 0, 0, 0),
            _ => primary << 26,
        };
        let mut cpu = installed(&[word], 0);
        let result = cpu.step();
        if accepted.contains(&primary) {
            assert!(result.is_ok(), "primary {primary:#x}: {result:?}");
        } else {
            assert!(matches!(
                result,
                Err(PaRisc11Error::UnknownInstruction { .. })
            ));
        }
    }
}
