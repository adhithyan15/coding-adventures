#![allow(dead_code)]

use pa_risc11_gatelevel::PaRisc11GateSimulator;
use pa_risc11_simulator::{PaRisc11Simulator, PaRisc11State};

pub fn words(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

pub fn pair(words_: &[u32], origin: u32) -> (PaRisc11GateSimulator, PaRisc11Simulator) {
    let image = words(words_);
    let mut gate = PaRisc11GateSimulator::new();
    let mut oracle = PaRisc11Simulator::new();
    gate.load(&image, origin).unwrap();
    oracle.load(&image, origin).unwrap();
    (gate, oracle)
}

pub fn mutate_pair(
    gate: &mut PaRisc11GateSimulator,
    oracle: &mut PaRisc11Simulator,
    update: impl Fn(&mut PaRisc11State),
) {
    let mut state = gate.snapshot().unwrap();
    update(&mut state);
    gate.restore(&state).unwrap();
    let mut state = oracle.snapshot().unwrap();
    update(&mut state);
    oracle.restore(&state).unwrap();
}

pub fn assert_step_equal(gate: &mut PaRisc11GateSimulator, oracle: &mut PaRisc11Simulator) {
    let gate_result = gate.step();
    let oracle_result = oracle.step();
    assert_eq!(gate_result, oracle_result);
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());
}

pub fn low_sign(value: i32, width: u32) -> u32 {
    let mask = (1_u32 << width) - 1;
    let assembled = (value as u32) & mask;
    ((assembled & ((1 << (width - 1)) - 1)) << 1) | (assembled >> (width - 1))
}

pub fn alu(extension: u32, right: u32, left: u32, target: u32) -> u32 {
    (0x02 << 26) | (right << 21) | (left << 16) | (extension << 6) | target
}

pub fn immediate(primary: u32, base: u32, target: u32, value: i32) -> u32 {
    (primary << 26) | (base << 21) | (target << 16) | low_sign(value, 14)
}

pub fn assemble_21_inverse(value: u32) -> u32 {
    ((value >> 20) & 1)
        | (((value >> 9) & 0x7ff) << 1)
        | ((value & 3) << 12)
        | (((value >> 7) & 3) << 14)
        | (((value >> 2) & 0x1f) << 16)
}

pub fn shd(right: u32, left: u32, count: u32, target: u32) -> u32 {
    (0x34 << 26) | (right << 21) | (left << 16) | (2 << 10) | ((31 - count) << 5) | target
}

fn disp12(displacement: i32) -> (u32, u32) {
    let assembled = ((displacement >> 2) as u32) & 0xfff;
    (
        ((assembled & 0x3ff) << 1) | ((assembled >> 10) & 1),
        assembled >> 11,
    )
}

pub fn conditional(
    primary: u32,
    right: u32,
    left: u32,
    condition: u32,
    displacement: i32,
    nullify: bool,
) -> u32 {
    let (w1, w) = disp12(displacement);
    (primary << 26)
        | (right << 21)
        | (left << 16)
        | (condition << 13)
        | (w1 << 2)
        | (u32::from(nullify) << 1)
        | w
}

fn disp17(displacement: i32) -> (u32, u32, u32) {
    let assembled = ((displacement >> 2) as u32) & 0x1ffff;
    (
        (assembled >> 11) & 0x1f,
        ((assembled & 0x3ff) << 1) | ((assembled >> 10) & 1),
        assembled >> 16,
    )
}

pub fn bl(target: u32, displacement: i32, nullify: bool) -> u32 {
    let (w1, w2, w) = disp17(displacement);
    (0x3a << 26) | (target << 21) | (w1 << 16) | (w2 << 2) | (u32::from(nullify) << 1) | w
}

pub fn blr(target: u32, index: u32, nullify: bool) -> u32 {
    (0x3a << 26) | (target << 21) | (index << 16) | (2 << 13) | (u32::from(nullify) << 1)
}

pub fn bv(base: u32, index: u32, nullify: bool) -> u32 {
    (0x3a << 26) | (base << 21) | (index << 16) | (6 << 13) | (u32::from(nullify) << 1)
}
