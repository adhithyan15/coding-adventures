use ibm_system370_gatelevel::{
    gates, System370Error, System370GateLevel, System370State, FLIP_FLOP_COUNT,
};
use ibm_system370_simulator::System370Simulator;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Route {
    Unknown,
    Inherited(usize),
    Extension(usize),
}

// Independent reviewed literal oracle: one entry for every possible first
// byte. It is intentionally not generated from production decode ranges.
const FIRST_BYTE_TABLE: [Route; 256] = [
    Route::Unknown,      // 00
    Route::Unknown,      // 01
    Route::Unknown,      // 02
    Route::Unknown,      // 03
    Route::Unknown,      // 04
    Route::Inherited(2), // 05
    Route::Inherited(2), // 06
    Route::Inherited(2), // 07
    Route::Unknown,      // 08
    Route::Unknown,      // 09
    Route::Inherited(2), // 0a
    Route::Unknown,      // 0b
    Route::Unknown,      // 0c
    Route::Unknown,      // 0d
    Route::Extension(2), // 0e
    Route::Extension(2), // 0f
    Route::Inherited(2), // 10
    Route::Inherited(2), // 11
    Route::Inherited(2), // 12
    Route::Inherited(2), // 13
    Route::Inherited(2), // 14
    Route::Inherited(2), // 15
    Route::Inherited(2), // 16
    Route::Inherited(2), // 17
    Route::Inherited(2), // 18
    Route::Inherited(2), // 19
    Route::Inherited(2), // 1a
    Route::Inherited(2), // 1b
    Route::Inherited(2), // 1c
    Route::Inherited(2), // 1d
    Route::Inherited(2), // 1e
    Route::Inherited(2), // 1f
    Route::Inherited(2), // 20
    Route::Inherited(2), // 21
    Route::Inherited(2), // 22
    Route::Inherited(2), // 23
    Route::Inherited(2), // 24
    Route::Inherited(2), // 25
    Route::Inherited(2), // 26
    Route::Inherited(2), // 27
    Route::Inherited(2), // 28
    Route::Inherited(2), // 29
    Route::Inherited(2), // 2a
    Route::Inherited(2), // 2b
    Route::Inherited(2), // 2c
    Route::Inherited(2), // 2d
    Route::Inherited(2), // 2e
    Route::Inherited(2), // 2f
    Route::Inherited(2), // 30
    Route::Inherited(2), // 31
    Route::Inherited(2), // 32
    Route::Inherited(2), // 33
    Route::Inherited(2), // 34
    Route::Inherited(2), // 35
    Route::Inherited(2), // 36
    Route::Inherited(2), // 37
    Route::Inherited(2), // 38
    Route::Inherited(2), // 39
    Route::Inherited(2), // 3a
    Route::Inherited(2), // 3b
    Route::Inherited(2), // 3c
    Route::Inherited(2), // 3d
    Route::Inherited(2), // 3e
    Route::Inherited(2), // 3f
    Route::Inherited(4), // 40
    Route::Inherited(4), // 41
    Route::Inherited(4), // 42
    Route::Inherited(4), // 43
    Route::Inherited(4), // 44
    Route::Inherited(4), // 45
    Route::Inherited(4), // 46
    Route::Inherited(4), // 47
    Route::Inherited(4), // 48
    Route::Inherited(4), // 49
    Route::Inherited(4), // 4a
    Route::Inherited(4), // 4b
    Route::Inherited(4), // 4c
    Route::Inherited(4), // 4d
    Route::Inherited(4), // 4e
    Route::Inherited(4), // 4f
    Route::Inherited(4), // 50
    Route::Inherited(4), // 51
    Route::Inherited(4), // 52
    Route::Inherited(4), // 53
    Route::Inherited(4), // 54
    Route::Inherited(4), // 55
    Route::Inherited(4), // 56
    Route::Inherited(4), // 57
    Route::Inherited(4), // 58
    Route::Inherited(4), // 59
    Route::Inherited(4), // 5a
    Route::Inherited(4), // 5b
    Route::Inherited(4), // 5c
    Route::Inherited(4), // 5d
    Route::Inherited(4), // 5e
    Route::Inherited(4), // 5f
    Route::Inherited(4), // 60
    Route::Inherited(4), // 61
    Route::Inherited(4), // 62
    Route::Inherited(4), // 63
    Route::Inherited(4), // 64
    Route::Inherited(4), // 65
    Route::Inherited(4), // 66
    Route::Inherited(4), // 67
    Route::Inherited(4), // 68
    Route::Inherited(4), // 69
    Route::Inherited(4), // 6a
    Route::Inherited(4), // 6b
    Route::Inherited(4), // 6c
    Route::Inherited(4), // 6d
    Route::Inherited(4), // 6e
    Route::Inherited(4), // 6f
    Route::Inherited(4), // 70
    Route::Inherited(4), // 71
    Route::Inherited(4), // 72
    Route::Inherited(4), // 73
    Route::Inherited(4), // 74
    Route::Inherited(4), // 75
    Route::Inherited(4), // 76
    Route::Inherited(4), // 77
    Route::Inherited(4), // 78
    Route::Inherited(4), // 79
    Route::Inherited(4), // 7a
    Route::Inherited(4), // 7b
    Route::Inherited(4), // 7c
    Route::Inherited(4), // 7d
    Route::Inherited(4), // 7e
    Route::Inherited(4), // 7f
    Route::Unknown,      // 80
    Route::Unknown,      // 81
    Route::Unknown,      // 82
    Route::Unknown,      // 83
    Route::Unknown,      // 84
    Route::Unknown,      // 85
    Route::Inherited(4), // 86
    Route::Inherited(4), // 87
    Route::Inherited(4), // 88
    Route::Inherited(4), // 89
    Route::Inherited(4), // 8a
    Route::Inherited(4), // 8b
    Route::Inherited(4), // 8c
    Route::Inherited(4), // 8d
    Route::Inherited(4), // 8e
    Route::Inherited(4), // 8f
    Route::Inherited(4), // 90
    Route::Inherited(4), // 91
    Route::Inherited(4), // 92
    Route::Inherited(4), // 93
    Route::Inherited(4), // 94
    Route::Inherited(4), // 95
    Route::Inherited(4), // 96
    Route::Inherited(4), // 97
    Route::Inherited(4), // 98
    Route::Unknown,      // 99
    Route::Unknown,      // 9a
    Route::Unknown,      // 9b
    Route::Unknown,      // 9c
    Route::Unknown,      // 9d
    Route::Unknown,      // 9e
    Route::Unknown,      // 9f
    Route::Unknown,      // a0
    Route::Unknown,      // a1
    Route::Unknown,      // a2
    Route::Unknown,      // a3
    Route::Unknown,      // a4
    Route::Unknown,      // a5
    Route::Unknown,      // a6
    Route::Unknown,      // a7
    Route::Unknown,      // a8
    Route::Unknown,      // a9
    Route::Unknown,      // aa
    Route::Unknown,      // ab
    Route::Unknown,      // ac
    Route::Unknown,      // ad
    Route::Unknown,      // ae
    Route::Unknown,      // af
    Route::Unknown,      // b0
    Route::Unknown,      // b1
    Route::Unknown,      // b2
    Route::Unknown,      // b3
    Route::Unknown,      // b4
    Route::Unknown,      // b5
    Route::Unknown,      // b6
    Route::Unknown,      // b7
    Route::Unknown,      // b8
    Route::Unknown,      // b9
    Route::Unknown,      // ba
    Route::Unknown,      // bb
    Route::Unknown,      // bc
    Route::Extension(4), // bd
    Route::Extension(4), // be
    Route::Extension(4), // bf
    Route::Unknown,      // c0
    Route::Unknown,      // c1
    Route::Unknown,      // c2
    Route::Unknown,      // c3
    Route::Unknown,      // c4
    Route::Unknown,      // c5
    Route::Unknown,      // c6
    Route::Unknown,      // c7
    Route::Unknown,      // c8
    Route::Unknown,      // c9
    Route::Unknown,      // ca
    Route::Unknown,      // cb
    Route::Unknown,      // cc
    Route::Unknown,      // cd
    Route::Unknown,      // ce
    Route::Unknown,      // cf
    Route::Unknown,      // d0
    Route::Unknown,      // d1
    Route::Inherited(6), // d2
    Route::Unknown,      // d3
    Route::Inherited(6), // d4
    Route::Inherited(6), // d5
    Route::Inherited(6), // d6
    Route::Inherited(6), // d7
    Route::Unknown,      // d8
    Route::Unknown,      // d9
    Route::Unknown,      // da
    Route::Unknown,      // db
    Route::Unknown,      // dc
    Route::Unknown,      // dd
    Route::Unknown,      // de
    Route::Unknown,      // df
    Route::Unknown,      // e0
    Route::Unknown,      // e1
    Route::Unknown,      // e2
    Route::Unknown,      // e3
    Route::Unknown,      // e4
    Route::Unknown,      // e5
    Route::Unknown,      // e6
    Route::Unknown,      // e7
    Route::Unknown,      // e8
    Route::Unknown,      // e9
    Route::Unknown,      // ea
    Route::Unknown,      // eb
    Route::Unknown,      // ec
    Route::Unknown,      // ed
    Route::Unknown,      // ee
    Route::Unknown,      // ef
    Route::Unknown,      // f0
    Route::Unknown,      // f1
    Route::Unknown,      // f2
    Route::Unknown,      // f3
    Route::Unknown,      // f4
    Route::Unknown,      // f5
    Route::Unknown,      // f6
    Route::Unknown,      // f7
    Route::Unknown,      // f8
    Route::Unknown,      // f9
    Route::Unknown,      // fa
    Route::Unknown,      // fb
    Route::Unknown,      // fc
    Route::Unknown,      // fd
    Route::Unknown,      // fe
    Route::Unknown,      // ff
];

impl Route {
    fn width(self) -> Option<usize> {
        match self {
            Self::Unknown => None,
            Self::Inherited(width) | Self::Extension(width) => Some(width),
        }
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

fn rr(opcode: u8, r1: u8, r2: u8) -> [u8; 2] {
    [opcode, (r1 << 4) | r2]
}

fn rx(opcode: u8, r1: u8, address: u16) -> [u8; 4] {
    [opcode, r1 << 4, ((address >> 8) & 15) as u8, address as u8]
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
            match FIRST_BYTE_TABLE[usize::from(opcode)].width() {
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
fn literal_table_routes_and_extension_neighbor_toggles_are_independent() {
    for opcode in 0_u8..=u8::MAX {
        let extension = gates::decode_extension(opcode)
            .into_iter()
            .any(|line| line == 1);
        assert_eq!(
            extension,
            matches!(FIRST_BYTE_TABLE[usize::from(opcode)], Route::Extension(_)),
            "route {opcode:#04x}"
        );
    }
    for extension in [0x0e_u8, 0x0f, 0xbd, 0xbe, 0xbf] {
        assert!(matches!(
            FIRST_BYTE_TABLE[usize::from(extension)],
            Route::Extension(_)
        ));
        for bit in 0..8 {
            let toggled = extension ^ (1 << bit);
            assert_eq!(
                gates::decode_extension(toggled)
                    .into_iter()
                    .any(|line| line == 1),
                matches!(FIRST_BYTE_TABLE[usize::from(toggled)], Route::Extension(_)),
                "extension={extension:#04x} bit={bit}"
            );
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

    let (mut gate, mut oracle) = pair_with(&[0x0e, 0x24]);
    write_gpr_pair(&mut gate, &mut oracle, 2, 0x100);
    write_gpr_pair(&mut gate, &mut oracle, 3, 2);
    write_gpr_pair(
        &mut gate,
        &mut oracle,
        4,
        ibm_system370_gatelevel::MEMORY_BYTES as u32 - 1,
    );
    write_gpr_pair(&mut gate, &mut oracle, 5, (u32::from(b'@') << 24) | 1);
    write_memory_pair(
        &mut gate,
        &mut oracle,
        ibm_system370_gatelevel::MEMORY_BYTES as u32 - 1,
        b"A",
    );
    assert_pair(&mut gate, &mut oracle, "MVCL pads past final source byte");
    assert_eq!(gate.read_memory(0x100, 2).unwrap(), b"A@");
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
fn every_public_typed_fault_family_is_differential_and_atomic() {
    for program in [vec![], vec![0, 0]] {
        let mut gate = System370GateLevel::new();
        let mut oracle = System370Simulator::new();
        let gate_before = gate.get_state();
        let oracle_before = oracle.get_state();
        let origin = if program.is_empty() { 0 } else { 1 };
        assert_eq!(gate.load(&program, origin), oracle.load(&program, origin));
        assert_eq!(gate.get_state(), gate_before);
        assert_eq!(oracle.get_state(), oracle_before);
    }
    let mut gate = System370GateLevel::new();
    let mut oracle = System370Simulator::new();
    let gate_before = gate.get_state();
    let oracle_before = oracle.get_state();
    assert_eq!(
        gate.load(&[0, 0], ibm_system370_gatelevel::MEMORY_BYTES as u32),
        oracle.load(&[0, 0], ibm_system370_gatelevel::MEMORY_BYTES as u32)
    );
    assert_eq!(gate.get_state(), gate_before);
    assert_eq!(oracle.get_state(), oracle_before);

    for program in [
        vec![0, 0],
        vec![0xbf, 0],
        vec![0x0a, 1],
        rx(0x58, 1, 2).to_vec(),
    ] {
        let (mut gate, mut oracle) = pair_with(&program);
        assert_pair(&mut gate, &mut oracle, "transport or alignment fault");
    }

    let (mut gate, mut oracle) = pair_with(&rr(0x13, 0, 2));
    write_gpr_pair(&mut gate, &mut oracle, 2, 0x8000_0000);
    assert_pair(&mut gate, &mut oracle, "fixed-point overflow");

    let (mut gate, mut oracle) = pair_with(&rr(0x1d, 0, 2));
    write_gpr_pair(&mut gate, &mut oracle, 0, 1);
    write_gpr_pair(&mut gate, &mut oracle, 1, 1);
    assert_pair(&mut gate, &mut oracle, "fixed-point divide");

    let (mut gate, mut oracle) = pair_with(&rr(0x2d, 0, 2));
    gate.write_fpr(0, 0x4110_0000_0000_0000).unwrap();
    oracle.write_fpr(0, 0x4110_0000_0000_0000).unwrap();
    assert_pair(&mut gate, &mut oracle, "floating divide");

    let (mut gate, mut oracle) = pair_with(&rr(0x2c, 0, 2));
    gate.write_fpr(0, 0x7f10_0000_0000_0000).unwrap();
    oracle.write_fpr(0, 0x7f10_0000_0000_0000).unwrap();
    gate.write_fpr(2, 0x7f10_0000_0000_0000).unwrap();
    oracle.write_fpr(2, 0x7f10_0000_0000_0000).unwrap();
    assert_pair(&mut gate, &mut oracle, "floating exponent");

    let (mut gate, mut oracle) = pair_with(&[0x0e, 0x12]);
    assert_pair(&mut gate, &mut oracle, "register-pair specification fault");

    let (mut gate, mut oracle) = pair_with(&[0x18, 0x00]);
    gate.set_program_counter(2).unwrap();
    oracle.set_program_counter(2).unwrap();
    assert_pair(&mut gate, &mut oracle, "fetch outside installed image");

    let (mut gate, mut oracle) = pair_with(&[0x0a, 0]);
    assert_pair(&mut gate, &mut oracle, "halt transition");
    assert_pair(&mut gate, &mut oracle, "post-halt fault");

    let mut invalid_gate = System370GateLevel::new();
    let mut invalid_oracle = System370Simulator::new();
    let mut invalid = invalid_gate.get_state();
    invalid.condition_code = 4;
    let gate_before = invalid_gate.get_state();
    let oracle_before = invalid_oracle.get_state();
    assert_eq!(
        invalid_gate.restore(invalid.clone()),
        invalid_oracle.restore(invalid)
    );
    assert_eq!(invalid_gate.get_state(), gate_before);
    assert_eq!(invalid_oracle.get_state(), oracle_before);
    assert_eq!(invalid_gate.read_gpr(16), invalid_oracle.read_gpr(16));
    assert_eq!(invalid_gate.write_fpr(1, 0), invalid_oracle.write_fpr(1, 0));
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
