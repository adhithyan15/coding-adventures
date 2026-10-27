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

// ---------------------------------------------------------------------------
// APSR MSR write masks
// ---------------------------------------------------------------------------
//
// `MSR <spec_reg>, Rn` (T1) is two halfwords:
//
// ```text
//   first : 1111 0011 1000 nnnn          0xf380 | Rn
//   second: 10m0 mm00 SYSm SYSm          bits 15:12 = 1000, bits 9:8 = 00
//                ^^                      bits 11:10 = mask<1:0>
// ```
//
// For the APSR aliases (`SYSm` 0..=3: APSR, IAPSR, EAPSR, XPSR) the two mask
// bits name *which* APSR field groups the write touches:
//
// | mask (11:10) | assembler form   | NZCVQ written | GE written | M3  | M7  |
// |--------------|------------------|---------------|------------|-----|-----|
// | `00`         | (reserved)       | -             | -          | UND | UND |
// | `01`         | `APSR_g`         | no            | yes        | UND | ok  |
// | `10`         | `APSR_nzcvq`     | yes           | no         | ok  | ok  |
// | `11`         | `APSR_nzcvqg`    | yes           | yes        | UND | ok  |
//
// GE only exists with the DSP extension, so the inherited Cortex-M3 gate
// engine owns exactly mask `10` and refuses every other mask. The Cortex-M7
// companion therefore splits the work:
//
// * mask `10` is delegated to the inherited engine, which writes NZCVQ; the
//   GE transport below is qualified by mask bit `g` (bit 10), which is 0 for
//   this form, so the GE DFFs keep their old value.
// * masks `01` and `11` are *owned* by this crate: `apsr_msr_owned` raises a
//   line that routes the instruction to the M7 path, where NZCVQ moves only
//   through a mux selected by bit `nzcvq` (bit 11) and GE only through a mux
//   selected by bit `g` (bit 10).
// * mask `00`, and any GE-writing mask aimed at a non-APSR `SYSm`, is left to
//   the inherited decoder, which refuses it exactly as the functional
//   Cortex-M7 simulator does.
//
// Every one of these decisions is a gate output: the host code only reads a
// settled wire to choose which already-built network's output to clock.

/// Opcode line for the T1 `MSR` first halfword (`0xf380 | Rn`).
pub fn msr_prefix<B: GateBackend>(backend: &mut B, first: u16) -> u8 {
    equal_masked(backend, first, 0xfff0, 0xf380)
}

/// The two architectural mask wires of an `MSR` second halfword.
///
/// Returns `(nzcvq, g)`: bit 11 enables the N/Z/C/V/Q group and bit 10
/// enables the `GE[3:0]` group. Each wire is produced by an equality network
/// so a faulty gate is observable rather than hidden behind a host shift.
pub fn apsr_msr_mask<B: GateBackend>(backend: &mut B, second: u16) -> (u8, u8) {
    let nzcvq = equal_masked(backend, second, 0x0800, 0x0800);
    let g = equal_masked(backend, second, 0x0400, 0x0400);
    (nzcvq, g)
}

/// Line that is 1 exactly for the GE-writing APSR `MSR` forms (masks `01` and
/// `11`) that the inherited non-DSP engine refuses and this crate executes.
///
/// ```text
///   owned = msr_prefix(first)
///         AND (second & 0xf3fc == 0x8000)   -- fixed bits and SYSm 0..=3
///         AND g                             -- mask bit 10
/// ```
pub fn apsr_msr_owned<B: GateBackend>(backend: &mut B, first: u16, second: u16) -> u8 {
    let opcode = msr_prefix(backend, first);
    let apsr_shape = equal_masked(backend, second, 0xf3fc, 0x8000);
    let (_, g) = apsr_msr_mask(backend, second);
    let shaped = backend.and(opcode, apsr_shape);
    backend.and(shaped, g)
}

/// Source-register field of an `MSR`, with the gate-derived "is R0..R12" line.
///
/// SP, LR, and PC are UNPREDICTABLE sources and fault, exactly like the
/// functional simulator and the DSP operand checks above.
pub fn msr_source<B: GateBackend>(backend: &mut B, first: u16) -> (usize, u8) {
    let rn = (first & 0xf) as u8;
    (rn as usize, valid_low_register(backend, rn))
}

/// Mask-qualified NZCVQ transport for `MSR APSR_nzcvq*`.
///
/// Each of the 32 APSR output wires is a 2:1 mux. With `write = 0` the old Q
/// value passes through untouched. With `write = 1` the mux selects the
/// source register bit ANDed with the NZCVQ field mask `0xf800_0000`, so bits
/// 31:27 copy `Rn` and bits 26:0 become 0, which is the same replacement the
/// functional core performs (`apsr = Rn & 0xf800_0000`).
pub fn transport_nzcvq<B: GateBackend>(
    backend: &mut B,
    old_apsr: u32,
    source: u32,
    write: u8,
) -> u32 {
    const NZCVQ_FIELD: u32 = 0xf800_0000;
    let old = bits32(old_apsr);
    let source = bits32(source);
    let field = bits32(NZCVQ_FIELD);
    let result = std::array::from_fn(|bit| {
        let masked = backend.and(source[bit], field[bit]);
        backend.mux(old[bit], masked, write)
    });
    from_bits32(result)
}
