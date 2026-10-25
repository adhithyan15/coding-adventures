use motorola6800_gatelevel::{
    gates, instruction_width, Motorola6800Error, Motorola6800GateLevel, Motorola6800Simulator,
    CC_C, CC_H, CC_I, CC_N, CC_V, CC_Z, ESTIMATED_GATE_COUNT, FLIP_FLOP_COUNT, MEMORY_SIZE,
};

fn pair(program: &[u8], origin: u16) -> (Motorola6800Simulator, Motorola6800GateLevel) {
    let mut functional = Motorola6800Simulator::new();
    let mut gate = Motorola6800GateLevel::new();
    functional.load(program, origin).unwrap();
    gate.load(program, origin).unwrap();
    assert_eq!(gate.get_state(), functional.get_state());
    (functional, gate)
}

fn step_equal(functional: &mut Motorola6800Simulator, gate: &mut Motorola6800GateLevel) {
    let actual = gate.step();
    let expected = functional.step();
    assert_eq!(actual, expected);
    if let Err(error) = actual {
        panic!("lockstep helper requires a successful step: {error}");
    }
    assert_eq!(gate.get_state(), functional.get_state());
}

#[test]
fn primitive_networks_cover_decode_arithmetic_logic_shift_select_and_branch() {
    for opcode in 0_u8..=u8::MAX {
        let decoded = gates::decode_byte(opcode);
        assert_eq!(
            decoded
                .iter()
                .map(|value| usize::from(*value))
                .sum::<usize>(),
            1
        );
        assert_eq!(decoded[usize::from(opcode)], 1);
    }
    assert_eq!(gates::add_8(0xff, 1, 0), (0, 1, 0, 1));
    assert_eq!(gates::add_8(0x7f, 1, 0), (0x80, 0, 1, 1));
    assert_eq!(gates::sub_8(0, 1, 0), (0xff, 1, 0));
    assert_eq!(gates::add_16(0xffff, 1, 0).0, 0);
    assert_eq!(gates::sub_16(0x8000, 1), (0x7fff, 0, 1));
    assert_eq!(gates::bit_and_8(0xf0, 0x5a), 0x50);
    assert_eq!(gates::bit_or_8(0xf0, 0x5a), 0xfa);
    assert_eq!(gates::bit_xor_8(0xf0, 0x5a), 0xaa);
    assert_eq!(gates::shift_left_8(0x81, 1), 0x03);
    assert_eq!(gates::shift_right_8(0x81, 1), 0xc0);
    assert_eq!(gates::select_8(1, 2, 0), 1);
    assert_eq!(gates::select_16(1, 2, 1), 2);
    assert_eq!(gates::branch_selected(0x27, CC_Z), 1);
    assert_eq!(gates::branch_selected(0x27, 0), 0);
}

#[test]
fn topology_and_complete_transport_are_exact() {
    assert_eq!(FLIP_FLOP_COUNT, 524_361);
    let cpu = Motorola6800GateLevel::new();
    assert_eq!(cpu.flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(cpu.gate_count(), ESTIMATED_GATE_COUNT);
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    assert_eq!(cpu.read_cc(), 0xc0);

    let (mut functional, mut gate) = pair(&[0x01, 0x3e], 0x1000);
    functional.write_a(0x12);
    gate.write_a(0x12);
    functional.write_b(0x34);
    gate.write_b(0x34);
    functional.write_x(0x4567);
    gate.write_x(0x4567);
    functional.write_sp(0x89ab);
    gate.write_sp(0x89ab);
    functional.write_cc(CC_H | CC_I | CC_N | CC_Z | CC_V | CC_C);
    gate.write_cc(CC_H | CC_I | CC_N | CC_Z | CC_V | CC_C);
    functional.write_memory(0x2000, &[1, 2, 3]).unwrap();
    gate.write_memory(0x2000, &[1, 2, 3]).unwrap();
    assert_eq!(gate.get_state(), functional.get_state());

    let snapshot = gate.get_state();
    gate.reset();
    assert_eq!(gate.read_memory(0x2000, 3).unwrap(), vec![0, 0, 0]);
    gate.restore(snapshot.clone()).unwrap();
    assert_eq!(gate.get_state(), snapshot);
    step_equal(&mut functional, &mut gate);
}

#[test]
fn every_first_byte_classification_is_lockstep_and_faults_are_atomic() {
    let mut valid = 0;
    for opcode in 0_u8..=u8::MAX {
        let width = instruction_width(opcode);
        let mut bytes = vec![opcode, 0, 0];
        bytes.truncate(usize::from(width.unwrap_or(1)));
        let (mut functional, mut gate) = pair(&bytes, 0);
        functional.write_a(0x12);
        gate.write_a(0x12);
        functional.write_b(0x34);
        gate.write_b(0x34);
        functional.write_x(0x0200);
        gate.write_x(0x0200);
        functional.write_sp(0x0300);
        gate.write_sp(0x0300);
        functional.write_memory(0x0200, &[0x55, 0xaa]).unwrap();
        gate.write_memory(0x0200, &[0x55, 0xaa]).unwrap();
        functional.write_memory(0xfffa, &[0, 0]).unwrap();
        gate.write_memory(0xfffa, &[0, 0]).unwrap();
        let before = gate.get_state();
        assert_eq!(gate.step(), functional.step(), "opcode {opcode:#04x}");
        assert_eq!(
            gate.get_state(),
            functional.get_state(),
            "opcode {opcode:#04x}"
        );
        if width.is_some() {
            valid += 1;
        } else {
            assert_eq!(gate.get_state(), before, "opcode {opcode:#04x}");
        }
    }
    assert_eq!(valid, 198);
}

#[test]
fn addressing_arithmetic_shift_and_decimal_paths_are_lockstep() {
    let program = [
        0x86, 0x09, 0x8b, 0x09, 0x19, 0x16, 0x48, 0x49, 0x43, 0x4a, 0x4c, 0x84, 0x0f, 0x88, 0x03,
        0x8a, 0x80, 0x97, 0x40, 0xd6, 0x40, 0xce, 0x12, 0x34, 0xff, 0x20, 0x00, 0xa6, 0x10, 0xb7,
        0x20, 0x02, 0x3e,
    ];
    let (mut functional, mut gate) = pair(&program, 0);
    functional.write_memory(0x1244, &[0xa5]).unwrap();
    gate.write_memory(0x1244, &[0xa5]).unwrap();
    loop {
        step_equal(&mut functional, &mut gate);
        if gate.get_state().halted {
            break;
        }
    }
    assert_eq!(gate.read_memory(0x2000, 3).unwrap(), vec![0x12, 0x34, 0xa5]);

    let unary = [
        0x60, 0, 0x63, 0, 0x64, 0, 0x66, 0, 0x67, 0, 0x68, 0, 0x69, 0, 0x6a, 0, 0x6c, 0, 0x6d, 0,
        0x6f, 0, 0x3e,
    ];
    let (mut functional, mut gate) = pair(&unary, 0);
    functional.write_x(0x3000);
    gate.write_x(0x3000);
    functional.write_memory(0x3000, &[0x81]).unwrap();
    gate.write_memory(0x3000, &[0x81]).unwrap();
    while !gate.get_state().halted {
        step_equal(&mut functional, &mut gate);
    }
}

#[test]
fn every_conditional_branch_path_is_lockstep() {
    let cases = [
        (0x22, 0, CC_C),
        (0x23, CC_C, 0),
        (0x24, 0, CC_C),
        (0x25, CC_C, 0),
        (0x26, 0, CC_Z),
        (0x27, CC_Z, 0),
        (0x28, 0, CC_V),
        (0x29, CC_V, 0),
        (0x2a, 0, CC_N),
        (0x2b, CC_N, 0),
        (0x2c, 0, CC_N),
        (0x2d, CC_N, 0),
        (0x2e, 0, CC_Z),
        (0x2f, CC_Z, 0),
    ];
    for (opcode, taken, not_taken) in cases {
        for cc in [taken, not_taken] {
            let (mut functional, mut gate) = pair(&[opcode, 2, 0x01, 0x01, 0x3e], 0);
            functional.write_cc(cc);
            gate.write_cc(cc);
            step_equal(&mut functional, &mut gate);
        }
    }
}

#[test]
fn stack_subroutine_interrupt_and_return_paths_are_lockstep() {
    let (mut functional, mut gate) = pair(&[0xbd, 0, 4, 0x3e, 0x86, 42, 0x39], 0);
    functional.write_sp(0x0200);
    gate.write_sp(0x0200);
    while !gate.get_state().halted {
        step_equal(&mut functional, &mut gate);
    }

    let (mut functional, mut gate) = pair(&[0x3f, 0x3e, 0x01, 0x3b], 0);
    functional.write_sp(0x0300);
    gate.write_sp(0x0300);
    functional.write_a(0x12);
    gate.write_a(0x12);
    functional.write_b(0x34);
    gate.write_b(0x34);
    functional.write_x(0x5678);
    gate.write_x(0x5678);
    functional.write_memory(0xfffa, &[0, 3]).unwrap();
    gate.write_memory(0xfffa, &[0, 3]).unwrap();
    while !gate.get_state().halted {
        step_equal(&mut functional, &mut gate);
    }
}

#[test]
fn typed_faults_and_bounded_runs_are_atomic() {
    for image in [&[0x00][..], &[0x86][..]] {
        let (mut functional, mut gate) = pair(image, 0);
        let before = gate.get_state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.get_state(), before);
    }

    let (mut functional, mut gate) = pair(&[0x3e], 0);
    step_equal(&mut functional, &mut gate);
    let before = gate.get_state();
    assert_eq!(gate.step(), Err(Motorola6800Error::Halted));
    assert_eq!(gate.get_state(), before);

    let (mut functional, mut gate) = pair(&[0x01, 0x01, 0x3e], 0);
    let before = gate.get_state();
    assert_eq!(gate.run(1), functional.run(1));
    assert_eq!(gate.get_state(), before);
}

#[test]
fn execute_and_public_failure_boundaries_match() {
    let mut functional = Motorola6800Simulator::new();
    let mut gate = Motorola6800GateLevel::new();
    assert_eq!(
        gate.execute(&[0x86, 42, 0x3e], 0x200, 8),
        functional.execute(&[0x86, 42, 0x3e], 0x200, 8)
    );
    assert_eq!(gate.get_state(), functional.get_state());

    let before = gate.get_state();
    assert!(gate.write_memory(0xffff, &[1, 2]).is_err());
    assert_eq!(gate.get_state(), before);
    assert!(Motorola6800GateLevel::new().load(&[], 0).is_err());
    assert!(Motorola6800GateLevel::new().load(&[0; 2], 0xffff).is_err());

    let mut invalid = gate.get_state();
    invalid.memory.pop();
    assert!(gate.restore(invalid).is_err());
    assert_eq!(gate.get_state(), before);
}
