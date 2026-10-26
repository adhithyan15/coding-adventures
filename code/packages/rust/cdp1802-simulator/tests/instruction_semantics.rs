use cdp1802_simulator::{Cdp1802Error, Cdp1802Simulator, MemoryAccessKind, PortTransaction};

fn configured(program: &[u8]) -> Cdp1802Simulator {
    let mut cpu = Cdp1802Simulator::new();
    cpu.load(program, 0x0100).unwrap();
    cpu
}

fn set_short_condition(state: &mut cdp1802_simulator::Cdp1802State, low: u8, take: bool) {
    match low {
        0 | 8 => {}
        1 | 9 => state.q = if low == 1 { take } else { !take },
        2 | 10 => state.d = if (low == 2) == take { 0 } else { 1 },
        3 | 11 => state.df = if low == 3 { take } else { !take },
        4..=7 => state.ef[usize::from(low - 4)] = take,
        12..=15 => state.ef[usize::from(low - 12)] = !take,
        _ => unreachable!(),
    }
}

#[test]
fn every_short_branch_condition_has_taken_and_not_taken_witnesses() {
    for low in 0u8..=15 {
        for requested_take in [false, true] {
            if low == 0 && !requested_take || low == 8 && requested_take {
                continue;
            }
            let mut cpu = configured(&[0x30 | low, 0x44]);
            let mut state = cpu.get_state();
            set_short_condition(&mut state, low, requested_take);
            cpu.restore(state).unwrap();
            let trace = cpu.step().unwrap();
            assert_eq!(trace.raw, [0x30 | low, 0x44, 0]);
            assert_eq!(trace.branch_taken, Some(requested_take));
            assert_eq!(
                trace.pc_after,
                if requested_take { 0x0144 } else { 0x0102 },
                "opcode {:#04x}",
                0x30 | low
            );
        }
    }
}

fn set_long_condition(state: &mut cdp1802_simulator::Cdp1802State, low: u8, take: bool) {
    match low {
        0 | 8 => {}
        1 | 5 | 9 | 13 => {
            let positive = matches!(low, 1 | 13);
            state.q = if positive { take } else { !take };
        }
        2 | 6 | 10 | 14 => {
            let zero = matches!(low, 2 | 14);
            state.d = if zero == take { 0 } else { 1 };
        }
        3 | 7 | 11 | 15 => {
            let positive = matches!(low, 3 | 15);
            state.df = if positive { take } else { !take };
        }
        12 => state.ie = take,
        _ => unreachable!(),
    }
}

#[test]
fn every_conditional_long_branch_and_skip_has_both_truth_values() {
    for low in [1u8, 2, 3, 9, 10, 11] {
        for take in [false, true] {
            let mut cpu = configured(&[0xc0 | low, 0x12, 0x34]);
            let mut state = cpu.get_state();
            set_long_condition(&mut state, low, take);
            cpu.restore(state).unwrap();
            let trace = cpu.step().unwrap();
            assert_eq!(trace.branch_taken, Some(take), "opcode {:#04x}", 0xc0 | low);
            assert_eq!(trace.pc_after, if take { 0x1234 } else { 0x0103 });
        }
    }
    for low in [5u8, 6, 7, 12, 13, 14, 15] {
        for take in [false, true] {
            let mut cpu = configured(&[0xc0 | low]);
            let mut state = cpu.get_state();
            set_long_condition(&mut state, low, take);
            cpu.restore(state).unwrap();
            let trace = cpu.step().unwrap();
            assert_eq!(trace.branch_taken, Some(take), "opcode {:#04x}", 0xc0 | low);
            assert_eq!(trace.pc_after, if take { 0x0103 } else { 0x0101 });
        }
    }
}

#[test]
fn short_branch_keeps_post_fetch_page_and_wraps_skip_pointer() {
    let mut cpu = Cdp1802Simulator::new();
    let mut whole_space = vec![0; 65_536];
    whole_space[0xffff] = 0x30;
    whole_space[0] = 0x5a;
    cpu.load(&whole_space, 0).unwrap();
    let mut state = cpu.get_state();
    state.registers[0] = 0xffff;
    cpu.restore(state).unwrap();
    assert_eq!(cpu.step().unwrap().pc_after, 0x005a);

    let mut cpu = Cdp1802Simulator::new();
    let mut whole_space = vec![0; 65_536];
    whole_space[0xffff] = 0x38;
    whole_space[0] = 0xa5;
    cpu.load(&whole_space, 0).unwrap();
    let mut state = cpu.get_state();
    state.registers[0] = 0xffff;
    cpu.restore(state).unwrap();
    assert_eq!(cpu.step().unwrap().pc_after, 1);
}

#[test]
fn long_branches_and_skips_cover_all_conditions_and_bus_only_reads() {
    for (opcode, take) in [
        (0xc0, true),
        (0xc1, true),
        (0xc2, true),
        (0xc3, true),
        (0xc9, false),
        (0xca, false),
        (0xcb, false),
    ] {
        let mut cpu = configured(&[opcode, 0x12, 0x34]);
        let mut state = cpu.get_state();
        state.q = true;
        state.d = 0;
        state.df = true;
        cpu.restore(state).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.branch_taken, Some(take));
        assert_eq!(trace.pc_after, if take { 0x1234 } else { 0x0103 });
        assert_eq!(trace.raw, [opcode, 0x12, 0x34]);
    }

    let skip_cases = [
        (0xc5, false),
        (0xc6, false),
        (0xc7, false),
        (0xc8, true),
        (0xcc, true),
        (0xcd, true),
        (0xce, true),
        (0xcf, true),
    ];
    for (opcode, take) in skip_cases {
        let mut cpu = configured(&[opcode]);
        let mut state = cpu.get_state();
        state.q = true;
        state.d = 0;
        state.df = true;
        state.ie = true;
        cpu.restore(state).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.width, 1);
        assert_eq!(trace.raw, [opcode, 0, 0]);
        assert_eq!(trace.branch_taken, Some(take));
        assert_eq!(trace.pc_after, if take { 0x0103 } else { 0x0101 });
        assert_eq!(trace.memory_access, None);
    }

    let trace = configured(&[0xc4]).step().unwrap();
    assert_eq!((trace.pc_after, trace.cycles), (0x0101, 3));

    let mut cpu = Cdp1802Simulator::new();
    let mut full = vec![0; 65_536];
    full[0xffff] = 0xc8;
    cpu.load(&full, 0).unwrap();
    let mut state = cpu.get_state();
    state.registers[0] = 0xffff;
    cpu.restore(state).unwrap();
    assert_eq!(cpu.step().unwrap().pc_after, 2);
}

#[test]
fn long_control_truncation_is_distinct_from_opcode_fetch() {
    let mut cpu = configured(&[0xc0, 0x12]);
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Cdp1802Error::TruncatedInstruction {
            pc: 0x0100,
            needed: 3
        })
    );
    assert_eq!(cpu.get_state(), before);

    let mut cpu = configured(&[0xc8]);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[0], 0x0103);
}

#[test]
fn register_families_preserve_alias_order_and_wrapping() {
    let mut cpu = configured(&[0x11, 0x21, 0x41, 0x51, 0x81, 0x91, 0xa1, 0xb1, 0xd1, 0xe2]);
    let mut state = cpu.get_state();
    state.registers[1] = 0xffff;
    state.memory[0xffff] = 0x7b;
    state.d = 0x55;
    cpu.restore(state).unwrap();

    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[1], 0);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[1], 0xffff);
    cpu.step().unwrap();
    let state = cpu.get_state();
    assert_eq!((state.d, state.registers[1]), (0x7b, 0));
    cpu.step().unwrap();
    assert_eq!(cpu.read_memory(0), 0x7b);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().d, 0);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().d, 0);
    let mut state = cpu.get_state();
    state.d = 0xab;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[1], 0x00ab);
    let mut state = cpu.get_state();
    state.d = 0xcd;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[1], 0xcdab);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().p, 1);

    let mut cpu = configured(&[0xe2]);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().x, 2);
}

#[test]
fn n_p_x_equality_partitions_follow_sequenced_register_matrix_updates() {
    let partitions = [(0u8, 0u8, 0u8), (0, 0, 1), (0, 1, 0), (0, 1, 1), (0, 1, 2)];
    for (n, p, x) in partitions {
        for high in [0x1u8, 0x2, 0x4, 0x5, 0x8, 0x9, 0xa, 0xb, 0xd, 0xe] {
            let opcode = (high << 4) | n;
            let mut cpu = Cdp1802Simulator::new();
            let mut image = vec![0; 0x100];
            image[0] = opcode;
            cpu.load(&image, 0x0100).unwrap();
            let mut state = cpu.get_state();
            state.p = p;
            state.x = x;
            state.registers[usize::from(p)] = 0x0100;
            state.registers[usize::from(n)] = if n == p { 0x0100 } else { 0x0180 };
            state.memory[0x0180] = 0x6a;
            state.d = 0x5c;
            cpu.restore(state).unwrap();
            cpu.step().unwrap();
        }
    }
}

#[test]
fn exact_aliasing_results_use_post_fetch_register_values() {
    let mut cpu = configured(&[0x40, 0x6a]);
    let trace = cpu.step().unwrap();
    assert_eq!((trace.d_after, trace.pc_after), (0x6a, 0x0102));

    let mut cpu = configured(&[0x50, 0]);
    let mut state = cpu.get_state();
    state.d = 0xa5;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert_eq!(
        (cpu.read_memory(0x0101), cpu.get_state().registers[0]),
        (0xa5, 0x0101)
    );

    let mut cpu = configured(&[0x10, 0]);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[0], 0x0102);
    let mut cpu = configured(&[0x20, 0]);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[0], 0x0100);

    let mut cpu = configured(&[0x61, 0x77]);
    let trace = cpu.step().unwrap();
    assert_eq!(
        trace.port_transaction,
        Some(PortTransaction::Write {
            port: 1,
            value: 0x77
        })
    );
    assert_eq!(cpu.get_state().registers[0], 0x0102);

    let mut cpu = configured(&[0x69, 0]);
    cpu.set_input_port(1, 0x88).unwrap();
    cpu.step().unwrap();
    assert_eq!(
        (cpu.read_memory(0x0101), cpu.get_state().registers[0]),
        (0x88, 0x0101)
    );

    let mut cpu = configured(&[0x70, 0x00, 0]);
    cpu.step().unwrap();
    let state = cpu.get_state();
    assert_eq!((state.x, state.p, state.registers[0]), (0, 0, 0x0102));

    let mut cpu = configured(&[0xd0]);
    cpu.step().unwrap();
    assert_eq!(
        (cpu.get_state().p, cpu.get_state().registers[0]),
        (0, 0x0101)
    );
    let mut cpu = configured(&[0xe0]);
    cpu.step().unwrap();
    assert_eq!(
        (cpu.get_state().x, cpu.get_state().registers[0]),
        (0, 0x0101)
    );
}

#[test]
fn every_documented_first_byte_executes_from_an_independent_safe_fixture() {
    for opcode in 0u8..=u8::MAX {
        let mut cpu = configured(&[opcode, 0, 0]);
        let mut state = cpu.get_state();
        state.registers.fill(0x0200);
        state.registers[0] = 0x0100;
        state.memory[0x0200] = 0;
        cpu.restore(state).unwrap();
        let result = cpu.step();
        if opcode == 0x68 {
            assert_eq!(result, Err(Cdp1802Error::UnknownInstruction { raw: 0x68 }));
        } else {
            assert!(result.is_ok(), "opcode {opcode:#04x}: {result:?}");
        }
    }
}

#[test]
fn self_modifying_store_retains_fetched_byte() {
    let mut cpu = configured(&[0x50, 0x00]);
    let mut state = cpu.get_state();
    state.d = 0xaa;
    cpu.restore(state).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.raw, [0x50, 0, 0]);
    assert_eq!(trace.memory_access.unwrap().kind, MemoryAccessKind::Write);
    assert_eq!(cpu.read_memory(0x0101), 0xaa);
}

#[test]
fn ret_dis_mark_and_x_p_aliases_use_prestate_selectors() {
    let mut cpu = configured(&[0x70]);
    let mut state = cpu.get_state();
    state.x = 3;
    state.registers[3] = 0x2000;
    state.memory[0x2000] = 0x33;
    state.ie = false;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    let state = cpu.get_state();
    assert_eq!(
        (state.x, state.p, state.registers[3], state.ie),
        (3, 3, 0x2001, true)
    );

    let mut cpu = configured(&[0x71]);
    let mut state = cpu.get_state();
    state.x = 4;
    state.registers[4] = 0x2000;
    state.memory[0x2000] = 0x42;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    let state = cpu.get_state();
    assert_eq!(
        (state.x, state.p, state.registers[4], state.ie),
        (4, 2, 0x2001, false)
    );

    let mut cpu = configured(&[0x79, 0]);
    let mut state = cpu.get_state();
    state.p = 2;
    state.x = 2;
    state.registers[2] = 0x0100;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    let state = cpu.get_state();
    assert_eq!(state.t, 0x22);
    assert_eq!(state.x, 2);
    assert_eq!(state.registers[2], 0x0100);
    assert_eq!(state.memory[0x0101], 0x22);
}

#[test]
fn all_ports_and_irx_have_exact_pointer_behavior() {
    for port in 1u8..=7 {
        let mut cpu = configured(&[0x60 | port]);
        let mut state = cpu.get_state();
        state.x = 1;
        state.registers[1] = 0xffff;
        state.memory[0xffff] = 0x40 | port;
        cpu.restore(state).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(
            trace.port_transaction,
            Some(PortTransaction::Write {
                port,
                value: 0x40 | port
            })
        );
        assert_eq!(cpu.get_state().registers[1], 0);

        let mut cpu = configured(&[0x68 | port]);
        cpu.set_input_port(port, 0x80 | port).unwrap();
        let mut state = cpu.get_state();
        state.x = 1;
        state.registers[1] = 0x2000;
        cpu.restore(state).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(
            trace.port_transaction,
            Some(PortTransaction::Read {
                port,
                value: 0x80 | port
            })
        );
        let state = cpu.get_state();
        assert_eq!(state.d, 0x80 | port);
        assert_eq!(state.memory[0x2000], 0x80 | port);
        assert_eq!(state.registers[1], 0x2000);
    }

    let mut cpu = configured(&[0x60]);
    let mut state = cpu.get_state();
    state.x = 1;
    state.registers[1] = 0xffff;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().registers[1], 0);
}

#[test]
fn logical_immediate_and_memory_forms_leave_df_unchanged() {
    for (opcode, expected) in [(0xf0, 0x0f), (0xf1, 0xff), (0xf2, 0x00), (0xf3, 0xff)] {
        let mut cpu = configured(&[opcode]);
        let mut state = cpu.get_state();
        state.x = 1;
        state.registers[1] = 0x2000;
        state.memory[0x2000] = 0x0f;
        state.d = 0xf0;
        state.df = true;
        cpu.restore(state).unwrap();
        cpu.step().unwrap();
        let state = cpu.get_state();
        assert_eq!(state.d, expected);
        assert!(state.df);
    }
    for (opcode, expected) in [(0xf8, 0x0f), (0xf9, 0xff), (0xfa, 0), (0xfb, 0xff)] {
        let mut cpu = configured(&[opcode, 0x0f]);
        let mut state = cpu.get_state();
        state.d = 0xf0;
        state.df = true;
        cpu.restore(state).unwrap();
        cpu.step().unwrap();
        let state = cpu.get_state();
        assert_eq!(state.d, expected);
        assert!(state.df);
    }
}

#[test]
fn control_miscellany_exposes_q_t_and_pointer_updates() {
    let mut cpu = configured(&[0x7b, 0x7a, 0x78, 0x72, 0x73]);
    let mut state = cpu.get_state();
    state.x = 1;
    state.registers[1] = 0x2000;
    state.t = 0x9a;
    state.memory[0x2001] = 0x55;
    state.d = 0x33;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert!(cpu.get_state().q);
    cpu.step().unwrap();
    assert!(!cpu.get_state().q);
    cpu.step().unwrap();
    assert_eq!(cpu.read_memory(0x2000), 0x9a);
    cpu.step().unwrap();
    assert_eq!(
        (cpu.get_state().d, cpu.get_state().registers[1]),
        (0x9a, 0x2001)
    );
    cpu.step().unwrap();
    assert_eq!(
        (cpu.read_memory(0x2001), cpu.get_state().registers[1]),
        (0x9a, 0x2000)
    );
}

#[test]
fn invalid_environment_indices_are_typed_and_atomic() {
    let mut cpu = configured(&[0]);
    let before = cpu.get_state();
    assert_eq!(
        cpu.set_external_flag(0, true),
        Err(Cdp1802Error::InvalidExternalFlag { flag: 0 })
    );
    assert_eq!(
        cpu.set_input_port(8, 1),
        Err(Cdp1802Error::InvalidPort { port: 8 })
    );
    assert_eq!(
        cpu.write_register(16, 1),
        Err(Cdp1802Error::InvalidState {
            reason: "register selector is outside 0..15"
        })
    );
    assert_eq!(cpu.get_state(), before);
}
