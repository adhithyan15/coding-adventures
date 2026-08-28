use coding_adventures_motorola68k_gatelevel::{Cpu68K, M68kError};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vector {
    name: String,
    program: Vec<u8>,
    d: [u32; 8],
    a: [u32; 8],
    sr: u16,
    memory: Vec<[usize; 2]>,
    steps: usize,
    error: Option<String>,
    expected: Expected,
}

#[derive(Deserialize)]
struct Expected {
    d: [u32; 8],
    a: [u32; 8],
    pc: u32,
    sr: u16,
    halted: bool,
    memory_hash: String,
}

fn fnv1a(bytes: &[u8]) -> String {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{value:016x}")
}

#[test]
fn complete_functional_oracle_corpus_matches_gate_transitions() {
    let fixture = include_str!("../../m68k-simulator/tests/python_oracle.jsonl");
    let vectors: Vec<Vector> = fixture
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid generated vector"))
        .collect();
    assert_eq!(vectors.len(), 82);

    for vector in vectors {
        let mut cpu = Cpu68K::new();
        cpu.mem[..vector.program.len()].copy_from_slice(&vector.program);
        cpu.rf.d = vector.d;
        cpu.rf.a = vector.a;
        cpu.rf.pc = 0;
        cpu.rf.sr = vector.sr;
        for [address, value] in vector.memory {
            cpu.mem[address] = value as u8;
        }

        if vector.error.is_some() {
            let initial = cpu.get_state();
            let error = cpu.step_checked().expect_err("oracle error is typed");
            assert!(
                matches!(error, M68kError::Execution(_)),
                "{} expected a typed execution error, got {error:?}",
                vector.name
            );
            assert_eq!(cpu.get_state(), initial, "{} atomic error", vector.name);
            continue;
        }
        for _ in 0..vector.steps {
            if cpu.halted {
                break;
            }
            cpu.step();
        }
        let state = cpu.get_state();
        assert_eq!(state.d, vector.expected.d, "{} D registers", vector.name);
        assert_eq!(state.a, vector.expected.a, "{} A registers", vector.name);
        assert_eq!(state.pc, vector.expected.pc, "{} PC", vector.name);
        assert_eq!(state.sr, vector.expected.sr, "{} SR", vector.name);
        assert_eq!(state.halted, vector.expected.halted, "{} halt", vector.name);
        assert_eq!(
            fnv1a(&state.memory[..65_536]),
            vector.expected.memory_hash,
            "{} memory",
            vector.name
        );
    }
}
