#![allow(dead_code)]

use microwatt_powerisa_simulator::encoding::{d_form, xfx_form};
use microwatt_powerisa_simulator::{State, Xer, MEMORY_SIZE};

pub const TARGET_CIA: u64 = 0xa4;
const CONSTANT_BASE: usize = 0x1000;
const DATA_BASE: usize = 0x2000;

#[derive(Debug)]
pub struct Row {
    pub name: &'static str,
    pub raw: u32,
    pub cia_before: u64,
    pub cia_after: u64,
    pub before_gprs: [u64; 32],
    pub after_gprs: [u64; 32],
    pub before_lr: u64,
    pub after_lr: u64,
    pub before_ctr: u64,
    pub after_ctr: u64,
    pub before_cr: u32,
    pub after_cr: u32,
    pub before_xer: u64,
    pub after_xer: u64,
    pub memory_sha256: &'static str,
    pub writes: &'static str,
}

fn parse_hex_u64(value: &str) -> u64 {
    u64::from_str_radix(value, 16).unwrap()
}

fn parse_gprs(value: &str) -> [u64; 32] {
    let values = value.split(',').map(parse_hex_u64).collect::<Vec<_>>();
    values.try_into().expect("exactly 32 GPRs")
}

pub fn rows() -> Vec<Row> {
    include_str!("../rtl/vectors.tsv")
        .lines()
        .skip(1)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 16, "malformed RTL row: {line}");
            Row {
                name: fields[0],
                raw: u32::from_str_radix(fields[1], 16).unwrap(),
                cia_before: parse_hex_u64(fields[2]),
                cia_after: parse_hex_u64(fields[3]),
                before_gprs: parse_gprs(fields[4]),
                after_gprs: parse_gprs(fields[5]),
                before_lr: parse_hex_u64(fields[6]),
                after_lr: parse_hex_u64(fields[7]),
                before_ctr: parse_hex_u64(fields[8]),
                after_ctr: parse_hex_u64(fields[9]),
                before_cr: u32::from_str_radix(fields[10], 16).unwrap(),
                after_cr: u32::from_str_radix(fields[11], 16).unwrap(),
                before_xer: parse_hex_u64(fields[12]),
                after_xer: parse_hex_u64(fields[13]),
                memory_sha256: fields[14],
                writes: fields[15],
            }
        })
        .collect()
}

pub fn initial_state(row: &Row) -> State {
    let mut memory = [0_u8; MEMORY_SIZE];
    let mut words = (0..32)
        .map(|index| d_form(58, index, 0, (CONSTANT_BASE + index as usize * 8) as i32))
        .collect::<Vec<_>>();
    words.extend([
        d_form(58, 30, 0, 0x1100),
        (31 << 26) | (30 << 21) | (0xff << 12) | (144 << 1),
        d_form(58, 30, 0, 0x1108),
        xfx_form(30, 1, 467),
        d_form(58, 30, 0, 0x1110),
        xfx_form(30, 8, 467),
        d_form(58, 30, 0, 0x1118),
        xfx_form(30, 9, 467),
        d_form(58, 30, 0, (CONSTANT_BASE + 30 * 8) as i32),
        row.raw,
        0x0000_0200,
    ]);
    for (index, word) in words.into_iter().enumerate() {
        memory[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    for (index, value) in row.before_gprs.iter().enumerate() {
        memory[CONSTANT_BASE + index * 8..CONSTANT_BASE + index * 8 + 8]
            .copy_from_slice(&value.to_le_bytes());
    }
    for (address, value) in [
        (0x1100, u64::from(row.before_cr)),
        (0x1108, row.before_xer),
        (0x1110, row.before_lr),
        (0x1118, row.before_ctr),
    ] {
        memory[address..address + 8].copy_from_slice(&value.to_le_bytes());
    }
    for index in 0..256 {
        memory[DATA_BASE + index] = (index * 73 + 19) as u8;
    }
    State {
        gpr: row.before_gprs,
        cia: row.cia_before,
        lr: row.before_lr,
        ctr: row.before_ctr,
        cr: row.before_cr,
        xer: Xer::from_u64(row.before_xer),
        memory,
        halted: false,
        program_origin: Some(row.cia_before as u16),
        program_len: 8,
    }
}

pub fn expected_state(row: &Row, mut actual: State) -> State {
    actual.gpr = row.after_gprs;
    actual.lr = row.after_lr;
    actual.ctr = row.after_ctr;
    actual.cr = row.after_cr;
    actual.xer = Xer::from_u64(row.after_xer);
    actual.halted = row.name == "attn";
    actual
}
