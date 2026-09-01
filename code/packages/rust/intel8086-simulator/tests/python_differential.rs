//! Full-state differential against the repository's Python 8086 oracle.

use std::collections::BTreeMap;

use intel8086_simulator::Intel8086Simulator;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Vector {
    name: String,
    program: Vec<u8>,
    setup: BTreeMap<String, serde_json::Value>,
    steps: usize,
    memory: Vec<(usize, u8)>,
    inputs: Vec<(usize, u8)>,
    error: Option<String>,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
struct Expected {
    ax: u16,
    bx: u16,
    cx: u16,
    dx: u16,
    si: u16,
    di: u16,
    sp: u16,
    bp: u16,
    cs: u16,
    ds: u16,
    ss: u16,
    es: u16,
    ip: u16,
    flags: u16,
    halted: bool,
    memory_hash: String,
    input_hash: String,
    output_hash: String,
}

fn number(values: &BTreeMap<String, serde_json::Value>, name: &str, default: u16) -> u16 {
    values
        .get(name)
        .and_then(serde_json::Value::as_u64)
        .map_or(default, |value| value as u16)
}

fn boolean(values: &BTreeMap<String, serde_json::Value>, name: &str, default: bool) -> bool {
    values
        .get(name)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default)
}

fn apply_setup(sim: &mut Intel8086Simulator, vector: &Vector) {
    let setup = &vector.setup;
    sim.ax = number(setup, "ax", 0x1234);
    sim.bx = number(setup, "bx", 0x0200);
    sim.cx = number(setup, "cx", 0x0003);
    sim.dx = number(setup, "dx", 0x0040);
    sim.si = number(setup, "si", 0x0010);
    sim.di = number(setup, "di", 0x0020);
    sim.sp = number(setup, "sp", 0x8000);
    sim.bp = number(setup, "bp", 0x0300);
    sim.cs = number(setup, "cs", 0);
    sim.ds = number(setup, "ds", 0x1000);
    sim.ss = number(setup, "ss", 0x2000);
    sim.es = number(setup, "es", 0x3000);
    sim.ip = number(setup, "ip", 0);
    sim.flag_cf = boolean(setup, "cf", true);
    sim.flag_pf = boolean(setup, "pf", false);
    sim.flag_af = boolean(setup, "af", true);
    sim.flag_zf = boolean(setup, "zf", false);
    sim.flag_sf = boolean(setup, "sf", false);
    sim.flag_tf = boolean(setup, "tf", false);
    sim.flag_if = boolean(setup, "if", true);
    sim.flag_df = boolean(setup, "df", false);
    sim.flag_of = boolean(setup, "of", false);
    for &(address, value) in &vector.memory {
        sim.mem.write_byte(address, value);
    }
    for &(port, value) in &vector.inputs {
        sim.input_ports[port] = value;
    }
}

fn flags(sim: &Intel8086Simulator) -> u16 {
    u16::from(sim.flag_cf)
        | 2
        | (u16::from(sim.flag_pf) << 2)
        | (u16::from(sim.flag_af) << 4)
        | (u16::from(sim.flag_zf) << 6)
        | (u16::from(sim.flag_sf) << 7)
        | (u16::from(sim.flag_tf) << 8)
        | (u16::from(sim.flag_if) << 9)
        | (u16::from(sim.flag_df) << 10)
        | (u16::from(sim.flag_of) << 11)
}

fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> String {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        value ^= u64::from(byte);
        value = value.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{value:016x}")
}

#[test]
fn all_python_oracle_vectors_match_complete_state() {
    let fixture = include_str!("python_oracle.jsonl");
    let vectors: Vec<Vector> = fixture
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| serde_json::from_str(line).expect("valid generated vector"))
        .collect();
    assert_eq!(vectors.len(), 461);

    for vector in vectors {
        assert!(
            vector.error.is_none(),
            "oracle vector {} raised {:?}",
            vector.name,
            vector.error
        );
        let mut sim = Intel8086Simulator::new(1 << 20);
        sim.load_program_checked(&vector.program).unwrap();
        apply_setup(&mut sim, &vector);
        for _ in 0..vector.steps {
            if sim.halted {
                break;
            }
            sim.step();
        }

        let expected = vector.expected;
        let actual_registers = [
            sim.ax, sim.bx, sim.cx, sim.dx, sim.si, sim.di, sim.sp, sim.bp, sim.cs, sim.ds, sim.ss,
            sim.es, sim.ip,
        ];
        let expected_registers = [
            expected.ax,
            expected.bx,
            expected.cx,
            expected.dx,
            expected.si,
            expected.di,
            expected.sp,
            expected.bp,
            expected.cs,
            expected.ds,
            expected.ss,
            expected.es,
            expected.ip,
        ];
        assert_eq!(
            actual_registers, expected_registers,
            "{} registers",
            vector.name
        );
        assert_eq!(flags(&sim), expected.flags, "{} flags", vector.name);
        assert_eq!(sim.halted, expected.halted, "{} halted", vector.name);
        assert_eq!(
            fnv1a((0..(1 << 20)).map(|address| sim.mem.read_byte(address))),
            expected.memory_hash,
            "{} memory",
            vector.name
        );
        assert_eq!(
            fnv1a(sim.input_ports),
            expected.input_hash,
            "{} input ports",
            vector.name
        );
        assert_eq!(
            fnv1a(sim.output_ports),
            expected.output_hash,
            "{} output ports",
            vector.name
        );
    }
}
