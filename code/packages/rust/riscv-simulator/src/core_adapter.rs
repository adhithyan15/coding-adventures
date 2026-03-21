//! Adapter to run RISC-V on a D05 Core pipeline.
//!
//! Bridges the RISC-V decoder/executor with a Core ISADecoder interface.
//! Decode fills control signals from a mnemonic truth table. Execute computes
//! ALU results for all RV32I + M-mode instructions without accessing memory
//! or writing registers (those happen in later pipeline stages).

use std::collections::HashMap;

use crate::csr::{CSRFile, CSR_MCAUSE, CSR_MEPC, CSR_MSTATUS, CSR_MTVEC, CAUSE_ECALL_M_MODE, MIE};
use crate::decode;

/// Pipeline token -- a dictionary-like struct carrying instruction state
/// through pipeline stages.
#[derive(Debug, Clone)]
pub struct PipelineToken {
    pub pc: i32,
    pub opcode: String,
    pub rd: i32,
    pub rs1: i32,
    pub rs2: i32,
    pub immediate: i32,
    pub alu_result: i32,
    pub write_data: i32,
    pub branch_taken: bool,
    pub branch_target: i32,
    pub reg_write: bool,
    pub mem_read: bool,
    pub mem_write: bool,
    pub is_branch: bool,
    pub is_halt: bool,
    pub raw_instruction: u32,
}

impl PipelineToken {
    pub fn new(pc: i32) -> Self {
        Self {
            pc,
            opcode: String::new(),
            rd: -1,
            rs1: -1,
            rs2: -1,
            immediate: 0,
            alu_result: 0,
            write_data: 0,
            branch_taken: false,
            branch_target: 0,
            reg_write: false,
            mem_read: false,
            mem_write: false,
            is_branch: false,
            is_halt: false,
            raw_instruction: 0,
        }
    }
}

/// Simple register file for the Core adapter.
pub struct SimpleRegisterFile {
    regs: Vec<i32>,
}

impl SimpleRegisterFile {
    pub fn new(count: usize) -> Self {
        Self { regs: vec![0i32; count] }
    }

    pub fn read(&self, index: i32) -> i32 {
        if index < 0 || index as usize >= self.regs.len() {
            return 0;
        }
        self.regs[index as usize]
    }

    pub fn write(&mut self, index: i32, value: i32) {
        if index > 0 && (index as usize) < self.regs.len() {
            self.regs[index as usize] = value;
        }
    }
}

fn get_field(fields: &HashMap<String, i32>, key: &str, default: i32) -> i32 {
    fields.get(key).copied().unwrap_or(default)
}

/// Adapts the RISC-V decoder to a Core ISADecoder interface.
pub struct RiscVISADecoder {
    csr: CSRFile,
}

impl RiscVISADecoder {
    pub fn new() -> Self {
        Self { csr: CSRFile::new() }
    }

    /// Access the CSR file.
    pub fn csr(&self) -> &CSRFile {
        &self.csr
    }

    /// Mutable access to the CSR file.
    pub fn csr_mut(&mut self) -> &mut CSRFile {
        &mut self.csr
    }

    /// All RV32I instructions are 4 bytes.
    pub fn instruction_size(&self) -> i32 {
        4
    }

    /// Decode a raw RISC-V instruction into a PipelineToken.
    pub fn decode(&self, raw_instruction: u32, token: &mut PipelineToken) {
        let decoded = decode::decode(raw_instruction, token.pc);

        token.opcode = decoded.mnemonic.clone();
        token.rd = get_field(&decoded.fields, "rd", -1);
        token.rs1 = get_field(&decoded.fields, "rs1", -1);
        token.rs2 = get_field(&decoded.fields, "rs2", -1);
        token.immediate = get_field(&decoded.fields, "imm", 0);

        match decoded.mnemonic.as_str() {
            "add" | "sub" | "sll" | "slt" | "sltu" | "xor" | "srl" | "sra" | "or" | "and" => {
                token.reg_write = true;
            }
            "addi" | "slti" | "sltiu" | "xori" | "ori" | "andi" | "slli" | "srli" | "srai" => {
                token.reg_write = true;
            }
            "lui" | "auipc" => {
                token.reg_write = true;
            }
            "lb" | "lh" | "lw" | "lbu" | "lhu" => {
                token.reg_write = true;
                token.mem_read = true;
            }
            "sb" | "sh" | "sw" => {
                token.mem_write = true;
            }
            "beq" | "bne" | "blt" | "bge" | "bltu" | "bgeu" => {
                token.is_branch = true;
            }
            "jal" | "jalr" => {
                token.reg_write = true;
                token.is_branch = true;
            }
            "ecall" => {
                if self.csr.read(CSR_MTVEC) == 0 {
                    token.is_halt = true;
                }
            }
            "csrrw" | "csrrs" | "csrrc" => {
                token.reg_write = true;
            }
            "mret" => {
                token.is_branch = true;
            }
            _ => {}
        }
    }

    /// Perform ALU computation. Does NOT access memory or write registers.
    pub fn execute(&mut self, token: &mut PipelineToken, reg_file: &SimpleRegisterFile) {
        let rs1_val = if token.rs1 >= 0 { reg_file.read(token.rs1) } else { 0 };
        let rs2_val = if token.rs2 >= 0 { reg_file.read(token.rs2) } else { 0 };

        let rs1_u = rs1_val as u32;
        let rs2_u = rs2_val as u32;
        let imm = token.immediate;
        let pc = token.pc;

        match token.opcode.as_str() {
            // R-type arithmetic
            "add" => {
                let result = (rs1_u as i32).wrapping_add(rs2_u as i32);
                token.alu_result = result;
                token.write_data = result;
            }
            "sub" => {
                let result = (rs1_u as i32).wrapping_sub(rs2_u as i32);
                token.alu_result = result;
                token.write_data = result;
            }
            "sll" => {
                let result = (rs1_u << (rs2_u & 0x1F)) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "slt" => {
                let result = if (rs1_u as i32) < (rs2_u as i32) { 1 } else { 0 };
                token.alu_result = result;
                token.write_data = result;
            }
            "sltu" => {
                let result = if rs1_u < rs2_u { 1 } else { 0 };
                token.alu_result = result;
                token.write_data = result;
            }
            "xor" => {
                let result = (rs1_u ^ rs2_u) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "srl" => {
                let result = (rs1_u >> (rs2_u & 0x1F)) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "sra" => {
                let result = (rs1_u as i32) >> (rs2_u & 0x1F);
                token.alu_result = result;
                token.write_data = result;
            }
            "or" => {
                let result = (rs1_u | rs2_u) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "and" => {
                let result = (rs1_u & rs2_u) as i32;
                token.alu_result = result;
                token.write_data = result;
            }

            // I-type arithmetic
            "addi" => {
                let result = (rs1_u as i32).wrapping_add(imm);
                token.alu_result = result;
                token.write_data = result;
            }
            "slti" => {
                let result = if (rs1_u as i32) < imm { 1 } else { 0 };
                token.alu_result = result;
                token.write_data = result;
            }
            "sltiu" => {
                let result = if rs1_u < (imm as u32) { 1 } else { 0 };
                token.alu_result = result;
                token.write_data = result;
            }
            "xori" => {
                let result = (rs1_u ^ (imm as u32)) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "ori" => {
                let result = (rs1_u | (imm as u32)) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "andi" => {
                let result = (rs1_u & (imm as u32)) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "slli" => {
                let shamt = (imm as u32) & 0x1F;
                let result = (rs1_u << shamt) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "srli" => {
                let shamt = (imm as u32) & 0x1F;
                let result = (rs1_u >> shamt) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "srai" => {
                let shamt = (imm as u32) & 0x1F;
                let result = (rs1_u as i32) >> shamt;
                token.alu_result = result;
                token.write_data = result;
            }

            // Upper immediate
            "lui" => {
                let result = ((imm << 12) as u32) as i32;
                token.alu_result = result;
                token.write_data = result;
            }
            "auipc" => {
                let result = ((pc as u32).wrapping_add((imm << 12) as u32)) as i32;
                token.alu_result = result;
                token.write_data = result;
            }

            // Load: compute effective address
            "lb" | "lh" | "lw" | "lbu" | "lhu" => {
                let addr = (rs1_u as i32).wrapping_add(imm);
                token.alu_result = addr;
            }

            // Store: compute address + prepare data
            "sb" | "sh" | "sw" => {
                let addr = (rs1_u as i32).wrapping_add(imm);
                token.alu_result = addr;
                token.write_data = rs2_val;
            }

            // Branch instructions
            "beq" => self.execute_branch(token, rs1_u == rs2_u, pc, imm),
            "bne" => self.execute_branch(token, rs1_u != rs2_u, pc, imm),
            "blt" => self.execute_branch(token, (rs1_u as i32) < (rs2_u as i32), pc, imm),
            "bge" => self.execute_branch(token, (rs1_u as i32) >= (rs2_u as i32), pc, imm),
            "bltu" => self.execute_branch(token, rs1_u < rs2_u, pc, imm),
            "bgeu" => self.execute_branch(token, rs1_u >= rs2_u, pc, imm),

            // Jump instructions
            "jal" => {
                let return_addr = pc + 4;
                let target = pc + imm;
                token.alu_result = target;
                token.write_data = return_addr;
                token.branch_taken = true;
                token.branch_target = target;
            }
            "jalr" => {
                let return_addr = pc + 4;
                let target = ((rs1_u as i32).wrapping_add(imm)) & !1;
                token.alu_result = target;
                token.write_data = return_addr;
                token.branch_taken = true;
                token.branch_target = target;
            }

            // CSR instructions
            "csrrw" => {
                let csr_addr = (token.raw_instruction >> 20) & 0xFFF;
                let old_val = self.csr.read_write(csr_addr, rs1_u);
                token.alu_result = old_val as i32;
                token.write_data = old_val as i32;
            }
            "csrrs" => {
                let csr_addr = (token.raw_instruction >> 20) & 0xFFF;
                let old_val = self.csr.read_set(csr_addr, rs1_u);
                token.alu_result = old_val as i32;
                token.write_data = old_val as i32;
            }
            "csrrc" => {
                let csr_addr = (token.raw_instruction >> 20) & 0xFFF;
                let old_val = self.csr.read_clear(csr_addr, rs1_u);
                token.alu_result = old_val as i32;
                token.write_data = old_val as i32;
            }

            // ecall
            "ecall" => {
                let mtvec = self.csr.read(CSR_MTVEC);
                if mtvec != 0 {
                    self.csr.write(CSR_MEPC, pc as u32);
                    self.csr.write(CSR_MCAUSE, CAUSE_ECALL_M_MODE);
                    let mstatus = self.csr.read(CSR_MSTATUS);
                    self.csr.write(CSR_MSTATUS, mstatus & !MIE);
                    token.branch_taken = true;
                    token.branch_target = mtvec as i32;
                    token.alu_result = mtvec as i32;
                }
            }

            // mret
            "mret" => {
                let mepc = self.csr.read(CSR_MEPC);
                let mstatus = self.csr.read(CSR_MSTATUS);
                self.csr.write(CSR_MSTATUS, mstatus | MIE);
                token.branch_taken = true;
                token.branch_target = mepc as i32;
                token.alu_result = mepc as i32;
            }

            _ => {} // Unknown: NOP
        }
    }

    fn execute_branch(&self, token: &mut PipelineToken, taken: bool, pc: i32, imm: i32) {
        let target = pc + imm;
        token.branch_taken = taken;
        token.branch_target = target;
        token.alu_result = if taken { target } else { pc + 4 };
    }
}

impl Default for RiscVISADecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory function.
pub fn new_riscv_core() -> RiscVISADecoder {
    RiscVISADecoder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;

    fn make_token(pc: i32) -> PipelineToken {
        PipelineToken::new(pc)
    }

    #[test]
    fn test_instruction_size() {
        let decoder = RiscVISADecoder::new();
        assert_eq!(decoder.instruction_size(), 4);
    }

    #[test]
    fn test_csr_accessor() {
        let decoder = RiscVISADecoder::new();
        // Just verify it doesn't panic
        let _ = decoder.csr();
    }

    #[test]
    fn test_factory_function() {
        let _ = new_riscv_core();
    }

    // Helper to check control signals
    fn check_signals(
        raw: u32,
        reg_write: bool,
        mem_read: bool,
        mem_write: bool,
        is_branch: bool,
        is_halt: bool,
    ) {
        let decoder = RiscVISADecoder::new();
        let mut token = make_token(0);
        decoder.decode(raw, &mut token);
        assert_eq!(token.reg_write, reg_write, "reg_write for {}", token.opcode);
        assert_eq!(token.mem_read, mem_read, "mem_read for {}", token.opcode);
        assert_eq!(token.mem_write, mem_write, "mem_write for {}", token.opcode);
        assert_eq!(token.is_branch, is_branch, "is_branch for {}", token.opcode);
        assert_eq!(token.is_halt, is_halt, "is_halt for {}", token.opcode);
    }

    #[test]
    fn test_decode_r_type() {
        for raw in [
            encode_add(3, 1, 2), encode_sub(3, 1, 2), encode_sll(3, 1, 2),
            encode_slt(3, 1, 2), encode_sltu(3, 1, 2), encode_xor(3, 1, 2),
            encode_srl(3, 1, 2), encode_sra(3, 1, 2), encode_or(3, 1, 2),
            encode_and(3, 1, 2),
        ] {
            check_signals(raw, true, false, false, false, false);
        }
    }

    #[test]
    fn test_decode_i_type() {
        for raw in [
            encode_addi(1, 2, 5), encode_slti(1, 2, 5), encode_sltiu(1, 2, 5),
            encode_xori(1, 2, 5), encode_ori(1, 2, 5), encode_andi(1, 2, 5),
            encode_slli(1, 2, 3), encode_srli(1, 2, 3), encode_srai(1, 2, 3),
        ] {
            check_signals(raw, true, false, false, false, false);
        }
    }

    #[test]
    fn test_decode_upper_imm() {
        check_signals(encode_lui(1, 0x12345), true, false, false, false, false);
        check_signals(encode_auipc(1, 0x12345), true, false, false, false, false);
    }

    #[test]
    fn test_decode_loads() {
        for raw in [
            encode_lb(1, 2, 0), encode_lh(1, 2, 0), encode_lw(1, 2, 0),
            encode_lbu(1, 2, 0), encode_lhu(1, 2, 0),
        ] {
            check_signals(raw, true, true, false, false, false);
        }
    }

    #[test]
    fn test_decode_stores() {
        for raw in [encode_sb(1, 2, 0), encode_sh(1, 2, 0), encode_sw(1, 2, 0)] {
            check_signals(raw, false, false, true, false, false);
        }
    }

    #[test]
    fn test_decode_branches() {
        for raw in [
            encode_beq(1, 2, 8), encode_bne(1, 2, 8), encode_blt(1, 2, 8),
            encode_bge(1, 2, 8), encode_bltu(1, 2, 8), encode_bgeu(1, 2, 8),
        ] {
            check_signals(raw, false, false, false, true, false);
        }
    }

    #[test]
    fn test_decode_jumps() {
        check_signals(encode_jal(1, 8), true, false, false, true, false);
        check_signals(encode_jalr(1, 2, 0), true, false, false, true, false);
    }

    #[test]
    fn test_decode_ecall() {
        check_signals(encode_ecall(), false, false, false, false, true);
    }

    #[test]
    fn test_decode_csr() {
        check_signals(encode_csrrw(1, 0x300, 2), true, false, false, false, false);
        check_signals(encode_csrrs(1, 0x300, 2), true, false, false, false, false);
        check_signals(encode_csrrc(1, 0x300, 2), true, false, false, false, false);
    }

    #[test]
    fn test_decode_mret() {
        check_signals(encode_mret(), false, false, false, true, false);
    }

    #[test]
    fn test_execute_alu() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(1, 10);
        regs.write(2, 3);

        let cases: Vec<(&str, u32, i32)> = vec![
            ("add", encode_add(3, 1, 2), 13),
            ("sub", encode_sub(3, 1, 2), 7),
            ("sll", encode_sll(3, 1, 2), 80),
            ("srl", encode_srl(3, 1, 2), 1),
            ("xor", encode_xor(3, 1, 2), (10i32 ^ 3)),
            ("or", encode_or(3, 1, 2), (10 | 3)),
            ("and", encode_and(3, 1, 2), (10 & 3)),
            ("addi", encode_addi(3, 1, 5), 15),
            ("slli", encode_slli(3, 1, 2), 40),
            ("srli", encode_srli(3, 1, 1), 5),
            ("lui", encode_lui(3, 1), 4096),
            ("lw", encode_lw(3, 1, 4), 14),
            ("sw", encode_sw(2, 1, 8), 18),
        ];

        for (name, raw, expected_alu) in cases {
            let mut token = make_token(0);
            decoder.decode(raw, &mut token);
            decoder.execute(&mut token, &regs);
            assert_eq!(token.alu_result, expected_alu, "{}: ALU result", name);
        }
    }

    #[test]
    fn test_beq_not_taken() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(1, 10);
        regs.write(2, 3);

        let mut token = make_token(0);
        decoder.decode(encode_beq(1, 2, 100), &mut token);
        decoder.execute(&mut token, &regs);
        assert_eq!(token.alu_result, 4); // PC+4
    }

    #[test]
    fn test_branch_execution() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(1, 5);
        regs.write(2, 5);
        regs.write(3, 10);

        let cases: Vec<(&str, u32, bool, i32)> = vec![
            ("beq_taken", encode_beq(1, 2, 20), true, 20),
            ("beq_not_taken", encode_beq(1, 3, 20), false, 0),
            ("bne_taken", encode_bne(1, 3, 20), true, 20),
            ("bne_not_taken", encode_bne(1, 2, 20), false, 0),
            ("blt_taken", encode_blt(1, 3, 20), true, 20),
            ("blt_not_taken", encode_blt(3, 1, 20), false, 0),
            ("bge_taken", encode_bge(3, 1, 20), true, 20),
            ("bge_not_taken", encode_bge(1, 3, 20), false, 0),
            ("bltu_taken", encode_bltu(1, 3, 20), true, 20),
            ("bgeu_taken", encode_bgeu(3, 1, 20), true, 20),
        ];

        for (name, raw, expected_taken, expected_target) in cases {
            let mut token = make_token(0);
            decoder.decode(raw, &mut token);
            decoder.execute(&mut token, &regs);
            assert_eq!(token.branch_taken, expected_taken, "{}: branch_taken", name);
            if expected_taken {
                assert_eq!(token.branch_target, expected_target, "{}: branch_target", name);
            }
        }
    }

    #[test]
    fn test_jal() {
        let mut decoder = RiscVISADecoder::new();
        let regs = SimpleRegisterFile::new(32);

        let mut token = make_token(8);
        decoder.decode(encode_jal(1, 20), &mut token);
        decoder.execute(&mut token, &regs);

        assert!(token.branch_taken);
        assert_eq!(token.branch_target, 28);
        assert_eq!(token.write_data, 12);
    }

    #[test]
    fn test_jalr() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(5, 100);

        let mut token = make_token(16);
        decoder.decode(encode_jalr(1, 5, 8), &mut token);
        decoder.execute(&mut token, &regs);

        assert!(token.branch_taken);
        assert_eq!(token.branch_target, 108);
        assert_eq!(token.write_data, 20);
    }

    #[test]
    fn test_slt() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(1, 5);
        regs.write(2, 10);

        let mut token = make_token(0);
        decoder.decode(encode_slt(3, 1, 2), &mut token);
        decoder.execute(&mut token, &regs);
        assert_eq!(token.alu_result, 1);
    }

    #[test]
    fn test_sltu() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(1, 5);
        regs.write(2, 10);

        let mut token = make_token(0);
        decoder.decode(encode_sltu(3, 1, 2), &mut token);
        decoder.execute(&mut token, &regs);
        assert_eq!(token.alu_result, 1);
    }

    #[test]
    fn test_sra_negative() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(1, -16i32); // 0xFFFFFFF0
        regs.write(2, 2);

        let mut token = make_token(0);
        decoder.decode(encode_sra(3, 1, 2), &mut token);
        decoder.execute(&mut token, &regs);
        assert_eq!(token.alu_result, -4);
    }

    #[test]
    fn test_unknown_instruction_no_crash() {
        let mut decoder = RiscVISADecoder::new();
        let regs = SimpleRegisterFile::new(32);

        let mut token = make_token(0);
        token.opcode = "SOMETHING_UNKNOWN".to_string();
        decoder.execute(&mut token, &regs);
    }

    #[test]
    fn test_csrrw() {
        let mut decoder = RiscVISADecoder::new();
        let mut regs = SimpleRegisterFile::new(32);
        regs.write(2, 42);

        let raw = encode_csrrw(1, 0x300, 2);
        let mut token = make_token(0);
        token.raw_instruction = raw;
        decoder.decode(raw, &mut token);
        decoder.execute(&mut token, &regs);

        assert_eq!(token.alu_result, 0);
        assert_eq!(decoder.csr().read(0x300), 42);
    }

    #[test]
    fn test_get_field_existing() {
        let mut fields = HashMap::new();
        fields.insert("rd".to_string(), 5);
        assert_eq!(get_field(&fields, "rd", -1), 5);
    }

    #[test]
    fn test_get_field_missing() {
        let fields: HashMap<String, i32> = HashMap::new();
        assert_eq!(get_field(&fields, "rd", 42), 42);
    }

    #[test]
    fn test_sparse_memory_integration() {
        use cpu_simulator::sparse_memory::{MemoryRegion, SparseMemory};

        let mut mem = SparseMemory::new(vec![
            MemoryRegion::new(0x00000000, 0x10000, "RAM", false),
            MemoryRegion::new(0xFFFF0000, 0x100, "ROM", true),
        ]);

        let program = assemble(&[encode_addi(1, 0, 42), encode_ecall()]);
        mem.load_bytes(0, &program);

        assert_ne!(mem.read_word(0), 0);
        assert_eq!(mem.read_byte(0xFFFF0000), 0);
    }
}
