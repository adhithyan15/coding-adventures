use pdp8_simulator::*;

fn bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn run_one(instruction: u16, ac: u16, link: bool) -> Pdp8State {
    let mut cpu = Pdp8Simulator::new();
    cpu.load_words(&[instruction, HLT], 0).unwrap();
    cpu.set_accumulator(ac).unwrap();
    cpu.set_link(link);
    cpu.step().unwrap();
    cpu.state()
}

#[test]
fn construction_load_transport_and_reset_are_strict() {
    let mut cpu = Pdp8Simulator::new();
    assert_eq!(cpu, Pdp8Simulator::default());
    assert_eq!(cpu.state().memory.as_ref(), &[0; MEMORY_WORDS]);

    assert_eq!(cpu.load(&bytes(&[0o1234, HLT]), 0o200).unwrap(), 2);
    assert_eq!(cpu.state().pc, 0o200);
    assert_eq!(cpu.state().loaded_origin, 0o200);
    assert_eq!(cpu.read_memory(0o200).unwrap(), 0o1234);

    let before = cpu.state();
    assert_eq!(
        cpu.load(&[0], 0),
        Err(Pdp8Error::OddProgramLength { bytes: 1 })
    );
    assert_eq!(cpu.state(), before);
    assert!(matches!(
        cpu.load_words(&[0x1000], 0),
        Err(Pdp8Error::InvalidWord { .. })
    ));
    assert_eq!(cpu.state(), before);
    assert!(matches!(
        cpu.load_words(&[0, 0], MEMORY_WORDS - 1),
        Err(Pdp8Error::ProgramTooLarge { .. })
    ));
    assert_eq!(cpu.state(), before);

    cpu.reset();
    assert_eq!(cpu, Pdp8Simulator::new());
}

#[test]
fn direct_memory_register_and_restore_boundaries_fail_closed() {
    let mut cpu = Pdp8Simulator::new();
    assert_eq!(
        cpu.read_memory(MEMORY_WORDS),
        Err(Pdp8Error::InvalidAddress {
            address: MEMORY_WORDS
        })
    );
    assert!(cpu.write_memory(MEMORY_WORDS, 0).is_err());
    assert!(cpu.write_memory(0, 0x1000).is_err());
    assert!(cpu.set_accumulator(0x1000).is_err());
    assert!(cpu.set_program_counter(0x1000).is_err());
    assert!(cpu.set_multiplier_quotient(0x1000).is_err());
    assert!(cpu.set_switch_register(0x1000).is_err());

    let mut invalid = cpu.state();
    invalid.memory[7] = 0x1000;
    assert!(cpu.restore(&invalid).is_err());
    assert_eq!(cpu, Pdp8Simulator::new());

    let mut valid = cpu.state();
    valid.ac = 0o1234;
    valid.pc = 0o7654;
    valid.mq = 0o4321;
    valid.switch_register = 0o7070;
    valid.link = true;
    cpu.restore(&valid).unwrap();
    assert_eq!(cpu.state(), valid);
}

#[test]
fn all_memory_reference_operations_execute() {
    let mut cpu = Pdp8Simulator::new();
    let program = [
        encode_memory_reference(Opcode::And, false, false, 0o20),
        encode_memory_reference(Opcode::Tad, false, false, 0o21),
        encode_memory_reference(Opcode::Isz, false, false, 0o22),
        encode_memory_reference(Opcode::Dca, false, false, 0o23),
        encode_memory_reference(Opcode::Jms, false, false, 0o30),
    ];
    cpu.load_words(&program, 0).unwrap();
    cpu.set_accumulator(0o7070).unwrap();
    cpu.write_memory(0o20, 0o0777).unwrap();
    cpu.write_memory(0o21, 1).unwrap();
    cpu.write_memory(0o22, WORD_MASK).unwrap();

    assert_eq!(cpu.step().unwrap().mnemonic, "AND 0020");
    assert_eq!(cpu.state().ac, 0o0070);
    cpu.step().unwrap();
    assert_eq!(cpu.state().ac, 0o0071);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 4, "ISZ skipped word three");
    cpu.step().unwrap();
    assert_eq!(cpu.read_memory(0o30).unwrap(), 5);
    assert_eq!(cpu.state().pc, 0o31);
}

#[test]
fn dca_clears_ac_and_jmp_transfers_control() {
    let mut cpu = Pdp8Simulator::new();
    cpu.load_words(
        &[
            encode_memory_reference(Opcode::Dca, false, false, 0o20),
            encode_memory_reference(Opcode::Jmp, false, false, 0o10),
        ],
        0,
    )
    .unwrap();
    cpu.set_accumulator(0o4567).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().ac, 0);
    assert_eq!(cpu.read_memory(0o20).unwrap(), 0o4567);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 0o10);
}

#[test]
fn tad_toggles_link_on_each_carry() {
    let mut cpu = Pdp8Simulator::new();
    let tad = encode_memory_reference(Opcode::Tad, false, false, 0o20);
    cpu.load_words(&[tad, tad], 0).unwrap();
    cpu.set_accumulator(WORD_MASK).unwrap();
    cpu.write_memory(0o20, 1).unwrap();
    cpu.step().unwrap();
    assert_eq!((cpu.state().ac, cpu.state().link), (0, true));
    cpu.set_accumulator(WORD_MASK).unwrap();
    cpu.step().unwrap();
    assert_eq!((cpu.state().ac, cpu.state().link), (0, false));
}

#[test]
fn current_page_indirect_and_auto_index_addressing_execute() {
    let mut cpu = Pdp8Simulator::new();
    cpu.load_words(
        &[
            encode_memory_reference(Opcode::Tad, false, true, 0o77),
            encode_memory_reference(Opcode::Tad, true, false, 0o30),
            encode_memory_reference(Opcode::Tad, true, false, 0o10),
            HLT,
        ],
        0o200,
    )
    .unwrap();
    cpu.write_memory(0o277, 1).unwrap();
    cpu.write_memory(0o30, 0o400).unwrap();
    cpu.write_memory(0o400, 2).unwrap();
    cpu.write_memory(0o10, 0o500).unwrap();
    cpu.write_memory(0o501, 4).unwrap();

    assert_eq!(cpu.step().unwrap().effective_address, Some(0o277));
    assert_eq!(cpu.step().unwrap().effective_address, Some(0o400));
    assert_eq!(cpu.step().unwrap().effective_address, Some(0o501));
    assert_eq!(cpu.state().ac, 7);
    assert_eq!(cpu.read_memory(0o10).unwrap(), 0o501);
}

#[test]
fn group_one_honors_micro_order_and_rotations() {
    let state = run_one(0o7351, 0o1234, true); // CLA CLL CMA CML IAC RAR
    assert_eq!((state.link, state.ac), (false, 0o4000));

    assert_eq!(run_one(0o7004, 0o4000, false).ac, 0);
    let ral = run_one(0o7004, 0o4000, false);
    assert!(ral.link);
    assert_eq!(run_one(0o7006, 0o2000, false).ac, 0);
    assert_eq!(run_one(0o7010, 1, false).ac, 0);
    assert!(run_one(0o7010, 1, false).link);
    assert_eq!(run_one(0o7012, 2, false).ac, 0);
    assert_eq!(run_one(0o7002, 0o1234, false).ac, 0o3412);
}

#[test]
fn invalid_group_one_and_group_three_are_atomic() {
    for instruction in [0o7014, 0o7401] {
        let mut cpu = Pdp8Simulator::new();
        cpu.load_words(&[instruction], 0).unwrap();
        cpu.set_accumulator(0o1234).unwrap();
        let before = cpu.state();
        assert!(cpu.step().is_err());
        assert_eq!(cpu.state(), before);
    }
}

#[test]
fn group_two_skip_reverse_switch_and_halt_execute() {
    let mut cpu = Pdp8Simulator::new();
    cpu.load_words(&[0o7540, NOP, 0o7604, HLT], 0).unwrap(); // SMA SZA; CLA OSR
    cpu.set_accumulator(0).unwrap();
    cpu.set_switch_register(0o1234).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 2);
    cpu.step().unwrap();
    assert_eq!(cpu.state().ac, 0o1234);
    cpu.step().unwrap();
    assert!(cpu.state().halted);
    assert_eq!(cpu.step(), Err(Pdp8Error::Halted));

    let skp = run_one(0o7410, 1, false);
    assert_eq!(skp.pc, 2, "RSS with no conditions is unconditional SKP");
    let not_zero = run_one(0o7450, 1, false); // SZA RSS
    assert_eq!(not_zero.pc, 2);
}

#[test]
fn cpu_iots_external_events_and_interrupt_entry_execute() {
    let mut cpu = Pdp8Simulator::new();
    cpu.load_words(&[encode_iot(0, 1), NOP, NOP, encode_iot(0o03, 0o5), HLT], 0)
        .unwrap();
    cpu.set_interrupt_request(true);
    assert_eq!(cpu.step().unwrap().mnemonic, "ION");
    assert_eq!(cpu.state().interrupt_delay, 1);
    cpu.step().unwrap();
    assert_eq!(cpu.state().interrupt_delay, 0);
    let interrupt = cpu.step().unwrap();
    assert_eq!(interrupt.instruction, None);
    assert_eq!(cpu.read_memory(0).unwrap(), 2);
    assert_eq!(cpu.state().pc, 1);

    cpu.set_interrupt_request(false);
    cpu.set_program_counter(3).unwrap();
    let external = cpu.step().unwrap();
    assert_eq!(
        external.iot,
        Some(IotEvent {
            device: 3,
            pulses: 5
        })
    );
}

#[test]
fn cpu_skip_disable_request_and_clear_iots_execute() {
    let mut cpu = Pdp8Simulator::new();
    cpu.load_words(
        &[
            encode_iot(0, 1),
            encode_iot(0, 0),
            NOP,
            encode_iot(0, 3),
            NOP,
            encode_iot(0, 7),
            HLT,
        ],
        0,
    )
    .unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 3, "SKON skipped and disabled interrupts");
    cpu.set_interrupt_request(true);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 5, "SRQ skipped on a pending request");
    cpu.step().unwrap();
    assert!(!cpu.state().interrupt_pending);
}

#[test]
fn unsupported_cpu_iot_rolls_back_fetch() {
    let mut cpu = Pdp8Simulator::new();
    cpu.load_words(&[encode_iot(0, 4)], 0).unwrap();
    let before = cpu.state();
    assert_eq!(
        cpu.step(),
        Err(Pdp8Error::UnsupportedCpuIot {
            instruction: encode_iot(0, 4)
        })
    );
    assert_eq!(cpu.state(), before);
}

#[test]
fn bounded_run_execute_and_trace_contracts_hold() {
    let mut cpu = Pdp8Simulator::new();
    let loop_instruction = encode_memory_reference(Opcode::Jmp, false, false, 0);
    cpu.load_words(&[loop_instruction], 0).unwrap();
    assert_eq!(
        cpu.run(4),
        Err(Pdp8Error::MaxStepsExceeded { max_steps: 4 })
    );

    let result = cpu.execute(&bytes(&[HLT]), 0o200, 1).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert_eq!(result.traces[0].instruction, Some(HLT));
    assert_eq!(result.traces[0].pc_before, 0o200);
}

#[test]
fn error_messages_identify_the_failed_boundary() {
    assert!(Pdp8Error::InvalidOrigin { origin: 4096 }
        .to_string()
        .contains("10000"));
    assert!(Pdp8Error::UnsupportedEae {
        instruction: 0o7401
    }
    .to_string()
    .contains("KE8-E"));
    assert_eq!(Pdp8Error::Halted.to_string(), "the PDP-8 is halted");
}
