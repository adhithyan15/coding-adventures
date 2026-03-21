//! Instruction executor for all RV32I + M-mode instructions.

use cpu_simulator::{RegisterFile, Memory};
use crate::csr::*;
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

pub fn execute(
    decoded: &DecodeResult,
    regs: &mut RegisterFile,
    mem: &mut Memory,
    csr: &mut CSRFile,
    pc: i32,
) -> ExecuteResult {
    match decoded.mnemonic.as_str() {
        // I-type arithmetic
        "addi"  => exec_imm_arith(decoded, regs, pc, |a, b| (a.wrapping_add(b)) as u32),
        "slti"  => exec_imm_arith(decoded, regs, pc, |a, b| if a < b { 1 } else { 0 }),
        "sltiu" => exec_imm_arith(decoded, regs, pc, |a, b| if (a as u32) < (b as u32) { 1 } else { 0 }),
        "xori"  => exec_imm_arith(decoded, regs, pc, |a, b| (a ^ b) as u32),
        "ori"   => exec_imm_arith(decoded, regs, pc, |a, b| (a | b) as u32),
        "andi"  => exec_imm_arith(decoded, regs, pc, |a, b| (a & b) as u32),
        "slli"  => exec_shift_imm(decoded, regs, pc, |v, s| v << s),
        "srli"  => exec_shift_imm(decoded, regs, pc, |v, s| v >> s),
        "srai"  => exec_shift_imm(decoded, regs, pc, |v, s| ((v as i32) >> s) as u32),
        // R-type arithmetic
        "add"   => exec_reg_arith(decoded, regs, pc, |a, b| (a as i32).wrapping_add(b as i32) as u32),
        "sub"   => exec_reg_arith(decoded, regs, pc, |a, b| (a as i32).wrapping_sub(b as i32) as u32),
        "sll"   => exec_reg_arith(decoded, regs, pc, |a, b| a << (b & 0x1F)),
        "slt"   => exec_reg_arith(decoded, regs, pc, |a, b| if (a as i32) < (b as i32) { 1 } else { 0 }),
        "sltu"  => exec_reg_arith(decoded, regs, pc, |a, b| if a < b { 1 } else { 0 }),
        "xor"   => exec_reg_arith(decoded, regs, pc, |a, b| a ^ b),
        "srl"   => exec_reg_arith(decoded, regs, pc, |a, b| a >> (b & 0x1F)),
        "sra"   => exec_reg_arith(decoded, regs, pc, |a, b| ((a as i32) >> (b & 0x1F)) as u32),
        "or"    => exec_reg_arith(decoded, regs, pc, |a, b| a | b),
        "and"   => exec_reg_arith(decoded, regs, pc, |a, b| a & b),
        // Loads
        "lb" | "lh" | "lw" | "lbu" | "lhu" => exec_load(decoded, regs, mem, pc),
        // Stores
        "sb" | "sh" | "sw" => exec_store(decoded, regs, mem, pc),
        // Branches
        "beq"  => exec_branch(decoded, regs, pc, |a, b| a == b),
        "bne"  => exec_branch(decoded, regs, pc, |a, b| a != b),
        "blt"  => exec_branch(decoded, regs, pc, |a, b| (a as i32) < (b as i32)),
        "bge"  => exec_branch(decoded, regs, pc, |a, b| (a as i32) >= (b as i32)),
        "bltu" => exec_branch(decoded, regs, pc, |a, b| a < b),
        "bgeu" => exec_branch(decoded, regs, pc, |a, b| a >= b),
        // Jumps
        "jal"  => exec_jal(decoded, regs, pc),
        "jalr" => exec_jalr(decoded, regs, pc),
        // Upper immediates
        "lui"   => exec_lui(decoded, regs, pc),
        "auipc" => exec_auipc(decoded, regs, pc),
        // System
        "ecall" => exec_ecall(regs, csr, pc),
        "mret"  => exec_mret(csr, pc),
        "csrrw" => exec_csrrw(decoded, regs, csr, pc),
        "csrrs" => exec_csrrs(decoded, regs, csr, pc),
        "csrrc" => exec_csrrc(decoded, regs, csr, pc),
        _ => ExecuteResult { next_pc: pc + 4, halted: false },
    }
}

fn exec_imm_arith(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(i32, i32) -> u32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let imm = get_field(d, "imm");
    let rs1_val = regs.read(rs1 as usize) as i32;
    let result = op(rs1_val, imm);
    write_rd(regs, rd, result);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_shift_imm(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(u32, u32) -> u32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let shamt = (get_field(d, "imm") & 0x1F) as u32;
    let rs1_val = regs.read(rs1 as usize);
    let result = op(rs1_val, shamt);
    write_rd(regs, rd, result);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_reg_arith(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, op: fn(u32, u32) -> u32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let rs2 = get_field(d, "rs2");
    let result = op(regs.read(rs1 as usize), regs.read(rs2 as usize));
    write_rd(regs, rd, result);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_load(d: &DecodeResult, regs: &mut RegisterFile, mem: &Memory, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let imm = get_field(d, "imm");
    let addr = (regs.read(rs1 as usize) as i32).wrapping_add(imm) as usize;

    let result = match d.mnemonic.as_str() {
        "lb" => {
            let b = mem.read_byte(addr);
            (b as i8) as i32 as u32
        }
        "lh" => {
            let lo = mem.read_byte(addr) as u16;
            let hi = mem.read_byte(addr + 1) as u16;
            let half = lo | (hi << 8);
            (half as i16) as i32 as u32
        }
        "lw" => mem.read_word(addr),
        "lbu" => mem.read_byte(addr) as u32,
        "lhu" => {
            let lo = mem.read_byte(addr) as u32;
            let hi = mem.read_byte(addr + 1) as u32;
            lo | (hi << 8)
        }
        _ => 0,
    };

    write_rd(regs, rd, result);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_store(d: &DecodeResult, regs: &mut RegisterFile, mem: &mut Memory, pc: i32) -> ExecuteResult {
    let rs1 = get_field(d, "rs1");
    let rs2 = get_field(d, "rs2");
    let imm = get_field(d, "imm");
    let addr = (regs.read(rs1 as usize) as i32).wrapping_add(imm) as usize;
    let val = regs.read(rs2 as usize);

    match d.mnemonic.as_str() {
        "sb" => mem.write_byte(addr, (val & 0xFF) as u8),
        "sh" => {
            mem.write_byte(addr, (val & 0xFF) as u8);
            mem.write_byte(addr + 1, ((val >> 8) & 0xFF) as u8);
        }
        "sw" => mem.write_word(addr, val),
        _ => {}
    }

    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_branch(d: &DecodeResult, regs: &mut RegisterFile, pc: i32, cond: fn(u32, u32) -> bool) -> ExecuteResult {
    let rs1 = get_field(d, "rs1");
    let rs2 = get_field(d, "rs2");
    let imm = get_field(d, "imm");
    let taken = cond(regs.read(rs1 as usize), regs.read(rs2 as usize));
    let next_pc = if taken { pc + imm } else { pc + 4 };
    ExecuteResult { next_pc, halted: false }
}

fn exec_jal(d: &DecodeResult, regs: &mut RegisterFile, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let imm = get_field(d, "imm");
    write_rd(regs, rd, (pc + 4) as u32);
    ExecuteResult { next_pc: pc + imm, halted: false }
}

fn exec_jalr(d: &DecodeResult, regs: &mut RegisterFile, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let imm = get_field(d, "imm");
    let target = ((regs.read(rs1 as usize) as i32).wrapping_add(imm)) & !1;
    write_rd(regs, rd, (pc + 4) as u32);
    ExecuteResult { next_pc: target, halted: false }
}

fn exec_lui(d: &DecodeResult, regs: &mut RegisterFile, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let imm = get_field(d, "imm");
    write_rd(regs, rd, (imm << 12) as u32);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_auipc(d: &DecodeResult, regs: &mut RegisterFile, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let imm = get_field(d, "imm");
    write_rd(regs, rd, (pc as u32).wrapping_add((imm << 12) as u32));
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_ecall(_regs: &mut RegisterFile, csr: &mut CSRFile, pc: i32) -> ExecuteResult {
    let mtvec = csr.read(CSR_MTVEC);
    if mtvec == 0 {
        return ExecuteResult { next_pc: pc, halted: true };
    }
    csr.write(CSR_MEPC, pc as u32);
    csr.write(CSR_MCAUSE, CAUSE_ECALL_M_MODE);
    let mstatus = csr.read(CSR_MSTATUS);
    csr.write(CSR_MSTATUS, mstatus & !MIE);
    ExecuteResult { next_pc: mtvec as i32, halted: false }
}

fn exec_mret(csr: &mut CSRFile, _pc: i32) -> ExecuteResult {
    let mepc = csr.read(CSR_MEPC);
    let mstatus = csr.read(CSR_MSTATUS);
    csr.write(CSR_MSTATUS, mstatus | MIE);
    ExecuteResult { next_pc: mepc as i32, halted: false }
}

fn exec_csrrw(d: &DecodeResult, regs: &mut RegisterFile, csr: &mut CSRFile, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let csr_addr = get_field(d, "csr") as u32;
    let old = csr.read_write(csr_addr, regs.read(rs1 as usize));
    write_rd(regs, rd, old);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_csrrs(d: &DecodeResult, regs: &mut RegisterFile, csr: &mut CSRFile, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let csr_addr = get_field(d, "csr") as u32;
    let old = csr.read_set(csr_addr, regs.read(rs1 as usize));
    write_rd(regs, rd, old);
    ExecuteResult { next_pc: pc + 4, halted: false }
}

fn exec_csrrc(d: &DecodeResult, regs: &mut RegisterFile, csr: &mut CSRFile, pc: i32) -> ExecuteResult {
    let rd = get_field(d, "rd");
    let rs1 = get_field(d, "rs1");
    let csr_addr = get_field(d, "csr") as u32;
    let old = csr.read_clear(csr_addr, regs.read(rs1 as usize));
    write_rd(regs, rd, old);
    ExecuteResult { next_pc: pc + 4, halted: false }
}
