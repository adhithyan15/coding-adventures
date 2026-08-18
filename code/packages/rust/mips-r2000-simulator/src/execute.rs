//! Instruction executor for the MIPS R2000 ISA.
//!
//! # Branch-delay slots — NOT modeled
//!
//! Real MIPS CPUs execute the instruction immediately after a branch/jump
//! (the "delay slot") before the branch takes effect — a side effect of the
//! 5-stage pipeline's instruction fetch.  Matching the Python original
//! (`mips_r2000_simulator/simulator.py`), this executor does **not** model
//! delay slots: branches and jumps take effect immediately.  Programs that
//! rely on delay-slot semantics will not behave correctly here.
//!
//! # Memory byte order — big-endian
//!
//! MIPS R2000's default byte order is big-endian, unlike the
//! `cpu_simulator::Memory::read_word`/`write_word` helpers (which are
//! little-endian, matching RISC-V/ARM/x86).  This module therefore builds
//! its own big-endian word/halfword accessors on top of `Memory`'s
//! endian-agnostic `read_byte`/`write_byte`.
//!
//! # Signed-overflow / divide-by-zero handling
//!
//! The Python original raises `ValueError` on `ADD`/`ADDI`/`SUB` signed
//! 32-bit overflow and on `DIV`/`DIVU` by zero (matching MIPS hardware trap
//! behavior).  This simulator has no exception-propagation channel through
//! `step() -> String`, so it models both as a **fail-closed halt**: the
//! faulting instruction does not write its destination register (or
//! HI/LO), and `halted` is set — mirroring how `riscv-simulator` halts on
//! an invalid checked f64-to-i64 conversion rather than panicking or
//! silently corrupting state.

use cpu_simulator::Memory;
use cpu_simulator::RegisterFile;

use crate::decode::DecodeResult;

/// Result of executing one instruction.
pub struct ExecuteResult {
    pub next_pc: i32,
    pub halted: bool,
}

fn get_field(decoded: &DecodeResult, name: &str) -> i32 {
    decoded.fields.get(name).copied().unwrap_or(0)
}

fn write_rd(regs: &mut RegisterFile, rd: i32, value: u32) {
    if rd != 0 {
        regs.write(rd as usize, value);
    }
}

// ===========================================================================
// Big-endian memory accessors
// ===========================================================================

pub(crate) fn read_word_be(mem: &Memory, addr: usize) -> u32 {
    let b0 = mem.read_byte(addr) as u32;
    let b1 = mem.read_byte(addr + 1) as u32;
    let b2 = mem.read_byte(addr + 2) as u32;
    let b3 = mem.read_byte(addr + 3) as u32;
    (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}

fn write_word_be(mem: &mut Memory, addr: usize, value: u32) {
    mem.write_byte(addr, (value >> 24) as u8);
    mem.write_byte(addr + 1, (value >> 16) as u8);
    mem.write_byte(addr + 2, (value >> 8) as u8);
    mem.write_byte(addr + 3, value as u8);
}

fn read_half_be(mem: &Memory, addr: usize) -> u16 {
    let hi = mem.read_byte(addr) as u16;
    let lo = mem.read_byte(addr + 1) as u16;
    (hi << 8) | lo
}

fn write_half_be(mem: &mut Memory, addr: usize, value: u16) {
    mem.write_byte(addr, (value >> 8) as u8);
    mem.write_byte(addr + 1, value as u8);
}

// ===========================================================================
// Dispatch
// ===========================================================================

/// Execute one decoded instruction.
///
/// `pc` is the address of the instruction being executed (not yet
/// advanced).  `hi`/`lo` are the special HI/LO registers used by
/// MULT/MULTU/DIV/DIVU and MFHI/MTHI/MFLO/MTLO.
#[allow(clippy::too_many_lines)]
pub fn execute(
    decoded: &DecodeResult,
    regs: &mut RegisterFile,
    mem: &mut Memory,
    hi: &mut u32,
    lo: &mut u32,
    pc: i32,
) -> ExecuteResult {
    match decoded.mnemonic.as_str() {
        "syscall" => ExecuteResult {
            next_pc: pc,
            halted: true,
        },
        "break" => ExecuteResult {
            next_pc: pc,
            halted: true,
        },

        // R-type shifts
        "sll" => exec_shift(decoded, regs, pc, |v, s| v << s),
        "srl" => exec_shift(decoded, regs, pc, |v, s| v >> s),
        "sra" => exec_shift(decoded, regs, pc, |v, s| ((v as i32) >> s) as u32),
        "sllv" => exec_shift_v(decoded, regs, pc, |v, s| v << (s & 31)),
        "srlv" => exec_shift_v(decoded, regs, pc, |v, s| v >> (s & 31)),
        "srav" => exec_shift_v(decoded, regs, pc, |v, s| ((v as i32) >> (s & 31)) as u32),

        // R-type jumps
        "jr" => {
            let rs = get_field(decoded, "rs");
            ExecuteResult {
                next_pc: regs.read(rs as usize) as i32,
                halted: false,
            }
        }
        "jalr" => {
            let rs = get_field(decoded, "rs");
            let rd = get_field(decoded, "rd");
            let target = regs.read(rs as usize) as i32;
            write_rd(regs, rd, (pc + 4) as u32);
            ExecuteResult {
                next_pc: target,
                halted: false,
            }
        }

        // HI/LO moves
        "mfhi" => {
            let rd = get_field(decoded, "rd");
            write_rd(regs, rd, *hi);
            ExecuteResult { next_pc: pc + 4, halted: false }
        }
        "mthi" => {
            let rs = get_field(decoded, "rs");
            *hi = regs.read(rs as usize);
            ExecuteResult { next_pc: pc + 4, halted: false }
        }
        "mflo" => {
            let rd = get_field(decoded, "rd");
            write_rd(regs, rd, *lo);
            ExecuteResult { next_pc: pc + 4, halted: false }
        }
        "mtlo" => {
            let rs = get_field(decoded, "rs");
            *lo = regs.read(rs as usize);
            ExecuteResult { next_pc: pc + 4, halted: false }
        }

        // Multiply / divide
        "mult" => {
            let rs = get_field(decoded, "rs");
            let rt = get_field(decoded, "rt");
            let a = regs.read(rs as usize) as i32 as i64;
            let b = regs.read(rt as usize) as i32 as i64;
            let product = (a * b) as u64;
            *lo = product as u32;
            *hi = (product >> 32) as u32;
            ExecuteResult { next_pc: pc + 4, halted: false }
        }
        "multu" => {
            let rs = get_field(decoded, "rs");
            let rt = get_field(decoded, "rt");
            let product = regs.read(rs as usize) as u64 * regs.read(rt as usize) as u64;
            *lo = product as u32;
            *hi = (product >> 32) as u32;
            ExecuteResult { next_pc: pc + 4, halted: false }
        }
        "div" => {
            let rs = get_field(decoded, "rs");
            let rt = get_field(decoded, "rt");
            let divisor = regs.read(rt as usize) as i32;
            if divisor == 0 {
                return ExecuteResult { next_pc: pc, halted: true };
            }
            let dividend = regs.read(rs as usize) as i32;
            // MIPS DIV truncates toward zero (Rust's `/` on signed ints
            // already truncates toward zero, matching the ISA directly —
            // unlike the Python original, which special-cased this
            // because Python's `//` floors).
            let (q, r) = if dividend == i32::MIN && divisor == -1 {
                (i32::MIN, 0)
            } else {
                (dividend / divisor, dividend % divisor)
            };
            *lo = q as u32;
            *hi = r as u32;
            ExecuteResult { next_pc: pc + 4, halted: false }
        }
        "divu" => {
            let rs = get_field(decoded, "rs");
            let rt = get_field(decoded, "rt");
            let divisor = regs.read(rt as usize);
            if divisor == 0 {
                return ExecuteResult { next_pc: pc, halted: true };
            }
            let dividend = regs.read(rs as usize);
            *lo = dividend / divisor;
            *hi = dividend % divisor;
            ExecuteResult { next_pc: pc + 4, halted: false }
        }

        // R-type ALU
        "add" => exec_checked_add(decoded, regs, pc, false),
        "sub" => exec_checked_add(decoded, regs, pc, true),
        "addu" => exec_reg_arith(decoded, regs, pc, |a, b| a.wrapping_add(b)),
        "subu" => exec_reg_arith(decoded, regs, pc, |a, b| a.wrapping_sub(b)),
        "and" => exec_reg_arith(decoded, regs, pc, |a, b| a & b),
        "or" => exec_reg_arith(decoded, regs, pc, |a, b| a | b),
        "xor" => exec_reg_arith(decoded, regs, pc, |a, b| a ^ b),
        "nor" => exec_reg_arith(decoded, regs, pc, |a, b| !(a | b)),
        "slt" => exec_reg_arith(decoded, regs, pc, |a, b| {
            u32::from((a as i32) < (b as i32))
        }),
        "sltu" => exec_reg_arith(decoded, regs, pc, |a, b| u32::from(a < b)),

        // I-type branches
        "beq" => exec_branch(decoded, regs, pc, |a, b| a == b),
        "bne" => exec_branch(decoded, regs, pc, |a, b| a != b),
        "blez" => exec_branch1(decoded, regs, pc, |a| (a as i32) <= 0),
        "bgtz" => exec_branch1(decoded, regs, pc, |a| (a as i32) > 0),
        "bltz" => exec_branch1(decoded, regs, pc, |a| (a as i32) < 0),
        "bgez" => exec_branch1(decoded, regs, pc, |a| (a as i32) >= 0),
        "bltzal" => exec_branch_and_link(decoded, regs, pc, |a| (a as i32) < 0),
        "bgezal" => exec_branch_and_link(decoded, regs, pc, |a| (a as i32) >= 0),

        // J-type jumps
        "j" => {
            let target = get_field(decoded, "target") as u32;
            ExecuteResult {
                next_pc: ((((pc + 4) as u32) & 0xF000_0000) | (target << 2)) as i32,
                halted: false,
            }
        }
        "jal" => {
            let target = get_field(decoded, "target") as u32;
            write_rd(regs, 31, (pc + 4) as u32);
            ExecuteResult {
                next_pc: ((((pc + 4) as u32) & 0xF000_0000) | (target << 2)) as i32,
                halted: false,
            }
        }

        // I-type arithmetic / logic
        "addi" => exec_checked_addi(decoded, regs, pc),
        "addiu" => exec_imm_arith(decoded, regs, pc, |a, b| (a.wrapping_add(b)) as u32),
        "slti" => exec_imm_arith(decoded, regs, pc, |a, b| u32::from(a < b)),
        "sltiu" => exec_imm_arith(decoded, regs, pc, |a, b| u32::from((a as u32) < (b as u32))),
        "andi" => exec_imm_logic(decoded, regs, pc, |a, b| a & b),
        "ori" => exec_imm_logic(decoded, regs, pc, |a, b| a | b),
        "xori" => exec_imm_logic(decoded, regs, pc, |a, b| a ^ b),
        "lui" => {
            let rt = get_field(decoded, "rt");
            let imm = get_field(decoded, "imm") as u32 & 0xFFFF;
            write_rd(regs, rt, imm << 16);
            ExecuteResult { next_pc: pc + 4, halted: false }
        }

        // Loads
        "lb" | "lh" | "lw" | "lbu" | "lhu" => exec_load(decoded, regs, mem, pc),
        // Stores
        "sb" | "sh" | "sw" => exec_store(decoded, regs, mem, pc),

        _ => ExecuteResult { next_pc: pc + 4, halted: false },
    }
}

fn exec_shift(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(u32, u32) -> u32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rt = get_field(d, "rt");
    let shamt = get_field(d, "shamt") as u32;
    let val = regs.read(rt as usize);
    write_rd(regs, rd, op(val, shamt));
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_shift_v(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(u32, u32) -> u32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rt = get_field(d, "rt");
    let rs = get_field(d, "rs");
    let val = regs.read(rt as usize);
    let shamt = regs.read(rs as usize);
    write_rd(regs, rd, op(val, shamt));
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_reg_arith(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(u32, u32) -> u32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs = get_field(d, "rs");
    let rt = get_field(d, "rt");
    let result = op(regs.read(rs as usize), regs.read(rt as usize));
    write_rd(regs, rd, result);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

/// `ADD`/`SUB` — signed add/subtract that fails closed (halts, does not
/// write `rd`) on 32-bit signed overflow, matching the MIPS hardware trap
/// (the Python original raises `ValueError`).
fn exec_checked_add(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, subtract: bool) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs = get_field(d, "rs");
    let rt = get_field(d, "rt");
    let a = regs.read(rs as usize) as i32 as i64;
    let b = regs.read(rt as usize) as i32 as i64;
    let sum = if subtract { a - b } else { a + b };
    if !(i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&sum) {
        return ExecuteResult { next_pc: pc, halted: true };
    }
    write_rd(regs, rd, sum as i32 as u32);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

/// `ADDI` — signed add-immediate that fails closed on 32-bit signed
/// overflow, matching the MIPS hardware trap.
fn exec_checked_addi(d: &DecodeResult, regs: &mut RegisterFile, pc: i32) -> ExecuteResult {
    let rt = get_field(d, "rt");
    let rs = get_field(d, "rs");
    let imm = get_field(d, "imm") as i64;
    let a = regs.read(rs as usize) as i32 as i64;
    let sum = a + imm;
    if !(i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&sum) {
        return ExecuteResult { next_pc: pc, halted: true };
    }
    write_rd(regs, rt, sum as i32 as u32);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_imm_arith(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(i32, i32) -> u32) -> ExecuteResult {
    let rt = get_field(d, "rt");
    let rs = get_field(d, "rs");
    let imm = get_field(d, "imm");
    let rs_val = regs.read(rs as usize) as i32;
    write_rd(regs, rt, op(rs_val, imm));
    ExecuteResult { next_pc: pc + 4, halted: false }
}

/// `ANDI`/`ORI`/`XORI` use the raw **zero-extended** 16-bit immediate, not
/// the sign-extended value `decode` stores in `fields["imm"]`.  Masking a
/// sign-extended `i32` back to 16 bits recovers the original unsigned bits.
fn exec_imm_logic(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(u32, u32) -> u32) -> ExecuteResult {
    let rt = get_field(d, "rt");
    let rs = get_field(d, "rs");
    let imm16 = get_field(d, "imm") as u32 & 0xFFFF;
    let rs_val = regs.read(rs as usize);
    write_rd(regs, rt, op(rs_val, imm16));
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_load(d: &DecodeResult, regs: &mut RegisterFile, mem: &Memory, pc: i32) -> ExecuteResult {
    let rt = get_field(d, "rt");
    let rs = get_field(d, "rs");
    let imm = get_field(d, "imm");
    let addr = (regs.read(rs as usize) as i32).wrapping_add(imm) as usize;

    let result = match d.mnemonic.as_str() {
        "lb" => {
            let b = mem.read_byte(addr);
            (b as i8) as i32 as u32
        }
        "lh" => {
            let half = read_half_be(mem, addr);
            (half as i16) as i32 as u32
        }
        "lw" => read_word_be(mem, addr),
        "lbu" => mem.read_byte(addr) as u32,
        "lhu" => read_half_be(mem, addr) as u32,
        _ => 0,
    };

    write_rd(regs, rt, result);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_store(d: &DecodeResult, regs: &mut RegisterFile, mem: &mut Memory, pc: i32) -> ExecuteResult {
    let rt = get_field(d, "rt");
    let rs = get_field(d, "rs");
    let imm = get_field(d, "imm");
    let addr = (regs.read(rs as usize) as i32).wrapping_add(imm) as usize;
    let val = regs.read(rt as usize);

    match d.mnemonic.as_str() {
        "sb" => mem.write_byte(addr, (val & 0xFF) as u8),
        "sh" => write_half_be(mem, addr, (val & 0xFFFF) as u16),
        "sw" => write_word_be(mem, addr, val),
        _ => {}
    }

    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_branch(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, cond: fn(u32, u32) -> bool) -> ExecuteResult {
    let rs = get_field(d, "rs");
    let rt = get_field(d, "rt");
    let imm = get_field(d, "imm");
    let taken = cond(regs.read(rs as usize), regs.read(rt as usize));
    let next_pc = if taken { pc + 4 + (imm << 2) } else { pc + 4 };
    ExecuteResult { next_pc, halted: false }
}

fn exec_branch1(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, cond: fn(u32) -> bool) -> ExecuteResult {
    let rs = get_field(d, "rs");
    let imm = get_field(d, "imm");
    let taken = cond(regs.read(rs as usize));
    let next_pc = if taken { pc + 4 + (imm << 2) } else { pc + 4 };
    ExecuteResult { next_pc, halted: false }
}

fn exec_branch_and_link(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, cond: fn(u32) -> bool) -> ExecuteResult {
    let rs = get_field(d, "rs");
    let imm = get_field(d, "imm");
    write_rd(regs, 31, (pc + 4) as u32);
    let taken = cond(regs.read(rs as usize));
    let next_pc = if taken { pc + 4 + (imm << 2) } else { pc + 4 };
    ExecuteResult { next_pc, halted: false }
}
