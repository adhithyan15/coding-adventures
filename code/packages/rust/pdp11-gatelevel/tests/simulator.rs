use pdp11_gatelevel::*;

fn push(program: &mut Vec<u8>, bytes: [u8; 2]) {
    program.extend_from_slice(&bytes);
}

fn sync_register(
    functional: &mut Pdp11Simulator,
    gate: &mut Pdp11GateLevel,
    index: usize,
    value: u16,
) {
    functional.write_register(index, value).unwrap();
    gate.write_register(index, value).unwrap();
}

fn sync_psw(functional: &mut Pdp11Simulator, gate: &mut Pdp11GateLevel, value: u16) {
    functional.write_psw(value);
    gate.write_psw(value);
}

fn assert_step(functional: &mut Pdp11Simulator, gate: &mut Pdp11GateLevel) {
    assert_eq!(gate.state(), functional.state());
    assert_eq!(gate.step(), functional.step());
    assert_eq!(gate.state(), functional.state());
}

#[test]
fn topology_lifecycle_and_boundary_contract_match() {
    let gate = Pdp11GateLevel::new();
    assert_eq!(gate.flip_flop_count(), 524_433);
    assert_eq!(gate.gate_count(), 3_226_598);
    assert_eq!(gate.state(), Pdp11Simulator::new().state());

    let mut functional = Pdp11Simulator::new();
    let mut gate = Pdp11GateLevel::new();
    assert_eq!(gate.load(&HALT), functional.load(&HALT));
    functional.write_byte(0x2001, 0x55);
    gate.write_byte(0x2001, 0x55);
    functional.write_word(0x2002, 0x1234).unwrap();
    gate.write_word(0x2002, 0x1234).unwrap();
    assert_eq!(gate.read_byte(0x2001), functional.read_byte(0x2001));
    assert_eq!(gate.read_word(0x2002), functional.read_word(0x2002));
    assert_step(&mut functional, &mut gate);
    assert_eq!(gate.step(), functional.step());
    gate.reset();
    assert_eq!(gate.state(), Pdp11Simulator::new().state());
}

#[test]
fn all_single_operand_variants_match_complete_state_and_trace() {
    let cases = [
        (0x00c0, 0x12ab, 0),
        (0x0a00, 0x1234, 0),
        (0x8a00, 0xabcd, 0),
        (0x0a40, 0x00ff, 0),
        (0x8a40, 0xab0f, 0),
        (0x0a80, 0x7fff, PSW_C),
        (0x8a80, 0xab7f, PSW_C),
        (0x0ac0, 0x8000, PSW_C),
        (0x8ac0, 0xab80, PSW_C),
        (0x0b00, 1, 0),
        (0x8b00, 0xab01, 0),
        (0x0b40, 0xffff, PSW_C),
        (0x8b40, 0xabff, PSW_C),
        (0x0b80, 0, PSW_C),
        (0x8b80, 0xab00, PSW_C),
        (0x0bc0, 0x8000, 0),
        (0x8bc0, 0xab80, 0),
        (0x0c00, 1, PSW_C),
        (0x8c00, 0xab01, PSW_C),
        (0x0c40, 0x8000, PSW_C),
        (0x8c40, 0xab80, PSW_C),
        (0x0c80, 0x8001, 0),
        (0x8c80, 0xab81, 0),
        (0x0cc0, 0x8001, 0),
        (0x8cc0, 0xab81, 0),
    ];
    let mut functional = Pdp11Simulator::new();
    let mut gate = Pdp11GateLevel::new();
    functional.load(&HALT).unwrap();
    gate.load(&HALT).unwrap();
    for (base, input, psw) in cases {
        functional
            .write_word(
                LOAD_ADDRESS,
                u16::from_le_bytes(single_instruction(base, 0, 0)),
            )
            .unwrap();
        gate.write_word(
            LOAD_ADDRESS,
            u16::from_le_bytes(single_instruction(base, 0, 0)),
        )
        .unwrap();
        sync_register(&mut functional, &mut gate, PC, LOAD_ADDRESS);
        sync_register(&mut functional, &mut gate, 0, input);
        sync_psw(&mut functional, &mut gate, psw);
        assert_step(&mut functional, &mut gate);
    }
}

#[test]
fn all_double_operand_variants_match_complete_state_and_trace() {
    let cases = [
        (0x1000, 0x00f0, 0x0f0f),
        (0x9000, 0x0080, 0x0f0f),
        (0x2000, 5, 7),
        (0xa000, 5, 7),
        (0x3000, 0x00f0, 0x0f0f),
        (0xb000, 0x00f0, 0x0f0f),
        (0x4000, 0x00f0, 0x0fff),
        (0xc000, 0x00f0, 0xabff),
        (0x5000, 0x00f0, 0x0f0f),
        (0xd000, 0x00f0, 0xab0f),
        (0x6000, 1, 0xffff),
        (0xe000, 1, 0),
    ];
    let mut functional = Pdp11Simulator::new();
    let mut gate = Pdp11GateLevel::new();
    functional.load(&HALT).unwrap();
    gate.load(&HALT).unwrap();
    for (base, source, destination) in cases {
        let instruction = double_instruction(base, 0, 0, 0, 1);
        functional
            .write_word(LOAD_ADDRESS, u16::from_le_bytes(instruction))
            .unwrap();
        gate.write_word(LOAD_ADDRESS, u16::from_le_bytes(instruction))
            .unwrap();
        sync_register(&mut functional, &mut gate, PC, LOAD_ADDRESS);
        sync_register(&mut functional, &mut gate, 0, source);
        sync_register(&mut functional, &mut gate, 1, destination);
        sync_psw(&mut functional, &mut gate, PSW_C);
        assert_step(&mut functional, &mut gate);
    }
}

#[test]
fn every_branch_predicate_matches_taken_and_fallthrough() {
    let cases = [
        (0x01, 0),
        (0x02, 0),
        (0x03, PSW_Z),
        (0x04, 0),
        (0x05, PSW_N),
        (0x06, 0),
        (0x07, PSW_Z),
        (0x80, 0),
        (0x81, PSW_N),
        (0x82, 0),
        (0x83, PSW_C),
        (0x84, 0),
        (0x85, PSW_V),
        (0x86, 0),
        (0x87, PSW_C),
    ];
    let mut functional = Pdp11Simulator::new();
    let mut gate = Pdp11GateLevel::new();
    functional.load(&HALT).unwrap();
    gate.load(&HALT).unwrap();
    for (opcode, psw) in cases {
        let instruction = branch_instruction(opcode, -2);
        functional
            .write_word(LOAD_ADDRESS, u16::from_le_bytes(instruction))
            .unwrap();
        gate.write_word(LOAD_ADDRESS, u16::from_le_bytes(instruction))
            .unwrap();
        for flags in [psw, psw ^ 0x0f] {
            sync_register(&mut functional, &mut gate, PC, LOAD_ADDRESS);
            sync_psw(&mut functional, &mut gate, flags);
            assert_step(&mut functional, &mut gate);
        }
    }
}

#[test]
fn all_addressing_modes_and_byte_steps_match() {
    for mode in 0..8_u8 {
        let mut program = Vec::new();
        push(&mut program, double_instruction(0x1000, mode, 0, 0, 1));
        if mode >= 6 {
            push(&mut program, word(0x0010));
        }
        push(&mut program, HALT);
        let mut functional = Pdp11Simulator::new();
        let mut gate = Pdp11GateLevel::new();
        functional.load(&program).unwrap();
        gate.load(&program).unwrap();
        sync_register(&mut functional, &mut gate, 0, 0x2000);
        for (address, value) in [
            (0x2000, 0x2100),
            (0x1ffe, 0x2200),
            (0x2010, 0x2300),
            (0x2100, 0x1111),
            (0x2200, 0x2222),
            (0x2300, 0x3333),
        ] {
            functional.write_word(address, value).unwrap();
            gate.write_word(address, value).unwrap();
        }
        assert_step(&mut functional, &mut gate);
    }

    let mut functional = Pdp11Simulator::new();
    let mut gate = Pdp11GateLevel::new();
    let instruction = double_instruction(0x9000, 2, 0, 0, 1);
    functional.load(&instruction).unwrap();
    gate.load(&instruction).unwrap();
    sync_register(&mut functional, &mut gate, 0, 0x2000);
    functional.write_byte(0x2000, 0x80);
    gate.write_byte(0x2000, 0x80);
    assert_step(&mut functional, &mut gate);
}

#[test]
fn calls_interrupts_sob_and_workloads_match() {
    let mut sum = Vec::new();
    push(&mut sum, double_instruction(0x1000, 2, PC as u8, 0, 1));
    push(&mut sum, word(10));
    push(&mut sum, single_instruction(0x0a00, 0, 0));
    push(&mut sum, double_instruction(0x6000, 0, 1, 0, 0));
    push(&mut sum, single_instruction(0x0ac0, 0, 1));
    push(&mut sum, branch_instruction(0x02, -3));
    push(&mut sum, HALT);
    let functional_result = Pdp11Simulator::new().execute(&sum, 100).unwrap();
    let gate_result = Pdp11GateLevel::new().execute(&sum, 100).unwrap();
    assert_eq!(gate_result, functional_result);
    assert_eq!(gate_result.final_state.r[0], 55);

    let subroutine = 0x100a;
    let mut calls = Vec::new();
    push(&mut calls, double_instruction(0x1000, 2, PC as u8, 0, 0));
    push(&mut calls, word(7));
    push(&mut calls, jsr_instruction(PC as u8, 3, PC as u8));
    push(&mut calls, word(subroutine));
    push(&mut calls, HALT);
    push(&mut calls, double_instruction(0x6000, 0, 0, 0, 0));
    push(&mut calls, rts_instruction(PC as u8));
    assert_eq!(
        Pdp11GateLevel::new().execute(&calls, 20).unwrap(),
        Pdp11Simulator::new().execute(&calls, 20).unwrap()
    );

    let mut functional = Pdp11Simulator::new();
    let mut gate = Pdp11GateLevel::new();
    functional.load(&word(0x0002)).unwrap();
    gate.load(&word(0x0002)).unwrap();
    sync_register(&mut functional, &mut gate, SP, 0x2000);
    for (address, value) in [(0x2000, 0x3456), (0x2002, 0x000f)] {
        functional.write_word(address, value).unwrap();
        gate.write_word(address, value).unwrap();
    }
    assert_step(&mut functional, &mut gate);
}

#[test]
fn all_failure_boundaries_are_atomic_and_bounded() {
    for instruction in [
        word(0xffff),
        single_instruction(0x0040, 0, 0),
        jsr_instruction(0, 0, 0),
    ] {
        let mut functional = Pdp11Simulator::new();
        let mut gate = Pdp11GateLevel::new();
        functional.load(&instruction).unwrap();
        gate.load(&instruction).unwrap();
        let before = gate.state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.state(), before);
    }

    let mut functional = Pdp11Simulator::new();
    let mut gate = Pdp11GateLevel::new();
    let loop_program = branch_instruction(0x01, -1);
    functional.load(&loop_program).unwrap();
    gate.load(&loop_program).unwrap();
    assert_eq!(gate.run(7), functional.run(7));

    let oversized = vec![0; MEMORY_BYTES - usize::from(LOAD_ADDRESS) + 1];
    let before = gate.state();
    assert_eq!(gate.load(&oversized), functional.load(&oversized));
    assert_eq!(gate.state(), before);
}
