use std::collections::BTreeMap;

use coding_adventures_intel8086_gatelevel::Cpu8086;
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
        .map_or(default, |v| v as u16)
}

fn boolean(values: &BTreeMap<String, serde_json::Value>, name: &str, default: bool) -> u8 {
    u8::from(
        values
            .get(name)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default),
    )
}

fn apply_setup(cpu: &mut Cpu8086, vector: &Vector) {
    let s = &vector.setup;
    cpu.rf.write16(0, number(s, "ax", 0x1234));
    cpu.rf.write16(3, number(s, "bx", 0x0200));
    cpu.rf.write16(1, number(s, "cx", 3));
    cpu.rf.write16(2, number(s, "dx", 0x40));
    cpu.rf.write16(6, number(s, "si", 0x10));
    cpu.rf.write16(7, number(s, "di", 0x20));
    cpu.rf.write16(4, number(s, "sp", 0x8000));
    cpu.rf.write16(5, number(s, "bp", 0x300));
    cpu.rf.write_seg(1, number(s, "cs", 0));
    cpu.rf.write_seg(3, number(s, "ds", 0x1000));
    cpu.rf.write_seg(2, number(s, "ss", 0x2000));
    cpu.rf.write_seg(0, number(s, "es", 0x3000));
    cpu.rf.set_ip(number(s, "ip", 0));
    let flags = u16::from(boolean(s, "cf", true))
        | (u16::from(boolean(s, "pf", false)) << 2)
        | (u16::from(boolean(s, "af", true)) << 4)
        | (u16::from(boolean(s, "zf", false)) << 6)
        | (u16::from(boolean(s, "sf", false)) << 7)
        | (u16::from(boolean(s, "tf", false)) << 8)
        | (u16::from(boolean(s, "if", true)) << 9)
        | (u16::from(boolean(s, "df", false)) << 10)
        | (u16::from(boolean(s, "of", false)) << 11);
    cpu.rf.unpack_flags(flags);
    for &(address, value) in &vector.memory {
        cpu.write_memory(address, value);
    }
    for &(port, value) in &vector.inputs {
        cpu.set_input_port(port, value).unwrap();
    }
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
fn functional_oracle_vectors_match_complete_state() {
    let fixture = include_str!("../../intel8086-simulator/tests/python_oracle.jsonl");
    let vectors: Vec<Vector> = fixture
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(vectors.len(), 461);
    for vector in vectors {
        assert!(vector.error.is_none());
        let mut cpu = Cpu8086::new();
        cpu.load(&vector.program, 0);
        apply_setup(&mut cpu, &vector);
        for _ in 0..vector.steps {
            if cpu.halted() {
                break;
            }
            cpu.step();
        }
        let e = vector.expected;
        let state = cpu.get_state();
        assert_eq!(
            [
                state.ax, state.bx, state.cx, state.dx, state.si, state.di, state.sp, state.bp,
                state.cs, state.ds, state.ss, state.es, state.ip
            ],
            [e.ax, e.bx, e.cx, e.dx, e.si, e.di, e.sp, e.bp, e.cs, e.ds, e.ss, e.es, e.ip],
            "{} registers",
            vector.name
        );
        assert_eq!(state.flags, e.flags, "{} flags", vector.name);
        assert_eq!(state.halted, e.halted, "{} halted", vector.name);
        assert_eq!(
            fnv1a(cpu.memory_snapshot().iter().copied()),
            e.memory_hash,
            "{} memory",
            vector.name
        );
        assert_eq!(
            fnv1a(state.input_ports),
            e.input_hash,
            "{} inputs",
            vector.name
        );
        assert_eq!(
            fnv1a(state.output_ports),
            e.output_hash,
            "{} outputs",
            vector.name
        );
    }
}
