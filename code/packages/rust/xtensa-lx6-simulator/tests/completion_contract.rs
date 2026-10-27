use xtensa_lx6_simulator::{
    classify_instruction, DataAccess, UndefinedReason, XtensaLx6Error, XtensaLx6Simulator,
    XtensaLx6State, MEMORY_SIZE,
};

fn rrr(op2: u8, op1: u8, r: u8, s: u8, t: u8) -> u32 {
    u32::from(t) << 4
        | u32::from(s) << 8
        | u32::from(r) << 12
        | u32::from(op1) << 16
        | u32::from(op2) << 20
}

fn rri8(r: u8, s: u8, t: u8, immediate: u8, op0: u8) -> u32 {
    u32::from(op0)
        | u32::from(t) << 4
        | u32::from(s) << 8
        | u32::from(r) << 12
        | u32::from(immediate) << 16
}

fn bytes24(raw: u32) -> [u8; 3] {
    [raw as u8, (raw >> 8) as u8, (raw >> 16) as u8]
}
fn bytes16(raw: u16) -> [u8; 2] {
    raw.to_le_bytes()
}

fn loaded(raw: u32) -> XtensaLx6Simulator {
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes24(raw), 0x100).unwrap();
    simulator
}

fn with_state(
    mut simulator: XtensaLx6Simulator,
    change: impl FnOnce(&mut XtensaLx6State),
) -> XtensaLx6Simulator {
    let mut state = simulator.snapshot().unwrap();
    change(&mut state);
    simulator.restore(state).unwrap();
    simulator
}

#[test]
fn deterministic_lifecycle_and_complete_restore_validation() {
    let mut simulator = XtensaLx6Simulator::new();
    let reset = simulator.snapshot().unwrap();
    assert_eq!(reset.memory.len(), MEMORY_SIZE);
    assert_eq!(reset.window_start, 1);
    assert_eq!(reset.origin, None);
    assert_eq!(simulator.load(&[], 0), Err(XtensaLx6Error::EmptyProgram));
    assert_eq!(
        simulator.load(&[0; 2], 65_535),
        Err(XtensaLx6Error::ProgramOutOfRange {
            origin: 65_535,
            length: 2
        })
    );
    assert!(matches!(
        simulator.load(&[0; 2], u32::MAX),
        Err(XtensaLx6Error::ProgramAddressOverflow { .. })
    ));

    simulator.load(&bytes16(0xf03d), 3).unwrap();
    let installed = simulator.snapshot().unwrap();
    assert_eq!(installed.pc, 3);
    assert_eq!(installed.origin, Some(3));
    simulator.reset();
    simulator.restore(installed.clone()).unwrap();
    assert_eq!(simulator.snapshot().unwrap(), installed);

    for invalid in [
        {
            let mut x = installed.clone();
            x.memory.pop();
            x
        },
        {
            let mut x = installed.clone();
            x.sar = 64;
            x
        },
        {
            let mut x = installed.clone();
            x.window_base = 16;
            x
        },
        {
            let mut x = installed.clone();
            x.call_inc = 4;
            x
        },
        {
            let mut x = installed.clone();
            x.window_start = 0;
            x
        },
    ] {
        let before = simulator.snapshot().unwrap();
        assert!(matches!(
            simulator.restore(invalid),
            Err(XtensaLx6Error::InvalidState { .. })
        ));
        assert_eq!(simulator.snapshot().unwrap(), before);
    }
}

#[test]
fn every_physical_window_mapping_is_owned_and_restorable() {
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes16(0xf03d), 0x100).unwrap();
    for window_base in 0..16u8 {
        let mut state = simulator.snapshot().unwrap();
        state.window_base = window_base;
        state.window_start = 1 << window_base;
        simulator.restore(state).unwrap();
        for logical in 0..16u8 {
            let value = u32::from(window_base) << 8 | u32::from(logical);
            simulator.set_logical_register(logical, value).unwrap();
            let physical = (usize::from(window_base) * 4 + usize::from(logical)) & 63;
            assert_eq!(simulator.snapshot().unwrap().ar[physical], value);
            assert_eq!(simulator.logical_register(logical).unwrap(), value);
        }
    }
    let snapshot = simulator.snapshot().unwrap();
    simulator.reset();
    simulator.restore(snapshot.clone()).unwrap();
    assert_eq!(simulator.snapshot().unwrap(), snapshot);
}

#[test]
fn mixed_width_fetch_uses_exact_bytes_at_every_residue() {
    for origin in 0..4 {
        let mut simulator = XtensaLx6Simulator::new();
        simulator.load(&bytes16(0xf03d), origin).unwrap();
        let trace = simulator.step().unwrap();
        assert_eq!(trace.width, 2);
        assert_eq!(trace.mnemonic, "NOP.N");
        assert_eq!(trace.raw_bytes, [0x3d, 0xf0, 0]);

        let mut simulator = XtensaLx6Simulator::new();
        simulator.load(&bytes24(0x0020f0), origin).unwrap();
        let trace = simulator.step().unwrap();
        assert_eq!(trace.width, 3);
        assert_eq!(trace.mnemonic, "NOP");
        assert_eq!(trace.raw_bytes, [0xf0, 0x20, 0]);
    }
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&[0x00, 0x20], 0).unwrap();
    assert_eq!(
        simulator.step(),
        Err(XtensaLx6Error::TruncatedInstruction {
            pc: 0,
            needed: 3,
            available: 2
        })
    );
}

#[test]
fn fetch_memory_and_restore_boundaries_are_atomic() {
    let mut simulator = XtensaLx6Simulator::new();
    assert_eq!(simulator.step(), Err(XtensaLx6Error::NoProgramLoaded));

    simulator.load(&[0x08], 0x100).unwrap();
    assert_eq!(
        simulator.step(),
        Err(XtensaLx6Error::TruncatedInstruction {
            pc: 0x100,
            needed: 2,
            available: 1,
        })
    );
    simulator.load(&[0x00, 0x20], 0x100).unwrap();
    assert_eq!(
        simulator.step(),
        Err(XtensaLx6Error::TruncatedInstruction {
            pc: 0x100,
            needed: 3,
            available: 2,
        })
    );

    for width in [1, 2, 4] {
        let last = MEMORY_SIZE as u32 - u32::from(width);
        simulator.write_memory(last, width, 0x8877_6655).unwrap();
        let mask = match width {
            1 => 0xff,
            2 => 0xffff,
            4 => u32::MAX,
            _ => unreachable!(),
        };
        assert_eq!(
            simulator.read_memory(last, width).unwrap(),
            0x8877_6655 & mask
        );
        for overrun in 1..u32::from(width) {
            let address = last + overrun;
            let before = simulator.snapshot().unwrap();
            assert_eq!(
                simulator.write_memory(address, width, 0x1122_3344),
                Err(XtensaLx6Error::DataOutOfRange { address, width })
            );
            assert_eq!(simulator.snapshot().unwrap(), before);
            assert_eq!(
                simulator.read_memory(address, width),
                Err(XtensaLx6Error::DataOutOfRange { address, width })
            );
        }
    }

    let installed = simulator.snapshot().unwrap();
    for invalid in [
        {
            XtensaLx6State {
                halted: true,
                ..XtensaLx6State::default()
            }
        },
        {
            let mut state = installed.clone();
            state.length = 0;
            state
        },
        {
            let mut state = installed.clone();
            state.origin = Some(u32::MAX);
            state.length = 2;
            state
        },
    ] {
        let before = simulator.snapshot().unwrap();
        assert!(matches!(
            simulator.restore(invalid),
            Err(XtensaLx6Error::InvalidState { .. })
        ));
        assert_eq!(simulator.snapshot().unwrap(), before);
    }
    let mut outside_pc = installed;
    outside_pc.pc = u32::MAX;
    simulator.restore(outside_pc).unwrap();
    assert_eq!(
        simulator.step(),
        Err(XtensaLx6Error::FetchOutsideProgram { pc: u32::MAX })
    );
}

#[test]
fn production_classifier_covers_every_major_width_class() {
    let examples = [
        (rrr(8, 0, 1, 2, 3), "ADD"),
        (1 | (1 << 4), "L32R"),
        (rri8(0, 1, 2, 0, 2), "L8UI"),
        (5, "CALL0"),
        (6, "J"),
        (rri8(1, 2, 3, 1, 7), "BEQ"),
    ];
    for (raw, mnemonic) in examples {
        assert_eq!(classify_instruction(raw, 3).unwrap().mnemonic, mnemonic);
    }
    for (raw, mnemonic) in [
        (0x0128, "L32I.N"),
        (0x0129, "S32I.N"),
        (0x123a, "ADD.N"),
        (0x123b, "ADDI.N"),
        (0x010d, "MOV.N"),
        (0xf03d, "NOP.N"),
    ] {
        assert_eq!(classify_instruction(raw, 2).unwrap().mnemonic, mnemonic);
    }
    assert!(matches!(
        classify_instruction(3, 3),
        Err(XtensaLx6Error::UnsupportedOptionInstruction { .. })
    ));
    assert!(matches!(
        classify_instruction(0xffff, 2),
        Err(XtensaLx6Error::UnknownInstruction { .. })
    ));
}

#[test]
fn arithmetic_logic_aliasing_and_immediates_are_exact() {
    let families = [
        (rrr(8, 0, 1, 2, 3), 7u32),
        (rrr(9, 0, 1, 2, 3), 11),
        (rrr(10, 0, 1, 2, 3), 19),
        (rrr(11, 0, 1, 2, 3), 35),
        (rrr(12, 0, 1, 2, 3), 1),
        (rrr(13, 0, 1, 2, 3), 5),
        (rrr(14, 0, 1, 2, 3), 13),
        (rrr(15, 0, 1, 2, 3), 29),
        (rrr(1, 0, 1, 2, 3), 0),
        (rrr(2, 0, 1, 2, 3), 7),
        (rrr(3, 0, 1, 2, 3), 7),
    ];
    for (raw, expected) in families {
        let mut simulator = with_state(loaded(raw), |state| {
            state.ar[2] = 4;
            state.ar[3] = 3;
        });
        simulator.step().unwrap();
        assert_eq!(
            simulator.logical_register(1).unwrap(),
            expected,
            "raw {raw:#08x}"
        );
    }
    let mut simulator = with_state(loaded(rri8(12, 2, 2, 0xff, 2)), |state| state.ar[2] = 1);
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(2).unwrap(), 0);
    let movi = rri8(10, 0x0f, 4, 0xff, 2);
    let mut simulator = loaded(movi);
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(4).unwrap(), u32::MAX);
}

#[test]
fn sar_boundaries_ssa8b_and_extui_reserved_pairs() {
    for (low, expected) in [(0, 32), (1, 24), (2, 16), (3, 8)] {
        let mut simulator = with_state(loaded(rrr(4, 0, 3, 2, 0)), |state| state.ar[2] = low);
        simulator.step().unwrap();
        assert_eq!(simulator.snapshot().unwrap().sar, expected);
    }
    let srl = rrr(9, 1, 1, 0, 2);
    for sar in 0..=63 {
        let mut simulator = with_state(loaded(srl), |state| {
            state.sar = sar;
            state.ar[2] = u32::MAX;
        });
        let before = simulator.snapshot().unwrap();
        if sar <= 32 {
            simulator.step().unwrap();
        } else {
            assert_eq!(
                simulator.step(),
                Err(XtensaLx6Error::UndefinedOperation {
                    raw: srl,
                    width: 3,
                    reason: UndefinedReason::ShiftAmountOutOfRange
                })
            );
            assert_eq!(simulator.snapshot().unwrap(), before);
        }
    }
    let sll = rrr(10, 1, 1, 2, 0);
    for sar in 0..=63 {
        let mut simulator = with_state(loaded(sll), |state| {
            state.sar = sar;
            state.ar[2] = 1;
        });
        let before = simulator.snapshot().unwrap();
        if sar <= 32 {
            simulator.step().unwrap();
            let expected = if sar == 0 { 0 } else { 1 << (32 - sar) };
            assert_eq!(simulator.logical_register(1).unwrap(), expected);
        } else {
            assert_eq!(
                simulator.step(),
                Err(XtensaLx6Error::UndefinedOperation {
                    raw: sll,
                    width: 3,
                    reason: UndefinedReason::ShiftAmountOutOfRange
                })
            );
            assert_eq!(simulator.snapshot().unwrap(), before);
        }
    }
    for raw in [rrr(11, 1, 1, 0, 2), rrr(8, 1, 1, 2, 3)] {
        for sar in 0..=63 {
            let mut simulator = with_state(loaded(raw), |state| {
                state.sar = sar;
                state.ar[2] = 0x8000_0000;
                state.ar[3] = 1;
            });
            let before = simulator.snapshot().unwrap();
            if sar <= 32 {
                simulator.step().unwrap();
            } else {
                assert!(matches!(
                    simulator.step(),
                    Err(XtensaLx6Error::UndefinedOperation {
                        reason: UndefinedReason::ShiftAmountOutOfRange,
                        ..
                    })
                ));
                assert_eq!(simulator.snapshot().unwrap(), before);
            }
        }
    }
    for sa in 0..32u8 {
        for op2 in 0..16u8 {
            let raw = rrr(op2, 4 | (sa >> 4), 1, sa & 15, 2);
            let accepted = sa as u16 + op2 as u16 <= 31;
            assert_eq!(
                classify_instruction(raw, 3).is_ok(),
                accepted,
                "sa={sa} op2={op2}"
            );
        }
    }
}

#[test]
fn unaligned_memory_endianness_sign_extension_and_store_rollback() {
    let load = rri8(1, 2, 3, 1, 2);
    let mut simulator = with_state(loaded(load), |state| {
        state.ar[2] = 0x200;
        state.memory[0x202] = 0x34;
        state.memory[0x203] = 0x80;
    });
    let trace = simulator.step().unwrap();
    assert_eq!(simulator.logical_register(3).unwrap(), 0x8034);
    assert_eq!(
        trace.data_access,
        Some(DataAccess::Read {
            address: 0x202,
            width: 2,
            value: 0x8034
        })
    );

    let signed = rri8(9, 2, 3, 1, 2);
    let mut simulator = with_state(loaded(signed), |state| {
        state.ar[2] = 0x200;
        state.memory[0x202] = 0x34;
        state.memory[0x203] = 0x80;
    });
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(3).unwrap(), 0xffff_8034);

    let store = rri8(6, 2, 3, 0, 2);
    let mut simulator = with_state(loaded(store), |state| {
        state.ar[2] = 65_533;
        state.ar[3] = 0x4433_2211;
    });
    let before = simulator.snapshot().unwrap();
    assert_eq!(
        simulator.step(),
        Err(XtensaLx6Error::DataOutOfRange {
            address: 65_533,
            width: 4
        })
    );
    assert_eq!(simulator.snapshot().unwrap(), before);
}

#[test]
fn self_modifying_store_trace_keeps_original_instruction_bytes() {
    let store = rri8(4, 2, 3, 0, 2);
    let mut simulator = with_state(loaded(store), |state| {
        state.ar[2] = 0x100;
        state.ar[3] = 0xff;
    });
    let trace = simulator.step().unwrap();
    assert_eq!(trace.raw_bytes, bytes24(store));
    assert_eq!(simulator.snapshot().unwrap().memory[0x100], 0xff);
}

#[test]
fn every_reserved_branch_displacement_precedes_window_checks() {
    let selectors = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    for selector in selectors {
        for displacement in [-4i8, -3, -2, -1, 0, 1] {
            let raw = rri8(selector, 15, 15, displacement as u8, 7);
            let result = classify_instruction(raw, 3);
            assert_eq!(
                result.is_ok(),
                matches!(displacement, -4 | 1),
                "selector={selector} disp={displacement}"
            );
        }
    }
}

#[test]
fn every_branch_relation_has_taken_and_fallthrough_witnesses() {
    let register_cases = [
        (0, 0, 1, 1, 1),
        (1, 1, 1, 1, 2),
        (2, u32::MAX, 0, 0, u32::MAX),
        (3, 0, 1, 1, 0),
        (4, 3, 1, 0, 1),
        (5, 0, 1, 2, 1),
        (6, 0, 0, 1u32 << 3, 0),
        (7, 0, 0, 1u32 << 19, 0),
        (8, 1, 1, 0, 1),
        (9, 1, 2, 1, 1),
        (10, 0, u32::MAX, u32::MAX, 0),
        (11, 1, 0, 0, 1),
        (12, 0, 1, 1, 1),
        (13, 1, 0, 2, 0),
        (14, 1u32 << 3, 0, 0, 0),
        (15, 1u32 << 19, 0, 0, 0),
    ];
    for (selector, taken_left, taken_right, fall_left, fall_right) in register_cases {
        let raw = rri8(selector, 2, 3, 1, 7);
        for (left, right, expected_pc) in [
            (taken_left, taken_right, 0x105),
            (fall_left, fall_right, 0x103),
        ] {
            let mut simulator = with_state(loaded(raw), |state| {
                state.ar[2] = left;
                state.ar[3] = right;
            });
            simulator.step().unwrap();
            assert_eq!(
                simulator.snapshot().unwrap().pc,
                expected_pc,
                "r={selector}"
            );
        }
    }

    let signed = [-1i32, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256];
    let unsigned = [
        32768u32, 65536, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256,
    ];
    for index in 0..16u8 {
        let cases: [(u8, u32, u32); 6] = [
            (2, signed[index as usize] as u32, 0),
            (6, 0, signed[index as usize] as u32),
            (
                10,
                (signed[index as usize] - 1) as u32,
                signed[index as usize] as u32,
            ),
            (
                14,
                signed[index as usize] as u32,
                (signed[index as usize] - 1) as u32,
            ),
            (
                11,
                unsigned[index as usize].wrapping_sub(1),
                unsigned[index as usize],
            ),
            (
                15,
                unsigned[index as usize],
                unsigned[index as usize].wrapping_sub(1),
            ),
        ];
        for (t, taken, fallthrough) in cases {
            let raw = 6 | (u32::from(t) << 4) | (2 << 8) | (u32::from(index) << 12) | (1 << 16);
            for (value, expected_pc) in [(taken, 0x105), (fallthrough, 0x103)] {
                let mut simulator = with_state(loaded(raw), |state| state.ar[2] = value);
                simulator.step().unwrap();
                assert_eq!(simulator.snapshot().unwrap().pc, expected_pc);
            }
        }
    }
}

#[test]
fn lazy_window_priority_false_conditions_and_raw_reserved_order() {
    let beq = rri8(1, 15, 15, 1, 7);
    let mut simulator = with_state(loaded(beq), |state| {
        state.window_start |= (1 << 1) | (1 << 2) | (1 << 3)
    });
    let before = simulator.snapshot().unwrap();
    assert_eq!(
        simulator.step(),
        Err(XtensaLx6Error::WindowOverflow { increment: 1 })
    );
    assert_eq!(simulator.snapshot().unwrap(), before);

    let reserved = rri8(1, 15, 15, 0, 7);
    let mut simulator = with_state(loaded(reserved), |state| state.window_start |= 1 << 1);
    assert!(matches!(
        simulator.step(),
        Err(XtensaLx6Error::UnknownInstruction { .. })
    ));
}

#[test]
fn movsp_exhausts_caller_availability_and_self_move_is_legal() {
    for (source, destination) in [(2, 2), (2, 3)] {
        let movsp = rrr(0, 0, 1, source, destination);
        for mask in 0..8u16 {
            let mut simulator = with_state(loaded(movsp), |state| {
                state.window_base = 4;
                state.window_start = 1 << 4;
                state.ar[18] = 0x55;
                state.ar[19] = 0xaa;
                if mask & 1 != 0 {
                    state.window_start |= 1 << 1;
                }
                if mask & 2 != 0 {
                    state.window_start |= 1 << 2;
                }
                if mask & 4 != 0 {
                    state.window_start |= 1 << 3;
                }
            });
            let before = simulator.snapshot().unwrap();
            if mask == 0 {
                assert_eq!(simulator.step(), Err(XtensaLx6Error::AllocaCause));
                assert_eq!(simulator.snapshot().unwrap(), before);
            } else {
                simulator.step().unwrap();
                assert_eq!(simulator.logical_register(destination).unwrap(), 0x55);
            }
        }
    }

    for (source, active_increment) in [(4, 1), (8, 2), (12, 3)] {
        let movsp = rrr(0, 0, 1, source, 1);
        let mut simulator = with_state(loaded(movsp), |state| {
            state.window_base = 4;
            state.window_start = (1 << 4) | (1 << (4 + active_increment));
        });
        let before = simulator.snapshot().unwrap();
        assert_eq!(
            simulator.step(),
            Err(XtensaLx6Error::WindowOverflow {
                increment: active_increment,
            })
        );
        assert_eq!(simulator.snapshot().unwrap(), before);
    }
}

#[test]
fn entry_reads_caller_then_writes_callee_and_retw_is_atomic() {
    let entry = 6 | (3 << 4) | (2 << 8) | (1 << 12);
    for call_inc in 0..=3u8 {
        let mut simulator = with_state(loaded(entry), |state| {
            state.call_inc = call_inc;
            state.ar[2] = 0x1000;
        });
        let before = simulator.snapshot().unwrap();
        simulator.step().unwrap();
        let state = simulator.snapshot().unwrap();
        let destination = usize::from(call_inc) * 4 + 2;
        assert_eq!(state.window_base, call_inc);
        assert_eq!(state.ar[destination], 0x0ff8);
        assert_ne!(state.window_start & (1 << call_inc), 0);
        for physical in 0..64 {
            if physical != destination {
                assert_eq!(state.ar[physical], before.ar[physical]);
            }
        }
    }

    let bad_entry = 6 | (3 << 4) | (4 << 8);
    let mut simulator = with_state(loaded(bad_entry), |state| {
        state.call_inc = 1;
        state.window_start |= 2;
    });
    assert_eq!(
        simulator.step(),
        Err(XtensaLx6Error::WindowOverflow { increment: 1 })
    );

    let retw = rrr(0, 0, 0, 0, 9);
    let mut simulator = with_state(loaded(retw), |state| {
        state.window_base = 1;
        state.window_start = 3;
        state.ar[4] = 0x4000_0123;
    });
    simulator.step().unwrap();
    let state = simulator.snapshot().unwrap();
    assert_eq!(state.window_base, 0);
    assert_eq!(state.pc, 0x123);
    assert_eq!(state.window_start, 1);

    for source in 4..=15u8 {
        let raw = 6 | (3 << 4) | (u32::from(source) << 8);
        let mut simulator = loaded(raw);
        let before = simulator.snapshot().unwrap();
        assert_eq!(
            simulator.step(),
            Err(XtensaLx6Error::UndefinedOperation {
                raw,
                width: 3,
                reason: UndefinedReason::InvalidEntrySource,
            })
        );
        assert_eq!(simulator.snapshot().unwrap(), before);
    }

    for increment in 0..=3u8 {
        for preceding in 0..8u16 {
            let mut simulator = with_state(loaded(retw), |state| {
                state.window_base = 4;
                state.window_start = 1 << 4;
                for back in 1..=3u8 {
                    if preceding & (1 << (back - 1)) != 0 {
                        state.window_start |= 1 << (4 - back);
                    }
                }
                state.ar[16] = (u32::from(increment) << 30) | 0x123;
            });
            let before = simulator.snapshot().unwrap();
            let nearest = (1..=3u8).find(|back| preceding & (1 << (back - 1)) != 0);
            if increment == 0 || nearest.is_some_and(|value| value != increment) {
                assert!(matches!(
                    simulator.step(),
                    Err(XtensaLx6Error::UndefinedOperation {
                        reason: UndefinedReason::InvalidReturnWindow,
                        ..
                    })
                ));
                assert_eq!(simulator.snapshot().unwrap(), before);
            } else if nearest.is_none() {
                assert_eq!(
                    simulator.step(),
                    Err(XtensaLx6Error::WindowUnderflow { increment })
                );
                assert_eq!(simulator.snapshot().unwrap(), before);
            } else {
                simulator.step().unwrap();
                assert_eq!(simulator.snapshot().unwrap().window_base, 4 - increment);
            }
        }
    }
}

#[test]
fn loop_setup_loopback_and_control_exit_follow_fetch_candidate() {
    let loop_raw = rri8(8, 1, 7, 3, 6);
    let mut program = Vec::from(bytes24(loop_raw));
    program.extend_from_slice(&bytes16(0xf03d));
    program.extend_from_slice(&bytes16(0xf03d));
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&program, 0x101).unwrap();
    simulator.set_logical_register(1, 2).unwrap();
    simulator.step().unwrap();
    let state = simulator.snapshot().unwrap();
    assert_eq!((state.lbeg, state.lend, state.lcount), (0x104, 0x108, 1));
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().pc, 0x106);
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().pc, 0x104);
    assert_eq!(simulator.snapshot().unwrap().lcount, 0);
}

#[test]
fn loop_count_condition_alignment_and_exit_matrix() {
    let loop_always = rri8(8, 1, 7, 2, 6);
    let mut wide_program = Vec::from(bytes24(loop_always));
    wide_program.extend_from_slice(&bytes24(0x0020f0));
    for count in [0, 1, 2, u32::MAX] {
        let mut simulator = XtensaLx6Simulator::new();
        simulator.load(&wide_program, 0x101).unwrap();
        simulator.set_logical_register(1, count).unwrap();
        let mut state = simulator.snapshot().unwrap();
        state.lbeg = 0x20;
        state.lend = 0x30;
        state.lcount = 7;
        simulator.restore(state).unwrap();
        simulator.step().unwrap();
        let state = simulator.snapshot().unwrap();
        assert_eq!(state.lcount, count.wrapping_sub(1));
        assert_eq!((state.lbeg, state.lend, state.pc), (0x104, 0x107, 0x104));
    }

    for (selector, count, skipped) in [
        (9, 0, true),
        (9, 1, false),
        (10, 0, true),
        (10, u32::MAX, true),
        (10, 1, false),
    ] {
        let setup = rri8(selector, 1, 7, 2, 6);
        let mut program = Vec::from(bytes24(setup));
        program.extend_from_slice(&bytes24(0x0020f0));
        let mut simulator = XtensaLx6Simulator::new();
        simulator.load(&program, 0x101).unwrap();
        simulator.set_logical_register(1, count).unwrap();
        simulator.step().unwrap();
        assert_eq!(
            simulator.snapshot().unwrap().pc,
            if skipped { 0x107 } else { 0x104 }
        );
    }

    let loop_density = rri8(8, 1, 7, 1, 6);
    let mut density_program = Vec::from(bytes24(loop_density));
    density_program.extend_from_slice(&bytes16(0xf03d));
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&density_program, 0x101).unwrap();
    simulator.set_logical_register(1, 1).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().lend, 0x106);

    for (origin, setup) in [(0x103, loop_always), (0x101, rri8(8, 1, 7, 0, 6))] {
        let mut program = Vec::from(bytes24(setup));
        program.extend_from_slice(&bytes24(0x0020f0));
        let mut simulator = XtensaLx6Simulator::new();
        simulator.load(&program, origin).unwrap();
        simulator.set_logical_register(1, 2).unwrap();
        let before = simulator.snapshot().unwrap();
        assert_eq!(simulator.step(), Err(XtensaLx6Error::InvalidLoop));
        assert_eq!(simulator.snapshot().unwrap(), before);
    }

    let jump_to_lend = (0x3ffff << 6) | 6;
    let mut exit_program = Vec::from(bytes24(loop_always));
    exit_program.extend_from_slice(&bytes24(jump_to_lend));
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&exit_program, 0x101).unwrap();
    simulator.set_logical_register(1, 2).unwrap();
    simulator.step().unwrap();
    simulator.step().unwrap();
    let state = simulator.snapshot().unwrap();
    assert_eq!(state.pc, state.lend);
    assert_eq!(state.lcount, 1, "taken J must suppress automatic loopback");
}

#[test]
fn run_and_execute_are_wholly_transactional() {
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes16(0xf03d), 0x20).unwrap();
    let before = simulator.snapshot().unwrap();
    assert_eq!(
        simulator.run(1),
        Err(XtensaLx6Error::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(simulator.snapshot().unwrap(), before);

    let old = simulator.snapshot().unwrap();
    assert!(matches!(
        simulator.execute(&bytes24(0xfffff0), 0x100, 1),
        Err(XtensaLx6Error::UnknownInstruction { .. })
    ));
    assert_eq!(simulator.snapshot().unwrap(), old);

    let mut program = Vec::from(bytes16(0xf03d));
    program.extend_from_slice(&bytes24(0x004000));
    let result = simulator.execute(&program, 0x100, 2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(simulator.step(), Err(XtensaLx6Error::Halted));
}

#[test]
fn full_included_classification_surface_uses_exact_tokens() {
    let wide = [
        (rrr(6, 0, 1, 0, 2), "NEG"),
        (rrr(6, 0, 1, 1, 2), "ABS"),
        (rri8(13, 2, 1, 4, 2), "ADDMI"),
        (rrr(8, 3, 1, 2, 3), "MOVEQZ"),
        (rrr(9, 3, 1, 2, 3), "MOVNEZ"),
        (rrr(10, 3, 1, 2, 3), "MOVLTZ"),
        (rrr(11, 3, 1, 2, 3), "MOVGEZ"),
        (rrr(0, 1, 1, 2, 1), "SLLI"),
        (rrr(2, 1, 1, 2, 3), "SRAI"),
        (rrr(4, 1, 1, 2, 3), "SRLI"),
        (rrr(8, 1, 1, 2, 3), "SRC"),
        (rrr(9, 1, 1, 0, 3), "SRL"),
        (rrr(10, 1, 1, 2, 0), "SLL"),
        (rrr(11, 1, 1, 0, 3), "SRA"),
        (rrr(4, 0, 0, 2, 0), "SSR"),
        (rrr(4, 0, 1, 2, 0), "SSL"),
        (rrr(4, 0, 2, 2, 0), "SSA8L"),
        (rrr(4, 0, 4, 2, 0), "SSAI"),
        (rri8(2, 2, 3, 1, 2), "L32I"),
        (rri8(5, 2, 3, 1, 2), "S16I"),
        (rri8(6, 2, 3, 1, 2), "S32I"),
        (6 | (1 << 4) | (2 << 8) | (1 << 12), "BEQZ"),
        (6 | (5 << 4) | (2 << 8) | (1 << 12), "BNEZ"),
        (6 | (9 << 4) | (2 << 8) | (1 << 12), "BLTZ"),
        (6 | (13 << 4) | (2 << 8) | (1 << 12), "BGEZ"),
        (6 | (2 << 4) | (2 << 8) | (1 << 12) | (1 << 16), "BEQI"),
        (6 | (6 << 4) | (2 << 8) | (1 << 12) | (1 << 16), "BNEI"),
        (6 | (10 << 4) | (2 << 8) | (1 << 12) | (1 << 16), "BLTI"),
        (6 | (14 << 4) | (2 << 8) | (1 << 12) | (1 << 16), "BGEI"),
        (6 | (11 << 4) | (2 << 8) | (1 << 12) | (1 << 16), "BLTUI"),
        (6 | (15 << 4) | (2 << 8) | (1 << 12) | (1 << 16), "BGEUI"),
        (rrr(0, 0, 0, 2, 10), "JX"),
        (rrr(0, 0, 0, 2, 12), "CALLX0"),
        (rrr(0, 0, 0, 2, 13), "CALLX4"),
        (rrr(0, 0, 0, 2, 14), "CALLX8"),
        (rrr(0, 0, 0, 2, 15), "CALLX12"),
        (5 | (1 << 4), "CALL4"),
        (5 | (2 << 4), "CALL8"),
        (5 | (3 << 4), "CALL12"),
        (rrr(0, 0, 0, 0, 8), "RET"),
        (rrr(0, 0, 1, 2, 3), "MOVSP"),
        (rrr(0, 3, 0, 3, 2), "RSR"),
        (rrr(1, 3, 0, 3, 2), "WSR"),
        (rrr(6, 1, 0, 3, 2), "XSR"),
        (rrr(0, 0, 2, 0, 0), "ISYNC"),
        (rrr(0, 0, 2, 0, 1), "RSYNC"),
        (rrr(0, 0, 2, 0, 2), "ESYNC"),
        (rrr(0, 0, 2, 0, 3), "DSYNC"),
        (rrr(0, 0, 2, 0, 12), "MEMW"),
        (rrr(0, 0, 2, 0, 13), "EXTW"),
    ];
    for (raw, token) in wide {
        assert_eq!(
            classify_instruction(raw, 3).unwrap().mnemonic,
            token,
            "raw={raw:#08x}"
        );
    }
    let narrow = [
        (0x1238, "L32I.N"),
        (0x1239, "S32I.N"),
        (0x123a, "ADD.N"),
        (0x123b, "ADDI.N"),
        (0x128c, "BEQZ.N"),
        (0x12cc, "BNEZ.N"),
        (0xf00d, "RET.N"),
        (0xf01d, "RETW.N"),
    ];
    for (raw, token) in narrow {
        assert_eq!(classify_instruction(raw, 2).unwrap().mnemonic, token);
    }
}

#[test]
fn privileged_window_handler_load_store_are_recognized() {
    for raw in [0x09_0000, 0x49_0000] {
        assert_eq!(
            classify_instruction(raw, 3).unwrap_err(),
            XtensaLx6Error::PrivilegedInstruction { raw, width: 3 }
        );
    }

    let deferred = [
        rrr(8, 2, 1, 2, 3),                   // MULLI
        rrr(0, 8, 1, 2, 3),                   // LSX
        rrr(0, 10, 1, 2, 3),                  // ADD.S
        rri8(11, 1, 2, 0, 2),                 // L32AI
        3,                                    // LSI floating-point option
        6 | (7 << 4) | (1 << 12) | (4 << 16), // BFP
        rrr(0, 0, 2, 0, 8),                   // EXCW
    ];
    for raw in deferred {
        assert!(matches!(
            classify_instruction(raw, 3),
            Err(XtensaLx6Error::UnsupportedOptionInstruction { .. })
        ));
    }
    let reserved = [
        rrr(3, 2, 1, 2, 3),
        rrr(2, 8, 1, 2, 3),
        rrr(2, 9, 1, 2, 3),
        rrr(0, 0, 3, 15, 15),
        rrr(0, 0, 7, 1, 1),
    ];
    for raw in reserved {
        assert!(
            matches!(
                classify_instruction(raw, 3),
                Err(XtensaLx6Error::UnknownInstruction { .. })
            ),
            "raw={raw:#08x}"
        );
    }
}

#[test]
fn representative_shift_conditional_special_and_control_semantics() {
    let cases = [
        (rrr(8, 3, 1, 2, 3), 0, 9),
        (rrr(9, 3, 1, 2, 3), 1, 9),
        (rrr(10, 3, 1, 2, 3), u32::MAX, 9),
        (rrr(11, 3, 1, 2, 3), 0, 9),
    ];
    for (raw, test, expected) in cases {
        let mut simulator = with_state(loaded(raw), |state| {
            state.ar[1] = 4;
            state.ar[2] = expected;
            state.ar[3] = test;
        });
        simulator.step().unwrap();
        assert_eq!(simulator.logical_register(1).unwrap(), expected);
    }
    for (raw, input, expected) in [
        (rrr(1, 1, 1, 2, 13), 1, 8),
        (rrr(2, 1, 1, 1, 2), u32::MAX, u32::MAX),
        (rrr(4, 1, 1, 3, 2), 8, 1),
    ] {
        let mut simulator = with_state(loaded(raw), |state| state.ar[2] = input);
        simulator.step().unwrap();
        assert_eq!(simulator.logical_register(1).unwrap(), expected);
    }
    let mut simulator = with_state(loaded(rrr(4, 0, 1, 2, 0)), |state| state.ar[2] = 7);
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().sar, 25);
    let mut simulator = with_state(loaded(rrr(4, 0, 0, 2, 0)), |state| state.ar[2] = 35);
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().sar, 3);
    let mut simulator = loaded(rrr(4, 0, 4, 3, 1));
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().sar, 19);

    let rsr = rrr(0, 3, 0, 3, 2);
    let mut simulator = with_state(loaded(rsr), |state| state.sar = 17);
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(2).unwrap(), 17);
    let wsr = rrr(1, 3, 0, 3, 2);
    let mut simulator = with_state(loaded(wsr), |state| state.ar[2] = 70);
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().sar, 6);
    let xsr = rrr(6, 1, 0, 0, 2);
    let mut simulator = with_state(loaded(xsr), |state| {
        state.lbeg = 0x44;
        state.ar[2] = 0x88;
    });
    simulator.step().unwrap();
    let state = simulator.snapshot().unwrap();
    assert_eq!(state.lbeg, 0x88);
    assert_eq!(state.ar[2], 0x44);

    let xsr_sar = rrr(6, 1, 0, 3, 2);
    let mut simulator = with_state(loaded(xsr_sar), |state| {
        state.sar = 17;
        state.ar[2] = 35;
    });
    simulator.step().unwrap();
    let state = simulator.snapshot().unwrap();
    assert_eq!(state.sar, 35);
    assert_eq!(state.ar[2], 17);

    let jx = rrr(0, 0, 0, 2, 10);
    let mut simulator = with_state(loaded(jx), |state| state.ar[2] = 0x1234);
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().pc, 0x1234);

    let mut simulator = with_state(loaded(5), |state| state.call_inc = 3);
    simulator.step().unwrap();
    let state = simulator.snapshot().unwrap();
    assert_eq!(state.call_inc, 3, "CALL0 must preserve PS.CALLINC");
    assert_eq!(state.ar[0], 0x103);
    assert_eq!(state.pc, 0x104);

    let mut simulator = with_state(loaded(5 | (1 << 4)), |state| state.call_inc = 3);
    simulator.step().unwrap();
    let state = simulator.snapshot().unwrap();
    assert_eq!(state.call_inc, 1);
    assert_eq!(state.ar[4], 0x4000_0103);
}

#[test]
fn direct_indirect_call_and_return_links_are_exact() {
    for increment in 0..=3u8 {
        let direct = 5 | (u32::from(increment) << 4);
        let mut simulator = with_state(loaded(direct), |state| state.call_inc = 3);
        simulator.step().unwrap();
        let state = simulator.snapshot().unwrap();
        assert_eq!(state.pc, 0x104);
        assert_eq!(
            state.ar[usize::from(increment) * 4],
            (u32::from(increment) << 30) | 0x103
        );
        assert_eq!(state.call_inc, if increment == 0 { 3 } else { increment });

        let indirect = rrr(0, 0, 0, 2, 12 + increment);
        let mut simulator = with_state(loaded(indirect), |state| {
            state.ar[2] = 0x1234_5000 + u32::from(increment);
            state.call_inc = 3;
        });
        simulator.step().unwrap();
        let state = simulator.snapshot().unwrap();
        assert_eq!(state.pc, 0x1234_5000 + u32::from(increment));
        assert_eq!(
            state.ar[usize::from(increment) * 4],
            (u32::from(increment) << 30) | 0x103
        );
        assert_eq!(state.call_inc, if increment == 0 { 3 } else { increment });
    }

    for (program, target) in [
        (Vec::from(bytes24(rrr(0, 0, 0, 0, 8))), 0x2222_3333),
        (Vec::from(bytes16(0xf00d)), 0x4444_5555),
    ] {
        let mut simulator = XtensaLx6Simulator::new();
        simulator.load(&program, 0x100).unwrap();
        simulator.set_logical_register(0, target).unwrap();
        simulator.step().unwrap();
        assert_eq!(simulator.snapshot().unwrap().pc, target);
        let transferred = simulator.snapshot().unwrap();
        assert_eq!(
            simulator.step(),
            Err(XtensaLx6Error::FetchOutsideProgram { pc: target })
        );
        assert_eq!(simulator.snapshot().unwrap(), transferred);
    }

    let jump = 6 | (1 << 6);
    let mut simulator = loaded(jump);
    simulator.step().unwrap();
    assert_eq!(simulator.snapshot().unwrap().pc, 0x105);
}

#[test]
fn narrow_execution_forms_and_public_memory_helpers() {
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes16(0x123a), 0x100).unwrap();
    simulator.set_logical_register(2, 5).unwrap();
    simulator.set_logical_register(3, 7).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(1).unwrap(), 12);

    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes16(0x123b), 0x100).unwrap();
    simulator.set_logical_register(2, 9).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(1).unwrap(), 12);
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes16(0x012d), 0x100).unwrap();
    simulator.set_logical_register(2, 0x55).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(1).unwrap(), 0x55);
    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes16(0x710c), 0x100).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(1).unwrap(), 7);

    let mut simulator = XtensaLx6Simulator::new();
    simulator.load(&bytes16(0x3128), 0x100).unwrap();
    simulator.set_logical_register(1, 0x200).unwrap();
    simulator.write_memory(0x20c, 4, 0x4433_2211).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.logical_register(2).unwrap(), 0x4433_2211);
    assert_eq!(simulator.read_memory(0x20c, 4).unwrap(), 0x4433_2211);
    assert!(matches!(
        simulator.read_memory(0, 3),
        Err(XtensaLx6Error::InvalidState { .. })
    ));
    assert!(simulator.logical_register(16).is_err());
    assert!(simulator.set_logical_register(16, 0).is_err());
}

#[test]
fn typed_error_messages_and_limit_paths_are_nonempty() {
    let errors = vec![
        XtensaLx6Error::EmptyProgram,
        XtensaLx6Error::ProgramOutOfRange {
            origin: 1,
            length: 2,
        },
        XtensaLx6Error::ProgramAddressOverflow {
            origin: u32::MAX,
            length: 2,
        },
        XtensaLx6Error::NoProgramLoaded,
        XtensaLx6Error::FetchOutsideProgram { pc: 1 },
        XtensaLx6Error::TruncatedInstruction {
            pc: 1,
            needed: 3,
            available: 2,
        },
        XtensaLx6Error::FetchOutOfRange { pc: 1, width: 3 },
        XtensaLx6Error::UnknownInstruction { raw: 1, width: 3 },
        XtensaLx6Error::UnsupportedOptionInstruction { raw: 1, width: 3 },
        XtensaLx6Error::UndefinedOperation {
            raw: 1,
            width: 3,
            reason: UndefinedReason::InvalidEntrySource,
        },
        XtensaLx6Error::UnsupportedSpecialRegister { number: 9 },
        XtensaLx6Error::PrivilegedInstruction { raw: 1, width: 3 },
        XtensaLx6Error::Breakpoint { code: 1 },
        XtensaLx6Error::DataOutOfRange {
            address: 1,
            width: 4,
        },
        XtensaLx6Error::InvalidLoop,
        XtensaLx6Error::WindowOverflow { increment: 1 },
        XtensaLx6Error::WindowUnderflow { increment: 1 },
        XtensaLx6Error::AllocaCause,
        XtensaLx6Error::InvalidState { reason: "test" },
        XtensaLx6Error::Halted,
        XtensaLx6Error::TraceLimitExceeded {
            requested: 65,
            limit: 64,
        },
        XtensaLx6Error::StepLimitExceeded { limit: 1 },
        XtensaLx6Error::AllocationFailed,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
    let mut simulator = XtensaLx6Simulator::new();
    assert_eq!(
        simulator.profile(),
        xtensa_lx6_simulator::XtensaProfile::Esp32Lx6IntegerV1
    );
    assert!(matches!(
        simulator.run(65),
        Err(XtensaLx6Error::TraceLimitExceeded { .. })
    ));
}
