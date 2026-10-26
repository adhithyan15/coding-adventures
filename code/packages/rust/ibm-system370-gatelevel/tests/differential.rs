use ibm_system370_gatelevel::{
    System370Error, System370GateLevel, System370State, FLIP_FLOP_COUNT,
};
use ibm_system370_simulator::System370Simulator;

fn expected_width(opcode: u8) -> Option<usize> {
    match opcode {
        0x05..=0x07 | 0x0a | 0x0e..=0x3f => Some(2),
        0x40..=0x7f | 0x86..=0x98 | 0xbd..=0xbf => Some(4),
        0xd2 | 0xd4..=0xd7 => Some(6),
        _ => None,
    }
}

fn assert_pair(gate: &mut System370GateLevel, oracle: &mut System370Simulator, label: &str) {
    let gate_before = gate.get_state();
    let oracle_before = oracle.get_state();
    assert_eq!(gate_before, oracle_before, "pre-state {label}");
    let gate_result = gate.step();
    let oracle_result = oracle.step();
    assert_eq!(gate_result, oracle_result, "result {label}");
    assert_eq!(gate.get_state(), oracle.get_state(), "post-state {label}");
    if gate_result.is_err() {
        assert_eq!(gate.get_state(), gate_before, "gate atomicity {label}");
        assert_eq!(
            oracle.get_state(),
            oracle_before,
            "oracle atomicity {label}"
        );
    }
}

fn pair_with(program: &[u8]) -> (System370GateLevel, System370Simulator) {
    let mut gate = System370GateLevel::new();
    let mut oracle = System370Simulator::new();
    gate.load(program, 0).unwrap();
    oracle.load(program, 0).unwrap();
    (gate, oracle)
}

fn write_gpr_pair(
    gate: &mut System370GateLevel,
    oracle: &mut System370Simulator,
    register: usize,
    value: u32,
) {
    gate.write_gpr(register, value).unwrap();
    oracle.write_gpr(register, value).unwrap();
}

fn write_memory_pair(
    gate: &mut System370GateLevel,
    oracle: &mut System370Simulator,
    address: u32,
    bytes: &[u8],
) {
    gate.write_memory(address, bytes).unwrap();
    oracle.write_memory(address, bytes).unwrap();
}

fn rs(opcode: u8, register: u8, mask: u8, address: u16) -> [u8; 4] {
    [
        opcode,
        (register << 4) | mask,
        ((address >> 8) & 15) as u8,
        address as u8,
    ]
}

#[test]
fn topology_and_reset_state_are_exact() {
    let cpu = System370GateLevel::new();
    assert_eq!(FLIP_FLOP_COUNT, 8_389_403);
    assert_eq!(cpu.flip_flop_count(), 8_389_403);
    assert!(cpu.gate_count() > cpu.flip_flop_count() * 6);
    assert_eq!(cpu.get_state(), System370Simulator::new().get_state());
}

#[test]
fn literal_256_by_installed_width_transport_matrix_is_exact_and_atomic() {
    for opcode in 0_u8..=u8::MAX {
        for installed in 1..=6 {
            let mut program = [0_u8; 6];
            program[0] = opcode;
            let (mut gate, mut oracle) = pair_with(&program[..installed]);
            let before = gate.get_state();
            let gate_result = gate.step();
            let oracle_result = oracle.step();
            assert_eq!(
                gate_result, oracle_result,
                "opcode={opcode:#04x} bytes={installed}"
            );
            assert_eq!(gate.get_state(), oracle.get_state());
            match expected_width(opcode) {
                None => assert_eq!(
                    gate_result,
                    Err(System370Error::UnknownOpcode { opcode, pc: 0 })
                ),
                Some(width) if installed < width => assert_eq!(
                    gate_result,
                    Err(System370Error::TruncatedInstruction {
                        pc: 0,
                        needed: width
                    })
                ),
                Some(_) => {}
            }
            if gate_result.is_err() {
                assert_eq!(gate.get_state(), before);
            }
        }
    }
}

#[test]
fn every_inherited_first_byte_and_nontrivial_state_match_the_gate_engine() {
    for opcode in 0_u8..=u8::MAX {
        if matches!(opcode, 0x0e | 0x0f | 0xbd..=0xbf) {
            continue;
        }
        let program = [opcode, 0, 0, 0, 0, 0];
        let (mut gate, mut oracle) = pair_with(&program);
        write_gpr_pair(&mut gate, &mut oracle, 2, 0x1234_5678);
        gate.write_fpr(6, 0x1234_5678_9abc_def0).unwrap();
        oracle.write_fpr(6, 0x1234_5678_9abc_def0).unwrap();
        assert_pair(&mut gate, &mut oracle, &format!("opcode {opcode:#04x}"));
    }
}

#[test]
fn mvcl_length_padding_overlap_and_boundaries_match() {
    for (destination, destination_len, source, source_len, pad) in [
        (0x100_u32, 5_u32, 0x200_u32, 3_u32, 0x40_u8),
        (0x100, 2, 0x200, 3, 0xa5),
        (0x100, 2, 0x200, 2, 0),
        (0x201, 3, 0x200, 3, 0x7e),
        (0x200, 3, 0x201, 3, 0),
        (0x0f_fffe, 3, 0x100, 3, 0),
    ] {
        let (mut gate, mut oracle) = pair_with(&[0x0e, 0x24]);
        write_gpr_pair(&mut gate, &mut oracle, 2, 0xaa00_0000 | destination);
        write_gpr_pair(&mut gate, &mut oracle, 3, 0xc500_0000 | destination_len);
        write_gpr_pair(&mut gate, &mut oracle, 4, 0xbb00_0000 | source);
        write_gpr_pair(
            &mut gate,
            &mut oracle,
            5,
            (u32::from(pad) << 24) | source_len,
        );
        if source <= 0x0f_fffc {
            write_memory_pair(&mut gate, &mut oracle, source, b"ABCD");
        }
        assert_pair(
            &mut gate,
            &mut oracle,
            &format!("MVCL d={destination:#x}/{destination_len} s={source:#x}/{source_len}"),
        );
    }
}

#[test]
fn clcl_relations_padding_and_pair_faults_match() {
    for (first, second, first_len, second_len, pad) in [
        (b"ABC".as_slice(), b"ABC".as_slice(), 3, 3, b'@'),
        (b"AZZ".as_slice(), b"ABZ".as_slice(), 3, 3, b'@'),
        (b"AA".as_slice(), b"A".as_slice(), 2, 1, b'@'),
        (b"A".as_slice(), b"A@".as_slice(), 1, 2, b'@'),
    ] {
        let (mut gate, mut oracle) = pair_with(&[0x0f, 0x24]);
        write_gpr_pair(&mut gate, &mut oracle, 2, 0x100);
        write_gpr_pair(&mut gate, &mut oracle, 3, first_len);
        write_gpr_pair(&mut gate, &mut oracle, 4, 0x200);
        write_gpr_pair(
            &mut gate,
            &mut oracle,
            5,
            (u32::from(pad) << 24) | second_len,
        );
        write_memory_pair(&mut gate, &mut oracle, 0x100, first);
        write_memory_pair(&mut gate, &mut oracle, 0x200, second);
        assert_pair(&mut gate, &mut oracle, "CLCL relation");
    }
    for raw in [[0x0e, 0x12], [0x0f, 0x21]] {
        let (mut gate, mut oracle) = pair_with(&raw);
        assert_pair(&mut gate, &mut oracle, "odd long pair");
    }
}

#[test]
fn all_masks_and_character_condition_codes_match() {
    for opcode in [0xbd, 0xbe, 0xbf] {
        for mask in 0..16 {
            for bytes in [[0, 0, 0, 0], [0x80, 0, 1, 0], [0x11, 0x22, 0x33, 0x44]] {
                let instruction = rs(opcode, 2, mask, 0x100);
                let (mut gate, mut oracle) = pair_with(&instruction);
                write_gpr_pair(&mut gate, &mut oracle, 2, 0x1122_3344);
                write_memory_pair(&mut gate, &mut oracle, 0x100, &bytes);
                let mut state = gate.get_state();
                state.condition_code = 3;
                gate.restore(state.clone()).unwrap();
                oracle.restore(state).unwrap();
                assert_pair(
                    &mut gate,
                    &mut oracle,
                    &format!("opcode={opcode:#04x} mask={mask:#x}"),
                );
            }
        }
    }
}

#[test]
fn lifecycle_restore_helpers_and_faults_preserve_complete_state() {
    let mut gate = System370GateLevel::new();
    gate.load(&[0x18, 0x12], 0x20).unwrap();
    gate.write_gpr(1, 7).unwrap();
    gate.write_fpr(4, 0x1122_3344_5566_7788).unwrap();
    gate.write_memory(0x100, &[1, 2, 3]).unwrap();
    let complete = gate.get_state();
    gate.reset();
    gate.restore(complete.clone()).unwrap();
    assert_eq!(gate.get_state(), complete);

    let mut invalid = complete.clone();
    invalid.memory.pop();
    assert!(matches!(
        gate.restore(invalid),
        Err(System370Error::InvalidState { .. })
    ));
    assert_eq!(gate.get_state(), complete);
    assert!(gate.write_gpr(16, 0).is_err());
    assert!(gate
        .write_memory(ibm_system370_gatelevel::MEMORY_BYTES as u32, &[1])
        .is_err());
    assert_eq!(gate.get_state(), complete);
}

#[test]
fn run_execute_are_transactional() {
    let mut gate = System370GateLevel::new();
    gate.load(&[0x18, 0x12, 0x0a, 0], 0).unwrap();
    gate.write_gpr(2, 0xfeed_face).unwrap();
    let result = gate.run(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.final_state, gate.get_state());

    let mut bounded = System370GateLevel::new();
    bounded.load(&[0x18, 0x12, 0x18, 0x12], 0).unwrap();
    let before = bounded.get_state();
    assert_eq!(
        bounded.run(1),
        Err(System370Error::MaxStepsExceeded { max_steps: 1 })
    );
    assert_eq!(bounded.get_state(), before);

    let mut execute = System370GateLevel::new();
    execute.write_gpr(1, 9).unwrap();
    let result = execute.execute(&[0x0a, 0], 0x20, 1).unwrap();
    assert!(result.halted);

    let mut fault = System370GateLevel::new();
    fault.write_gpr(1, 9).unwrap();
    let before = fault.get_state();
    assert_eq!(fault.execute(&[], 0, 1), Err(System370Error::EmptyProgram));
    assert_eq!(fault.get_state(), before);
}

#[allow(dead_code)]
fn _public_state_type_is_system370(_: System370State) {}
