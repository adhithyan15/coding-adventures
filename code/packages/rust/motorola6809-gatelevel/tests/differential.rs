use motorola6809_gatelevel::{
    gates, indexed_postbyte_is_valid, opcode_is_documented, Motorola6809Error,
    Motorola6809GateLevel, Motorola6809Simulator, CC_C, CC_E, CC_F, CC_H, CC_I, CC_N, CC_V, CC_Z,
    ESTIMATED_GATE_COUNT, FLIP_FLOP_COUNT, MAX_RETAINED_STEPS, MEMORY_SIZE,
};

fn pair(program: &[u8], origin: u16) -> (Motorola6809Simulator, Motorola6809GateLevel) {
    let mut functional = Motorola6809Simulator::new();
    let mut gate = Motorola6809GateLevel::new();
    functional.load(program, origin).unwrap();
    gate.load(program, origin).unwrap();
    assert_eq!(gate.get_state(), functional.get_state());
    (functional, gate)
}

fn step_equal(functional: &mut Motorola6809Simulator, gate: &mut Motorola6809GateLevel) {
    let actual = gate.step();
    let expected = functional.step();
    assert_eq!(actual, expected);
    if let Err(error) = actual {
        panic!("lockstep helper requires a successful step: {error}");
    }
    assert_eq!(gate.get_state(), functional.get_state());
}

fn initialize(functional: &mut Motorola6809Simulator, gate: &mut Motorola6809GateLevel) {
    functional.write_a(0x12);
    gate.write_a(0x12);
    functional.write_b(0x34);
    gate.write_b(0x34);
    functional.write_x(0x2000);
    gate.write_x(0x2000);
    functional.write_y(0x2100);
    gate.write_y(0x2100);
    functional.write_u(0x3000);
    gate.write_u(0x3000);
    functional.write_s(0x4000);
    gate.write_s(0x4000);
    functional.write_dp(0x40);
    gate.write_dp(0x40);
    functional.write_memory(0xfff2, &[0, 0]).unwrap();
    gate.write_memory(0xfff2, &[0, 0]).unwrap();
    functional.write_memory(0xfff4, &[0, 0]).unwrap();
    gate.write_memory(0xfff4, &[0, 0]).unwrap();
    functional.write_memory(0xfffa, &[0, 0]).unwrap();
    gate.write_memory(0xfffa, &[0, 0]).unwrap();
}

#[test]
fn primitive_networks_cover_decode_arithmetic_logic_shift_branch_and_multiply() {
    for value in 0_u8..=u8::MAX {
        let decoded = gates::decode_byte(value);
        assert_eq!(decoded.iter().copied().map(usize::from).sum::<usize>(), 1);
        assert_eq!(decoded[usize::from(value)], 1);
    }
    assert_eq!(gates::add_8(0xff, 1, 0), (0, 1, 0, 1));
    assert_eq!(gates::add_8(0x7f, 1, 0), (0x80, 0, 1, 1));
    assert_eq!(gates::sub_8(0, 1, 0), (0xff, 1, 0));
    assert_eq!(gates::add_16(0xffff, 1, 0).0, 0);
    assert_eq!(gates::sub_16(0x8000, 1), (0x7fff, 0, 1));
    assert_eq!(gates::multiply_8(0x12, 0x34), 0x03a8);
    assert_eq!(gates::bit_and_8(0xf0, 0x5a), 0x50);
    assert_eq!(gates::bit_or_8(0xf0, 0x5a), 0xfa);
    assert_eq!(gates::bit_xor_8(0xf0, 0x5a), 0xaa);
    assert_eq!(gates::shift_left_8(0x81, 1), 0x03);
    assert_eq!(gates::shift_right_8(0x81, 1), 0xc0);
    assert_eq!(gates::select_16(0x1234, 0xabcd, 0), 0x1234);
    assert_eq!(gates::select_16(0x1234, 0xabcd, 1), 0xabcd);
    assert_eq!(gates::branch_selected(0x27, CC_Z), 1);
    assert_eq!(gates::branch_selected(0x27, 0), 0);
}

#[test]
fn topology_and_complete_transport_are_exact() {
    assert_eq!(FLIP_FLOP_COUNT, 524_434);
    let cpu = Motorola6809GateLevel::new();
    assert_eq!(cpu, Motorola6809GateLevel::default());
    assert_eq!(cpu.flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(cpu.gate_count(), ESTIMATED_GATE_COUNT);
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    assert_eq!(cpu.read_cc(), CC_I | CC_F);

    let (mut functional, mut gate) = pair(&[0x12, 0x13], 0x1000);
    initialize(&mut functional, &mut gate);
    functional.write_d(0x1234);
    gate.write_d(0x1234);
    assert_eq!(gate.read_d(), 0x1234);
    functional.write_cc(CC_E | CC_F | CC_H | CC_I | CC_N | CC_Z | CC_V | CC_C);
    gate.write_cc(CC_E | CC_F | CC_H | CC_I | CC_N | CC_Z | CC_V | CC_C);
    functional.write_memory(0x5000, &[1, 2, 3]).unwrap();
    gate.write_memory(0x5000, &[1, 2, 3]).unwrap();
    assert_eq!(gate.get_state(), functional.get_state());

    let snapshot = gate.get_state();
    gate.reset();
    assert_eq!(gate.read_memory(0x5000, 3).unwrap(), vec![0, 0, 0]);
    gate.restore(snapshot.clone()).unwrap();
    assert_eq!(gate.get_state(), snapshot);
    step_equal(&mut functional, &mut gate);
}

#[test]
fn every_opcode_page_classification_is_lockstep_and_atomic() {
    let mut documented = [0_usize; 3];
    for (page_index, page) in [0_u8, 0x10, 0x11].into_iter().enumerate() {
        for opcode in 0_u8..=u8::MAX {
            if page == 0 && matches!(opcode, 0x10 | 0x11) {
                continue;
            }
            let program = if page == 0 {
                vec![opcode, 0, 0, 0, 0, 0, 0, 0x13]
            } else {
                vec![page, opcode, 0, 0, 0, 0, 0, 0x13]
            };
            let (mut functional, mut gate) = pair(&program, 0);
            initialize(&mut functional, &mut gate);
            let before = gate.get_state();
            let expected = functional.step();
            let actual = gate.step();
            assert_eq!(actual, expected, "page {page:#04x} opcode {opcode:#04x}");
            assert_eq!(gate.get_state(), functional.get_state());
            if opcode_is_documented(page, opcode) {
                documented[page_index] += 1;
                assert!(
                    actual.is_ok(),
                    "documented page {page:#04x} opcode {opcode:#04x}"
                );
            } else {
                assert!(actual.is_err());
                assert_eq!(gate.get_state(), before);
            }
        }
    }
    assert_eq!(documented, [221, 38, 9]);
}

#[test]
fn every_indexed_postbyte_classification_is_lockstep() {
    let mut valid = 0;
    for postbyte in 0_u8..=u8::MAX {
        let (mut functional, mut gate) = pair(&[0xa6, postbyte, 0, 0, 0x13], 0);
        initialize(&mut functional, &mut gate);
        functional
            .write_memory(0x2000, &[0x5a, 0x20, 0x00])
            .unwrap();
        gate.write_memory(0x2000, &[0x5a, 0x20, 0x00]).unwrap();
        let before = gate.get_state();
        let expected = functional.step();
        let actual = gate.step();
        assert_eq!(actual, expected, "postbyte {postbyte:#04x}");
        assert_eq!(gate.get_state(), functional.get_state());
        if indexed_postbyte_is_valid(postbyte) {
            valid += 1;
            assert!(actual.is_ok());
        } else {
            assert!(matches!(
                actual,
                Err(Motorola6809Error::InvalidIndexedPostbyte { .. })
            ));
            assert_eq!(gate.get_state(), before);
        }
    }
    assert_eq!(valid, 217);
}

#[test]
fn arithmetic_memory_stack_control_and_interrupt_paths_are_lockstep() {
    let program = [
        0x86, 0x12, 0xc6, 0x34, 0x1f, 0x01, 0x34, 0x17, 0x4f, 0x5f, 0x35, 0x17, 0x8b, 1, 0xcb, 2,
        0x3d, 0x97, 0x20, 0xd7, 0x21, 0x13,
    ];
    let (mut functional, mut gate) = pair(&program, 0);
    functional.write_s(0x3000);
    gate.write_s(0x3000);
    functional.write_dp(0x40);
    gate.write_dp(0x40);
    while !gate.get_state().halted {
        step_equal(&mut functional, &mut gate);
    }
    assert_eq!(gate.read_memory(0x4020, 2).unwrap(), vec![0x04, 0x02]);

    for (instruction, vector) in [
        (&[0x3f][..], 0xfffa),
        (&[0x10, 0x3f], 0xfff4),
        (&[0x11, 0x3f], 0xfff2),
    ] {
        let mut program = instruction.to_vec();
        program.extend_from_slice(&[0x13, 0x3b]);
        let handler = (instruction.len() + 1) as u16;
        let (mut functional, mut gate) = pair(&program, 0);
        initialize(&mut functional, &mut gate);
        functional
            .write_memory(vector, &handler.to_be_bytes())
            .unwrap();
        gate.write_memory(vector, &handler.to_be_bytes()).unwrap();
        for _ in 0..2 {
            step_equal(&mut functional, &mut gate);
        }
    }
}

#[test]
fn short_and_long_branches_cover_both_outcomes() {
    for code in 0_u8..16 {
        let opcode = 0x20 | code;
        for cc in [0, CC_C, CC_Z, CC_V, CC_N, CC_N | CC_V, CC_Z | CC_C] {
            let (mut functional, mut gate) = pair(&[opcode, 2, 0x12, 0x13, 0x13], 0);
            functional.write_cc(cc);
            gate.write_cc(cc);
            step_equal(&mut functional, &mut gate);
            if code != 0 {
                let (mut functional, mut gate) = pair(&[0x10, opcode, 0, 2, 0x12, 0x13, 0x13], 0);
                functional.write_cc(cc);
                gate.write_cc(cc);
                step_equal(&mut functional, &mut gate);
            }
        }
    }
}

#[test]
fn transfer_exchange_and_both_complete_stack_masks_are_lockstep() {
    for code in 0_u8..=5 {
        let (mut functional, mut gate) = pair(&[0x1e, code, 0x13], 0);
        initialize(&mut functional, &mut gate);
        step_equal(&mut functional, &mut gate);
    }
    for code in 8_u8..=0x0b {
        let postbyte = 0x80 | code;
        let (mut functional, mut gate) = pair(&[0x1e, postbyte, 0x13], 0);
        initialize(&mut functional, &mut gate);
        step_equal(&mut functional, &mut gate);
    }

    let (mut functional, mut gate) = pair(&[0x34, 0xff, 0x35, 0xff, 0x13], 0);
    initialize(&mut functional, &mut gate);
    step_equal(&mut functional, &mut gate);
    step_equal(&mut functional, &mut gate);

    let (mut functional, mut gate) = pair(&[0x36, 0xff, 0x37, 0xff, 0x13], 0);
    initialize(&mut functional, &mut gate);
    gate.write_u(0x5000);
    functional.write_u(0x5000);
    step_equal(&mut functional, &mut gate);
    step_equal(&mut functional, &mut gate);
}

#[test]
fn typed_faults_bounded_runs_and_execute_are_atomic() {
    for bytes in [
        &[0x10][..],
        &[0x10, 0x12],
        &[0xa6, 0x87],
        &[0x1f, 0x06],
        &[0x86],
    ] {
        let (mut functional, mut gate) = pair(bytes, 0);
        let before = gate.get_state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.get_state(), before);
    }

    let (mut functional, mut gate) = pair(&[0x13], 0);
    step_equal(&mut functional, &mut gate);
    let before = gate.get_state();
    assert_eq!(gate.step(), Err(Motorola6809Error::Halted));
    assert_eq!(gate.get_state(), before);

    let (mut functional, mut gate) = pair(&[0x01], 0);
    let before = gate.get_state();
    assert_eq!(gate.run(4), functional.run(4));
    assert_eq!(gate.get_state(), before);

    let (mut functional, mut gate) = pair(&[0x12], 0);
    step_equal(&mut functional, &mut gate);
    let before = gate.get_state();
    assert_eq!(gate.step(), functional.step());
    assert_eq!(gate.get_state(), before);

    let (mut functional, mut gate) = pair(&[0x12, 0x13], 0);
    let before = gate.get_state();
    assert_eq!(gate.run(1), functional.run(1));
    assert_eq!(gate.get_state(), before);
    assert_eq!(
        gate.run(MAX_RETAINED_STEPS + 1),
        Err(Motorola6809Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS,
        })
    );

    let mut functional = Motorola6809Simulator::new();
    let mut gate = Motorola6809GateLevel::new();
    assert_eq!(
        gate.execute(&[0x86, 42, 0x13], 0x200, 8),
        functional.execute(&[0x86, 42, 0x13], 0x200, 8)
    );
    assert_eq!(gate.get_state(), functional.get_state());

    let before = gate.get_state();
    assert_eq!(
        gate.execute(&[0x01], 0, 8),
        functional.execute(&[0x01], 0, 8)
    );
    assert_eq!(gate.get_state(), before);
    assert_eq!(
        gate.execute(&[0x13], 0, MAX_RETAINED_STEPS + 1),
        Err(Motorola6809Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS,
        })
    );

    let before = gate.get_state();
    assert!(gate.write_memory(0xffff, &[1, 2]).is_err());
    assert!(gate.read_memory(1, usize::MAX).is_err());
    assert_eq!(gate.get_state(), before);
    assert!(Motorola6809GateLevel::new().load(&[], 0).is_err());
    assert!(Motorola6809GateLevel::new().load(&[0; 2], 0xffff).is_err());
    let mut invalid = gate.get_state();
    invalid.memory.pop();
    assert!(gate.restore(invalid).is_err());
    assert_eq!(gate.get_state(), before);
}
