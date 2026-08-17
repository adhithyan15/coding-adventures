//! # `intel8051-simulator::execute` — instruction semantics.
//!
//! Takes a [`crate::decode::DecodedInstr`] (opcode + already-fetched
//! operand bytes) and a `&mut` [`crate::simulator::Intel8051Simulator`]
//! and performs the state mutation, returning the instruction's
//! mnemonic string — the same contract as `simulator.py::_execute_one`,
//! split so decode stays pure and testable on its own.
//!
//! Every branch below is a direct, mechanical port of
//! `code/packages/python/intel8051-simulator/src/intel8051_simulator/
//! simulator.py`'s `_execute_one` method (opcode-by-opcode) and
//! `flags.py`'s `add8_flags`/`sub8_flags`/`da_flags` arithmetic helpers
//! — see that file's inline comments for the flag-computation
//! rationale (the truth tables are reproduced here too, so this module
//! is self-contained).

use crate::decode::DecodedInstr;
use crate::opcodes::*;
use crate::simulator::Intel8051Simulator;

// ===========================================================================
// Flag helpers — ported from flags.py
// ===========================================================================

/// Even-parity bit of an 8-bit value: 1 if `val` has an *odd* popcount
/// (so that the 9-bit `{P, val}` pair has *even* parity overall — the
/// 8051's documented convention for PSW.P).
fn parity(val: u8) -> u8 {
    let mut v = val;
    v ^= v >> 4;
    v ^= v >> 2;
    v ^= v >> 1;
    v & 1
}

/// `A + B + cin` → `(result, CY, AC, OV, P)`.
///
/// ```text
/// CY: result > 0xFF (unsigned carry out of bit 7)
/// AC: (a&0xF) + (b&0xF) + cin > 0xF (carry from bit 3 to bit 4)
/// OV: same-sign operands, different-sign result (signed overflow)
/// P:  even parity of the 8-bit result
/// ```
fn add8_flags(a: u8, b: u8, cin: u8) -> (u8, u8, u8, u8, u8) {
    let full = a as u16 + b as u16 + cin as u16;
    let result = (full & 0xFF) as u8;
    let cy = u8::from(full > 0xFF);
    let ac = u8::from((a & 0x0F) as u16 + (b & 0x0F) as u16 + cin as u16 > 0x0F);
    let (sa, sb, sr) = (a >> 7, b >> 7, result >> 7);
    let ov = u8::from(sa == sb && sr != sa);
    (result, cy, ac, ov, parity(result))
}

/// `A - B - borrow` (SUBB) → `(result, CY, AC, OV, P)`.
///
/// CY here is a **borrow** flag: `1` means unsigned underflow occurred
/// (`a < b + borrow`) — the opposite convention from x86's "carry =
/// no borrow".
fn sub8_flags(a: u8, b: u8, borrow: u8) -> (u8, u8, u8, u8, u8) {
    let full = a as i16 - b as i16 - borrow as i16;
    let result = (full & 0xFF) as u8;
    let cy = u8::from(full < 0);
    let ac = u8::from((a & 0x0F) < (b & 0x0F) + borrow);
    let (sa, sb, sr) = (a >> 7, b >> 7, result >> 7);
    let ov = u8::from(sa != sb && sr == sb);
    (result, cy, ac, ov, parity(result))
}

/// `DA A` — decimal-adjust after BCD `ADD`/`ADDC`.  Returns
/// `(result, new_cy, new_p)`.
fn da_flags(a: u8, cy_in: u8, ac_in: u8) -> (u8, u8, u8) {
    let mut a = a;
    let mut new_cy = cy_in;
    if (a & 0x0F) > 9 || ac_in != 0 {
        a = a.wrapping_add(0x06);
    }
    if (a >> 4) > 9 || cy_in != 0 {
        a = a.wrapping_add(0x60);
        new_cy = 1;
    }
    (a, new_cy, parity(a))
}

// ===========================================================================
// Sign-extension helper for relative branch offsets
// ===========================================================================

/// Add a signed 8-bit relative offset to `pc`, wrapping at 64 KiB —
/// `SJMP`/`Jcc`/`CJNE`/`DJNZ`'s target-address arithmetic.
fn add_rel(pc: u16, rel: u8) -> u16 {
    pc.wrapping_add((rel as i8) as u16)
}

// ===========================================================================
// execute
// ===========================================================================

/// Execute one already-decoded instruction against `sim`, returning its
/// mnemonic.  `sim.pc` must already equal `d.next_pc` (the caller,
/// `Intel8051Simulator::step`, sets it before calling this) — every
/// branch/call instruction below computes its target relative to that
/// post-fetch PC, matching `simulator.py`'s "self._pc already advanced"
/// invariant.
#[allow(clippy::too_many_lines)]
pub fn execute(sim: &mut Intel8051Simulator, d: &DecodedInstr) -> &'static str {
    let opcode = d.opcode;

    if opcode == HALT_OPCODE {
        sim.halted = true;
        return "HALT";
    }
    if opcode == NOP {
        return "NOP";
    }

    // ── Data transfer ───────────────────────────────────────────────
    if (MOV_A_RN_BASE..=MOV_A_RN_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let v = sim.rn(n);
        sim.set_acc(v);
        return "MOV A,Rn";
    }
    if opcode == MOV_A_DIR {
        let v = sim.direct_read(d.operands[0]);
        sim.set_acc(v);
        return "MOV A,dir";
    }
    if opcode == MOV_A_AT_RI_BASE || opcode == MOV_A_AT_RI_BASE + 1 {
        let v = sim.indirect_read(opcode & 1);
        sim.set_acc(v);
        return "MOV A,@Ri";
    }
    if opcode == MOV_A_IMM {
        sim.set_acc(d.operands[0]);
        return "MOV A,#imm";
    }
    if (MOV_RN_A_BASE..=MOV_RN_A_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let a = sim.acc();
        sim.set_rn(n, a);
        return "MOV Rn,A";
    }
    if (MOV_RN_DIR_BASE..=MOV_RN_DIR_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let v = sim.direct_read(d.operands[0]);
        sim.set_rn(n, v);
        return "MOV Rn,dir";
    }
    if (MOV_RN_IMM_BASE..=MOV_RN_IMM_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        sim.set_rn(n, d.operands[0]);
        return "MOV Rn,#imm";
    }
    if opcode == MOV_DIR_A {
        let a = sim.acc();
        sim.direct_write(d.operands[0], a);
        return "MOV dir,A";
    }
    if (MOV_DIR_RN_BASE..=MOV_DIR_RN_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let v = sim.rn(n);
        sim.direct_write(d.operands[0], v);
        return "MOV dir,Rn";
    }
    if opcode == MOV_DIR_DIR {
        // Encoding is `0x85 src dst`: operand[0]=src, operand[1]=dst.
        let v = sim.direct_read(d.operands[0]);
        sim.direct_write(d.operands[1], v);
        return "MOV dir,dir";
    }
    if opcode == MOV_DIR_AT_RI_BASE || opcode == MOV_DIR_AT_RI_BASE + 1 {
        let v = sim.indirect_read(opcode & 1);
        sim.direct_write(d.operands[0], v);
        return "MOV dir,@Ri";
    }
    if opcode == MOV_DIR_IMM {
        sim.direct_write(d.operands[0], d.operands[1]);
        return "MOV dir,#imm";
    }
    if opcode == MOV_AT_RI_A_BASE || opcode == MOV_AT_RI_A_BASE + 1 {
        let a = sim.acc();
        sim.indirect_write(opcode & 1, a);
        return "MOV @Ri,A";
    }
    if opcode == MOV_AT_RI_DIR_BASE || opcode == MOV_AT_RI_DIR_BASE + 1 {
        let v = sim.direct_read(d.operands[0]);
        sim.indirect_write(opcode & 1, v);
        return "MOV @Ri,dir";
    }
    if opcode == MOV_AT_RI_IMM_BASE || opcode == MOV_AT_RI_IMM_BASE + 1 {
        sim.indirect_write(opcode & 1, d.operands[0]);
        return "MOV @Ri,#imm";
    }
    if opcode == MOV_DPTR_IMM {
        let hi = d.operands[0];
        let lo = d.operands[1];
        sim.set_dptr(((hi as u16) << 8) | lo as u16);
        return "MOV DPTR,#imm16";
    }
    if opcode == MOVC_A_AT_A_DPTR {
        let ea = (sim.acc() as u16).wrapping_add(sim.dptr());
        let v = sim.code[ea as usize];
        sim.set_acc(v);
        return "MOVC A,@A+DPTR";
    }
    if opcode == MOVC_A_AT_A_PC {
        // sim.pc is already past this 1-byte instruction (d.next_pc).
        let ea = (sim.acc() as u16).wrapping_add(sim.pc);
        let v = sim.code[ea as usize];
        sim.set_acc(v);
        return "MOVC A,@A+PC";
    }
    if opcode == MOVX_A_AT_RI_BASE || opcode == MOVX_A_AT_RI_BASE + 1 {
        let addr = sim.rn(opcode & 1);
        let v = sim.xdata[addr as usize];
        sim.set_acc(v);
        return "MOVX A,@Ri";
    }
    if opcode == MOVX_A_AT_DPTR {
        let v = sim.xdata[sim.dptr() as usize];
        sim.set_acc(v);
        return "MOVX A,@DPTR";
    }
    if opcode == MOVX_AT_RI_A_BASE || opcode == MOVX_AT_RI_A_BASE + 1 {
        let addr = sim.rn(opcode & 1);
        sim.xdata[addr as usize] = sim.acc();
        return "MOVX @Ri,A";
    }
    if opcode == MOVX_AT_DPTR_A {
        let addr = sim.dptr();
        sim.xdata[addr as usize] = sim.acc();
        return "MOVX @DPTR,A";
    }
    if opcode == PUSH {
        let v = sim.direct_read(d.operands[0]);
        sim.push8(v);
        return "PUSH";
    }
    if opcode == POP {
        let v = sim.pop8();
        sim.direct_write(d.operands[0], v);
        return "POP";
    }
    if (XCH_A_RN_BASE..=XCH_A_RN_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let (a, rn) = (sim.acc(), sim.rn(n));
        sim.set_acc(rn);
        sim.set_rn(n, a);
        return "XCH A,Rn";
    }
    if opcode == XCH_A_DIR {
        let a = sim.acc();
        let mem = sim.direct_read(d.operands[0]);
        sim.set_acc(mem);
        sim.direct_write(d.operands[0], a);
        return "XCH A,dir";
    }
    if opcode == XCH_A_AT_RI_BASE || opcode == XCH_A_AT_RI_BASE + 1 {
        let i = opcode & 1;
        let (a, mem) = (sim.acc(), sim.indirect_read(i));
        sim.set_acc(mem);
        sim.indirect_write(i, a);
        return "XCH A,@Ri";
    }
    if opcode == XCHD_A_AT_RI_BASE || opcode == XCHD_A_AT_RI_BASE + 1 {
        let i = opcode & 1;
        let (a, mem) = (sim.acc(), sim.indirect_read(i));
        let swapped_a = (a & 0xF0) | (mem & 0x0F);
        let swapped_mem = (mem & 0xF0) | (a & 0x0F);
        sim.set_acc(swapped_a);
        sim.indirect_write(i, swapped_mem);
        return "XCHD A,@Ri";
    }

    // ── Arithmetic ──────────────────────────────────────────────────
    if (ADD_A_RN_BASE..=ADD_A_RN_BASE + 7).contains(&opcode) {
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), sim.rn(opcode & 7), 0);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADD A,Rn";
    }
    if opcode == ADD_A_DIR {
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), sim.direct_read(d.operands[0]), 0);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADD A,dir";
    }
    if opcode == ADD_A_AT_RI_BASE || opcode == ADD_A_AT_RI_BASE + 1 {
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), sim.indirect_read(opcode & 1), 0);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADD A,@Ri";
    }
    if opcode == ADD_A_IMM {
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), d.operands[0], 0);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADD A,#imm";
    }
    if (ADDC_A_RN_BASE..=ADDC_A_RN_BASE + 7).contains(&opcode) {
        let cin = sim.cy_bit();
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), sim.rn(opcode & 7), cin);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADDC A,Rn";
    }
    if opcode == ADDC_A_DIR {
        let cin = sim.cy_bit();
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), sim.direct_read(d.operands[0]), cin);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADDC A,dir";
    }
    if opcode == ADDC_A_AT_RI_BASE || opcode == ADDC_A_AT_RI_BASE + 1 {
        let cin = sim.cy_bit();
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), sim.indirect_read(opcode & 1), cin);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADDC A,@Ri";
    }
    if opcode == ADDC_A_IMM {
        let cin = sim.cy_bit();
        let (r, cy, ac, ov, _p) = add8_flags(sim.acc(), d.operands[0], cin);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "ADDC A,#imm";
    }
    if (SUBB_A_RN_BASE..=SUBB_A_RN_BASE + 7).contains(&opcode) {
        let borrow = sim.cy_bit();
        let (r, cy, ac, ov, _p) = sub8_flags(sim.acc(), sim.rn(opcode & 7), borrow);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "SUBB A,Rn";
    }
    if opcode == SUBB_A_DIR {
        let borrow = sim.cy_bit();
        let (r, cy, ac, ov, _p) = sub8_flags(sim.acc(), sim.direct_read(d.operands[0]), borrow);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "SUBB A,dir";
    }
    if opcode == SUBB_A_AT_RI_BASE || opcode == SUBB_A_AT_RI_BASE + 1 {
        let borrow = sim.cy_bit();
        let (r, cy, ac, ov, _p) = sub8_flags(sim.acc(), sim.indirect_read(opcode & 1), borrow);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "SUBB A,@Ri";
    }
    if opcode == SUBB_A_IMM {
        let borrow = sim.cy_bit();
        let (r, cy, ac, ov, _p) = sub8_flags(sim.acc(), d.operands[0], borrow);
        sim.iram[SFR_ACC as usize] = r;
        sim.set_flags(cy, ac, ov);
        sim.update_parity();
        return "SUBB A,#imm";
    }
    if opcode == INC_A {
        sim.iram[SFR_ACC as usize] = sim.acc().wrapping_add(1);
        sim.update_parity();
        return "INC A";
    }
    if (INC_RN_BASE..=INC_RN_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let v = sim.rn(n).wrapping_add(1);
        sim.set_rn(n, v);
        return "INC Rn";
    }
    if opcode == INC_DIR {
        let v = sim.direct_read(d.operands[0]).wrapping_add(1);
        sim.direct_write(d.operands[0], v);
        return "INC dir";
    }
    if opcode == INC_AT_RI_BASE || opcode == INC_AT_RI_BASE + 1 {
        let i = opcode & 1;
        let v = sim.indirect_read(i).wrapping_add(1);
        sim.indirect_write(i, v);
        return "INC @Ri";
    }
    if opcode == INC_DPTR {
        let v = sim.dptr().wrapping_add(1);
        sim.set_dptr(v);
        return "INC DPTR";
    }
    if opcode == DEC_A {
        sim.iram[SFR_ACC as usize] = sim.acc().wrapping_sub(1);
        sim.update_parity();
        return "DEC A";
    }
    if (DEC_RN_BASE..=DEC_RN_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let v = sim.rn(n).wrapping_sub(1);
        sim.set_rn(n, v);
        return "DEC Rn";
    }
    if opcode == DEC_DIR {
        let v = sim.direct_read(d.operands[0]).wrapping_sub(1);
        sim.direct_write(d.operands[0], v);
        return "DEC dir";
    }
    if opcode == DEC_AT_RI_BASE || opcode == DEC_AT_RI_BASE + 1 {
        let i = opcode & 1;
        let v = sim.indirect_read(i).wrapping_sub(1);
        sim.indirect_write(i, v);
        return "DEC @Ri";
    }
    if opcode == MUL_AB {
        let product = sim.acc() as u16 * sim.iram[SFR_B as usize] as u16;
        sim.iram[SFR_ACC as usize] = (product & 0xFF) as u8;
        sim.iram[SFR_B as usize] = (product >> 8) as u8;
        let ov = u8::from((product >> 8) != 0);
        sim.set_flags(0, 0, ov);
        sim.update_parity();
        return "MUL AB";
    }
    if opcode == DIV_AB {
        let divisor = sim.iram[SFR_B as usize];
        let a = sim.acc();
        match a.checked_div(divisor) {
            None => sim.set_flags(0, 0, 1),
            Some(q) => {
                sim.iram[SFR_ACC as usize] = q;
                sim.iram[SFR_B as usize] = a % divisor;
                sim.set_flags(0, 0, 0);
            }
        }
        sim.update_parity();
        return "DIV AB";
    }
    if opcode == DA_A {
        let cy_in = sim.cy_bit();
        let ac_in = u8::from(sim.iram[SFR_PSW as usize] & PSW_AC != 0);
        let (result, new_cy, new_p) = da_flags(sim.acc(), cy_in, ac_in);
        sim.iram[SFR_ACC as usize] = result;
        let mut psw = sim.iram[SFR_PSW as usize];
        psw = if new_cy != 0 { psw | PSW_CY } else { psw & !PSW_CY };
        psw = if new_p != 0 { psw | PSW_P } else { psw & !PSW_P };
        sim.iram[SFR_PSW as usize] = psw;
        return "DA A";
    }

    // ── Logic ───────────────────────────────────────────────────────
    if (ANL_A_RN_BASE..=ANL_A_RN_BASE + 7).contains(&opcode) {
        let v = sim.acc() & sim.rn(opcode & 7);
        sim.set_acc(v);
        return "ANL A,Rn";
    }
    if opcode == ANL_A_DIR {
        let v = sim.acc() & sim.direct_read(d.operands[0]);
        sim.set_acc(v);
        return "ANL A,dir";
    }
    if opcode == ANL_A_AT_RI_BASE || opcode == ANL_A_AT_RI_BASE + 1 {
        let v = sim.acc() & sim.indirect_read(opcode & 1);
        sim.set_acc(v);
        return "ANL A,@Ri";
    }
    if opcode == ANL_A_IMM {
        let v = sim.acc() & d.operands[0];
        sim.set_acc(v);
        return "ANL A,#imm";
    }
    if opcode == ANL_DIR_A {
        let v = sim.direct_read(d.operands[0]) & sim.acc();
        sim.direct_write(d.operands[0], v);
        return "ANL dir,A";
    }
    if opcode == ANL_DIR_IMM {
        let v = sim.direct_read(d.operands[0]) & d.operands[1];
        sim.direct_write(d.operands[0], v);
        return "ANL dir,#imm";
    }
    if (ORL_A_RN_BASE..=ORL_A_RN_BASE + 7).contains(&opcode) {
        let v = sim.acc() | sim.rn(opcode & 7);
        sim.set_acc(v);
        return "ORL A,Rn";
    }
    if opcode == ORL_A_DIR {
        let v = sim.acc() | sim.direct_read(d.operands[0]);
        sim.set_acc(v);
        return "ORL A,dir";
    }
    if opcode == ORL_A_AT_RI_BASE || opcode == ORL_A_AT_RI_BASE + 1 {
        let v = sim.acc() | sim.indirect_read(opcode & 1);
        sim.set_acc(v);
        return "ORL A,@Ri";
    }
    if opcode == ORL_A_IMM {
        let v = sim.acc() | d.operands[0];
        sim.set_acc(v);
        return "ORL A,#imm";
    }
    if opcode == ORL_DIR_A {
        let v = sim.direct_read(d.operands[0]) | sim.acc();
        sim.direct_write(d.operands[0], v);
        return "ORL dir,A";
    }
    if opcode == ORL_DIR_IMM {
        let v = sim.direct_read(d.operands[0]) | d.operands[1];
        sim.direct_write(d.operands[0], v);
        return "ORL dir,#imm";
    }
    if (XRL_A_RN_BASE..=XRL_A_RN_BASE + 7).contains(&opcode) {
        let v = sim.acc() ^ sim.rn(opcode & 7);
        sim.set_acc(v);
        return "XRL A,Rn";
    }
    if opcode == XRL_A_DIR {
        let v = sim.acc() ^ sim.direct_read(d.operands[0]);
        sim.set_acc(v);
        return "XRL A,dir";
    }
    if opcode == XRL_A_AT_RI_BASE || opcode == XRL_A_AT_RI_BASE + 1 {
        let v = sim.acc() ^ sim.indirect_read(opcode & 1);
        sim.set_acc(v);
        return "XRL A,@Ri";
    }
    if opcode == XRL_A_IMM {
        let v = sim.acc() ^ d.operands[0];
        sim.set_acc(v);
        return "XRL A,#imm";
    }
    if opcode == XRL_DIR_A {
        let v = sim.direct_read(d.operands[0]) ^ sim.acc();
        sim.direct_write(d.operands[0], v);
        return "XRL dir,A";
    }
    if opcode == XRL_DIR_IMM {
        let v = sim.direct_read(d.operands[0]) ^ d.operands[1];
        sim.direct_write(d.operands[0], v);
        return "XRL dir,#imm";
    }
    if opcode == CLR_A {
        sim.set_acc(0);
        return "CLR A";
    }
    if opcode == CPL_A {
        let v = !sim.acc();
        sim.set_acc(v);
        return "CPL A";
    }
    if opcode == RL_A {
        let a = sim.acc();
        sim.set_acc(a.rotate_left(1));
        return "RL A";
    }
    if opcode == RLC_A {
        let a = sim.acc();
        let new_cy = a >> 7;
        let cy_in = sim.cy_bit();
        sim.set_acc((a << 1) | cy_in);
        sim.iram[SFR_PSW as usize] = if new_cy != 0 {
            sim.iram[SFR_PSW as usize] | PSW_CY
        } else {
            sim.iram[SFR_PSW as usize] & !PSW_CY
        };
        return "RLC A";
    }
    if opcode == RR_A {
        let a = sim.acc();
        sim.set_acc(a.rotate_right(1));
        return "RR A";
    }
    if opcode == RRC_A {
        let a = sim.acc();
        let new_cy = a & 1;
        let cy_in = sim.cy_bit();
        sim.set_acc((a >> 1) | (cy_in << 7));
        sim.iram[SFR_PSW as usize] = if new_cy != 0 {
            sim.iram[SFR_PSW as usize] | PSW_CY
        } else {
            sim.iram[SFR_PSW as usize] & !PSW_CY
        };
        return "RRC A";
    }
    if opcode == SWAP_A {
        let a = sim.acc();
        // SWAP does not update parity (no logical change, just nibble
        // reorder) — write iram directly rather than via set_acc.
        sim.iram[SFR_ACC as usize] = a.rotate_left(4);
        return "SWAP A";
    }

    // ── Bit operations ──────────────────────────────────────────────
    if opcode == CLR_C {
        sim.iram[SFR_PSW as usize] &= !PSW_CY;
        return "CLR C";
    }
    if opcode == CLR_BIT {
        sim.write_bit(d.operands[0], 0);
        return "CLR bit";
    }
    if opcode == SETB_C {
        sim.iram[SFR_PSW as usize] |= PSW_CY;
        return "SETB C";
    }
    if opcode == SETB_BIT {
        sim.write_bit(d.operands[0], 1);
        return "SETB bit";
    }
    if opcode == CPL_C {
        sim.iram[SFR_PSW as usize] ^= PSW_CY;
        return "CPL C";
    }
    if opcode == CPL_BIT {
        let bit = d.operands[0];
        let cur = sim.read_bit(bit);
        sim.write_bit(bit, 1 - cur);
        return "CPL bit";
    }
    if opcode == ANL_C_BIT {
        if sim.read_bit(d.operands[0]) == 0 {
            sim.iram[SFR_PSW as usize] &= !PSW_CY;
        }
        return "ANL C,bit";
    }
    if opcode == ANL_C_NBIT {
        if sim.read_bit(d.operands[0]) != 0 {
            sim.iram[SFR_PSW as usize] &= !PSW_CY;
        }
        return "ANL C,/bit";
    }
    if opcode == ORL_C_BIT {
        if sim.read_bit(d.operands[0]) != 0 {
            sim.iram[SFR_PSW as usize] |= PSW_CY;
        }
        return "ORL C,bit";
    }
    if opcode == ORL_C_NBIT {
        if sim.read_bit(d.operands[0]) == 0 {
            sim.iram[SFR_PSW as usize] |= PSW_CY;
        }
        return "ORL C,/bit";
    }
    if opcode == MOV_C_BIT {
        let bit_val = sim.read_bit(d.operands[0]);
        if bit_val != 0 {
            sim.iram[SFR_PSW as usize] |= PSW_CY;
        } else {
            sim.iram[SFR_PSW as usize] &= !PSW_CY;
        }
        return "MOV C,bit";
    }
    if opcode == MOV_BIT_C {
        let cy = sim.cy_bit();
        sim.write_bit(d.operands[0], cy);
        return "MOV bit,C";
    }

    // ── Jumps ───────────────────────────────────────────────────────
    if opcode == LJMP {
        sim.pc = ((d.operands[0] as u16) << 8) | d.operands[1] as u16;
        return "LJMP";
    }
    if opcode == SJMP {
        sim.pc = add_rel(sim.pc, d.operands[0]);
        return "SJMP";
    }
    if opcode == JMP_AT_A_DPTR {
        sim.pc = (sim.acc() as u16).wrapping_add(sim.dptr());
        return "JMP @A+DPTR";
    }
    if opcode & 0x1F == AJMP_PATTERN {
        let addr11_hi = ((opcode >> 5) & 0x7) as u16;
        let addr11_lo = d.operands[0] as u16;
        sim.pc = (sim.pc & 0xF800) | (addr11_hi << 8) | addr11_lo;
        return "AJMP";
    }
    if opcode == JZ {
        if sim.acc() == 0 {
            sim.pc = add_rel(sim.pc, d.operands[0]);
        }
        return "JZ";
    }
    if opcode == JNZ {
        if sim.acc() != 0 {
            sim.pc = add_rel(sim.pc, d.operands[0]);
        }
        return "JNZ";
    }
    if opcode == JC {
        if sim.cy_bit() != 0 {
            sim.pc = add_rel(sim.pc, d.operands[0]);
        }
        return "JC";
    }
    if opcode == JNC {
        if sim.cy_bit() == 0 {
            sim.pc = add_rel(sim.pc, d.operands[0]);
        }
        return "JNC";
    }
    if opcode == JB {
        let (bit, rel) = (d.operands[0], d.operands[1]);
        if sim.read_bit(bit) != 0 {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "JB";
    }
    if opcode == JNB {
        let (bit, rel) = (d.operands[0], d.operands[1]);
        if sim.read_bit(bit) == 0 {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "JNB";
    }
    if opcode == JBC {
        let (bit, rel) = (d.operands[0], d.operands[1]);
        if sim.read_bit(bit) != 0 {
            sim.write_bit(bit, 0);
            sim.pc = add_rel(sim.pc, rel);
        }
        return "JBC";
    }
    if opcode == CJNE_A_DIR {
        let (dir, rel) = (d.operands[0], d.operands[1]);
        let val = sim.direct_read(dir);
        set_cy(sim, sim.acc() < val);
        if sim.acc() != val {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "CJNE A,dir";
    }
    if opcode == CJNE_A_IMM {
        let (imm, rel) = (d.operands[0], d.operands[1]);
        set_cy(sim, sim.acc() < imm);
        if sim.acc() != imm {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "CJNE A,#imm";
    }
    if (CJNE_RN_IMM_BASE..=CJNE_RN_IMM_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let (imm, rel) = (d.operands[0], d.operands[1]);
        let rn = sim.rn(n);
        set_cy(sim, rn < imm);
        if rn != imm {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "CJNE Rn,#imm";
    }
    if opcode == CJNE_AT_RI_IMM_BASE || opcode == CJNE_AT_RI_IMM_BASE + 1 {
        let i = opcode & 1;
        let (imm, rel) = (d.operands[0], d.operands[1]);
        let mem = sim.indirect_read(i);
        set_cy(sim, mem < imm);
        if mem != imm {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "CJNE @Ri,#imm";
    }
    if (DJNZ_RN_BASE..=DJNZ_RN_BASE + 7).contains(&opcode) {
        let n = opcode & 7;
        let rel = d.operands[0];
        let val = sim.rn(n).wrapping_sub(1);
        sim.set_rn(n, val);
        if val != 0 {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "DJNZ Rn";
    }
    if opcode == DJNZ_DIR {
        let (dir, rel) = (d.operands[0], d.operands[1]);
        let val = sim.direct_read(dir).wrapping_sub(1);
        sim.direct_write(dir, val);
        if val != 0 {
            sim.pc = add_rel(sim.pc, rel);
        }
        return "DJNZ dir";
    }

    // ── Subroutines ─────────────────────────────────────────────────
    if opcode == LCALL {
        let addr = ((d.operands[0] as u16) << 8) | d.operands[1] as u16;
        sim.push_pc();
        sim.pc = addr;
        return "LCALL";
    }
    if opcode & 0x1F == ACALL_PATTERN {
        let addr11_hi = ((opcode >> 5) & 0x7) as u16;
        let addr11_lo = d.operands[0] as u16;
        sim.push_pc();
        sim.pc = (sim.pc & 0xF800) | (addr11_hi << 8) | addr11_lo;
        return "ACALL";
    }
    if opcode == RET {
        sim.pop_pc();
        return "RET";
    }
    if opcode == RETI {
        // Same as RET for this behavioral sim — no interrupt controller.
        sim.pop_pc();
        return "RETI";
    }

    panic!(
        "intel8051-simulator: unknown/reserved opcode 0x{opcode:02X} at PC=0x{:04X}",
        d.pc_before
    );
}

/// Set/clear PSW.CY per a boolean condition — a tiny helper shared by
/// the `CJNE` family (`Python`'s inline if/else on `self._iram[SFR_PSW]`).
fn set_cy(sim: &mut Intel8051Simulator, carry: bool) {
    if carry {
        sim.iram[SFR_PSW as usize] |= PSW_CY;
    } else {
        sim.iram[SFR_PSW as usize] &= !PSW_CY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add8_flags_simple() {
        let (r, cy, ac, ov, p) = add8_flags(1, 1, 0);
        assert_eq!((r, cy, ac, ov), (2, 0, 0, 0));
        assert_eq!(p, parity(2));
    }

    #[test]
    fn add8_flags_carry_out() {
        let (r, cy, _ac, _ov, _p) = add8_flags(0xFF, 0x01, 0);
        assert_eq!(r, 0x00);
        assert_eq!(cy, 1);
    }

    #[test]
    fn add8_flags_aux_carry() {
        let (_r, _cy, ac, _ov, _p) = add8_flags(0x0F, 0x01, 0);
        assert_eq!(ac, 1);
    }

    #[test]
    fn add8_flags_signed_overflow() {
        // 0x7F + 0x01 = 0x80: two positives producing a negative
        // (sign bit set) result -> signed overflow.
        let (r, _cy, _ac, ov, _p) = add8_flags(0x7F, 0x01, 0);
        assert_eq!(r, 0x80);
        assert_eq!(ov, 1);
    }

    #[test]
    fn sub8_flags_borrow() {
        let (r, cy, _ac, _ov, _p) = sub8_flags(0x00, 0x01, 0);
        assert_eq!(r, 0xFF);
        assert_eq!(cy, 1, "borrow expected");
    }

    #[test]
    fn sub8_flags_no_borrow() {
        let (r, cy, _ac, _ov, _p) = sub8_flags(0x05, 0x03, 0);
        assert_eq!(r, 0x02);
        assert_eq!(cy, 0);
    }

    #[test]
    fn da_flags_below_nine_no_adjust() {
        let (result, cy, _p) = da_flags(0x05, 0, 0);
        assert_eq!(result, 0x05);
        assert_eq!(cy, 0);
    }

    #[test]
    fn da_flags_low_nibble_over_nine() {
        // 0x0A -> low nibble > 9, add 0x06 -> 0x10.
        let (result, _cy, _p) = da_flags(0x0A, 0, 0);
        assert_eq!(result, 0x10);
    }

    #[test]
    fn parity_even_and_odd() {
        assert_eq!(parity(0x00), 0); // 0 bits set -> even popcount -> P=0
        assert_eq!(parity(0x01), 1); // 1 bit set -> odd popcount -> P=1
        assert_eq!(parity(0x03), 0); // 2 bits set -> even -> P=0
    }

    #[test]
    fn add_rel_forward_and_backward() {
        assert_eq!(add_rel(0x0010, 0x05), 0x0015);
        // 0xFE = -2 signed
        assert_eq!(add_rel(0x0010, 0xFE), 0x000E);
    }
}
