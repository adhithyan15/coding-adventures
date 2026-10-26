//! Compiled gate-backend helpers for the Armv7E-M extension.

use logic_gates::{combinational::mux2, gates};

pub trait GateBackend {
    fn and(&mut self, left: u8, right: u8) -> u8;
    fn or(&mut self, left: u8, right: u8) -> u8;
    fn xor(&mut self, left: u8, right: u8) -> u8;
    fn not(&mut self, value: u8) -> u8;
    fn mux(&mut self, d0: u8, d1: u8, select: u8) -> u8;
}

#[derive(Default)]
pub struct RepositoryGates;

impl GateBackend for RepositoryGates {
    fn and(&mut self, left: u8, right: u8) -> u8 {
        gates::and_gate(left, right)
    }
    fn or(&mut self, left: u8, right: u8) -> u8 {
        gates::or_gate(left, right)
    }
    fn xor(&mut self, left: u8, right: u8) -> u8 {
        gates::xor_gate(left, right)
    }
    fn not(&mut self, value: u8) -> u8 {
        gates::not_gate(value)
    }
    fn mux(&mut self, d0: u8, d1: u8, select: u8) -> u8 {
        mux2(d0, d1, select)
    }
}

fn bits16(value: u16) -> [u8; 16] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits32(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn from_bits32(bits: [u8; 32]) -> u32 {
    bits.into_iter()
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (u32::from(wire) << bit))
}

fn equal_masked<B: GateBackend>(backend: &mut B, value: u16, mask: u16, expected: u16) -> u8 {
    let value = bits16(value);
    let mask = bits16(mask);
    let expected = bits16(expected);
    let mut selected = 1;
    for bit in 0..16 {
        let different = backend.xor(value[bit], expected[bit]);
        let same = backend.not(different);
        let relevant = backend.mux(1, same, mask[bit]);
        selected = backend.and(selected, relevant);
    }
    selected
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DspClass {
    Qadd,
    Qsub,
    Sadd8,
    Uadd8,
}

pub fn dsp_prefix<B: GateBackend>(backend: &mut B, first: u16) -> u8 {
    equal_masked(backend, first, 0xfff0, 0xfa80)
}

pub fn classify<B: GateBackend>(backend: &mut B, first: u16, second: u16) -> Option<DspClass> {
    let prefix = dsp_prefix(backend, first);
    let qadd_line = equal_masked(backend, second, 0xf0f0, 0xf080);
    let qsub_line = equal_masked(backend, second, 0xf0f0, 0xf0a0);
    let sadd8_line = equal_masked(backend, second, 0xf0f0, 0xf000);
    let uadd8_line = equal_masked(backend, second, 0xf0f0, 0xf040);
    let qadd = backend.and(prefix, qadd_line);
    let qsub = backend.and(prefix, qsub_line);
    let sadd8 = backend.and(prefix, sadd8_line);
    let uadd8 = backend.and(prefix, uadd8_line);
    if qadd == 1 {
        Some(DspClass::Qadd)
    } else if qsub == 1 {
        Some(DspClass::Qsub)
    } else if sadd8 == 1 {
        Some(DspClass::Sadd8)
    } else if uadd8 == 1 {
        Some(DspClass::Uadd8)
    } else {
        None
    }
}

fn valid_low_register<B: GateBackend>(backend: &mut B, register: u8) -> u8 {
    let high = (register >> 3) & 1;
    let above_twelve = backend.or((register >> 1) & 1, register & 1);
    let top_pair = backend.and(high, (register >> 2) & 1);
    let reserved = backend.and(top_pair, above_twelve);
    backend.not(reserved)
}

pub fn fields<B: GateBackend>(
    backend: &mut B,
    first: u16,
    second: u16,
    class: DspClass,
) -> ([usize; 3], u8) {
    let rd = ((second >> 8) & 0xf) as u8;
    let (rn, rm) = match class {
        DspClass::Qadd | DspClass::Qsub => (second as u8 & 0xf, first as u8 & 0xf),
        DspClass::Sadd8 | DspClass::Uadd8 => (first as u8 & 0xf, second as u8 & 0xf),
    };
    let rd_ok = valid_low_register(backend, rd);
    let rn_ok = valid_low_register(backend, rn);
    let rm_ok = valid_low_register(backend, rm);
    let sources_ok = backend.and(rn_ok, rm_ok);
    let operands_ok = backend.and(rd_ok, sources_ok);
    ([rd as usize, rn as usize, rm as usize], operands_ok)
}

fn full_adder<B: GateBackend>(backend: &mut B, a: u8, b: u8, carry: u8) -> (u8, u8) {
    let pair = backend.xor(a, b);
    let sum = backend.xor(pair, carry);
    let generated = backend.and(a, b);
    let propagated = backend.and(pair, carry);
    let carry = backend.or(generated, propagated);
    (sum, carry)
}

fn add_width<B: GateBackend, const WIDTH: usize>(
    backend: &mut B,
    left: [u8; WIDTH],
    right: [u8; WIDTH],
    carry_in: u8,
) -> ([u8; WIDTH], u8, u8) {
    let mut carry = carry_in;
    let mut into_sign = carry_in;
    let mut sum = [0; WIDTH];
    for bit in 0..WIDTH {
        if bit + 1 == WIDTH {
            into_sign = carry;
        }
        (sum[bit], carry) = full_adder(backend, left[bit], right[bit], carry);
    }
    let overflow = backend.xor(into_sign, carry);
    (sum, carry, overflow)
}

pub fn saturating<B: GateBackend>(
    backend: &mut B,
    left: u32,
    right: u32,
    subtract: u8,
    old_q: u8,
) -> (u32, u8, u8) {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    let (selected, _carry, q, overflow) =
        saturating_width(backend, left_bits, right_bits, subtract, old_q);
    (from_bits32(selected), q, overflow)
}

fn saturating_width<B: GateBackend, const WIDTH: usize>(
    backend: &mut B,
    left_bits: [u8; WIDTH],
    right_bits: [u8; WIDTH],
    subtract: u8,
    old_q: u8,
) -> ([u8; WIDTH], u8, u8, u8) {
    let adjusted = std::array::from_fn(|bit| backend.xor(right_bits[bit], subtract));
    let (sum, carry, overflow) = add_width(backend, left_bits, adjusted, subtract);
    let limit: [u8; WIDTH] = std::array::from_fn(|bit| {
        if bit + 1 == WIDTH {
            left_bits[WIDTH - 1]
        } else {
            backend.not(left_bits[WIDTH - 1])
        }
    });
    let selected = std::array::from_fn(|bit| backend.mux(sum[bit], limit[bit], overflow));
    (selected, carry, backend.or(old_q, overflow), overflow)
}

/// Reduced-width exhaustive proof surface for the production ripple/saturation
/// algorithm. The 32-bit path above instantiates the identical generic helper.
#[doc(hidden)]
pub fn reduced_saturating_8<B: GateBackend>(
    backend: &mut B,
    left: u8,
    right: u8,
    subtract: u8,
    old_q: u8,
) -> (u8, u8, u8, u8) {
    let left_bits: [u8; 8] = std::array::from_fn(|bit| (left >> bit) & 1);
    let right_bits: [u8; 8] = std::array::from_fn(|bit| (right >> bit) & 1);
    let (result, carry, q, overflow) =
        saturating_width(backend, left_bits, right_bits, subtract, old_q);
    let result = result
        .into_iter()
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (wire << bit));
    (result, carry, overflow, q)
}

pub fn parallel_add<B: GateBackend>(
    backend: &mut B,
    left: u32,
    right: u32,
    signed: u8,
) -> (u32, u8) {
    let mut result = 0_u32;
    let mut ge = 0_u8;
    for lane in 0..4 {
        let shift = lane * 8;
        let a = ((left >> shift) & 0xff) as u8;
        let b = ((right >> shift) & 0xff) as u8;
        let a_bits: [u8; 9] = std::array::from_fn(|bit| {
            if bit < 8 {
                (a >> bit) & 1
            } else {
                (a >> 7) & 1
            }
        });
        let b_bits: [u8; 9] = std::array::from_fn(|bit| {
            if bit < 8 {
                (b >> bit) & 1
            } else {
                (b >> 7) & 1
            }
        });
        let (signed_sum, _, _) = add_width(backend, a_bits, b_bits, 0);
        let unsigned_a: [u8; 8] = std::array::from_fn(|bit| (a >> bit) & 1);
        let unsigned_b: [u8; 8] = std::array::from_fn(|bit| (b >> bit) & 1);
        let (unsigned_sum, carry, _) = add_width(backend, unsigned_a, unsigned_b, 0);
        let lane_result: [u8; 8] =
            std::array::from_fn(|bit| backend.mux(unsigned_sum[bit], signed_sum[bit], signed));
        let mut byte = 0_u8;
        for (bit, wire) in lane_result.into_iter().enumerate() {
            byte |= wire << bit;
        }
        result |= u32::from(byte) << shift;
        let signed_ge = backend.not(signed_sum[8]);
        let ge_wire = backend.mux(carry, signed_ge, signed);
        ge |= ge_wire << lane;
    }
    (result, ge)
}

pub fn condition_holds<B: GateBackend>(backend: &mut B, apsr: u32, condition: u8) -> u8 {
    let n = ((apsr >> 31) & 1) as u8;
    let z = ((apsr >> 30) & 1) as u8;
    let c = ((apsr >> 29) & 1) as u8;
    let v = ((apsr >> 28) & 1) as u8;
    let nv_different = backend.xor(n, v);
    let nv_equal = backend.not(nv_different);
    let nz = backend.not(z);
    let nc = backend.not(c);
    let nn = backend.not(n);
    let nv = backend.not(v);
    let nv_different = backend.not(nv_equal);
    let hi = backend.and(c, nz);
    let ls = backend.or(nc, z);
    let gt = backend.and(nz, nv_equal);
    let le = backend.or(z, nv_different);
    let mut lines = [
        z,
        nz,
        c,
        nc,
        n,
        nn,
        v,
        nv,
        hi,
        ls,
        nv_equal,
        nv_different,
        gt,
        le,
        1,
        0,
    ];
    for stage in 0..4 {
        let select = (condition >> stage) & 1;
        let stride = 1 << stage;
        for base in (0..16).step_by(stride * 2) {
            for offset in 0..stride {
                lines[base + offset] =
                    backend.mux(lines[base + offset], lines[base + stride + offset], select);
            }
        }
    }
    lines[0]
}

pub fn it_active<B: GateBackend>(backend: &mut B, itstate: u8) -> u8 {
    (0..8).fold(0, |active, bit| backend.or(active, (itstate >> bit) & 1))
}

pub fn advance_itstate<B: GateBackend>(backend: &mut B, itstate: u8) -> u8 {
    let continue_block = (0..3).fold(0, |active, bit| backend.or(active, (itstate >> bit) & 1));
    let shifted = (itstate & 0xe0) | ((itstate << 1) & 0x1f);
    let mut result = 0;
    for bit in 0..8 {
        result |= backend.mux(0, (shifted >> bit) & 1, continue_block) << bit;
    }
    result
}

pub fn write_enable<B: GateBackend>(backend: &mut B, in_it: u8, condition: u8) -> u8 {
    backend.mux(1, condition, in_it)
}

pub fn transport_ge<B: GateBackend>(backend: &mut B, old_ge: u8, source: u32, write: u8) -> u8 {
    let source = ((source >> 16) & 0xf) as u8;
    let mut result = 0;
    for bit in 0..4 {
        result |= backend.mux((old_ge >> bit) & 1, (source >> bit) & 1, write) << bit;
    }
    result
}

pub fn apsr_transport_lines<B: GateBackend>(backend: &mut B, first: u16, second: u16) -> (u8, u8) {
    let apsr_alias = equal_masked(backend, second, 0x00fc, 0);
    let mrs_opcode = equal_masked(backend, first, 0xffff, 0xf3ef);
    let msr_opcode = equal_masked(backend, first, 0xfff0, 0xf380);
    (
        backend.and(apsr_alias, mrs_opcode),
        backend.and(apsr_alias, msr_opcode),
    )
}
