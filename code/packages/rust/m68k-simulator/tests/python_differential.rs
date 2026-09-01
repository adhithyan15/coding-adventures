use m68k_simulator::{execute::decode_and_execute, M68kSimulator};
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

fn fnv1a(simulator: &M68kSimulator) -> String {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for address in 0..simulator.mem.size() {
        value ^= u64::from(simulator.mem.read_byte(address));
        value = value.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{value:016x}")
}

#[test]
fn python_oracle_vectors_match_complete_state() {
    let fixture = include_str!("python_oracle.jsonl");
    let vectors: Vec<Vector> = fixture
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid generated vector"))
        .collect();
    assert_eq!(vectors.len(), 82);

    for vector in vectors {
        let mut simulator = M68kSimulator::new(65_536);
        simulator.load_program(&vector.program);
        simulator.d = vector.d;
        simulator.a = vector.a;
        simulator.sr = vector.sr;
        for [address, value] in vector.memory {
            simulator.mem.write_byte(address, value as u8);
        }

        let mut error = None;
        for _ in 0..vector.steps {
            if simulator.halted {
                break;
            }
            if let Err(message) = decode_and_execute(&mut simulator) {
                error = Some(message);
                break;
            }
        }

        assert_eq!(
            error.is_some(),
            vector.error.is_some(),
            "{} error",
            vector.name
        );
        assert_eq!(
            simulator.d, vector.expected.d,
            "{} D registers",
            vector.name
        );
        assert_eq!(
            simulator.a, vector.expected.a,
            "{} A registers",
            vector.name
        );
        assert_eq!(simulator.pc, vector.expected.pc, "{} PC", vector.name);
        assert_eq!(simulator.sr, vector.expected.sr, "{} SR", vector.name);
        assert_eq!(
            simulator.halted, vector.expected.halted,
            "{} halt",
            vector.name
        );
        assert_eq!(
            fnv1a(&simulator),
            vector.expected.memory_hash,
            "{} memory",
            vector.name
        );
    }
}
