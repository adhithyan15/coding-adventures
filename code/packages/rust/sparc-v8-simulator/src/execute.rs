//! Instruction executor for the SPARC V8 ISA.
//!
//! Straight transcription of
//! `sparc_v8_simulator.simulator.SPARCSimulator._execute_one` and its
//! `_exec_*` helpers (Python), cross-checked against
//! `sparc-v8-gatelevel` where the two overlap (register-window
//! rotation).
//!
//! # Branch/CALL displacement is relative to the instruction's *own* PC
//!
//! Unlike `mips-r2000-simulator` (whose branch target is
//! `pc_of_branch + 4 + offset*4`, MIPS's delay-slot-shaped convention),
//! SPARC V8 branch/CALL targets are `pc_of_instr + disp*4` — relative to
//! the control-transfer instruction's own address, with no `+4`.  This
//! matches the Python original and real SPARC V8 silicon exactly (SPARC
//! branch displacement fields *are* delay-slot-shaped on real hardware
//! too, but this simulator does not execute the delay slot, so the
//! effective formula collapses to the simpler `pc + disp*4`).
//!
//! # HALT (`ta 0`) still advances PC by 4
//!
//! Unlike `mips-r2000-simulator`'s `SYSCALL` (which reports
//! `next_pc == pc`, i.e. unchanged), the Python SPARC original's fetch
//! pipeline (`_fetch32`) advances `self._pc` to the next sequential
//! instruction *before* dispatching on the decoded opcode — so by the
//! time the `ta 0` / `Ticc cond=TA` handler sets `halted = True` and
//! returns, `self._pc` has already moved to `pc_of_instr + 4`.  This
//! executor mirrors that: the `"ta"` arm below returns
//! `next_pc = pc + 4`, not `next_pc = pc`.
//!
//! # Branch-delay slots — NOT modeled
//!
//! Matching the Python original, this executor does not model SPARC's
//! branch-delay slot: branches/calls/JMPL take effect immediately, with
//! no execution of the following instruction first.
//!
//! # Memory byte order — big-endian
//!
//! Matches MIPS R2000 (and unlike RISC-V/ARM/x86): this module builds
//! its own big-endian word/halfword accessors on top of
//! `cpu_simulator::Memory`'s endian-agnostic `read_byte`/`write_byte`.
//!
//! # Fault handling — fail-closed halt, not exceptions
//!
//! The Python original raises `ValueError` on `UDIV`/`SDIV` by zero,
//! register-window overflow (`SAVE` past `NWINDOWS - 1` nesting), and
//! any `Ticc` trap condition other than "always" (untrapped conditional
//! traps are not modeled).  This executor has no exception channel
//! through `execute() -> ExecuteResult`, so — mirroring
//! `mips-r2000-simulator`'s treatment of signed-overflow `ADD`/`ADDI`/
//! `SUB` and `DIV`/`DIVU`-by-zero — all three fault the same way: the
//! faulting instruction does not write its destination register (or
//! `Y`/CWP), and the simulator halts.

use cpu_simulator::Memory;

use crate::decode::DecodeResult;
use crate::opcodes::*;
use crate::registers::RegisterWindowFile;

/// PSR (Processor Status Register) condition-code bits this simulator
/// tracks.  `cwp` lives on [`RegisterWindowFile`] instead, since it
/// governs register addressing rather than condition evaluation.
#[derive(Debug, Clone, Copy, Default)]
pub struct Psr {
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
}

/// Result of executing one instruction.
pub struct ExecuteResult {
    pub next_pc: i32,
    pub halted: bool,
}

fn get_field(d: &DecodeResult, name: &str) -> i32 {
    d.fields.get(name).copied().unwrap_or(0)
}

/// SPARC V8 Format 3's second ALU operand: `rs2` (register) when `i=0`,
/// or the sign-extended 13-bit immediate when `i=1`.
fn operand2(d: &DecodeResult, regs: &RegisterWindowFile) -> u32 {
    if get_field(d, "i") == 1 {
        get_field(d, "simm13") as u32
    } else {
        regs.read(get_field(d, "rs2") as u32)
    }
}

// ===========================================================================
// Big-endian memory accessors
// ===========================================================================

fn read_word_be(mem: &Memory, addr: usize) -> u32 {
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

/// Fetch one big-endian 32-bit instruction word from memory at `addr`.
/// Exposed for [`crate::simulator`]'s fetch stage.
pub fn fetch_word_be(mem: &Memory, addr: usize) -> u32 {
    read_word_be(mem, addr)
}

// ===========================================================================
// Condition-code updates (SPARC V8 manual §5.7)
// ===========================================================================

fn update_cc_add(a: u32, b: u32, result: u32) -> Psr {
    Psr {
        n: (result >> 31) & 1 != 0,
        z: result == 0,
        v: (((!a & !b & result) | (a & b & !result)) >> 31) & 1 != 0,
        c: (u64::from(a) + u64::from(b)) > 0xFFFF_FFFF,
    }
}

fn update_cc_sub(a: u32, b: u32, result: u32) -> Psr {
    Psr {
        n: (result >> 31) & 1 != 0,
        z: result == 0,
        v: (((a & !b & !result) | (!a & b & result)) >> 31) & 1 != 0,
        c: a < b,
    }
}

fn update_cc_logic(result: u32) -> Psr {
    Psr {
        n: (result >> 31) & 1 != 0,
        z: result == 0,
        v: false,
        c: false,
    }
}

/// Evaluate a Bicc/Ticc condition-code field (SPARC V8 manual §A.7).
fn branch_taken(cond: u32, psr: &Psr) -> bool {
    let (n, z, v, c) = (psr.n, psr.z, psr.v, psr.c);
    match cond & 0xF {
        COND_BA => true,
        COND_BN => false,
        COND_BNE => !z,
        COND_BE => z,
        COND_BG => !z && (n == v),
        COND_BLE => z || (n != v),
        COND_BGE => n == v,
        COND_BL => n != v,
        COND_BGU => !c && !z,
        COND_BLEU => c || z,
        COND_BCC => !c,
        COND_BCS => c,
        COND_BPOS => !n,
        COND_BNEG => n,
        COND_BVC => !v,
        COND_BVS => v,
        _ => false,
    }
}

// ===========================================================================
// Dispatch
// ===========================================================================

/// Execute one decoded instruction.
///
/// `pc` is the address of the instruction being executed (not yet
/// advanced).  `regs` holds the windowed register file; `psr`/`y` are
/// the condition-code flags and the multiply/divide auxiliary register.
#[allow(clippy::too_many_lines)]
pub fn execute(
    decoded: &DecodeResult,
    regs: &mut RegisterWindowFile,
    mem: &mut Memory,
    psr: &mut Psr,
    y: &mut u32,
    pc: i32,
) -> ExecuteResult {
    let seq = ExecuteResult {
        next_pc: pc + 4,
        halted: false,
    };
    let fault = ExecuteResult {
        next_pc: pc + 4,
        halted: true,
    };

    match decoded.mnemonic.as_str() {
        // ── HALT ──────────────────────────────────────────────────────
        "ta" => ExecuteResult {
            next_pc: pc + 4,
            halted: true,
        },
        "nop" => seq,

        // ── Format 1: CALL ────────────────────────────────────────────
        "call" => {
            let disp30 = get_field(decoded, "disp30");
            regs.write(15, pc as u32); // %o7 = pc_of_instr
            ExecuteResult {
                next_pc: pc.wrapping_add(disp30.wrapping_mul(4)),
                halted: false,
            }
        }

        // ── Format 2: SETHI ───────────────────────────────────────────
        "sethi" => {
            let rd = get_field(decoded, "rd") as u32;
            let imm22 = get_field(decoded, "imm22") as u32;
            regs.write(rd, imm22 << 10);
            seq
        }

        // ── Format 2: Bicc ────────────────────────────────────────────
        "ba" | "bn" | "bne" | "be" | "bg" | "ble" | "bge" | "bl" | "bgu" | "bleu" | "bcc"
        | "bcs" | "bpos" | "bneg" | "bvc" | "bvs" => {
            let cond = get_field(decoded, "cond") as u32;
            let disp22 = get_field(decoded, "disp22");
            if branch_taken(cond, psr) {
                ExecuteResult {
                    next_pc: pc.wrapping_add(disp22.wrapping_mul(4)),
                    halted: false,
                }
            } else {
                seq
            }
        }

        // ── ADD family ────────────────────────────────────────────────
        "add" | "addcc" => {
            let (a, src) = ab(decoded, regs);
            let result = a.wrapping_add(src);
            if decoded.mnemonic == "addcc" {
                *psr = update_cc_add(a, src, result);
            }
            write_rd(decoded, regs, result);
            seq
        }
        "addx" | "addxcc" => {
            let (a, src) = ab(decoded, regs);
            let c_in = u32::from(psr.c);
            let result = a.wrapping_add(src).wrapping_add(c_in);
            if decoded.mnemonic == "addxcc" {
                *psr = update_cc_add(a, src.wrapping_add(c_in), result);
            }
            write_rd(decoded, regs, result);
            seq
        }

        // ── SUB family ────────────────────────────────────────────────
        "sub" | "subcc" => {
            let (a, src) = ab(decoded, regs);
            let result = a.wrapping_sub(src);
            if decoded.mnemonic == "subcc" {
                *psr = update_cc_sub(a, src, result);
            }
            write_rd(decoded, regs, result);
            seq
        }
        "subx" | "subxcc" => {
            let (a, src) = ab(decoded, regs);
            let c_in = u32::from(psr.c);
            let result = a.wrapping_sub(src).wrapping_sub(c_in);
            if decoded.mnemonic == "subxcc" {
                *psr = update_cc_sub(a, src.wrapping_add(c_in), result);
            }
            write_rd(decoded, regs, result);
            seq
        }

        // ── Logic family ──────────────────────────────────────────────
        "and" | "andcc" => {
            logic_op(decoded, regs, psr, |a, b| a & b, decoded.mnemonic == "andcc");
            seq
        }
        "andn" | "andncc" => {
            logic_op(decoded, regs, psr, |a, b| a & !b, decoded.mnemonic == "andncc");
            seq
        }
        "or" | "orcc" => {
            logic_op(decoded, regs, psr, |a, b| a | b, decoded.mnemonic == "orcc");
            seq
        }
        "orn" | "orncc" => {
            logic_op(decoded, regs, psr, |a, b| a | !b, decoded.mnemonic == "orncc");
            seq
        }
        "xor" | "xorcc" => {
            logic_op(decoded, regs, psr, |a, b| a ^ b, decoded.mnemonic == "xorcc");
            seq
        }
        "xnor" | "xnorcc" => {
            logic_op(decoded, regs, psr, |a, b| !(a ^ b), decoded.mnemonic == "xnorcc");
            seq
        }

        // ── Shifts ────────────────────────────────────────────────────
        "sll" => {
            let (a, src) = ab(decoded, regs);
            write_rd(decoded, regs, a.wrapping_shl(src & 31));
            seq
        }
        "srl" => {
            let (a, src) = ab(decoded, regs);
            write_rd(decoded, regs, a.wrapping_shr(src & 31));
            seq
        }
        "sra" => {
            let (a, src) = ab(decoded, regs);
            write_rd(decoded, regs, ((a as i32).wrapping_shr(src & 31)) as u32);
            seq
        }

        // ── Multiply ──────────────────────────────────────────────────
        "umul" | "umulcc" => {
            let (a, src) = ab(decoded, regs);
            let product = u64::from(a) * u64::from(src);
            *y = (product >> 32) as u32;
            let result = product as u32;
            if decoded.mnemonic == "umulcc" {
                *psr = update_cc_logic(result);
            }
            write_rd(decoded, regs, result);
            seq
        }
        "smul" | "smulcc" => {
            let (a, src) = ab(decoded, regs);
            let product = i64::from(a as i32) * i64::from(src as i32);
            let product64 = product as u64;
            *y = (product64 >> 32) as u32;
            let result = product64 as u32;
            if decoded.mnemonic == "smulcc" {
                *psr = update_cc_logic(result);
            }
            write_rd(decoded, regs, result);
            seq
        }

        // ── Divide (64÷32 -> 32) ──────────────────────────────────────
        "udiv" | "udivcc" => {
            let (a, src) = ab(decoded, regs);
            if src == 0 {
                return fault;
            }
            let dividend = (u64::from(*y) << 32) | u64::from(a);
            let q = (dividend / u64::from(src)).min(u64::from(u32::MAX));
            let result = q as u32;
            if decoded.mnemonic == "udivcc" {
                *psr = update_cc_logic(result);
            }
            write_rd(decoded, regs, result);
            seq
        }
        "sdiv" | "sdivcc" => {
            let (a, src) = ab(decoded, regs);
            if src == 0 {
                return fault;
            }
            let dividend = (i64::from(*y as i32) << 32) | i64::from(a);
            let divisor = src as i32 as i64;
            let mut q = dividend / divisor; // truncates toward zero, matching SPARC/Rust `/`
            q = q.clamp(i64::from(i32::MIN), i64::from(i32::MAX));
            let result = q as i32 as u32;
            if decoded.mnemonic == "sdivcc" {
                *psr = update_cc_logic(result);
            }
            write_rd(decoded, regs, result);
            seq
        }

        // ── MULScc (one step of restoring multiply) ──────────────────
        "mulscc" => {
            let rd = get_field(decoded, "rd") as u32;
            let (a, src) = ab(decoded, regs);
            let y_lsb = *y & 1;
            let add: u32 = if psr.n != psr.v { a } else { 0 };
            let shifted = (src >> 1) | (y_lsb << 31);
            let result = shifted.wrapping_add(add);
            *psr = update_cc_add(shifted, add, result);
            let old_rd = regs.read(rd);
            *y = (old_rd >> 1) | ((result & 1) << 31);
            regs.write(rd, result);
            seq
        }

        // ── Y register ────────────────────────────────────────────────
        "wry" => {
            let (a, src) = ab(decoded, regs);
            *y = a ^ src;
            seq
        }
        "rdy" => {
            let rd = get_field(decoded, "rd") as u32;
            regs.write(rd, *y);
            seq
        }

        // ── JMPL ──────────────────────────────────────────────────────
        "jmpl" => {
            let rd = get_field(decoded, "rd") as u32;
            let (a, src) = ab(decoded, regs);
            let target = a.wrapping_add(src);
            regs.write(rd, pc as u32);
            ExecuteResult {
                next_pc: target as i32,
                halted: false,
            }
        }

        // ── SAVE / RESTORE (register-window rotation) ────────────────
        "save" => {
            let rd = get_field(decoded, "rd") as u32;
            let (a, src) = ab(decoded, regs);
            let result = a.wrapping_add(src); // computed in the CALLER's window
            if regs.rotate_save().is_err() {
                return fault; // window overflow: fail closed, no rotation applied
            }
            regs.write(rd, result); // written into the NEW (callee) window
            seq
        }
        "restore" => {
            let rd = get_field(decoded, "rd") as u32;
            let (a, src) = ab(decoded, regs);
            let result = a.wrapping_add(src); // computed in the CALLEE's window
            regs.rotate_restore();
            regs.write(rd, result); // written into the NEW (caller) window
            seq
        }

        // ── Ticc: trap on integer condition ──────────────────────────
        "ticc" => {
            let cond = get_field(decoded, "cond") as u32;
            if cond == COND_BA {
                ExecuteResult {
                    next_pc: pc + 4,
                    halted: true,
                }
            } else {
                // Conditional/non-halt traps are not modeled -- fail closed.
                fault
            }
        }

        // ── Loads ─────────────────────────────────────────────────────
        "ld" | "ldub" | "lduh" | "ldsb" | "ldsh" => exec_load(decoded, regs, mem, seq),
        // ── Stores ────────────────────────────────────────────────────
        "st" | "stb" | "sth" => exec_store(decoded, regs, mem, seq),

        // Unknown/unhandled opcodes are treated as a silent no-op (same
        // fallback `mips-r2000-simulator::execute` uses for its
        // `UNKNOWN(op=...)` decode results).
        _ => seq,
    }
}

/// Read `rs1` and the second ALU operand for a Format-3 instruction.
fn ab(d: &DecodeResult, regs: &RegisterWindowFile) -> (u32, u32) {
    let rs1 = get_field(d, "rs1") as u32;
    (regs.read(rs1), operand2(d, regs))
}

fn write_rd(d: &DecodeResult, regs: &mut RegisterWindowFile, value: u32) {
    let rd = get_field(d, "rd") as u32;
    regs.write(rd, value);
}

fn logic_op(
    d: &DecodeResult,
    regs: &mut RegisterWindowFile,
    psr: &mut Psr,
    op: fn(u32, u32) -> u32,
    update_cc: bool,
) {
    let (a, src) = ab(d, regs);
    let result = op(a, src);
    if update_cc {
        *psr = update_cc_logic(result);
    }
    write_rd(d, regs, result);
}

fn exec_load(d: &DecodeResult, regs: &mut RegisterWindowFile, mem: &Memory, seq: ExecuteResult) -> ExecuteResult {
    let rd = get_field(d, "rd") as u32;
    let (base, off) = ab(d, regs);
    let ea = base.wrapping_add(off) as usize;

    let result = match d.mnemonic.as_str() {
        "ld" => read_word_be(mem, ea),
        "ldub" => mem.read_byte(ea) as u32,
        "ldsb" => {
            let b = mem.read_byte(ea);
            (b as i8) as i32 as u32
        }
        "lduh" => read_half_be(mem, ea) as u32,
        "ldsh" => {
            let h = read_half_be(mem, ea);
            (h as i16) as i32 as u32
        }
        _ => 0,
    };
    regs.write(rd, result);
    seq
}

fn exec_store(d: &DecodeResult, regs: &RegisterWindowFile, mem: &mut Memory, seq: ExecuteResult) -> ExecuteResult {
    let rd = get_field(d, "rd") as u32;
    let (base, off) = ab(d, regs);
    let ea = base.wrapping_add(off) as usize;
    let val = regs.read(rd);

    match d.mnemonic.as_str() {
        "st" => write_word_be(mem, ea, val),
        "stb" => mem.write_byte(ea, (val & 0xFF) as u8),
        "sth" => write_half_be(mem, ea, (val & 0xFFFF) as u16),
        _ => {}
    }
    seq
}
