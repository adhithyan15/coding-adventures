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
    cpu.rf.ax = number(s, "ax", 0x1234);
    cpu.rf.bx = number(s, "bx", 0x0200);
    cpu.rf.cx = number(s, "cx", 3);
    cpu.rf.dx = number(s, "dx", 0x40);
    cpu.rf.si = number(s, "si", 0x10);
    cpu.rf.di = number(s, "di", 0x20);
    cpu.rf.sp = number(s, "sp", 0x8000);
    cpu.rf.bp = number(s, "bp", 0x300);
    cpu.rf.cs = number(s, "cs", 0);
    cpu.rf.ds = number(s, "ds", 0x1000);
    cpu.rf.ss = number(s, "ss", 0x2000);
    cpu.rf.es = number(s, "es", 0x3000);
    cpu.rf.ip = number(s, "ip", 0);
    cpu.rf.flag_cf = boolean(s, "cf", true);
    cpu.rf.flag_pf = boolean(s, "pf", false);
    cpu.rf.flag_af = boolean(s, "af", true);
    cpu.rf.flag_zf = boolean(s, "zf", false);
    cpu.rf.flag_sf = boolean(s, "sf", false);
    cpu.rf.flag_tf = boolean(s, "tf", false);
    cpu.rf.flag_if = boolean(s, "if", true);
    cpu.rf.flag_df = boolean(s, "df", false);
    cpu.rf.flag_of = boolean(s, "of", false);
    for &(address, value) in &vector.memory {
        cpu.write_memory(address, value);
    }
    for &(port, value) in &vector.inputs {
        cpu.input_ports[port] = value;
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
            if cpu.halted {
                break;
            }
            cpu.step();
        }
        let e = vector.expected;
        assert_eq!(
            [
                cpu.rf.ax, cpu.rf.bx, cpu.rf.cx, cpu.rf.dx, cpu.rf.si, cpu.rf.di, cpu.rf.sp,
                cpu.rf.bp, cpu.rf.cs, cpu.rf.ds, cpu.rf.ss, cpu.rf.es, cpu.rf.ip
            ],
            [e.ax, e.bx, e.cx, e.dx, e.si, e.di, e.sp, e.bp, e.cs, e.ds, e.ss, e.es, e.ip],
            "{} registers",
            vector.name
        );
        assert_eq!(cpu.rf.pack_flags(), e.flags, "{} flags", vector.name);
        assert_eq!(cpu.halted, e.halted, "{} halted", vector.name);
        assert_eq!(
            fnv1a(cpu.memory_snapshot().iter().copied()),
            e.memory_hash,
            "{} memory",
            vector.name
        );
        assert_eq!(
            fnv1a(cpu.input_ports),
            e.input_hash,
            "{} inputs",
            vector.name
        );
        assert_eq!(
            fnv1a(cpu.output_ports),
            e.output_hash,
            "{} outputs",
            vector.name
        );
    }
}
