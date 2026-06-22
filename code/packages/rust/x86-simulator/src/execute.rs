//! Execute one decoded [`Instr`] against the CPU + memory.
//!
//! All register operations are 64-bit. `Cmp`/`Test` set flags without writing a
//! result; the other ALU ops write back. Control-flow instructions return a
//! [`Flow`] the step loop acts on.

use crate::decode::{AluOp, Instr, Operand, ShiftOp, SseOp, XmmRm};
use crate::flags::{add_with_flags, logic_flags, sub_with_flags};
use crate::state::{CpuState, Flags, Reg};
use crate::memory::Memory;
use crate::trap::Trap;

/// Read the scalar `f64` in the low lane of XMM register `n`.
fn read_xmm(st: &CpuState, n: u8) -> f64 {
    f64::from_bits(st.xmm[n as usize] as u64)
}

/// Write a scalar `f64` into the low lane of XMM register `n` (high lane kept).
fn write_xmm(st: &mut CpuState, n: u8, v: f64) {
    let hi = st.xmm[n as usize] & (!0u128 << 64);
    st.xmm[n as usize] = hi | (v.to_bits() as u128);
}

/// Read an XMM-or-memory source as an `f64`.
fn read_xmmrm(st: &CpuState, mem: &Memory, src: &XmmRm, next_ip: u64) -> Result<f64, Trap> {
    match src {
        XmmRm::Xmm(n) => Ok(read_xmm(st, *n)),
        XmmRm::Mem(op) => Ok(f64::from_bits(mem.load(effective_addr(st, op, next_ip), 8)?)),
    }
}

/// What the step loop should do after an instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flow {
    /// Fall through to `next_ip`.
    Next,
    /// Set rip to this absolute address.
    Jump(u64),
    /// `call`: `target` is the internal address; `site` is the rel32 patch offset
    /// the step loop checks against the relocation table — if it names an external
    /// symbol a host shim runs, otherwise it's an internal call to `target`.
    Call { target: u64, site: usize },
    /// `ret`: pop the return address into rip.
    Ret,
    /// `ud2`.
    Trap,
}

/// Effective address of a memory operand. `next_ip` is the absolute address of
/// the *following* instruction (RIP-relative base).
fn effective_addr(st: &CpuState, op: &Operand, next_ip: u64) -> u64 {
    match op {
        Operand::Reg(_) => unreachable!("effective_addr on a register operand"),
        Operand::Mem { base, index, scale, disp, rip } => {
            let mut a = if *rip { next_ip } else { 0 };
            if let Some(b) = base { a = a.wrapping_add(st.get(*b)); }
            if let Some(i) = index { a = a.wrapping_add(st.get(*i).wrapping_mul(*scale as u64)); }
            a.wrapping_add(*disp as u64)
        }
    }
}

/// Read an operand as a `width`-byte (1/8) value, zero-extended.
fn read_op(st: &CpuState, mem: &Memory, op: &Operand, next_ip: u64, width: u8) -> Result<u64, Trap> {
    match op {
        Operand::Reg(r) => Ok(if width == 8 { st.get(*r) } else { st.get(*r) & ((1u64 << (8 * width)) - 1) }),
        Operand::Mem { .. } => mem.load(effective_addr(st, op, next_ip), width),
    }
}

/// Write a `width`-byte value to an operand.
fn write_op(st: &mut CpuState, mem: &mut Memory, op: &Operand, next_ip: u64, width: u8, v: u64) -> Result<(), Trap> {
    match op {
        Operand::Reg(r) => { st.set(*r, v); Ok(()) }
        Operand::Mem { .. } => { let a = effective_addr(st, op, next_ip); mem.store(a, width, v) }
    }
}

/// Execute one instruction. `code_base` is where the code region starts in
/// memory (branch targets are code-region offsets); `next_ip` is the absolute
/// address of the following instruction.
pub fn exec_one(
    st: &mut CpuState,
    mem: &mut Memory,
    ins: &Instr,
    code_base: u64,
    next_ip: u64,
    instr_off: usize,
) -> Result<Flow, Trap> {
    match ins {
        Instr::Nop => Ok(Flow::Next),
        Instr::Push(r) => {
            let sp = st.get(Reg::Rsp).wrapping_sub(8);
            mem.store(sp, 8, st.get(*r))?;
            st.set(Reg::Rsp, sp);
            Ok(Flow::Next)
        }
        Instr::Pop(r) => {
            let sp = st.get(Reg::Rsp);
            let v = mem.load(sp, 8)?;
            st.set(*r, v);
            st.set(Reg::Rsp, sp.wrapping_add(8));
            Ok(Flow::Next)
        }
        Instr::Mov { dst, src } => {
            let v = read_op(st, mem, src, next_ip, 8)?;
            write_op(st, mem, dst, next_ip, 8, v)?;
            Ok(Flow::Next)
        }
        Instr::MovImm { dst, imm } => {
            write_op(st, mem, dst, next_ip, 8, *imm as u64)?;
            Ok(Flow::Next)
        }
        Instr::Movzx { dst, src } => {
            let v = read_op(st, mem, src, next_ip, 1)?;
            st.set(*dst, v & 0xFF);
            Ok(Flow::Next)
        }
        Instr::Lea { dst, src } => {
            let a = effective_addr(st, src, next_ip);
            st.set(*dst, a);
            Ok(Flow::Next)
        }
        Instr::Alu { op, dst, src } => {
            let a = read_op(st, mem, dst, next_ip, 8)?;
            let b = read_op(st, mem, src, next_ip, 8)?;
            apply_alu(st, mem, *op, dst, next_ip, a, b)
        }
        Instr::AluImm { op, dst, imm } => {
            let a = read_op(st, mem, dst, next_ip, 8)?;
            apply_alu(st, mem, *op, dst, next_ip, a, *imm as u64)
        }
        Instr::Shift { op, dst, amount } => {
            let a = read_op(st, mem, dst, next_ip, 8)?;
            let s = (*amount as u32) & 63;
            let res = match op {
                ShiftOp::Shl => a.wrapping_shl(s),
                ShiftOp::Shr => a.wrapping_shr(s),
                ShiftOp::Sar => (a as i64).wrapping_shr(s) as u64,
            };
            // Flags after a shift: ZF/SF/PF from the result (CF/OF modelling is
            // not needed by emitted code, which only branches on the value).
            st.flags = logic_flags(res);
            write_op(st, mem, dst, next_ip, 8, res)?;
            Ok(Flow::Next)
        }
        Instr::Imul { dst, src } => {
            let a = st.get(*dst);
            let b = read_op(st, mem, src, next_ip, 8)?;
            st.set(*dst, a.wrapping_mul(b));
            Ok(Flow::Next)
        }
        Instr::Not { dst } => {
            // Bitwise complement; `not` does NOT touch flags (unlike `neg`).
            let v = read_op(st, mem, dst, next_ip, 8)?;
            write_op(st, mem, dst, next_ip, 8, !v)?;
            Ok(Flow::Next)
        }
        Instr::Neg { dst } => {
            // Two's-complement negate == `sub 0, dst`, including its flag effects.
            let v = read_op(st, mem, dst, next_ip, 8)?;
            let (res, flags) = sub_with_flags(0, v);
            st.flags = flags;
            write_op(st, mem, dst, next_ip, 8, res)?;
            Ok(Flow::Next)
        }
        Instr::Cqo => {
            // Sign-extend rax into rdx: rdx becomes all-ones iff rax is negative.
            let hi = if (st.get(Reg::Rax) as i64) < 0 { u64::MAX } else { 0 };
            st.set(Reg::Rdx, hi);
            Ok(Flow::Next)
        }
        Instr::Div { divisor, signed } => {
            let d = read_op(st, mem, divisor, next_ip, 8)?;
            if d == 0 {
                return Err(Trap::DivideError(instr_off as u64));
            }
            let lo = st.get(Reg::Rax);
            let hi = st.get(Reg::Rdx);
            // The dividend is the 128-bit `rdx:rax` pair.
            let dividend = ((hi as u128) << 64) | (lo as u128);
            let (quot, rem) = if *signed {
                // The 128-bit `rdx:rax` pattern interpreted as a signed integer.
                let n = dividend as i128;
                let divisor_i = d as i64 as i128;
                // `checked_div`/`checked_rem` return `None` exactly on the one
                // overflowing case `i128::MIN / -1` (divide-by-zero is already
                // excluded above).  That case is unreachable from real backend
                // output (it always `cqo`s, so `rdx:rax` sign-extends a 64-bit
                // value), but a crafted register state could hit it — so trap
                // rather than panic, keeping the sandbox fail-closed.
                let (q, r) = match (n.checked_div(divisor_i), n.checked_rem(divisor_i)) {
                    (Some(q), Some(r)) => (q, r),
                    _ => return Err(Trap::DivideError(instr_off as u64)),
                };
                // x86 also raises `#DE` when the quotient doesn't fit the 64-bit
                // dest (e.g. `i64::MIN / -1` ⇒ +2^63).
                if q < i64::MIN as i128 || q > i64::MAX as i128 {
                    return Err(Trap::DivideError(instr_off as u64));
                }
                (q as u64, r as u64)
            } else {
                let divisor_u = d as u128;
                let q = dividend / divisor_u;
                let r = dividend % divisor_u;
                // Unsigned `#DE` when the quotient overflows 64 bits.
                if q > u64::MAX as u128 {
                    return Err(Trap::DivideError(instr_off as u64));
                }
                (q as u64, r as u64)
            };
            st.set(Reg::Rax, quot);
            st.set(Reg::Rdx, rem);
            Ok(Flow::Next)
        }
        // ── SSE2 scalar double ────────────────────────────────────────────────
        Instr::MovsdLoad { xmm, src } => {
            let v = f64::from_bits(read_op(st, mem, src, next_ip, 8)?);
            write_xmm(st, *xmm, v);
            Ok(Flow::Next)
        }
        Instr::MovsdStore { dst, xmm } => {
            let bits = read_xmm(st, *xmm).to_bits();
            write_op(st, mem, dst, next_ip, 8, bits)?;
            Ok(Flow::Next)
        }
        Instr::MovsdRr { dst, src } => {
            let v = read_xmm(st, *src);
            write_xmm(st, *dst, v);
            Ok(Flow::Next)
        }
        Instr::SseArith { op, dst, src } => {
            let a = read_xmm(st, *dst);
            let b = read_xmmrm(st, mem, src, next_ip)?;
            let r = match op {
                SseOp::Add => a + b,
                SseOp::Sub => a - b,
                SseOp::Mul => a * b,
                SseOp::Div => a / b,
            };
            write_xmm(st, *dst, r);
            Ok(Flow::Next)
        }
        Instr::Ucomisd { a, b } => {
            let x = read_xmm(st, *a);
            let y = read_xmmrm(st, mem, b, next_ip)?;
            // x86 `ucomisd` sets ZF/PF/CF; OF/SF/AF cleared. Unordered (NaN) sets
            // ZF=PF=CF=1; otherwise PF=0 and ZF/CF encode the ordering.
            st.flags = if x.is_nan() || y.is_nan() {
                Flags { zf: true, pf: true, cf: true, ..Flags::default() }
            } else if x < y {
                Flags { cf: true, ..Flags::default() }
            } else if x > y {
                Flags::default()
            } else {
                Flags { zf: true, ..Flags::default() }
            };
            Ok(Flow::Next)
        }
        Instr::Setcc { cond, dst } => {
            let c = crate::flags::Cond::from_nibble(*cond);
            let bit = crate::flags::condition_holds(c, &st.flags) as u64;
            // setcc writes only the low byte; keep the upper bits (a `movzx`
            // typically follows). For a register dst that means a masked write.
            match dst {
                Operand::Reg(r) => { let v = (st.get(*r) & !0xFF) | bit; st.set(*r, v); }
                Operand::Mem { .. } => write_op(st, mem, dst, next_ip, 1, bit)?,
            }
            Ok(Flow::Next)
        }
        Instr::Jmp(t) => Ok(Flow::Jump(code_base.wrapping_add(*t as u64))),
        Instr::Jcc { cond, rel } => {
            let c = crate::flags::Cond::from_nibble(*cond);
            if crate::flags::condition_holds(c, &st.flags) {
                Ok(Flow::Jump(code_base.wrapping_add(*rel as u64)))
            } else {
                Ok(Flow::Next)
            }
        }
        Instr::Call(t) => {
            // `instr_off` is this call's offset; the rel32 patch site is at +1.
            // The step loop decides internal vs external using that site.
            Ok(Flow::Call { target: code_base.wrapping_add(*t as u64), site: instr_off + 1 })
        }
        Instr::Ret => Ok(Flow::Ret),
        Instr::Ud2 => Ok(Flow::Trap),
    }
}

fn apply_alu(st: &mut CpuState, mem: &mut Memory, op: AluOp, dst: &Operand, next_ip: u64, a: u64, b: u64) -> Result<Flow, Trap> {
    let (res, flags, writes) = match op {
        AluOp::Add => { let (r, f) = add_with_flags(a, b); (r, f, true) }
        AluOp::Sub => { let (r, f) = sub_with_flags(a, b); (r, f, true) }
        AluOp::Cmp => { let (r, f) = sub_with_flags(a, b); (r, f, false) }
        AluOp::And => { let r = a & b; (r, logic_flags(r), true) }
        AluOp::Or => { let r = a | b; (r, logic_flags(r), true) }
        AluOp::Xor => { let r = a ^ b; (r, logic_flags(r), true) }
        AluOp::Test => { let r = a & b; (r, logic_flags(r), false) }
    };
    st.flags = flags;
    if writes {
        write_op(st, mem, dst, next_ip, 8, res)?;
    }
    Ok(Flow::Next)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a single register-only instruction against a fresh CPU and return the
    /// mutated state.  A tiny scratch `Memory` satisfies the signature; these ops
    /// never touch it.
    fn exec(st: &mut CpuState, ins: &Instr) -> Result<Flow, Trap> {
        let mut mem = Memory::new(64, 0, 0);
        exec_one(st, &mut mem, ins, 0, 0, 0)
    }

    #[test]
    fn not_complements_without_touching_flags() {
        let mut st = CpuState::default();
        st.set(Reg::Rax, 0);
        st.flags = Flags { zf: true, cf: true, ..Flags::default() };
        exec(&mut st, &Instr::Not { dst: Operand::Reg(Reg::Rax) }).unwrap();
        assert_eq!(st.get(Reg::Rax), u64::MAX); // ~0
        // `not` must leave flags untouched (unlike `neg`).
        assert!(st.flags.zf && st.flags.cf);
    }

    #[test]
    fn neg_negates_and_sets_flags() {
        let mut st = CpuState::default();
        st.set(Reg::Rax, 5);
        exec(&mut st, &Instr::Neg { dst: Operand::Reg(Reg::Rax) }).unwrap();
        assert_eq!(st.get(Reg::Rax) as i64, -5);
        // neg of a non-zero value sets CF (it is `sub 0, x`).
        assert!(st.flags.cf);
        // neg 0 ⇒ 0, ZF set, CF clear.
        st.set(Reg::Rax, 0);
        exec(&mut st, &Instr::Neg { dst: Operand::Reg(Reg::Rax) }).unwrap();
        assert_eq!(st.get(Reg::Rax), 0);
        assert!(st.flags.zf && !st.flags.cf);
    }

    #[test]
    fn cqo_sign_extends_rax_into_rdx() {
        let mut st = CpuState::default();
        st.set(Reg::Rax, (-7i64) as u64);
        exec(&mut st, &Instr::Cqo).unwrap();
        assert_eq!(st.get(Reg::Rdx), u64::MAX); // negative ⇒ all ones
        st.set(Reg::Rax, 7);
        exec(&mut st, &Instr::Cqo).unwrap();
        assert_eq!(st.get(Reg::Rdx), 0); // non-negative ⇒ zero
    }

    #[test]
    fn unsigned_div_yields_quotient_and_remainder() {
        // rdx:rax = 17, rcx = 5 ⇒ rax=3, rdx=2.
        let mut st = CpuState::default();
        st.set(Reg::Rdx, 0);
        st.set(Reg::Rax, 17);
        st.set(Reg::Rcx, 5);
        exec(&mut st, &Instr::Div { divisor: Operand::Reg(Reg::Rcx), signed: false }).unwrap();
        assert_eq!(st.get(Reg::Rax), 3);
        assert_eq!(st.get(Reg::Rdx), 2);
    }

    #[test]
    fn signed_idiv_handles_negatives() {
        // -17 / 5 ⇒ quotient -3, remainder -2 (truncated toward zero).
        let mut st = CpuState::default();
        st.set(Reg::Rax, (-17i64) as u64);
        exec(&mut st, &Instr::Cqo).unwrap(); // sign-extend into rdx
        st.set(Reg::Rcx, 5);
        exec(&mut st, &Instr::Div { divisor: Operand::Reg(Reg::Rcx), signed: true }).unwrap();
        assert_eq!(st.get(Reg::Rax) as i64, -3);
        assert_eq!(st.get(Reg::Rdx) as i64, -2);
    }

    #[test]
    fn divide_by_zero_traps() {
        let mut st = CpuState::default();
        st.set(Reg::Rax, 1);
        st.set(Reg::Rcx, 0);
        let err = exec(&mut st, &Instr::Div { divisor: Operand::Reg(Reg::Rcx), signed: false }).unwrap_err();
        assert!(matches!(err, Trap::DivideError(_)));
    }

    #[test]
    fn signed_overflow_div_traps() {
        // i64::MIN / -1 overflows the quotient ⇒ #DE, like real hardware.
        let mut st = CpuState::default();
        st.set(Reg::Rax, i64::MIN as u64);
        exec(&mut st, &Instr::Cqo).unwrap();
        st.set(Reg::Rcx, (-1i64) as u64);
        let err = exec(&mut st, &Instr::Div { divisor: Operand::Reg(Reg::Rcx), signed: true }).unwrap_err();
        assert!(matches!(err, Trap::DivideError(_)));
    }

    #[test]
    fn i128_min_dividend_div_neg1_traps_not_panics() {
        // A *crafted* rdx:rax = i128::MIN (rdx top bit set, rax = 0) divided by
        // -1 overflows the i128 division itself — must trap, never panic. (Real
        // backend output can't reach this because it always `cqo`s first; this
        // guards the sandbox against arbitrary register state.)
        let mut st = CpuState::default();
        st.set(Reg::Rdx, 0x8000_0000_0000_0000);
        st.set(Reg::Rax, 0);
        st.set(Reg::Rcx, (-1i64) as u64);
        let err = exec(&mut st, &Instr::Div { divisor: Operand::Reg(Reg::Rcx), signed: true }).unwrap_err();
        assert!(matches!(err, Trap::DivideError(_)));
    }
}
