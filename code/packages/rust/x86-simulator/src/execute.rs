//! Execute one decoded [`Instr`] against the CPU + memory.
//!
//! All register operations are 64-bit. `Cmp`/`Test` set flags without writing a
//! result; the other ALU ops write back. Control-flow instructions return a
//! [`Flow`] the step loop acts on.

use crate::decode::{AluOp, Instr, Operand, ShiftOp};
use crate::flags::{add_with_flags, logic_flags, sub_with_flags};
use crate::state::{CpuState, Reg};
use crate::memory::Memory;
use crate::trap::Trap;

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
