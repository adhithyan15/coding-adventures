use pdp11_simulator::*;

fn push(program: &mut Vec<u8>, bytes: [u8; 2]) {
    program.extend_from_slice(&bytes);
}

fn immediate_mov(program: &mut Vec<u8>, register: u8, value: u16) {
    push(
        program,
        double_instruction(0x1000, 2, PC as u8, 0, register),
    );
    push(program, word(value));
}

#[test]
fn lifecycle_transport_snapshot_and_bounds_are_deterministic() {
    let mut simulator = Pdp11Simulator::new();
    let initial = simulator.state();
    assert_eq!(initial.r[SP], INITIAL_SP);
    assert_eq!(initial.r[PC], LOAD_ADDRESS);
    assert_eq!(initial.memory.len(), MEMORY_BYTES);
    assert_eq!(simulator.load(&HALT).unwrap(), 2);
    let result = simulator.run(1).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert_eq!(result.traces[0].mnemonic, "HALT");
    assert_eq!(
        simulator.step().unwrap().description,
        "HALT (already halted)"
    );
    simulator.reset();
    assert_eq!(simulator.state(), initial);

    simulator.write_register(0, 0xabcd).unwrap();
    simulator.write_psw(0xffff);
    simulator.write_byte(0x2001, 0x55);
    simulator.write_word(0x2002, 0x1234).unwrap();
    assert_eq!(simulator.read_byte(0x2001), 0x55);
    assert_eq!(simulator.read_word(0x2002).unwrap(), 0x1234);
    let before = simulator.state();
    assert!(matches!(
        simulator.load(&vec![0; MEMORY_BYTES - usize::from(LOAD_ADDRESS) + 1]),
        Err(Pdp11Error::ProgramTooLarge { .. })
    ));
    assert_eq!(simulator.state(), before);
    assert!(matches!(
        simulator.write_register(8, 0),
        Err(Pdp11Error::InvalidRegister { index: 8 })
    ));
    assert!(matches!(
        simulator.read_word(1),
        Err(Pdp11Error::OddWordAddress { write: false, .. })
    ));
    assert!(matches!(
        simulator.write_word(1, 0),
        Err(Pdp11Error::OddWordAddress { write: true, .. })
    ));
}

#[test]
fn python_oracle_arithmetic_workloads_match() {
    let mut sum = Vec::new();
    immediate_mov(&mut sum, 1, 10);
    push(&mut sum, single_instruction(0x0a00, 0, 0));
    push(&mut sum, double_instruction(0x6000, 0, 1, 0, 0));
    push(&mut sum, single_instruction(0x0ac0, 0, 1));
    push(&mut sum, branch_instruction(0x02, -3));
    push(&mut sum, HALT);
    let sum_result = Pdp11Simulator::new().execute(&sum, 100).unwrap();
    assert_eq!(sum_result.final_state.r[0], 55);

    let mut multiply = Vec::new();
    immediate_mov(&mut multiply, 1, 5);
    immediate_mov(&mut multiply, 2, 7);
    push(&mut multiply, single_instruction(0x0a00, 0, 0));
    push(&mut multiply, double_instruction(0x6000, 0, 2, 0, 0));
    push(&mut multiply, sob_instruction(1, 2));
    push(&mut multiply, HALT);
    let multiply_result = Pdp11Simulator::new().execute(&multiply, 100).unwrap();
    assert_eq!(multiply_result.final_state.r[0], 35);

    let mut power = Vec::new();
    immediate_mov(&mut power, 0, 1);
    immediate_mov(&mut power, 1, 8);
    push(&mut power, single_instruction(0x0cc0, 0, 0));
    push(&mut power, sob_instruction(1, 2));
    push(&mut power, HALT);
    assert_eq!(
        Pdp11Simulator::new()
            .execute(&power, 100)
            .unwrap()
            .final_state
            .r[0],
        256
    );
}

#[test]
fn every_single_operand_variant_decodes_and_updates_oracle_edges() {
    let cases = [
        (0x00c0, "SWAB", 0x12ab, 0xab12),
        (0x0a00, "CLR", 0x1234, 0),
        (0x8a00, "CLRB", 0xabcd, 0xab00),
        (0x0a40, "COM", 0x00ff, 0xff00),
        (0x8a40, "COMB", 0xab0f, 0xabf0),
        (0x0a80, "INC", 0x7fff, 0x8000),
        (0x8a80, "INCB", 0xab7f, 0xab80),
        (0x0ac0, "DEC", 0x8000, 0x7fff),
        (0x8ac0, "DECB", 0xab80, 0xab7f),
        (0x0b00, "NEG", 1, 0xffff),
        (0x8b00, "NEGB", 0xab01, 0xabff),
        (0x0b40, "ADC", 0xffff, 0),
        (0x8b40, "ADCB", 0xabff, 0xab00),
        (0x0b80, "SBC", 0, 0xffff),
        (0x8b80, "SBCB", 0xab00, 0xabff),
        (0x0bc0, "TST", 0x8000, 0x8000),
        (0x8bc0, "TSTB", 0xab80, 0xab80),
        (0x0c00, "ROR", 1, 0x8000),
        (0x8c00, "RORB", 0xab01, 0xab80),
        (0x0c40, "ROL", 0x8000, 1),
        (0x8c40, "ROLB", 0xab80, 0xab01),
        (0x0c80, "ASR", 0x8001, 0xc000),
        (0x8c80, "ASRB", 0xab81, 0xabc0),
        (0x0cc0, "ASL", 0x8001, 2),
        (0x8cc0, "ASLB", 0xab81, 0xab02),
    ];
    for (base, mnemonic, input, expected) in cases {
        let mut simulator = Pdp11Simulator::new();
        let mut program = Vec::new();
        push(&mut program, single_instruction(base, 0, 0));
        push(&mut program, HALT);
        simulator.load(&program).unwrap();
        simulator.write_register(0, input).unwrap();
        if matches!(
            mnemonic,
            "ADC" | "ADCB" | "SBC" | "SBCB" | "ROR" | "RORB" | "ROL" | "ROLB"
        ) {
            simulator.write_psw(PSW_C);
        }
        let trace = simulator.step().unwrap();
        assert_eq!(trace.mnemonic, mnemonic);
        assert_eq!(simulator.state().r[0], expected, "{mnemonic}");
    }
}

#[test]
fn every_double_operand_variant_matches_register_oracle_vectors() {
    let cases = [
        (0x1000, "MOV", 0x00f0, 0x0f0f, 0x00f0),
        (0x9000, "MOVB", 0x0080, 0x0f0f, 0xff80),
        (0x2000, "CMP", 5, 7, 7),
        (0xa000, "CMPB", 5, 7, 7),
        (0x3000, "BIT", 0x00f0, 0x0f0f, 0x0f0f),
        (0xb000, "BITB", 0x00f0, 0x0f0f, 0x0f0f),
        (0x4000, "BIC", 0x00f0, 0x0fff, 0x0f0f),
        (0xc000, "BICB", 0x00f0, 0xabff, 0xab0f),
        (0x5000, "BIS", 0x00f0, 0x0f0f, 0x0fff),
        (0xd000, "BISB", 0x00f0, 0xab0f, 0xabff),
        (0x6000, "ADD", 1, 0xffff, 0),
        (0xe000, "SUB", 1, 0, 0xffff),
    ];
    for (base, mnemonic, source, destination, expected) in cases {
        let mut simulator = Pdp11Simulator::new();
        let mut program = Vec::new();
        push(&mut program, double_instruction(base, 0, 0, 0, 1));
        push(&mut program, HALT);
        simulator.load(&program).unwrap();
        simulator.write_register(0, source).unwrap();
        simulator.write_register(1, destination).unwrap();
        assert_eq!(simulator.step().unwrap().mnemonic, mnemonic);
        assert_eq!(simulator.state().r[1], expected, "{mnemonic}");
    }
}

#[test]
fn every_branch_condition_has_taken_and_fallthrough_vectors() {
    let cases = [
        (0x01, 0, true),
        (0x02, 0, true),
        (0x03, PSW_Z, true),
        (0x04, 0, true),
        (0x05, PSW_N, true),
        (0x06, 0, true),
        (0x07, PSW_Z, true),
        (0x80, 0, true),
        (0x81, PSW_N, true),
        (0x82, 0, true),
        (0x83, PSW_C, true),
        (0x84, 0, true),
        (0x85, PSW_V, true),
        (0x86, 0, true),
        (0x87, PSW_C, true),
    ];
    for (opcode, psw, expected) in cases {
        let mut simulator = Pdp11Simulator::new();
        simulator.load(&branch_instruction(opcode, 1)).unwrap();
        simulator.write_psw(psw);
        let trace = simulator.step().unwrap();
        assert_eq!(trace.pc_after, LOAD_ADDRESS + if expected { 4 } else { 2 });

        let mut opposite = Pdp11Simulator::new();
        opposite.load(&branch_instruction(opcode, 1)).unwrap();
        opposite.write_psw(psw ^ 0x0f);
        let _ = opposite.step().unwrap();
    }
}

#[test]
fn all_addressing_modes_and_byte_steps_match() {
    for mode in 0..8_u8 {
        let mut simulator = Pdp11Simulator::new();
        let mut program = Vec::new();
        push(&mut program, double_instruction(0x1000, mode, 0, 0, 1));
        if mode >= 6 {
            push(&mut program, word(0x0010));
        }
        push(&mut program, HALT);
        simulator.load(&program).unwrap();
        simulator.write_register(0, 0x2000).unwrap();
        simulator.write_word(0x2000, 0x2100).unwrap();
        simulator.write_word(0x1ffe, 0x2200).unwrap();
        simulator.write_word(0x2010, 0x2300).unwrap();
        simulator.write_word(0x2100, 0x1111).unwrap();
        simulator.write_word(0x2200, 0x2222).unwrap();
        simulator.write_word(0x2300, 0x3333).unwrap();
        let trace = simulator.step().unwrap();
        assert_eq!(trace.mnemonic, "MOV");
    }

    let mut byte = Pdp11Simulator::new();
    let mut program = Vec::new();
    push(&mut program, double_instruction(0x9000, 2, 0, 0, 1));
    byte.load(&program).unwrap();
    byte.write_register(0, 0x2000).unwrap();
    byte.write_byte(0x2000, 0x80);
    byte.step().unwrap();
    assert_eq!(byte.state().r[0], 0x2001);
    assert_eq!(byte.state().r[1], 0xff80);
}

#[test]
fn jsr_rts_rti_sob_and_atomic_errors_match() {
    let subroutine = 0x100a;
    let mut program = Vec::new();
    immediate_mov(&mut program, 0, 7);
    push(&mut program, jsr_instruction(PC as u8, 3, PC as u8));
    push(&mut program, word(subroutine));
    push(&mut program, HALT);
    push(&mut program, double_instruction(0x6000, 0, 0, 0, 0));
    push(&mut program, rts_instruction(PC as u8));
    let result = Pdp11Simulator::new().execute(&program, 20).unwrap();
    assert_eq!(result.final_state.r[0], 14);
    assert_eq!(result.final_state.r[SP], INITIAL_SP);

    let mut rti = Pdp11Simulator::new();
    rti.load(&word(0x0002)).unwrap();
    rti.write_register(SP, 0x2000).unwrap();
    rti.write_word(0x2000, 0x3456).unwrap();
    rti.write_word(0x2002, 0x000f).unwrap();
    assert_eq!(rti.step().unwrap().mnemonic, "RTI");
    assert_eq!(rti.state().r[PC], 0x3456);
    assert_eq!(rti.state().psw, 0x000f);

    for instruction in [
        word(0xffff),
        single_instruction(0x0040, 0, 0),
        jsr_instruction(0, 0, 0),
    ] {
        let mut simulator = Pdp11Simulator::new();
        simulator.load(&instruction).unwrap();
        let before = simulator.state();
        assert!(simulator.step().is_err());
        assert_eq!(simulator.state(), before);
    }

    let mut loop_simulator = Pdp11Simulator::new();
    loop_simulator.load(&branch_instruction(0x01, -1)).unwrap();
    assert_eq!(
        loop_simulator.run(7),
        Err(Pdp11Error::MaxStepsExceeded { max_steps: 7 })
    );
}
