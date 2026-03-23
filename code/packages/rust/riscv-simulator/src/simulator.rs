//! Top-level RISC-V simulator combining all components.

use cpu_simulator::{RegisterFile, Memory};
use crate::csr::CSRFile;
use crate::decode;
use crate::execute;
use crate::encoding::assemble;

/// Complete RISC-V simulator with registers, memory, CSR file, and PC.
pub struct RiscVSimulator {
    pub regs: RegisterFile,
    pub mem: Memory,
    pub csr: CSRFile,
    pub pc: i32,
    pub halted: bool,
}

impl RiscVSimulator {
    /// Create a new simulator with the given memory size.
    pub fn new(memory_size: usize) -> Self {
        Self {
            regs: RegisterFile::new(32, true),
            mem: Memory::new(memory_size),
            csr: CSRFile::new(),
            pc: 0,
            halted: false,
        }
    }

    /// Load a program (as raw bytes) into memory at address 0.
    pub fn load_program(&mut self, program: &[u8]) {
        self.mem.load_bytes(0, program);
    }

    /// Run until halted or 10000 steps (safety limit).
    pub fn run(&mut self, program: &[u8]) {
        self.load_program(program);
        for _ in 0..10000 {
            if self.halted {
                break;
            }
            self.step();
        }
    }

    /// Run instructions from already loaded program.
    pub fn run_loaded(&mut self) {
        for _ in 0..10000 {
            if self.halted {
                break;
            }
            self.step();
        }
    }

    /// Execute a single instruction.
    pub fn step(&mut self) -> String {
        if self.halted {
            return "halted".to_string();
        }

        // Fetch
        let raw = self.mem.read_word(self.pc as usize);

        // Decode
        let decoded = decode::decode(raw, self.pc);
        let mnemonic = decoded.mnemonic.clone();

        // Execute
        let result = execute::execute(&decoded, &mut self.regs, &mut self.mem, &mut self.csr, self.pc);
        self.pc = result.next_pc;
        self.halted = result.halted;

        mnemonic
    }

    /// Run a list of instruction words (convenience for tests).
    pub fn run_instructions(&mut self, instructions: &[u32]) {
        let program = assemble(instructions);
        self.run(&program);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;
    use crate::csr::*;

    fn run_program(instructions: &[u32]) -> RiscVSimulator {
        let mut sim = RiscVSimulator::new(65536);
        sim.run_instructions(instructions);
        sim
    }

    // I-type arithmetic
    #[test] fn test_addi() {
        let sim = run_program(&[encode_addi(1,0,42), encode_addi(2,1,10), encode_addi(3,0,-5), encode_addi(4,3,3), encode_ecall()]);
        assert_eq!(sim.regs.read(1), 42);
        assert_eq!(sim.regs.read(2), 52);
        assert_eq!(sim.regs.read(3) as i32, -5);
        assert_eq!(sim.regs.read(4) as i32, -2);
    }
    #[test] fn test_slti() {
        let sim = run_program(&[encode_addi(1,0,5), encode_slti(2,1,10), encode_slti(3,1,3), encode_slti(4,1,5), encode_addi(5,0,-1), encode_slti(6,5,0), encode_ecall()]);
        assert_eq!(sim.regs.read(2), 1); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(4), 0); assert_eq!(sim.regs.read(6), 1);
    }
    #[test] fn test_sltiu() {
        let sim = run_program(&[encode_addi(1,0,5), encode_sltiu(2,1,10), encode_sltiu(3,1,3), encode_addi(4,0,-1), encode_sltiu(5,4,1), encode_ecall()]);
        assert_eq!(sim.regs.read(2), 1); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(5), 0);
    }
    #[test] fn test_xori() { let sim = run_program(&[encode_addi(1,0,0xFF), encode_xori(2,1,0x0F), encode_ecall()]); assert_eq!(sim.regs.read(2), 0xF0); }
    #[test] fn test_ori() { let sim = run_program(&[encode_addi(1,0,0x50), encode_ori(2,1,0x0F), encode_ecall()]); assert_eq!(sim.regs.read(2), 0x5F); }
    #[test] fn test_andi() { let sim = run_program(&[encode_addi(1,0,0xFF), encode_andi(2,1,0x0F), encode_ecall()]); assert_eq!(sim.regs.read(2), 0x0F); }
    #[test] fn test_slli() { let sim = run_program(&[encode_addi(1,0,1), encode_slli(2,1,4), encode_slli(3,1,31), encode_ecall()]); assert_eq!(sim.regs.read(2), 16); assert_eq!(sim.regs.read(3), 0x80000000); }
    #[test] fn test_srli() { let sim = run_program(&[encode_addi(1,0,-1), encode_srli(2,1,4), encode_srli(3,1,31), encode_ecall()]); assert_eq!(sim.regs.read(2), 0x0FFFFFFF); assert_eq!(sim.regs.read(3), 1); }
    #[test] fn test_srai() { let sim = run_program(&[encode_addi(1,0,-16), encode_srai(2,1,2), encode_addi(3,0,16), encode_srai(4,3,2), encode_ecall()]); assert_eq!(sim.regs.read(2) as i32, -4); assert_eq!(sim.regs.read(4), 4); }

    // R-type arithmetic
    #[test] fn test_add_sub() { let sim = run_program(&[encode_addi(1,0,10), encode_addi(2,0,20), encode_add(3,1,2), encode_sub(4,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 30); assert_eq!(sim.regs.read(4) as i32, -10); }
    #[test] fn test_sll() { let sim = run_program(&[encode_addi(1,0,1), encode_addi(2,0,8), encode_sll(3,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 256); }
    #[test] fn test_slt() { let sim = run_program(&[encode_addi(1,0,-5), encode_addi(2,0,3), encode_slt(3,1,2), encode_slt(4,2,1), encode_ecall()]); assert_eq!(sim.regs.read(3), 1); assert_eq!(sim.regs.read(4), 0); }
    #[test] fn test_sltu() { let sim = run_program(&[encode_addi(1,0,-1), encode_addi(2,0,1), encode_sltu(3,2,1), encode_sltu(4,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 1); assert_eq!(sim.regs.read(4), 0); }
    #[test] fn test_xor() { let sim = run_program(&[encode_addi(1,0,0xFF), encode_addi(2,0,0x0F), encode_xor(3,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 0xF0); }
    #[test] fn test_srl() { let sim = run_program(&[encode_addi(1,0,-1), encode_addi(2,0,4), encode_srl(3,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 0x0FFFFFFF); }
    #[test] fn test_sra() { let sim = run_program(&[encode_addi(1,0,-16), encode_addi(2,0,2), encode_sra(3,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3) as i32, -4); }
    #[test] fn test_or() { let sim = run_program(&[encode_addi(1,0,0x50), encode_addi(2,0,0x0F), encode_or(3,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 0x5F); }
    #[test] fn test_and() { let sim = run_program(&[encode_addi(1,0,0xFF), encode_addi(2,0,0x0F), encode_and(3,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 0x0F); }

    // Loads & stores
    #[test] fn test_sw_lw() { let sim = run_program(&[encode_addi(1,0,0x100), encode_addi(2,0,0x42), encode_sw(2,1,0), encode_lw(3,1,0), encode_ecall()]); assert_eq!(sim.regs.read(3), 0x42); }
    #[test] fn test_sb_lbu() { let sim = run_program(&[encode_addi(1,0,0x200), encode_addi(2,0,0xAB), encode_sb(2,1,0), encode_lbu(3,1,0), encode_ecall()]); assert_eq!(sim.regs.read(3), 0xAB); }
    #[test] fn test_lb_sign() { let sim = run_program(&[encode_addi(1,0,0x200), encode_addi(2,0,0xFF), encode_sb(2,1,0), encode_lb(3,1,0), encode_lbu(4,1,0), encode_ecall()]); assert_eq!(sim.regs.read(3) as i32, -1); assert_eq!(sim.regs.read(4), 0xFF); }
    #[test] fn test_sh_lhu() { let sim = run_program(&[encode_addi(1,0,0x200), encode_lui(2,0), encode_addi(2,0,0x1FF), encode_sh(2,1,0), encode_lhu(3,1,0), encode_ecall()]); assert_eq!(sim.regs.read(3), 0x1FF); }
    #[test] fn test_lh_sign() { let sim = run_program(&[encode_addi(1,0,0x200), encode_addi(2,0,-1), encode_sh(2,1,0), encode_lh(3,1,0), encode_lhu(4,1,0), encode_ecall()]); assert_eq!(sim.regs.read(3) as i32, -1); assert_eq!(sim.regs.read(4), 0xFFFF); }
    #[test] fn test_sw_lw_offset() { let sim = run_program(&[encode_addi(1,0,0x200), encode_addi(2,0,99), encode_sw(2,1,4), encode_lw(3,1,4), encode_ecall()]); assert_eq!(sim.regs.read(3), 99); }

    // Branches
    #[test] fn test_beq_taken() { let sim = run_program(&[encode_addi(1,0,5), encode_addi(2,0,5), encode_beq(1,2,8), encode_addi(3,0,999), encode_addi(4,0,42), encode_ecall()]); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(4), 42); }
    #[test] fn test_beq_not_taken() { let sim = run_program(&[encode_addi(1,0,5), encode_addi(2,0,10), encode_beq(1,2,8), encode_addi(3,0,42), encode_ecall()]); assert_eq!(sim.regs.read(3), 42); }
    #[test] fn test_bne() { let sim = run_program(&[encode_addi(1,0,5), encode_addi(2,0,10), encode_bne(1,2,8), encode_addi(3,0,999), encode_addi(4,0,42), encode_ecall()]); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(4), 42); }
    #[test] fn test_blt() { let sim = run_program(&[encode_addi(1,0,-5), encode_addi(2,0,3), encode_blt(1,2,8), encode_addi(3,0,999), encode_addi(4,0,42), encode_ecall()]); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(4), 42); }
    #[test] fn test_bge() { let sim = run_program(&[encode_addi(1,0,5), encode_addi(2,0,5), encode_bge(1,2,8), encode_addi(3,0,999), encode_addi(4,0,42), encode_ecall()]); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(4), 42); }
    #[test] fn test_bltu() { let sim = run_program(&[encode_addi(1,0,1), encode_addi(2,0,-1), encode_bltu(1,2,8), encode_addi(3,0,999), encode_addi(4,0,42), encode_ecall()]); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(4), 42); }
    #[test] fn test_bgeu() { let sim = run_program(&[encode_addi(1,0,-1), encode_addi(2,0,1), encode_bgeu(1,2,8), encode_addi(3,0,999), encode_addi(4,0,42), encode_ecall()]); assert_eq!(sim.regs.read(3), 0); assert_eq!(sim.regs.read(4), 42); }
    #[test] fn test_branch_backward() { let sim = run_program(&[encode_addi(1,0,0), encode_addi(2,0,3), encode_addi(1,1,1), encode_bne(1,2,-4), encode_ecall()]); assert_eq!(sim.regs.read(1), 3); }

    // Jumps
    #[test] fn test_jal() { let sim = run_program(&[encode_jal(1,8), encode_addi(2,0,999), encode_addi(3,0,42), encode_ecall()]); assert_eq!(sim.regs.read(1), 4); assert_eq!(sim.regs.read(2), 0); assert_eq!(sim.regs.read(3), 42); }
    #[test] fn test_jalr() { let sim = run_program(&[encode_addi(5,0,12), encode_jalr(1,5,0), encode_addi(2,0,999), encode_addi(3,0,42), encode_ecall()]); assert_eq!(sim.regs.read(1), 8); assert_eq!(sim.regs.read(2), 0); assert_eq!(sim.regs.read(3), 42); }
    #[test] fn test_jalr_offset() { let sim = run_program(&[encode_addi(5,0,8), encode_jalr(1,5,4), encode_addi(2,0,999), encode_addi(3,0,42), encode_ecall()]); assert_eq!(sim.regs.read(1), 8); assert_eq!(sim.regs.read(2), 0); assert_eq!(sim.regs.read(3), 42); }
    #[test] fn test_call_return() { let sim = run_program(&[encode_jal(1,12), encode_addi(11,0,99), encode_ecall(), encode_addi(10,0,42), encode_jalr(0,1,0)]); assert_eq!(sim.regs.read(1), 4); assert_eq!(sim.regs.read(10), 42); assert_eq!(sim.regs.read(11), 99); }

    // LUI / AUIPC
    #[test] fn test_lui() { let sim = run_program(&[encode_lui(1, 0x12345), encode_ecall()]); assert_eq!(sim.regs.read(1), 0x12345000); }
    #[test] fn test_lui_addi() { let sim = run_program(&[encode_lui(1, 0x12345), encode_addi(1,1,0x678), encode_ecall()]); assert_eq!(sim.regs.read(1), 0x12345678); }
    #[test] fn test_auipc() { let sim = run_program(&[encode_auipc(1, 1), encode_ecall()]); assert_eq!(sim.regs.read(1), 0x1000); }
    #[test] fn test_auipc_nonzero() { let sim = run_program(&[encode_addi(0,0,0), encode_auipc(1, 2), encode_ecall()]); assert_eq!(sim.regs.read(1), 0x2004); }

    // x0 hardwired
    #[test] fn test_x0_hardwired() { let sim = run_program(&[encode_addi(0,0,42), encode_ecall()]); assert_eq!(sim.regs.read(0), 0); }
    #[test] fn test_x0_r_type() { let sim = run_program(&[encode_addi(1,0,5), encode_addi(2,0,10), encode_add(0,1,2), encode_ecall()]); assert_eq!(sim.regs.read(0), 0); }

    // CSR operations
    #[test] fn test_csrrw() {
        let mut sim = RiscVSimulator::new(65536);
        sim.run_instructions(&[encode_addi(1,0,0x100), encode_csrrw(2, CSR_MSCRATCH, 1), encode_csrrw(3, CSR_MSCRATCH, 0), encode_ecall()]);
        assert_eq!(sim.regs.read(2), 0); assert_eq!(sim.regs.read(3), 0x100);
    }
    #[test] fn test_csrrs() {
        let mut sim = RiscVSimulator::new(65536);
        sim.run_instructions(&[encode_addi(1,0,8), encode_csrrs(2, CSR_MSTATUS, 1), encode_csrrs(3, CSR_MSTATUS, 0), encode_ecall()]);
        assert_eq!(sim.regs.read(2), 0); assert_eq!(sim.regs.read(3), 8);
    }
    #[test] fn test_csrrc() {
        let mut sim = RiscVSimulator::new(65536);
        sim.run_instructions(&[encode_addi(1,0,0xFF), encode_csrrw(0, CSR_MSCRATCH, 1), encode_addi(2,0,0x0F), encode_csrrc(3, CSR_MSCRATCH, 2), encode_csrrs(4, CSR_MSCRATCH, 0), encode_ecall()]);
        assert_eq!(sim.regs.read(3), 0xFF); assert_eq!(sim.regs.read(4), 0xF0);
    }

    // ecall trap
    #[test] fn test_ecall_halt() { let sim = run_program(&[encode_addi(1,0,42), encode_ecall()]); assert!(sim.halted); assert_eq!(sim.regs.read(1), 42); }

    #[test] fn test_ecall_trap_handler() {
        let mut sim = RiscVSimulator::new(65536);
        let mut main_code = vec![encode_addi(1,0,0x100), encode_csrrw(0,CSR_MTVEC,1), encode_ecall(), encode_addi(11,0,77), encode_csrrw(0,CSR_MTVEC,0), encode_ecall()];
        let pad_count = (0x100/4) - main_code.len();
        for _ in 0..pad_count { main_code.push(encode_addi(0,0,0)); }
        main_code.extend_from_slice(&[encode_addi(10,0,99), encode_csrrs(20,CSR_MEPC,0), encode_addi(20,20,4), encode_csrrw(0,CSR_MEPC,20), encode_mret()]);
        sim.run_instructions(&main_code);
        assert_eq!(sim.regs.read(10), 99); assert_eq!(sim.regs.read(11), 77); assert!(sim.halted);
    }

    #[test] fn test_ecall_sets_csrs() {
        let mut sim = RiscVSimulator::new(65536);
        let mut code = vec![encode_addi(1,0,0x200), encode_csrrw(0,CSR_MTVEC,1), encode_addi(2,0,8), encode_csrrs(0,CSR_MSTATUS,2), encode_ecall()];
        let pad_count = (0x200/4) - code.len();
        for _ in 0..pad_count { code.push(encode_addi(0,0,0)); }
        code.extend_from_slice(&[encode_csrrs(20,CSR_MEPC,0), encode_csrrs(21,CSR_MCAUSE,0), encode_csrrs(22,CSR_MSTATUS,0), encode_csrrw(0,CSR_MTVEC,0), encode_ecall()]);
        sim.run_instructions(&code);
        assert_eq!(sim.regs.read(20), 16); assert_eq!(sim.regs.read(21), CAUSE_ECALL_M_MODE); assert_eq!(sim.regs.read(22) & MIE, 0);
    }

    // mret
    #[test] fn test_mret() {
        let mut sim = RiscVSimulator::new(65536);
        sim.csr.write(CSR_MEPC, 12);
        sim.run_instructions(&[encode_mret(), encode_addi(1,0,999), encode_addi(2,0,999), encode_addi(3,0,42), encode_ecall()]);
        assert_eq!(sim.regs.read(1), 0); assert_eq!(sim.regs.read(2), 0); assert_eq!(sim.regs.read(3), 42);
    }
    #[test] fn test_mret_reenables() {
        let mut sim = RiscVSimulator::new(65536);
        sim.csr.write(CSR_MSTATUS, 0);
        sim.csr.write(CSR_MEPC, 4);
        sim.run_instructions(&[encode_mret(), encode_ecall()]);
        assert_ne!(sim.csr.read(CSR_MSTATUS) & MIE, 0);
    }

    // Misc
    #[test] fn test_unknown() { let sim = run_program(&[0xFFFFFFFF, encode_ecall()]); assert_eq!(sim.regs.read(1), 0); }
    #[test] fn test_neg_imm() { let sim = run_program(&[encode_addi(1,0,-5), encode_ecall()]); assert_eq!(sim.regs.read(1) as i32, -5); }

    // Integration
    #[test] fn test_fibonacci() {
        let sim = run_program(&[encode_addi(1,0,0), encode_addi(2,0,1), encode_addi(4,0,2), encode_addi(5,0,11), encode_add(3,1,2), encode_addi(1,2,0), encode_addi(2,3,0), encode_addi(4,4,1), encode_bne(4,5,-16), encode_ecall()]);
        assert_eq!(sim.regs.read(2), 55);
    }
    #[test] fn test_memcpy() {
        let mut sim = RiscVSimulator::new(65536);
        sim.mem.write_byte(0x200, 0xDE); sim.mem.write_byte(0x201, 0xAD);
        sim.mem.write_byte(0x202, 0xBE); sim.mem.write_byte(0x203, 0xEF);
        sim.run_instructions(&[encode_addi(1,0,0x200), encode_addi(2,0,0x300), encode_lw(3,1,0), encode_sw(3,2,0), encode_ecall()]);
        for i in 0..4 { assert_eq!(sim.mem.read_byte(0x200+i), sim.mem.read_byte(0x300+i)); }
    }
    #[test] fn test_stack() {
        let sim = run_program(&[encode_addi(2,0,0x400), encode_addi(10,0,42), encode_addi(11,0,99), encode_addi(2,2,-4), encode_sw(10,2,0), encode_addi(2,2,-4), encode_sw(11,2,0), encode_lw(12,2,0), encode_addi(2,2,4), encode_lw(13,2,0), encode_addi(2,2,4), encode_ecall()]);
        assert_eq!(sim.regs.read(12), 99); assert_eq!(sim.regs.read(13), 42); assert_eq!(sim.regs.read(2), 0x400);
    }

    // Step
    #[test] fn test_step() {
        let mut sim = RiscVSimulator::new(65536);
        let prog = assemble(&[encode_addi(1,0,1), encode_addi(2,0,2), encode_ecall()]);
        sim.load_program(&prog);
        let m1 = sim.step(); assert_eq!(m1, "addi"); assert_eq!(sim.regs.read(1), 1);
        let m2 = sim.step(); assert_eq!(m2, "addi"); assert_eq!(sim.regs.read(2), 2);
    }

    // Encode-decode round-trip
    #[test] fn test_round_trip() {
        let cases: Vec<(&str, u32)> = vec![
            ("addi", encode_addi(1,2,42)), ("slti", encode_slti(1,2,-5)), ("sltiu", encode_sltiu(1,2,5)),
            ("xori", encode_xori(1,2,0xFF)), ("ori", encode_ori(1,2,0xFF)), ("andi", encode_andi(1,2,0xFF)),
            ("slli", encode_slli(1,2,5)), ("srli", encode_srli(1,2,5)), ("srai", encode_srai(1,2,5)),
            ("add", encode_add(1,2,3)), ("sub", encode_sub(1,2,3)), ("sll", encode_sll(1,2,3)),
            ("slt", encode_slt(1,2,3)), ("sltu", encode_sltu(1,2,3)), ("xor", encode_xor(1,2,3)),
            ("srl", encode_srl(1,2,3)), ("sra", encode_sra(1,2,3)), ("or", encode_or(1,2,3)), ("and", encode_and(1,2,3)),
            ("lb", encode_lb(1,2,4)), ("lh", encode_lh(1,2,4)), ("lw", encode_lw(1,2,4)),
            ("lbu", encode_lbu(1,2,4)), ("lhu", encode_lhu(1,2,4)),
            ("sb", encode_sb(3,2,4)), ("sh", encode_sh(3,2,4)), ("sw", encode_sw(3,2,4)),
            ("beq", encode_beq(1,2,8)), ("bne", encode_bne(1,2,8)), ("blt", encode_blt(1,2,8)),
            ("bge", encode_bge(1,2,8)), ("bltu", encode_bltu(1,2,8)), ("bgeu", encode_bgeu(1,2,8)),
            ("jal", encode_jal(1,8)), ("jalr", encode_jalr(1,2,4)),
            ("lui", encode_lui(1, 0x12345)), ("auipc", encode_auipc(1, 0x12345)),
            ("ecall", encode_ecall()), ("mret", encode_mret()),
            ("csrrw", encode_csrrw(1, 0x300, 2)), ("csrrs", encode_csrrs(1, 0x300, 2)), ("csrrc", encode_csrrc(1, 0x300, 2)),
        ];
        for (name, encoded) in &cases {
            let result = decode::decode(*encoded, 0);
            assert_eq!(result.mnemonic, *name, "Decode({}) failed", name);
        }
    }

    // CSR unit tests
    #[test] fn test_csr_rw() { let mut csr = CSRFile::new(); assert_eq!(csr.read(CSR_MSTATUS), 0); csr.write(CSR_MSTATUS, 0x1234); assert_eq!(csr.read(CSR_MSTATUS), 0x1234); }
    #[test] fn test_csr_rw_atomic() { let mut csr = CSRFile::new(); csr.write(CSR_MSCRATCH, 42); let old = csr.read_write(CSR_MSCRATCH, 99); assert_eq!(old, 42); assert_eq!(csr.read(CSR_MSCRATCH), 99); }
    #[test] fn test_csr_rs() { let mut csr = CSRFile::new(); csr.write(CSR_MSTATUS, 0xF0); let old = csr.read_set(CSR_MSTATUS, 0x0F); assert_eq!(old, 0xF0); assert_eq!(csr.read(CSR_MSTATUS), 0xFF); }
    #[test] fn test_csr_rc() { let mut csr = CSRFile::new(); csr.write(CSR_MSTATUS, 0xFF); let old = csr.read_clear(CSR_MSTATUS, 0x0F); assert_eq!(old, 0xFF); assert_eq!(csr.read(CSR_MSTATUS), 0xF0); }

    // Edge cases
    #[test] fn test_shift_masking() { let sim = run_program(&[encode_addi(1,0,1), encode_addi(2,0,33), encode_sll(3,1,2), encode_ecall()]); assert_eq!(sim.regs.read(3), 2); }
    #[test] fn test_assemble() { let bytes = assemble(&[0x12345678]); assert_eq!(bytes.len(), 4); assert_eq!(bytes[0], 0x78); assert_eq!(bytes[1], 0x56); assert_eq!(bytes[2], 0x34); assert_eq!(bytes[3], 0x12); }
}
