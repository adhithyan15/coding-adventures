//! GenericISA -- a simplified, educational instruction set.
//!
//! # What is this?
//!
//! This is the default [`InstructionSet`] implementation -- a vendor-neutral ISA
//! designed for teaching, not for matching any real hardware. It proves that
//! the pluggable ISA design works: if you can implement GenericISA, you can
//! implement NVIDIA PTX, AMD GCN, Intel Xe, or ARM Mali the same way.
//!
//! # How it works
//!
//! The `GenericISA::execute()` method is a big match statement. For each
//! opcode, it:
//!
//! 1. Reads source registers
//! 2. Calls the appropriate `fp-arithmetic` function
//! 3. Writes the result to the destination register
//! 4. Returns an [`ExecuteResult`] describing what happened
//!
//! ```text
//! FADD R2, R0, R1:
//!     a = registers.read(R0)          // read 3.14
//!     b = registers.read(R1)          // read 2.71
//!     result = fp_add(a, b)           // 3.14 + 2.71 = 5.85
//!     registers.write(R2, result)     // store in R2
//!     return ExecuteResult("R2 = R0 + R1 = 3.14 + 2.71 = 5.85", ...)
//! ```
//!
//! # Future ISAs follow the same pattern
//!
//! ```text
//! struct PtxIsa;
//! impl InstructionSet for PtxIsa {
//!     fn execute(&self, inst, regs, mem) {
//!         match inst.opcode {
//!             PtxOp::AddF32 => ...   // same as Fadd but with PTX naming
//!             PtxOp::FmaRnF32 => ... // same as Ffma but with PTX naming
//!         }
//!     }
//! }
//! ```
//!
//! The GPUCore doesn't care which ISA is plugged in -- it just calls
//! `isa.execute()` and processes the ExecuteResult.

use std::collections::HashMap;

use fp_arithmetic::{bits_to_float, fp_abs, fp_add, fp_compare, fp_fma, fp_mul, fp_neg, fp_sub};

use crate::memory::LocalMemory;
use crate::opcodes::{Instruction, Opcode};
use crate::protocols::{ExecuteResult, InstructionSet};
use crate::registers::FPRegisterFile;

/// A simplified, educational instruction set for GPU cores.
///
/// This ISA is not tied to any vendor -- it's a teaching tool. It has
/// 16 opcodes covering arithmetic, memory, data movement, and control
/// flow. Any floating-point program can be expressed with these.
///
/// To use a different ISA, create a type that implements [`InstructionSet`]
/// and pass it to `GPUCore::new()`.
#[derive(Debug, Clone, Copy)]
pub struct GenericISA;

impl InstructionSet for GenericISA {
    fn name(&self) -> &str {
        "Generic"
    }

    fn execute(
        &self,
        instruction: &Instruction,
        registers: &mut FPRegisterFile,
        memory: &mut LocalMemory,
    ) -> ExecuteResult {
        match instruction.opcode {
            // --- Floating-point arithmetic ---
            Opcode::Fadd => exec_fadd(instruction, registers),
            Opcode::Fsub => exec_fsub(instruction, registers),
            Opcode::Fmul => exec_fmul(instruction, registers),
            Opcode::Ffma => exec_ffma(instruction, registers),
            Opcode::Fneg => exec_fneg(instruction, registers),
            Opcode::Fabs => exec_fabs(instruction, registers),

            // --- Memory ---
            Opcode::Load => exec_load(instruction, registers, memory),
            Opcode::Store => exec_store(instruction, registers, memory),

            // --- Data movement ---
            Opcode::Mov => exec_mov(instruction, registers),
            Opcode::Limm => exec_limm(instruction, registers),

            // --- Control flow ---
            Opcode::Beq => exec_beq(instruction, registers),
            Opcode::Blt => exec_blt(instruction, registers),
            Opcode::Bne => exec_bne(instruction, registers),
            Opcode::Jmp => exec_jmp(instruction),
            Opcode::Nop => ExecuteResult::new("No operation"),
            Opcode::Halt => ExecuteResult::new("Halted").with_halted(true),
        }
    }
}

// ---------------------------------------------------------------------------
// Arithmetic implementations
// ---------------------------------------------------------------------------

/// FADD Rd, Rs1, Rs2 -> Rd = Rs1 + Rs2.
fn exec_fadd(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let b = regs.read(inst.rs2).expect("read rs2");
    let result = fp_add(&a, &b);
    let (a_f, b_f, r_f) = (bits_to_float(&a), bits_to_float(&b), bits_to_float(&result));
    regs.write(inst.rd, result).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), r_f);
    ExecuteResult::new(format!(
        "R{} = R{} + R{} = {} + {} = {}",
        inst.rd, inst.rs1, inst.rs2, a_f, b_f, r_f
    ))
    .with_registers_changed(changed)
}

/// FSUB Rd, Rs1, Rs2 -> Rd = Rs1 - Rs2.
fn exec_fsub(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let b = regs.read(inst.rs2).expect("read rs2");
    let result = fp_sub(&a, &b);
    let (a_f, b_f, r_f) = (bits_to_float(&a), bits_to_float(&b), bits_to_float(&result));
    regs.write(inst.rd, result).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), r_f);
    ExecuteResult::new(format!(
        "R{} = R{} - R{} = {} - {} = {}",
        inst.rd, inst.rs1, inst.rs2, a_f, b_f, r_f
    ))
    .with_registers_changed(changed)
}

/// FMUL Rd, Rs1, Rs2 -> Rd = Rs1 * Rs2.
fn exec_fmul(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let b = regs.read(inst.rs2).expect("read rs2");
    let result = fp_mul(&a, &b);
    let (a_f, b_f, r_f) = (bits_to_float(&a), bits_to_float(&b), bits_to_float(&result));
    regs.write(inst.rd, result).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), r_f);
    ExecuteResult::new(format!(
        "R{} = R{} * R{} = {} * {} = {}",
        inst.rd, inst.rs1, inst.rs2, a_f, b_f, r_f
    ))
    .with_registers_changed(changed)
}

/// FFMA Rd, Rs1, Rs2, Rs3 -> Rd = Rs1 * Rs2 + Rs3.
fn exec_ffma(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let b = regs.read(inst.rs2).expect("read rs2");
    let c = regs.read(inst.rs3).expect("read rs3");
    let result = fp_fma(&a, &b, &c);
    let (a_f, b_f, c_f, r_f) = (
        bits_to_float(&a),
        bits_to_float(&b),
        bits_to_float(&c),
        bits_to_float(&result),
    );
    regs.write(inst.rd, result).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), r_f);
    ExecuteResult::new(format!(
        "R{} = R{} * R{} + R{} = {} * {} + {} = {}",
        inst.rd, inst.rs1, inst.rs2, inst.rs3, a_f, b_f, c_f, r_f
    ))
    .with_registers_changed(changed)
}

/// FNEG Rd, Rs1 -> Rd = -Rs1.
fn exec_fneg(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let result = fp_neg(&a);
    let (a_f, r_f) = (bits_to_float(&a), bits_to_float(&result));
    regs.write(inst.rd, result).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), r_f);
    ExecuteResult::new(format!(
        "R{} = -R{} = -{} = {}",
        inst.rd, inst.rs1, a_f, r_f
    ))
    .with_registers_changed(changed)
}

/// FABS Rd, Rs1 -> Rd = |Rs1|.
fn exec_fabs(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let result = fp_abs(&a);
    let (a_f, r_f) = (bits_to_float(&a), bits_to_float(&result));
    regs.write(inst.rd, result).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), r_f);
    ExecuteResult::new(format!(
        "R{} = |R{}| = |{}| = {}",
        inst.rd, inst.rs1, a_f, r_f
    ))
    .with_registers_changed(changed)
}

// ---------------------------------------------------------------------------
// Memory implementations
// ---------------------------------------------------------------------------

/// LOAD Rd, [Rs1+imm] -> Rd = Mem[Rs1 + immediate].
fn exec_load(
    inst: &Instruction,
    regs: &mut FPRegisterFile,
    memory: &mut LocalMemory,
) -> ExecuteResult {
    let base = regs.read_float(inst.rs1);
    let address = (base + inst.immediate) as usize;
    let value = memory.load_float(address, regs.fmt).expect("load float");
    let val_f = bits_to_float(&value);
    regs.write(inst.rd, value).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), val_f);
    ExecuteResult::new(format!(
        "R{} = Mem[R{}+{}] = Mem[{}] = {}",
        inst.rd, inst.rs1, inst.immediate, address, val_f
    ))
    .with_registers_changed(changed)
}

/// STORE [Rs1+imm], Rs2 -> Mem[Rs1 + immediate] = Rs2.
fn exec_store(
    inst: &Instruction,
    regs: &mut FPRegisterFile,
    memory: &mut LocalMemory,
) -> ExecuteResult {
    let base = regs.read_float(inst.rs1);
    let address = (base + inst.immediate) as usize;
    let value = regs.read(inst.rs2).expect("read rs2");
    let val_f = bits_to_float(&value);
    memory.store_float(address, &value).expect("store float");

    let mut changed = HashMap::new();
    changed.insert(address, val_f);
    ExecuteResult::new(format!(
        "Mem[R{}+{}] = R{} -> Mem[{}] = {}",
        inst.rs1, inst.immediate, inst.rs2, address, val_f
    ))
    .with_memory_changed(changed)
}

// ---------------------------------------------------------------------------
// Data movement implementations
// ---------------------------------------------------------------------------

/// MOV Rd, Rs1 -> Rd = Rs1.
fn exec_mov(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let value = regs.read(inst.rs1).expect("read rs1");
    let val_f = bits_to_float(&value);
    regs.write(inst.rd, value).expect("write rd");

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), val_f);
    ExecuteResult::new(format!(
        "R{} = R{} = {}",
        inst.rd, inst.rs1, val_f
    ))
    .with_registers_changed(changed)
}

/// LIMM Rd, immediate -> Rd = float literal.
fn exec_limm(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    regs.write_float(inst.rd, inst.immediate);

    let mut changed = HashMap::new();
    changed.insert(format!("R{}", inst.rd), inst.immediate);
    ExecuteResult::new(format!("R{} = {}", inst.rd, inst.immediate))
        .with_registers_changed(changed)
}

// ---------------------------------------------------------------------------
// Control flow implementations
// ---------------------------------------------------------------------------

/// BEQ Rs1, Rs2, offset -> if Rs1 == Rs2: PC += offset.
fn exec_beq(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let b = regs.read(inst.rs2).expect("read rs2");
    let cmp = fp_compare(&a, &b);
    let taken = cmp == 0;
    let offset = if taken { inst.immediate as i64 } else { 1 };
    let (a_f, b_f) = (bits_to_float(&a), bits_to_float(&b));
    let taken_str = if taken {
        "Yes -> branch"
    } else {
        "No -> fall through"
    };
    ExecuteResult::new(format!(
        "BEQ R{}({}) == R{}({})? {}",
        inst.rs1, a_f, inst.rs2, b_f, taken_str
    ))
    .with_next_pc_offset(offset)
}

/// BLT Rs1, Rs2, offset -> if Rs1 < Rs2: PC += offset.
fn exec_blt(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let b = regs.read(inst.rs2).expect("read rs2");
    let cmp = fp_compare(&a, &b);
    let taken = cmp < 0;
    let offset = if taken { inst.immediate as i64 } else { 1 };
    let (a_f, b_f) = (bits_to_float(&a), bits_to_float(&b));
    let taken_str = if taken {
        "Yes -> branch"
    } else {
        "No -> fall through"
    };
    ExecuteResult::new(format!(
        "BLT R{}({}) < R{}({})? {}",
        inst.rs1, a_f, inst.rs2, b_f, taken_str
    ))
    .with_next_pc_offset(offset)
}

/// BNE Rs1, Rs2, offset -> if Rs1 != Rs2: PC += offset.
fn exec_bne(inst: &Instruction, regs: &mut FPRegisterFile) -> ExecuteResult {
    let a = regs.read(inst.rs1).expect("read rs1");
    let b = regs.read(inst.rs2).expect("read rs2");
    let cmp = fp_compare(&a, &b);
    let taken = cmp != 0;
    let offset = if taken { inst.immediate as i64 } else { 1 };
    let (a_f, b_f) = (bits_to_float(&a), bits_to_float(&b));
    let taken_str = if taken {
        "Yes -> branch"
    } else {
        "No -> fall through"
    };
    ExecuteResult::new(format!(
        "BNE R{}({}) != R{}({})? {}",
        inst.rs1, a_f, inst.rs2, b_f, taken_str
    ))
    .with_next_pc_offset(offset)
}

/// JMP target -> PC = target (absolute jump).
fn exec_jmp(inst: &Instruction) -> ExecuteResult {
    let target = inst.immediate as i64;
    ExecuteResult::new(format!("Jump to PC={}", target))
        .with_next_pc_offset(target)
        .with_absolute_jump(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes;
    use fp_arithmetic::FP32;

    fn make_regs() -> FPRegisterFile {
        FPRegisterFile::new(32, FP32)
    }

    fn make_mem() -> LocalMemory {
        LocalMemory::new(4096)
    }

    #[test]
    fn test_fadd() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 3.0);
        regs.write_float(1, 4.0);
        let result = isa.execute(&opcodes::fadd(2, 0, 1), &mut regs, &mut mem);
        assert_eq!(regs.read_float(2), 7.0);
        assert!(result.description.contains("7"));
        assert!(result.registers_changed.is_some());
    }

    #[test]
    fn test_fsub() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 10.0);
        regs.write_float(1, 3.0);
        isa.execute(&opcodes::fsub(2, 0, 1), &mut regs, &mut mem);
        assert_eq!(regs.read_float(2), 7.0);
    }

    #[test]
    fn test_fmul() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 3.0);
        regs.write_float(1, 4.0);
        isa.execute(&opcodes::fmul(2, 0, 1), &mut regs, &mut mem);
        assert_eq!(regs.read_float(2), 12.0);
    }

    #[test]
    fn test_ffma() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 2.0);
        regs.write_float(1, 3.0);
        regs.write_float(2, 1.0);
        isa.execute(&opcodes::ffma(3, 0, 1, 2), &mut regs, &mut mem);
        assert_eq!(regs.read_float(3), 7.0);
    }

    #[test]
    fn test_fneg() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 5.0);
        isa.execute(&opcodes::fneg(1, 0), &mut regs, &mut mem);
        assert_eq!(regs.read_float(1), -5.0);
    }

    #[test]
    fn test_fabs() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, -5.0);
        isa.execute(&opcodes::fabs(1, 0), &mut regs, &mut mem);
        assert_eq!(regs.read_float(1), 5.0);
    }

    #[test]
    fn test_load_store() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        // Store 42.0 to memory address 0
        regs.write_float(0, 0.0); // base address
        regs.write_float(1, 42.0);
        isa.execute(&opcodes::store(0, 1, 0.0), &mut regs, &mut mem);

        // Load it back into R2
        isa.execute(&opcodes::load(2, 0, 0.0), &mut regs, &mut mem);
        assert_eq!(regs.read_float(2), 42.0);
    }

    #[test]
    fn test_load_store_with_offset() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 0.0); // base address
        regs.write_float(1, 99.0);
        isa.execute(&opcodes::store(0, 1, 8.0), &mut regs, &mut mem);

        isa.execute(&opcodes::load(2, 0, 8.0), &mut regs, &mut mem);
        assert_eq!(regs.read_float(2), 99.0);
    }

    #[test]
    fn test_mov() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 7.5);
        isa.execute(&opcodes::mov(1, 0), &mut regs, &mut mem);
        assert_eq!(regs.read_float(1), 7.5);
    }

    #[test]
    fn test_limm() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        isa.execute(&opcodes::limm(0, 3.14), &mut regs, &mut mem);
        assert!((regs.read_float(0) - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_beq_taken() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 5.0);
        regs.write_float(1, 5.0);
        let result = isa.execute(&opcodes::beq(0, 1, 3), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 3);
        assert!(result.description.contains("Yes"));
    }

    #[test]
    fn test_beq_not_taken() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 5.0);
        regs.write_float(1, 6.0);
        let result = isa.execute(&opcodes::beq(0, 1, 3), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 1);
        assert!(result.description.contains("No"));
    }

    #[test]
    fn test_blt_taken() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 3.0);
        regs.write_float(1, 5.0);
        let result = isa.execute(&opcodes::blt(0, 1, 2), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 2);
    }

    #[test]
    fn test_blt_not_taken() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 5.0);
        regs.write_float(1, 3.0);
        let result = isa.execute(&opcodes::blt(0, 1, 2), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 1);
    }

    #[test]
    fn test_bne_taken() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 5.0);
        regs.write_float(1, 6.0);
        let result = isa.execute(&opcodes::bne(0, 1, 4), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 4);
    }

    #[test]
    fn test_bne_not_taken() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        regs.write_float(0, 5.0);
        regs.write_float(1, 5.0);
        let result = isa.execute(&opcodes::bne(0, 1, 4), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 1);
    }

    #[test]
    fn test_jmp() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        let result = isa.execute(&opcodes::jmp(10), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 10);
        assert!(result.absolute_jump);
    }

    #[test]
    fn test_nop() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        let result = isa.execute(&opcodes::nop(), &mut regs, &mut mem);
        assert_eq!(result.next_pc_offset, 1);
        assert!(!result.halted);
    }

    #[test]
    fn test_halt() {
        let isa = GenericISA;
        let mut regs = make_regs();
        let mut mem = make_mem();

        let result = isa.execute(&opcodes::halt(), &mut regs, &mut mem);
        assert!(result.halted);
    }

    #[test]
    fn test_isa_name() {
        let isa = GenericISA;
        assert_eq!(isa.name(), "Generic");
    }
}
