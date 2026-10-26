use pdp8_gatelevel::*;

fn bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn pair(program: &[u16], origin: usize) -> (Pdp8Simulator, Pdp8GateLevel) {
    let mut functional = Pdp8Simulator::new();
    let mut gate = Pdp8GateLevel::new();
    assert_eq!(
        gate.load_words(program, origin),
        functional.load_words(program, origin)
    );
    (functional, gate)
}

fn assert_step(functional: &mut Pdp8Simulator, gate: &mut Pdp8GateLevel) {
    assert_eq!(gate.state(), functional.state());
    assert_eq!(gate.step(), functional.step());
    assert_eq!(gate.state(), functional.state());
}

fn sync_memory(
    functional: &mut Pdp8Simulator,
    gate: &mut Pdp8GateLevel,
    address: usize,
    value: u16,
) {
    functional.write_memory(address, value).unwrap();
    gate.write_memory(address, value).unwrap();
}

fn sync_ac(functional: &mut Pdp8Simulator, gate: &mut Pdp8GateLevel, value: u16) {
    functional.set_accumulator(value).unwrap();
    gate.set_accumulator(value).unwrap();
}

fn sync_link(functional: &mut Pdp8Simulator, gate: &mut Pdp8GateLevel, value: bool) {
    functional.set_link(value);
    gate.set_link(value);
}

#[test]
fn topology_lifecycle_and_boundaries_match() {
    let mut gate = Pdp8GateLevel::new();
    assert_eq!(gate.flip_flop_count(), 49_205);
    assert_eq!(gate.gate_count(), 319_230);
    assert_eq!(gate.state(), Pdp8Simulator::new().state());

    assert_eq!(
        gate.load(&[0], 0),
        Err(Pdp8Error::OddProgramLength { bytes: 1 })
    );
    let oversized_transport = vec![0; (MEMORY_WORDS + 1) * 2];
    assert_eq!(
        gate.load(&oversized_transport, 0),
        Err(Pdp8Error::ProgramTooLarge {
            words: MEMORY_WORDS + 1,
            capacity: MEMORY_WORDS,
        })
    );
    assert_eq!(gate.state(), Pdp8Simulator::new().state());
    assert!(gate.load_words(&[0x1000], 0).is_err());
    assert!(gate.write_memory(MEMORY_WORDS, 0).is_err());
    assert!(gate.write_memory(0, 0x1000).is_err());
    assert!(gate.set_accumulator(0x1000).is_err());
    assert!(gate.set_program_counter(0x1000).is_err());
    assert!(gate.set_multiplier_quotient(0x1000).is_err());
    assert!(gate.set_switch_register(0x1000).is_err());

    let mut state = gate.state();
    state.ac = 0o1234;
    state.link = true;
    state.mq = 0o7654;
    state.switch_register = 0o7070;
    gate.restore(&state).unwrap();
    assert_eq!(gate.state(), state);
    gate.reset();
    assert_eq!(gate.state(), Pdp8Simulator::new().state());
}

#[test]
fn all_memory_reference_datapaths_match() {
    let program = [
        encode_memory_reference(Opcode::And, false, false, 0o20),
        encode_memory_reference(Opcode::Tad, false, false, 0o21),
        encode_memory_reference(Opcode::Isz, false, false, 0o22),
        encode_memory_reference(Opcode::Dca, false, false, 0o23),
        encode_memory_reference(Opcode::Jms, false, false, 0o30),
    ];
    let (mut functional, mut gate) = pair(&program, 0);
    sync_ac(&mut functional, &mut gate, 0o7070);
    sync_memory(&mut functional, &mut gate, 0o20, 0o0777);
    sync_memory(&mut functional, &mut gate, 0o21, 0o7710);
    sync_memory(&mut functional, &mut gate, 0o22, WORD_MASK);
    for _ in 0..4 {
        assert_step(&mut functional, &mut gate);
    }
    assert_eq!(gate.state().pc, 0o31);

    let (mut functional, mut gate) = pair(
        &[encode_memory_reference(Opcode::Jmp, false, false, 0o40)],
        0,
    );
    assert_step(&mut functional, &mut gate);
    assert_eq!(gate.state().pc, 0o40);
}

#[test]
fn page_indirect_and_auto_index_datapaths_match() {
    let program = [
        encode_memory_reference(Opcode::Tad, false, true, 0o77),
        encode_memory_reference(Opcode::Tad, true, false, 0o30),
        encode_memory_reference(Opcode::Tad, true, false, 0o10),
        HLT,
    ];
    let (mut functional, mut gate) = pair(&program, 0o200);
    for (address, value) in [
        (0o277, 1),
        (0o30, 0o400),
        (0o400, 2),
        (0o10, 0o500),
        (0o501, 4),
    ] {
        sync_memory(&mut functional, &mut gate, address, value);
    }
    for _ in 0..4 {
        assert_step(&mut functional, &mut gate);
    }
    assert_eq!(gate.state().ac, 7);
}

#[test]
fn group_one_microoperations_match() {
    for (instruction, ac, link) in [
        (NOP, 0o1234, false),
        (0o7200, 0o7777, true),
        (0o7100, 0o1234, true),
        (0o7040, 0o1234, false),
        (0o7020, 0, false),
        (0o7001, WORD_MASK, false),
        (0o7010, 1, false),
        (0o7012, 2, false),
        (0o7004, 0o4000, false),
        (0o7006, 0o2000, false),
        (0o7002, 0o1234, false),
        (0o7351, 0o1234, true),
    ] {
        let (mut functional, mut gate) = pair(&[instruction, HLT], 0);
        sync_ac(&mut functional, &mut gate, ac);
        sync_link(&mut functional, &mut gate, link);
        assert_step(&mut functional, &mut gate);
    }
}

#[test]
fn group_two_predicates_switch_and_halt_match() {
    for (instruction, ac, link, switch) in [
        (0o7500, 0o4000, false, 0),
        (0o7440, 0, false, 0),
        (0o7420, 1, true, 0),
        (0o7410, 1, false, 0),
        (0o7450, 1, false, 0),
        (0o7604, 0o7777, false, 0o1234),
        (HLT, 0, false, 0),
    ] {
        let (mut functional, mut gate) = pair(&[instruction, HLT], 0);
        sync_ac(&mut functional, &mut gate, ac);
        sync_link(&mut functional, &mut gate, link);
        functional.set_switch_register(switch).unwrap();
        gate.set_switch_register(switch).unwrap();
        assert_step(&mut functional, &mut gate);
    }
}

#[test]
fn iot_and_interrupt_wires_match() {
    let program = [
        encode_iot(0, 1),
        NOP,
        NOP,
        encode_iot(3, 5),
        encode_iot(0, 3),
        encode_iot(0, 7),
        HLT,
    ];
    let (mut functional, mut gate) = pair(&program, 0);
    functional.set_interrupt_request(true);
    gate.set_interrupt_request(true);
    assert_step(&mut functional, &mut gate);
    assert_step(&mut functional, &mut gate);
    assert_step(&mut functional, &mut gate);

    functional.set_interrupt_request(false);
    gate.set_interrupt_request(false);
    functional.set_program_counter(3).unwrap();
    gate.set_program_counter(3).unwrap();
    for _ in 0..4 {
        assert_step(&mut functional, &mut gate);
    }
}

#[test]
fn typed_decode_faults_are_atomic_and_identical() {
    for instruction in [0o7014, 0o7401, encode_iot(0, 4)] {
        let (mut functional, mut gate) = pair(&[instruction], 0);
        let before = gate.state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.state(), before);
        assert_eq!(gate.state(), functional.state());
    }
}

#[test]
fn bounded_runs_and_a_countdown_workload_match() {
    let loop_instruction = encode_memory_reference(Opcode::Jmp, false, false, 0);
    let (mut functional, mut gate) = pair(&[loop_instruction], 0);
    assert_eq!(gate.run(3), functional.run(3));

    let program = [
        0o7300, // CLA CLL
        encode_memory_reference(Opcode::Tad, false, false, 0o20),
        encode_memory_reference(Opcode::Isz, false, false, 0o21),
        encode_memory_reference(Opcode::Jmp, false, false, 1),
        HLT,
    ];
    let (mut functional, mut gate) = pair(&program, 0);
    sync_memory(&mut functional, &mut gate, 0o20, 1);
    sync_memory(&mut functional, &mut gate, 0o21, WORD_MASK - 2);
    assert_eq!(gate.run(20), functional.run(20));
    assert_eq!(gate.state(), functional.state());
    assert!(gate.state().halted);

    let mut gate = Pdp8GateLevel::new();
    let mut functional = Pdp8Simulator::new();
    assert_eq!(
        gate.execute(&bytes(&[HLT]), 0o200, 1),
        functional.execute(&bytes(&[HLT]), 0o200, 1)
    );
}
